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

## Pair `picoquic/bbr.c:BBRInitRandom`
C: `picoquic/bbr.c:411-432 BBRInitRandom`
Rust: `rs/fq/src/bbr.rs:565-575 init_random`

### C body
```c
{
    uint64_t random_context = 0xfedcba9876543210ull;
    random_context ^= current_time;
    if (path_x->cnx->client_mode) {
        random_context += 0x0123456789abcdefull;
    }
    if (path_x->unique_path_id > 0 && path_x->unique_path_id != UINT64_MAX) {
        random_context *= (path_x->unique_path_id + 1);
    }
    bbr_state->random_context = random_context;
}
```

### Rust body
```rust
    pub fn init_random(&mut self, connection: &Connection, path_x: &Path, current_time: u64) {
        let mut ctx: u64 = 0xfedcba9876543210;
        ctx ^= current_time;
        if connection.client_mode {
            ctx = ctx.wrapping_add(0x0123456789abcdef);
        }
        if path_x.unique_path_id > 0 && path_x.unique_path_id != u64::MAX {
            ctx = ctx.wrapping_mul(path_x.unique_path_id.wrapping_add(1));
        }
        self.random_context = ctx;
    }
```

## Pair `picoquic/bbr.c:picoquic_bbr_reset`
C: `picoquic/bbr.c:598-601 picoquic_bbr_reset`
Rust: `rs/fq/src/bbr.rs:2526-2534 picoquic_bbr_reset`

### C body
```c
{
    BBROnInit(bbr_state, path_x, current_time, bbr_state->option_string);
}
```

### Rust body
```rust
) {
    let option_string = bbr_state.option_string.clone();
    bbr_state.on_init(connection, path_x, current_time, option_string);
}
```

## Pair `picoquic/bbr.c:BBRProbeRTTCwnd`
C: `picoquic/bbr.c:673-680 BBRProbeRTTCwnd`
Rust: `rs/fq/src/bbr.rs:1296-1300 probe_rtt_cwnd`

### C body
```c
{
    uint64_t probe_rtt_cwnd = BBRBDPMultiple( bbr_state, path_x, BBRProbeRTTCwndGain);
    if (probe_rtt_cwnd < BBRMinPipeCwnd * path_x->send_mtu) {
        probe_rtt_cwnd = BBRMinPipeCwnd * path_x->send_mtu;
    }
    return probe_rtt_cwnd;
}
```

### Rust body
```rust
    fn probe_rtt_cwnd(&mut self, path_x: &Path) -> u64 {
        const BBR_MIN_PIPE_CWND: u64 = 4;
        let cwnd = self.bdp_multiple(path_x, BBR_PROBE_RTT_CWND_GAIN);
        cwnd.max(BBR_MIN_PIPE_CWND * path_x.send_mtu as u64)
    }
```

## Pair `picoquic/bbr.c:BBRRestoreCwnd`
C: `picoquic/bbr.c:736-744 BBRRestoreCwnd`
Rust: `rs/fq/src/bbr.rs:687-714 restore_cwnd`

### C body
```c
{
    if (bbr_state->prior_cwnd > path_x->cwin) {
        return bbr_state->prior_cwnd;
    }
    else {
        return path_x->cwin;
    }
}
```

### Rust body
```rust
    pub fn set_pacing_rate_with_gain(&mut self, pacing_gain: f64) {
        let rate = pacing_gain * (self.bw * (100 - BBR_PACING_MARGIN_PERCENT)) as f64 / 100.0;
        let rate = if self.state == BbrAlgState::StartupResume
            && !self.filled_pipe
            && self.bdp_seed > 0
            && self.min_rtt > 0
            && self.min_rtt != u64::MAX
        {
            let bdp_rate = self.bdp_seed as f64 * 1_000_000.0 / self.min_rtt as f64;
            rate.max(bdp_rate)
        } else {
            rate
        };
        if self.filled_pipe || rate > self.pacing_rate {
            self.pacing_rate = rate;
        }
    }
```

## Pair `picoquic/bbr.c:BBROnEnterRTO`
C: `picoquic/bbr.c:790-806 BBROnEnterRTO`
Rust: `rs/fq/src/bbr.rs:2300-2311 on_enter_rto`

