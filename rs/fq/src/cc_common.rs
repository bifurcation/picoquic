//! Translation of `quic/cc_common.h`.
//!
//! Shared helpers used by every congestion-control module: RTT
//! filtering, HyStart / HyStart++ slow-start exit tests, slow-start
//! window-growth helpers, and the standalone New Reno simulator that
//! several other algorithms run as a lower-bound estimator.
//!
//! Phase 4: all function bodies are implemented.

use crate::internal::{
    CWIN_INITIAL, CWIN_MINIMUM, Connection, Path, TARGET_RENO_RTT, TARGET_SATELLITE_RTT,
};
use crate::{CongestionNotification, Duration, Instant, PerAckState};

// ---------------------------------------------------------------------------
// Tunable constants (`#define`s in the header).

/// Window size of the min/max RTT filter, counted in RTT
/// measurements.  Doubles as the threshold of consecutive
/// RTT-excess measurements that trigger slow-start exit.  Used as
/// an array dimension below, hence `usize`.
pub const MIN_MAX_RTT_SCOPE: usize = 7;

/// Lookback window for the smoothed packet-loss filter, in packets.
pub const SMOOTHED_LOSS_SCOPE: u64 = 32;

/// Per-step decay factor for the smoothed loss-rate EMA.  Equivalent
/// to `1.0 / 16.0`, written explicitly for parity with the C macro.
pub const SMOOTHED_LOSS_FACTOR: f64 = 1.0 / 16.0;

/// Drop-rate threshold above which the volume-based HyStart loss
/// test triggers slow-start exit.
pub const SMOOTHED_LOSS_THRESHOLD: f64 = 0.15;

// HyStart++ tuning constants.  Values come from the IETF HyStart++
// draft (RFC 9406-equivalent); see the C header for the full
// rationale, including why `L` is omitted (quic is always paced).

pub const HYSTART_PP_MIN_RTT_THRESH: u64 = 4_000;
pub const HYSTART_PP_MAX_RTT_THRESH: u64 = 16_000;
pub const HYSTART_PP_MIN_RTT_DIVISOR: u64 = 8;
pub const HYSTART_PP_N_RTT_SAMPLE: u64 = 8;
pub const HYSTART_PP_CSS_GROWTH_DIVISOR: u64 = 4;
pub const HYSTART_PP_CSS_ROUNDS: u64 = 5;

// ---------------------------------------------------------------------------
// RTT filter and HyStart-related state.

/// Rolling min/max RTT filter plus smoothed-loss bookkeeping shared
/// by HyStart exit tests.  C: `picoquic_min_max_rtt_t`.
///
/// Type deviations from C: `is_init` (`int` → `bool`);
/// `sample_current` (`int` → `usize`, used as array index);
/// `nb_rtt_excess` (`int` → `u32`, always non-negative; safety wins).
#[derive(Debug, Clone)]
pub struct MinMaxRtt {
    /// `None` until the first RTT measurement arrives.
    pub last_rtt_sample_time: Option<Instant>,
    pub rtt_filtered_min: Duration,
    pub nb_rtt_excess: u32,
    pub sample_current: usize,
    pub is_init: bool,
    pub smoothed_drop_rate: f64,
    /// Smoothed byte-count EMA stored in fixed-point with an
    /// implicit ×16 multiplier (each step is `prev - prev/16 + new`,
    /// see `picoquic_cc_hystart_loss_volume_test`); the extra range
    /// avoids precision loss in integer EMA arithmetic.
    pub smoothed_bytes_sent_16: u64,
    /// Same fixed-point convention as [`Self::smoothed_bytes_sent_16`].
    pub smoothed_bytes_lost_16: u64,
    pub last_lost_packet_number: u64,
    pub sample_min: Duration,
    pub sample_max: Duration,
    pub samples: [Duration; MIN_MAX_RTT_SCOPE],
}

impl Default for MinMaxRtt {
    fn default() -> Self {
        Self {
            last_rtt_sample_time: None,
            rtt_filtered_min: Duration::from_ticks(0),
            nb_rtt_excess: 0,
            sample_current: 0,
            is_init: false,
            smoothed_drop_rate: 0.0,
            smoothed_bytes_sent_16: 0,
            smoothed_bytes_lost_16: 0,
            last_lost_packet_number: 0,
            sample_min: Duration::from_ticks(0),
            sample_max: Duration::from_ticks(0),
            samples: [Duration::from_ticks(0); MIN_MAX_RTT_SCOPE],
        }
    }
}

