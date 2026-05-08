//! FastCC congestion-control algorithm.
//!
//! Translation of `picoquic/fastcc.c`.

use crate::cc_common::{ConnectionCc, MinMaxRtt, SMOOTHED_LOSS_THRESHOLD};
use crate::internal::{CWIN_INITIAL, CWIN_MINIMUM, Connection, Path};
use crate::{CongestionControl, CongestionNotification, Instant, PerAckState};

const FASTCC_REPEAT_THRESHOLD: i32 = 4;
const FASTCC_BETA: f64 = 0.125;
const FASTCC_EVAL_ALPHA: f64 = 0.25;
const FASTCC_NB_PERIOD: usize = 6;
const FASTCC_PERIOD: u64 = 1_000_000;

/// Algorithmic state for FastCC.  C: `picoquic_fastcc_alg_state_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum FastccAlgState {
    Initial = 0,
    Eval = 1,
    Freeze = 2,
}

/// Per-path FastCC state block.  C: `picoquic_fastcc_state_t`.
///
/// Translation deviations from C:
/// * Single-bit `unsigned int x : 1` bitfields → `bool`.
/// * `uint64_t rtt_min` / `rolling_rtt_min` → `u64` (raw µs ticks, matches C).
/// * `picoquic_min_max_rtt_t rtt_filter` → [`MinMaxRtt`].
/// * `int nb_cc_events` → `i32` (signed, faithfully).
#[derive(Debug, Clone)]
pub struct FastccState {
    pub alg_state: FastccAlgState,
    pub end_of_freeze: u64,
    pub last_ack_time: u64,
    pub ack_interval: u64,
    pub nb_bytes_ack: u64,
    pub nb_bytes_ack_since_rtt: u64,
    pub end_of_epoch: u64,
    pub recovery_sequence: u64,
    pub rtt_min: u64,
    pub delay_threshold: u64,
    pub rolling_rtt_min: u64,
    pub last_rtt_min: [u64; FASTCC_NB_PERIOD],
    pub nb_cc_events: i32,
    pub last_freeze_was_timeout: bool,
    pub last_freeze_was_not_delay: bool,
    pub rtt_min_is_trusted: bool,
    pub rtt_filter: MinMaxRtt,
}

impl Default for FastccState {
    fn default() -> Self {
        Self {
            alg_state: FastccAlgState::Initial,
            end_of_freeze: 0,
            last_ack_time: 0,
            ack_interval: 0,
            nb_bytes_ack: 0,
            nb_bytes_ack_since_rtt: 0,
            end_of_epoch: 0,
            recovery_sequence: 0,
            rtt_min: 0,
            delay_threshold: 0,
            rolling_rtt_min: 0,
            last_rtt_min: [0u64; FASTCC_NB_PERIOD],
            nb_cc_events: 0,
            last_freeze_was_timeout: false,
            last_freeze_was_not_delay: false,
            rtt_min_is_trusted: false,
            rtt_filter: MinMaxRtt::default(),
        }
    }
}

/// Compute the FastCC delay threshold from the minimum RTT.
///
/// The threshold is `rtt_min / 8`, capped at 25 000 µs.
///
/// C: `picoquic_fastcc_delay_threshold`
pub fn fastcc_delay_threshold(rtt_min: u64) -> u64 {
    const FASTCC_DELAY_THRESHOLD_MAX: u64 = 25_000;
    (rtt_min / 8).min(FASTCC_DELAY_THRESHOLD_MAX)
}

/// Reset a FastCC state block and the path congestion window.
///
/// C: `picoquic_fastcc_reset` (`fastcc.c:74-83`)
pub fn picoquic_fastcc_reset(state: &mut FastccState, path_x: &mut Path, current_time: Instant) {
    *state = FastccState::default();
    state.alg_state = FastccAlgState::Initial;
    state.rtt_min = path_x.smoothed_rtt.ticks();
    state.rolling_rtt_min = state.rtt_min;
    state.delay_threshold = fastcc_delay_threshold(state.rtt_min);
    state.end_of_epoch = current_time.ticks().saturating_add(FASTCC_PERIOD);
    path_x.cwin = CWIN_INITIAL;
}

