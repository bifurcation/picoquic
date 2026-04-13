//! DualQ AQM (Active Queue Management) implementation.
//!
//! Translated from picoquic/dualq_aqm.c.
//!
//! Implementation of the DualQ Coupled AQM based on RFC 9332.
//! This implements the "timestamp" variant of DualQ PI2.
//!
//! The DualQ manages:
//! - A queue of pending L4S packets (lq) - Low Latency, Low Loss, Scalable
//! - A queue of pending classic packets (cq)
//! - Weighted fair scheduling between queues (15:1 ratio favoring L4S)
//! - PI2 controller for computing marking/drop probabilities
//! - ECN CE marking for L4S queue, drops for classic queue

// =============================================================================
// DualQ Constants
// =============================================================================

/// Maximum link rate for simulation: 125,000,000 Bytes/sec = 1 Gbps.
pub const DUALQ_MAX_LINK_RATE: u64 = 125_000_000;

/// ECN codepoint: Not ECN-Capable Transport.
pub const PICOQUIC_ECN_NOT_ECT: u8 = 0;

/// ECN codepoint: ECN-Capable Transport (1).
pub const PICOQUIC_ECN_ECT_1: u8 = 1;

/// ECN codepoint: ECN-Capable Transport (0).
pub const PICOQUIC_ECN_ECT_0: u8 = 2;

/// ECN codepoint: Congestion Experienced.
pub const PICOQUIC_ECN_CE: u8 = 3;

// =============================================================================
// DualQ Queue Structure
// =============================================================================

/// Statistics and state for a single queue (L4S or Classic).
#[derive(Debug, Clone, Default)]
pub struct DualqQueueStats {
    /// Total bytes in the queue.
    pub queue_bytes: u64,
    /// Queue time in microseconds (sojourn time of head packet).
    pub queue_time: u64,
    /// Number of packets in queue.
    pub count: f64,
    /// Sum of marking probabilities for recurrence relation.
    pub sum_p: f64,
}

impl DualqQueueStats {
    /// Reset queue statistics.
    pub fn reset(&mut self) {
        self.queue_bytes = 0;
        self.queue_time = 0;
        self.count = 0.0;
        self.sum_p = 0.0;
    }

    /// Record packet enqueue.
    pub fn on_enqueue(&mut self, packet_length: u64) {
        self.count += 1.0;
        self.queue_bytes += packet_length;
    }

    /// Record packet dequeue.
    pub fn on_dequeue(&mut self, packet_length: u64) {
        if self.count > 0.0 {
            self.count -= 1.0;
        }
        if packet_length < self.queue_bytes {
            self.queue_bytes -= packet_length;
        } else {
            // Error case - use 1 to avoid stopping dequeue
            self.queue_bytes = if self.count > 0.0 { 1 } else { 0 };
        }
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.count <= 0.0
    }

    /// Probabilistic recurrence for marking/dropping.
    ///
    /// Returns true with probability `likelihood` using a recurrence relation.
    /// This implements the "recur" function from RFC 9332.
    pub fn recur(&mut self, likelihood: f64) -> bool {
        self.sum_p += likelihood;
        if self.sum_p > 1.0 {
            self.sum_p -= 1.0;
            true
        } else {
            false
        }
    }
}

// =============================================================================
// DualQ State Structure
// =============================================================================

/// DualQ PI2 AQM state.
///
/// Implements RFC 9332 DualQ Coupled AQM with PI2 controller.
#[derive(Debug, Clone)]
pub struct DualqState {
    // Initialization parameters
    /// PI2 queue delay target for both L4S and Classic (microseconds).
    pub target: u64,
    /// Coupling factor (default: 2.0).
    pub k: f64,
    /// Above this drop probability, classic queue uses drops instead of marks.
    pub p_cmax: f64,
    /// Interval between updates of PI2 parameters (microseconds).
    pub t_update: u64,
    /// PI integral gain (in 1/microsecond, spec says Hz).
    pub pi2_alpha: f64,
    /// PI proportional gain (in 1/microsecond).
    pub pi2_beta: f64,
    /// Above this queue size threshold, L queue behaves as classic.
    pub max_th: u64,
    /// Queue size above which L queue starts CE marking.
    pub min_th: u64,
    /// Range: max_th - min_th.
    pub range: u64,
    /// Above this drop probability, L4S queue uses drops instead of marks.
    pub p_lmax: f64,
    /// Maximum size of L4S + Classic queues (bytes).
    pub limit: u64,

