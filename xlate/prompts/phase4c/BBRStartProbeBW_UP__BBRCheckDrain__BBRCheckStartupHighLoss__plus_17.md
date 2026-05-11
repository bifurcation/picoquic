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

## Pair `picoquic/bbr.c:BBRStartProbeBW_UP`
C: `picoquic/bbr.c:1837-1848 BBRStartProbeBW_UP`
Rust: `rs/fq/src/bbr.rs:2077-2094 start_probe_bw_up`

### C body
```c
{
    bbr_state->nb_rtt_excess = 0;
    bbr_state->pacing_gain = BBRProbeBwUpPacingGain;  /* pace at rate */
    bbr_state->cwnd_gain = BBRProbeBwUpCwndGain;   /* maintain cwnd */
    bbr_state->ack_phase = picoquic_bbr_acks_probe_starting;
    BBRStartRound(bbr_state, path_x);
    bbr_state->cycle_stamp = current_time; /* start wall clock */
    bbr_state->state = picoquic_bbr_alg_probe_bw_up;
    BBRRaiseInflightHiSlope(bbr_state, path_x);
    path_x->is_cca_probing_up = 1;
}
```

### Rust body
```rust
    ) {
        const BBR_PROBE_BW_UP_PACING_GAIN: f64 = 1.25; // C: BBRProbeBwUpPacingGain
        const BBR_PROBE_BW_UP_CWND_GAIN: f64 = 2.25; // C: BBRProbeBwUpCwndGain
        self.nb_rtt_excess = 0;
        self.pacing_gain = BBR_PROBE_BW_UP_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_UP_CWND_GAIN;
        self.ack_phase = BbrAckPhase::ProbeStarting;
        self.start_round(connection, path_x);
        self.cycle_stamp = current_time;
        self.state = BbrAlgState::ProbeBwUp;
        self.raise_inflight_hi_slope(path_x);
        path_x.is_cca_probing_up = true;
    }
```

## Pair `picoquic/bbr.c:BBRCheckDrain`
C: `picoquic/bbr.c:1943-1948 BBRCheckDrain`
Rust: `rs/fq/src/bbr.rs:1602-1610 check_drain`

### C body
```c
{
    if (bbr_state->state == picoquic_bbr_alg_drain && path_x->bytes_in_transit <= BBRInflight(bbr_state, path_x, 1.0)) {
        BBREnterProbeBW(bbr_state, path_x, current_time);  /* we estimate that the queue is drained */
    }
}
```

### Rust body
```rust
    fn check_drain(&mut self, connection: &Connection, path_x: &mut Path, current_time: u64) {
        if self.state == BbrAlgState::Drain {
            let bw = self.bw;
            let target = self.inflight_with_bw(path_x, 1.0, bw);
            if path_x.bytes_in_transit <= target {
                self.enter_probe_bw(connection, path_x, current_time);
            }
        }
    }
```

## Pair `picoquic/bbr.c:BBRCheckStartupHighLoss`
C: `picoquic/bbr.c:2002-2018 BBRCheckStartupHighLoss`
Rust: `rs/fq/src/bbr.rs:1501-1504 check_startup_high_loss`

### C body
```c
{
    /* TODO: no sample code provided */
    /*
    * A second method BBR uses for estimating the bottleneck is full is by looking at sustained packet losses.
    Specifically for a case where the following criteria are all met:
    The connection has been in fast recovery for at least one full round trip.
    The loss rate over the time scale of a single full round trip exceeds BBRLossThresh (2%).
    There are at least BBRStartupFullLossCnt=3 discontiguous sequence ranges lost in that round trip.

    If these criteria are all met, then BBRCheckStartupHighLoss() sets BBR.filled_pipe = true, which will cause exit Startup and enters Drain
    */
    if (IsInflightTooHigh(bbr_state, path_x, rs)) {
        bbr_state->filled_pipe = 1;
    }
}
```

### Rust body
```rust
        if self.is_inflight_too_high(path_x, rs) {
            self.filled_pipe = true;
        }
```

## Pair `picoquic/bbr.c:BBRReEnterStartup`
C: `picoquic/bbr.c:2069-2076 BBRReEnterStartup`
Rust: `rs/fq/src/bbr.rs:1306-1312 re_enter_startup`

