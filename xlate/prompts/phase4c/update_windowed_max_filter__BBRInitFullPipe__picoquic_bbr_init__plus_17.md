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

## Pair `picoquic/bbr.c:update_windowed_max_filter`
C: `picoquic/bbr.c:381-392 update_windowed_max_filter`
Rust: `rs/fq/src/bbr.rs:2473-2490 update_windowed_max_filter`

### C body
```c
{
    if (filter[cycle % filterLen] < v) {
        filter[cycle % filterLen] = v;
    }
    for (unsigned int i = 0; i < filterLen; i++) {
        if (filter[i] > v) {
            v = filter[i];
        }
    }
    return v;
}
```

### Rust body
```rust
) -> u64 {
    let idx = (cycle as usize) % filter_len;
    if filter[idx] < v {
        filter[idx] = v;
    }
    let mut result = v;
    for &slot in &filter[..filter_len] {
        if slot > result {
            result = slot;
        }
    }
    result
}
```

## Pair `picoquic/bbr.c:BBRInitFullPipe`
C: `picoquic/bbr.c:434-439 BBRInitFullPipe`
Rust: `rs/fq/src/bbr.rs:524-542 init_full_pipe`

### C body
```c
{
    bbr_state->filled_pipe = 0;
    bbr_state->full_bw = 0;
    bbr_state->full_bw_count = 0;
}
```

### Rust body
```rust
    pub fn init_lower_bounds(&mut self, path_x: &Path) {
        if self.bw_lo == u64::MAX {
            self.bw_lo = self.max_bw;
        }
        if self.inflight_lo == u64::MAX {
            self.inflight_lo = path_x.cwin;
        }
    }
```

## Pair `picoquic/bbr.c:picoquic_bbr_init`
C: `picoquic/bbr.c:603-612 picoquic_bbr_init`
Rust: `rs/fq/src/bbr.rs:2542-2556 picoquic_bbr_init`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    picoquic_bbr_state_t* bbr_state = (picoquic_bbr_state_t*)malloc(sizeof(picoquic_bbr_state_t));

    path_x->congestion_alg_state = (void*)bbr_state;
    if (bbr_state != NULL) {
        BBROnInit(bbr_state, path_x, current_time, option_string);
    }
}
```

### Rust body
```rust
) {
    let mut bbr_state = BbrState::default();
    bbr_state.on_init(
        connection,
        path_x,
        current_time,
        option_string.map(str::to_owned),
    );
    path_x.congestion_alg_state = Some(Box::new(bbr_state));
}
```

## Pair `picoquic/bbr.c:BBRBoundCwndForProbeRTT`
C: `picoquic/bbr.c:682-690 BBRBoundCwndForProbeRTT`
Rust: `rs/fq/src/bbr.rs:1807-1814 bound_cwnd_for_probe_rtt`

### C body
```c
{
    if (bbr_state->state == picoquic_bbr_alg_probe_rtt) {
        uint64_t cap = BBRProbeRTTCwnd(bbr_state, path_x);
        if (path_x->cwin > cap) {
            path_x->cwin = cap;
        }
    }
}
```

### Rust body
```rust
    fn bound_cwnd_for_probe_rtt(&mut self, path_x: &mut Path) {
        if self.state == BbrAlgState::ProbeRtt {
            let cap = self.probe_rtt_cwnd(path_x);
            if path_x.cwin > cap {
                path_x.cwin = cap;
            }
        }
    }
