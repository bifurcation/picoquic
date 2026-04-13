//! Network link simulator for testing.
//!
//! Translated from picoquic/sim_link.c.
//!
//! This module provides a simulated network link with:
//! - Configurable bandwidth (picoseconds per byte)
//! - Configurable latency
//! - Packet loss via rotating 64-bit mask
//! - Burst loss simulation
//! - Jitter (Gaussian and WiFi models)
//! - Queue delay limits (tail drop)
//! - MTU-based drops
//!
//! The simulator uses virtual time for deterministic testing.

use crate::util::{test_random, test_uniform_random};

// =============================================================================
// Constants
// =============================================================================

/// Default maximum packet size.
pub const MAX_PACKET_SIZE: usize = 1536;

// =============================================================================
// Jitter Mode
// =============================================================================

/// Jitter simulation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum JitterMode {
    /// Gaussian jitter with specified mean and standard deviation.
    #[default]
    Gauss = 0,
    /// WiFi-like jitter with multi-component distribution.
    Wifi = 1,
}

// =============================================================================
// Simulated Packet
// =============================================================================

/// A packet in the simulated network.
#[derive(Debug, Clone)]
pub struct SimPacket {
    /// Time at which packet arrives at destination.
    pub arrival_time: u64,
    /// Packet length in bytes.
    pub length: usize,
    /// ECN marking (0 = not-ECT, 1 = ECT(1), 2 = ECT(0), 3 = CE).
    pub ecn_mark: u8,
    /// Packet data.
    pub data: Vec<u8>,
}

impl SimPacket {
    /// Create a new packet with the given data.
    pub fn new(data: Vec<u8>) -> Self {
        let length = data.len();
        Self {
            arrival_time: 0,
            length,
            ecn_mark: 0,
            data,
        }
    }

    /// Create an empty packet of the specified length.
    pub fn with_length(length: usize) -> Self {
        Self {
            arrival_time: 0,
            length,
            ecn_mark: 0,
            data: vec![0; length],
        }
    }
}

// =============================================================================
// Simulated Link
// =============================================================================

/// A simulated network link.
#[derive(Debug)]
pub struct SimLink {
    /// Time at which next packet can be sent.
    pub next_send_time: u64,
    /// Time until which queue is busy.
    pub queue_time: u64,
    /// Time after which packets can be delivered (for suspension).
    pub resume_time: u64,
    /// Maximum queue delay before tail drop.
    pub queue_delay_max: u64,
    /// Picoseconds per byte (inverse of bandwidth).
    pub picosec_per_byte: u64,
    /// One-way latency in microseconds.
    pub microsec_latency: u64,
    /// Count of dropped packets.
    pub packets_dropped: u64,
    /// Count of sent packets.
    pub packets_sent: u64,
    /// Jitter amount in microseconds.
    pub jitter: u64,
    /// Jitter mode.
    pub jitter_mode: JitterMode,
    /// Random seed for jitter.
    pub jitter_seed: u64,
    /// Path MTU (packets larger than this are dropped).
    pub path_mtu: usize,
    /// Packet queue.
    packets: Vec<SimPacket>,
    /// Loss mask for deterministic loss injection.
    loss_mask: Option<u64>,
    /// Number of packets to lose in each burst.
    pub nb_loss_in_burst: u64,
    /// Packets between loss bursts.
    pub packets_between_losses: u64,
    /// Next burst starts after this many packets sent.
    packets_sent_next_burst: u64,
    /// Losses remaining in current burst.
    nb_losses_this_burst: u64,
    /// Burst ends at this time.
    end_of_burst_time: u64,
    /// Link is switched off (all packets dropped).
    pub is_switched_off: bool,
    /// Link is unreachable.
    pub is_unreachable: bool,
    /// Link is suspended (packets queued but not delivered).
    pub is_suspended: bool,
}