### C body
```c
{
    bbr_state->full_bw = 0;
    bbr_state->filled_pipe = 0;
    bbr_state->full_bw_count = 0;
    bbr_state->probe_probe_bw_quickly = 1;
    BBREnterStartup(bbr_state, path_x);
}
```

### Rust body
```rust
    fn re_enter_startup(&mut self, path_x: &mut Path) {
        self.full_bw = 0;
        self.filled_pipe = false;
        self.full_bw_count = 0;
        self.probe_probe_bw_quickly = true;
        self.enter_startup(path_x);
    }
```

## Pair `picoquic/bbr.c:BBRUpdateStartupLongRtt`
C: `picoquic/bbr.c:2155-2171 BBRUpdateStartupLongRtt`
Rust: `rs/fq/src/bbr.rs:2262-2277 update_startup_long_rtt`

### C body
```c
{
    if (path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
        path_x->cwin += picoquic_cc_slow_start_increase(path_x, rs->newly_acked);
    }

    uint64_t max_win = PICOQUIC_BYTES_FROM_RATE(bbr_state->min_rtt, path_x->peak_bandwidth_estimate);
    if (max_win < bbr_state->bdp_seed) {
        max_win = bbr_state->bdp_seed;
    }

    uint64_t min_win = max_win /= 2;

    if (path_x->cwin < min_win) {
        path_x->cwin = min_win;
    }
}
```

### Rust body
```rust
    pub fn update_startup_long_rtt(&mut self, path_x: &mut Path, rs: &BbrPerAckState) {
        if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
            path_x.cwin += path_x.slow_start_increase(rs.newly_acked);
        }
        // PICOQUIC_BYTES_FROM_RATE(rtt_us, bps) = rtt * bps / 1_000_000
        let mut max_win = self.min_rtt * path_x.peak_bandwidth_estimate / 1_000_000;
        if max_win < self.bdp_seed {
            max_win = self.bdp_seed;
        }
        // C: `uint64_t min_win = max_win /= 2;` — both halves max_win and
        // assigns the halved value to min_win.
        let min_win = max_win / 2;
        if path_x.cwin < min_win {
            path_x.cwin = min_win;
        }
    }
```

## Pair `picoquic/bbr.c:BBRComputeEcnFrac`
C: `picoquic/bbr.c:2262-2286 BBRComputeEcnFrac`
Rust: `rs/fq/src/bbr.rs:1756-1765 compute_ecn_frac`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = BBRAccessEcnPacketContext(path_x);
    uint64_t delta_ect1 = 0;
    uint64_t delta_ce = 0;
    rs->ecn_frac = 0.0;

    if (pkt_ctx != NULL &&
        pkt_ctx->ecn_ect1_total_remote >= bbr_state->ecn_ect1_last_round &&
        pkt_ctx->ecn_ce_total_remote >= bbr_state->ecn_ce_last_round) {
        if (pkt_ctx->ecn_ect1_total_remote == 0) {
            /* Probably legacy ECN -- treat it the same way we would treat proportional ECN */
            delta_ect1 = (rs->delivered/path_x->send_mtu);
        }
        else {
            delta_ect1 = pkt_ctx->ecn_ect1_total_remote - bbr_state->ecn_ect1_last_round;
            delta_ce = pkt_ctx->ecn_ce_total_remote - bbr_state->ecn_ce_last_round;
        }
        if (delta_ect1 + delta_ce > 0) {
            rs->ecn_ce = delta_ce;
            rs->ecn_frac = (double)delta_ce / (double)(delta_ect1 + delta_ce);
            rs->ecn_alpha = (rs->ecn_frac + 15.0 * bbr_state->ecn_alpha) / 16.0;
        }
    }
}
```

### Rust body
```rust
        let Some(pkt_ctx) = access_ecn_packet_context(connection, path_x) else {
            return;
        };
