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

## Pair `picoquic/bbr.c:BBRUpdateProbeBWCyclePhase`
C: `picoquic/bbr.c:1850-1923 BBRUpdateProbeBWCyclePhase`
Rust: `rs/fq/src/bbr.rs:2140-2208 update_probe_bw_cycle_phase`

### C body
```c
{
    if (!bbr_state->filled_pipe) {
        return;  /* only handling steady-state behavior here */
    }
    BBRAdaptUpperBounds(bbr_state, path_x, rs, current_time);

    switch (bbr_state->state) {
    case picoquic_bbr_alg_probe_bw_down:
        if (BBRCheckTimeToProbeBW(bbr_state, path_x, rs, current_time))
            return; /* already decided state transition */
#ifdef RTTJitterBufferProbe
        if (BBRCheckPathSaturated(bbr_state, path_x, rs)) {
            return;
        }
#endif
        if (BBRCheckTimeToCruise(bbr_state, path_x)) {
            if (15 * bbr_state->max_bw >= 16 * bbr_state->full_bw &&
                rs->ecn_alpha <= BBRExcessiveEcnCE) {
                /* still growing? */
                bbr_state->full_bw = bbr_state->max_bw;    /* record new baseline level */
                bbr_state->full_bw_count = 0;
                bbr_state->probe_probe_bw_quickly = 1;
            }
            else {
                bbr_state->full_bw_count++;
                if (bbr_state->full_bw_count > 3 ||
                    rs->ecn_alpha > BBRExcessiveEcnCE) {
                    bbr_state->probe_probe_bw_quickly = 0;
                    bbr_state->full_bw_count = 0;
                }
            }
            BBRStartProbeBW_CRUISE(bbr_state);
        }
        break;

    case picoquic_bbr_alg_probe_bw_cruise:
#ifdef RTTJitterBufferProbe
        if (BBRCheckPathSaturated(bbr_state, path_x, rs)) {
            return;
        }
#endif
        if (BBRCheckTimeToProbeBW(bbr_state, path_x, rs, current_time))
            return; /* already decided state transition */
        break;

    case picoquic_bbr_alg_probe_bw_refill:
        /* After one round of REFILL, start UP */
        if (bbr_state->round_start) {
            bbr_state->bw_probe_samples = 1;
            BBRStartProbeBW_UP(bbr_state, path_x, current_time);
        }
        break;

    case picoquic_bbr_alg_probe_bw_up:
        if (BBRHasElapsedInPhase(bbr_state, bbr_state->min_rtt, current_time) &&
            bbr_state->min_rtt > PICOQUIC_MINRTT_THRESHOLD &&
            BBRExpTest(bbr_state, do_exit_probeBW_up_on_delay) &&
            (bbr_state->nb_rtt_excess > 0 ||
                path_x->bytes_in_transit > BBRInflightWithBw(bbr_state, path_x, 1.25, bbr_state->max_bw))) {
            BBRStartProbeBW_DOWN(bbr_state, path_x, current_time);
        }
        break;

    default:
        /* In non probe BW states, do nothing. */
        return;
    }
    /* Only in probe BW states, if BW > ceiling, enter startup */
    if (bbr_state->bw > bbr_state->bw_probe_ceiling) {
        BBRReEnterStartup(bbr_state, path_x);
    }
}
```