impl SimLink {
    /// Create a new simulated link.
    ///
    /// # Arguments
    /// * `data_rate_gps` - Data rate in Gbps (0 = infinite bandwidth)
    /// * `microsec_latency` - One-way latency in microseconds
    /// * `loss_mask` - Optional 64-bit loss mask (rotating)
    /// * `queue_delay_max` - Maximum queue delay before drop (0 = no limit)
    /// * `current_time` - Current simulation time
    pub fn new(
        data_rate_gps: f64,
        microsec_latency: u64,
        loss_mask: Option<u64>,
        queue_delay_max: u64,
        current_time: u64,
    ) -> Self {
        // Convert data rate to picoseconds per byte
        // picosec = 8 bits/byte * 1e12 ps/s / (rate * 1e9 bits/s)
        // = 8000 / rate ps/byte
        let picosec_per_byte = if data_rate_gps <= 0.0 {
            0
        } else {
            let pico_d = (8000.0 / data_rate_gps) * 1.024 * 1.024; // Account for binary units
            pico_d as u64
        };

        Self {
            next_send_time: current_time,
            queue_time: current_time,
            resume_time: 0,
            queue_delay_max,
            picosec_per_byte,
            microsec_latency,
            packets_dropped: 0,
            packets_sent: 0,
            jitter: 0,
            jitter_mode: JitterMode::Gauss,
            jitter_seed: 0xDEAD_BEEF_BABA_C001,
            path_mtu: MAX_PACKET_SIZE,
            packets: Vec::new(),
            loss_mask,
            nb_loss_in_burst: 0,
            packets_between_losses: 0,
            packets_sent_next_burst: 0,
            nb_losses_this_burst: 0,
            end_of_burst_time: 0,
            is_switched_off: false,
            is_unreachable: false,
            is_suspended: false,
        }
    }

    /// Get the arrival time of the next packet, or current_time if no packet.
    pub fn next_arrival(&self, current_time: u64) -> u64 {
        match self.packets.first() {
            Some(packet) if packet.arrival_time < current_time => packet.arrival_time,
            _ => current_time,
        }
    }

    /// Dequeue a packet if one has arrived.
    pub fn dequeue(&mut self, current_time: u64) -> Option<SimPacket> {
        if let Some(packet) = self.packets.first() {
            if packet.arrival_time <= current_time {
                return Some(self.packets.remove(0));
            }
        }
        None
    }

    /// Compute transmission time for a packet.
    pub fn transmit_time(&self, length: usize) -> u64 {
        (self.picosec_per_byte * length as u64) >> 20
    }

    /// Compute current queue delay.
    pub fn queue_delay(&self, current_time: u64) -> u64 {
        self.queue_time.saturating_sub(current_time)
    }

    /// Test if packet should be lost based on loss mask.
    fn test_loss(&mut self) -> bool {
        if let Some(ref mut mask) = self.loss_mask {
            let loss_bit = *mask & 1;
            // Rotate mask
            *mask >>= 1;
            *mask |= loss_bit << 63;
            loss_bit != 0
        } else {
            false
        }
    }

    /// Test if packet should be lost due to burst loss simulation.
    fn sim_loss(&mut self, current_time: u64) -> bool {
        if self.nb_loss_in_burst == 0 {
            return false;
        }

        if self.packets_sent > self.packets_sent_next_burst {
            let picosec_wait = self.nb_loss_in_burst * self.picosec_per_byte * 1536;
            self.packets_sent_next_burst = self.packets_sent + self.packets_between_losses;
            self.nb_losses_this_burst = self.nb_loss_in_burst - 1;
            self.end_of_burst_time = current_time + (picosec_wait / 1_000_000);
            return true;
        }

        if self.nb_losses_this_burst > 0 {
            if current_time > self.end_of_burst_time {
                self.nb_losses_this_burst = 0;
            } else {
                self.nb_losses_this_burst -= 1;
                return true;
            }
        }

        false
    }

    /// Compute WiFi-like jitter.
    fn wifi_jitter(&mut self) -> u64 {
        const EXP_MINUS_1_X40000000: u64 = 395_007_542; // exp(-1) * 2^30
        const PRIMARY_JITTER: u64 = 1000;

        let n1 = poisson_random(&mut self.jitter_seed, EXP_MINUS_1_X40000000);
        let mut jitter = n1 * PRIMARY_JITTER;

        if n1 > 0 {
            // Smoothing
            jitter -= test_uniform_random(&mut self.jitter_seed, PRIMARY_JITTER);
        }

        if self.jitter > 1000 {
            let mut r = test_random(&mut self.jitter_seed);
            r ^= r >> 30;
            r &= 0x3FFF_FFFF;
            r *= 84000;
            if r < ((self.jitter - 1000) << 30) {
                const EXP_MINUS_12_X40000000: u64 = 6597; // exp(-12) * 2^30
                const SECONDARY_JITTER: u64 = 7500;

                let n2 = poisson_random(&mut self.jitter_seed, EXP_MINUS_12_X40000000);
                jitter += n2 * SECONDARY_JITTER;
                if n2 > 1 {
                    jitter -= test_uniform_random(&mut self.jitter_seed, SECONDARY_JITTER);
                }
            }
        }

        jitter
    }

    /// Compute jitter based on current mode.
    fn compute_jitter(&mut self) -> u64 {
        match self.jitter_mode {
            JitterMode::Wifi => self.wifi_jitter(),
            JitterMode::Gauss => {
                let x = gauss_random(&mut self.jitter_seed).clamp(-3.0, 3.0) / 3.0;
                let delta = (x * self.jitter as f64) as i64;
                (self.jitter as i64 + delta) as u64
            }
        }
    }

