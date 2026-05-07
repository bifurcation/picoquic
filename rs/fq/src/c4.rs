//! Translation of `picoquic/c4.c` — C4 congestion control.

use crate::cc_common::{ConnectionCc, PathCc, SMOOTHED_LOSS_FACTOR, SMOOTHED_LOSS_SCOPE};
use crate::internal::{CWIN_INITIAL, Connection, Path};
use crate::utils::{bytes_from_rate, rate_from_bytes};
use crate::{CongestionNotification, PacketContext, PerAckState, State};

// ---------------------------------------------------------------------------
// Constants from c4.c `#define`s.

const C4_ALPHA_PUSH_LOW_1024: u64 = 1088; // C: C4_ALPHA_PUSH_LOW_1024 (106.25%)
const C4_ALPHA_PUSH_VERY_LOW_1024: u64 = 1056; // C: C4_ALPHA_PUSH_VERY_LOW_1024 (103.125%)
const C4_ALPHA_PUSH_1024: u64 = 1280; // C: C4_ALPHA_PUSH_1024 (125%)
/// 100%.  C: `C4_ALPHA_NEUTRAL_1024`.
const C4_ALPHA_NEUTRAL_1024: u64 = 1024;
/// 87.50%.  C: `C4_ALPHA_RECOVER2_1024`.
const C4_ALPHA_RECOVER2_1024: u64 = 896;
/// 100%.  C: `C4_ALPHA_CRUISE_1024`.
const C4_ALPHA_CRUISE_1024: u64 = 1024;
/// 25% congestion back-off.  C: `C4_BETA_LOSS_1024`.
const C4_BETA_LOSS_1024: u64 = 256;
/// Loss count before exiting initial.  C: `C4_NB_PACKETS_BEFORE_LOSS`.
const C4_NB_PACKETS_BEFORE_LOSS: u64 = 20;
/// Cruise eras before attempting a push.  C: `C4_NB_CRUISE_BEFORE_PUSH`.
const C4_NB_CRUISE_BEFORE_PUSH: u64 = 4;
/// RTT margin for CWIN buffer (15 ms).  C: `C4_RTT_MARGIN_DELAY`.
const C4_RTT_MARGIN_DELAY: u64 = 15_000;
/// Minimum nominal max-RTT (1 ms).  C: `C4_MAX_RTT_MIN`.
const C4_MAX_RTT_MIN: u64 = 1_000;
/// Max jitter cap on era max-RTT (250 ms).  C: `C4_MAX_JITTER`.
const C4_MAX_JITTER: u64 = 250_000;
/// Maximum probe level before re-entering Initial.  C: `C4_PROBE_LEVEL_MAX`.
const C4_PROBE_LEVEL_MAX: i32 = 3;
/// Push-alpha table indexed by probe level.  C: `c4_push_rate_by_probe_level`.
const C4_PUSH_RATE_BY_PROBE_LEVEL: [u64; 4] = [
    C4_ALPHA_PUSH_VERY_LOW_1024,
    C4_ALPHA_PUSH_LOW_1024,
    C4_ALPHA_PUSH_1024,
    C4_ALPHA_PUSH_1024,
];
/// Gain parameter for the ECN alpha EWMA: g = 1/2^4.  C: `C4_ECN_SHIFT_G`.
const C4_ECN_SHIFT_G: u32 = 4;
/// Maximum delay threshold in microseconds (25 ms).  C: `C4_DELAY_THRESHOLD_MAX`.
const C4_DELAY_THRESHOLD_MAX: u64 = 25_000;
/// Default probe level on entry to Initial.  C: `C4_PROBE_LEVEL_DEFAULT`.
const C4_PROBE_LEVEL_DEFAULT: i32 = 1;
/// Alpha multiplier during Initial slow-start (200%).  C: `C4_ALPHA_INITIAL`.
const C4_ALPHA_INITIAL: u64 = 2048;
/// Alpha multiplier during Recovery (93.75%).  C: `C4_ALPHA_RECOVER_1024`.
const C4_ALPHA_RECOVER_1024: u64 = 960;

// ---------------------------------------------------------------------------
// `MULT1024(c, v)` — fixed-point scale by c/1024.  C: `MULT1024`.

#[inline]
fn mult1024(c: u64, v: u64) -> u64 {
    (v * c) >> 10
}

// ---------------------------------------------------------------------------
// Algorithm-state enum.  C: `c4_alg_state_t`.

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum C4AlgState {
    #[default]
    Initial = 0,
    Recovery,
    Cruising,
    Pushing,
}

// ---------------------------------------------------------------------------
// Congestion-event discriminant.  C: `c4_congestion_t`.

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum C4Congestion {
    #[default]
    None = 0,
    Delay,
    Ecn,
    Loss,
}

// ---------------------------------------------------------------------------
// C4 algorithm state.  C: `c4_state_t` (picoquic/c4.c:143).
//
// Type deviations from C:
//   * single-bit `unsigned int x : 1` bitfields → `bool`
//   * `const char* option_string` → `Option<String>` (owned copy)

pub struct C4State {
    pub alg_state: C4AlgState,
    pub nominal_rate: u64,
    pub nominal_max_rtt: u64,
    pub initial_cwnd: u64,
    pub running_min_rtt: u64,
    pub alpha_1024_current: u64,
    pub alpha_1024_previous: u64,
    pub nb_packets_in_startup: u64,
    pub era_sequence: u64,
    pub nb_cruise_left_before_push: u64,
    pub seed_cwin: u64,
    pub seed_rate: u64,
    pub probe_level: i32,
    pub nb_eras_no_increase: i32,
    pub push_rate_old: u64,
    pub push_alpha: u64,
    pub era_max_rtt: u64,
    pub era_min_rtt: u64,
    pub delay_threshold: u64,
    pub recent_delay_excess: u64,
    pub last_lost_packet_number: u64,
    pub smoothed_drop_rate: f64,
    pub ecn_alpha: u64,
    pub ecn_ect1: u64,
    pub ecn_ce: u64,
    pub ecn_threshold: u64,
    pub congestion_notified: bool,
    pub push_was_not_limited: bool,
    pub use_seed_cwin: bool,
    pub initial_after_jitter: bool,
    pub excess_ce_after_push: bool,
    pub option_string: Option<String>,
}

impl C4State {
    /// C: `c4_sensitivity_1024` (picoquic/c4.c:244)
    ///
    /// Fixed-point sensitivity in [0, 1024] scaled to `nominal_rate`:
    /// 0 below 50 kbps, 1024 above 10 Mbps, linearly interpolated in between.
    fn sensitivity_1024(&self) -> u64 {
        if self.nominal_rate < 50_000 {
            0
        } else if self.nominal_rate > 10_000_000 {
            1024
        } else if self.nominal_rate < 1_000_000 {
            (self.nominal_rate - 50_000) * 963 / 950_000
        } else {
            963 + ((self.nominal_rate - 1_000_000) * 61 / 9_000_000)
        }
    }

