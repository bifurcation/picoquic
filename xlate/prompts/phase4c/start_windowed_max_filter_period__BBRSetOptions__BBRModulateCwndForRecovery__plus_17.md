# Phase 4C body-only translation audit

Compare each C/Rust pair using only the function bodies shown
below. Do not infer from dependencies, type definitions, callers,
module context, tests, or external knowledge. This is a cheap
superficial check for obvious inconsistencies.

Return only JSON with this shape:

```json
{"reviews":[{"c_id":"...","status":"ok|suspect|definitely_not_ok","rationale":"body-visible reason"}]}
```

Status meanings:
* `ok`: no obvious body-level concern.
* `suspect`: possible mismatch visible from the bodies.
* `definitely_not_ok`: clear mismatch or placeholder-like code.

## Pair `picoquic/bbr.c:start_windowed_max_filter_period`
C: `picoquic/bbr.c:394-397 start_windowed_max_filter_period`
Rust: `rs/fq/src/bbr.rs:2496-2498 start_windowed_max_filter_period`

### C body
```c
{
    filter[cycle % filterLen] = 0;
}
```

### Rust body
```rust
pub fn start_windowed_max_filter_period(filter: &mut [u64], cycle: u32, filter_len: usize) {
    filter[(cycle as usize) % filter_len] = 0;
}
```

## Pair `picoquic/bbr.c:BBRSetOptions`
C: `picoquic/bbr.c:441-556 BBRSetOptions`
Rust: `rs/fq/src/bbr.rs:855-917 set_options`

### C body
```c
{
    const char* x = bbr_state->option_string;
#ifdef BBRExperiment
    bbr_state->exp_flags.do_early_exit = 1;
    bbr_state->exp_flags.do_rapid_start = 1;
    bbr_state->exp_flags.do_handle_suspension = 1;
    bbr_state->exp_flags.do_control_lost = 1;
    bbr_state->exp_flags.do_exit_probeBW_up_on_delay = 1;
    bbr_state->exp_flags.do_enter_probeBW_after_limited = 1;
#endif

    if (x != NULL) {
        char c;
        while ((c = *x) != 0) {
            x++;
            switch (c) {
#ifdef BBRExperiment
            case 'E':
                bbr_state->exp_flags.do_early_exit = 0;
                break;
            case 'R':
                bbr_state->exp_flags.do_rapid_start = 0;
                break;
            case 'H':
                bbr_state->exp_flags.do_handle_suspension = 0;
                break;
            case 'L':
                bbr_state->exp_flags.do_control_lost = 0;
                break;
            case 'D':
                bbr_state->exp_flags.do_exit_probeBW_up_on_delay = 0;
                break;
            case 'A':
                bbr_state->exp_flags.do_enter_probeBW_after_limited = 0;
                break;
#endif
            case 'T': {
                /* Reading digits into an uint64_t  */
                uint64_t u = 0;
                while ((c = *x) != 0) {
                    if (c >= '0' && c <= '9') {
                        u *= 10;
                        u += c - '0';
                        x++;
                    }
                    else {
                        break;
                    }
                }
                bbr_state->wifi_shadow_rtt = u;
                break;
            }
            case 'Q': {
                /* Reading digits ad one dot into a double  */
                while ((c = *x) != 0) {
                    double d = 0;
                    double div = 1.0;
                    int dotted = 0;
                    while ((c = *x) != 0) {
                        if (c >= '0' && c <= '9') {
                            if (!dotted) {
                                d *= 10;
                                d += c - '0';
                            }
                            else {
                                div /= 10.0;
                                d += div * (c - '0');
                            }
                            x++;
                        }
                        else if (c == '.') {
                            if (dotted) {
                                break;
                            }
                            else {
                                dotted = 1;
                                x++;
                            }
                        }
                        else {
                            break;
                        }
                    }
                    bbr_state->quantum_ratio = d;
                    break;
                }
            }
            case ':':
                /* Ignore */
                break;
            default:
                break;
            }
        }
    }
}
```