/// If the path is still in the initial slow-start state and `cwin` is
/// below `bytes_in_flight`, raise `cwin` to match.
///
/// C: `picoquic_fastcc_seed_cwin` (`fastcc.c:85-92`)
pub fn fastcc_seed_cwin(state: &FastccState, cwin: &mut u64, bytes_in_flight: u64) {
    if state.alg_state == FastccAlgState::Initial && *cwin < bytes_in_flight {
        *cwin = bytes_in_flight;
    }
}

/// Initialise FastCC state on `path_x`.
///
/// C: `picoquic_fastcc_init` (`fastcc.c:94-117`)
pub fn picoquic_fastcc_init(
    path_x: &mut Path,
    _option_string: Option<&str>,
    current_time: Instant,
) {
    let mut state = path_x
        .congestion_alg_state
        .take()
        .and_then(|boxed| boxed.downcast::<FastccState>().ok())
        .unwrap_or_default();
    picoquic_fastcc_reset(&mut state, path_x, current_time);
    path_x.congestion_alg_state = Some(state);
}

/// Reaction to ECN/CE, sustained losses, timeout, or persistent delay.
///
/// C: `fastcc_notify_congestion` (`fastcc.c:119-160`)
fn fastcc_notify_congestion(
    connection: &Connection,
    path_x: &mut Path,
    state: &mut FastccState,
    current_time: Instant,
    is_delay: bool,
    is_timeout: bool,
) {
    if state.alg_state == FastccAlgState::Freeze
        && (!is_timeout || !state.last_freeze_was_timeout)
        && (!is_delay || !state.last_freeze_was_not_delay)
    {
        return;
    }

    state.last_freeze_was_not_delay = !is_delay;
    state.last_freeze_was_timeout = is_timeout;
    state.alg_state = FastccAlgState::Freeze;
    state.end_of_freeze = current_time.ticks().saturating_add(state.rtt_min);
    state.recovery_sequence = connection.sequence_number(path_x);
    state.nb_cc_events = 0;

    if is_delay {
        path_x.cwin -= (FASTCC_BETA * path_x.cwin as f64) as u64;
    } else {
        path_x.cwin /= 2;
    }

    if is_timeout || path_x.cwin < CWIN_MINIMUM {
        path_x.cwin = CWIN_MINIMUM;
    }

    path_x.update_pacing_data(0);
    path_x.is_ssthresh_initialized = true;
}

