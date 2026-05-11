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

## Pair `picoquic/bbr.c:BBRStartProbeBW_CRUISE`
C: `picoquic/bbr.c:1816-1821 BBRStartProbeBW_CRUISE`
Rust: `rs/fq/src/bbr.rs:735-747 start_probe_bw_cruise`

### C body
```c
{
    bbr_state->pacing_gain = BBRProbeBwCruisePacingGain;  /* pace at rate */
    bbr_state->cwnd_gain = BBRProbeBwCruiseCwndGain;   /* maintain cwnd */
    bbr_state->state = picoquic_bbr_alg_probe_bw_cruise;
}
```

### Rust body
```rust
    pub fn target_inflight(&self, path_x: &Path) -> u64 {
        self.bdp.min(path_x.cwin)
    }
```

## Pair `picoquic/bbr.c:BBREnterProbeBW`
C: `picoquic/bbr.c:1925-1929 BBREnterProbeBW`
Rust: `rs/fq/src/bbr.rs:1318-1321 enter_probe_bw`

### C body
```c
{
    bbr_state->bw_probe_ceiling = bbr_state->bw + bbr_state->bw / 2;
    BBRStartProbeBW_DOWN(bbr_state, path_x, current_time);
}
```

### Rust body
```rust
    fn enter_probe_bw(&mut self, connection: &Connection, path_x: &mut Path, current_time: u64) {
        self.bw_probe_ceiling = self.bw + self.bw / 2;
        self.start_probe_bw_down(connection, path_x, current_time);
    }
```

## Pair `picoquic/bbr.c:BBREnterStartupResume`
C: `picoquic/bbr.c:1972-1980 BBREnterStartupResume`
Rust: `rs/fq/src/bbr.rs:476-492 enter_startup_resume`

### C body
```c
{
    /* This code is called either when the "bdp seed" is set, or
     * upon "Enter Startup"
     */
    bbr_state->state = picoquic_bbr_alg_startup_resume;
    bbr_state->pacing_gain = BBRStartupResumePacingGain;
    bbr_state->cwnd_gain = BBRStartupResumeCwndGain;
}
```

### Rust body
```rust
    pub fn enter_startup(&mut self, path_x: &mut Path) {
        self.state = BbrAlgState::Startup;
        self.pacing_gain = BBR_STARTUP_PACING_GAIN;
        self.cwnd_gain = BBR_STARTUP_CWND_GAIN;
        path_x.is_cca_probing_up = true;
    }
```

## Pair `picoquic/bbr.c:BBRCheckStartupDone`
C: `picoquic/bbr.c:2042-2059 BBRCheckStartupDone`
Rust: `rs/fq/src/bbr.rs:1514-1529 check_startup_done`

### C body
```c
{
    if (bbr_state->state == picoquic_bbr_alg_startup) {
        BBRCheckStartupFullBandwidth(bbr_state, rs);
        BBRCheckStartupHighLoss(bbr_state, path_x, rs);
#ifdef RTTJitterBufferStartup
        if (bbr_state->min_rtt > PICOQUIC_MINRTT_THRESHOLD && IsRTTTooHigh(bbr_state)) {
            bbr_state->filled_pipe = 1;
        }
#endif
        if (bbr_state->filled_pipe) {
            bbr_state->probe_probe_bw_quickly = 1;
            bbr_state->full_bw_count = 0;
            BBREnterDrain(bbr_state, path_x);
        }
    }
}
```

### Rust body
```rust
    pub fn check_startup_done(&mut self, path_x: &mut Path, rs: &BbrPerAckState) {
        if self.state != BbrAlgState::Startup {
            return;
        }
        self.check_startup_full_bandwidth(rs);
        self.check_startup_high_loss(path_x, rs);
        // RTTJitterBufferStartup is defined in bbr.c:37.
        if self.min_rtt > MINRTT_THRESHOLD && self.is_rtt_too_high() {
            self.filled_pipe = true;
        }
        if self.filled_pipe {
            self.probe_probe_bw_quickly = true;
            self.full_bw_count = 0;
            self.enter_drain(path_x);
        }
    }
```