### Rust body
```rust
    ) {
        if !self.filled_pipe {
            return;
        }
        self.adapt_upper_bounds(connection, path_x, rs, current_time);

        match self.state {
            BbrAlgState::ProbeBwDown => {
                if self.check_time_to_probe_bw(connection, path_x, rs, current_time) {
                    return;
                }
                // RTTJitterBufferProbe not defined — skip check_path_saturated.
                if self.check_time_to_cruise(path_x) {
                    let max_bw = self.max_bw;
                    let full_bw = self.full_bw;
                    if 15 * max_bw >= 16 * full_bw && rs.ecn_alpha <= BBR_EXCESSIVE_ECN_CE {
                        // Still growing: record new baseline.
                        self.full_bw = max_bw;
                        self.full_bw_count = 0;
                        self.probe_probe_bw_quickly = true;
                    } else {
                        self.full_bw_count += 1;
                        if self.full_bw_count > 3 || rs.ecn_alpha > BBR_EXCESSIVE_ECN_CE {
                            self.probe_probe_bw_quickly = false;
                            self.full_bw_count = 0;
                        }
                    }
                    self.start_probe_bw_cruise();
                }
            }
            BbrAlgState::ProbeBwCruise => {
                // RTTJitterBufferProbe not defined — skip check_path_saturated.
                if self.check_time_to_probe_bw(connection, path_x, rs, current_time) {
                    return;
                }
            }
            BbrAlgState::ProbeBwRefill => {
                if self.round_start {
                    self.bw_probe_samples = 1;
                    self.start_probe_bw_up(connection, path_x, current_time);
                }
            }
            BbrAlgState::ProbeBwUp => {
                let min_rtt = self.min_rtt;
                let max_bw = self.max_bw;
                if self.has_elapsed_in_phase(min_rtt, current_time)
                    && min_rtt > MINRTT_THRESHOLD
                    && self.exp_flags.do_exit_probe_bw_up_on_delay
                    && (self.nb_rtt_excess > 0
                        || path_x.bytes_in_transit > self.inflight_with_bw(path_x, 1.25, max_bw))
                {
                    self.start_probe_bw_down(connection, path_x, current_time);
                }
            }
            _ => {
                return; // non-ProbeBW states: do nothing
            }
        }
        // Only in ProbeBW states: if BW exceeds ceiling, re-enter Startup.
        if self.bw > self.bw_probe_ceiling {
            self.re_enter_startup(path_x);
        }
    }
```

## Pair `picoquic/bbr.c:BBRCheckStartupFullBandwidthGeneric`
C: `picoquic/bbr.c:1951-1970 BBRCheckStartupFullBandwidthGeneric`
Rust: `rs/fq/src/bbr.rs:393-407 check_startup_full_bandwidth_generic`

### C body
```c
{
    if (bbr_state->filled_pipe ||
        !bbr_state->round_start || rs->is_app_limited) {
        return;  /* no need to check for a full pipe now */
    }

    if ((double)bbr_state->max_bw >= threshold*((double)bbr_state->full_bw)) {
        /* still growing? */
        bbr_state->full_bw = bbr_state->max_bw;    /* record new baseline level */
        bbr_state->full_bw_count = 0;
        return;
    }
    bbr_state->full_bw_count++; /* another round w/o much growth */
    if (bbr_state->full_bw_count >= 3) {
        bbr_state->filled_pipe = 1;
    }
}
```

### Rust body
```rust
    pub fn check_startup_full_bandwidth_generic(&mut self, rs: &BbrPerAckState, threshold: f64) {
        if self.filled_pipe || !self.round_start || rs.is_app_limited {
            return;
        }
        if self.max_bw as f64 >= threshold * self.full_bw as f64 {
            // still growing
            self.full_bw = self.max_bw;
            self.full_bw_count = 0;
            return;
        }
        self.full_bw_count += 1; // another round without much growth
        if self.full_bw_count >= 3 {
            self.filled_pipe = true;
        }
    }
```

## Pair `picoquic/bbr.c:BBRCheckStartupFullBandwidth`
C: `picoquic/bbr.c:2020-2040 BBRCheckStartupFullBandwidth`
Rust: `rs/fq/src/bbr.rs:410-427 check_startup_full_bandwidth`

### C body
```c
{
    if (bbr_state->filled_pipe ||
        !bbr_state->round_start || rs->is_app_limited) {
        return;  /* no need to check for a full pipe now */
    }
    /* Using here 5/4 test instead of double 1.25 */
    if (4*bbr_state->max_bw >= 5*bbr_state->full_bw) {
        /* still growing? */
        bbr_state->full_bw = bbr_state->max_bw;    /* record new baseline level */
        bbr_state->full_bw_count = 0;
        if (rs->ecn_frac < 0.2) {
            return;
        }
    }
    bbr_state->full_bw_count++; /* another round w/o much growth */
    if (bbr_state->full_bw_count >= 3 || rs->ecn_frac >= BBRExcessiveEcnCE) {
        bbr_state->filled_pipe = 1;
    }
}
```