### Rust body
```rust
    pub fn set_options(&mut self) {
        // BBRExperiment is always on.
        self.exp_flags.do_early_exit = true;
        self.exp_flags.do_rapid_start = true;
        self.exp_flags.do_handle_suspension = true;
        self.exp_flags.do_control_lost = true;
        self.exp_flags.do_exit_probe_bw_up_on_delay = true;
        self.exp_flags.do_enter_probe_bw_after_limited = true;

        let s = match self.option_string.clone() {
            Some(s) => s,
            None => return,
        };
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                'E' => self.exp_flags.do_early_exit = false,
                'R' => self.exp_flags.do_rapid_start = false,
                'H' => self.exp_flags.do_handle_suspension = false,
                'L' => self.exp_flags.do_control_lost = false,
                'D' => self.exp_flags.do_exit_probe_bw_up_on_delay = false,
                'A' => self.exp_flags.do_enter_probe_bw_after_limited = false,
                'T' => {
                    let mut u: u64 = 0;
                    while let Some(&d) = chars.peek() {
                        if d.is_ascii_digit() {
                            u = u * 10 + (d as u64 - b'0' as u64);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    self.wifi_shadow_rtt = u;
                }
                'Q' => {
                    let mut d: f64 = 0.0;
                    let mut div: f64 = 1.0;
                    let mut dotted = false;
                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_digit() {
                            if !dotted {
                                d = d * 10.0 + (ch as u8 - b'0') as f64;
                            } else {
                                div /= 10.0;
                                d += div * (ch as u8 - b'0') as f64;
                            }
                            chars.next();
                        } else if ch == '.' {
                            if dotted {
                                break;
                            }
                            dotted = true;
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    self.quantum_ratio = d;
                }
                _ => {}
            }
        }
    }
```

## Pair `picoquic/bbr.c:BBRModulateCwndForRecovery`
C: `picoquic/bbr.c:627-644 BBRModulateCwndForRecovery`
Rust: `rs/fq/src/bbr.rs:622-633 modulate_cwnd_for_recovery`

### C body
```c
{
    if (rs->newly_lost > 0) {
        if (path_x->cwin > rs->newly_lost + path_x->send_mtu) {
            path_x->cwin = path_x->cwin - rs->newly_lost;
        }
        else {
            path_x->cwin = path_x->send_mtu;
        }
    }
    if (bbr_state->packet_conservation && path_x->cwin < (path_x->bytes_in_transit + rs->newly_acked)) {
        path_x->cwin = path_x->bytes_in_transit + rs->newly_acked;
    }
}
```

### Rust body
```rust
    pub fn modulate_cwnd_for_recovery(&self, path_x: &mut Path, rs: &BbrPerAckState) {
        if rs.newly_lost > 0 {
            if path_x.cwin > rs.newly_lost + path_x.send_mtu as u64 {
                path_x.cwin -= rs.newly_lost;
            } else {
                path_x.cwin = path_x.send_mtu as u64;
            }
        }
        if self.packet_conservation && path_x.cwin < path_x.bytes_in_transit + rs.newly_acked {
            path_x.cwin = path_x.bytes_in_transit + rs.newly_acked;
        }
    }
```

## Pair `picoquic/bbr.c:BBRSetCwnd`
C: `picoquic/bbr.c:692-717 BBRSetCwnd`
Rust: `rs/fq/src/bbr.rs:1846-1868 set_cwnd`

### C body
```c
{
    BBRUpdateMaxInflight(bbr_state, path_x);
    /* TODO: check whether this should be done in every state */
    BBRModulateCwndForRecovery(bbr_state, path_x, rs);
    if (!bbr_state->packet_conservation) {
        if (bbr_state->filled_pipe) {
            path_x->cwin += rs->newly_acked;
            if (path_x->cwin > bbr_state->max_inflight) {
                path_x->cwin = bbr_state->max_inflight;
            }
        }
        else if (bbr_state->state == picoquic_bbr_alg_startup_resume &&
            bbr_state->bdp_seed > path_x->cwin) {
            path_x->cwin = bbr_state->bdp_seed;
        }
        else if (path_x->cwin < bbr_state->max_inflight || path_x->delivered < PICOQUIC_CWIN_INITIAL) {
            path_x->cwin = path_x->cwin+ rs->newly_acked;
        }
        if (path_x->cwin < BBRMinPipeCwnd * path_x->send_mtu) {
            path_x->cwin = BBRMinPipeCwnd * path_x->send_mtu;
        }
    }
    BBRBoundCwndForProbeRTT(bbr_state, path_x);
    BBRBoundCwndForModel(bbr_state, path_x);
}
```