```

## Pair `picoquic/bbr.c:BBRUpdateOnACK`
C: `picoquic/bbr.c:2335-2344 BBRUpdateOnACK`
Rust: `rs/fq/src/bbr.rs:2242-2255 update_on_ack`

### C body
```c
{
    BBRUpdateModelAndState(bbr_state, path_x, rs, current_time);
    if (bbr_state->state == picoquic_bbr_alg_startup_long_rtt) {
        BBRUpdateStartupLongRtt(bbr_state, path_x, rs);
    }
    else {
        BBRUpdateControlParameters(bbr_state, path_x, rs);
    }
}
```

### Rust body
```rust
    ) {
        self.update_model_and_state(connection, path_x, rs, current_time);
        if self.state == BbrAlgState::StartupLongRtt {
            self.update_startup_long_rtt(path_x, rs);
        } else {
            self.update_control_parameters(path_x, rs);
        }
    }
```

## Pair `picoquic/bbr.c:picoquic_bbr_observe`
C: `picoquic/bbr.c:2468-2473 picoquic_bbr_observe`
Rust: `rs/fq/src/bbr.rs:951-953 observe`

### C body
```c
{
    picoquic_bbr_state_t* bbr_state = (picoquic_bbr_state_t*)path_x->congestion_alg_state;
    *cc_state = (uint64_t)bbr_state->state;
    *cc_param = bbr_state->bw;
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        (self.state as u64, self.bw)
    }
```

## Pair `picoquic/bbr1.c:BBR1SetSendQuantum`
C: `picoquic/bbr1.c:334-348 BBR1SetSendQuantum`
Rust: `rs/fq/src/bbr1.rs:351-362 set_send_quantum`

### C body
```c
{
    if (bbr1_state->pacing_rate < BBR1_PACING_RATE_LOW) {
        bbr1_state->send_quantum = 1ull * path_x->send_mtu;
    } 
    else if (bbr1_state->pacing_rate < BBR1_PACING_RATE_MEDIUM) {
        bbr1_state->send_quantum = 2ull * path_x->send_mtu;
    }
    else {
        bbr1_state->send_quantum = (uint64_t)(bbr1_state->pacing_rate * bbr1_state->quantum_ratio);
        if (bbr1_state->send_quantum > 0x10000) {
            bbr1_state->send_quantum = 0x10000;
        }
    }
}
```

### Rust body
```rust
    pub fn set_send_quantum(&mut self, path_x: &Path) {
        if self.pacing_rate < BBR1_PACING_RATE_LOW {
            self.send_quantum = path_x.send_mtu as u64;
        } else if self.pacing_rate < BBR1_PACING_RATE_MEDIUM {
            self.send_quantum = 2 * path_x.send_mtu as u64;
        } else {
            self.send_quantum = (self.pacing_rate * self.quantum_ratio) as u64;
            if self.send_quantum > 0x10000 {
                self.send_quantum = 0x10000;
            }
        }
    }
```

## Pair `picoquic/bbr1.c:picoquic_bbr1_reset`
C: `picoquic/bbr1.c:449-467 picoquic_bbr1_reset`
Rust: `rs/fq/src/bbr1.rs:1089-1139 reset`

### C body
```c
{
    memset(bbr1_state, 0, sizeof(picoquic_bbr1_state_t));
    path_x->cwin = PICOQUIC_CWIN_INITIAL;
    bbr1_state->rt_prop = UINT64_MAX; 
    picoquic_bbr1_set_options(bbr1_state);
    if (bbr1_state->quantum_ratio == 0) {
        bbr1_state->quantum_ratio = 0.001;
    }

    bbr1_state->rt_prop_stamp = current_time;
    bbr1_state->cycle_stamp = current_time;
    bbr1_state->cycle_index = 0;
    bbr1_state->cycle_start = 0;

    BBR1EnterStartup(bbr1_state);
    BBR1SetSendQuantum(bbr1_state, path_x);
    BBR1UpdateTargetCwnd(bbr1_state);
}
```

### Rust body
```rust
        *self = Self {
            state: Bbr1AlgState::Startup,
            btl_bw: 0,
            next_round_delivered: 0,
            btl_bw_filter: [0; BBR1_BTL_BW_FILTER_LENGTH],
            full_bw: 0,
            rt_prop: 0,
            rt_prop_stamp: 0,
            cycle_stamp: 0,
            probe_rtt_done_stamp: 0,
            prior_cwnd: 0,
            prior_in_flight: 0,
            bytes_delivered: 0,
            send_quantum: 0,
            rtt_filter: MinMaxRtt::default(),
            target_cwnd: 0,
            pacing_gain: 0.0,
            cwnd_gain: 0.0,
            pacing_rate: 0.0,
            cycle_index: 0,
            cycle_start: 0,
            round_count: 0,
            full_bw_count: 0,
            lt_rtt_cnt: 0,
            lt_bw: 0,
            lt_last_stamp: 0,
            previous_round_lost: 0,
            previous_sampling_delivered: 0,
            previous_sampling_lost: 0,
            loss_interval_start: 0,
            congestion_sequence: 0,
            cwin_before_suspension: 0,
            option_string: None,
            wifi_shadow_rtt: 0,
            quantum_ratio: 0.0,
            filled_pipe: false,
            round_start: false,
            rt_prop_expired: false,
            probe_rtt_round_done: false,
            idle_restart: false,
            packet_conservation: false,
            btl_bw_increased: false,
            lt_use_bw: false,
            lt_is_sampling: false,
            last_loss_was_timeout: false,
            cycle_on_loss: false,
            is_suspended: false,
            is_suspension_nearly_over: false,
        };