### Rust body
```rust
    pub fn check_startup_full_bandwidth(&mut self, rs: &BbrPerAckState) {
        if self.filled_pipe || !self.round_start || rs.is_app_limited {
            return;
        }
        // 5/4 integer approximation of ×1.25 avoids floating-point drift.
        if 4 * self.max_bw >= 5 * self.full_bw {
            // still growing
            self.full_bw = self.max_bw;
            self.full_bw_count = 0;
            if rs.ecn_frac < 0.2 {
                return;
            }
        }
        self.full_bw_count += 1; // another round without much growth
        if self.full_bw_count >= 3 || rs.ecn_frac >= BBR_EXCESSIVE_ECN_CE {
            self.filled_pipe = true;
        }
    }
```

## Pair `picoquic/bbr.c:BBREnterStartupLongRTT`
C: `picoquic/bbr.c:2080-2101 BBREnterStartupLongRTT`
Rust: `rs/fq/src/bbr.rs:501-519 enter_startup_long_rtt`

### C body
```c
{
    uint64_t cwnd = PICOQUIC_CWIN_INITIAL;
    bbr_state->state = picoquic_bbr_alg_startup_long_rtt;

    if (path_x->rtt_min > PICOQUIC_TARGET_RENO_RTT) {
        if (path_x->rtt_min > PICOQUIC_TARGET_SATELLITE_RTT) {
            cwnd = (uint64_t)((double)cwnd * (double)PICOQUIC_TARGET_SATELLITE_RTT / (double)PICOQUIC_TARGET_RENO_RTT);
        }
        else {
            cwnd = (uint64_t)((double)cwnd * (double)path_x->rtt_min / (double)PICOQUIC_TARGET_RENO_RTT);
        }
    }
    if (cwnd < bbr_state->bdp_seed) {
        cwnd = bbr_state->bdp_seed;
    }
    if (cwnd > path_x->cwin) {
        path_x->cwin = cwnd;
    }
    path_x->is_cca_probing_up = 1;
}
```

### Rust body
```rust
    pub fn enter_startup_long_rtt(&mut self, path_x: &mut Path) {
        let mut cwnd = CWIN_INITIAL;
        self.state = BbrAlgState::StartupLongRtt;
        if path_x.rtt_min > TARGET_RENO_RTT {
            let rtt_cap = if path_x.rtt_min > TARGET_SATELLITE_RTT {
                TARGET_SATELLITE_RTT
            } else {
                path_x.rtt_min
            };
            cwnd = (cwnd as f64 * rtt_cap.ticks() as f64 / TARGET_RENO_RTT.ticks() as f64) as u64;
        }
        if cwnd < self.bdp_seed {
            cwnd = self.bdp_seed;
        }
        if cwnd > path_x.cwin {
            path_x.cwin = cwnd;
        }
        path_x.is_cca_probing_up = true;
    }
```

## Pair `picoquic/bbr.c:BBRSetBdpSeed`
C: `picoquic/bbr.c:2173-2180 BBRSetBdpSeed`
Rust: `rs/fq/src/bbr.rs:1875-1880 set_bdp_seed`

### C body
```c
{
    bbr_state->bdp_seed = bdp_seed;
    if (bbr_state->state == picoquic_bbr_alg_startup &&
        bbr_state->bdp_seed > bbr_state->max_bw) {
        BBREnterStartupResume(bbr_state);
    }
}
```

### Rust body
```rust
    pub fn set_bdp_seed(&mut self, bdp_seed: u64) {
        self.bdp_seed = bdp_seed;
        if self.state == BbrAlgState::Startup && self.bdp_seed > self.max_bw {
            self.enter_startup_resume();
        }
    }
```

## Pair `picoquic/bbr.c:BBRAdvanceEcnFrac`
C: `picoquic/bbr.c:2288-2305 BBRAdvanceEcnFrac`
Rust: `rs/fq/src/bbr.rs:1125-1144 advance_ecn_frac`