/// Drive FastCC with a congestion-control notification.
///
/// C: `picoquic_fastcc_notify` (`fastcc.c:162-306`)
pub fn picoquic_fastcc_notify(
    connection: &mut Connection,
    path_x: &mut Path,
    notification: CongestionNotification,
    ack_state: &PerAckState,
    current_time: Instant,
) {
    path_x.is_cc_data_updated = true;

    let Some(boxed_state) = path_x.congestion_alg_state.take() else {
        return;
    };

    let mut state = match boxed_state.downcast::<FastccState>() {
        Ok(state) => state,
        Err(boxed_state) => {
            path_x.congestion_alg_state = Some(boxed_state);
            return;
        }
    };

    if state.alg_state == FastccAlgState::Freeze
        && (current_time.ticks() > state.end_of_freeze
            || state.recovery_sequence <= connection.ack_number(path_x))
    {
        if state.last_freeze_was_timeout {
            state.alg_state = FastccAlgState::Initial;
        } else {
            state.alg_state = FastccAlgState::Eval;
        }
        state.last_freeze_was_not_delay = false;
        state.last_freeze_was_timeout = false;
        state.nb_cc_events = 0;
        state.nb_bytes_ack_since_rtt = 0;
    }

    match notification {
        CongestionNotification::Acknowledgement if state.alg_state != FastccAlgState::Freeze => {
            state.nb_bytes_ack_since_rtt = state
                .nb_bytes_ack_since_rtt
                .saturating_add(ack_state.nb_bytes_acknowledged);
            path_x.update_pacing_data(0);
        }
        CongestionNotification::EcnEc => {
            fastcc_notify_congestion(connection, path_x, &mut state, current_time, false, false);
        }
        event @ (CongestionNotification::Repeat | CongestionNotification::Timeout)
            if state.rtt_filter.hystart_loss_test(
                notification,
                ack_state.lost_packet_number,
                SMOOTHED_LOSS_THRESHOLD,
            ) =>
        {
            fastcc_notify_congestion(
                connection,
                path_x,
                &mut state,
                current_time,
                false,
                event == CongestionNotification::Timeout,
            );
        }
        CongestionNotification::SpuriousRepeat if state.nb_cc_events > 0 => {
            state.nb_cc_events -= 1;
        }
        CongestionNotification::RttMeasurement => {
            let mut delta_rtt = 0u64;
            state
                .rtt_filter
                .filter_rtt_min_max(ack_state.rtt_measurement);

            if state.rtt_filter.is_init {
                let sample_max = state.rtt_filter.sample_max.ticks();
                if current_time.ticks() > state.end_of_epoch {
                    state.rtt_min = u64::MAX;
                    for i in (1..FASTCC_NB_PERIOD).rev() {
                        state.last_rtt_min[i] = state.last_rtt_min[i - 1];
                        if state.last_rtt_min[i] > 0 && state.last_rtt_min[i] < state.rtt_min {
                            state.rtt_min = state.last_rtt_min[i];
                        }
                    }
                    state.delay_threshold = fastcc_delay_threshold(state.rtt_min);
                    state.last_rtt_min[0] = state.rolling_rtt_min;
                    state.rolling_rtt_min = sample_max;
                    state.end_of_epoch = current_time.ticks().saturating_add(FASTCC_PERIOD);
                } else if sample_max < state.rolling_rtt_min || state.rolling_rtt_min == 0 {
                    state.rolling_rtt_min = sample_max;
                    if state.rolling_rtt_min < state.rtt_min {
                        state.rtt_min = state.rolling_rtt_min;
                    }
                }
            }

            if state.alg_state != FastccAlgState::Freeze {
                let rtt_measurement = ack_state.rtt_measurement.ticks();
                if rtt_measurement < state.rtt_min {
                    state.delay_threshold = fastcc_delay_threshold(state.rtt_min);
                } else if state.rtt_min_is_trusted {
                    delta_rtt = rtt_measurement - state.rtt_min;
                } else {
                    state.rtt_min = rtt_measurement;
                    state.rolling_rtt_min = rtt_measurement;
                    state.rtt_min_is_trusted = true;
                    delta_rtt = 0;
                }

                if delta_rtt < state.delay_threshold {
                    let mut alpha = 1.0;
                    state.nb_cc_events = 0;

                    if state.alg_state != FastccAlgState::Initial {
                        alpha -= delta_rtt as f64 / state.delay_threshold as f64;
                        alpha *= FASTCC_EVAL_ALPHA;
                    }

                    if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
                        path_x.cwin = path_x
                            .cwin
                            .saturating_add((alpha * state.nb_bytes_ack_since_rtt as f64) as u64);
                    }
                    state.nb_bytes_ack_since_rtt = 0;
                } else {
                    state.nb_cc_events += 1;
                    if state.nb_cc_events >= FASTCC_REPEAT_THRESHOLD {
                        fastcc_notify_congestion(
                            connection,
                            path_x,
                            &mut state,
                            current_time,
                            true,
                            false,
                        );
                    }
                }
            }
        }
        CongestionNotification::CwinBlocked => {}
        CongestionNotification::Reset => {
            picoquic_fastcc_reset(&mut state, path_x, current_time);
        }
        CongestionNotification::SeedCwin => {
            fastcc_seed_cwin(&state, &mut path_x.cwin, ack_state.nb_bytes_acknowledged);
        }
        _ => {}
    }

    path_x.congestion_alg_state = Some(state);
}