```

## Pair `picoquic/bbr1.c:BBR1ltbwIntervalDone`
C: `picoquic/bbr1.c:511-528 BBR1ltbwIntervalDone`
Rust: `rs/fq/src/bbr1.rs:563-577 ltbw_interval_done`

### C body
```c
{
    if (bbr1_state->lt_bw) {
        /* This is not the first limited interval. Look whether it is close enough */
        uint64_t diff = (bw > bbr1_state->lt_bw) ? bw - bbr1_state->lt_bw : bbr1_state->lt_bw - bw;
        if (diff * BBR1_LT_BW_RATIO_INVERSE < bbr1_state->lt_bw ||
            diff < BBR1_LT_BW_BYTES_PER_SEC_DIFF) {
            bbr1_state->lt_bw = (bbr1_state->lt_bw + bw) / 2;
            bbr1_state->lt_use_bw = 1;
            bbr1_state->pacing_gain = 1.0;
            bbr1_state->lt_rtt_cnt = 0;
            return;
        }
    }
    /* If first interval or non-matching rate, just remember */
    bbr1_state->lt_bw = bw;
    BBR1ltbwResetInterval(bbr1_state, path_x, current_time);
}
```

### Rust body
```rust
    fn ltbw_interval_done(&mut self, path_x: &Path, bw: u64, current_time: u64) {
        if self.lt_bw > 0 {
            let diff = bw.abs_diff(self.lt_bw);
            if diff * BBR1_LT_BW_RATIO_INVERSE < self.lt_bw || diff < BBR1_LT_BW_BYTES_PER_SEC_DIFF
            {
                self.lt_bw = (self.lt_bw + bw) / 2;
                self.lt_use_bw = true;
                self.pacing_gain = 1.0;
                self.lt_rtt_cnt = 0;
                return;
            }
        }
        self.lt_bw = bw;
        self.ltbw_reset_interval(path_x, current_time);
    }