### C body
```c
{
    if (!bbr_state->is_in_recovery) {
        bbr_state->prior_cwnd = BBRSaveCwnd(bbr_state, path_x);
        bbr_state->is_in_recovery = 1;
    }
    if (!bbr_state->is_pto_recovery) {
        path_x->cwin = path_x->bytes_in_transit + path_x->send_mtu;
        bbr_state->recovery_packet_number = lost_packet_number;
        bbr_state->is_pto_recovery = 1;
        bbr_state->recovery_delivered = path_x->delivered;
    }
}
```

### Rust body
```rust
    fn on_enter_rto(&mut self, path_x: &mut Path, lost_packet_number: u64) {
        if !self.is_in_recovery {
            self.prior_cwnd = self.save_cwnd(path_x);
            self.is_in_recovery = true;
        }
        if !self.is_pto_recovery {
            path_x.cwin = path_x.bytes_in_transit + path_x.send_mtu as u64;
            self.recovery_packet_number = lost_packet_number;
            self.is_pto_recovery = true;
            self.recovery_delivered = path_x.delivered;
        }
    }
```

## Pair `picoquic/bbr.c:BBRCheckRecovery`
C: `picoquic/bbr.c:852-866 BBRCheckRecovery`
Rust: `rs/fq/src/bbr.rs:1474-1491 check_recovery`

### C body
```c
{
    if (InLossRecovery(bbr_state)) {
        /* Exit loss recovery if full roundtrip expired */
        if (picoquic_cc_get_ack_number(path_x->cnx, path_x) >= bbr_state->recovery_packet_number) {
            BBROnExitRecovery(bbr_state, path_x, current_time);
        }
    }
    else {
        /* Enter loss recovery if new losses */
        if (IsInflightTooHigh(bbr_state, path_x, rs)) {
            BBROnEnterFastRecovery(bbr_state, path_x, rs);
        }
    }
}
```

### Rust body
```rust
        if self.in_loss_recovery() {
            // Mirror cc_get_ack_number multipath logic.
            let ack_number = if connection.is_multipath_enabled {
                path_x.pkt_ctx.highest_acknowledged
            } else {
                connection.pkt_ctx[PacketContext::Application as usize].highest_acknowledged
            };
            if ack_number >= self.recovery_packet_number {
                self.on_exit_recovery(connection, path_x, current_time);
            }
        } else if self.is_inflight_too_high(path_x, rs) {
```

## Pair `picoquic/bbr.c:BBRQuantizationBudget`
C: `picoquic/bbr.c:888-901 BBRQuantizationBudget`
Rust: `rs/fq/src/bbr.rs:1538-1548 quantization_budget`

### C body
```c
{
    BBRUpdateOffloadBudget(bbr_state);
    if (inflight < bbr_state->offload_budget) {
        inflight = bbr_state->offload_budget;
    }
    if (inflight < BBRMinPipeCwnd * path_x->send_mtu) {
        inflight = BBRMinPipeCwnd * path_x->send_mtu;
    }
    if (bbr_state->state == picoquic_bbr_alg_probe_bw_up) {
        inflight += 2*path_x->send_mtu;
    }
    return inflight;
}
```

### Rust body
```rust
    fn quantization_budget(&mut self, path_x: &Path, inflight: u64) -> u64 {
        const BBR_MIN_PIPE_CWND: u64 = 4;
        self.update_offload_budget();
        let inflight = inflight.max(self.offload_budget);
        let inflight = inflight.max(BBR_MIN_PIPE_CWND * path_x.send_mtu as u64);
        if self.state == BbrAlgState::ProbeBwUp {
            inflight + 2 * path_x.send_mtu as u64
        } else {
            inflight
        }
    }
```

## Pair `picoquic/bbr.c:BBRInitPacingRate`
C: `picoquic/bbr.c:932-942 BBRInitPacingRate`
Rust: `rs/fq/src/bbr.rs:549-552 init_pacing_rate`