```

## Pair `picoquic/bbr.c:BBROnEnterFastRecovery`
C: `picoquic/bbr.c:746-763 BBROnEnterFastRecovery`
Rust: `rs/fq/src/bbr.rs:1344-1363 on_enter_fast_recovery`

### C body
```c
{
    bbr_state->prior_cwnd = BBRSaveCwnd(bbr_state, path_x);
    uint64_t additional_cwnd = path_x->send_mtu;
    if (rs->newly_acked > additional_cwnd) {
        additional_cwnd = rs->newly_acked;
    }
    path_x->cwin = path_x->bytes_in_transit + additional_cwnd;
    bbr_state->recovery_packet_number = picoquic_cc_get_sequence_number(path_x->cnx, path_x);
    bbr_state->packet_conservation = 1;
    bbr_state->is_in_recovery = 1;
    bbr_state->is_pto_recovery = 0;
    bbr_state->recovery_delivered = path_x->delivered;
}
```

### Rust body
```rust
    ) {
        self.prior_cwnd = self.save_cwnd(path_x);
        let additional_cwnd = rs.newly_acked.max(path_x.send_mtu as u64);
        path_x.cwin = path_x.bytes_in_transit + additional_cwnd;
        // Mirror cc_get_sequence_number multipath logic (same as start_round).
        self.recovery_packet_number = if connection.is_multipath_enabled {
            path_x.pkt_ctx.send_sequence
        } else {
            connection.pkt_ctx[PacketContext::Application as usize].send_sequence
        };
        self.packet_conservation = true;
        self.is_in_recovery = true;
        self.is_pto_recovery = false;
        self.recovery_delivered = path_x.delivered;
    }
```

## Pair `picoquic/bbr.c:BBROnExitRecovery`
C: `picoquic/bbr.c:808-838 BBROnExitRecovery`
Rust: `rs/fq/src/bbr.rs:1372-1391 on_exit_recovery`

### C body
```c
{
    if (bbr_state->is_in_recovery) {
        path_x->bandwidth_estimate_max = 0;
        path_x->cwin = BBRRestoreCwnd(bbr_state, path_x);
        bbr_state->recovery_packet_number = UINT64_MAX;
        bbr_state->packet_conservation = 0;

        if (bbr_state->is_pto_recovery && BBRExpTest(bbr_state, do_handle_suspension)) {
            /* TODO:
             * we should try to enter startup with a high enough BW. However, 
             * simple attempts to restore the BW parameters have proven ineffective.
             */
            BBRReEnterStartup(bbr_state, path_x);
        }
        else if(bbr_state->state == picoquic_bbr_alg_probe_bw_up) {
            /* Perform same processing as after encountering a high loss */
            BBRStartProbeBW_DOWN(bbr_state, path_x, current_time);
        }
        bbr_state->recovery_delivered = path_x->delivered;
        bbr_state->is_in_recovery = 0;
        bbr_state->is_pto_recovery = 0;
        /* Reset the RTT time stamp, to avoid going into probe RTT during loss events */
        bbr_state->probe_rtt_min_stamp = current_time;
        bbr_state->min_rtt_stamp = current_time;
    }
}
```

### Rust body
```rust
    fn on_exit_recovery(&mut self, connection: &Connection, path_x: &mut Path, current_time: u64) {
        if !self.is_in_recovery {
            return;
        }
        path_x.bandwidth_estimate_max = 0;
        path_x.cwin = self.restore_cwnd(path_x);
        self.recovery_packet_number = u64::MAX;
        self.packet_conservation = false;
        if self.is_pto_recovery && self.exp_flags.do_handle_suspension {
            self.re_enter_startup(path_x);
        } else if self.state == BbrAlgState::ProbeBwUp {
            self.start_probe_bw_down(connection, path_x, current_time);
        }
        self.recovery_delivered = path_x.delivered;
        self.is_in_recovery = false;
        self.is_pto_recovery = false;
        // Suppress ProbeRTT entry immediately after a loss event.
        self.probe_rtt_min_stamp = current_time;
        self.min_rtt_stamp = current_time;
    }
```

## Pair `picoquic/bbr.c:BBRBDPMultipleWithBw`
C: `picoquic/bbr.c:868-876 BBRBDPMultipleWithBw`
Rust: `rs/fq/src/bbr.rs:347-354 bdp_multiple_with_bw`

### C body
```c
{
    if (bbr_state->min_rtt == UINT64_MAX) {
        return PICOQUIC_CWIN_INITIAL*path_x->send_mtu; /* no valid RTT samples yet */
    }
    bbr_state->bdp = PICOQUIC_BYTES_FROM_RATE(bbr_state->min_rtt, bw);
    return (uint64_t)(gain * (double)bbr_state->bdp);
}
```

### Rust body
```rust
    pub fn bdp_multiple_with_bw(&mut self, path_x: &Path, gain: f64, bw: u64) -> u64 {
        if self.min_rtt == u64::MAX {
            return CWIN_INITIAL * path_x.send_mtu as u64;
        }
        // PICOQUIC_BYTES_FROM_RATE(min_rtt_us, bps) = min_rtt * bps / 1_000_000
        self.bdp = self.min_rtt * bw / 1_000_000;
        (gain * self.bdp as f64) as u64
    }
