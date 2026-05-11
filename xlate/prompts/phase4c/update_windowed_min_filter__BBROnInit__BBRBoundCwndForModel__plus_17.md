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

## Pair `picoquic/bbr.c:update_windowed_min_filter`
C: `picoquic/bbr.c:399-408 update_windowed_min_filter`
Rust: `rs/fq/src/bbr.rs:2505-2519 update_windowed_min_filter`

### C body
```c
{
    filter[cycle % filterLen] = v;
    for (unsigned int i = 0; i < filterLen; i++) {
        if (filter[i] < v) {
            v = filter[i];
        }
    }
    return v;
}
```

### Rust body
```rust
) -> u64 {
    filter[(cycle as usize) % filter_len] = v;
    let mut result = v;
    for &slot in &filter[..filter_len] {
        if slot < result {
            result = slot;
        }
    }
    result
}
```

## Pair `picoquic/bbr.c:BBROnInit`
C: `picoquic/bbr.c:558-596 BBROnInit`
Rust: `rs/fq/src/bbr.rs:1890-1927 on_init`

### C body
```c
{
    /* TODO:
    init_windowed_max_filter(filter = BBR.MaxBwFilter, value = 0, time = 0)
    */
    memset(bbr_state, 0, sizeof(picoquic_bbr_state_t));
    bbr_state->option_string = option_string;

    BBRInitRandom(bbr_state, path_x, current_time);
    /* If RTT was already sampled, use it, other wise set min RTT to infinity */
    if (path_x->smoothed_rtt == PICOQUIC_INITIAL_RTT
        && path_x->rtt_variant == 0) {
        bbr_state->min_rtt = UINT64_MAX;
    }
    else {
        bbr_state->min_rtt = path_x->smoothed_rtt;
    }
#ifdef RTTJitterBuffer
    BBRResetRTTJitterBuffer(bbr_state, bbr_state->min_rtt, current_time);
#endif
    bbr_state->probe_rtt_min_stamp = current_time;
    bbr_state->probe_rtt_min_delay = bbr_state->min_rtt;
    bbr_state->min_rtt_stamp = current_time;
    bbr_state->extra_acked_interval_start = current_time;
    bbr_state->extra_acked_delivered = 0;
    /* Support for the experimental options */
    BBRSetOptions(bbr_state);
    if (bbr_state->quantum_ratio == 0) {
        bbr_state->quantum_ratio = 0.001;
    }

    BBRResetCongestionSignals(bbr_state);
    BBRResetLowerBounds(bbr_state);
    BBRInitRoundCounting(bbr_state, path_x);
    BBRInitFullPipe(bbr_state);
    BBRInitPacingRate(bbr_state, path_x);
    BBREnterStartup(bbr_state, path_x);
}
```

### Rust body
```rust
    ) {
        *self = BbrState::default();
        self.option_string = option_string;

        self.init_random(connection, path_x, current_time);
        if path_x.smoothed_rtt == INITIAL_RTT && path_x.rtt_variant.ticks() == 0 {
            self.min_rtt = u64::MAX;
        } else {
            self.min_rtt = path_x.smoothed_rtt.ticks();
        }
        // RTTJitterBuffer is always enabled in this build.
        let min_rtt = self.min_rtt;
        self.reset_rtt_jitter_buffer(min_rtt, current_time);

        self.probe_rtt_min_stamp = current_time;
        self.probe_rtt_min_delay = self.min_rtt;
        self.min_rtt_stamp = current_time;
        self.extra_acked_interval_start = current_time;
        self.extra_acked_delivered = 0;

        self.set_options();
        if self.quantum_ratio == 0.0 {
            self.quantum_ratio = 0.001;
        }

        self.reset_congestion_signals();
        self.reset_lower_bounds();
        self.init_round_counting(connection, path_x);
        self.init_full_pipe();
        self.init_pacing_rate(path_x);
        self.enter_startup(path_x);
    }
```

## Pair `picoquic/bbr.c:BBRBoundCwndForModel`
C: `picoquic/bbr.c:647-671 BBRBoundCwndForModel`
Rust: `rs/fq/src/bbr.rs:1096-1103 bound_cwnd_for_model`