    /// C: `c4_growth_evaluate` (picoquic/c4.c:429)
    ///
    /// Returns `true` when the previous era produced meaningful throughput
    /// growth.  If `push_alpha` was large enough to measure reliably, growth
    /// is defined as exceeding the alpha-scaled old rate; otherwise it falls
    /// back to a simple rate comparison guarded by the absence of congestion.
    fn growth_evaluate(&self) -> bool {
        if self.push_alpha > C4_ALPHA_PUSH_LOW_1024 {
            let target_rate =
                (3 * self.push_rate_old + mult1024(self.push_alpha, self.push_rate_old)) / 4;
            self.nominal_rate > target_rate
        } else {
            self.nominal_rate > self.push_rate_old && !self.congestion_notified
        }
    }

    /// C: `c4_growth_reset` (picoquic/c4.c:449)
    fn growth_reset(&mut self) {
        self.congestion_notified = false;
        self.push_was_not_limited = false;
        self.push_rate_old = self.nominal_rate;
        // push_alpha is reset to current alpha here; caller sets it to correct
        // value when entering push state.
        self.push_alpha = self.alpha_1024_current;
    }

    /// C: `c4_set_options` (picoquic/c4.c:499)
    ///
    /// Parses `option_string` for recognized option characters.  No option
    /// characters are currently defined; the loop exits on the first unknown.
    fn set_options(&mut self) {
        if let Some(s) = self.option_string.as_deref() {
            // No recognized option characters are currently defined.  The C switch
            // hits `default: ended = 1` for every character, so the loop exits
            // immediately after consuming the first character.
            let _ = s.chars().next();
        }
    }

    /// C: `c4_seed_cwin` (picoquic/c4.c:527)
    pub fn seed_cwin(&mut self, bytes_in_flight: u64) {
        if self.alg_state == C4AlgState::Initial {
            self.use_seed_cwin = true;
            self.seed_cwin = bytes_in_flight;
        }
    }

    /// C: `c4_observe` (picoquic/c4.c:1121)
    ///
    /// Returns `(cc_state, cc_param)`: the algorithm-state discriminant and
    /// `nominal_max_rtt`, matching the C convention of filling two `uint64_t*`
    /// out-params.
    pub fn observe(&self) -> (u64, u64) {
        (self.alg_state as u64, self.nominal_max_rtt)
    }