```

## Pair `picoquic/bbr.c:BBRInflightWithBw`
C: `picoquic/bbr.c:903-907 BBRInflightWithBw`
Rust: `rs/fq/src/bbr.rs:1553-1556 inflight_with_bw`

### C body
```c
{
    uint64_t inflight = BBRBDPMultipleWithBw(bbr_state, path_x, gain, bw);
    return BBRQuantizationBudget(bbr_state, path_x, inflight);
}
```

### Rust body
```rust
    fn inflight_with_bw(&mut self, path_x: &Path, gain: f64, bw: u64) -> u64 {
        let inflight = self.bdp_multiple_with_bw(path_x, gain, bw);
        self.quantization_budget(path_x, inflight)
    }
```

## Pair `picoquic/bbr.c:BBRSetPacingRateWithGain`
C: `picoquic/bbr.c:944-960 BBRSetPacingRateWithGain`
Rust: `rs/fq/src/bbr.rs:698-714 set_pacing_rate_with_gain`

### C body
```c
{
    double rate = pacing_gain * ((double)(bbr_state->bw * (100 - BBRPacingMarginPercent))) / (double)100;

    if (bbr_state->state == picoquic_bbr_alg_startup_resume &&
        !bbr_state->filled_pipe &&
        bbr_state->bdp_seed > 0) {
        double bdp_rate = (((double)bbr_state->bdp_seed*1000000.0) / (double)bbr_state->min_rtt);
        if (bdp_rate > rate) {
            rate = bdp_rate;
        }
    }

    if (bbr_state->filled_pipe || rate > bbr_state->pacing_rate) {
        bbr_state->pacing_rate = rate;
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

## Pair `picoquic/bbr.c:BBRAdvanceLatestDeliverySignals`
C: `picoquic/bbr.c:1006-1012 BBRAdvanceLatestDeliverySignals`
Rust: `rs/fq/src/bbr.rs:331-354 advance_latest_delivery_signals`

### C body
```c
static void BBRAdvanceLatestDeliverySignals(picoquic_bbr_state_t* bbr_state, bbr_per_ack_state_t * rs) {
    if (bbr_state->loss_round_start) {
        bbr_state->bw_latest = rs->delivery_rate;
        bbr_state->inflight_latest = rs->delivered;
    }
}
```

### Rust body
```rust
    pub fn bdp_multiple_with_bw(&mut self, path_x: &Path, gain: f64, bw: u64) -> u64 {
        if self.min_rtt == u64::MAX {
            return CWIN_INITIAL * path_x.send_mtu as u64;
        }
        // PICOQUIC_BYTES_FROM_RATE(min_rtt_us, bps) = min_rtt * bps / 1_000_000
        self.bdp = self.min_rtt * bw / 1_000_000;
        (gain * self.bdp as f64) as u64
    }
```

## Pair `picoquic/bbr.c:BBRAdaptLowerBoundsFromCongestion`
C: `picoquic/bbr.c:1051-1051 BBRAdaptLowerBoundsFromCongestion`
Rust: `rs/fq/src/bbr.rs:960-963 adapt_lower_bounds_from_congestion`

### C body
```c
static void BBRAdaptLowerBoundsFromCongestion(picoquic_bbr_state_t* bbr_state, picoquic_path_t* path_x)
```

### Rust body
```rust
        if self.is_probing_bw() {
            return;
        }
```

## Pair `picoquic/bbr.c:BBRUpdateMaxBw`
C: `picoquic/bbr.c:1107-1116 BBRUpdateMaxBw`
Rust: `rs/fq/src/bbr.rs:1964-1980 update_max_bw`

### C body
```c
{
    BBRUpdateRound(bbr_state, path_x);

    if (rs->delivery_rate >= bbr_state->MaxBwFilter[bbr_state->cycle_count%BBRMaxBwFilterLen] || !rs->is_app_limited) {
        bbr_state->max_bw = update_windowed_max_filter(
            bbr_state->MaxBwFilter, rs->delivery_rate, bbr_state->cycle_count, BBRMaxBwFilterLen);
    }
}
```

### Rust body
```rust
    ) {
        self.update_round(connection, path_x);
        let slot = (self.cycle_count as usize) % self.max_bw_filter.len();
        if rs.delivery_rate >= self.max_bw_filter[slot] || !rs.is_app_limited {
            self.max_bw = update_windowed_max_filter(
                &mut self.max_bw_filter,
                rs.delivery_rate,
                self.cycle_count,
                2, // BBRMaxBwFilterLen
            );
        }
    }
```

## Pair `picoquic/bbr.c:BBRHandleInflightTooHigh`
C: `picoquic/bbr.c:1174-1186 BBRHandleInflightTooHigh`
Rust: `rs/fq/src/bbr.rs:1185-1200 handle_inflight_too_high`

### C body
```c
{
    /* The computation below compares the number of bytes in flight when the 
     * acked packet was sent to the current target */
    bbr_state->bw_probe_samples = 0;  /* only react once per bw probe */
    if (!rs->is_app_limited) {
        uint64_t beta_target = (uint64_t)(((double)BBRTargetInflight(bbr_state, path_x)) * BBRBeta);
        bbr_state->inflight_hi = (rs->tx_in_flight > beta_target) ? rs->tx_in_flight : beta_target;
    }
    if(bbr_state->state == picoquic_bbr_alg_probe_bw_up) {
        BBRStartProbeBW_DOWN(bbr_state, path_x, current_time);
    }
}
```

### Rust body
```rust
    ) {
        self.bw_probe_samples = 0; // only react once per bw probe
        if !rs.is_app_limited {
            let beta_target = (self.target_inflight(path_x) as f64 * BBR_BETA) as u64;
            self.inflight_hi = rs.tx_in_flight.max(beta_target);
        }
        if self.state == BbrAlgState::ProbeBwUp {
            self.start_probe_bw_down(connection, path_x, current_time);
        }
    }
```

## Pair `picoquic/bbr.c:BBRUpdateRound`
C: `picoquic/bbr.c:1219-1231 BBRUpdateRound`
Rust: `rs/fq/src/bbr.rs:1938-1957 update_round`

### C body
```c
{
    if (picoquic_cc_get_ack_number(path_x->cnx, path_x) >= bbr_state->round_start_pn) {
        BBRStartRound(bbr_state, path_x);
        bbr_state->round_count++;
        bbr_state->rounds_since_probe++;
        bbr_state->round_start = 1;
        start_windowed_max_filter_period(bbr_state->ExtraACKedFilter, bbr_state->round_count, BBRExtraAckedFilterLen);
    }
    else {
        bbr_state->round_start = 0;
    }
}
```

### Rust body
```rust
    fn update_round(&mut self, connection: &Connection, path_x: &Path) {
        let ack_number = if connection.is_multipath_enabled {
            path_x.pkt_ctx.highest_acknowledged
        } else {
            connection.pkt_ctx[PacketContext::Application as usize].highest_acknowledged
        };
        if ack_number >= self.round_start_pn {
            self.start_round(connection, path_x);
            self.round_count += 1;
            self.rounds_since_probe += 1;
            self.round_start = true;
            start_windowed_max_filter_period(
                &mut self.extra_acked_filter,
                self.round_count,
                10, // BBRExtraAckedFilterLen
            );
        } else {
            self.round_start = false;
        }
    }
```

## Pair `picoquic/bbr.c:BBRUpdateMinRTT`
C: `picoquic/bbr.c:1332-1384 BBRUpdateMinRTT`
Rust: `rs/fq/src/bbr.rs:2024-2070 update_min_rtt`

### C body
```c
{
    BBRAdaptMinRttMargin(bbr_state, path_x);
    /* maintain filter of last BBRRTTJitterBufferLen samples, to handle jitter */
    BBRUpdateRTTJitterBuffer(bbr_state, rs, current_time);
    /* Compute the BBR expired limit */
    if (bbr_state->min_rtt < UINT64_MAX) {
        if (bbr_state->min_rtt <= BBRLongRttThreshold) {
            bbr_state->probe_rtt_expired =
                current_time > bbr_state->probe_rtt_min_stamp + BBRProbeRTTInterval;
        }
        else {
            bbr_state->probe_rtt_expired =
                current_time > bbr_state->probe_rtt_min_stamp + bbr_state->min_rtt * 100;
        }
    }
    /* Update min rtt */
    if (bbr_state->rtt_short_term_max < bbr_state->probe_rtt_min_delay ||
            bbr_state->probe_rtt_expired ||
            bbr_state->rtt_jitter_cycle < BBRRTTJitterBufferLen) {
        bbr_state->probe_rtt_min_delay = bbr_state->rtt_short_term_max;
        bbr_state->probe_rtt_min_stamp = current_time;
    }
    else {
        /* Deviation from BBRv3: test whether the new measurment does not differ from min_rtt
         * by more than a "margin of error, and in that case delay the need to reevaluate min_rtt */
        if (bbr_state->rtt_short_term_min < (bbr_state->min_rtt + bbr_state->min_rtt_margin)) {
            bbr_state->probe_rtt_min_stamp = current_time;
            bbr_state->min_rtt_stamp = current_time;
        }
    }
    int min_rtt_expired =
        current_time > bbr_state->min_rtt_stamp + BBRMinRTTFilterLen;
    if (bbr_state->probe_rtt_min_delay < bbr_state->min_rtt ||
        min_rtt_expired ||
        bbr_state->rtt_jitter_cycle < BBRRTTJitterBufferLen) {
        bbr_state->min_rtt = bbr_state->probe_rtt_min_delay;
        bbr_state->min_rtt_stamp = bbr_state->probe_rtt_min_stamp;
    }

    if (bbr_state->rtt_short_term_min > bbr_state->min_rtt && bbr_state->min_rtt > PICOQUIC_MINRTT_THRESHOLD)
    {
        uint64_t delta_max = PICOQUIC_MINRTT_MARGIN + bbr_state->min_rtt / 4;
        if (bbr_state->rtt_short_term_min > bbr_state->min_rtt + delta_max) {
            bbr_state->nb_rtt_excess++;
        }
    }
    else
    {
        bbr_state->nb_rtt_excess = 0;
    }

}
```

### Rust body
```rust
    pub(crate) fn update_min_rtt(&mut self, path_x: &Path, rs: &BbrPerAckState, current_time: u64) {
        const BBR_LONG_RTT_THRESHOLD: u64 = 250_000;
        const BBR_PROBE_RTT_INTERVAL: u64 = 5_000_000;
        const BBR_MIN_RTT_FILTER_LEN: u64 = 10_000_000;

        self.adapt_min_rtt_margin(path_x);
        self.update_rtt_jitter_buffer(rs, current_time);

        if self.min_rtt < u64::MAX {
            if self.min_rtt <= BBR_LONG_RTT_THRESHOLD {
                self.probe_rtt_expired =
                    current_time > self.probe_rtt_min_stamp + BBR_PROBE_RTT_INTERVAL;
            } else {
                self.probe_rtt_expired =
                    current_time > self.probe_rtt_min_stamp + self.min_rtt * 100;
            }
        }

        if self.rtt_short_term_max < self.probe_rtt_min_delay
            || self.probe_rtt_expired
            || self.rtt_jitter_cycle < BBR_RTT_JITTER_BUFFER_LEN as u64
        {
            self.probe_rtt_min_delay = self.rtt_short_term_max;
            self.probe_rtt_min_stamp = current_time;
        } else if self.rtt_short_term_min < self.min_rtt + self.min_rtt_margin {
            self.probe_rtt_min_stamp = current_time;
            self.min_rtt_stamp = current_time;
        }

        let min_rtt_expired = current_time > self.min_rtt_stamp + BBR_MIN_RTT_FILTER_LEN;
        if self.probe_rtt_min_delay < self.min_rtt
            || min_rtt_expired
            || self.rtt_jitter_cycle < BBR_RTT_JITTER_BUFFER_LEN as u64
        {
            self.min_rtt = self.probe_rtt_min_delay;
            self.min_rtt_stamp = self.probe_rtt_min_stamp;
        }

        if self.rtt_short_term_min > self.min_rtt && self.min_rtt > MINRTT_THRESHOLD {
            let delta_max = MINRTT_MARGIN + self.min_rtt / 4;
            if self.rtt_short_term_min > self.min_rtt + delta_max {
                self.nb_rtt_excess += 1;
            }
        } else {
            self.nb_rtt_excess = 0;
        }
    }
```

## Pair `picoquic/bbr.c:BBRHandleProbeRTT`
C: `picoquic/bbr.c:1456-1485 BBRHandleProbeRTT`
Rust: `rs/fq/src/bbr.rs:1399-1411 handle_probe_rtt`

### C body
```c
{
    /* Ignore low rate samples during ProbeRTT.*/
    /* We do not implement:
    * MarkConnectionAppLimited();
    * because the app_limited status is maintained as part of app logic.
    */
    /* 
    * testing the bytes in flight when the last ACK was sent,
    * as they reflect the size of the queue encountered when
    * measuring the RTT.
    */
    if (bbr_state->probe_rtt_done_stamp == 0 &&
        rs->tx_in_flight <= BBRProbeRTTCwnd(bbr_state, path_x)) {
        /* Wait for at least ProbeRTTDuration to elapse: */
        bbr_state->probe_rtt_done_stamp =
            current_time + BBRProbeRTTDuration;
        /* Wait for at least one round to elapse: */
        bbr_state->probe_rtt_round_done = 0;
        BBRStartRound(bbr_state, path_x);
    }
    else if (bbr_state->probe_rtt_done_stamp != 0) {
        if (bbr_state->round_start) {
            bbr_state->probe_rtt_round_done = 1;
        }
        if (bbr_state->probe_rtt_round_done) {
            BBRCheckProbeRTTDone(bbr_state, path_x, current_time);
        }
    }
}
```

### Rust body
```rust
        if self.probe_rtt_done_stamp == 0 && rs.tx_in_flight <= self.probe_rtt_cwnd(path_x) {
            self.probe_rtt_done_stamp = current_time + BBR_PROBE_RTT_DURATION;
            self.probe_rtt_round_done = false;
            self.start_round(connection, path_x);
        } else if self.probe_rtt_done_stamp != 0 {
```

## Pair `picoquic/bbr.c:BBRIsProbingBW`
C: `picoquic/bbr.c:1532-1540 BBRIsProbingBW`
Rust: `rs/fq/src/bbr.rs:597-615 is_probing_bw`

### C body
```c
{
    picoquic_bbr_alg_state_t state = bbr_state->state;

    return (state == picoquic_bbr_alg_probe_bw_down ||
        state == picoquic_bbr_alg_probe_bw_cruise ||
        state == picoquic_bbr_alg_drain ||
        state == picoquic_bbr_alg_probe_rtt) ? 0 : 1;
}
```

### Rust body
```rust
    pub fn loss_lower_bounds(&mut self) {
        self.bw_lo = ((BBR_BETA * self.bw_lo as f64) as u64).max(self.bw_latest);
        self.inflight_lo = ((BBR_BETA * self.inflight_lo as f64) as u64).max(self.inflight_latest);
    }
```

## Pair `picoquic/bbr.c:BBRAdaptUpperBounds`
C: `picoquic/bbr.c:1589-1624 BBRAdaptUpperBounds`
Rust: `rs/fq/src/bbr.rs:1242-1274 adapt_upper_bounds`

### C body
```c
{
    /*
    picoquic_bbr_acks_probe_starting = 0,
    picoquic_bbr_acks_probe_stopping,
    picoquic_bbr_acks_refilling,
    */
    if (bbr_state->ack_phase == picoquic_bbr_acks_probe_starting && bbr_state->round_start) {
        /* starting to get bw probing samples */
        bbr_state->ack_phase = picoquic_bbr_acks_probe_feedback;
    }
    if (bbr_state->ack_phase == picoquic_bbr_acks_probe_stopping && bbr_state->round_start) {
        /* end of samples from bw probing phase */
        if (IsInAProbeBWState(bbr_state) && !rs->is_app_limited) {
            BBRAdvanceMaxBwFilter(bbr_state);
        }
    }
    if (!CheckInflightTooHigh(bbr_state, path_x, rs, current_time)) {
        /* Loss rate is safe. Adjust upper bounds upward. */
        if (bbr_state->inflight_hi == UINT64_MAX || bbr_state->bw_hi == UINT64_MAX) {
            return; /* no upper bounds to raise */
        }
        if (rs->tx_in_flight > bbr_state->inflight_hi) {
            /* the bytes in flight at the time the packet was sent did not create a queue. */
            bbr_state->inflight_hi = rs->tx_in_flight;
        }
        if (rs->delivery_rate > bbr_state->bw_hi) {
            bbr_state->bw_hi = rs->delivery_rate;
        }
        if (bbr_state->state == picoquic_bbr_alg_probe_bw_up) {
            BBRProbeInflightHiUpward(bbr_state, path_x, rs);
        }
    }
}
```

### Rust body
```rust
    ) {
        if self.ack_phase == BbrAckPhase::ProbeStarting && self.round_start {
            self.ack_phase = BbrAckPhase::ProbeFeedback;
        }
        if self.ack_phase == BbrAckPhase::ProbeStopping
            && self.round_start
            && self.is_in_a_probe_bw_state()
            && !rs.is_app_limited
        {
            self.advance_max_bw_filter();
        }
        if !self.check_inflight_too_high(connection, path_x, rs, current_time) {
            // Loss rate is safe.  Adjust upper bounds upward.
            if self.inflight_hi == u64::MAX || self.bw_hi == u64::MAX {
                return; // no upper bounds to raise
            }
            if rs.tx_in_flight > self.inflight_hi {
                self.inflight_hi = rs.tx_in_flight;
            }
            if rs.delivery_rate > self.bw_hi {
                self.bw_hi = rs.delivery_rate;
            }
            if self.state == BbrAlgState::ProbeBwUp {
                self.probe_inflight_hi_upward(path_x, rs);
            }
        }
    }
```

## Pair `picoquic/bbr.c:BBRPickProbeWaitEarly`
C: `picoquic/bbr.c:1675-1690 BBRPickProbeWaitEarly`
Rust: `rs/fq/src/bbr.rs:1024-1032 pick_probe_wait_early`

### C body
```c
{
    /* Decide random round-trip bound for wait: */
    bbr_state->rounds_since_bw_probe =
        (uint32_t)BBRRandomIntBetween(bbr_state, 0, 1); /* 0 or 1 */

    /* Decide the random wall clock bound for wait: */
    if (bbr_state->min_rtt < BBRLongRttThreshold) {
        bbr_state->bw_probe_wait =
            bbr_state->min_rtt + BBRRandomIntBetween(bbr_state, 0, BBRLongRttThreshold);
    }
    else {
        bbr_state->bw_probe_wait =
            bbr_state->min_rtt + BBRRandomIntBetween(bbr_state, 0, bbr_state->min_rtt);
    }
}
```

### Rust body
```rust
    fn pick_probe_wait_early(&mut self) {
        const BBR_LONG_RTT_THRESHOLD: u64 = 250_000;
        self.rounds_since_bw_probe = self.random_int_between(0, 1) as u32;
        self.bw_probe_wait = if self.min_rtt < BBR_LONG_RTT_THRESHOLD {
            self.min_rtt + self.random_int_between(0, BBR_LONG_RTT_THRESHOLD)
        } else {
            self.min_rtt + self.random_int_between(0, self.min_rtt)
        };
    }
```

## Pair `picoquic/bbr.c:BBRIsRenoCoexistenceProbeTime`
C: `picoquic/bbr.c:1768-1773 BBRIsRenoCoexistenceProbeTime`
Rust: `rs/fq/src/bbr.rs:1592-1596 is_reno_coexistence_probe_time`

### C body
```c
{
    uint64_t reno_rounds = (BBRTargetInflight(bbr_state, path_x)/path_x->send_mtu);
    uint64_t rounds = (reno_rounds < 63) ? reno_rounds : 63;
    return (bbr_state->rounds_since_bw_probe >= rounds);
}
```

### Rust body
```rust
    fn is_reno_coexistence_probe_time(&self, path_x: &Path) -> bool {
        let reno_rounds = self.target_inflight(path_x) / path_x.send_mtu as u64;
        let rounds = reno_rounds.min(63);
        self.rounds_since_bw_probe as u64 >= rounds
    }
```