### C body
```c
{
    uint64_t cap = UINT64_MAX;
    if (IsInAProbeBWState(bbr_state) &&
        bbr_state->state != picoquic_bbr_alg_probe_bw_cruise) {
        if (bbr_state->inflight_hi > 0) {
            cap = bbr_state->inflight_hi;
        }
    }
    else if (bbr_state->state == picoquic_bbr_alg_probe_rtt ||
        bbr_state->state == picoquic_bbr_alg_probe_bw_cruise) {
        cap = BBRInflightWithHeadroom(bbr_state, path_x);
    }

    /* apply inflight_lo (possibly infinite): */
    if (cap > bbr_state->inflight_lo) {
        cap = bbr_state->inflight_lo;
    }
    if (cap < BBRMinPipeCwnd * path_x->send_mtu) {
        cap = BBRMinPipeCwnd * path_x->send_mtu;
    }
    if (path_x->cwin > cap) {
        path_x->cwin = cap;
    }
}
```

### Rust body
```rust
        if self.is_in_a_probe_bw_state() && self.state != BbrAlgState::ProbeBwCruise {
            if self.inflight_hi > 0 {
                cap = self.inflight_hi;
            }
        } else if self.state == BbrAlgState::ProbeRtt || self.state == BbrAlgState::ProbeBwCruise {
```

## Pair `picoquic/bbr.c:BBRSaveCwnd`
C: `picoquic/bbr.c:720-734 BBRSaveCwnd`
Rust: `rs/fq/src/bbr.rs:1284-1287 save_cwnd`

### C body
```c
{

    if ( !InLossRecovery(bbr_state) && bbr_state->state != picoquic_bbr_alg_probe_rtt) {
        return path_x->cwin;
    }
    else {
        if (bbr_state->prior_cwnd > path_x->cwin) {
            return bbr_state->prior_cwnd;
        }
        else {
            return path_x->cwin;
        }
    }
}
```

### Rust body
```rust
        if !self.in_loss_recovery() && self.state != BbrAlgState::ProbeRtt {
            path_x.cwin
        } else {
```

## Pair `picoquic/bbr.c:BBRExitLostFeedback`
C: `picoquic/bbr.c:782-788 BBRExitLostFeedback`
Rust: `rs/fq/src/bbr.rs:454-468 exit_lost_feedback`

### C body
```c
{
    if (bbr_state->is_handling_lost_feedback) {
        path_x->cwin = bbr_state->cwin_before_lost_feedback;
        bbr_state->is_handling_lost_feedback = 0;
    }
}
```

### Rust body
```rust
    pub(crate) fn has_elapsed_in_phase(&self, interval: u64, current_time: u64) -> bool {
        current_time > self.cycle_stamp + interval
    }
```

## Pair `picoquic/bbr.c:InLossRecovery`
C: `picoquic/bbr.c:847-850 InLossRecovery`
Rust: `rs/fq/src/bbr.rs:772-789 in_loss_recovery`

### C body
```c
{
    return (bbr_state->is_in_recovery);
}
```

### Rust body
```rust
    pub fn is_in_a_probe_bw_state(&self) -> bool {
        matches!(
            self.state,
            BbrAlgState::ProbeBwDown
                | BbrAlgState::ProbeBwCruise
                | BbrAlgState::ProbeBwRefill
                | BbrAlgState::ProbeBwUp
        )
    }
```

## Pair `picoquic/bbr.c:BBRUpdateOffloadBudget`
C: `picoquic/bbr.c:883-886 BBRUpdateOffloadBudget`
Rust: `rs/fq/src/bbr.rs:796-825 update_offload_budget`

### C body
```c
{
    bbr_state->offload_budget = 3 * bbr_state->send_quantum;
}
```

### Rust body
```rust
    pub fn update_rtt_jitter_buffer(&mut self, rs: &BbrPerAckState, current_time: u64) {
        if current_time > self.last_rtt_sample_stamp + 1000 {
            let idx = (self.rtt_jitter_cycle as usize) % BBR_RTT_JITTER_BUFFER_LEN;
            self.rtt_jitter_buffer[idx] = rs.rtt_sample;
            self.rtt_jitter_cycle += 1;
            self.last_rtt_sample_stamp = current_time;
            self.rtt_short_term_min = u64::MAX;
            self.rtt_short_term_max = 0;
            let valid = (self.rtt_jitter_cycle as usize).min(BBR_RTT_JITTER_BUFFER_LEN);
            for i in 0..valid {
                let sample = self.rtt_jitter_buffer[i];
                if sample > self.rtt_short_term_max {
                    self.rtt_short_term_max = sample;
                }
                if sample < self.rtt_short_term_min {
                    self.rtt_short_term_min = sample;
                }
            }
        }
    }
```

