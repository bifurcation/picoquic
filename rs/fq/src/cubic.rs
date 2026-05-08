//! Translation of `picoquic/cubic.c` — CUBIC congestion control.
//!
//! Phase 4: `cubic_W_cubic` (private helper) and `cubic_observe`
//! (public API method on [`CubicState`]).

use crate::cc_common::{ConnectionCc, MinMaxRtt, PathCc, SMOOTHED_LOSS_THRESHOLD};
use crate::internal::{CWIN_INITIAL, CWIN_MINIMUM, Connection, Path, TARGET_RENO_RTT};
use crate::{CongestionNotification, PerAckState};

// ---------------------------------------------------------------------------
// Constants from cubic.c `#define`s.

const CUBIC_C: f64 = 0.4;
const CUBIC_BETA_ECN: f64 = 7.0 / 8.0;
#[allow(dead_code)]
const CUBIC_BETA: f64 = 3.0 / 4.0;

// ---------------------------------------------------------------------------
// Algorithm-state enum.  C: `picoquic_cubic_alg_state_t`.

/// C: `picoquic_cubic_alg_state_t`
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum CubicAlgState {
    #[default]
    SlowStart = 0,
    Recovery,
    CongestionAvoidance,
}

// ---------------------------------------------------------------------------
// Per-path CUBIC state.  C: `picoquic_cubic_state_t`.

/// C: `picoquic_cubic_state_t`
///
/// Type deviations from C:
/// * `alg_state` (`picoquic_cubic_alg_state_t`) → [`CubicAlgState`]
/// * `previous_alg_state` stored as `u64` to preserve raw casts done in
///   the C body (`cubic_state->previous_alg_state = cubic_state->alg_state`)
/// * `K`, `W_max`, `W_last_max`, `W_reno` (`double`) → `f64`
/// * `rtt_filter` (`picoquic_min_max_rtt_t`) → [`MinMaxRtt`]
pub struct CubicState {
    pub alg_state: CubicAlgState,
    pub recovery_sequence: u64,
    pub start_of_epoch: u64,
    pub previous_start_of_epoch: u64,
    pub previous_alg_state: u64,
    pub previous_ssthresh: u64,
    pub previous_cwin: u64,
    pub k: f64,
    pub w_max: f64,
    pub w_last_max: f64,
    pub w_reno: f64,
    pub ssthresh: u64,
    pub rtt_filter: MinMaxRtt,
}

impl Default for CubicState {
    fn default() -> Self {
        Self {
            alg_state: CubicAlgState::SlowStart,
            recovery_sequence: 0,
            start_of_epoch: 0,
            previous_start_of_epoch: 0,
            previous_alg_state: 0,
            previous_ssthresh: u64::MAX,
            previous_cwin: 0,
            k: 0.0,
            w_max: 0.0,
            w_last_max: 0.0,
            w_reno: 0.0,
            ssthresh: u64::MAX,
            rtt_filter: MinMaxRtt::default(),
        }
    }
}

impl CubicState {
    /// C: `cubic_reset` (picoquic/cubic.c:53)
    ///
    /// Reset `self` to the initial CUBIC state for `path_x` at `current_time`.
    /// Mirrors the C body: zero the struct, set `path_x.cwin` to
    /// [`CWIN_INITIAL`], then populate per-epoch fields.
    pub fn reset(&mut self, path_x: &mut Path, current_time: u64) {
        let w_last_max = u64::MAX as f64 / path_x.send_mtu as f64;
        path_x.cwin = CWIN_INITIAL;
        *self = Self {
            alg_state: CubicAlgState::SlowStart,
            recovery_sequence: 0,
            start_of_epoch: current_time,
            previous_start_of_epoch: current_time,
            previous_alg_state: CubicAlgState::SlowStart as u64,
            previous_ssthresh: u64::MAX,
            previous_cwin: CWIN_INITIAL,
            k: 0.0,
            w_max: w_last_max,
            w_last_max,
            w_reno: CWIN_INITIAL as f64,
            ssthresh: u64::MAX,
            rtt_filter: MinMaxRtt::default(),
        };
    }