### C body
```c
{
    if (bbr_state->round_start) {
        picoquic_packet_context_t* pkt_ctx = BBRAccessEcnPacketContext(path_x);

        if (pkt_ctx != NULL) {
            if (pkt_ctx->ecn_ect1_total_remote < bbr_state->ecn_ect1_last_round ||
                pkt_ctx->ecn_ce_total_remote < bbr_state->ecn_ce_last_round) {
                bbr_state->ecn_alpha = 0;
            }
            else {
                bbr_state->ecn_alpha = (rs->ecn_frac + 15.0 * bbr_state->ecn_alpha) / 16;
            }
            bbr_state->ecn_ect1_last_round = pkt_ctx->ecn_ect1_total_remote;
            bbr_state->ecn_ce_last_round = pkt_ctx->ecn_ce_total_remote;
        }
    }
}
```

### Rust body
```rust
        if let Some(pkt_ctx) = access_ecn_packet_context(connection, path_x) {
            if pkt_ctx.ecn_ect1_total_remote < self.ecn_ect1_last_round
                || pkt_ctx.ecn_ce_total_remote < self.ecn_ce_last_round
            {
                self.ecn_alpha = 0.0;
            } else {
                self.ecn_alpha = (rs.ecn_frac + 15.0 * self.ecn_alpha) / 16.0;
            }
            self.ecn_ect1_last_round = pkt_ctx.ecn_ect1_total_remote;
            self.ecn_ce_last_round = pkt_ctx.ecn_ce_total_remote;
        }
```

## Pair `picoquic/bbr.c:BBRSetRsFromAckState`
C: `picoquic/bbr.c:2346-2386 BBRSetRsFromAckState`
Rust: `rs/fq/src/bbr.rs:2443-2451 set_rs_from_ack_state`

### C body
```c
{
    /* Need to compute the delivery rate */
    if (path_x->bandwidth_estimate > 0) {
        rs->delivery_rate = path_x->bandwidth_estimate;
    }
    else if (ack_state->rtt_measurement > 0) {
        rs->delivery_rate = PICOQUIC_RATE_FROM_BYTES(ack_state->nb_bytes_delivered_since_packet_sent, ack_state->rtt_measurement);
    }
    else
    {
        rs->delivery_rate = 40000;
    }
    rs->delivered = ack_state->nb_bytes_delivered_since_packet_sent;
    /* variable in path */
    rs->rtt_sample = path_x->rtt_sample;
    /* variables from call */
    rs->newly_acked = ack_state->nb_bytes_acknowledged; /* volume of data acked by current ack */
    rs->newly_lost = ack_state->nb_bytes_newly_lost; /* volume of data marked lost on ack received */
    rs->lost = ack_state->nb_bytes_lost_since_packet_sent;
    rs->tx_in_flight = ack_state->inflight_prior;
    rs->is_app_limited = ack_state->is_app_limited; /*Checked that this is properly implemented */   
    rs->is_cwnd_limited = ack_state->is_cwnd_limited;
}
```

### Rust body
```rust
    } else if ack_state.rtt_measurement.ticks() > 0 {
        rate_from_bytes(
            ack_state.nb_bytes_delivered_since_packet_sent,
            ack_state.rtt_measurement.ticks(),
        )
    } else {
```

## Pair `picoquic/bbr1.c:BBR1GetBtlBW`
C: `picoquic/bbr1.c:304-307 BBR1GetBtlBW`
Rust: `rs/fq/src/bbr1.rs:275-301 get_btl_bw`

### C body
```c
{
    return (bbr1_state->lt_use_bw) ? bbr1_state->lt_bw : bbr1_state->btl_bw;
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

## Pair `picoquic/bbr1.c:BBR1Inflight`
C: `picoquic/bbr1.c:350-364 BBR1Inflight`
Rust: `rs/fq/src/bbr1.rs:753-761 inflight`

### C body
```c
{
    uint64_t cwnd = PICOQUIC_CWIN_INITIAL;
    if (bbr1_state->rt_prop != UINT64_MAX){
        /* Bandwidth is estimated in bytes per second, rtt in microseconds*/
        uint64_t rt_target = bbr1_state->rt_prop;
        if (bbr1_state->rt_prop < bbr1_state->wifi_shadow_rtt) {
            rt_target = bbr1_state->wifi_shadow_rtt;
        }
        double estimated_bdp = (((double)BBR1GetBtlBW(bbr1_state) * (double)rt_target) / 1000000.0);
        uint64_t quanta = 3 * bbr1_state->send_quantum;       
        cwnd = (uint64_t)(gain * estimated_bdp) + quanta;
    }
    return cwnd;
}
```

### Rust body
```rust
    pub fn inflight(&self, gain: f64) -> u64 {
        let mut cwnd = CWIN_INITIAL;
        if self.rt_prop != u64::MAX {
            let rt_target = self.rt_prop.max(self.wifi_shadow_rtt);
            let estimated_bdp = self.get_btl_bw() as f64 * rt_target as f64 / 1_000_000.0;
            cwnd = (gain * estimated_bdp) as u64 + 3 * self.send_quantum;
        }
        cwnd
    }