### C body
```c
{
    /* nominal_bandwidth = InitialCwnd / (SRTT ? SRTT : 1ms); */
    uint64_t initial_rtt = PICOQUIC_INITIAL_RTT; /* 1ms */
    if (path_x->smoothed_rtt != PICOQUIC_INITIAL_RTT || path_x->rtt_variant != 0) {
        initial_rtt = path_x->smoothed_rtt;
    }
    double nominal_bandwidth = ((double)(1000000ull * PICOQUIC_CWIN_INITIAL)) / (double)initial_rtt;
    bbr_state->pacing_rate = BBRStartupPacingGain * nominal_bandwidth;
}
```

### Rust body
```rust
        let initial_rtt = if path_x.smoothed_rtt != INITIAL_RTT || path_x.rtt_variant.ticks() != 0 {
            path_x.smoothed_rtt.ticks()
        } else {
```

## Pair `picoquic/bbr.c:BBRUpdateLatestDeliverySignals`
C: `picoquic/bbr.c:985-1004 BBRUpdateLatestDeliverySignals`
Rust: `rs/fq/src/bbr.rs:754-767 update_latest_delivery_signals`

### C body
```c
{
    /* BBR.bw_latest = max(BBR.bw_latest, rs.delivery_rate) */
    bbr_state->loss_round_start = 0;
    if (bbr_state->bw_latest < rs->delivery_rate) {
        bbr_state->bw_latest = rs->delivery_rate;
    }
    /* BBR.inflight_latest = max(BBR.inflight_latest, rs.delivered) */
    if (bbr_state->inflight_latest < rs->delivered) {
        bbr_state->inflight_latest = rs->delivered;
    }
    
    uint64_t prior_delivered = path_x->delivered - rs->delivered;
    if (prior_delivered >= bbr_state->loss_round_delivered) {
        bbr_state->loss_round_delivered = path_x->delivered;
        bbr_state->loss_round_start = 1;
    }
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

## Pair `picoquic/bbr.c:BBRLossLowerBounds`
C: `picoquic/bbr.c:1035-1048 BBRLossLowerBounds`
Rust: `rs/fq/src/bbr.rs:612-615 loss_lower_bounds`

### C body
```c
{
    /* set: bw_lo = max(bw_latest, bw_lo*BBRBeta) */
    bbr_state->bw_lo = (uint64_t)(BBRBeta * (double)bbr_state->bw_lo);
    if (bbr_state->bw_lo < bbr_state->bw_latest) {
        bbr_state->bw_lo = bbr_state->bw_latest;
    }
    /* Set: inflight_lo = max(inflight_latest, BBRBeta * bbr_state->inflight_lo) */
    bbr_state->inflight_lo = (uint64_t)(BBRBeta * (double)bbr_state->inflight_lo);
    if (bbr_state->inflight_lo < bbr_state->inflight_latest) {
        bbr_state->inflight_lo = bbr_state->inflight_latest;
    }
}
```

### Rust body
```rust
    pub fn loss_lower_bounds(&mut self) {
        self.bw_lo = ((BBR_BETA * self.bw_lo as f64) as u64).max(self.bw_latest);
        self.inflight_lo = ((BBR_BETA * self.inflight_lo as f64) as u64).max(self.inflight_latest);
    }
```

## Pair `picoquic/bbr.c:BBRBoundBWForModel`
C: `picoquic/bbr.c:1094-1104 BBRBoundBWForModel`
Rust: `rs/fq/src/bbr.rs:362-371 bound_bw_for_model`

### C body
```c
static void  BBRBoundBWForModel(picoquic_bbr_state_t* bbr_state) {
    /* set bw = min(max_bw, bw_lo, bw_hi)  */
    bbr_state->bw = bbr_state->max_bw;
    if (bbr_state->bw > bbr_state->bw_lo) {
        bbr_state->bw = bbr_state->bw_lo;
    }
    /* TODO: remove the test bw_hi != 0 once variables properly initialized. */
    if (bbr_state->bw > bbr_state->bw_hi && bbr_state->bw_hi != 0) {
        bbr_state->bw = bbr_state->bw_hi;
    }
}
```

### Rust body
```rust
        if self.bw > self.bw_hi && self.bw_hi != 0 {
            self.bw = self.bw_hi;
        }