    /// C: `cubic_W_cubic` (picoquic/cubic.c:121)
    ///
    /// Compute `W_cubic(t) = C * (t − K)³ + W_max` where `t` is the
    /// elapsed time in seconds since `start_of_epoch`.  Returns the
    /// window size in packets (not bytes).
    ///
    /// The subtraction `current_time - start_of_epoch` mirrors the
    /// unsigned C arithmetic; wrapping is intentional for out-of-order
    /// calls (same as C `uint64_t` semantics).
    fn w_cubic(&self, current_time: u64) -> f64 {
        let delta_t_sec =
            current_time.wrapping_sub(self.start_of_epoch) as f64 / 1_000_000.0 - self.k;
        CUBIC_C * (delta_t_sec * delta_t_sec * delta_t_sec) + self.w_max
    }

    /// C: `cubic_enter_avoidance` (picoquic/cubic.c:132)
    ///
    /// Compute the CUBIC coefficient `K` and switch to congestion
    /// avoidance at `current_time`.
    pub fn enter_avoidance(&mut self, current_time: u64) {
        self.k = cubic_root(self.w_max * (1.0 - CUBIC_BETA_ECN) / CUBIC_C);
        self.alg_state = CubicAlgState::CongestionAvoidance;
        self.start_of_epoch = current_time;
        self.previous_start_of_epoch = self.start_of_epoch;
    }

    /// C: `cubic_observe` (picoquic/cubic.c:562)
    ///
    /// Report the current algorithm state and `W_max` to an observer.
    /// Returns `(cc_state, cc_param)` where `cc_state` is the numeric
    /// discriminant of [`CubicAlgState`] and `cc_param` is `W_max`
    /// cast to `u64` (matching the C `(uint64_t)cubic_state->W_max`).
    pub fn observe(&self) -> (u64, u64) {
        (self.alg_state as u64, self.w_max as u64)
    }

    /// Convenience wrapper that fills the two out-parameters used by
    /// the C `cubic_observe` call-site signature
    /// `(path_x, &mut cc_state, &mut cc_param)`.
    pub fn observe_into(&self, cc_state: &mut u64, cc_param: &mut u64) {
        let (s, p) = self.observe();
        *cc_state = s;
        *cc_param = p;
    }

    /// C: `cubic_init` (picoquic/cubic.c:70)
    ///
    /// Initialise per-path CUBIC state and store it in
    /// `path_x.congestion_alg_state`.  Mirrors the C pattern of
    /// `malloc` + `cubic_reset`; the Rust translation boxes a
    /// `CubicState` into `Option<Box<dyn Any>>`.
    pub fn init(path_x: &mut Path, _option_string: Option<&str>, current_time: u64) {
        let mut state = Box::new(CubicState::default());
        state.reset(path_x, current_time);
        path_x.congestion_alg_state = Some(state);
    }

    /// C: `cubic_enter_recovery` (picoquic/cubic.c:144)
    ///
    /// Record the start of a recovery epoch, apply the CUBIC fast-
    /// convergence rule, compute the new slow-start threshold, and
    /// either fall back to slow start (if the new threshold is too
    /// small) or enter congestion avoidance immediately.
    pub fn enter_recovery(
        &mut self,
        connection: &Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        current_time: u64,
    ) {
        self.recovery_sequence = connection.sequence_number(path_x);
        self.previous_start_of_epoch = self.start_of_epoch;
        self.previous_alg_state = self.alg_state as u64;
        self.previous_ssthresh = self.ssthresh;
        self.previous_cwin = path_x.cwin;

        self.w_max = path_x.cwin as f64 / path_x.send_mtu as f64;

        // Fast convergence.
        if self.w_max < self.w_last_max {
            self.w_last_max = self.w_max;
            self.w_max *= CUBIC_BETA_ECN;
        } else {
            self.w_last_max = self.w_max;
        }

        self.ssthresh = (self.w_max * CUBIC_BETA_ECN * path_x.send_mtu as f64) as u64;

        if self.ssthresh < CWIN_MINIMUM {
            // Things are very bad — fall back to slow start.
            self.alg_state = CubicAlgState::SlowStart;
            self.ssthresh = u64::MAX;
            path_x.is_ssthresh_initialized = false;
            self.previous_start_of_epoch = self.start_of_epoch;
            self.start_of_epoch = current_time;
            self.w_reno = CWIN_MINIMUM as f64;
            path_x.cwin = CWIN_MINIMUM;
        } else if notification == CongestionNotification::Timeout {
            path_x.cwin = CWIN_MINIMUM;
            self.previous_start_of_epoch = self.start_of_epoch;
            self.start_of_epoch = current_time;
            self.alg_state = CubicAlgState::SlowStart;
        } else {
            // Enter congestion avoidance immediately.
            self.enter_avoidance(current_time);
            let w_cubic = self.w_cubic(current_time);
            let win_cubic = (w_cubic * path_x.send_mtu as f64) as u64;
            self.w_reno = path_x.cwin as f64 / 2.0;
            // See comment in cubic.c:195-200: w_cubic >= w_reno holds for
            // CUBIC_BETA_ECN > 0.618, so we pick win_cubic unconditionally.
            path_x.cwin = win_cubic;
        }
    }