## Pair `picoquic/bbr.c:BBRExitStartupLongRtt`
C: `picoquic/bbr.c:2103-2126 BBRExitStartupLongRtt`
Rust: `rs/fq/src/bbr.rs:1617-1640 exit_startup_long_rtt`

### C body
```c
{
    /* Reset the round filter so it will start at current time */
    BBRStartRound(bbr_state, path_x);
    bbr_state->round_count++;
    bbr_state->rounds_since_probe++;
    bbr_state->round_start = 1;
    /* Set the filled pipe indicator */
    bbr_state->filled_pipe = 1;
    /* Check the RTT measurement for pathological cases */
    if ((bbr_state->rtt_filter.is_init || bbr_state->rtt_filter.sample_current > 0) &&
        bbr_state->min_rtt > 30000000 &&
        bbr_state->rtt_filter.sample_max < bbr_state->min_rtt) {
        bbr_state->min_rtt = bbr_state->rtt_filter.sample_max;
        bbr_state->min_rtt_stamp = current_time;
    }
#ifdef RTTJitterBuffer_maybe
    BBRResetRTTJitterBuffer(bbr_state, bbr_state->min_rtt, current_time);
#endif
    /* Enter drain */
    BBREnterDrain(bbr_state, path_x);
    /* If there were just few bytes in transit, enter probe */
    BBRCheckDrain(bbr_state, path_x, current_time);
}
```

### Rust body
```rust
    ) {
        self.start_round(connection, path_x);
        self.round_count += 1;
        self.rounds_since_probe += 1;
        self.round_start = true;
        self.filled_pipe = true;
        // If min_rtt is implausibly large (>30 s) and the RTT filter has a
        // smaller max, trust the filter. C: bbr.c:2113-2118.
        if (self.rtt_filter.is_init || self.rtt_filter.sample_current > 0)
            && self.min_rtt > 30_000_000
            && self.rtt_filter.sample_max.ticks() < self.min_rtt
        {
            self.min_rtt = self.rtt_filter.sample_max.ticks();
            self.min_rtt_stamp = current_time;
        }
        // RTTJitterBuffer_maybe is not defined in this build; skip its reset.
        self.enter_drain(path_x);
        self.check_drain(connection, path_x, current_time);
    }
```

## Pair `picoquic/bbr.c:BBRUpdateRecoveryOnLoss`
C: `picoquic/bbr.c:2201-2216 BBRUpdateRecoveryOnLoss`
Rust: `rs/fq/src/bbr.rs:832-844 update_recovery_on_loss`

### C body
```c
{
    if (path_x->nb_retransmit >= 1 && bbr_state->is_in_recovery && bbr_state->is_pto_recovery) {
        if (path_x->cwin > newly_lost) {
            path_x->cwin -= newly_lost;
            if (path_x->cwin < 2 * path_x->send_mtu) {
                path_x->cwin = 2 * path_x->send_mtu;
            }
        }
    }
}
```

### Rust body
```rust
    pub fn update_recovery_on_loss(&self, path_x: &mut Path, newly_lost: u64) {
        if path_x.nb_retransmit >= 1
            && self.is_in_recovery
            && self.is_pto_recovery
            && path_x.cwin > newly_lost
        {
            path_x.cwin -= newly_lost;
            let floor = 2 * path_x.send_mtu as u64;
            if path_x.cwin < floor {
                path_x.cwin = floor;
            }
        }
    }
```

## Pair `picoquic/bbr.c:BBRUpdateModelAndState`
C: `picoquic/bbr.c:2307-2326 BBRUpdateModelAndState`
Rust: `rs/fq/src/bbr.rs:2214-2235 update_model_and_state`

### C body
```c
{
    BBRUpdateLatestDeliverySignals(bbr_state, path_x, rs);
    BBRUpdateCongestionSignals(bbr_state, path_x, rs);
    BBRUpdateACKAggregation(bbr_state, path_x, rs, current_time);
    BBRCheckStartupLongRtt(bbr_state, path_x, rs, current_time);
    BBRCheckStartupResume(bbr_state, path_x, rs);
    BBRCheckStartupDone(bbr_state, path_x, rs);
    BBRCheckRecovery(bbr_state, path_x, rs, current_time);
    BBRCheckDrain(bbr_state, path_x, current_time);
    BBRUpdateProbeBWCyclePhase(bbr_state, path_x, rs, current_time);
    BBRUpdateMinRTT(bbr_state, path_x, rs, current_time);
    BBRCheckProbeRTT(bbr_state, path_x, rs, current_time);
    BBRAdvanceLatestDeliverySignals(bbr_state, rs);
    BBRAdvanceEcnFrac(bbr_state, path_x, rs);
    BBRBoundBWForModel(bbr_state);
}
```

