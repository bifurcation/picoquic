//! Translation of `picoquic/newreno.c` — standalone New Reno congestion control.
//!
//! The embedded New Reno simulator ([`NewRenoSimState`], [`NewRenoAlgState`])
//! lives in [`crate::cc_common`] because several other congestion-control
//! algorithms share it.  This module provides the per-path algorithm state
//! used when New Reno runs as the primary (non-embedded) controller.

use crate::cc_common::{ConnectionCc, MinMaxRtt, NewRenoAlgState, NewRenoSimState, PathCc};
use crate::internal::{CWIN_MINIMUM, Connection, Path, TARGET_RENO_RTT};
use crate::{CongestionControl, CongestionNotification, Instant, PerAckState};

/// Per-path state for the standalone New Reno algorithm.
/// C: `picoquic_newreno_state_t` (picoquic/newreno.c:177-180).
#[derive(Default)]
pub struct NewrenoState {
    pub nrss: NewRenoSimState,
    pub rtt_filter: MinMaxRtt,
}

/// Enter New Reno simulator recovery for loss, ECN, or timeout.
///
/// C: `picoquic_newreno_sim_enter_recovery` (picoquic/newreno.c:45-71).
pub(crate) fn picoquic_newreno_sim_enter_recovery(
    nr_state: &mut NewRenoSimState,
    connection: &Connection,
    path_x: &Path,
    notification: CongestionNotification,
    current_time: Instant,
) {
    nr_state.ssthresh = nr_state.cwin / 2;
    if nr_state.ssthresh < CWIN_MINIMUM {
        nr_state.ssthresh = CWIN_MINIMUM;
    }

    if notification == CongestionNotification::Timeout {
        nr_state.cwin = CWIN_MINIMUM;
        nr_state.alg_state = NewRenoAlgState::SlowStart;
    } else {
        nr_state.cwin = nr_state.ssthresh;
        nr_state.alg_state = NewRenoAlgState::CongestionAvoidance;
    }

    nr_state.recovery_start = current_time.ticks();
    nr_state.recovery_sequence = connection.sequence_number(path_x);
    nr_state.residual_ack = 0;
}

/// Reset standalone New Reno state and install the initial congestion window.
///
/// C: `picoquic_newreno_reset` (picoquic/newreno.c:182-187).
fn picoquic_newreno_reset(nr_state: &mut NewrenoState, path_x: &mut Path) {
    *nr_state = NewrenoState::default();
    nr_state.nrss.reset();
    path_x.cwin = nr_state.nrss.cwin;
}

/// Initialize the standalone New Reno state for a path.
///
/// C: `picoquic_newreno_init` (picoquic/newreno.c:189-204).
fn picoquic_newreno_init(path_x: &mut Path, _option_string: Option<&str>, _current_time: Instant) {
    let mut state = path_x
        .congestion_alg_state
        .take()
        .and_then(|boxed| boxed.downcast::<NewrenoState>().ok())
        .map(|boxed| *boxed)
        .unwrap_or_default();
    picoquic_newreno_reset(&mut state, path_x);
    path_x.congestion_alg_state = Some(Box::new(state));
}