    /// C: `cubic_correct_spurious` (picoquic/cubic.c:214)
    ///
    /// Undo the most-recent recovery step when the triggering loss
    /// turns out to have been spurious.
    pub fn correct_spurious(&mut self, path_x: &mut Path, current_time: u64) {
        if self.ssthresh != u64::MAX {
            self.w_max = self.w_last_max;
            self.start_of_epoch = self.previous_start_of_epoch;
            self.alg_state = match self.previous_alg_state {
                0 => CubicAlgState::SlowStart,
                1 => CubicAlgState::Recovery,
                _ => CubicAlgState::CongestionAvoidance,
            };
            if self.alg_state != CubicAlgState::SlowStart {
                self.enter_avoidance(self.previous_start_of_epoch);
                let w_cubic = self.w_cubic(current_time);
                self.w_reno = w_cubic * path_x.send_mtu as f64;
                self.ssthresh = (self.w_max * CUBIC_BETA * path_x.send_mtu as f64) as u64;
                path_x.cwin = self.w_reno as u64;
            } else {
                self.ssthresh = self.previous_ssthresh;
                path_x.cwin = self.previous_cwin;
                self.w_reno = self.previous_cwin as f64;
            }
        }
    }

    /// C: `cubic_notify` (picoquic/cubic.c:243)
    ///
    /// Main congestion-control event handler.  Called by the QUIC engine
    /// on every ACK, loss, ECN, RTT-measurement, and reset event.
    pub fn notify(
        &mut self,
        connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: u64,
    ) {
        use crate::Instant;

        path_x.is_cc_data_updated = true;

        match notification {
            CongestionNotification::Acknowledgement => match self.alg_state {
                CubicAlgState::SlowStart => {
                    path_x.cwin = path_x.update_target_cwin_estimation();
                    if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
                        path_x.cwin += path_x.slow_start_increase_ex(
                            connection,
                            ack_state.nb_bytes_acknowledged,
                            false,
                        );
                        if path_x.cwin >= self.ssthresh {
                            self.w_reno = path_x.cwin as f64 / 2.0;
                            path_x.is_ssthresh_initialized = true;
                            self.enter_avoidance(current_time);
                        }
                    }
                }
                CubicAlgState::Recovery => {
                    self.alg_state = CubicAlgState::SlowStart;
                    path_x.cwin += ack_state.nb_bytes_acknowledged;
                    if path_x.cwin >= self.ssthresh {
                        self.alg_state = CubicAlgState::CongestionAvoidance;
                    }
                }
                CubicAlgState::CongestionAvoidance => {
                    if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
                        // Protect against limited senders.
                        if self.start_of_epoch < path_x.last_sender_limited_time.ticks() {
                            self.start_of_epoch = path_x.last_sender_limited_time.ticks();
                        }
                        let w_cubic = self.w_cubic(current_time);
                        let win_cubic = (w_cubic * path_x.send_mtu as f64) as u64;
                        self.w_reno += ack_state.nb_bytes_acknowledged as f64
                            * path_x.send_mtu as f64
                            / self.w_reno;
                        if win_cubic as f64 > self.w_reno {
                            path_x.cwin = win_cubic;
                        } else {
                            path_x.cwin = self.w_reno as u64;
                        }
                    }
                }
            },