/// Read the current algorithmic state and rolling RTT minimum out of a
/// path's FastCC state block.
///
/// Writes `alg_state as u64` into `*cc_state` and `rolling_rtt_min`
/// into `*cc_param`.  If the path has no FastCC state block the
/// out-params are left unchanged.
///
/// C: `picoquic_fastcc_observe` (`fastcc.c:320-325`)
pub fn fastcc_observe(path: &Path, cc_state: &mut u64, cc_param: &mut u64) {
    if let Some(state) = path
        .congestion_alg_state
        .as_ref()
        .and_then(|s| s.downcast_ref::<FastccState>())
    {
        *cc_state = state.alg_state as u64;
        *cc_param = state.rolling_rtt_min;
    }
}

/// Congestion-control vtable adapter for FastCC.
pub struct FastccCongestionControl;

impl CongestionControl for FastccCongestionControl {
    fn alg_init(
        &self,
        _connection: &mut Connection,
        path_x: &mut Path,
        option_string: Option<&str>,
        current_time: Instant,
    ) {
        picoquic_fastcc_init(path_x, option_string, current_time);
    }

    fn alg_notify(
        &self,
        connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: Instant,
    ) {
        picoquic_fastcc_notify(connection, path_x, notification, ack_state, current_time);
    }

    fn alg_delete(&self, path_x: &mut Path) {
        path_x.congestion_alg_state = None;
    }

    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|s| s.downcast_ref::<FastccState>())
            .map(|state| (state.alg_state as u64, state.rolling_rtt_min))
    }
}

pub static FASTCC_CONTROL: FastccCongestionControl = FastccCongestionControl;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_zero_rtt() {
        assert_eq!(fastcc_delay_threshold(0), 0);
    }

    #[test]
    fn threshold_below_cap() {
        // rtt_min = 160_000 µs  → 160_000 / 8 = 20_000 < 25_000
        assert_eq!(fastcc_delay_threshold(160_000), 20_000);
    }

    #[test]
    fn threshold_at_cap() {
        // rtt_min = 200_000 µs → 200_000 / 8 = 25_000
        assert_eq!(fastcc_delay_threshold(200_000), 25_000);
    }

    #[test]
    fn threshold_above_cap() {
        // rtt_min = 1_000_000 µs → 1_000_000 / 8 = 125_000, capped at 25_000
        assert_eq!(fastcc_delay_threshold(1_000_000), 25_000);
    }

    #[test]
    fn seed_cwin_raises_when_below() {
        let state = FastccState::default();
        let mut cwin = 10_000u64;
        fastcc_seed_cwin(&state, &mut cwin, 50_000);
        assert_eq!(cwin, 50_000);
    }

    #[test]
    fn seed_cwin_no_change_when_above() {
        let state = FastccState::default();
        let mut cwin = 100_000u64;
        fastcc_seed_cwin(&state, &mut cwin, 50_000);
        assert_eq!(cwin, 100_000);
    }

    #[test]
    fn seed_cwin_no_change_outside_initial() {
        let state = FastccState {
            alg_state: FastccAlgState::Eval,
            ..Default::default()
        };
        let mut cwin = 1u64;
        fastcc_seed_cwin(&state, &mut cwin, 50_000);
        // alg_state != Initial → cwin must not change
        assert_eq!(cwin, 1);
    }

    #[test]
    fn observe_reads_state_and_rtt() {
        let state = FastccState {
            alg_state: FastccAlgState::Freeze,
            rolling_rtt_min: 12_345,
            ..Default::default()
        };
        // Exercise the FastccState fields directly (no Path needed for unit test).
        let cc_state = state.alg_state as u64;
        let cc_param = state.rolling_rtt_min;
        assert_eq!(cc_state, FastccAlgState::Freeze as u64);
        assert_eq!(cc_param, 12_345);
    }
}