```

## Pair `picoquic/bbr1.c:picoquic_bbr1_init`
C: `picoquic/bbr1.c:469-479 picoquic_bbr1_init`
Rust: `rs/fq/src/bbr1.rs:1345-1352 picoquic_bbr1_init`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    picoquic_bbr1_state_t* bbr1_state = (picoquic_bbr1_state_t*)malloc(sizeof(picoquic_bbr1_state_t));

    path_x->congestion_alg_state = (void*)bbr1_state;
    if (bbr1_state != NULL) {
        bbr1_state->option_string = option_string;
        picoquic_bbr1_reset(bbr1_state, path_x, current_time);
    }
}
```

### Rust body
```rust
fn picoquic_bbr1_init(path_x: &mut Path, option_string: Option<&str>, current_time: u64) {
    let mut bbr1_state = Bbr1State {
        option_string: option_string.map(str::to_owned),
        ..Bbr1State::default()
    };
    bbr1_state.reset(path_x, current_time);
    path_x.congestion_alg_state = Some(Box::new(bbr1_state));
}
```

## Pair `picoquic/bbr1.c:BBR1ltbwSampling`
C: `picoquic/bbr1.c:530-608 BBR1ltbwSampling`
Rust: `rs/fq/src/bbr1.rs:593-659 ltbw_sampling`

### C body
```c
{
    uint64_t losses = (path_x->total_bytes_lost > bbr1_state->previous_round_lost) ?
        path_x->total_bytes_lost - bbr1_state->previous_round_lost : 0;
    uint64_t delivered;
    uint64_t interval_microsec;
    uint64_t bw;

    if (bbr1_state->lt_use_bw) {
        if (bbr1_state->state == picoquic_bbr1_alg_probe_bw && bbr1_state->round_start) {
            bbr1_state->lt_rtt_cnt++;
            if (bbr1_state->lt_rtt_cnt > BBR1_LT_BW_MAX_RTTS) {
                BBR1ltbwResetSampling(bbr1_state, path_x, current_time);
                BBR1ResetProbeBwMode(bbr1_state, current_time);
                return;
            }
        }
    }
    
    if (!bbr1_state->lt_is_sampling) {
            /* Return if no loss; */
            if (losses == 0) {
                return;
            }
            /* Reset sampling otherwise. */
            BBR1ltbwResetSampling(bbr1_state, path_x, current_time);
            bbr1_state->lt_is_sampling = 1;
    }

    /* Reset sampling if app is limited */
    if (path_x->last_bw_estimate_path_limited) {
        BBR1ltbwResetSampling(bbr1_state, path_x, current_time);
        return;
    }
    /* Check whether we are reaching the end of the interval */
    if (!bbr1_state->round_start) {
        return;
    } else {
        bbr1_state->lt_rtt_cnt++;	/* count round trips in this interval */
        bbr1_state->previous_round_lost = path_x->total_bytes_lost;

        if (bbr1_state->lt_rtt_cnt < BBR1_LT_BW_INTERVAL_MIN_RTT) {
            return;		/* sampling interval needs to be longer */
        }
        if (bbr1_state->lt_rtt_cnt > BBR1_LT_BW_INTERVAL_MAX_RTT) {
            BBR1ltbwResetSampling(bbr1_state, path_x, current_time);  /* interval is too long */
            return;
        }
    }
    /* Continue sampling if no losses on this round */
    if (losses == 0) {
        return;
    }
    /* Calculate bytes lost and delivered in sampling interval.
     * Notice that the previous values of losses and delivered were for the round, not the interval.
     */
    if (path_x->delivered <= bbr1_state->previous_sampling_delivered) {
        /* No delivery at all, cannot calculate any ratio, wait some more. */
        return;
    } 
    losses = (path_x->total_bytes_lost > bbr1_state->previous_sampling_lost) ?
        path_x->total_bytes_lost - bbr1_state->previous_sampling_lost : 0;
    delivered = path_x->delivered - bbr1_state->previous_sampling_delivered;
    /* Check the loss ratio */
    if (losses * BBR1_LT_BW_RATIO_SCALE < BBR1_LT_BW_RATIO_SCALED_TARGET * delivered) {
        /* Not enough losses, continue sampling */
        return;
    }
    /* Find average delivery rate in this sampling interval. */
    interval_microsec = current_time - bbr1_state->lt_last_stamp;
    if (interval_microsec < 1000) {
        /* Interval too small for significant measurements, wait a bit */
        return;
    }
    /* Compute  bw in bytes per second */
    bw = PICOQUIC_RATE_FROM_BYTES(delivered, interval_microsec);
    /* Apply the changes */
    BBR1ltbwIntervalDone(bbr1_state, path_x, bw, current_time);
}
```