            CongestionNotification::Repeat
            | CongestionNotification::Timeout
            | CongestionNotification::EcnEc => match self.alg_state {
                CubicAlgState::SlowStart => {
                    if (notification == CongestionNotification::EcnEc
                        || self.rtt_filter.hystart_loss_test(
                            notification,
                            ack_state.lost_packet_number,
                            SMOOTHED_LOSS_THRESHOLD,
                        ))
                        && (current_time.wrapping_sub(self.start_of_epoch)
                            > path_x.smoothed_rtt.ticks()
                            || self.recovery_sequence <= connection.ack_number(path_x))
                    {
                        path_x.is_ssthresh_initialized = true;
                        self.enter_recovery(connection, path_x, notification, current_time);
                    }
                }
                CubicAlgState::Recovery | CubicAlgState::CongestionAvoidance => {
                    if ack_state.lost_packet_number >= self.recovery_sequence
                        && (notification == CongestionNotification::EcnEc
                            || self.rtt_filter.hystart_loss_test(
                                notification,
                                ack_state.lost_packet_number,
                                SMOOTHED_LOSS_THRESHOLD,
                            ))
                    {
                        self.enter_recovery(connection, path_x, notification, current_time);
                    }
                }
            },

            CongestionNotification::SpuriousRepeat => {
                self.correct_spurious(path_x, current_time);
            }

            CongestionNotification::RttMeasurement
                if self.alg_state == CubicAlgState::SlowStart && self.ssthresh == u64::MAX =>
            {
                let rtt_meas = if connection.is_time_stamp_enabled {
                    ack_state.one_way_delay
                } else {
                    ack_state.rtt_measurement
                };
                let ptime = Instant::from_ticks(
                    connection
                        .paths
                        .first()
                        .map_or(0, |p| p.pacing.packet_time_microsec.ticks()),
                );
                if self.rtt_filter.hystart_test(
                    rtt_meas,
                    ptime,
                    Instant::from_ticks(current_time),
                    connection.is_time_stamp_enabled,
                ) {
                    let rtt_min = self.rtt_filter.rtt_filtered_min;
                    let target_reno = crate::internal::TARGET_RENO_RTT;
                    let target_sat = crate::internal::TARGET_SATELLITE_RTT;

                    if rtt_min > target_reno {
                        let correction = if rtt_min > target_sat {
                            target_sat.ticks() as f64 / rtt_min.ticks() as f64
                        } else {
                            target_reno.ticks() as f64 / rtt_min.ticks() as f64
                        };
                        let base_window = (correction * path_x.cwin as f64) as u64;
                        let delta_window = path_x.cwin - base_window;
                        path_x.cwin -= delta_window / 2;
                    } else {
                        path_x.cwin /= 2;
                    }

                    self.ssthresh = path_x.cwin;
                    self.w_max = path_x.cwin as f64 / path_x.send_mtu as f64;
                    self.w_last_max = self.w_max;
                    self.w_reno = path_x.cwin as f64;
                    path_x.is_ssthresh_initialized = true;

                    self.enter_recovery(connection, path_x, notification, current_time);

                    // Adjust epoch so we enter the test phase immediately.
                    let k_micro = (self.k * 1_000_000.0) as u64;
                    if k_micro > current_time {
                        self.k = current_time as f64 / 1_000_000.0;
                        self.start_of_epoch = 0;
                    } else {
                        self.start_of_epoch = current_time - k_micro;
                    }
                }
            }

            CongestionNotification::SeedCwin
                if self.alg_state == CubicAlgState::SlowStart && self.ssthresh == u64::MAX =>
            {
                if path_x.cwin < ack_state.nb_bytes_acknowledged {
                    path_x.cwin = ack_state.nb_bytes_acknowledged;
                }
                self.ssthresh = ack_state.nb_bytes_acknowledged;
                self.w_max = path_x.cwin as f64 / path_x.send_mtu as f64;
                self.w_last_max = self.w_max;
                self.w_reno = path_x.cwin as f64;
                path_x.is_ssthresh_initialized = true;
                self.enter_avoidance(current_time);
            }

            CongestionNotification::Reset => {
                self.reset(path_x, current_time);
            }