/// Drive standalone New Reno with a congestion-control notification.
///
/// C: `picoquic_newreno_notify` (picoquic/newreno.c:207-296).
fn picoquic_newreno_notify(
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

    let mut state = match boxed_state.downcast::<NewrenoState>() {
        Ok(state) => *state,
        Err(boxed_state) => {
            path_x.congestion_alg_state = Some(boxed_state);
            return;
        }
    };

    match notification {
        CongestionNotification::Acknowledgement => {
            if state.nrss.alg_state == NewRenoAlgState::SlowStart && state.nrss.ssthresh == u64::MAX
            {
                path_x.cwin = path_x.update_target_cwin_estimation();
                state.nrss.cwin = path_x.cwin;
            }

            if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
                state
                    .nrss
                    .notify(connection, path_x, notification, ack_state, current_time);
                path_x.cwin = state.nrss.cwin;
            }
        }
        CongestionNotification::SeedCwin => {
            state
                .nrss
                .notify(connection, path_x, notification, ack_state, current_time);
            path_x.cwin = state.nrss.cwin;
        }
        CongestionNotification::EcnEc
        | CongestionNotification::Repeat
        | CongestionNotification::Timeout => {
            state
                .nrss
                .notify(connection, path_x, notification, ack_state, current_time);
            path_x.cwin = state.nrss.cwin;
        }
        CongestionNotification::SpuriousRepeat => {
            state
                .nrss
                .notify(connection, path_x, notification, ack_state, current_time);
            path_x.cwin = state.nrss.cwin;
            path_x.is_ssthresh_initialized = true;
        }
        CongestionNotification::RttMeasurement
            if state.nrss.alg_state == NewRenoAlgState::SlowStart
                && state.nrss.ssthresh == u64::MAX =>
        {
            if path_x.rtt_min > TARGET_RENO_RTT {
                path_x.cwin = path_x.update_cwin_for_long_rtt();
                state.nrss.cwin = path_x.cwin;
            }

            let rtt = if connection.is_time_stamp_enabled {
                ack_state.one_way_delay
            } else {
                ack_state.rtt_measurement
            };
            let packet_time = connection
                .paths
                .first()
                .map(|path| path.pacing.packet_time_microsec)
                .unwrap_or_else(|| path_x.pacing.packet_time_microsec);

            if state.rtt_filter.hystart_test(
                rtt,
                Instant::from_ticks(packet_time.ticks()),
                current_time,
                connection.is_time_stamp_enabled,
            ) {
                state.nrss.ssthresh = state.nrss.cwin;
                state.nrss.alg_state = NewRenoAlgState::CongestionAvoidance;
                path_x.cwin = state.nrss.cwin;
                path_x.is_ssthresh_initialized = true;
            }
        }
        CongestionNotification::Reset => {
            picoquic_newreno_reset(&mut state, path_x);
        }
        _ => {}
    }

    path_x.update_pacing_data(
        (state.nrss.alg_state == NewRenoAlgState::SlowStart && state.nrss.ssthresh == u64::MAX)
            as i32,
    );
    let is_primary_path = connection
        .paths
        .first()
        .is_some_and(|path| path.unique_path_id == path_x.unique_path_id);
    connection.report_pacing_update_for_path(path_x, is_primary_path);
    path_x.congestion_alg_state = Some(Box::new(state));
}

impl NewrenoState {
    /// Observe the current algorithm phase and slow-start threshold.
    ///
    /// Returns `(cc_state, cc_param)` where `cc_state` is the numeric
    /// discriminant of [`crate::cc_common::NewRenoAlgState`] (`SlowStart = 0`,
    /// `CongestionAvoidance = 1`) and `cc_param` is `ssthresh` (reported as
    /// `0` when `ssthresh == u64::MAX`, meaning the threshold has not yet
    /// been set).
    ///
    /// C: `picoquic_newreno_observe` (picoquic/newreno.c:309-314).
    pub fn observe(&self) -> (u64, u64) {
        let cc_state = self.nrss.alg_state as u64;
        let cc_param = if self.nrss.ssthresh == u64::MAX {
            0
        } else {
            self.nrss.ssthresh
        };
        (cc_state, cc_param)
    }
}

/// Congestion-control vtable adapter for standalone New Reno.
pub struct NewrenoCongestionControl;

impl CongestionControl for NewrenoCongestionControl {
    fn alg_init(&self, path_x: &mut Path, option_string: Option<&str>, current_time: Instant) {
        picoquic_newreno_init(path_x, option_string, current_time);
    }

    fn alg_notify(
        &self,
        connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: Instant,
    ) {
        picoquic_newreno_notify(connection, path_x, notification, ack_state, current_time);
    }

    fn alg_delete(&self, path_x: &mut Path) {
        path_x.congestion_alg_state = None;
    }

    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|state| state.downcast_ref::<NewrenoState>())
            .map(NewrenoState::observe)
    }
}

pub static NEWRENO_CONTROL: NewrenoCongestionControl = NewrenoCongestionControl;