### Rust body
```rust
    ) {
        self.update_latest_delivery_signals(path_x, rs);
        self.update_congestion_signals(connection, path_x, rs);
        self.update_ack_aggregation(path_x, rs, current_time);
        self.check_startup_long_rtt(connection, path_x, rs, current_time);
        self.check_startup_resume(path_x, rs);
        self.check_startup_done(path_x, rs);
        self.check_recovery(connection, path_x, rs, current_time);
        self.check_drain(connection, path_x, current_time);
        self.update_probe_bw_cycle_phase(connection, path_x, rs, current_time);
        self.update_min_rtt(path_x, rs, current_time);
        self.check_probe_rtt(connection, path_x, rs, current_time);
        self.advance_latest_delivery_signals(rs);
        self.advance_ecn_frac(connection, path_x, rs);
        self.bound_bw_for_model();
    }
```

## Pair `picoquic/bbr.c:picoquic_bbr_notify_ack`
C: `picoquic/bbr.c:2388-2398 picoquic_bbr_notify_ack`
Rust: `rs/fq/src/bbr.rs:2333-2343 notify_ack`

### C body
```c
{
    bbr_per_ack_state_t rs = { 0 };
    BBRSetRsFromAckState(path_x, ack_state, &rs);
    BBRComputeEcnFrac(bbr_state, path_x, &rs);
    BBRUpdateOnACK(bbr_state, path_x, &rs, current_time);
}
```

### Rust body
```rust
    ) {
        let mut rs = set_rs_from_ack_state(path_x, ack_state);
        self.compute_ecn_frac(connection, path_x, &mut rs);
        self.update_on_ack(connection, path_x, &rs, current_time);
    }
```

## Pair `picoquic/bbr1.c:BBR1EnterStartupLongRTT`
C: `picoquic/bbr1.c:309-325 BBR1EnterStartupLongRTT`
Rust: `rs/fq/src/bbr1.rs:287-301 enter_startup_long_rtt`

### C body
```c
{
    uint64_t cwnd = PICOQUIC_CWIN_INITIAL;
    bbr1_state->state = picoquic_bbr1_alg_startup_long_rtt;

    if (path_x->rtt_min > PICOQUIC_TARGET_RENO_RTT) {
        if (path_x->rtt_min > PICOQUIC_TARGET_SATELLITE_RTT) {
            cwnd = (uint64_t)((double)cwnd * (double)PICOQUIC_TARGET_SATELLITE_RTT / (double)PICOQUIC_TARGET_RENO_RTT);
        }
        else {
            cwnd = (uint64_t)((double)cwnd * (double)path_x->rtt_min / (double)PICOQUIC_TARGET_RENO_RTT);
        }
    }
    if (cwnd > path_x->cwin) {
        path_x->cwin = cwnd;
    }
}
```

### Rust body
```rust
    pub fn enter_startup_long_rtt(&mut self, path_x: &mut Path) {
        let mut cwnd = CWIN_INITIAL;
        self.state = Bbr1AlgState::StartupLongRtt;
        if path_x.rtt_min > TARGET_RENO_RTT {
            let rtt_cap = if path_x.rtt_min > TARGET_SATELLITE_RTT {
                TARGET_SATELLITE_RTT
            } else {
                path_x.rtt_min
            };
            cwnd = (cwnd as f64 * rtt_cap.ticks() as f64 / TARGET_RENO_RTT.ticks() as f64) as u64;
        }
        if cwnd > path_x.cwin {
            path_x.cwin = cwnd;
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1UpdateTargetCwnd`
C: `picoquic/bbr1.c:367-370 BBR1UpdateTargetCwnd`
Rust: `rs/fq/src/bbr1.rs:764-767 update_target_cwnd`

