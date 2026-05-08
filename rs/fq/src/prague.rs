//! Translation of `picoquic/prague.c` — Prague L4S congestion control.

use crate::cc_common::{ConnectionCc, MinMaxRtt, PathCc, SMOOTHED_LOSS_THRESHOLD};
use crate::internal::{
    CWIN_INITIAL, CWIN_MINIMUM, Connection, PacketContextState, Path, TARGET_RENO_RTT,
};
use crate::{CongestionControl, CongestionNotification, Instant, PacketContext, PerAckState};

const PRAGUE_SHIFT_G: u32 = 4;

// ---------------------------------------------------------------------------
// Algorithm-state enum.  C: `picoquic_prague_alg_state_t`.

/// C: `picoquic_prague_alg_state_t`
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum PragueAlgState {
    #[default]
    SlowStart = 0,
    CongestionAvoidance,
}

// ---------------------------------------------------------------------------
// Per-path Prague state.  C: `picoquic_prague_state_t`.

/// C: `picoquic_prague_state_t`
pub struct PragueState {
    pub alg_state: PragueAlgState,
    pub alpha: u64,
    pub residual_ack: u64,
    pub ssthresh: u64,
    pub recovery_stamp: u64,
    pub recovery_sequence: u64,
    pub l4s_update_sent: u64,
    pub l4s_epoch_send: u64,
    pub l4s_epoch_ect1: u64,
    pub l4s_epoch_ce: u64,
    pub l4s_packet_ect1: u64,
    pub l4s_packet_ce: u64,
    pub rtt_filter: MinMaxRtt,
}