    /// C: `c4_update_loss_rate` (picoquic/c4.c:301)
    ///
    /// Advances the smoothed drop-rate EMA to account for packets up to and
    /// including `lost_packet_number`.  Each step between the last recorded
    /// loss and the new one decays the rate; the final step adds one loss
    /// event.  Steps beyond `SMOOTHED_LOSS_SCOPE` behind the new number are
    /// skipped to cap the decay loop.
    pub fn update_loss_rate(&mut self, lost_packet_number: u64) {
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
        }
    }

    /// ECN alpha EWMA core.  C: `c4_update_ecn_alpha` (picoquic/c4.c:321) body.
    ///
    /// Takes the current totals directly so the logic can be tested without
    /// constructing `Path` / `Connection`.  Called by `update_ecn_alpha`.
    fn ecn_alpha_update_counts(&mut self, ecn_ect1_remote: u64, ecn_ce_remote: u64) {
        let delta_ect1 = ecn_ect1_remote as i64 - self.ecn_ect1 as i64;
        let delta_ce = ecn_ce_remote as i64 - self.ecn_ce as i64;

        self.ecn_ect1 = ecn_ect1_remote;
        self.ecn_ce = ecn_ce_remote;

        if delta_ce > 0 || delta_ect1 > 0 {
            let sum = delta_ce + delta_ect1;
            let frac: u64 = if sum > 0 {
                ((delta_ce * 1024) / sum) as u64
            } else {
                0
            };

            if frac > self.ecn_alpha && frac >= 512 {
                self.ecn_alpha = frac;
            } else {
                // EWMA: alpha = alpha*(1 - 1/2^shift) + frac*(1/2^shift)
                let alpha_shifted = (self.ecn_alpha << C4_ECN_SHIFT_G) - self.ecn_alpha + frac;
                self.ecn_alpha = alpha_shifted >> C4_ECN_SHIFT_G;
            }
        }
    }

    /// C: `c4_delay_threshold` (picoquic/c4.c:266)
    ///
    /// Compute the delay threshold for declaring congestion.  Sensitivity
    /// is 0 at ≤ 50 kbps, 1024 at ≥ 10 Mbps, and linearly interpolated
    /// in between.  The resulting fraction scales `nominal_max_rtt`, giving
    /// a threshold in [64/1024 × rtt, 260/1024 × rtt], capped at
    /// [`C4_DELAY_THRESHOLD_MAX`] (25 ms).
    pub fn delay_threshold(&self) -> u64 {
        let sensitivity = self.sensitivity_1024();
        let fraction = 64 + mult1024(1024 - sensitivity, 196);
        let delay = mult1024(fraction, self.nominal_max_rtt);
        delay.min(C4_DELAY_THRESHOLD_MAX)
    }

    /// C: `c4_update_ecn_alpha` (picoquic/c4.c:321)
    ///
    /// Selects the right packet context (path-scoped under multipath;
    /// connection application context otherwise) and delegates to
    /// `ecn_alpha_update_counts`.
    #[allow(dead_code)]
    pub(crate) fn update_ecn_alpha(&mut self, path_x: &Path, connection: &Connection) {
        let pkt_ctx = if connection.is_multipath_enabled {
            &path_x.pkt_ctx
        } else {
            &connection.pkt_ctx[PacketContext::Application as usize]
        };
        self.ecn_alpha_update_counts(pkt_ctx.ecn_ect1_total_remote, pkt_ctx.ecn_ce_total_remote);
    }

    /// C: `c4_ecn_threshold` (picoquic/c4.c:281)
    ///
    /// Computes the ECN marking fraction (over 1024) above which a CE-mark
    /// event triggers congestion notification.  At low sensitivity the
    /// threshold is high (~192/1024 ≈ 18.75%); at maximum sensitivity it
    /// falls to ~96/1024 ≈ 9.375%.
    pub fn ecn_threshold(&self) -> u64 {
        let sensitivity = self.sensitivity_1024();
        192 - mult1024(sensitivity, 96)
    }

    /// C: `c4_era_check` (picoquic/c4.c:463)
    ///
    /// Returns `true` when the current measurement era is complete: the
    /// connection must be in the Ready state, and the lowest unacknowledged
    /// sequence number must have advanced past `era_sequence`.
    #[allow(dead_code)]
    fn era_check(&self, path_x: &Path, connection: &Connection) -> bool {
        if connection.connection_state < State::Ready {
            return false;
        }
        path_x.lowest_not_ack() > self.era_sequence
    }

    /// C: `c4_era_reset` (picoquic/c4.c:475)
    ///
    /// Begins a new measurement era: advances `era_sequence` to the current
    /// send sequence number, resets per-era RTT accumulators, snapshots
    /// `alpha_1024_current` into `alpha_1024_previous`, and updates the ECN
    /// alpha from the latest packet-context counters.
    #[allow(dead_code)]
    fn era_reset(&mut self, path_x: &Path, connection: &Connection) {
        self.era_sequence = connection.sequence_number(path_x);
        self.era_max_rtt = 0;
        self.era_min_rtt = u64::MAX;
        self.alpha_1024_previous = self.alpha_1024_current;
        self.update_ecn_alpha(path_x, connection);
    }

    /// C: `c4_enter_initial` (picoquic/c4.c:486)
    ///
    /// Transitions to the Initial (slow-start) state: captures the current
    /// CWIN, sets probe level and alpha to their startup defaults, resets era
    /// and growth tracking, and zeroes the ECN alpha.
    #[allow(dead_code)]
    fn enter_initial(&mut self, path_x: &Path, connection: &Connection) {
        self.alg_state = C4AlgState::Initial;
        self.initial_cwnd = path_x.cwin;
        self.probe_level = C4_PROBE_LEVEL_DEFAULT;
        self.alpha_1024_current = C4_ALPHA_INITIAL;
        self.nb_packets_in_startup = 0;
        self.era_reset(path_x, connection);
        self.nb_eras_no_increase = 0;
        self.ecn_alpha = 0;
        self.growth_reset();
    }

    /// C: `c4_enter_recovery` (picoquic/c4.c:649)
    ///
    /// Transitions to the Recovery state.  If entering from Initial, first
    /// calls `growth_reset` so the growth comparison starts from a clean
    /// baseline.  Multiple simultaneous congestion signals are collapsed: if
    /// already in Recovery this is a no-op.
    fn enter_recovery(&mut self, path_x: &Path, connection: &Connection, c_mode: C4Congestion) {
        if self.alg_state == C4AlgState::Initial {
            self.growth_reset();
        }
        if self.alg_state != C4AlgState::Recovery {
            self.excess_ce_after_push = c_mode == C4Congestion::Ecn;
            self.alg_state = C4AlgState::Recovery;
            self.era_reset(path_x, connection);
            self.alpha_1024_current = C4_ALPHA_RECOVER_1024;
        }
    }

    /// C: `c4_loss_threshold` (picoquic/c4.c:290)
    ///
    /// Returns the smoothed-loss-rate threshold above which a loss event
    /// triggers a congestion notification.  Ranges from ~52% at low rates
    /// (sensitivity ≈ 0) to ~2% at maximum sensitivity.
    pub fn loss_threshold(&self) -> f64 {
        let sensitivity = self.sensitivity_1024();
        let fraction = sensitivity as f64 / 1024.0;
        0.02 + 0.50 * (1.0 - fraction)
    }

    /// C: `c4_enter_cruise` (picoquic/c4.c:715)
    ///
    /// Transitions to the Cruising state.  Resets the era, clears the seed
    /// CWIN flag, calculates how many cruise eras remain before the next
    /// push, and sets `alpha_1024_current`.  For very low-RTT paths (< 1 ms)
    /// pacing is loosened by an extra 48/1024 ≈ 4.7%.
    fn enter_cruise(&mut self, path_x: &mut Path, connection: &Connection) {
        self.era_reset(path_x, connection);
        self.use_seed_cwin = false;

        if self.probe_level > C4_PROBE_LEVEL_DEFAULT {
            self.nb_cruise_left_before_push = 0;
        } else if self.nb_cruise_left_before_push == 0 {
            self.nb_cruise_left_before_push = if self.probe_level == 0 {
                1
            } else {
                C4_NB_CRUISE_BEFORE_PUSH
            };
        }

        self.alpha_1024_current = C4_ALPHA_CRUISE_1024;
        if path_x.smoothed_rtt.ticks() < C4_MAX_RTT_MIN {
            self.alpha_1024_current += 48;
        }
        self.alg_state = C4AlgState::Cruising;
    }

    /// C: `c4_exit_initial` (picoquic/c4.c:535)
    ///
    /// Exits the Initial slow-start phase.  Derives `nominal_max_rtt` from
    /// the half-CWIN / rate ratio, clamps it to `C4_MAX_RTT_MIN`, updates
    /// the delay threshold, resets era counters, and enters Recovery with no
    /// congestion signal (used as the first cruise entry point).
    fn exit_initial(&mut self, path_x: &mut Path, connection: &Connection) {
        let ssthresh = self.initial_cwnd / 2;
        if let Some(nominal_max_rtt) = (ssthresh * 1_000_000).checked_div(self.nominal_rate) {
            self.nominal_max_rtt = nominal_max_rtt;
            if self.nominal_max_rtt < C4_MAX_RTT_MIN {
                self.nominal_max_rtt = C4_MAX_RTT_MIN;
            }
            self.delay_threshold = self.delay_threshold();
            self.nb_eras_no_increase = 0;
            self.probe_level = C4_PROBE_LEVEL_DEFAULT;
            self.enter_recovery(path_x, connection, C4Congestion::None);
        }
    }

    /// C: `c4_exit_recovery` (picoquic/c4.c:674)
    ///
    /// Exits Recovery.  Evaluates whether the previous push produced growth
    /// and adjusts `probe_level` accordingly: successful growth increments it
    /// (unless ECN excess was flagged); a rate-limited failure resets it to 1
    /// (0 under ECN excess).  After resetting growth tracking and clearing
    /// transient state, transitions to Initial if probe_level has exceeded the
    /// maximum, or to Cruising otherwise.
    fn exit_recovery(&mut self, path_x: &mut Path, connection: &Connection) {
        let is_growing = self.growth_evaluate();
        if is_growing {
            if !self.excess_ce_after_push {
                self.probe_level += 1;
            }
        } else if self.push_was_not_limited {
            self.probe_level = 1;
            if self.excess_ce_after_push {
                self.probe_level = 0;
            }
        }
        self.growth_reset();
        self.recent_delay_excess = 0;
        self.smoothed_drop_rate = 0.0;
        self.ecn_alpha = 0;

        if self.probe_level > C4_PROBE_LEVEL_MAX {
            self.enter_initial(path_x, connection);
        } else {
            self.enter_cruise(path_x, connection);
        }
    }

    /// C: `c4_initial_handle_ack` (picoquic/c4.c:576)
    ///
    /// Handles an ACK during Initial slow-start.  Grows `initial_cwnd` by the
    /// bytes acknowledged (with a right-shift damper proportional to how many
    /// eras without rate increase have been observed — similar to HyStart++).
    /// At each era boundary, checks whether growth is still happening; if not,
    /// increments `nb_eras_no_increase`.  After three eras without growth,
    /// or immediately if the seed-rate has been validated, exits Initial.
    fn initial_handle_ack(
        &mut self,
        path_x: &mut Path,
        connection: &Connection,
        ack_state: &PerAckState,
    ) {
        self.nb_packets_in_startup += 1;
        self.initial_cwnd +=
            ack_state.nb_bytes_acknowledged >> (3 * self.nb_eras_no_increase as u32);
        if self.use_seed_cwin && self.seed_rate > 0 && self.nominal_rate >= self.seed_rate {
            self.use_seed_cwin = false;
        }
        if self.era_check(path_x, connection) {
            let is_growing = self.growth_evaluate();
            if is_growing {
                self.nb_eras_no_increase = 0;
            } else if self.push_was_not_limited && self.nominal_rate > 0 {
                self.nb_eras_no_increase += 1;
            }
            self.era_reset(path_x, connection);
            if self.nb_eras_no_increase >= 3 {
                self.exit_initial(path_x, connection);
            } else {
                self.growth_reset();
            }
        }
    }

    /// C: `c4_apply_rate_and_cwin` (picoquic/c4.c:359)
    ///
    /// Computes and applies the pacing rate and CWIN from the current alpha
    /// and nominal-rate parameters.  In Initial state the CWIN is floored to
    /// `initial_cwnd` and may incorporate the seed CWIN or peak bandwidth
    /// estimate.  In other states an RTT-margin buffer and an extra Pushing
    /// padding are added.
    fn apply_rate_and_cwin(&mut self, path_x: &mut Path) {
        let mut pacing_rate = mult1024(self.alpha_1024_current, self.nominal_rate);
        let mut target_cwin = CWIN_INITIAL;
        if self.nominal_max_rtt != 0 && self.nominal_rate != 0 {
            target_cwin = bytes_from_rate(self.nominal_max_rtt, pacing_rate);
        }

        if self.alg_state == C4AlgState::Initial {
            if target_cwin < self.initial_cwnd {
                target_cwin = self.initial_cwnd;
            }
            if self.nb_packets_in_startup > 0 && path_x.peak_bandwidth_estimate > pacing_rate {
                pacing_rate = (pacing_rate + path_x.peak_bandwidth_estimate) / 2;
                let min_win =
                    bytes_from_rate(path_x.smoothed_rtt.ticks(), path_x.peak_bandwidth_estimate)
                        / 2;
                if min_win > target_cwin {
                    target_cwin = min_win;
                }
            }
            if self.use_seed_cwin && self.seed_cwin > target_cwin {
                target_cwin = (self.seed_cwin + target_cwin) / 2;
                self.seed_rate = rate_from_bytes(self.seed_cwin, path_x.smoothed_rtt.ticks());
                if self.seed_rate > pacing_rate {
                    pacing_rate = self.seed_rate;
                }
            }
            self.initial_cwnd = target_cwin;
        } else {
            let delta_rtt_target = if self.nominal_max_rtt < 4 * C4_RTT_MARGIN_DELAY {
                self.nominal_max_rtt / 4
            } else {
                C4_RTT_MARGIN_DELAY
            };
            target_cwin += bytes_from_rate(delta_rtt_target, pacing_rate);

            if self.alg_state == C4AlgState::Pushing {
                let delta_alpha = self.alpha_1024_current.saturating_sub(1024);
                let delta_rate = mult1024(delta_alpha, self.nominal_rate);
                let delta_cwin = bytes_from_rate(self.nominal_max_rtt, delta_rate);
                if delta_cwin < path_x.send_mtu as u64 {
                    target_cwin += path_x.send_mtu as u64 - delta_cwin;
                }
            }
        }

        path_x.cwin = target_cwin;
        let mut quantum = mult1024(4, pacing_rate);
        if quantum > 0x10000 {
            quantum = 0x10000;
        } else if quantum < 2 * path_x.send_mtu as u64 {
            quantum = 2 * path_x.send_mtu as u64;
        }
        path_x.update_pacing_rate(pacing_rate as f64, quantum);
    }

    /// C: `c4_notify_congestion` (picoquic/c4.c:893)
    ///
    /// Reacts to a congestion event (delay, ECN, or loss) by computing a
    /// `beta` back-off fraction, reducing `nominal_rate` and optionally
    /// `nominal_max_rtt`, then entering Recovery.  Congestion signals arriving
    /// while already in Recovery tighten `alpha_1024_current` further rather
    /// than re-entering.
    fn notify_congestion(
        &mut self,
        path_x: &mut Path,
        connection: &Connection,
        c_mode: C4Congestion,
    ) {
        let mut beta = C4_BETA_LOSS_1024;
        self.congestion_notified = true;

        if c_mode == C4Congestion::Loss {
            beta = (C4_BETA_LOSS_1024 + mult1024(self.sensitivity_1024(), C4_BETA_LOSS_1024)) / 2;
        } else if c_mode == C4Congestion::Ecn {
            if let Some(beta_ecn) = self
                .ecn_alpha
                .saturating_sub(self.ecn_threshold)
                .checked_mul(1024)
                .and_then(|v| v.checked_div(self.ecn_threshold))
            {
                beta = beta_ecn;
            }
            if beta > C4_BETA_LOSS_1024 {
                beta = C4_BETA_LOSS_1024;
            }
        }

        if c_mode == C4Congestion::Delay {
            if let Some(beta_delay) = self
                .recent_delay_excess
                .checked_mul(1024)
                .and_then(|v| v.checked_div(self.delay_threshold))
            {
                beta = beta_delay;
            }
            if beta > C4_BETA_LOSS_1024 {
                beta = C4_BETA_LOSS_1024;
            }
        } else {
            self.recent_delay_excess = 0;
        }

        if self.alg_state == C4AlgState::Recovery {
            if self.alpha_1024_current == C4_ALPHA_RECOVER_1024 {
                self.alpha_1024_current = C4_ALPHA_RECOVER2_1024;
                self.era_sequence = connection.sequence_number(path_x);
                c4_logger(path_x, 0, self, None, beta, c_mode);
            }
            if c_mode == C4Congestion::Ecn {
                self.excess_ce_after_push = true;
            }
        } else {
            if self.alg_state != C4AlgState::Pushing {
                self.nominal_rate -= mult1024(beta, self.nominal_rate);
                if c_mode == C4Congestion::Loss {
                    self.nominal_max_rtt -= mult1024(beta, self.nominal_max_rtt);
                    if self.nominal_max_rtt < C4_MAX_RTT_MIN {
                        self.nominal_max_rtt = C4_MAX_RTT_MIN;
                    }
                    self.delay_threshold = self.delay_threshold();
                }
                c4_logger(path_x, 0, self, None, beta, c_mode);
            }
            self.enter_recovery(path_x, connection, c_mode);
        }

        self.apply_rate_and_cwin(path_x);
        path_x.is_ssthresh_initialized = true;
    }

    /// C: `c4_initial_handle_loss` (picoquic/c4.c:568)
    ///
    /// Counts a loss event during Initial and exits Initial once
    /// `C4_NB_PACKETS_BEFORE_LOSS` losses have been seen.
    fn initial_handle_loss(&mut self, path_x: &mut Path, connection: &Connection) {
        self.nb_packets_in_startup += 1;
        if self.nb_packets_in_startup > C4_NB_PACKETS_BEFORE_LOSS {
            self.exit_initial(path_x, connection);
        }
    }

    /// C: `c4_update_rtt` (picoquic/c4.c:976)
    ///
    /// Updates per-era RTT accumulators and the running min-RTT from a new
    /// RTT sample.  If `nominal_max_rtt` has not been set yet, initialises it
    /// and the delay threshold.  Otherwise computes `recent_delay_excess` as
    /// how far the sample exceeds `nominal_max_rtt + delay_threshold`.
    fn update_rtt(&mut self, rtt_measurement: u64) {
        if rtt_measurement > self.era_max_rtt {
            self.era_max_rtt = rtt_measurement;
        }
        if rtt_measurement < self.era_min_rtt {
            self.era_min_rtt = rtt_measurement;
        }
        if rtt_measurement < self.running_min_rtt {
            self.running_min_rtt = rtt_measurement;
        }
        if self.nominal_max_rtt == 0 {
            self.nominal_max_rtt = rtt_measurement;
            if self.nominal_max_rtt < C4_MAX_RTT_MIN {
                self.nominal_max_rtt = C4_MAX_RTT_MIN;
            }
            self.delay_threshold = self.delay_threshold();
            self.recent_delay_excess = 0;
        } else {
            let target_rtt = self.nominal_max_rtt + self.delay_threshold;
            if rtt_measurement > target_rtt {
                self.recent_delay_excess = rtt_measurement - target_rtt;
            } else {
                self.recent_delay_excess = 0;
            }
        }
    }

    /// C: `c4_initial_handle_rtt_excess` (picoquic/c4.c:551)
    ///
    /// HyStart-style RTT-increase detector during Initial.  Exits Initial
    /// when delay excess has been seen, at least two eras without increase
    /// have elapsed, and the rate has not grown above the previous era's rate.
    fn initial_handle_rtt_excess(&mut self, path_x: &mut Path, connection: &Connection) {
        if self.recent_delay_excess > 0
            && self.nb_eras_no_increase > 1
            && self.push_rate_old >= self.nominal_rate
        {
            self.exit_initial(path_x, connection);
        }
    }

    /// C: `c4_handle_rtt_excess` (picoquic/c4.c:1008)
    ///
    /// Fires a delay-congestion notification when a delay excess was observed
    /// during a period where alpha was above neutral (i.e. we were probing).
    fn handle_rtt_excess(&mut self, path_x: &mut Path, connection: &Connection) {
        if self.recent_delay_excess > 0 && self.alpha_1024_previous > 1024 {
            self.notify_congestion(path_x, connection, C4Congestion::Delay);
        }
    }

    /// C: `c4_update_min_max_rtt` (picoquic/c4.c:756)
    ///
    /// Updates `running_min_rtt`, `nominal_max_rtt`, and `delay_threshold`
    /// from the era's min/max RTT samples.  Only applied when
    /// `alpha_1024_previous ≤ C4_ALPHA_NEUTRAL_1024` so that measurements
    /// taken during a push period don't corrupt the baseline.  A corrected max
    /// is capped at `running_min_rtt + C4_MAX_JITTER` to limit jitter impact.
    fn update_min_max_rtt(&mut self, path_x: &Path) {
        let rtt_sample = path_x.rtt_sample.ticks();
        if rtt_sample > self.era_max_rtt {
            self.era_max_rtt = rtt_sample;
        }
        if rtt_sample < self.era_min_rtt {
            self.era_min_rtt = rtt_sample;
        }
        if self.alpha_1024_previous <= C4_ALPHA_NEUTRAL_1024 {
            if self.era_min_rtt < self.running_min_rtt {
                self.running_min_rtt = self.era_min_rtt;
            } else {
                self.running_min_rtt = (7 * self.running_min_rtt + self.era_min_rtt) / 8;
            }

            let corrected_max = if self.era_max_rtt < self.running_min_rtt + C4_MAX_JITTER {
                self.era_max_rtt
            } else {
                self.running_min_rtt + C4_MAX_JITTER
            };

            if corrected_max > self.nominal_max_rtt {
                self.nominal_max_rtt = corrected_max;
            } else {
                self.nominal_max_rtt = (7 * self.nominal_max_rtt + corrected_max) / 8;
            }
            self.delay_threshold = self.delay_threshold();
        } else if self.nominal_max_rtt == 0 {
            self.nominal_max_rtt = self.era_max_rtt;
            self.delay_threshold = self.delay_threshold();
        }

        if self.nominal_max_rtt < C4_MAX_RTT_MIN {
            self.nominal_max_rtt = C4_MAX_RTT_MIN;
            self.delay_threshold = self.delay_threshold();
        }
    }

    /// C: `c4_enter_push` (picoquic/c4.c:746)
    ///
    /// Transitions to the Pushing state, setting alpha from the probe-level
    /// table, snapshotting that alpha into `push_alpha`, and resetting the era.
    fn enter_push(&mut self, path_x: &mut Path, connection: &Connection) {
        let level = self.probe_level.clamp(0, C4_PROBE_LEVEL_MAX) as usize;
        self.alpha_1024_current = C4_PUSH_RATE_BY_PROBE_LEVEL[level];
        self.push_alpha = self.alpha_1024_current;
        self.era_reset(path_x, connection);
        self.alg_state = C4AlgState::Pushing;
    }

    /// C: `c4_handle_ack` (picoquic/c4.c:804)
    ///
    /// Processes an acknowledgement: updates `nominal_rate` from the bandwidth
    /// estimate, sets `push_was_not_limited` when delivery was not
    /// application-limited, then advances the state machine at each era
    /// boundary (exit recovery → cruise → push → recovery cycle).
    fn handle_ack(&mut self, path_x: &mut Path, connection: &Connection, ack_state: &PerAckState) {
        let previous_rate = self.nominal_rate;

        if ack_state.rtt_measurement.ticks() > 0
            && ack_state.nb_bytes_delivered_since_packet_sent > 0
        {
            let rate_measurement = path_x.bandwidth_estimate;
            c4_logger(
                path_x,
                rate_measurement,
                self,
                Some(ack_state),
                0,
                C4Congestion::None,
            );

            if rate_measurement > self.nominal_rate
                && !(self.alg_state == C4AlgState::Recovery && self.congestion_notified)
            {
                self.push_was_not_limited = true;
                self.nominal_rate = rate_measurement;
                self.delay_threshold = self.delay_threshold();
            } else {
                let target_cwin = bytes_from_rate(self.running_min_rtt, previous_rate);
                if ack_state.nb_bytes_delivered_since_packet_sent > target_cwin {
                    self.push_was_not_limited = true;
                }
            }
        }

        if self.alg_state == C4AlgState::Initial {
            self.initial_handle_ack(path_x, connection, ack_state);
        } else if self.era_check(path_x, connection) {
            self.update_min_max_rtt(path_x);
            // Check whether jitter/competition warrants re-entering Initial.
            if !self.initial_after_jitter
                && self.nominal_max_rtt > 50_000
                && self.nominal_rate < 1_000_000
                && 5 * self.running_min_rtt < 2 * self.nominal_max_rtt
            {
                self.initial_after_jitter = true;
                self.enter_initial(path_x, connection);
            } else {
                match self.alg_state {
                    C4AlgState::Recovery => {
                        self.exit_recovery(path_x, connection);
                    }
                    C4AlgState::Cruising => {
                        if self.nb_cruise_left_before_push > 0 {
                            self.nb_cruise_left_before_push -= 1;
                        }
                        self.era_reset(path_x, connection);
                        if self.nb_cruise_left_before_push == 0
                            && path_x.last_time_acked_data_frame_sent
                                > path_x.last_sender_limited_time
                        {
                            self.enter_push(path_x, connection);
                        }
                    }
                    C4AlgState::Pushing => {
                        self.enter_recovery(path_x, connection, C4Congestion::None);
                    }
                    _ => {
                        self.era_reset(path_x, connection);
                    }
                }
            }
        }
    }

    /// C: `c4_reset` (picoquic/c4.c:517)
    ///
    /// Zeroes the C4 state (preserving `option_string`), then re-initialises
    /// via `enter_initial`.  Called on algorithm reset notifications and during
    /// `c4_init`.
    ///
    /// The C signature carries `option_string` as an explicit parameter so the
    /// caller can supply it directly (e.g. from `c4_init`).  In the Rust
    /// translation the field is owned by [`C4State`], so the method preserves
    /// the existing value — equivalent to the C pattern
    /// `c4_reset(state, path, state->option_string)` used in the Reset
    /// notification path.
    pub fn reset(&mut self, path_x: &mut Path, connection: &Connection) {
        let option_string = self.option_string.take();
        // Zero all fields.
        *self = C4State {
            alg_state: C4AlgState::default(),
            nominal_rate: 0,
            nominal_max_rtt: 0,
            initial_cwnd: 0,
            running_min_rtt: u64::MAX,
            alpha_1024_current: C4_ALPHA_INITIAL,
            alpha_1024_previous: 0,
            nb_packets_in_startup: 0,
            era_sequence: 0,
            nb_cruise_left_before_push: 0,
            seed_cwin: 0,
            seed_rate: 0,
            probe_level: 0,
            nb_eras_no_increase: 0,
            push_rate_old: 0,
            push_alpha: 0,
            era_max_rtt: 0,
            era_min_rtt: 0,
            delay_threshold: 0,
            recent_delay_excess: 0,
            last_lost_packet_number: 0,
            smoothed_drop_rate: 0.0,
            ecn_alpha: 0,
            ecn_ect1: 0,
            ecn_ce: 0,
            ecn_threshold: 0,
            congestion_notified: false,
            push_was_not_limited: false,
            use_seed_cwin: false,
            initial_after_jitter: false,
            excess_ce_after_push: false,
            option_string,
        };
        self.set_options();
        self.enter_initial(path_x, connection);
    }

    /// C: `c4_notify` (picoquic/c4.c:1025)
    ///
    /// Main congestion-control notification dispatch.  Sets `is_cc_data_updated`,
    /// filters non-application packet contexts, then routes the notification to
    /// the appropriate handler.
    pub fn notify(
        &mut self,
        connection: &Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: Option<&PerAckState>,
        _current_time: u64,
    ) {
        path_x.is_cc_data_updated = true;

        if let Some(a) = ack_state
            && a.pc != PacketContext::Application as i32
        {
            return;
        }

        match notification {
            CongestionNotification::Acknowledgement => {
                if let Some(a) = ack_state {
                    self.handle_ack(path_x, connection, a);
                }
                self.apply_rate_and_cwin(path_x);
            }
            CongestionNotification::EcnEc => {
                self.ecn_threshold = self.ecn_threshold();
                self.update_ecn_alpha(path_x, connection);
                if self.ecn_alpha > self.ecn_threshold {
                    if self.alg_state == C4AlgState::Initial {
                        if self.recent_delay_excess > 0
                            && self.nb_eras_no_increase > 1
                            && self.push_rate_old >= self.nominal_rate
                        {
                            self.exit_initial(path_x, connection);
                        }
                    } else {
                        self.notify_congestion(path_x, connection, C4Congestion::Ecn);
                    }
                }
            }
            CongestionNotification::Repeat => {
                if let Some(a) = ack_state {
                    if self.alg_state == C4AlgState::Recovery
                        && a.lost_packet_number < self.era_sequence
                    {
                        return;
                    }
                    self.update_loss_rate(a.lost_packet_number);
                    if self.smoothed_drop_rate > self.loss_threshold() {
                        if self.alg_state == C4AlgState::Initial {
                            self.initial_handle_loss(path_x, connection);
                        } else {
                            self.notify_congestion(path_x, connection, C4Congestion::Loss);
                        }
                    }
                }
            }
            CongestionNotification::Timeout => {}
            CongestionNotification::SpuriousRepeat => {}
            CongestionNotification::RttMeasurement => {
                if let Some(a) = ack_state {
                    self.update_rtt(a.rtt_measurement.ticks());
                    if self.alg_state == C4AlgState::Initial {
                        self.initial_handle_rtt_excess(path_x, connection);
                        self.apply_rate_and_cwin(path_x);
                    } else {
                        self.handle_rtt_excess(path_x, connection);
                    }
                }
            }
            CongestionNotification::LostFeedback => {}
            CongestionNotification::CwinBlocked => {}
            CongestionNotification::Reset => {
                self.reset(path_x, connection);
            }
            CongestionNotification::SeedCwin => {
                if let Some(a) = ack_state {
                    self.seed_cwin(a.nb_bytes_acknowledged);
                }
            }
        }
    }
}