## Pair `picoquic/bbr.c:BBRUpdateMaxInflight`
C: `picoquic/bbr.c:914-930 BBRUpdateMaxInflight`
Rust: `rs/fq/src/bbr.rs:1794-1802 update_max_inflight`

### C body
```c
{
    /*  The draft mentions here a call to BBRUpdateAggregationBudget(),
    * but does not define that function. Its purpose is apparently to set
    * `extra_acked`, but that variable is computed in
    * BBRUpdateACKAggregation(), which is called as part of 
    * BBRUpdateModelAndState(). There is probably no need to do an extra
    * call here. */
    uint64_t inflight = BBRBDPMultiple(bbr_state, path_x, bbr_state->cwnd_gain);

    inflight += bbr_state->extra_acked;

    if (bbr_state->min_rtt < bbr_state->wifi_shadow_rtt && bbr_state->min_rtt > 0){
        inflight = (uint64_t)(((double)inflight) * ((double)bbr_state->wifi_shadow_rtt) / ((double)bbr_state->min_rtt));
    }
    bbr_state->max_inflight = BBRQuantizationBudget(bbr_state, path_x, inflight);
}
```

### Rust body
```rust
    fn update_max_inflight(&mut self, path_x: &Path) {
        let cwnd_gain = self.cwnd_gain;
        let mut inflight = self.bdp_multiple(path_x, cwnd_gain);
        inflight += self.extra_acked;
        if self.min_rtt < self.wifi_shadow_rtt && self.min_rtt > 0 {
            inflight = (inflight as f64 * self.wifi_shadow_rtt as f64 / self.min_rtt as f64) as u64;
        }
        self.max_inflight = self.quantization_budget(path_x, inflight);
    }
```

## Pair `picoquic/bbr.c:BBRSetSendQuantum`
C: `picoquic/bbr.c:967-982 BBRSetSendQuantum`
Rust: `rs/fq/src/bbr.rs:721-729 set_send_quantum`

### C body
```c
{
    /* 1.2 Mbps = 150 kBps = 150000Bps  */
    uint64_t floor = 2 * path_x->send_mtu;
    if (bbr_state->pacing_rate < 150000) {
        floor = 1 * path_x->send_mtu;
    }
    /* 1 ms = 1000000us/1000 */
    bbr_state->send_quantum = (uint64_t)(bbr_state->pacing_rate * bbr_state->quantum_ratio); 
    if (bbr_state->send_quantum > 0x10000) {
        bbr_state->send_quantum = 0x10000;
    }
    if (bbr_state->send_quantum < floor) {
        bbr_state->send_quantum = floor;
    }
}
```

### Rust body
```rust
    pub fn set_send_quantum(&mut self, path_x: &Path) {
        let floor = if self.pacing_rate < 150_000.0 {
            path_x.send_mtu as u64
        } else {
            2 * path_x.send_mtu as u64
        };
        let quantum = (self.pacing_rate * self.quantum_ratio) as u64;
        self.send_quantum = quantum.min(0x10000).max(floor);
    }
```

## Pair `picoquic/bbr.c:BBRInitLowerBounds`
C: `picoquic/bbr.c:1024-1033 BBRInitLowerBounds`
Rust: `rs/fq/src/bbr.rs:535-557 init_lower_bounds`

### C body
```c
{
    if (bbr_state->bw_lo == UINT64_MAX) {
        bbr_state->bw_lo = bbr_state->max_bw;
    }
    if (bbr_state->inflight_lo == UINT64_MAX) {
        bbr_state->inflight_lo = path_x->cwin;
    }
}
```

### Rust body
```rust
    pub fn init_pacing_rate(&mut self, path_x: &Path) {
        let initial_rtt = if path_x.smoothed_rtt != INITIAL_RTT || path_x.rtt_variant.ticks() != 0 {
            path_x.smoothed_rtt.ticks()
        } else {
            INITIAL_RTT.ticks()
        };
        let nominal_bandwidth = (1_000_000u64 * CWIN_INITIAL) as f64 / initial_rtt as f64;
        self.pacing_rate = BBR_STARTUP_PACING_GAIN * nominal_bandwidth;
    }
```

## Pair `picoquic/bbr.c:BBRResetLowerBounds`
C: `picoquic/bbr.c:1088-1092 BBRResetLowerBounds`
Rust: `rs/fq/src/bbr.rs:662-680 reset_lower_bounds`