### Rust body
```rust
    pub fn set_cwnd(&mut self, path_x: &mut Path, rs: &BbrPerAckState) {
        const BBR_MIN_PIPE_CWND: u64 = 4;
        self.update_max_inflight(path_x);
        self.modulate_cwnd_for_recovery(path_x, rs);
        if !self.packet_conservation {
            if self.filled_pipe {
                path_x.cwin += rs.newly_acked;
                if path_x.cwin > self.max_inflight {
                    path_x.cwin = self.max_inflight;
                }
            } else if self.state == BbrAlgState::StartupResume && self.bdp_seed > path_x.cwin {
                path_x.cwin = self.bdp_seed;
            } else if path_x.cwin < self.max_inflight || path_x.delivered < CWIN_INITIAL {
                path_x.cwin += rs.newly_acked;
            }
            let floor = BBR_MIN_PIPE_CWND * path_x.send_mtu as u64;
            if path_x.cwin < floor {
                path_x.cwin = floor;
            }
        }
        self.bound_cwnd_for_probe_rtt(path_x);
        self.bound_cwnd_for_model(path_x);
    }
```

## Pair `picoquic/bbr.c:BBREnterLostFeedback`
C: `picoquic/bbr.c:765-780 BBREnterLostFeedback`
Rust: `rs/fq/src/bbr.rs:454-468 exit_lost_feedback`

### C body
```c
{
    if ((IsInAProbeBWState(bbr_state) || bbr_state->state == picoquic_bbr_alg_drain) &&
        !bbr_state->is_handling_lost_feedback &&
        path_x->cnx->cnx_state == picoquic_state_ready) {
        /* Remembering the old cwin, so the state can be restored when the
        * condition is lifted. */
        bbr_state->cwin_before_lost_feedback = path_x->cwin;
        /* setting the congestion window to exactly the bytes in transit, thus
         * preventing any further transmission until the condition is lifted */
        path_x->cwin = path_x->bytes_in_transit;
        bbr_state->is_handling_lost_feedback = 1;
    }
}
```

### Rust body
```rust
    pub(crate) fn has_elapsed_in_phase(&self, interval: u64, current_time: u64) -> bool {
        current_time > self.cycle_stamp + interval
    }
```

## Pair `picoquic/bbr.c:BBROnSpuriousLoss`
C: `picoquic/bbr.c:840-845 BBROnSpuriousLoss`
Rust: `rs/fq/src/bbr.rs:2317-2327 on_spurious_loss`

### C body
```c
{
    if (bbr_state->recovery_packet_number <= lost_packet_number && bbr_state->is_pto_recovery) {
        BBROnExitRecovery(bbr_state, path_x, current_time);
    }
}
```

### Rust body
```rust
    ) {
        if self.recovery_packet_number <= lost_packet_number && self.is_pto_recovery {
            self.on_exit_recovery(connection, path_x, current_time);
        }
    }
```

## Pair `picoquic/bbr.c:BBRBDPMultiple`
C: `picoquic/bbr.c:878-881 BBRBDPMultiple`
Rust: `rs/fq/src/bbr.rs:1084-1116 bdp_multiple`

### C body
```c
{
    return BBRBDPMultipleWithBw(bbr_state, path_x, gain, bbr_state->bw);
}
```

### Rust body
```rust
    pub fn bound_cwnd_for_model(&mut self, path_x: &mut Path) {
        const BBR_MIN_PIPE_CWND: u64 = 4;
        let mut cap = u64::MAX;
        if self.is_in_a_probe_bw_state() && self.state != BbrAlgState::ProbeBwCruise {
            if self.inflight_hi > 0 {
                cap = self.inflight_hi;
            }
        } else if self.state == BbrAlgState::ProbeRtt || self.state == BbrAlgState::ProbeBwCruise {
            cap = self.inflight_with_headroom(path_x);
        }
        if cap > self.inflight_lo {
            cap = self.inflight_lo;
        }
        let floor = BBR_MIN_PIPE_CWND * path_x.send_mtu as u64;
        if cap < floor {
            cap = floor;
        }
        if path_x.cwin > cap {
            path_x.cwin = cap;
        }
    }
```

