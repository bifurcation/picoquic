# Phase 4E repair confirmed translation mismatches

You are repairing Phase 4D `needs_fix` entries.  Phase 4D
already performed deeper classification and concluded that
these Rust translations need repair.

Rules:

* Edit Rust only.  Do not edit C sources.
* Keep edits limited to the owned Rust file(s) for this batch
  unless a directly related helper in `rs/fq/` must change.
* Preserve safe, idiomatic Rust and existing public API shape
  unless the current shape cannot express the C behavior.
* Do not replace code with stubs, placeholders, fabricated
  defaults, or weaker behavior.
* If deeper repair inspection proves Phase 4D was mistaken,
  report outcome `ok` and do not edit source.
* The driver will run a separate read-only re-triage before
  recording any `fixed` or `ok` result as resolved.
* Report `blocked` only with a concrete human-actionable
  reason.

Owned Rust file(s): `rs/fq/src/bbr.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/bbr.c:BBRCheckPathSaturated`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is visibly incomplete/truncated and omits the C condition shown before performing the state updates.
* Phase 4D analysis: The Rust method body itself faithfully implements the C predicate and state updates, but deeper caller context shows `update_probe_bw_cycle_phase` skips it even though C defines `RTTJitterBufferProbe`.
* Phase 4D fix note: Call `check_path_saturated` from the ProbeBwDown and ProbeBwCruise arms of `update_probe_bw_cycle_phase`, matching the enabled C macro paths.
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
