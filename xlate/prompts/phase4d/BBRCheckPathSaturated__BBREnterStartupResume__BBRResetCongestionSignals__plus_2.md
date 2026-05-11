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

## `picoquic/bbr.c:BBRCheckPathSaturated`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is visibly incomplete/truncated and omits the C condition shown before performing the state updates.
* C source: `picoquic/bbr.c:1698-1720`
* C signature: `int BBRCheckPathSaturated(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:1155-1177`
* Rust item: `check_path_saturated`

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

## `picoquic/bbr.c:BBREnterStartupResume`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C enters startup_resume with resume gains; Rust enters Startup with startup gains and also mutates path_x.is_cca_probing_up.
* C source: `picoquic/bbr.c:1972-1980`
* C signature: `void BBREnterStartupResume(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:476-492`
* Rust item: `enter_startup_resume`

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

## `picoquic/bbr.c:BBRResetCongestionSignals`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C resets loss/latest congestion signal fields, while Rust resets lower-bound fields bw_lo and inflight_lo to max values.
* C source: `picoquic/bbr.c:1014-1022`
* C signature: `void BBRResetCongestionSignals(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:652-665`
* Rust item: `reset_congestion_signals`

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

## `picoquic/bbr.c:InLossRecovery`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns is_in_recovery, while Rust body checks whether the state is one of several ProbeBW states.
* C source: `picoquic/bbr.c:847-850`
* C signature: `int InLossRecovery(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:772-789`
* Rust item: `in_loss_recovery`

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

## `picoquic/bbr1.c:BBR1GetBtlBW`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns lt_bw or btl_bw based on lt_use_bw, while the shown Rust body is an unrelated enter_startup_long_rtt implementation.
* C source: `picoquic/bbr1.c:304-307`
* C signature: `uint64_t BBR1GetBtlBW(picoquic_bbr1_state_t *)`
* Rust source: `rs/fq/src/bbr1.rs:275-301`
* Rust item: `get_btl_bw`

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