```

## Pair `picoquic/bbr.c:IsInflightTooHigh`
C: `picoquic/bbr.c:1149-1172 IsInflightTooHigh`
Rust: `rs/fq/src/bbr.rs:925-934 is_inflight_too_high`

### C body
```c
{
    if (rs->ecn_alpha > BBRExcessiveEcnCE) {
        return 1;
    }
    else {
        uint64_t rs_delivered = path_x->delivered - rs->delivered;
        if (rs_delivered > bbr_state->recovery_delivered &&
            rs->lost > (uint64_t)(((double)rs->tx_in_flight) * BBRLossThresh) &&
            rs->lost > 3 * path_x->send_mtu) {
            return 1;
        }
        else {
            return 0;
        }
    }
}
```

### Rust body
```rust
    pub fn is_inflight_too_high(&self, path_x: &Path, rs: &BbrPerAckState) -> bool {
        if rs.ecn_alpha > BBR_EXCESSIVE_ECN_CE {
            return true;
        }
        // rs_delivered = bytes delivered since the affected packets were sent.
        let rs_delivered = path_x.delivered.saturating_sub(rs.delivered);
        rs_delivered > self.recovery_delivered
            && rs.lost > (rs.tx_in_flight as f64 * BBR_LOSS_THRESH) as u64
            && rs.lost > 3 * path_x.send_mtu as u64
    }
```

## Pair `picoquic/bbr.c:BBRStartRound`
C: `picoquic/bbr.c:1212-1217 BBRStartRound`
Rust: `rs/fq/src/bbr.rs:1041-1048 start_round`

### C body
```c
{
    bbr_state->round_start_pn = picoquic_cc_get_sequence_number(path_x->cnx, path_x);

    bbr_state->next_round_delivered = path_x->delivered;
}
```

### Rust body
```rust
    pub(crate) fn start_round(&mut self, connection: &Connection, path_x: &Path) {
        self.round_start_pn = if connection.is_multipath_enabled {
            path_x.pkt_ctx.send_sequence
        } else {
            connection.pkt_ctx[PacketContext::Application as usize].send_sequence
        };
        self.next_round_delivered = path_x.delivered;
    }
```

## Pair `picoquic/bbr.c:BBRResetRTTJitterBuffer`
C: `picoquic/bbr.c:1322-1330 BBRResetRTTJitterBuffer`
Rust: `rs/fq/src/bbr.rs:673-680 reset_rtt_jitter_buffer`

### C body
```c
{
    bbr_state->rtt_jitter_cycle = 0;
    bbr_state->last_rtt_sample_stamp = current_time;
    bbr_state->rtt_short_term_min = rtt_init_value;
    bbr_state->rtt_short_term_max = rtt_init_value;
    bbr_state->probe_rtt_min_delay = rtt_init_value;
    bbr_state->nb_rtt_excess = 0;
}
```

### Rust body
```rust
    pub fn reset_rtt_jitter_buffer(&mut self, rtt_init_value: u64, current_time: u64) {
        self.rtt_jitter_cycle = 0;
        self.last_rtt_sample_stamp = current_time;
        self.rtt_short_term_min = rtt_init_value;
        self.rtt_short_term_max = rtt_init_value;
        self.probe_rtt_min_delay = rtt_init_value;
        self.nb_rtt_excess = 0;
    }
```

## Pair `picoquic/bbr.c:BBRCheckProbeRTTDone`
C: `picoquic/bbr.c:1444-1454 BBRCheckProbeRTTDone`
Rust: `rs/fq/src/bbr.rs:1426-1437 check_probe_rtt_done`

### C body
```c
{
    if (bbr_state->probe_rtt_done_stamp != 0 &&
        current_time > bbr_state->probe_rtt_done_stamp)
    {
        /* schedule next ProbeRTT: */
        bbr_state->probe_rtt_min_stamp = current_time;
        path_x->cwin = BBRRestoreCwnd(bbr_state, path_x);
        BBRExitProbeRTT(bbr_state, path_x, current_time);
    }
}
```

### Rust body
```rust
    ) {
        if self.probe_rtt_done_stamp != 0 && current_time > self.probe_rtt_done_stamp {
            self.probe_rtt_min_stamp = current_time;
            path_x.cwin = self.restore_cwnd(path_x);
            self.exit_probe_rtt(connection, path_x, current_time);
        }
    }