## Pair `picoquic/bbr.c:BBRInflight`
C: `picoquic/bbr.c:909-912 BBRInflight`
Rust: `rs/fq/src/bbr.rs:1563-1585 inflight`

### C body
```c
{
    return BBRInflightWithBw(bbr_state, path_x, gain, bbr_state->bw);
}
```

### Rust body
```rust
    fn start_probe_bw_refill(&mut self, connection: &Connection, path_x: &mut Path) {
        const BBR_PROBE_BW_REFILL_PACING_GAIN: f64 = 1.0;
        const BBR_PROBE_BW_REFILL_CWND_GAIN: f64 = 2.0;
        self.pacing_gain = BBR_PROBE_BW_REFILL_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_REFILL_CWND_GAIN;
        self.reset_lower_bounds();
        self.bw_probe_up_rounds = 0;
        self.bw_probe_up_acks = 0;
        self.full_bw = self.max_bw;
        self.ack_phase = BbrAckPhase::Refilling;
        self.start_round(connection, path_x);
        self.state = BbrAlgState::ProbeBwRefill;
        path_x.is_cca_probing_up = true;
    }
```

## Pair `picoquic/bbr.c:BBRSetPacingRate`
C: `picoquic/bbr.c:962-965 BBRSetPacingRate`
Rust: `rs/fq/src/bbr.rs:1836-1839 set_pacing_rate`

### C body
```c
{
    BBRSetPacingRateWithGain(bbr_state, bbr_state->pacing_gain);
}
```

### Rust body
```rust
    pub(crate) fn set_pacing_rate(&mut self) {
        let gain = self.pacing_gain;
        self.set_pacing_rate_with_gain(gain);
    }
```

## Pair `picoquic/bbr.c:BBRResetCongestionSignals`
C: `picoquic/bbr.c:1014-1022 BBRResetCongestionSignals`
Rust: `rs/fq/src/bbr.rs:652-665 reset_congestion_signals`

### C body
```c
{
    bbr_state->loss_in_round = 0;
#ifdef RTTJitterBuffer
    bbr_state->rtt_too_high_in_round = 0;
#endif
    bbr_state->bw_latest = 0;
    bbr_state->inflight_latest = 0;
}
```

### Rust body
```rust
    pub fn reset_lower_bounds(&mut self) {
        self.bw_lo = u64::MAX;
        self.inflight_lo = u64::MAX;
    }
```

## Pair `picoquic/bbr.c:BBRUpdateCongestionSignals`
C: `picoquic/bbr.c:1066-1086 BBRUpdateCongestionSignals`
Rust: `rs/fq/src/bbr.rs:2115-2131 update_congestion_signals`

### C body
```c
{
    BBRUpdateMaxBw(bbr_state, path_x, rs);
    if (rs->newly_lost > 0) {
        bbr_state->loss_in_round = 1;
    }
#ifdef RTTJitterBufferAdapt
    if (IsRTTTooHigh(bbr_state)) {
        bbr_state->rtt_too_high_in_round = 1;
    }
#endif
    if (!bbr_state->loss_round_start) {
        return;  /* wait until end of round trip */
    }
    BBRAdaptLowerBoundsFromCongestion(bbr_state, path_x);
    bbr_state->loss_in_round = 0;
#ifdef RTTJitterBufferAdapt
    bbr_state->rtt_too_high_in_round = 0;
#endif
}
```

### Rust body
```rust
    ) {
        self.update_max_bw(connection, path_x, rs);
        if rs.newly_lost > 0 {
            self.loss_in_round = true;
        }
        // RTTJitterBufferAdapt is not defined in this build; skip rtt_too_high_in_round.
        if !self.loss_round_start {
            return;
        }
        self.adapt_lower_bounds_from_congestion(path_x);
        self.loss_in_round = false;
    }
```