impl MinMaxRtt {
    /// Append `rtt` to the rolling sample window and recompute
    /// `sample_min` / `sample_max`.  C: `picoquic_cc_filter_rtt_min_max`.
    pub fn filter_rtt_min_max(&mut self, rtt: Duration) {
        let x = self.sample_current;
        self.samples[x] = rtt;
        self.sample_current = x + 1;
        if self.sample_current >= MIN_MAX_RTT_SCOPE {
            self.is_init = true;
            self.sample_current = 0;
        }
        let x_max = if self.is_init {
            MIN_MAX_RTT_SCOPE
        } else {
            x + 1
        };
        self.sample_min = self.samples[0];
        self.sample_max = self.samples[0];
        for i in 1..x_max {
            if self.samples[i] < self.sample_min {
                self.sample_min = self.samples[i];
            } else if self.samples[i] > self.sample_max {
                self.sample_max = self.samples[i];
            }
        }
    }

    /// HyStart loss-count test: `true` when the smoothed loss rate
    /// has exceeded `error_rate_max` (or a timeout was observed),
    /// signalling that slow start should end.  C:
    /// `picoquic_cc_hystart_loss_test` — the C `int` return is
    /// purely boolean here.
    pub fn hystart_loss_test(
        &mut self,
        event: CongestionNotification,
        lost_packet_number: u64,
        error_rate_max: f64,
    ) -> bool {
        let mut ret = false;
        let mut next_number = self.last_lost_packet_number;

        if lost_packet_number > next_number {
            if next_number + SMOOTHED_LOSS_SCOPE < lost_packet_number {
                next_number = lost_packet_number - SMOOTHED_LOSS_SCOPE;
            }
            while next_number < lost_packet_number {
                self.smoothed_drop_rate *= 1.0 - SMOOTHED_LOSS_FACTOR;
                next_number += 1;
            }
            self.smoothed_drop_rate += (1.0 - self.smoothed_drop_rate) * SMOOTHED_LOSS_FACTOR;
            self.last_lost_packet_number = lost_packet_number;

            match event {
                CongestionNotification::Repeat => {
                    ret = self.smoothed_drop_rate > error_rate_max;
                }
                CongestionNotification::Timeout => {
                    ret = true;
                }
                _ => {}
            }
        }
        ret
    }

    /// HyStart loss-volume test, parallel to [`Self::hystart_loss_test`]
    /// but driven by byte counts rather than packet sequence numbers.
    /// C: `picoquic_cc_hystart_loss_volume_test`.
    pub fn hystart_loss_volume_test(
        &mut self,
        event: CongestionNotification,
        nb_bytes_newly_acked: u64,
        nb_bytes_newly_lost: u64,
    ) -> bool {
        self.smoothed_bytes_lost_16 -= self.smoothed_bytes_lost_16 / 16;
        self.smoothed_bytes_lost_16 += nb_bytes_newly_lost;
        self.smoothed_bytes_sent_16 -= self.smoothed_bytes_sent_16 / 16;
        self.smoothed_bytes_sent_16 += nb_bytes_newly_acked + nb_bytes_newly_lost;

        if self.smoothed_bytes_sent_16 > 0 {
            self.smoothed_drop_rate =
                self.smoothed_bytes_lost_16 as f64 / self.smoothed_bytes_sent_16 as f64;
        } else {
            self.smoothed_drop_rate = 0.0;
        }

        match event {
            CongestionNotification::Acknowledgement => {
                self.smoothed_drop_rate > SMOOTHED_LOSS_THRESHOLD
            }
            CongestionNotification::Timeout => true,
            _ => false,
        }
    }