```

## Pair `picoquic/bbr.c:IsInAProbeBWState`
C: `picoquic/bbr.c:1522-1530 IsInAProbeBWState`
Rust: `rs/fq/src/bbr.rs:781-798 is_in_a_probe_bw_state`

### C body
```c
{
    picoquic_bbr_alg_state_t state = bbr_state->state;

    return (state == picoquic_bbr_alg_probe_bw_down ||
        state == picoquic_bbr_alg_probe_bw_cruise ||
        state == picoquic_bbr_alg_probe_bw_refill ||
        state == picoquic_bbr_alg_probe_bw_up);
}
```

### Rust body
```rust
    pub fn update_offload_budget(&mut self) {
        self.offload_budget = 3 * self.send_quantum;
    }
```

## Pair `picoquic/bbr.c:BBRProbeInflightHiUpward`
C: `picoquic/bbr.c:1570-1587 BBRProbeInflightHiUpward`
Rust: `rs/fq/src/bbr.rs:1225-1238 probe_inflight_hi_upward`

### C body
```c
{
    if (!rs->is_cwnd_limited || path_x->cwin < bbr_state->inflight_hi)
    {
        return;  /* not fully using inflight_hi, so don't grow it */
    }
    bbr_state->bw_probe_up_acks += rs->newly_acked;
    if (bbr_state->bw_probe_up_acks >= bbr_state->bw_probe_up_cnt*path_x->send_mtu) {
        uint64_t delta = (bbr_state->bw_probe_up_acks / bbr_state->bw_probe_up_cnt);
        bbr_state->bw_probe_up_acks -= delta * bbr_state->bw_probe_up_cnt;
        bbr_state->inflight_hi += delta;
    }

    if (bbr_state->round_start){
        BBRRaiseInflightHiSlope(bbr_state, path_x);
    }
}
```

### Rust body
```rust
    pub(crate) fn probe_inflight_hi_upward(&mut self, path_x: &Path, rs: &BbrPerAckState) {
        if !rs.is_cwnd_limited || path_x.cwin < self.inflight_hi {
            return; // not fully using inflight_hi, so don't grow it
        }
        self.bw_probe_up_acks += rs.newly_acked;
        if self.bw_probe_up_acks >= self.bw_probe_up_cnt as u64 * path_x.send_mtu as u64 {
            let delta = self.bw_probe_up_acks / self.bw_probe_up_cnt as u64;
            self.bw_probe_up_acks -= delta * self.bw_probe_up_cnt as u64;
            self.inflight_hi += delta;
        }
        if self.round_start {
            self.raise_inflight_hi_slope(path_x);
        }
    }
```

## Pair `picoquic/bbr.c:BBRPickProbeWait`
C: `picoquic/bbr.c:1658-1673 BBRPickProbeWait`
Rust: `rs/fq/src/bbr.rs:1012-1020 pick_probe_wait`

### C body
```c
{
    /* Decide random round-trip bound for wait: */
    bbr_state->rounds_since_bw_probe =
        (uint32_t)BBRRandomIntBetween(bbr_state, 0, 1); /* 0 or 1 */
    
    /* Decide the random wall clock bound for wait: */
    if (bbr_state->min_rtt < BBRLongRttThreshold) {
        bbr_state->bw_probe_wait =
            2000000 + BBRRandomIntBetween(bbr_state, 0, 1000000); /* 0..1 sec, in usec */
    }
    else {
        bbr_state->bw_probe_wait =
            8*bbr_state->min_rtt + BBRRandomIntBetween(bbr_state, 0, 4*bbr_state->min_rtt); /* 0..1 sec, in usec */
    }
}
```

### Rust body
```rust
    fn pick_probe_wait(&mut self) {
        const BBR_LONG_RTT_THRESHOLD: u64 = 250_000;
        self.rounds_since_bw_probe = self.random_int_between(0, 1) as u32;
        self.bw_probe_wait = if self.min_rtt < BBR_LONG_RTT_THRESHOLD {
            2_000_000 + self.random_int_between(0, 1_000_000)
        } else {
            8 * self.min_rtt + self.random_int_between(0, 4 * self.min_rtt)
        };
    }