/// C: `c4_logger` (picoquic/c4.c:199)
///
/// Emits a diagnostic log line with the current C4 rate-control parameters.
/// In C this is gated by `C4_WITH_LOGGING`; in Rust it always compiles and
/// emits at `log::debug` level (no-cost when the log level is above debug).
pub fn c4_logger(
    path_x: &Path,
    rate_measurement: u64,
    c4_state: &C4State,
    ack_state: Option<&PerAckState>,
    beta: u64,
    congestion_mode: C4Congestion,
) {
    log::debug!(
        "C4_rate, {},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        rate_measurement,
        c4_state.nominal_rate,
        ack_state.map_or(0, |a| a.nb_bytes_delivered_since_packet_sent),
        ack_state.map_or(0, |a| a.rtt_measurement.ticks()),
        ack_state.map_or(0, |a| a.send_delay.ticks()),
        c4_state.nominal_max_rtt,
        c4_state.alg_state as u64,
        path_x.bandwidth_estimate,
        path_x.bytes_in_transit,
        beta,
        congestion_mode as u64,
        c4_state.ecn_alpha,
        path_x.smoothed_rtt.ticks(),
        path_x.rtt_variant.ticks(),
        c4_state.alpha_1024_previous,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> C4State {
        C4State {
            alg_state: C4AlgState::default(),
            nominal_rate: 0,
            nominal_max_rtt: 0,
            initial_cwnd: 0,
            running_min_rtt: 0,
            alpha_1024_current: 0,
            alpha_1024_previous: 0,
            nb_packets_in_startup: 0,
            era_sequence: 0,
            nb_cruise_left_before_push: 0,
            seed_cwin: 0,
            seed_rate: 0,
            probe_level: 0,
            nb_eras_no_increase: 0,
            push_rate_old: 0,
            push_alpha: 0,
            era_max_rtt: 0,
            era_min_rtt: 0,
            delay_threshold: 0,
            recent_delay_excess: 0,
            last_lost_packet_number: 0,
            smoothed_drop_rate: 0.0,
            ecn_alpha: 0,
            ecn_ect1: 0,
            ecn_ce: 0,
            ecn_threshold: 0,
            congestion_notified: false,
            push_was_not_limited: false,
            use_seed_cwin: false,
            initial_after_jitter: false,
            excess_ce_after_push: false,
            option_string: None,
        }
    }

    #[test]
    fn growth_evaluate_high_alpha_growing() {
        let mut s = make_state();
        // push_alpha > 1088, push_rate_old = 1000, nominal_rate must exceed:
        // target = (3*1000 + MULT1024(1280, 1000)) / 4 = (3000 + 1250) / 4 = 1062
        s.push_alpha = 1280;
        s.push_rate_old = 1000;
        s.nominal_rate = 1100;
        assert!(s.growth_evaluate());
    }

    #[test]
    fn growth_evaluate_high_alpha_not_growing() {
        let mut s = make_state();
        s.push_alpha = 1280;
        s.push_rate_old = 1000;
        s.nominal_rate = 1000; // equal to target_rate or below
        assert!(!s.growth_evaluate());
    }

    #[test]
    fn growth_evaluate_low_alpha_growing_no_congestion() {
        let mut s = make_state();
        s.push_alpha = 1000; // <= 1088
        s.push_rate_old = 800;
        s.nominal_rate = 900;
        s.congestion_notified = false;
        assert!(s.growth_evaluate());
    }

    #[test]
    fn growth_evaluate_low_alpha_congestion_blocks_growth() {
        let mut s = make_state();
        s.push_alpha = 1000;
        s.push_rate_old = 800;
        s.nominal_rate = 900;
        s.congestion_notified = true;
        assert!(!s.growth_evaluate());
    }

    // --- sensitivity_1024 ---

    #[test]
    fn sensitivity_below_50k() {
        let mut s = make_state();
        s.nominal_rate = 49_999;
        assert_eq!(s.sensitivity_1024(), 0);
    }

    #[test]
    fn sensitivity_at_50k_boundary() {
        let mut s = make_state();
        s.nominal_rate = 50_000;
        // (50000 - 50000) * 963 / 950000 == 0
        assert_eq!(s.sensitivity_1024(), 0);
    }

    #[test]
    fn sensitivity_midrange() {
        let mut s = make_state();
        s.nominal_rate = 500_000;
        // (500000 - 50000) * 963 / 950000 = 450000 * 963 / 950000 = 456
        assert_eq!(s.sensitivity_1024(), 456);
    }

    #[test]
    fn sensitivity_at_1mbps() {
        let mut s = make_state();
        s.nominal_rate = 1_000_000;
        // hits else branch: 963 + (0 * 61 / 9000000) = 963
        assert_eq!(s.sensitivity_1024(), 963);
    }

    #[test]
    fn sensitivity_at_10mbps() {
        let mut s = make_state();
        s.nominal_rate = 10_000_000;
        // 963 + (9000000 * 61 / 9000000) = 963 + 61 = 1024
        assert_eq!(s.sensitivity_1024(), 1024);
    }

    #[test]
    fn sensitivity_above_10mbps() {
        let mut s = make_state();
        s.nominal_rate = 10_000_001;
        assert_eq!(s.sensitivity_1024(), 1024);
    }

    // --- growth_reset ---

    #[test]
    fn growth_reset_clears_flags_and_snapshots() {
        let mut s = make_state();
        s.nominal_rate = 5_000_000;
        s.alpha_1024_current = 1100;
        s.congestion_notified = true;
        s.push_was_not_limited = true;
        s.push_rate_old = 1_000_000;
        s.push_alpha = 900;

        s.growth_reset();

        assert!(!s.congestion_notified);
        assert!(!s.push_was_not_limited);
        assert_eq!(s.push_rate_old, 5_000_000);
        assert_eq!(s.push_alpha, 1100);
    }

    // --- set_options ---

    #[test]
    fn set_options_none_is_noop() {
        let mut s = make_state();
        s.option_string = None;
        s.set_options(); // must not panic
    }

    #[test]
    fn set_options_unknown_chars_are_noop() {
        let mut s = make_state();
        s.option_string = Some("xyz".to_owned());
        s.set_options(); // must not panic; no state change expected
    }

    // --- seed_cwin ---

    #[test]
    fn seed_cwin_sets_when_initial() {
        let mut s = make_state();
        s.alg_state = C4AlgState::Initial;
        s.seed_cwin(123_456);
        assert!(s.use_seed_cwin);
        assert_eq!(s.seed_cwin, 123_456);
    }

    #[test]
    fn seed_cwin_ignored_when_not_initial() {
        let mut s = make_state();
        s.alg_state = C4AlgState::Cruising;
        s.seed_cwin(123_456);
        assert!(!s.use_seed_cwin);
        assert_eq!(s.seed_cwin, 0);
    }

    // --- observe ---

    #[test]
    fn observe_returns_state_and_rtt() {
        let mut s = make_state();
        s.alg_state = C4AlgState::Pushing;
        s.nominal_max_rtt = 42_000;
        let (cc_state, cc_param) = s.observe();
        assert_eq!(cc_state, C4AlgState::Pushing as u64);
        assert_eq!(cc_param, 42_000);
    }

    #[test]
    fn observe_initial_state() {
        let s = make_state();
        let (cc_state, cc_param) = s.observe();
        assert_eq!(cc_state, 0); // C4AlgState::Initial == 0
        assert_eq!(cc_param, 0);
    }

    // --- update_loss_rate ---

    #[test]
    fn loss_rate_no_update_when_not_newer() {
        let mut s = make_state();
        s.last_lost_packet_number = 100;
        s.smoothed_drop_rate = 0.5;
        s.update_loss_rate(100); // same number → no change
        assert_eq!(s.smoothed_drop_rate, 0.5);
        assert_eq!(s.last_lost_packet_number, 100);
    }

    #[test]
    fn loss_rate_single_step() {
        let mut s = make_state();
        s.last_lost_packet_number = 0;
        s.smoothed_drop_rate = 0.0;
        s.update_loss_rate(1);
        // one decay step: rate *= (1 - 1/16) = 0  (was 0)
        // then: rate += (1 - 0) * (1/16) = 1/16
        assert!((s.smoothed_drop_rate - 1.0 / 16.0).abs() < 1e-12);
        assert_eq!(s.last_lost_packet_number, 1);
    }

    #[test]
    fn loss_rate_caps_scope() {
        let mut s = make_state();
        // Gap far exceeds SMOOTHED_LOSS_SCOPE=32; loop should start at
        // lost_packet_number - 32 so only 32 decays happen, not 1000.
        s.last_lost_packet_number = 0;
        s.smoothed_drop_rate = 1.0;
        s.update_loss_rate(1000);
        // After 32 decay steps: 1.0 * (15/16)^32 ≈ 0.1209
        // Then one more step adding the loss event:
        // result = decayed + (1 - decayed) * (1/16)
        let expected_decayed = (15.0f64 / 16.0).powi(32);
        let expected = expected_decayed + (1.0 - expected_decayed) * (1.0 / 16.0);
        assert!((s.smoothed_drop_rate - expected).abs() < 1e-9);
        assert_eq!(s.last_lost_packet_number, 1000);
    }

    // --- ecn_threshold ---

    #[test]
    fn ecn_threshold_zero_sensitivity() {
        let mut s = make_state();
        s.nominal_rate = 0; // sensitivity = 0
        // 192 - MULT1024(0, 96) = 192
        assert_eq!(s.ecn_threshold(), 192);
    }

    #[test]
    fn ecn_threshold_max_sensitivity() {
        let mut s = make_state();
        s.nominal_rate = 10_000_001; // sensitivity = 1024
        // 192 - MULT1024(1024, 96) = 192 - 96 = 96
        assert_eq!(s.ecn_threshold(), 96);
    }

    #[test]
    fn ecn_threshold_mid_sensitivity() {
        let mut s = make_state();
        s.nominal_rate = 1_000_000; // sensitivity = 963
        // 192 - MULT1024(963, 96) = 192 - ((96 * 963) >> 10) = 192 - 90 = 102
        let expected = 192 - ((96u64 * 963) >> 10);
        assert_eq!(s.ecn_threshold(), expected);
    }

    // --- ecn_alpha_update_counts ---

    #[test]
    fn ecn_alpha_no_update_when_both_zero_delta() {
        let mut s = make_state();
        s.ecn_alpha = 500;
        s.ecn_ect1 = 100;
        s.ecn_ce = 50;
        // same totals → no delta → alpha unchanged
        s.ecn_alpha_update_counts(100, 50);
        assert_eq!(s.ecn_alpha, 500);
    }

    #[test]
    fn ecn_alpha_fast_path_large_frac() {
        let mut s = make_state();
        s.ecn_alpha = 400;
        s.ecn_ect1 = 0;
        s.ecn_ce = 0;
        // delta_ce = 600, delta_ect1 = 400 → frac = 600*1024/(600+400) = 614
        // frac (614) > alpha (400) and frac >= 512 → fast path: alpha = frac
        s.ecn_alpha_update_counts(400, 600);
        assert_eq!(s.ecn_alpha, 614);
    }

    #[test]
    fn ecn_alpha_ewma_path() {
        let mut s = make_state();
        s.ecn_alpha = 256;
        s.ecn_ect1 = 0;
        s.ecn_ce = 0;
        // delta_ce = 100, delta_ect1 = 900 → frac = 100*1024/1000 = 102
        // frac (102) < alpha (256) → EWMA path
        // alpha_shifted = 256 * 16 - 256 + 102 = 256*15 + 102 = 3840 + 102 = 3942
        // new alpha = 3942 / 16 = 246
        s.ecn_alpha_update_counts(900, 100);
        assert_eq!(s.ecn_alpha, 246);
    }

    #[test]
    fn ecn_alpha_only_ect1_delta() {
        let mut s = make_state();
        s.ecn_alpha = 200;
        s.ecn_ect1 = 0;
        s.ecn_ce = 0;
        // delta_ect1 = 500, delta_ce = 0 → frac = 0
        // frac (0) < alpha (200) → EWMA:
        //   alpha_shifted = 200 << 4 = 3200; 3200 - 200 = 3000; 3000 + 0 = 3000
        //   new alpha = 3000 >> 4 = 187
        s.ecn_alpha_update_counts(500, 0);
        assert_eq!(s.ecn_alpha, 187);
    }
}