## Pair `picoquic/bbr.c:BBRAdvanceMaxBwFilter`
C: `picoquic/bbr.c:1118-1127 BBRAdvanceMaxBwFilter`
Rust: `rs/fq/src/bbr.rs:1002-1008 advance_max_bw_filter`

### C body
```c
{
    bbr_state->cycle_count++;
    bbr_state->ack_phase = picoquic_bbr_acks_probe_starting;

    /* Should the current cycle value be set to zero? Or do we simply rely on the
     * natural rythm of updates, keeping the old value if we only see app limited updates?
     */
    start_windowed_max_filter_period(bbr_state->MaxBwFilter, bbr_state->cycle_count, BBRMaxBwFilterLen);
}
```

### Rust body
```rust
    pub(crate) fn advance_max_bw_filter(&mut self) {
        self.cycle_count += 1;
        self.ack_phase = BbrAckPhase::ProbeStarting;
        // Clear the slot for the new cycle so stale values don't persist.
        let slot = (self.cycle_count as usize) % self.max_bw_filter.len();
        self.max_bw_filter[slot] = 0;
    }
```

## Pair `picoquic/bbr.c:CheckInflightTooHigh`
C: `picoquic/bbr.c:1188-1201 CheckInflightTooHigh`
Rust: `rs/fq/src/bbr.rs:1205-1217 check_inflight_too_high`

### C body
```c
{
    if (IsInflightTooHigh(bbr_state, path_x, rs))
    {
        if (bbr_state->bw_probe_samples)
        {
            BBRHandleInflightTooHigh(bbr_state, path_x, rs, current_time);
        }
        return 1;  /* inflight too high */
    }
    else {
        return 0;
    }
}
```

### Rust body
```rust
        if self.is_inflight_too_high(path_x, rs) {
            if self.bw_probe_samples != 0 {
                self.handle_inflight_too_high(connection, path_x, rs, current_time);
            }
            true
        } else {
```

## Pair `picoquic/bbr.c:BBRAdaptMinRttMargin`
C: `picoquic/bbr.c:1289-1297 BBRAdaptMinRttMargin`
Rust: `rs/fq/src/bbr.rs:317-324 adapt_min_rtt_margin`

### C body
```c
{
    uint64_t margin = ((bbr_state->min_rtt * BBRMinRttMarginPercent) * 100 / 1000000);
    if (bbr_state->max_bw > 0) {
        margin += 2 * path_x->send_mtu * 1000000 / bbr_state->max_bw;
    }
    bbr_state->min_rtt_margin = margin;
}
```

### Rust body
```rust
    pub fn adapt_min_rtt_margin(&mut self, path_x: &Path) {
        let margin = (self.min_rtt * BBR_MIN_RTT_MARGIN_PERCENT) * 100 / 1_000_000;
        let margin = margin
            + (2 * path_x.send_mtu as u64 * 1_000_000)
                .checked_div(self.max_bw)
                .unwrap_or(0);
        self.min_rtt_margin = margin;
    }
```

## Pair `picoquic/bbr.c:IsRTTTooHigh`
C: `picoquic/bbr.c:1386-1389 IsRTTTooHigh`
Rust: `rs/fq/src/bbr.rs:941-953 is_rtt_too_high`

### C body
```c
{
    return (bbr_state->nb_rtt_excess > BBRRTTJitterBufferLen);
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        (self.state as u64, self.bw)
    }
```

## Pair `picoquic/bbr.c:BBREnterProbeRTT`
C: `picoquic/bbr.c:1487-1493 BBREnterProbeRTT`
Rust: `rs/fq/src/bbr.rs:375-390 enter_probe_rtt`

### C body
```c
{
    bbr_state->state = picoquic_bbr_alg_probe_rtt;
    bbr_state->pacing_gain = 1.0;
    bbr_state->cwnd_gain = BBRProbeRTTCwndGain;  /* 0.5 */
    path_x->is_cca_probing_up = 0;
}
```

### Rust body
```rust
    pub fn enter_drain(&mut self, path_x: &mut Path) {
        // Picoquic-specific: notify transport that the startup phase is complete.
        path_x.is_ssthresh_initialized = true;
        self.state = BbrAlgState::Drain;
        self.pacing_gain = 1.0 / BBR_STARTUP_CWND_GAIN; // pace slowly
        self.cwnd_gain = BBR_STARTUP_CWND_GAIN; // maintain cwnd
        path_x.is_cca_probing_up = false;
    }
```