```

## Pair `picoquic/bbr1.c:BBR1IsNextCyclePhase`
C: `picoquic/bbr1.c:698-713 BBR1IsNextCyclePhase`
Rust: `rs/fq/src/bbr1.rs:866-883 is_next_cycle_phase`

### C body
```c
{
    int is_full_length = bbr1_state->cycle_on_loss || (current_time - bbr1_state->cycle_stamp) > bbr1_state->rt_prop;
    
    if (bbr1_state->pacing_gain != 1.0) {
        if (bbr1_state->pacing_gain > 1.0) {
            is_full_length &=
                (packets_lost > 0 ||
                    prior_in_flight >= BBR1Inflight(bbr1_state, bbr1_state->pacing_gain));
        }
        else {  /*  (BBR1.pacing_gain < 1) */
            is_full_length &= prior_in_flight <= BBR1Inflight(bbr1_state, 1.0);
        }
    }
    return is_full_length;
}
```

### Rust body
```rust
    ) -> bool {
        let is_full_length = self.cycle_on_loss || (current_time - self.cycle_stamp) > self.rt_prop;
        if self.pacing_gain != 1.0 {
            if self.pacing_gain > 1.0 {
                is_full_length
                    && (packets_lost > 0 || prior_in_flight >= self.inflight(self.pacing_gain))
            } else {
                is_full_length && prior_in_flight <= self.inflight(1.0)
            }
        } else {
            is_full_length
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1ResetProbeBwMode`
C: `picoquic/bbr1.c:764-769 BBR1ResetProbeBwMode`
Rust: `rs/fq/src/bbr1.rs:582-586 reset_probe_bw_mode`

### C body
```c
{
    bbr1_state->state = picoquic_bbr1_alg_probe_bw;
    bbr1_state->cycle_index = 2;
    BBR1AdvanceCyclePhase(bbr1_state, current_time);
}
```

### Rust body
```rust
    fn reset_probe_bw_mode(&mut self, current_time: u64) {
        self.state = Bbr1AlgState::ProbeBw;
        self.cycle_index = 2;
        self.advance_cycle_phase(current_time);
    }
```

## Pair `picoquic/bbr1.c:BBR1CheckDrain`
C: `picoquic/bbr1.c:824-833 BBR1CheckDrain`
Rust: `rs/fq/src/bbr1.rs:903-913 check_drain`

### C body
```c
{
    if (bbr1_state->state == picoquic_bbr1_alg_startup && bbr1_state->filled_pipe) {
        BBR1EnterDrain(bbr1_state, path_x, current_time);
    }

    if (bbr1_state->state == picoquic_bbr1_alg_drain && bytes_in_transit <= BBR1Inflight(bbr1_state, 1.0)) {
        BBR1EnterProbeBW(bbr1_state, path_x, current_time);  /* we estimate queue is drained */
    }
}
```

### Rust body
```rust
    fn check_drain(&mut self, path_x: &mut Path, bytes_in_transit: u64, current_time: u64) {
        if self.state == Bbr1AlgState::Startup && self.filled_pipe {
            self.enter_drain(path_x, current_time);
        }
        if self.state == Bbr1AlgState::Drain {
            let target = self.inflight(1.0);
            if bytes_in_transit <= target {
                self.enter_probe_bw(path_x, current_time);
            }
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1ExitProbeRTT`
C: `picoquic/bbr1.c:892-900 BBR1ExitProbeRTT`
Rust: `rs/fq/src/bbr1.rs:693-699 exit_probe_rtt`

### C body
```c
{
    if (bbr1_state->filled_pipe) {
        BBR1EnterProbeBW(bbr1_state, path_x, current_time);
    }
    else {
        BBR1EnterStartup(bbr1_state);
    }
}
```

### Rust body
```rust
    pub fn exit_probe_rtt(&mut self, path_x: &Path, current_time: u64) {
        if self.filled_pipe {
            self.enter_probe_bw(path_x, current_time);
        } else {
            self.enter_startup();
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1HandleProbeRTT`
C: `picoquic/bbr1.c:926-953 BBR1HandleProbeRTT`
Rust: `rs/fq/src/bbr1.rs:706-727 handle_probe_rtt`

### C body
```c
{
#if 0
    /* Ignore low rate samples during ProbeRTT: */
    C.app_limited =
        (BW.delivered + bytes_in_transit) ? 0 : 1;
#endif

    if (bbr1_state->probe_rtt_done_stamp == 0 &&
        bytes_in_transit <= BBR1_MIN_PIPE_CWND((uint64_t)path_x->send_mtu)) {
        bbr1_state->probe_rtt_done_stamp =
            current_time + BBR1_PROBE_RTT_DURATION;
        bbr1_state->probe_rtt_round_done = 0;
        bbr1_state->next_round_delivered = path_x->delivered;
    }
    else if (bbr1_state->probe_rtt_done_stamp != 0) {
        if (bbr1_state->round_start) {
            bbr1_state->probe_rtt_round_done = 1;
        }
        
        if (bbr1_state->probe_rtt_round_done &&
            current_time > bbr1_state->probe_rtt_done_stamp) {
            bbr1_state->rt_prop_stamp = current_time;
            BBR1RestoreCwnd(bbr1_state, path_x);
            BBR1ExitProbeRTT(bbr1_state, path_x, current_time);
        }
    }
}
```

### Rust body
```rust
    ) {
        let min_pipe_cwnd = path_x.send_mtu as u64 * 4;
        if self.probe_rtt_done_stamp == 0 && bytes_in_transit <= min_pipe_cwnd {
            self.probe_rtt_done_stamp = current_time + BBR1_PROBE_RTT_DURATION;
            self.probe_rtt_round_done = false;
            self.next_round_delivered = path_x.delivered;
        } else if self.probe_rtt_done_stamp != 0 {
            if self.round_start {
                self.probe_rtt_round_done = true;
            }
            if self.probe_rtt_round_done && current_time > self.probe_rtt_done_stamp {
                self.rt_prop_stamp = current_time;
                self.restore_cwnd(path_x);
                self.exit_probe_rtt(path_x, current_time);
            }
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1SetPacingRate`
C: `picoquic/bbr1.c:991-994 BBR1SetPacingRate`
Rust: `rs/fq/src/bbr1.rs:854-857 set_pacing_rate`

### C body
```c
{
    BBR1SetPacingRateWithGain(bbr1_state, bbr1_state->pacing_gain);
}
```

### Rust body
```rust
    fn set_pacing_rate(&mut self) {
        let gain = self.pacing_gain;
        self.set_pacing_rate_with_gain(gain);
    }
```

## Pair `picoquic/bbr1.c:BBR1UpdateControlParameters`
C: `picoquic/bbr1.c:1050-1055 BBR1UpdateControlParameters`
Rust: `rs/fq/src/bbr1.rs:1009-1019 update_control_parameters`

### C body
```c
{
    BBR1SetPacingRate(bbr1_state);
    BBR1SetSendQuantum(bbr1_state, path_x);
    BBR1SetCwnd(bbr1_state, path_x, bytes_in_transit, packets_lost, bytes_delivered);
}
```

### Rust body
```rust
    ) {
        self.set_pacing_rate();
        self.set_send_quantum(path_x);
        self.set_cwnd(path_x, bytes_in_transit, packets_lost, bytes_delivered);
    }
```

## Pair `picoquic/bbr1.c:BBR1OnAllPacketsLost`
C: `picoquic/bbr1.c:1091-1095 BBR1OnAllPacketsLost`
Rust: `rs/fq/src/bbr1.rs:248-251 on_all_packets_lost`

### C body
```c
{
    bbr1_state->prior_cwnd = BBR1SaveCwnd(bbr1_state, path_x);
    path_x->cwin = path_x->send_mtu;
}
```

### Rust body
```rust
    pub fn on_all_packets_lost(&mut self, path_x: &mut Path) {
        self.prior_cwnd = self.save_cwnd(path_x);
        path_x.cwin = path_x.send_mtu as u64;
    }
```

## Pair `picoquic/bbr1.c:picoquic_bbr1_notify_congestion`
C: `picoquic/bbr1.c:1118-1157 picoquic_bbr1_notify_congestion`
Rust: `rs/fq/src/bbr1.rs:1161-1174 notify_congestion`

### C body
```c
{
    /* Apply filter of last loss */
    if ((bbr1_state->cycle_on_loss || current_time < bbr1_state->loss_interval_start + path_x->smoothed_rtt) &&
        (!is_timeout || bbr1_state->last_loss_was_timeout)) {
        /* filter repeated loss events */
        return;
    }
    if (is_timeout || path_x->cwin < PICOQUIC_CWIN_MINIMUM) {
        if (!bbr1_state->is_suspended) {
            bbr1_state->is_suspended = 1;
            bbr1_state->cwin_before_suspension = path_x->cwin;
        }
        path_x->cwin = PICOQUIC_CWIN_MINIMUM;
    } else {
        path_x->cwin = path_x->cwin / 2;
    }
    bbr1_state->loss_interval_start = current_time;
    bbr1_state->last_loss_was_timeout = is_timeout;
    bbr1_state->congestion_sequence = picoquic_cc_get_sequence_number(cnx, path_x);

    /* Update and check the packet loss rate */
    if (bbr1_state->state == picoquic_bbr1_alg_startup_long_rtt) {
        BBR1ExitStartupLongRtt(bbr1_state, path_x, current_time);
    }
    else if (bbr1_state->state == picoquic_bbr1_alg_startup) {
        bbr1_state->filled_pipe = 1;
        BBR1EnterDrain(bbr1_state, path_x, current_time);
    }
    else {
        bbr1_state->cycle_on_loss = 1;
    }
}
```

### Rust body
```rust
        {
            return;
        }
```