### Rust body
```rust
    pub fn ltbw_sampling(&mut self, path_x: &Path, current_time: u64) {
        let losses = path_x
            .total_bytes_lost
            .saturating_sub(self.previous_round_lost);

        if self.lt_use_bw && self.state == Bbr1AlgState::ProbeBw && self.round_start {
            self.lt_rtt_cnt += 1;
            if self.lt_rtt_cnt > BBR1_LT_BW_MAX_RTTS {
                self.ltbw_reset_sampling(path_x, current_time);
                self.reset_probe_bw_mode(current_time);
                return;
            }
        }

        if !self.lt_is_sampling {
            if losses == 0 {
                return;
            }
            self.ltbw_reset_sampling(path_x, current_time);
            self.lt_is_sampling = true;
        }

        if path_x.last_bw_estimate_path_limited {
            self.ltbw_reset_sampling(path_x, current_time);
            return;
        }

        if !self.round_start {
            return;
        }

        self.lt_rtt_cnt += 1;
        self.previous_round_lost = path_x.total_bytes_lost;

        if self.lt_rtt_cnt < BBR1_LT_BW_INTERVAL_MIN_RTT {
            return;
        }
        if self.lt_rtt_cnt > BBR1_LT_BW_INTERVAL_MAX_RTT {
            self.ltbw_reset_sampling(path_x, current_time);
            return;
        }

        if losses == 0 {
            return;
        }

        if path_x.delivered <= self.previous_sampling_delivered {
            return;
        }

        let interval_losses = path_x
            .total_bytes_lost
            .saturating_sub(self.previous_sampling_lost);
        let delivered = path_x.delivered - self.previous_sampling_delivered;

        if interval_losses * BBR1_LT_BW_RATIO_SCALE < BBR1_LT_BW_RATIO_SCALED_TARGET * delivered {
            return;
        }

        let interval_microsec = current_time - self.lt_last_stamp;
        if interval_microsec < 1000 {
            return;
        }

        let bw = rate_from_bytes(delivered, interval_microsec);
        self.ltbw_interval_done(path_x, bw, current_time);
    }
```

## Pair `picoquic/bbr1.c:BBR1SetMinimalGain`
C: `picoquic/bbr1.c:715-729 BBR1SetMinimalGain`
Rust: `rs/fq/src/bbr1.rs:397-408 set_minimal_gain`

### C body
```c
{
    if (bbr1_state->pacing_gain > 1.0 && bbr1_state->rt_prop > 0) {
        uint64_t target_cwin = PICOQUIC_BYTES_FROM_RATE(bbr1_state->rt_prop, bbr1_state->btl_bw);

        if (target_cwin < 4 * PICOQUIC_MAX_PACKET_SIZE) {
            double d_target = (double)target_cwin;
            double d_gain = ((double)(4 * PICOQUIC_MAX_PACKET_SIZE)) / d_target;

            if (d_gain > bbr1_state->pacing_gain) {
                bbr1_state->pacing_gain = d_gain;
            }
        }
    }
}
```