### C body
```c
{
    bbr1_state->target_cwnd = BBR1Inflight(bbr1_state, bbr1_state->cwnd_gain);
}
```

### Rust body
```rust
    fn update_target_cwnd(&mut self) {
        let gain = self.cwnd_gain;
        self.target_cwnd = self.inflight(gain);
    }
```

## Pair `picoquic/bbr1.c:BBR1ltbwResetInterval`
C: `picoquic/bbr1.c:494-501 BBR1ltbwResetInterval`
Rust: `rs/fq/src/bbr1.rs:385-391 ltbw_reset_interval`

### C body
```c
{
    bbr1_state->lt_last_stamp = current_time;
    bbr1_state->previous_sampling_delivered = path_x->delivered;
    bbr1_state->previous_sampling_lost = path_x->total_bytes_lost;
    bbr1_state->previous_round_lost = path_x->total_bytes_lost;
    bbr1_state->lt_rtt_cnt = 0;
}
```

### Rust body
```rust
    pub fn ltbw_reset_interval(&mut self, path_x: &Path, current_time: u64) {
        self.lt_last_stamp = current_time;
        self.previous_sampling_delivered = path_x.delivered;
        self.previous_sampling_lost = path_x.total_bytes_lost;
        self.previous_round_lost = path_x.total_bytes_lost;
        self.lt_rtt_cnt = 0;
    }
```

## Pair `picoquic/bbr1.c:BBR1UpdateBtlBw`
C: `picoquic/bbr1.c:616-677 BBR1UpdateBtlBw`
Rust: `rs/fq/src/bbr1.rs:927-980 update_btl_bw`

### C body
```c
{
    uint64_t bandwidth_estimate = path_x->bandwidth_estimate;

    if (bbr1_state->state == picoquic_bbr1_alg_startup &&
        bandwidth_estimate < (path_x->peak_bandwidth_estimate / 2)) {
        bandwidth_estimate = path_x->peak_bandwidth_estimate/2;
    }

    if (bbr1_state->rt_prop > 0) {
        /* Stop the bandwidth estimate from falling too low. */
        uint64_t min_bandwidth = PICOQUIC_RATE_FROM_BYTES(PICOQUIC_CWIN_MINIMUM, bbr1_state->rt_prop);
        if (bandwidth_estimate < min_bandwidth) {
            bandwidth_estimate = min_bandwidth;
        }
    }

    if (path_x->delivered_last_packet >= bbr1_state->next_round_delivered)
    {
        bbr1_state->next_round_delivered = path_x->delivered;
        bbr1_state->round_count++;
        bbr1_state->round_start = 1;
    }
    else {
        bbr1_state->round_start = 0;
    }

    BBR1ltbwSampling(bbr1_state, path_x, current_time);

    if (bbr1_state->round_start) {
        if (bandwidth_estimate > bbr1_state->btl_bw ||
            !path_x->last_bw_estimate_path_limited) {
            /* Forget the oldest BW round, shift by 1, compute the max BTL_BW for
            * the remaining rounds, set current round max to current value */
            bbr1_state->btl_bw = 0;
            for (int i = BBR1_BTL_BW_FILTER_LENGTH - 2; i >= 0; i--) {
                uint64_t b = bbr1_state->btl_bw_filter[i];
                bbr1_state->btl_bw_filter[i + 1] = b;
                if (b > bbr1_state->btl_bw) {
                    bbr1_state->btl_bw = b;
                }
            }
            bbr1_state->btl_bw_increased |= (bandwidth_estimate > bbr1_state->btl_bw_filter[0]);
            bbr1_state->btl_bw_filter[0] = bandwidth_estimate;
            if (bandwidth_estimate > bbr1_state->btl_bw) {
                bbr1_state->btl_bw = bandwidth_estimate;
            }
        }
        else {
            bbr1_state->btl_bw_increased = 0;
        }
    }
    else {
        if (bandwidth_estimate > bbr1_state->btl_bw_filter[0]) {
            bbr1_state->btl_bw_filter[0] =bandwidth_estimate;
            if (bandwidth_estimate > bbr1_state->btl_bw) {
                bbr1_state->btl_bw = bandwidth_estimate;
                bbr1_state->btl_bw_increased = 1;
            }
        }
    }
}
```