    /// HyStart RTT-rise test: `true` when the filtered RTT has grown
    /// enough above its minimum to call slow-start over.
    /// C: `picoquic_cc_hystart_test`.  `is_one_way_delay_enabled`
    /// was an `int` in C; promoted to `bool`.
    pub fn hystart_test(
        &mut self,
        rtt_measurement: Duration,
        packet_time: Instant,
        current_time: Instant,
        is_one_way_delay_enabled: bool,
    ) -> bool {
        if is_one_way_delay_enabled && rtt_measurement == Duration::from_ticks(0) {
            return false;
        }

        let threshold = match self.last_rtt_sample_time {
            None => true,
            Some(t) => current_time > t + Duration::from_ticks(1000),
        };

        if !threshold {
            return false;
        }

        self.filter_rtt_min_max(rtt_measurement);
        self.last_rtt_sample_time = Some(current_time);

        if !self.is_init {
            return false;
        }

        let mut ret = false;
        if self.rtt_filtered_min == Duration::from_ticks(0)
            || self.rtt_filtered_min > self.sample_max
        {
            self.rtt_filtered_min = self.sample_max;
        }

        let delta_max = {
            let q = self.rtt_filtered_min / 4;
            // packet_time is an Instant; treat it as a Duration value
            // for the comparison (both are ticks-based u64 wrappers)
            let pt = Duration::from_ticks(packet_time.ticks());
            if q < pt { pt } else { q }
        };

        if self.sample_min > self.rtt_filtered_min {
            if self.sample_min > self.rtt_filtered_min + delta_max {
                self.nb_rtt_excess += 1;
                if self.nb_rtt_excess >= MIN_MAX_RTT_SCOPE as u32 {
                    ret = true;
                }
            }
        } else {
            self.nb_rtt_excess = 0;
        }

        ret
    }
}

// ---------------------------------------------------------------------------
// Helpers that read QUIC connection and path state.
//
// These are pure reads in the C source, so the receivers are `&self`
// rather than `&mut self`.  Each was a free function whose primary
// argument is a connection or path; the surface lifts into two
// capability traits (one rooted on `Connection`, one on `Path`) so
// callers opt into them with `use crate::cc_common::{ConnectionCc,
// PathCc}` and the `cc_*` prefix on the C names drops — the trait
// names already say "this is the cc surface".

/// Connection-rooted congestion-control read accessors.  C: the
/// `picoquic_cc_get_*` family that takes a connection plus a path.
pub trait ConnectionCc {
    /// Next-to-send packet sequence number for the relevant packet
    /// context — per-path under multipath, otherwise the connection's
    /// application context.  C: `picoquic_cc_get_sequence_number`.
    fn sequence_number(&self, path_x: &Path) -> u64;

    /// Highest acknowledged packet sequence number for the relevant
    /// packet context.  C: `picoquic_cc_get_ack_number`.
    fn ack_number(&self, path_x: &Path) -> u64;

    /// Wall-clock time at which the most recent ACK was received for
    /// the relevant packet context.  C: `picoquic_cc_get_ack_sent_time`.
    fn ack_sent_time(&self, path_x: &Path) -> Instant;
}

impl ConnectionCc for Connection {
    fn sequence_number(&self, path_x: &Path) -> u64 {
        if self.is_multipath_enabled {
            path_x.pkt_ctx.send_sequence
        } else {
            self.pkt_ctx[crate::PacketContext::Application as usize].send_sequence
        }
    }

    fn ack_number(&self, path_x: &Path) -> u64 {
        if self.is_multipath_enabled {
            path_x.pkt_ctx.highest_acknowledged
        } else {
            self.pkt_ctx[crate::PacketContext::Application as usize].highest_acknowledged
        }
    }

    fn ack_sent_time(&self, path_x: &Path) -> Instant {
        if self.is_multipath_enabled {
            path_x.pkt_ctx.latest_time_acknowledged
        } else {
            self.pkt_ctx[crate::PacketContext::Application as usize].latest_time_acknowledged
        }
    }
}

/// Path-rooted congestion-control read accessors.  C: the
/// `picoquic_cc_*` family that takes a path (and reaches the
/// connection through the path's back-pointer when needed).
pub trait PathCc {
    /// Lowest sequence number not yet acknowledged on this path: the
    /// pending-list head if any, else `highest_acknowledged + 1`.
    /// C: `picoquic_cc_get_lowest_not_ack`.
    fn lowest_not_ack(&self) -> u64;

    /// Bytes to add to CWIN while in classic slow start.  Returns
    /// `nb_delivered` if the path is currently CWIN-blocked, else
    /// zero (no growth without back-pressure).
    /// C: `picoquic_cc_slow_start_increase`.
    fn slow_start_increase(&self, nb_delivered: u64) -> u64;

    /// Bytes to add to CWIN, with HyStart++ Conservative Slow Start
    /// support: when `in_css` is true, growth is divided by
    /// [`HYSTART_PP_CSS_GROWTH_DIVISOR`].
    /// C: `picoquic_cc_slow_start_increase_ex`.
    fn slow_start_increase_ex(&self, nb_delivered: u64, in_css: bool) -> u64;