            _ => {}
        }

        // Update pacing data.
        let in_slow_start = self.alg_state == CubicAlgState::SlowStart && self.ssthresh == u64::MAX;
        path_x.update_pacing_data(in_slow_start as i32);
    }

    /// C: `dcubic_exit_slow_start` (picoquic/cubic.c:424)
    ///
    /// Exit slow start in the delay-based CUBIC variant.  Unlike regular
    /// CUBIC, this is only called when the hystart delay signal fires or
    /// when high packet loss is detected — not on every congestion event.
    ///
    /// If slow start has not been explicitly ended before (ssthresh still
    /// `u64::MAX`), the threshold is set from the current window and the
    /// algorithm enters congestion avoidance with the epoch adjusted so
    /// the test phase begins immediately.  Otherwise, if enough time has
    /// elapsed since the start of the epoch or a new packet has been
    /// acknowledged, recovery is re-entered.
    fn dcubic_exit_slow_start(
        &mut self,
        connection: &Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        current_time: u64,
    ) {
        if self.ssthresh == u64::MAX {
            path_x.is_ssthresh_initialized = true;
            self.ssthresh = path_x.cwin;
            self.w_max = path_x.cwin as f64 / path_x.send_mtu as f64;
            self.w_last_max = self.w_max;
            self.w_reno = path_x.cwin as f64;
            self.enter_avoidance(current_time);
            // Apply a correction to enter the test phase immediately.
            let k_micro = (self.k * 1_000_000.0) as u64;
            if k_micro > current_time {
                self.k = current_time as f64 / 1_000_000.0;
                self.start_of_epoch = 0;
            } else {
                self.start_of_epoch = current_time - k_micro;
            }
        } else if current_time.wrapping_sub(self.start_of_epoch) > path_x.smoothed_rtt.ticks()
            || self.recovery_sequence <= connection.ack_number(path_x)
        {
            self.enter_recovery(connection, path_x, notification, current_time);
        }
    }

    /// C: `dcubic_notify` (picoquic/cubic.c:458)
    ///
    /// Delay-based CUBIC congestion-control event handler.  Differs from
    /// [`CubicState::notify`] in that:
    ///
    /// * `Repeat`/`Timeout`: only exits slow start on *high* losses
    ///   (hystart loss test); ignores these events in the Recovery state.
    /// * `RttMeasurement`: uses the hystart RTT test rather than the loss
    ///   test to exit slow start; Recovery and CongestionAvoidance may
    ///   re-enter recovery when the hystart test fires.
    /// * `SpuriousRepeat`/`EcnEc`: silently ignored (unlike regular CUBIC
    ///   which responds to both).
    /// * All other events: delegated to [`CubicState::notify`] and return
    ///   immediately (avoids updating pacing data twice).
    pub fn dcubic_notify(
        &mut self,
        connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: u64,
    ) {
        use crate::Instant;

        path_x.is_cc_data_updated = true;

        match notification {
            CongestionNotification::Repeat | CongestionNotification::Timeout => {
                match self.alg_state {
                    CubicAlgState::SlowStart => {
                        // In contrast to Cubic, only exit on high losses.
                        if self.rtt_filter.hystart_loss_test(
                            notification,
                            ack_state.lost_packet_number,
                            SMOOTHED_LOSS_THRESHOLD,
                        ) {
                            self.dcubic_exit_slow_start(
                                connection,
                                path_x,
                                notification,
                                current_time,
                            );
                        }
                    }
                    CubicAlgState::Recovery => {
                        // Do nothing in recovery.
                    }
                    CubicAlgState::CongestionAvoidance => {
                        // In contrast to Cubic, only exit on high losses.
                        if self.rtt_filter.hystart_loss_test(
                            notification,
                            ack_state.lost_packet_number,
                            SMOOTHED_LOSS_THRESHOLD,
                        ) && ack_state.lost_packet_number > self.recovery_sequence
                        {
                            self.enter_recovery(connection, path_x, notification, current_time);
                        }
                    }
                }
            }

            CongestionNotification::RttMeasurement => {
                match self.alg_state {
                    CubicAlgState::SlowStart => {
                        // Increase window for long-delay RTT if still in
                        // the unconstrained phase.
                        if path_x.rtt_min > TARGET_RENO_RTT && self.ssthresh == u64::MAX {
                            path_x.cwin = path_x.update_cwin_for_long_rtt();
                        }
                        // HyStart RTT-based exit test.
                        let rtt_meas = if connection.is_time_stamp_enabled {
                            ack_state.one_way_delay
                        } else {
                            ack_state.rtt_measurement
                        };
                        let ptime = Instant::from_ticks(
                            connection
                                .paths
                                .first()
                                .map_or(0, |p| p.pacing.packet_time_microsec.ticks()),
                        );
                        if self.rtt_filter.hystart_test(
                            rtt_meas,
                            ptime,
                            Instant::from_ticks(current_time),
                            connection.is_time_stamp_enabled,
                        ) {
                            self.dcubic_exit_slow_start(
                                connection,
                                path_x,
                                notification,
                                current_time,
                            );
                        }
                    }
                    // Recovery falls through to the same hystart logic as
                    // CongestionAvoidance (C `/* continue */` fall-through).
                    CubicAlgState::Recovery | CubicAlgState::CongestionAvoidance => {
                        if matches!(self.alg_state, CubicAlgState::Recovery)
                            && path_x.rtt_min > TARGET_RENO_RTT
                            && self.ssthresh == u64::MAX
                        {
                            path_x.cwin = path_x.update_cwin_for_long_rtt();
                        }
                        let rtt_meas = if connection.is_time_stamp_enabled {
                            ack_state.one_way_delay
                        } else {
                            ack_state.rtt_measurement
                        };
                        let ptime = Instant::from_ticks(
                            connection
                                .paths
                                .first()
                                .map_or(0, |p| p.pacing.packet_time_microsec.ticks()),
                        );
                        if self.rtt_filter.hystart_test(
                            rtt_meas,
                            ptime,
                            Instant::from_ticks(current_time),
                            connection.is_time_stamp_enabled,
                        ) && (current_time.wrapping_sub(self.start_of_epoch)
                            > path_x.smoothed_rtt.ticks()
                            || self.recovery_sequence <= connection.ack_number(path_x))
                        {
                            self.enter_recovery(connection, path_x, notification, current_time);
                        }
                    }
                }
            }

            CongestionNotification::SpuriousRepeat | CongestionNotification::EcnEc => {
                // In contrast to Cubic, do nothing here.
            }

            // All other notifications delegate to the regular cubic handler.
            _ => {
                self.notify(connection, path_x, notification, ack_state, current_time);
                // Return immediately to avoid calculating pacing data twice.
                return;
            }
        }

        // Update pacing data.
        let in_slow_start = self.alg_state == CubicAlgState::SlowStart && self.ssthresh == u64::MAX;
        path_x.update_pacing_data(in_slow_start as i32);
    }
}