### Rust body
```rust
    fn update_btl_bw(&mut self, path_x: &Path, current_time: u64) {
        let mut bandwidth_estimate = path_x.bandwidth_estimate;

        if self.state == Bbr1AlgState::Startup
            && bandwidth_estimate < path_x.peak_bandwidth_estimate / 2
        {
            bandwidth_estimate = path_x.peak_bandwidth_estimate / 2;
        }

        if self.rt_prop > 0 {
            let min_bandwidth = rate_from_bytes(CWIN_MINIMUM, self.rt_prop);
            if bandwidth_estimate < min_bandwidth {
                bandwidth_estimate = min_bandwidth;
            }
        }

        if path_x.delivered_last_packet >= self.next_round_delivered {
            self.next_round_delivered = path_x.delivered;
            self.round_count += 1;
            self.round_start = true;
        } else {
            self.round_start = false;
        }

        self.ltbw_sampling(path_x, current_time);

        if self.round_start {
            if bandwidth_estimate > self.btl_bw || !path_x.last_bw_estimate_path_limited {
                // Shift filter right, dropping the oldest slot, tracking the max.
                self.btl_bw = 0;
                for i in (0..BBR1_BTL_BW_FILTER_LENGTH - 1).rev() {
                    let b = self.btl_bw_filter[i];
                    self.btl_bw_filter[i + 1] = b;
                    if b > self.btl_bw {
                        self.btl_bw = b;
                    }
                }
                // btl_bw_filter[0] still holds the previous round's value here.
                self.btl_bw_increased |= bandwidth_estimate > self.btl_bw_filter[0];
                self.btl_bw_filter[0] = bandwidth_estimate;
                if bandwidth_estimate > self.btl_bw {
                    self.btl_bw = bandwidth_estimate;
                }
            } else {
                self.btl_bw_increased = false;
            }
        } else if bandwidth_estimate > self.btl_bw_filter[0] {
            self.btl_bw_filter[0] = bandwidth_estimate;
            if bandwidth_estimate > self.btl_bw {
                self.btl_bw = bandwidth_estimate;
                self.btl_bw_increased = true;
            }
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1AdvanceCyclePhase`
C: `picoquic/bbr1.c:731-754 BBR1AdvanceCyclePhase`
Rust: `rs/fq/src/bbr1.rs:509-529 advance_cycle_phase`

### C body
```c
{
    bbr1_state->cycle_on_loss = 0;
    bbr1_state->cycle_stamp = current_time;
    bbr1_state->cycle_index++;
    if (bbr1_state->cycle_index >= BBR1_GAIN_CYCLE_LEN) {
        unsigned int start = bbr1_state->cycle_start;
        if (bbr1_state->btl_bw_increased) {
            bbr1_state->btl_bw_increased = 0;
            start++;
            if (start > BBR1_GAIN_CYCLE_MAX_START) {
                start = BBR1_GAIN_CYCLE_MAX_START;
            }
        }
        else if (start > 0) {
            start--;
        }
        bbr1_state->cycle_index = start;
        bbr1_state->cycle_start = start;
    }
   
    bbr1_state->pacing_gain = bbr1_pacing_gain_cycle[bbr1_state->cycle_index];
    BBR1SetMinimalGain(bbr1_state);
}
```

### Rust body
```rust
    pub fn advance_cycle_phase(&mut self, current_time: u64) {
        self.cycle_on_loss = false;
        self.cycle_stamp = current_time;
        self.cycle_index += 1;
        if self.cycle_index >= BBR1_GAIN_CYCLE_LEN as u32 {
            let mut start = self.cycle_start;
            if self.btl_bw_increased {
                self.btl_bw_increased = false;
                start += 1;
                if start > BBR1_GAIN_CYCLE_MAX_START {
                    start = BBR1_GAIN_CYCLE_MAX_START;
                }
            } else {
                start = start.saturating_sub(1);
            }
            self.cycle_index = start;
            self.cycle_start = start;
        }
        self.pacing_gain = BBR1_PACING_GAIN_CYCLE[self.cycle_index as usize];
        self.set_minimal_gain();
    }
```