    // Runtime state
    /// Counter used for weighted fair queuing (15 for L4S, 1 for Classic).
    pub schedule_tick: u32,
    /// L4S queue statistics.
    pub lq: DualqQueueStats,
    /// Classic queue statistics.
    pub cq: DualqQueueStats,
    /// Current length of the classic queue (microseconds).
    pub curq: i64,
    /// Previous length of the classic queue (microseconds).
    pub prevq: i64,
    /// Next time PI2 parameters should be updated.
    pub update_next: u64,
    /// Last input time.
    pub last_input_time: u64,

    // Computed probabilities (from RFC 9332)
    /// p' coefficient: nominal mark rate of L4S queue derived from classic queue length.
    pub pprime: f64,
    /// p'_L coefficient: mark rate of L4S queue computed from L4S queue length.
    pub pprime_l: f64,
    /// Actual mark rate of L4S queue after combining with p_CL.
    pub p_l: f64,
    /// Coupled L4S probability = base prob pprime_L * coupling factor k.
    pub p_cl: f64,
    /// Nominal drop rate of classic queue = pprime_L^2.
    pub p_c: f64,
}

impl Default for DualqState {
    fn default() -> Self {
        Self {
            target: 15_000,   // 15ms default target
            k: 2.0,           // Coupling factor
            p_cmax: 0.25,     // 1/k^2 for k=2
            t_update: 33_333, // RTT_max/3 for 100ms RTT
            pi2_alpha: 0.0,
            pi2_beta: 0.0,
            max_th: 1200,
            min_th: 800,
            range: 400,
            p_lmax: 1.0,
            limit: 0,
            schedule_tick: 0,
            lq: DualqQueueStats::default(),
            cq: DualqQueueStats::default(),
            curq: 0,
            prevq: 0,
            update_next: 0,
            last_input_time: 0,
            pprime: 0.0,
            pprime_l: 0.0,
            p_l: 0.0,
            p_cl: 0.0,
            p_c: 0.0,
        }
    }
}

/// Result of dequeuing a packet from DualQ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DequeueResult {
    /// No packet available.
    Empty,
    /// Packet should be forwarded (possibly with modified ECN mark).
    Forward {
        /// Which queue the packet came from.
        from_lq: bool,
        /// New ECN mark to apply (None means keep original).
        new_ecn_mark: Option<u8>,
    },
    /// Packet should be dropped.
    Drop {
        /// Which queue the packet came from.
        from_lq: bool,
    },
}

impl DualqState {
    /// Initialize DualQ parameters based on link characteristics.
    ///
    /// # Arguments
    /// * `l4s_max` - Maximum L4S marking threshold (0 for default)
    /// * `queue_delay_max` - Maximum queue delay (microseconds)
    /// * `microsec_latency` - Link latency (microseconds)
    /// * `picosec_per_byte` - Transmission time per byte (picoseconds)
    pub fn init_params(
        &mut self,
        l4s_max: u64,
        queue_delay_max: u64,
        microsec_latency: u64,
        picosec_per_byte: u64,
    ) {
        // Compute buffer limit
        if queue_delay_max <= microsec_latency || picosec_per_byte == 0 {
            self.limit = bytes_from_rate(250, DUALQ_MAX_LINK_RATE);
        } else {
            let queue_delay = queue_delay_max - microsec_latency;
            self.limit = (queue_delay * 1_000_000) / picosec_per_byte;
        }

        // DualQ Coupled framework parameters
        self.k = 2.0;

        // PI2 Classic AQM parameters
        self.target = 15_000; // 15ms target
        let rtt_max: u64 = 100_000; // 100ms worst case RTT

        // PI2 constants
        self.p_cmax = 1.0 / (self.k * self.k);
        if self.p_cmax > 1.0 {
            self.p_cmax = 1.0;
        }

        // PI sampling interval
        self.t_update = rtt_max / 3;
        if self.t_update > self.target {
            self.t_update = self.target;
        }

        // PI coefficients
        // The spec says Hz, we measure in 1/us because times are in microseconds
        self.pi2_alpha = (0.1 * self.t_update as f64) / (rtt_max as f64 * rtt_max as f64);
        self.pi2_beta = 0.3 / rtt_max as f64;

        // L4S ramp AQM parameters
        if l4s_max == 0 {
            self.max_th = 1200;
            self.min_th = 800;
        } else {
            self.max_th = l4s_max;
            self.min_th = if l4s_max > 1200 { 800 } else { l4s_max / 3 };
        }
        self.range = self.max_th - self.min_th;

        // L4S constants
        self.p_lmax = 1.0;
    }