impl Default for PragueState {
    fn default() -> Self {
        Self {
            alg_state: PragueAlgState::SlowStart,
            alpha: 0,
            residual_ack: 0,
            ssthresh: u64::MAX,
            recovery_stamp: 0,
            recovery_sequence: 0,
            l4s_update_sent: 0,
            l4s_epoch_send: 0,
            l4s_epoch_ect1: 0,
            l4s_epoch_ce: 0,
            l4s_packet_ect1: 0,
            l4s_packet_ce: 0,
            rtt_filter: MinMaxRtt::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers.

/// Select the packet context for L4S ECN accounting: the per-connection
/// application context unless multipath is enabled, in which case the
/// per-path context is used instead.
///
/// C: `picoquic_prague_get_pkt_ctx` (picoquic/prague.c:144-154).
#[allow(dead_code)]
pub(crate) fn prague_get_pkt_ctx<'a>(
    cnx: &'a Connection,
    path_x: &'a Path,
) -> &'a PacketContextState {
    if cnx.is_multipath_enabled {
        &path_x.pkt_ctx
    } else {
        &cnx.pkt_ctx[PacketContext::Application as usize]
    }
}

/// Reset `pr_state` to initial slow-start phase and set `path_x.cwin`
/// to the initial congestion window.
///
/// C: `picoquic_prague_init_reno` (picoquic/prague.c:118-124).
#[allow(dead_code)]
pub(crate) fn prague_init_reno(pr_state: &mut PragueState, path_x: &mut Path) {
    pr_state.alg_state = PragueAlgState::SlowStart;
    pr_state.ssthresh = u64::MAX;
    pr_state.alpha = 0;
    path_x.cwin = CWIN_INITIAL;
}

/// Reset the L4S measurement context to the current packet-context counters.
///
/// C: `picoquic_prague_reset_l3s` (picoquic/prague.c:156-163).
fn prague_reset_l3s(cnx: &Connection, pr_state: &mut PragueState, path_x: &Path) {
    let pkt_ctx = prague_get_pkt_ctx(cnx, path_x);
    pr_state.l4s_epoch_send = pkt_ctx.send_sequence;
    pr_state.l4s_epoch_ect1 = pkt_ctx.ecn_ect1_total_remote;
    pr_state.l4s_epoch_ce = pkt_ctx.ecn_ce_total_remote;
    pr_state.alpha = 0;
}

/// Reset Prague state and reinstall the initial Reno-compatible CWIN.
///
/// C: `picoquic_prague_reset` (picoquic/prague.c:166-170).
fn prague_reset(cnx: &Connection, pr_state: &mut PragueState, path_x: &mut Path) {
    prague_init_reno(pr_state, path_x);
    prague_reset_l3s(cnx, pr_state, path_x);
}

/// Start a new Prague era at the current ECN counters and send sequence.
///
/// C: `picoquic_prague_initialize_era` (picoquic/prague.c:172-184).
fn prague_initialize_era(
    cnx: &Connection,
    path_x: &Path,
    pr_state: &mut PragueState,
    current_time: Instant,
) {
    let pkt_ctx = prague_get_pkt_ctx(cnx, path_x);
    pr_state.l4s_epoch_ect1 = pkt_ctx.ecn_ect1_total_remote;
    pr_state.l4s_epoch_ce = pkt_ctx.ecn_ce_total_remote;
    pr_state.recovery_stamp = current_time.ticks();
    pr_state.recovery_sequence = cnx.sequence_number(path_x);
}

/// Enter Prague recovery, halving CWIN with the configured minimum floor.
///
/// C: `picoquic_prague_enter_recovery` (picoquic/prague.c:186-208).
fn prague_enter_recovery(
    cnx: &Connection,
    path_x: &mut Path,
    pr_state: &mut PragueState,
    current_time: Instant,
) {
    pr_state.ssthresh = (path_x.cwin / 2).max(CWIN_MINIMUM);
    path_x.cwin = pr_state.ssthresh;
    pr_state.alg_state = PragueAlgState::CongestionAvoidance;
    prague_initialize_era(cnx, path_x, pr_state, current_time);
}

/// Update Prague's ECN alpha from the previous era's ECT(1)/CE deltas.
///
/// C: `picoquic_prague_update_alpha` (picoquic/prague.c:210-249).
fn prague_update_alpha(
    cnx: &mut Connection,
    path_x: &Path,
    pr_state: &mut PragueState,
    delta_ect1: u64,
    delta_ce: u64,
    current_time: Instant,
) {
    let mut frac = if delta_ce > 0 {
        delta_ce.saturating_mul(1024) / delta_ce.saturating_add(delta_ect1)
    } else {
        0
    };
    let mut is_suspect = false;

    if pr_state.l4s_update_sent != 0
        && frac >= 512
        && pr_state.alpha < 128
        && current_time.ticks().saturating_sub(pr_state.recovery_stamp)
            > path_x.smoothed_rtt.ticks()
    {
        is_suspect = true;
        frac = 128;
    }

    if delta_ce > 0 || delta_ect1 > 0 {
        if frac > pr_state.alpha && (frac >= 512 || is_suspect) {
            pr_state.alpha = frac;
        } else {
            let alpha_shifted = (pr_state.alpha << PRAGUE_SHIFT_G) - pr_state.alpha + frac;
            pr_state.alpha = alpha_shifted >> PRAGUE_SHIFT_G;
        }
    }

    crate::logger::Log::app_message(
        cnx,
        format_args!(
            "Prague: {},{},{},{},{},{}",
            current_time.ticks(),
            delta_ect1 as i32,
            delta_ce as i32,
            pr_state.alpha as i32,
            path_x.cwin,
            path_x.rtt_sample.ticks()
        ),
    );
}

/// Initialize Prague congestion-control state on a path.
///
/// C: `picoquic_prague_init` (picoquic/prague.c:126-142).
pub fn picoquic_prague_init(
    path_x: &mut Path,
    _option_string: Option<&str>,
    _current_time: Instant,
) {
    let mut state = PragueState::default();
    prague_init_reno(&mut state, path_x);
    path_x.congestion_alg_state = Some(Box::new(state));
}

/// Process an ACK while Prague is in congestion avoidance.
///
/// C: `picoquic_prague_process_ack` (picoquic/prague.c:251-292).
pub fn picoquic_prague_process_ack(
    cnx: &mut Connection,
    path_x: &mut Path,
    pr_state: &mut PragueState,
    ack_state: &PerAckState,
    current_time: Instant,
) {
    let (ecn_ect1_total_remote, ecn_ce_total_remote) = {
        let pkt_ctx = prague_get_pkt_ctx(cnx, path_x);
        (pkt_ctx.ecn_ect1_total_remote, pkt_ctx.ecn_ce_total_remote)
    };
    let next_sequence = cnx.ack_number(path_x);

    if next_sequence > pr_state.recovery_sequence {
        if ecn_ect1_total_remote >= pr_state.l4s_epoch_ect1
            && ecn_ce_total_remote >= pr_state.l4s_epoch_ce
        {
            let delta_ect1 = ecn_ect1_total_remote - pr_state.l4s_epoch_ect1;
            let delta_ce = ecn_ce_total_remote - pr_state.l4s_epoch_ce;

            if delta_ce > 0 || delta_ect1 > 0 {
                prague_update_alpha(cnx, path_x, pr_state, delta_ect1, delta_ce, current_time);

                let delta_cwin = path_x.cwin.saturating_mul(pr_state.alpha) / 2048;
                path_x.cwin = path_x.cwin.saturating_sub(delta_cwin).max(CWIN_MINIMUM);
                pr_state.ssthresh = path_x.cwin;
            }
        }
        prague_initialize_era(cnx, path_x, pr_state, current_time);
    }

    if ecn_ect1_total_remote >= pr_state.l4s_packet_ect1
        && ecn_ce_total_remote >= pr_state.l4s_packet_ce
        && path_x.cwin > 0
    {
        let delta_ect1_ack = ecn_ect1_total_remote - pr_state.l4s_packet_ect1;
        let delta_ce_ack = ecn_ce_total_remote - pr_state.l4s_packet_ce;
        let mut ack_bytes = ack_state.nb_bytes_acknowledged;

        let delta_ack_total = delta_ce_ack.saturating_add(delta_ect1_ack);
        if delta_ack_total > 0 {
            let frac_not_ce = delta_ect1_ack as f64 / delta_ack_total as f64;
            ack_bytes = (frac_not_ce * ack_bytes as f64) as u64;
        }
        path_x.cwin = path_x
            .cwin
            .saturating_add((path_x.send_mtu as u64).saturating_mul(ack_bytes) / path_x.cwin);
    }
}

/// Process an ACK while Prague is still in slow start.
///
/// C: `picoquic_prague_process_start_ack` (picoquic/prague.c:294-317).
pub fn picoquic_prague_process_start_ack(
    cnx: &mut Connection,
    path_x: &mut Path,
    pr_state: &mut PragueState,
    ack_state: &PerAckState,
    current_time: Instant,
) {
    let ecn_ce_total_remote = prague_get_pkt_ctx(cnx, path_x).ecn_ce_total_remote;

    if pr_state.ssthresh == u64::MAX {
        path_x.cwin = path_x.update_target_cwin_estimation();
    }

    if ecn_ce_total_remote > pr_state.l4s_epoch_ce {
        prague_enter_recovery(cnx, path_x, pr_state, current_time);
    } else {
        path_x.cwin = path_x.cwin.saturating_add(path_x.slow_start_increase_ex2(
            cnx,
            ack_state.nb_bytes_acknowledged,
            false,
            pr_state.alpha,
        ));

        if path_x.cwin >= pr_state.ssthresh {
            pr_state.alg_state = PragueAlgState::CongestionAvoidance;
            prague_initialize_era(cnx, path_x, pr_state, current_time);
        }
    }
}

/// Drive Prague with a congestion-control notification.
///
/// C: `picoquic_prague_notify` (picoquic/prague.c:320-394).
pub fn picoquic_prague_notify(
    cnx: &mut Connection,
    path_x: &mut Path,
    notification: CongestionNotification,
    ack_state: &PerAckState,
    current_time: Instant,
) {
    let Some(boxed_state) = path_x.congestion_alg_state.take() else {
        return;
    };

    let mut state = match boxed_state.downcast::<PragueState>() {
        Ok(state) => *state,
        Err(boxed_state) => {
            path_x.congestion_alg_state = Some(boxed_state);
            return;
        }
    };

    match notification {
        CongestionNotification::Acknowledgement => match state.alg_state {
            PragueAlgState::SlowStart => {
                picoquic_prague_process_start_ack(cnx, path_x, &mut state, ack_state, current_time);
            }
            PragueAlgState::CongestionAvoidance => {
                picoquic_prague_process_ack(cnx, path_x, &mut state, ack_state, current_time);
            }
        },
        CongestionNotification::EcnEc => {}
        CongestionNotification::Repeat
            if state.rtt_filter.hystart_loss_test(
                notification,
                ack_state.lost_packet_number,
                SMOOTHED_LOSS_THRESHOLD,
            ) && current_time.ticks().saturating_sub(state.recovery_stamp)
                > path_x.smoothed_rtt.ticks() =>
        {
            prague_enter_recovery(cnx, path_x, &mut state, current_time);
        }
        CongestionNotification::Repeat => {}
        CongestionNotification::Timeout | CongestionNotification::SpuriousRepeat => {}
        CongestionNotification::RttMeasurement
            if state.alg_state == PragueAlgState::SlowStart && state.ssthresh == u64::MAX =>
        {
            if path_x.rtt_min > TARGET_RENO_RTT {
                path_x.cwin = path_x.update_cwin_for_long_rtt();
            }

            let rtt = if cnx.is_time_stamp_enabled {
                ack_state.one_way_delay
            } else {
                ack_state.rtt_measurement
            };
            let packet_time = cnx
                .paths
                .first()
                .map(|path| path.pacing.packet_time_microsec)
                .unwrap_or(path_x.pacing.packet_time_microsec);

            if state.rtt_filter.hystart_test(
                rtt,
                Instant::from_ticks(packet_time.ticks()),
                current_time,
                cnx.is_time_stamp_enabled,
            ) {
                state.ssthresh = path_x.cwin;
                state.alg_state = PragueAlgState::CongestionAvoidance;
                path_x.is_ssthresh_initialized = true;
            }
        }
        CongestionNotification::Reset => {
            prague_reset(cnx, &mut state, path_x);
        }
        _ => {}
    }

    let in_unbounded_slow_start =
        state.alg_state == PragueAlgState::SlowStart && state.ssthresh == u64::MAX;
    path_x.update_pacing_data(in_unbounded_slow_start as i32);
    let is_primary_path = cnx
        .paths
        .first()
        .is_some_and(|path| path.unique_path_id == path_x.unique_path_id);
    cnx.report_pacing_update_for_path(path_x, is_primary_path);
    path_x.congestion_alg_state = Some(Box::new(state));
}

// ---------------------------------------------------------------------------
// Public API.

impl PragueState {
    /// Observe the current algorithm phase and slow-start threshold.
    ///
    /// Returns `(cc_state, cc_param)` where `cc_state` is the numeric
    /// discriminant of [`PragueAlgState`] and `cc_param` is `ssthresh`
    /// (reported as `0` when `ssthresh == u64::MAX`, meaning the threshold
    /// has not yet been set).
    ///
    /// C: `picoquic_prague_observe` (picoquic/prague.c:407-412).
    pub fn observe(&self) -> (u64, u64) {
        let cc_state = self.alg_state as u64;
        let cc_param = if self.ssthresh == u64::MAX {
            0
        } else {
            self.ssthresh
        };
        (cc_state, cc_param)
    }
}

/// Congestion-control vtable adapter for Prague.
pub struct PragueCongestionControl;

impl CongestionControl for PragueCongestionControl {
    fn alg_init(
        &self,
        _connection: &mut Connection,
        path_x: &mut Path,
        option_string: Option<&str>,
        current_time: Instant,
    ) {
        picoquic_prague_init(path_x, option_string, current_time);
    }

    fn alg_notify(
        &self,
        connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: Instant,
    ) {
        picoquic_prague_notify(connection, path_x, notification, ack_state, current_time);
    }

    fn alg_delete(&self, path_x: &mut Path) {
        path_x.congestion_alg_state = None;
    }

    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|state| state.downcast_ref::<PragueState>())
            .map(PragueState::observe)
    }
}

pub static PRAGUE_CONTROL: PragueCongestionControl = PragueCongestionControl;
