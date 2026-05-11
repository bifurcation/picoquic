# Phase 4D deep translation classification

You are classifying Phase 4C non-OK C/Rust function-pair
audit entries.  Phase 4C was intentionally body-only and
shallow; Phase 4D classification is allowed to inspect
broader context.

For each entry:

1. Read the C function and any directly relevant C context:
   types, constants/macros, helper callees, and callers when
   needed to understand observable behavior.
2. Read the Rust function in context, including local types,
   helpers, tests, and nearby translated functions.
3. Decide whether the Phase 4C concern is a false positive.

Do not edit files in this classification pass.  Report:

* `ok` when the Rust behavior is acceptable after deeper
  inspection.
* `needs_fix` when the Rust translation is actually wrong and
  should be repaired in a later 4D repair pass.
* `blocked` only when the analysis cannot be completed without
  a concrete external decision or missing dependency.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short deeper-review conclusion","fix_summary":"empty unless outcome is needs_fix","files_changed":[],"verification":[]}]}
```

Entries:

## `picoquic/bbr.c:BBRComputeEcnFrac`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only retrieves pkt_ctx and returns on None; it omits delta calculations, rs field resets/updates, and alpha computation present in C.
* C source: `picoquic/bbr.c:2262-2286`
* C signature: `void BBRComputeEcnFrac(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:1756-1765`
* Rust item: `compute_ecn_frac`

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

## `picoquic/bbr.c:BBRInflight`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns BBRInflightWithBw using bw and gain, while Rust body starts ProbeBW refill and mutates pacing, cwnd gain, ack phase, state, and probing flag.
* C source: `picoquic/bbr.c:909-912`
* C signature: `uint64_t BBRInflight(picoquic_bbr_state_t *, picoquic_path_t *, double)`
* Rust source: `rs/fq/src/bbr.rs:1563-1585`
* Rust item: `inflight`

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

## `picoquic/bbr.c:BBRSetRsFromAckState`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Shown Rust body is only a partial expression for delivery-rate fallback and omits most visible C assignments to rs fields.
* C source: `picoquic/bbr.c:2346-2386`
* C signature: `void BBRSetRsFromAckState(picoquic_path_t *, picoquic_per_ack_state_t *, bbr_per_ack_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:2443-2451`
* Rust item: `set_rs_from_ack_state`

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

## `picoquic/bbr1.c:BBR1EnterProbeBW`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown is only a partial start calculation and omits visible C effects such as setting state, gains, cycle fields, btl_bw_increased, advancing cycle phase, and sampling.
* C source: `picoquic/bbr1.c:787-812`
* C signature: `void BBR1EnterProbeBW(picoquic_bbr1_state_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr1.rs:666-678`
* Rust item: `enter_probe_bw`

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

## `picoquic/bbr1.c:InLossRecovery1`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns packet_conservation, while the shown Rust body returns a tuple of state and btl_bw from an unrelated observe function.
* C source: `picoquic/bbr1.c:902-905`
* C signature: `int InLossRecovery1(picoquic_bbr1_state_t *)`
* Rust source: `rs/fq/src/bbr1.rs:411-421`
* Rust item: `in_loss_recovery`

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