    /// Reset the DualQ state.
    pub fn reset(&mut self) {
        self.lq.reset();
        self.cq.reset();
        self.schedule_tick = 0;
        self.curq = 0;
        self.prevq = 0;
        self.pprime = 0.0;
        self.pprime_l = 0.0;
        self.p_l = 0.0;
        self.p_cl = 0.0;
        self.p_c = 0.0;
    }

    /// Classify a packet and record enqueue.
    ///
    /// Returns true if packet should be accepted, false if buffer is full.
    pub fn enqueue(&mut self, packet_length: u64, ecn_mark: u8) -> bool {
        // Check if buffer is full
        if self.cq.queue_bytes + self.lq.queue_bytes + packet_length > self.limit {
            return false; // Drop packet
        }

        // Classify based on ECN marking
        if ecn_mark == PICOQUIC_ECN_ECT_1 || ecn_mark == PICOQUIC_ECN_CE {
            // L4S packet
            self.lq.on_enqueue(packet_length);
        } else {
            // Classic packet
            self.cq.on_enqueue(packet_length);
        }

        true
    }

    /// Compute native L4S AQM probability based on queue sojourn time.
    ///
    /// # Arguments
    /// * `lq_head_arrival_time` - Arrival time of head packet in L4S queue
    /// * `current_time` - Current time
    pub fn laqm(&self, lq_head_arrival_time: Option<u64>, current_time: u64) -> f64 {
        if self.lq.count <= 1.0 {
            return 0.0;
        }

        let lq_time = match lq_head_arrival_time {
            Some(arrival) if arrival < current_time => current_time - arrival,
            _ => 0,
        };

        if lq_time >= self.max_th {
            1.0
        } else if lq_time > self.min_th {
            (lq_time - self.min_th) as f64 / self.range as f64
        } else {
            0.0
        }
    }

    /// Select which queue to dequeue from using weighted round robin.
    ///
    /// Returns (selected_lq, alternate_lq) - the primary choice and fallback.
    pub fn schedule(&mut self) -> (bool, bool) {
        // 15:1 ratio favoring L4S
        let select_lq = (self.schedule_tick & 0x0f) != 0;
        self.schedule_tick = (self.schedule_tick + 1) & 0x0f;
        (select_lq, !select_lq)
    }

    /// Process dequeue of a packet.
    ///
    /// This implements the marking/dropping logic from RFC 9332.
    ///
    /// # Arguments
    /// * `from_lq` - Whether packet came from L4S queue
    /// * `packet_length` - Length of the packet
    /// * `packet_ecn` - Current ECN marking of packet
    /// * `lq_head_arrival_time` - Arrival time of L4S queue head
    /// * `current_time` - Current time
    ///
    /// # Returns
    /// The dequeue result indicating forward/drop and any ECN modifications.
    pub fn process_dequeue(
        &mut self,
        from_lq: bool,
        packet_length: u64,
        packet_ecn: u8,
        lq_head_arrival_time: Option<u64>,
        current_time: u64,
    ) -> DequeueResult {
        if from_lq {
            self.lq.on_dequeue(packet_length);

            // Check for overload saturation
            if self.p_cl < self.p_lmax {
                // Normal L4S operation
                self.pprime_l = self.laqm(lq_head_arrival_time, current_time);
                self.p_l = if self.pprime_l > self.p_cl {
                    self.pprime_l
                } else {
                    self.p_cl
                };

                if self.lq.recur(self.pprime_l) {
                    // Linear marking
                    DequeueResult::Forward {
                        from_lq: true,
                        new_ecn_mark: Some(PICOQUIC_ECN_CE),
                    }
                } else {
                    DequeueResult::Forward {
                        from_lq: true,
                        new_ecn_mark: None,
                    }
                }
            } else {
                // Overload saturation
                if self.lq.recur(self.p_c) {
                    // Probability p_C = p'^2 - revert to Classic drop
                    DequeueResult::Drop { from_lq: true }
                } else if self.lq.recur(self.p_cl) {
                    // Probability p_CL = k * p' - linear marking
                    DequeueResult::Forward {
                        from_lq: true,
                        new_ecn_mark: Some(PICOQUIC_ECN_CE),
                    }
                } else {
                    DequeueResult::Forward {
                        from_lq: true,
                        new_ecn_mark: None,
                    }
                }
            }
        } else {
            self.cq.on_dequeue(packet_length);

            // Classic queue - probability p_C = p'^2
            if self.cq.recur(self.p_c) {
                if packet_ecn == PICOQUIC_ECN_NOT_ECT || self.p_c >= self.p_cmax {
                    // Not ECN capable or overload - drop
                    DequeueResult::Drop { from_lq: false }
                } else {
                    // Square marking
                    DequeueResult::Forward {
                        from_lq: false,
                        new_ecn_mark: Some(PICOQUIC_ECN_CE),
                    }
                }
            } else {
                DequeueResult::Forward {
                    from_lq: false,
                    new_ecn_mark: None,
                }
            }
        }
    }