### C body
```c
{
    bbr_state->bw_lo = UINT64_MAX;
    bbr_state->inflight_lo = UINT64_MAX;
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

## Pair `picoquic/bbr.c:BBRUpdateACKAggregation`
C: `picoquic/bbr.c:1129-1147 BBRUpdateACKAggregation`
Rust: `rs/fq/src/bbr.rs:1989-2015 update_ack_aggregation`

### C body
```c
{
    /* Find excess ACKed beyond expected amount over this interval */
    uint64_t interval = (current_time - bbr_state->extra_acked_interval_start);
    uint64_t expected_delivered = bbr_state->bw * interval;
    /* Reset interval if ACK rate is below expected rate: */
    if (bbr_state->extra_acked_delivered <= expected_delivered) {
        bbr_state->extra_acked_delivered = 0;
        bbr_state->extra_acked_interval_start = current_time;
        expected_delivered = 0;
    }
    bbr_state->extra_acked_delivered += rs->newly_acked;
    uint64_t extra = bbr_state->extra_acked_delivered - expected_delivered;
    if (extra > path_x->cwin) {
        extra = path_x->cwin;
    }
    bbr_state->extra_acked =
        update_windowed_max_filter(bbr_state->ExtraACKedFilter, extra, bbr_state->round_count, BBRExtraAckedFilterLen);
}
```

### Rust body
```rust
    ) {
        let interval = current_time.wrapping_sub(self.extra_acked_interval_start);
        // C: expected_delivered = bw * interval (bw bytes/s, interval µs; unit
        // mismatch is intentional — mirrors the C source verbatim).
        let mut expected_delivered = self.bw.saturating_mul(interval);
        if self.extra_acked_delivered <= expected_delivered {
            self.extra_acked_delivered = 0;
            self.extra_acked_interval_start = current_time;
            expected_delivered = 0;
        }
        self.extra_acked_delivered += rs.newly_acked;
        let extra = self
            .extra_acked_delivered
            .saturating_sub(expected_delivered)
            .min(path_x.cwin);
        self.extra_acked = update_windowed_max_filter(
            &mut self.extra_acked_filter,
            extra,
            self.round_count,
            10, // BBRExtraAckedFilterLen
        );
    }
```

## Pair `picoquic/bbr.c:BBRInitRoundCounting`
C: `picoquic/bbr.c:1203-1210 BBRInitRoundCounting`
Rust: `rs/fq/src/bbr.rs:1821-1830 init_round_counting`

### C body
```c
{
    bbr_state->next_round_delivered = 0;
    bbr_state->round_start = 0;
    bbr_state->round_count = 0;
    bbr_state->round_start_pn = picoquic_cc_get_sequence_number(path_x->cnx, path_x);
}
```

### Rust body
```rust
    pub(crate) fn init_round_counting(&mut self, connection: &Connection, path_x: &Path) {
        self.next_round_delivered = 0;
        self.round_start = false;
        self.round_count = 0;
        self.round_start_pn = if connection.is_multipath_enabled {
            path_x.pkt_ctx.send_sequence
        } else {
            connection.pkt_ctx[PacketContext::Application as usize].send_sequence
        };
    }