```

## Pair `picoquic/bbr.c:BBRCheckAppLimitedEnded`
C: `picoquic/bbr.c:1724-1766 BBRCheckAppLimitedEnded`
Rust: `rs/fq/src/bbr.rs:434-448 check_app_limited_ended`

### C body
```c
{
    int app_limited_ended = 0;
    if (bbr_state->round_start) {
        if (bbr_state->app_limited_this_round) {
            bbr_state->app_limited_round_count++;
        }
        else
        {
            app_limited_ended =
                (bbr_state->app_limited_round_count > BBRAppLimitedRoundsThreshold);
            bbr_state->app_limited_round_count = 0;
        }
        bbr_state->app_limited_this_round = 0;
    }
    else {
        bbr_state->app_limited_this_round |= rs->is_app_limited;
    }
    return app_limited_ended;
}
```

### Rust body
```rust
    pub fn check_app_limited_ended(&mut self, rs: &BbrPerAckState) -> bool {
        let mut app_limited_ended = false;
        if self.round_start {
            if self.app_limited_this_round != 0 {
                self.app_limited_round_count += 1;
            } else {
                app_limited_ended = self.app_limited_round_count > BBR_APP_LIMITED_ROUNDS_THRESHOLD;
                self.app_limited_round_count = 0;
            }
            self.app_limited_this_round = 0;
        } else {
            self.app_limited_this_round |= rs.is_app_limited as i32;
        }
        app_limited_ended
    }
```

## Pair `picoquic/bbr.c:BBRStartProbeBW_DOWN`
C: `picoquic/bbr.c:1793-1814 BBRStartProbeBW_DOWN`
Rust: `rs/fq/src/bbr.rs:1052-1077 start_probe_bw_down`

### C body
```c
{
    bbr_state->pacing_gain = BBRProbeBwDownPacingGain;  /* pace a bit slowly */
    bbr_state->cwnd_gain = BBRProbeBwDownCwndGain;   /* maintain cwnd */
    BBRResetCongestionSignals(bbr_state);
    bbr_state->bw_probe_up_cnt = UINT32_MAX; /* not growing inflight_hi */
    if (bbr_state->probe_probe_bw_quickly && BBRExpTest(bbr_state, do_rapid_start)) {
        BBRPickProbeWaitEarly(bbr_state);
    }
    else {
        BBRPickProbeWait(bbr_state);
    }
    bbr_state->cycle_stamp = current_time;  /* start wall clock */
    bbr_state->ack_phase = picoquic_bbr_acks_probe_stopping;
    BBRStartRound(bbr_state, path_x);
    bbr_state->state = picoquic_bbr_alg_probe_bw_down;
    bbr_state->nb_rtt_excess = 0;
    bbr_state->app_limited_round_count = 0;
    bbr_state->app_limited_this_round = 0;

    path_x->is_cca_probing_up = 0;
}
```

### Rust body
```rust
    ) {
        const BBR_PROBE_BW_DOWN_PACING_GAIN: f64 = 0.9;
        const BBR_PROBE_BW_DOWN_CWND_GAIN: f64 = 2.0;
        self.pacing_gain = BBR_PROBE_BW_DOWN_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_DOWN_CWND_GAIN;
        self.reset_congestion_signals();
        self.bw_probe_up_cnt = u32::MAX; // not growing inflight_hi
        if self.probe_probe_bw_quickly && self.exp_flags.do_rapid_start {
            self.pick_probe_wait_early();
        } else {
            self.pick_probe_wait();
        }
        self.cycle_stamp = current_time;
        self.ack_phase = BbrAckPhase::ProbeStopping;
        self.start_round(connection, path_x);
        self.state = BbrAlgState::ProbeBwDown;
        self.nb_rtt_excess = 0;
        self.app_limited_round_count = 0;
        self.app_limited_this_round = 0;
        path_x.is_cca_probing_up = false;
    }
```