## Pair `picoquic/bbr1.c:BBR1EnterProbeBW`
C: `picoquic/bbr1.c:787-812 BBR1EnterProbeBW`
Rust: `rs/fq/src/bbr1.rs:666-678 enter_probe_bw`

### C body
```c
{
    unsigned int start = 0;
    bbr1_state->state = picoquic_bbr1_alg_probe_bw;
    bbr1_state->pacing_gain = 1.0;
    bbr1_state->cwnd_gain = 2.0;

    if (bbr1_state->rt_prop > PICOQUIC_TARGET_RENO_RTT) {
        uint64_t ref_rt = (bbr1_state->rt_prop > PICOQUIC_TARGET_SATELLITE_RTT) ? PICOQUIC_TARGET_SATELLITE_RTT : bbr1_state->rt_prop;
        start = (unsigned int)(ref_rt / PICOQUIC_TARGET_RENO_RTT);
        if (start > BBR1_GAIN_CYCLE_MAX_START) {
            start = BBR1_GAIN_CYCLE_MAX_START;
        }
    }
    else {
        start = 2;
    }

    bbr1_state->cycle_index = start;
    bbr1_state->cycle_start = start;
    bbr1_state->btl_bw_increased = 1;

    BBR1AdvanceCyclePhase(bbr1_state, current_time);
    /* Start sampling */
    BBR1ltbwSampling(bbr1_state, path_x, current_time);
}
```

### Rust body
```rust
        let start = if self.rt_prop > TARGET_RENO_RTT.ticks() {
            let ref_rt = if self.rt_prop > TARGET_SATELLITE_RTT.ticks() {
                TARGET_SATELLITE_RTT.ticks()
            } else {
                self.rt_prop
            };
            ((ref_rt / TARGET_RENO_RTT.ticks()) as u32).min(BBR1_GAIN_CYCLE_MAX_START)
        } else {
```

## Pair `picoquic/bbr1.c:BBR1ExitStartupSeedBDP`
C: `picoquic/bbr1.c:860-883 BBR1ExitStartupSeedBDP`
Rust: `rs/fq/src/bbr1.rs:783-799 exit_startup_seed_bdp`

### C body
```c
{
    /* Set the BW to the value deduced from the BDP */
    uint64_t bandwidth_estimate = PICOQUIC_RATE_FROM_BYTES(bdp, path_x->rtt_min);
    path_x->cwin = bdp;
    /* Set the parameters */
    if (bandwidth_estimate > bbr1_state->btl_bw_filter[0]) {
        bbr1_state->btl_bw_filter[0] = bandwidth_estimate;
        if (bandwidth_estimate > bbr1_state->btl_bw) {
            bbr1_state->btl_bw = bandwidth_estimate;
            bbr1_state->btl_bw_increased = 1;
        }
    }


    BBR1UpdateRTprop(bbr1_state, path_x->rtt_min, current_time);
    /* Enter drain */
    BBR1EnterDrain(bbr1_state, path_x, current_time);
    /* If there were just few bytes in transit, enter probe */
    if (path_x->bytes_in_transit <= BBR1Inflight(bbr1_state, 1.0)) {
        BBR1EnterProbeBW(bbr1_state, path_x, current_time);
    }
}
```

### Rust body
```rust
    pub fn exit_startup_seed_bdp(&mut self, path_x: &mut Path, bdp: u64, current_time: u64) {
        let bandwidth_estimate = rate_from_bytes(bdp, path_x.rtt_min.ticks());
        path_x.cwin = bdp;
        if bandwidth_estimate > self.btl_bw_filter[0] {
            self.btl_bw_filter[0] = bandwidth_estimate;
            if bandwidth_estimate > self.btl_bw {
                self.btl_bw = bandwidth_estimate;
                self.btl_bw_increased = true;
            }
        }
        self.update_rt_prop(path_x.rtt_min.ticks(), current_time);
        self.enter_drain(path_x, current_time);
        let bytes_in_transit = path_x.bytes_in_transit;
        if bytes_in_transit <= self.inflight(1.0) {
            self.enter_probe_bw(path_x, current_time);
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1SaveCwnd`
C: `picoquic/bbr1.c:907-916 BBR1SaveCwnd`
Rust: `rs/fq/src/bbr1.rs:537-542 save_cwnd`