### Rust body
```rust
    pub fn set_minimal_gain(&mut self) {
        if self.pacing_gain > 1.0 && self.rt_prop > 0 {
            let target_cwin = bytes_from_rate(self.rt_prop, self.btl_bw);
            let min_cwin = 4 * MAX_PACKET_SIZE as u64;
            if target_cwin < min_cwin {
                let d_gain = min_cwin as f64 / target_cwin as f64;
                if d_gain > self.pacing_gain {
                    self.pacing_gain = d_gain;
                }
            }
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1CheckFullPipe`
C: `picoquic/bbr1.c:771-785 BBR1CheckFullPipe`
Rust: `rs/fq/src/bbr1.rs:191-203 check_full_pipe`

### C body
```c
{
    if (!bbr1_state->filled_pipe && bbr1_state->round_start && !rs_is_app_limited) {
        if (bbr1_state->btl_bw >= bbr1_state->full_bw * 1.25) {  // BBR1.BtlBw still growing?
            bbr1_state->full_bw = bbr1_state->btl_bw;   // record new baseline level
            bbr1_state->full_bw_count = 0;
        }
        else {
            bbr1_state->full_bw_count++; // another round w/o much growth
            if (bbr1_state->full_bw_count >= 3) {
                bbr1_state->filled_pipe = 1;
            }
        }
    }
}
```

### Rust body
```rust
    pub fn check_full_pipe(&mut self, rs_is_app_limited: bool) {
        if !self.filled_pipe && self.round_start && !rs_is_app_limited {
            if self.btl_bw as f64 >= self.full_bw as f64 * 1.25 {
                self.full_bw = self.btl_bw;
                self.full_bw_count = 0;
            } else {
                self.full_bw_count += 1;
                if self.full_bw_count >= 3 {
                    self.filled_pipe = true;
                }
            }
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1ExitStartupLongRtt`
C: `picoquic/bbr1.c:835-858 BBR1ExitStartupLongRtt`
Rust: `rs/fq/src/bbr1.rs:1027-1040 exit_startup_long_rtt`

### C body
```c
{
    /* Reset the round filter so it will start at current time */
    bbr1_state->next_round_delivered = path_x->delivered;
    bbr1_state->round_count++;
    bbr1_state->round_start = 1;
    /* Set the filled pipe indicator */
    bbr1_state->full_bw = bbr1_state->btl_bw;
    bbr1_state->full_bw_count = 3;
    bbr1_state->filled_pipe = 1;
    /* Check the RTT measurement for pathological cases */
    if ((bbr1_state->rtt_filter.is_init || bbr1_state->rtt_filter.sample_current > 0) &&
        bbr1_state->rt_prop > 30000000 &&
        bbr1_state->rtt_filter.sample_max < bbr1_state->rt_prop) {
        bbr1_state->rt_prop = bbr1_state->rtt_filter.sample_max;
        bbr1_state->rt_prop_stamp = current_time;
    }
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
        {
            self.rt_prop = self.rtt_filter.sample_max.ticks();
            self.rt_prop_stamp = current_time;
        }
```

## Pair `picoquic/bbr1.c:InLossRecovery1`
C: `picoquic/bbr1.c:902-905 InLossRecovery1`
Rust: `rs/fq/src/bbr1.rs:411-421 in_loss_recovery`

### C body
```c
{
    return bbr1_state->packet_conservation;
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        (self.state as u64, self.btl_bw)
    }
```

## Pair `picoquic/bbr1.c:BBR1CheckProbeRTT`
C: `picoquic/bbr1.c:955-969 BBR1CheckProbeRTT`
Rust: `rs/fq/src/bbr1.rs:733-745 check_probe_rtt`

### C body
```c
{
    if (bbr1_state->state != picoquic_bbr1_alg_probe_rtt &&
        bbr1_state->rt_prop_expired &&
        !bbr1_state->idle_restart) {
        BBR1EnterProbeRTT(bbr1_state);
        bbr1_state->prior_cwnd = BBR1SaveCwnd(bbr1_state, path_x);
        bbr1_state->probe_rtt_done_stamp = 0;
    }
    
    if (bbr1_state->state == picoquic_bbr1_alg_probe_rtt) {
        BBR1HandleProbeRTT(bbr1_state, path_x, bytes_in_transit, current_time);
        bbr1_state->idle_restart = 0;
    }
}
```