## Pair `picoquic/bbr.c:BBRInflightWithHeadroom`
C: `picoquic/bbr.c:1542-1559 BBRInflightWithHeadroom`
Rust: `rs/fq/src/bbr.rs:582-590 inflight_with_headroom`

### C body
```c
{
    if (bbr_state->inflight_hi == UINT64_MAX) {
        return UINT64_MAX;
    }

    /* This diverges from draft-bbr-02, but is correct per feedback from BBR authors. */
    uint64_t inflight_with_headroom = (uint64_t)((1.0-BBRHeadroom) * ((double)bbr_state->inflight_hi));
    if (inflight_with_headroom < (BBRMinPipeCwnd*path_x->send_mtu)) {
        inflight_with_headroom = BBRMinPipeCwnd*path_x->send_mtu;
    }
    return inflight_with_headroom;
}
```

### Rust body
```rust
    pub fn inflight_with_headroom(&self, path_x: &Path) -> u64 {
        const BBR_HEADROOM: f64 = 0.15;
        const BBR_MIN_PIPE_CWND: u64 = 4;
        if self.inflight_hi == u64::MAX {
            return u64::MAX;
        }
        let headroom = ((1.0 - BBR_HEADROOM) * self.inflight_hi as f64) as u64;
        headroom.max(BBR_MIN_PIPE_CWND * path_x.send_mtu as u64)
    }
```

## Pair `picoquic/bbr.c:BBRCheckTimeToCruise`
C: `picoquic/bbr.c:1626-1636 BBRCheckTimeToCruise`
Rust: `rs/fq/src/bbr.rs:1719-1722 check_time_to_cruise`

### C body
```c
{
    if (path_x->bytes_in_transit > BBRInflightWithHeadroom(bbr_state, path_x)) {
        return 0; /* not enough headroom */
    }
    if (path_x->bytes_in_transit <= BBRInflightWithBw(bbr_state, path_x, 1.0, bbr_state->max_bw)) {
        return 1;  /* inflight <= estimated BDP */
    }
    return 0;
}
```

### Rust body
```rust
        if path_x.bytes_in_transit > self.inflight_with_headroom(path_x) {
            return false; // not enough headroom yet
        }
```

## Pair `picoquic/bbr.c:BBRTargetInflight`
C: `picoquic/bbr.c:1691-1696 BBRTargetInflight`
Rust: `rs/fq/src/bbr.rs:745-767 target_inflight`

### C body
```c
{
    return (bbr_state->bdp < path_x->cwin) ? bbr_state->bdp : path_x->cwin;
}
```

### Rust body
```rust
    pub fn update_latest_delivery_signals(&mut self, path_x: &Path, rs: &BbrPerAckState) {
        self.loss_round_start = false;
        if self.bw_latest < rs.delivery_rate {
            self.bw_latest = rs.delivery_rate;
        }
        if self.inflight_latest < rs.delivered {
            self.inflight_latest = rs.delivered;
        }
        let prior_delivered = path_x.delivered.wrapping_sub(rs.delivered);
        if prior_delivered >= self.loss_round_delivered {
            self.loss_round_delivered = path_x.delivered;
            self.loss_round_start = true;
        }
    }
```

## Pair `picoquic/bbr.c:BBRHasElapsedInPhase`
C: `picoquic/bbr.c:1775-1778 BBRHasElapsedInPhase`
Rust: `rs/fq/src/bbr.rs:466-480 has_elapsed_in_phase`

### C body
```c
static int BBRHasElapsedInPhase(picoquic_bbr_state_t* bbr_state, uint64_t interval, uint64_t current_time) {
    return current_time > bbr_state->cycle_stamp + interval;
}
```

### Rust body
```rust
    pub(crate) fn enter_startup_resume(&mut self) {
        self.state = BbrAlgState::StartupResume;
        self.pacing_gain = BBR_STARTUP_RESUME_PACING_GAIN;
        self.cwnd_gain = BBR_STARTUP_RESUME_CWND_GAIN;
    }
```