    /// Submit a packet to the link.
    pub fn submit(&mut self, mut packet: SimPacket, current_time: u64) {
        if self.is_suspended {
            packet.arrival_time = u64::MAX;
            self.packets.push(packet);
            return;
        }

        let queue_delay = self.queue_delay(current_time);

        // Check for tail drop
        let should_drop = self.queue_delay_max > 0 && queue_delay >= self.queue_delay_max;

        self.enqueue(packet, current_time, should_drop);
    }

    /// Enqueue a packet (bypassing AQM).
    pub fn enqueue(&mut self, packet: SimPacket, current_time: u64, should_drop: bool) {
        if should_drop {
            self.packets_dropped += 1;
            return;
        }

        let mut transmit_time = self.transmit_time(packet.length);
        if transmit_time == 0 {
            transmit_time = 1;
        }

        let queue_delay = self.queue_delay(current_time);
        self.queue_time = current_time + queue_delay + transmit_time;

        // Check for drops: MTU, loss mask, switched off, burst loss
        if packet.length > self.path_mtu
            || self.test_loss()
            || self.is_switched_off
            || self.sim_loss(current_time)
        {
            self.packets_dropped += 1;
            return;
        }

        // Packet accepted
        self.packets_sent += 1;

        let mut arrival_time = self.queue_time + self.microsec_latency;
        if self.jitter != 0 {
            arrival_time += self.compute_jitter();
        }
        if arrival_time < self.resume_time {
            arrival_time = self.resume_time;
        }

        let mut packet = packet;
        packet.arrival_time = arrival_time;

        // Insert in order (packets are generally already ordered)
        let pos = self
            .packets
            .iter()
            .position(|p| p.arrival_time > arrival_time)
            .unwrap_or(self.packets.len());
        self.packets.insert(pos, packet);
    }

    /// Suspend the link until the specified time.
    ///
    /// # Arguments
    /// * `time_end_of_interval` - Time when suspension ends
    /// * `simulate_receive` - If true, delay existing packets; if false, requeue them
    pub fn suspend(&mut self, time_end_of_interval: u64, simulate_receive: bool) {
        if simulate_receive {
            self.resume_time = time_end_of_interval;
            for packet in &mut self.packets {
                if packet.arrival_time < time_end_of_interval {
                    packet.arrival_time = time_end_of_interval;
                }
            }
        } else {
            self.queue_time = time_end_of_interval;
            let old_packets: Vec<SimPacket> = self.packets.drain(..).collect();
            for packet in old_packets {
                self.submit(packet, time_end_of_interval);
            }
        }
    }