### Rust body
```rust
    pub fn check_probe_rtt(&mut self, path_x: &mut Path, bytes_in_transit: u64, current_time: u64) {
        if self.state != Bbr1AlgState::ProbeRtt && self.rt_prop_expired && !self.idle_restart {
            self.enter_probe_rtt();
            let prior = self.save_cwnd(path_x);
            self.prior_cwnd = prior;
            self.probe_rtt_done_stamp = 0;
        }

        if self.state == Bbr1AlgState::ProbeRtt {
            self.handle_probe_rtt(path_x, bytes_in_transit, current_time);
            self.idle_restart = false;
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1ModulateCwndForRecovery`
C: `picoquic/bbr1.c:996-1013 BBR1ModulateCwndForRecovery`
Rust: `rs/fq/src/bbr1.rs:316-333 modulate_cwnd_for_recovery`

### C body
```c
{
    if (bytes_lost > 0) {
        if (path_x->cwin > bytes_lost) {
            path_x->cwin -= bytes_lost;
        }
        else {
            path_x->cwin = path_x->send_mtu;
        }
    }
    if (bbr1_state->packet_conservation) {
        if (path_x->cwin < bytes_in_transit + bytes_delivered) {
            path_x->cwin = bytes_in_transit + bytes_delivered;
        }
    }
}
```

### Rust body
```rust
    ) {
        if bytes_lost > 0 {
            if path_x.cwin > bytes_lost {
                path_x.cwin -= bytes_lost;
            } else {
                path_x.cwin = path_x.send_mtu as u64;
            }
        }
        if self.packet_conservation && path_x.cwin < bytes_in_transit + bytes_delivered {
            path_x.cwin = bytes_in_transit + bytes_delivered;
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1HandleRestartFromIdle`
C: `picoquic/bbr1.c:1057-1066 BBR1HandleRestartFromIdle`
Rust: `rs/fq/src/bbr1.rs:227-234 handle_restart_from_idle`

### C body
```c
{
    if (bytes_in_transit == 0 && is_app_limited)
    {
        bbr1_state->idle_restart = 1;
        if (bbr1_state->state == picoquic_bbr1_alg_probe_bw) {
            BBR1SetPacingRateWithGain(bbr1_state, 1.0);
        }
    }
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

## Pair `picoquic/bbr1.c:BBR1OnEnterFastRecovery`
C: `picoquic/bbr1.c:1097-1105 BBR1OnEnterFastRecovery`
Rust: `rs/fq/src/bbr1.rs:257-269 on_enter_fast_recovery`

### C body
```c
{
    if (bytes_delivered < path_x->send_mtu) {
        bytes_delivered = path_x->send_mtu;
    }
    bbr1_state->prior_cwnd = BBR1SaveCwnd(bbr1_state, path_x);
    path_x->cwin = bytes_in_transit + bytes_delivered;
    bbr1_state->packet_conservation = 1;
}
```

### Rust body
```rust
    ) {
        if bytes_delivered < path_x.send_mtu as u64 {
            bytes_delivered = path_x.send_mtu as u64;
        }
        self.prior_cwnd = self.save_cwnd(path_x);
        path_x.cwin = bytes_in_transit + bytes_delivered;
        self.packet_conservation = true;
    }
```

## Pair `picoquic/bbr1.c:picoquic_bbr1_suspension_almost_over`
C: `picoquic/bbr1.c:1159-1172 picoquic_bbr1_suspension_almost_over`
Rust: `rs/fq/src/bbr1.rs:492-500 suspension_almost_over`

### C body
```c
{
    if (bbr1_state->is_suspended &&
        bbr1_state->cwin_before_suspension > 0 &&
        !bbr1_state->is_suspension_nearly_over &&
        bbr1_state->congestion_sequence >= lost_packet_number) {
        bbr1_state->is_suspension_nearly_over = 1;
    }
}
```

### Rust body
```rust
    pub fn suspension_almost_over(&mut self, lost_packet_number: u64) {
        if self.is_suspended
            && self.cwin_before_suspension > 0
            && !self.is_suspension_nearly_over
            && self.congestion_sequence >= lost_packet_number
        {
            self.is_suspension_nearly_over = true;
        }
    }
```