### C body
```c
uint64_t BBR1SaveCwnd(picoquic_bbr1_state_t* bbr1_state, picoquic_path_t* path_x) {
    uint64_t w = path_x->cwin;

    if ((InLossRecovery1(bbr1_state) || bbr1_state->state == picoquic_bbr1_alg_probe_bw) &&
        (path_x->cwin < bbr1_state->prior_cwnd)){
        w = bbr1_state->prior_cwnd;
    }
    
    return w;
}
```

### Rust body
```rust
        {
            self.prior_cwnd
        } else {
```

## Pair `picoquic/bbr1.c:BBR1UpdateModelAndState`
C: `picoquic/bbr1.c:971-980 BBR1UpdateModelAndState`
Rust: `rs/fq/src/bbr1.rs:988-1003 update_model_and_state`

### C body
```c
{
    BBR1UpdateBtlBw(bbr1_state, path_x, current_time);
    BBR1CheckCyclePhase(bbr1_state, packets_lost, current_time);
    BBR1CheckFullPipe(bbr1_state, path_x->last_bw_estimate_path_limited);
    BBR1CheckDrain(bbr1_state, path_x, bytes_in_transit, current_time);
    BBR1UpdateRTprop(bbr1_state, rtt_sample, current_time);
    BBR1CheckProbeRTT(bbr1_state, path_x, bytes_in_transit, current_time);
}
```

### Rust body
```rust
    ) {
        self.update_btl_bw(path_x, current_time);
        self.check_cycle_phase(packets_lost, current_time);
        let app_limited = path_x.last_bw_estimate_path_limited;
        self.check_full_pipe(app_limited);
        self.check_drain(path_x, bytes_in_transit, current_time);
        self.update_rt_prop(rtt_sample, current_time);
        self.check_probe_rtt(path_x, bytes_in_transit, current_time);
    }
```

## Pair `picoquic/bbr1.c:BBR1ModulateCwndForProbeRTT`
C: `picoquic/bbr1.c:1015-1023 BBR1ModulateCwndForProbeRTT`
Rust: `rs/fq/src/bbr1.rs:339-362 modulate_cwnd_for_probe_rtt`

### C body
```c
{
    if (bbr1_state->state == picoquic_bbr1_alg_probe_rtt)
    {
        if (path_x->cwin > BBR1_MIN_PIPE_CWND((uint64_t)path_x->send_mtu)) {
            path_x->cwin = BBR1_MIN_PIPE_CWND((uint64_t)path_x->send_mtu);
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

## Pair `picoquic/bbr1.c:BBR1UpdateOnACK`
C: `picoquic/bbr1.c:1074-1081 BBR1UpdateOnACK`
Rust: `rs/fq/src/bbr1.rs:1065-1082 update_on_ack`

### C body
```c
{
    BBR1UpdateModelAndState(bbr1_state, path_x, rtt_sample, bytes_in_transit,
        packets_lost, current_time);
    BBR1UpdateControlParameters(bbr1_state, path_x, bytes_in_transit, packets_lost, bytes_delivered);
}
```

### Rust body
```rust
    ) {
        self.update_model_and_state(
            path_x,
            rtt_sample,
            bytes_in_transit,
            packets_lost,
            current_time,
        );
        self.update_control_parameters(path_x, bytes_in_transit, packets_lost, bytes_delivered);
    }
```

## Pair `picoquic/bbr1.c:BBR1AfterOneRoundtripInFastRecovery`
C: `picoquic/bbr1.c:1107-1110 BBR1AfterOneRoundtripInFastRecovery`
Rust: `rs/fq/src/bbr1.rs:219-234 after_one_roundtrip_in_fast_recovery`

### C body
```c
{
    bbr1_state->packet_conservation = 0;
}
```

### Rust body
```rust
    pub fn handle_restart_from_idle(&mut self, bytes_in_transit: u64, is_app_limited: bool) {
        if bytes_in_transit == 0 && is_app_limited {
            self.idle_restart = true;
            if self.state == Bbr1AlgState::ProbeBw {
                self.set_pacing_rate_with_gain(1.0);
            }
        }
    }
```