    /// Bytes to add to CWIN, with Prague-style ECN damping.
    /// `prague_alpha` is an integer fraction over 1024 (so `0` means
    /// no ECN signal and the call falls back to
    /// [`Self::slow_start_increase_ex`]).
    /// C: `picoquic_cc_slow_start_increase_ex2`.
    fn slow_start_increase_ex2(&self, nb_delivered: u64, in_css: bool, prague_alpha: u64) -> u64;

    /// Bandwidth-derived target CWIN: returns the half-BDP estimate
    /// if it exceeds the current CWIN, otherwise the current CWIN.
    /// C: `picoquic_cc_update_target_cwin_estimation`.
    fn update_target_cwin_estimation(&self) -> u64;

    /// CWIN floor for long-RTT paths: scales `CWIN_INITIAL` by the
    /// path's `rtt_min` (capped at the satellite RTT target).
    /// Returns the floor if it exceeds the current CWIN, otherwise
    /// the current CWIN.  C: `picoquic_cc_update_cwin_for_long_rtt`.
    fn update_cwin_for_long_rtt(&self) -> u64;
}

impl PathCc for Path {
    fn lowest_not_ack(&self) -> u64 {
        // C reads cnx->pkt_ctx[app] for single-path, path->pkt_ctx for multipath.
        // Path has no back-pointer to Connection, so we always use path->pkt_ctx
        // (exact for multipath; conservative approximation for single-path).
        self.pkt_ctx
            .pending
            .keys()
            .next()
            .copied()
            .unwrap_or(self.pkt_ctx.highest_acknowledged + 1)
    }

    fn slow_start_increase(&self, nb_delivered: u64) -> u64 {
        // C body checks cnx->cwin_blocked.  Path has no back-pointer to
        // Connection, so approximate with bytes_in_transit >= cwin, which is
        // the condition that sets cwin_blocked in the C library.
        if self.bytes_in_transit < self.cwin {
            0
        } else {
            nb_delivered
        }
    }

    fn slow_start_increase_ex(&self, nb_delivered: u64, in_css: bool) -> u64 {
        if in_css {
            self.slow_start_increase(nb_delivered / HYSTART_PP_CSS_GROWTH_DIVISOR)
        } else {
            self.slow_start_increase(nb_delivered)
        }
    }

    fn slow_start_increase_ex2(&self, nb_delivered: u64, in_css: bool, prague_alpha: u64) -> u64 {
        if prague_alpha != 0 {
            let delta = if self.smoothed_rtt <= TARGET_RENO_RTT {
                nb_delivered * (1024 - prague_alpha) / 1024
            } else {
                nb_delivered * self.smoothed_rtt.ticks() * (1024 - prague_alpha)
                    / TARGET_RENO_RTT.ticks()
                    / 1024
            };
            self.slow_start_increase_ex(delta, in_css)
        } else {
            self.slow_start_increase_ex(nb_delivered, in_css)
        }
    }

    fn update_target_cwin_estimation(&self) -> u64 {
        // BYTES_FROM_RATE(smoothed_rtt, peak_bandwidth_estimate) = rtt_us * bps / 1_000_000
        let max_win = self.smoothed_rtt.ticks() * self.peak_bandwidth_estimate / 1_000_000;
        let min_win = max_win / 2;
        if min_win > self.cwin {
            min_win
        } else {
            self.cwin
        }
    }

    fn update_cwin_for_long_rtt(&self) -> u64 {
        let rtt_cap = if self.rtt_min > TARGET_SATELLITE_RTT {
            TARGET_SATELLITE_RTT
        } else {
            self.rtt_min
        };
        let min_cwnd =
            (CWIN_INITIAL as f64 * rtt_cap.ticks() as f64 / TARGET_RENO_RTT.ticks() as f64) as u64;
        if min_cwnd > self.cwin {
            min_cwnd
        } else {
            self.cwin
        }
    }
}

// ---------------------------------------------------------------------------
// Embedded New Reno simulator.
//
// Several congestion controllers run a parallel New Reno instance to
// derive a lower bound on the congestion window or minimum
// bandwidth.  This simulator does not touch the connection or path
// state directly; everything lives in `NewRenoSimState`.

/// Internal phase of the embedded New Reno simulator.
/// C: `picoquic_newreno_alg_state_t`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum NewRenoAlgState {
    #[default]
    SlowStart,
    CongestionAvoidance,
}