    /// Update PI2 controller parameters.
    ///
    /// This should be called every T_update interval.
    ///
    /// # Arguments
    /// * `cq_head_arrival_time` - Arrival time of classic queue head
    /// * `lq_head_arrival_time` - Arrival time of L4S queue head
    /// * `current_time` - Current time
    pub fn pi2_update(
        &mut self,
        cq_head_arrival_time: Option<u64>,
        lq_head_arrival_time: Option<u64>,
        current_time: u64,
    ) {
        // Compute queue sojourn times
        let cq_time = match cq_head_arrival_time {
            Some(arrival) if arrival < current_time => current_time - arrival,
            _ => 0,
        };
        let lq_time = match lq_head_arrival_time {
            Some(arrival) if arrival < current_time => current_time - arrival,
            _ => 0,
        };

        // Use max of both queues for curq
        self.curq = cq_time.max(lq_time) as i64;

        let target_delta = self.curq - self.target as i64;
        let delta_q = self.curq - self.prevq;

        // PI controller update
        self.pprime += self.pi2_alpha * target_delta as f64 + self.pi2_beta * delta_q as f64;

        // Bound p' to [0..1]
        self.pprime = self.pprime.clamp(0.0, 1.0);

        // Coupled L4S probability = base prob * coupling factor
        self.p_cl = self.pprime * self.k;
        if self.p_cl > 1.0 {
            self.p_cl = 1.0;
        }

        // Classic drop probability = p' * p'_L
        self.p_c = self.pprime * self.pprime_l;

        self.prevq = self.curq;
    }

    /// Check if PI2 update is needed and return the next update time.
    pub fn check_update_needed(&mut self, current_time: u64) -> bool {
        if current_time >= self.update_next {
            self.update_next = current_time + self.t_update;
            true
        } else {
            false
        }
    }

    /// Check if there are pending packets in either queue.
    pub fn has_pending(&self) -> bool {
        !self.lq.is_empty() || !self.cq.is_empty()
    }

