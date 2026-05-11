# Phase 4E repair confirmation

This is a read-only re-triage after a Phase 4E repair or
repair-level `ok` claim.  Do not edit files.

For each entry, inspect directly relevant C and Rust context
and decide whether the current Rust translation is now
acceptable.

Report:

* `ok` when the current Rust behavior is acceptable.
* `needs_fix` when a real mismatch remains.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short confirmation conclusion","fix_summary":"remaining mismatch if any, or empty","files_changed":[],"verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/bbr.c:BBRCheckPathSaturated`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is visibly incomplete/truncated and omits the C condition shown before performing the state updates.
* Prior Phase 4D analysis: The Rust method body itself faithfully implements the C predicate and state updates, but deeper caller context shows `update_probe_bw_cycle_phase` skips it even though C defines `RTTJitterBufferProbe`.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Confirmed Phase 4D mismatch: Rust had the saturation predicate translated but skipped the enabled RTTJitterBufferProbe caller path in ProbeBW DOWN and CRUISE.
* Phase 4E fix summary: Called check_path_saturated from ProbeBwDown after check_time_to_probe_bw and from ProbeBwCruise before check_time_to_probe_bw, returning immediately on saturation to match C.
* C source: `picoquic/bbr.c:1698-1720`
* C signature: `int BBRCheckPathSaturated(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *)`
* Current Rust source: `rs/fq/src/bbr.rs:1155-1177`
* Current Rust item: `check_path_saturated`

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

### Current Rust body
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