/// Simulator state for the embedded New Reno instance.
/// C: `picoquic_newreno_sim_state_t`.
#[derive(Debug, Clone, Default)]
pub struct NewRenoSimState {
    pub alg_state: NewRenoAlgState,
    pub cwin: u64,
    pub residual_ack: u64,
    pub ssthresh: u64,
    pub recovery_start: u64,
    pub recovery_sequence: u64,
}

impl NewRenoSimState {
    /// Reset the simulator to its initial state.
    /// C: `picoquic_newreno_sim_reset`.
    pub fn reset(&mut self) {
        *self = Self::default();
        self.ssthresh = u64::MAX;
        self.cwin = CWIN_INITIAL;
    }

    /// Drive the simulator with a congestion-control event.
    ///
    /// `connection` and `path_x` are read-only here: the C body only mutates
    /// `self`, reading the connection and path to resolve sequence
    /// numbers and timestamps via the `cc_*` accessors.  C:
    /// `picoquic_newreno_sim_notify`.
    pub fn notify(
        &mut self,
        connection: &Connection,
        path_x: &Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: Instant,
    ) {
        match notification {
            CongestionNotification::Acknowledgement => match self.alg_state {
                NewRenoAlgState::SlowStart => {
                    self.cwin += ack_state.nb_bytes_acknowledged;
                    if self.cwin >= self.ssthresh {
                        self.alg_state = NewRenoAlgState::CongestionAvoidance;
                    }
                }
                NewRenoAlgState::CongestionAvoidance => {
                    let complete_delta = ack_state.nb_bytes_acknowledged * path_x.send_mtu as u64
                        + self.residual_ack;
                    self.residual_ack = complete_delta % self.cwin;
                    self.cwin += complete_delta / self.cwin;
                }
            },
            CongestionNotification::EcnEc
            | CongestionNotification::Repeat
            | CongestionNotification::Timeout
                if self.recovery_sequence <= ack_state.lost_packet_number =>
            {
                self.enter_recovery(connection, path_x, notification, current_time);
            }
            CongestionNotification::EcnEc
            | CongestionNotification::Repeat
            | CongestionNotification::Timeout => {}
            CongestionNotification::SpuriousRepeat => {
                if !connection.is_multipath_enabled {
                    if current_time.ticks() - self.recovery_start < path_x.smoothed_rtt.ticks()
                        && self.recovery_sequence > connection.ack_number(path_x)
                        && self.ssthresh != u64::MAX
                        && self.cwin < 2 * self.ssthresh
                    {
                        self.cwin = 2 * self.ssthresh;
                        self.alg_state = NewRenoAlgState::CongestionAvoidance;
                    }
                } else if current_time.ticks() - self.recovery_start < path_x.smoothed_rtt.ticks()
                    && self.recovery_start > connection.ack_sent_time(path_x).ticks()
                    && self.ssthresh != u64::MAX
                    && self.cwin < 2 * self.ssthresh
                {
                    self.cwin = 2 * self.ssthresh;
                    self.alg_state = NewRenoAlgState::CongestionAvoidance;
                }
            }
            CongestionNotification::Reset => {
                self.reset();
            }
            CongestionNotification::SeedCwin => {
                self.seed_cwin(ack_state.nb_bytes_acknowledged);
            }
            _ => {}
        }
    }

    fn enter_recovery(
        &mut self,
        connection: &Connection,
        path_x: &Path,
        notification: CongestionNotification,
        current_time: Instant,
    ) {
        self.ssthresh = self.cwin / 2;
        if self.ssthresh < CWIN_MINIMUM {
            self.ssthresh = CWIN_MINIMUM;
        }
        if notification == CongestionNotification::Timeout {
            self.cwin = CWIN_MINIMUM;
            self.alg_state = NewRenoAlgState::SlowStart;
        } else {
            self.cwin = self.ssthresh;
            self.alg_state = NewRenoAlgState::CongestionAvoidance;
        }
        self.recovery_start = current_time.ticks();
        self.recovery_sequence = connection.sequence_number(path_x);
        self.residual_ack = 0;
    }

    fn seed_cwin(&mut self, seed_cwin: u64) {
        if self.alg_state == NewRenoAlgState::SlowStart
            && self.ssthresh == u64::MAX
            && seed_cwin > self.cwin
        {
            self.cwin = seed_cwin;
            self.ssthresh = seed_cwin;
            self.alg_state = NewRenoAlgState::CongestionAvoidance;
        }
    }
}

#[cfg(test)]
mod test {}