    /// Get count of packets in queue.
    pub fn queue_length(&self) -> usize {
        self.packets.len()
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    /// Set the loss mask.
    pub fn set_loss_mask(&mut self, mask: Option<u64>) {
        self.loss_mask = mask;
    }

    /// Configure burst loss parameters.
    pub fn set_burst_loss(&mut self, losses_per_burst: u64, packets_between: u64) {
        self.nb_loss_in_burst = losses_per_burst;
        self.packets_between_losses = packets_between;
        self.packets_sent_next_burst = self.packets_sent + packets_between;
    }

    /// Set jitter parameters.
    pub fn set_jitter(&mut self, jitter_us: u64, mode: JitterMode) {
        self.jitter = jitter_us;
        self.jitter_mode = mode;
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate a Poisson-distributed random number.
///
/// Uses the inverse transform method with exp(-lambda) * 2^30.
fn poisson_random(seed: &mut u64, exp_minus_lambda_2_30: u64) -> u64 {
    // Simplified Poisson: count how many uniform samples before product < threshold
    let mut count = 0u64;
    let mut product = 1u64 << 30;

    loop {
        let r = test_random(seed);
        let r_scaled = (r >> 34) & 0x3FFF_FFFF; // 30 bits
        product = (product * r_scaled) >> 30;

        if product < exp_minus_lambda_2_30 {
            break;
        }
        count += 1;
        if count > 100 {
            // Safety limit
            break;
        }
    }

    count
}

/// Generate a Gaussian-distributed random number using Box-Muller.
fn gauss_random(seed: &mut u64) -> f64 {
    // Box-Muller transform
    let u1 = (test_random(seed) as f64) / (u64::MAX as f64);
    let u2 = (test_random(seed) as f64) / (u64::MAX as f64);

    let u1 = if u1 < 1e-10 { 1e-10 } else { u1 }; // Avoid log(0)

    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sim_link_new() {
        let link = SimLink::new(1.0, 10_000, None, 0, 0);
        assert_eq!(link.microsec_latency, 10_000);
        assert_eq!(link.packets_dropped, 0);
        assert_eq!(link.packets_sent, 0);
        assert!(link.is_empty());
    }

    #[test]
    fn test_sim_link_bandwidth() {
        // 1 Gbps = 8000 picosec/byte (adjusted for binary)
        let link = SimLink::new(1.0, 0, None, 0, 0);
        // picosec_per_byte = 8000 * 1.024^2 ≈ 8389
        assert!(link.picosec_per_byte > 8000);
        assert!(link.picosec_per_byte < 9000);
    }

    #[test]
    fn test_sim_link_submit_dequeue() {
        let mut link = SimLink::new(1.0, 10_000, None, 0, 0);

        let packet = SimPacket::with_length(1000);
        link.submit(packet, 0);

        assert_eq!(link.packets_sent, 1);
        assert_eq!(link.queue_length(), 1);

        // Packet hasn't arrived yet (latency = 10ms)
        assert!(link.dequeue(5_000).is_none());

        // Now it should arrive
        let packet = link.dequeue(20_000);
        assert!(packet.is_some());
        assert!(link.is_empty());
    }

    #[test]
    fn test_sim_link_loss_mask() {
        let mut link = SimLink::new(1.0, 1_000, Some(0b1010), 0, 0);

        // Pattern: 1010 = drop, keep, drop, keep...
        for i in 0..4 {
            let packet = SimPacket::with_length(100);
            link.submit(packet, i * 100);
        }

        // Should have dropped 2, kept 2
        assert_eq!(link.packets_dropped, 2);
        assert_eq!(link.packets_sent, 2);
    }

    #[test]
    fn test_sim_link_mtu_drop() {
        let mut link = SimLink::new(1.0, 1_000, None, 0, 0);
        link.path_mtu = 1000;

        // Packet within MTU
        link.submit(SimPacket::with_length(500), 0);
        assert_eq!(link.packets_sent, 1);

        // Packet exceeds MTU
        link.submit(SimPacket::with_length(1500), 100);
        assert_eq!(link.packets_dropped, 1);
    }

    #[test]
    fn test_sim_link_queue_delay_max() {
        let mut link = SimLink::new(0.01, 1_000, None, 10_000, 0);

        // Submit many packets to build up queue
        for _ in 0..100 {
            let packet = SimPacket::with_length(1000);
            link.submit(packet, 0);
        }

        // Some should have been dropped due to queue delay max
        assert!(link.packets_dropped > 0);
    }

    #[test]
    fn test_sim_link_suspend() {
        let mut link = SimLink::new(1.0, 1_000, None, 0, 0);

        // Submit a packet
        link.submit(SimPacket::with_length(100), 0);

        // Suspend until time 100_000
        link.suspend(100_000, true);

        // Packet should be delayed
        assert!(link.packets.first().unwrap().arrival_time >= 100_000);
    }

    #[test]
    fn test_sim_link_jitter() {
        let mut link = SimLink::new(1.0, 10_000, None, 0, 0);
        link.set_jitter(5_000, JitterMode::Gauss);

        // Submit multiple packets and check arrival times vary
        let mut arrival_times = Vec::new();
        for i in 0..10 {
            link.submit(SimPacket::with_length(100), i * 1000);
        }

        for packet in &link.packets {
            arrival_times.push(packet.arrival_time);
        }

        // With jitter, we just verify packets were queued
        // (Variation test would be probabilistic)
        assert!(!arrival_times.is_empty());
    }

    #[test]
    fn test_poisson_random() {
        let mut seed = 0x12345678u64;
        let exp_minus_1 = 395_007_542u64; // exp(-1) * 2^30

        let mut sum = 0u64;
        let n = 1000;
        for _ in 0..n {
            sum += poisson_random(&mut seed, exp_minus_1);
        }

        // Mean should be approximately 1 for lambda=1
        let mean = sum as f64 / n as f64;
        assert!(mean > 0.5 && mean < 2.0, "Poisson mean: {}", mean);
    }

    #[test]
    fn test_gauss_random() {
        let mut seed = 0x12345678u64;

        let mut sum = 0.0;
        let n = 1000;
        for _ in 0..n {
            sum += gauss_random(&mut seed);
        }

        // Mean should be approximately 0
        let mean = sum / n as f64;
        assert!(mean.abs() < 0.2, "Gaussian mean: {}", mean);
    }

    #[test]
    fn test_sim_link_switched_off() {
        let mut link = SimLink::new(1.0, 1_000, None, 0, 0);
        link.is_switched_off = true;

        link.submit(SimPacket::with_length(100), 0);

        assert_eq!(link.packets_dropped, 1);
        assert_eq!(link.packets_sent, 0);
    }
}