// ---------------------------------------------------------------------------

/// C: `cubic_root` (picoquic/cubic.c:83)
///
/// Newton-Raphson cube-root approximation using bit-shift bracketing
/// followed by three Newton steps.  Input `x` is the CUBIC `W_max *
/// (1 − β) / C` value; result is `K`, the time-to-reach-W_max.
fn cubic_root(x: f64) -> f64 {
    let mut v: f64 = 1.0;
    let mut y: f64 = 1.0;

    while v > x * 8.0 {
        v /= 8.0;
        y /= 2.0;
    }
    while v < x {
        v *= 8.0;
        y *= 2.0;
    }
    for _ in 0..3 {
        let y2 = y * y;
        let y3 = y2 * y;
        y += (x - y3) / (3.0 * y2);
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w_cubic_at_epoch_start() {
        let state = CubicState {
            start_of_epoch: 1_000_000,
            k: 0.0,
            w_max: 10.0,
            ..Default::default()
        };
        // At t = start_of_epoch: delta_t_sec = 0 - 0 = 0; W_cubic = 0 + W_max
        let w = state.w_cubic(1_000_000);
        assert!((w - 10.0).abs() < 1e-10);
    }

    #[test]
    fn w_cubic_positive_t() {
        let state = CubicState {
            start_of_epoch: 0,
            k: 0.0,
            w_max: 0.0,
            ..Default::default()
        };
        // delta_t_sec = 1.0; W_cubic = 0.4 * 1^3 + 0 = 0.4
        let w = state.w_cubic(1_000_000);
        assert!((w - 0.4).abs() < 1e-10);
    }

    #[test]
    fn observe_returns_state_and_w_max() {
        let state = CubicState {
            alg_state: CubicAlgState::CongestionAvoidance,
            w_max: 42.7,
            ..Default::default()
        };
        let (cc_state, cc_param) = state.observe();
        assert_eq!(cc_state, CubicAlgState::CongestionAvoidance as u64);
        assert_eq!(cc_param, 42); // truncated cast, same as C (uint64_t)42.7 = 42
    }

    #[test]
    fn cubic_root_cube() {
        // cubic_root(8) ≈ 2.0
        let r = cubic_root(8.0);
        assert!((r - 2.0).abs() < 1e-6);
    }
}