```

## Pair `picoquic/bbr.c:BBRUpdateRTTJitterBuffer`
C: `picoquic/bbr.c:1299-1320 BBRUpdateRTTJitterBuffer`
Rust: `rs/fq/src/bbr.rs:806-825 update_rtt_jitter_buffer`

### C body
```c
{
    if (current_time > bbr_state->last_rtt_sample_stamp + 1000) {
        bbr_state->rtt_jitter_buffer[bbr_state->rtt_jitter_cycle % BBRRTTJitterBufferLen] = rs->rtt_sample;
        bbr_state->rtt_jitter_cycle++;
        bbr_state->last_rtt_sample_stamp = current_time;
        bbr_state->rtt_short_term_min = UINT64_MAX;
        bbr_state->rtt_short_term_max = 0;
        for (unsigned int i = 0; i < BBRRTTJitterBufferLen; i++) {
            if (i >= bbr_state->rtt_jitter_cycle) {
                break;
            }
            if (bbr_state->rtt_jitter_buffer[i] > bbr_state->rtt_short_term_max) {
                bbr_state->rtt_short_term_max = bbr_state->rtt_jitter_buffer[i];
            }
            if (bbr_state->rtt_jitter_buffer[i] < bbr_state->rtt_short_term_min) {
                bbr_state->rtt_short_term_min = bbr_state->rtt_jitter_buffer[i];
            }
        }
    }
}
```

### Rust body
```rust
    pub fn update_rtt_jitter_buffer(&mut self, rs: &BbrPerAckState, current_time: u64) {
        if current_time > self.last_rtt_sample_stamp + 1000 {
            let idx = (self.rtt_jitter_cycle as usize) % BBR_RTT_JITTER_BUFFER_LEN;
            self.rtt_jitter_buffer[idx] = rs.rtt_sample;
            self.rtt_jitter_cycle += 1;
            self.last_rtt_sample_stamp = current_time;
            self.rtt_short_term_min = u64::MAX;
            self.rtt_short_term_max = 0;
            let valid = (self.rtt_jitter_cycle as usize).min(BBR_RTT_JITTER_BUFFER_LEN);
            for i in 0..valid {
                let sample = self.rtt_jitter_buffer[i];
                if sample > self.rtt_short_term_max {
                    self.rtt_short_term_max = sample;
                }
                if sample < self.rtt_short_term_min {
                    self.rtt_short_term_min = sample;
                }
            }
        }
    }
```

## Pair `picoquic/bbr.c:BBRExitProbeRTT`
C: `picoquic/bbr.c:1431-1442 BBRExitProbeRTT`
Rust: `rs/fq/src/bbr.rs:1328-1337 exit_probe_rtt`

### C body
```c
{
    BBRResetLowerBounds(bbr_state);
    path_x->rtt_min = bbr_state->min_rtt;
    if (bbr_state->filled_pipe) {
        BBREnterProbeBW(bbr_state, path_x, current_time);
        BBRStartProbeBW_CRUISE(bbr_state);
    }
    else {
        BBREnterStartup(bbr_state, path_x);
    }
}
```

### Rust body
```rust
    fn exit_probe_rtt(&mut self, connection: &Connection, path_x: &mut Path, current_time: u64) {
        self.reset_lower_bounds();
        path_x.rtt_min = crate::Duration::from_ticks(self.min_rtt);
        if self.filled_pipe {
            self.enter_probe_bw(connection, path_x, current_time);
            self.start_probe_bw_cruise();
        } else {
            self.enter_startup(path_x);
        }
    }
```

## Pair `picoquic/bbr.c:BBRCheckProbeRTT`
C: `picoquic/bbr.c:1495-1513 BBRCheckProbeRTT`
Rust: `rs/fq/src/bbr.rs:1446-1467 check_probe_rtt`

### C body
```c
{
    if (bbr_state->state != picoquic_bbr_alg_probe_rtt &&
        bbr_state->probe_rtt_expired &&
        !bbr_state->idle_restart) {
        BBREnterProbeRTT(bbr_state, path_x);
        bbr_state->min_rtt = rs->rtt_sample;
        bbr_state->prior_cwnd = BBRSaveCwnd(bbr_state, path_x);
        bbr_state->probe_rtt_done_stamp = 0;
        bbr_state->ack_phase = picoquic_bbr_acks_probe_stopping;
        BBRStartRound(bbr_state, path_x);
    }
    if (bbr_state->state == picoquic_bbr_alg_probe_rtt) {
        BBRHandleProbeRTT(bbr_state, path_x, rs, current_time);
    }
    if (rs->delivered > 0) {
        bbr_state->idle_restart = 0;
    }
}
```

### Rust body
```rust
    ) {
        if self.state != BbrAlgState::ProbeRtt && self.probe_rtt_expired && !self.idle_restart {
            self.enter_probe_rtt(path_x);
            self.min_rtt = rs.rtt_sample;
            self.prior_cwnd = self.save_cwnd(path_x);
            self.probe_rtt_done_stamp = 0;
            self.ack_phase = BbrAckPhase::ProbeStopping;
            self.start_round(connection, path_x);
        }
        if self.state == BbrAlgState::ProbeRtt {
            self.handle_probe_rtt(connection, path_x, rs, current_time);
        }
        if rs.delivered > 0 {
            self.idle_restart = false;
        }
    }