    /// Get queue statistics.
    pub fn get_stats(&self) -> (u64, u64, f64, f64) {
        (
            self.lq.queue_bytes,
            self.cq.queue_bytes,
            self.lq.count,
            self.cq.count,
        )
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute bytes from rate and duration.
fn bytes_from_rate(duration_us: u64, rate_bytes_per_sec: u64) -> u64 {
    (duration_us * rate_bytes_per_sec) / 1_000_000
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dualq_state_default() {
        let state = DualqState::default();
        assert_eq!(state.target, 15_000);
        assert!((state.k - 2.0).abs() < f64::EPSILON);
        assert!(!state.has_pending());
    }

    #[test]
    fn test_dualq_queue_stats() {
        let mut stats = DualqQueueStats::default();
        assert!(stats.is_empty());

        stats.on_enqueue(1000);
        assert!(!stats.is_empty());
        assert_eq!(stats.queue_bytes, 1000);
        assert!((stats.count - 1.0).abs() < f64::EPSILON);

        stats.on_enqueue(500);
        assert_eq!(stats.queue_bytes, 1500);
        assert!((stats.count - 2.0).abs() < f64::EPSILON);

        stats.on_dequeue(1000);
        assert_eq!(stats.queue_bytes, 500);
        assert!((stats.count - 1.0).abs() < f64::EPSILON);

        stats.on_dequeue(500);
        assert_eq!(stats.queue_bytes, 0);
        assert!(stats.is_empty());
    }

    #[test]
    fn test_dualq_recur() {
        let mut stats = DualqQueueStats::default();

        // With probability 0.3, should trigger after ~3-4 calls
        let mut triggered_count = 0;
        for _ in 0..10 {
            if stats.recur(0.3) {
                triggered_count += 1;
            }
        }
        // Should trigger roughly 3 times (10 * 0.3)
        assert!(triggered_count >= 2 && triggered_count <= 4);
    }

    #[test]
    fn test_dualq_recur_probability_one() {
        let mut stats = DualqQueueStats::default();

        // With probability 1.0, should trigger every time after first
        assert!(!stats.recur(1.0)); // First call: 0 + 1.0 = 1.0, not > 1.0
        assert!(stats.recur(1.0)); // Second call: 1.0 + 1.0 = 2.0 > 1.0
        assert!(stats.recur(1.0)); // Third call: 1.0 + 1.0 = 2.0 > 1.0
    }

    #[test]
    fn test_dualq_init_params() {
        let mut state = DualqState::default();
        state.init_params(0, 0, 0, 0);

        assert_eq!(state.target, 15_000);
        assert!((state.k - 2.0).abs() < f64::EPSILON);
        assert!((state.p_cmax - 0.25).abs() < f64::EPSILON);
        assert!(state.pi2_alpha > 0.0);
        assert!(state.pi2_beta > 0.0);
        assert_eq!(state.max_th, 1200);
        assert_eq!(state.min_th, 800);
        assert_eq!(state.range, 400);
    }

    #[test]
    fn test_dualq_init_params_custom_l4s_max() {
        let mut state = DualqState::default();
        state.init_params(2000, 0, 0, 0);

        assert_eq!(state.max_th, 2000);
        assert_eq!(state.min_th, 800);
        assert_eq!(state.range, 1200);
    }

    #[test]
    fn test_dualq_enqueue_classify() {
        let mut state = DualqState::default();
        state.limit = 10000;

        // Classic packet (no ECN)
        assert!(state.enqueue(1000, PICOQUIC_ECN_NOT_ECT));
        assert_eq!(state.cq.queue_bytes, 1000);
        assert_eq!(state.lq.queue_bytes, 0);

        // L4S packet (ECT1)
        assert!(state.enqueue(500, PICOQUIC_ECN_ECT_1));
        assert_eq!(state.cq.queue_bytes, 1000);
        assert_eq!(state.lq.queue_bytes, 500);

        // L4S packet (CE)
        assert!(state.enqueue(500, PICOQUIC_ECN_CE));
        assert_eq!(state.lq.queue_bytes, 1000);
    }

    #[test]
    fn test_dualq_enqueue_buffer_full() {
        let mut state = DualqState::default();
        state.limit = 1000;

        assert!(state.enqueue(500, PICOQUIC_ECN_NOT_ECT));
        assert!(state.enqueue(400, PICOQUIC_ECN_ECT_1));
        // This should fail - would exceed limit
        assert!(!state.enqueue(200, PICOQUIC_ECN_NOT_ECT));
    }

    #[test]
    fn test_dualq_schedule() {
        let mut state = DualqState::default();

        // Count L4S vs Classic selections over 16 ticks
        let mut lq_count = 0;
        let mut cq_count = 0;
        for _ in 0..16 {
            let (select_lq, _) = state.schedule();
            if select_lq {
                lq_count += 1;
            } else {
                cq_count += 1;
            }
        }

        // Should be 15:1 ratio
        assert_eq!(lq_count, 15);
        assert_eq!(cq_count, 1);
    }

    #[test]
    fn test_dualq_laqm() {
        let mut state = DualqState::default();
        state.min_th = 800;
        state.max_th = 1200;
        state.range = 400;
        state.lq.count = 2.0;

        let current_time = 10_000;

        // Below min_th
        let p = state.laqm(Some(current_time - 500), current_time);
        assert!((p - 0.0).abs() < f64::EPSILON);

        // In ramp region
        let p = state.laqm(Some(current_time - 1000), current_time);
        // (1000 - 800) / 400 = 0.5
        assert!((p - 0.5).abs() < f64::EPSILON);

        // Above max_th
        let p = state.laqm(Some(current_time - 1500), current_time);
        assert!((p - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_dualq_pi2_update() {
        let mut state = DualqState::default();
        state.init_params(0, 0, 0, 0);

        let current_time = 100_000;

        // With no queue, pprime should stay at 0
        state.pi2_update(None, None, current_time);
        assert!((state.pprime - 0.0).abs() < 0.01);

        // With queue above target, pprime should increase
        state.pi2_update(Some(current_time - 20_000), None, current_time);
        assert!(state.pprime > 0.0);
    }

    #[test]
    fn test_dualq_pi2_update_bounds() {
        let mut state = DualqState::default();
        state.init_params(0, 0, 0, 0);

        // Force pprime negative (won't actually happen in practice)
        state.pprime = -0.5;
        state.pi2_update(None, None, 100_000);
        assert!(state.pprime >= 0.0);

        // Force pprime above 1.0
        state.pprime = 1.5;
        state.curq = 0;
        state.prevq = 0;
        state.pi2_update(None, None, 100_000);
        // With target_delta = 0 - 15000 = -15000, pprime decreases but stays bounded
        assert!(state.pprime <= 1.0);
    }

    #[test]
    fn test_dualq_process_dequeue_lq_normal() {
        let mut state = DualqState::default();
        state.lq.on_enqueue(1000);
        state.lq.on_enqueue(1000);
        state.p_cl = 0.0; // Below p_lmax
        state.p_lmax = 1.0;
        state.pprime_l = 0.0;

        let result = state.process_dequeue(true, 1000, PICOQUIC_ECN_ECT_1, Some(0), 100);

        match result {
            DequeueResult::Forward { from_lq, .. } => assert!(from_lq),
            _ => panic!("Expected Forward"),
        }
    }

    #[test]
    fn test_dualq_process_dequeue_cq() {
        let mut state = DualqState::default();
        state.cq.on_enqueue(1000);
        state.p_c = 0.0; // No drop probability

        let result = state.process_dequeue(false, 1000, PICOQUIC_ECN_NOT_ECT, None, 100);

        match result {
            DequeueResult::Forward {
                from_lq,
                new_ecn_mark,
            } => {
                assert!(!from_lq);
                assert!(new_ecn_mark.is_none());
            }
            _ => panic!("Expected Forward"),
        }
    }

    #[test]
    fn test_dualq_check_update_needed() {
        let mut state = DualqState::default();
        state.t_update = 1000;
        state.update_next = 5000;

        assert!(!state.check_update_needed(4000));
        assert!(state.check_update_needed(5000));
        assert_eq!(state.update_next, 6000);
    }

    #[test]
    fn test_dualq_has_pending() {
        let mut state = DualqState::default();
        assert!(!state.has_pending());

        state.lq.on_enqueue(100);
        assert!(state.has_pending());

        state.lq.on_dequeue(100);
        assert!(!state.has_pending());

        state.cq.on_enqueue(100);
        assert!(state.has_pending());
    }

    #[test]
    fn test_dualq_reset() {
        let mut state = DualqState::default();
        state.lq.on_enqueue(100);
        state.cq.on_enqueue(200);
        state.pprime = 0.5;
        state.schedule_tick = 10;

        state.reset();

        assert!(!state.has_pending());
        assert!((state.pprime - 0.0).abs() < f64::EPSILON);
        assert_eq!(state.schedule_tick, 0);
    }

    #[test]
    fn test_bytes_from_rate() {
        // 1 second at 1 Gbps = 125 MB
        let bytes = bytes_from_rate(1_000_000, DUALQ_MAX_LINK_RATE);
        assert_eq!(bytes, DUALQ_MAX_LINK_RATE);

        // 1ms at 1 Gbps = 125 KB
        let bytes = bytes_from_rate(1_000, DUALQ_MAX_LINK_RATE);
        assert_eq!(bytes, 125_000);
    }

    #[test]
    fn test_dequeue_result_variants() {
        let empty = DequeueResult::Empty;
        let forward_lq = DequeueResult::Forward {
            from_lq: true,
            new_ecn_mark: Some(PICOQUIC_ECN_CE),
        };
        let drop_cq = DequeueResult::Drop { from_lq: false };

        assert_eq!(empty, DequeueResult::Empty);
        match forward_lq {
            DequeueResult::Forward {
                from_lq,
                new_ecn_mark,
            } => {
                assert!(from_lq);
                assert_eq!(new_ecn_mark, Some(PICOQUIC_ECN_CE));
            }
            _ => panic!("Wrong variant"),
        }
        match drop_cq {
            DequeueResult::Drop { from_lq } => assert!(!from_lq),
            _ => panic!("Wrong variant"),
        }
    }
}