```

## Pair `picoquic/bbr.c:BBRRaiseInflightHiSlope`
C: `picoquic/bbr.c:1561-1568 BBRRaiseInflightHiSlope`
Rust: `rs/fq/src/bbr.rs:641-646 raise_inflight_hi_slope`

### C body
```c
{
    uint64_t growth_this_round = path_x->send_mtu << bbr_state->bw_probe_up_rounds;
    bbr_state->bw_probe_up_rounds = (bbr_state->bw_probe_up_rounds + 1 < 30) ? bbr_state->bw_probe_up_rounds + 1 : 30;
    uint32_t up_cnt = (uint32_t)(path_x->cwin / growth_this_round);
    bbr_state->bw_probe_up_cnt = (up_cnt > 1) ? up_cnt : 1;
}
```

### Rust body
```rust
    pub fn raise_inflight_hi_slope(&mut self, path_x: &Path) {
        let growth_this_round = (path_x.send_mtu as u64) << self.bw_probe_up_rounds;
        self.bw_probe_up_rounds = (self.bw_probe_up_rounds + 1).min(30);
        let up_cnt = (path_x.cwin / growth_this_round) as u32;
        self.bw_probe_up_cnt = up_cnt.max(1);
    }
```

## Pair `picoquic/bbr.c:BBRRandomIntBetween`
C: `picoquic/bbr.c:1639-1647 BBRRandomIntBetween`
Rust: `rs/fq/src/bbr.rs:988-998 random_int_between`

### C body
```c
{
    return (low + picoquic_test_uniform_random(&bbr_state->random_context, (high - low) + 1));
}
```

### Rust body
```rust
    fn random_int_between(&mut self, low: u64, high: u64) -> u64 {
        let range = high - low + 1;
        let rnd_min = u64::MAX % range;
        let rnd = loop {
            let v = self.bbr_random();
            if v >= rnd_min {
                break v;
            }
        };
        low + rnd % range
    }
```

## Pair `picoquic/bbr.c:BBRCheckPathSaturated`
C: `picoquic/bbr.c:1698-1720 BBRCheckPathSaturated`
Rust: `rs/fq/src/bbr.rs:1155-1177 check_path_saturated`

### C body
```c
{
    if (IsInAProbeBWState(bbr_state) &&
        rs->rtt_sample > 2*bbr_state->min_rtt &&
        bbr_state->rounds_since_bw_probe >= 1 &&
        bbr_state->pacing_rate > 3 * rs->delivery_rate &&
        bbr_state->wifi_shadow_rtt == 0) {
        bbr_state->prior_cwnd = rs->delivered;
        bbr_state->probe_rtt_done_stamp = 0;
        bbr_state->ack_phase = picoquic_bbr_acks_probe_stopping;
        bbr_state->MaxBwFilter[0] = rs->delivery_rate;
        bbr_state->MaxBwFilter[1] = rs->delivery_rate;
        bbr_state->max_bw = rs->delivery_rate;
        bbr_state->full_bw = rs->delivery_rate;
        BBREnterDrain(bbr_state, path_x);
        BBRStartRound(bbr_state, path_x);
        return 1;
    }
    else {
        return 0;
    }
}
```

### Rust body
```rust
        {
            self.prior_cwnd = rs.delivered;
            self.probe_rtt_done_stamp = 0;
            self.ack_phase = BbrAckPhase::ProbeStopping;
            self.max_bw_filter[0] = rs.delivery_rate;
            self.max_bw_filter[1] = rs.delivery_rate;
            self.max_bw = rs.delivery_rate;
            self.full_bw = rs.delivery_rate;
            self.enter_drain(path_x);
            self.start_round(connection, path_x);
            true
        } else {
```

## Pair `picoquic/bbr.c:BBRCheckTimeToProbeBW`
C: `picoquic/bbr.c:1780-1791 BBRCheckTimeToProbeBW`
Rust: `rs/fq/src/bbr.rs:1733-1747 check_time_to_probe_bw`

### C body
```c
{
    if (BBRHasElapsedInPhase(bbr_state, bbr_state->bw_probe_wait, current_time) ||
        BBRIsRenoCoexistenceProbeTime(bbr_state, path_x) ||
        (BBRExpTest(bbr_state, do_enter_probeBW_after_limited) && BBRCheckAppLimitedEnded(bbr_state, rs))) {
        BBRStartProbeBW_REFILL(bbr_state, path_x);
        return 1;
    }
    else {
        return 0;
    }
}
```

### Rust body
```rust
        {
            self.start_probe_bw_refill(connection, path_x);
            return true;
        }
```
