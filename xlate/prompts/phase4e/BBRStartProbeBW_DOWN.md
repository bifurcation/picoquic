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

## `picoquic/bbr.c:BBRStartProbeBW_DOWN`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust matches most assignments but replaces BBRExpTest(bbr_state, do_rapid_start) with direct exp_flags.do_rapid_start, which may skip visible test behavior.
* Phase 4D analysis: C's BBRExpTest macro is selected before the local BBRExperiment define, and no external BBRExperiment define was found, so this call expands to true in the canonical C build. Rust instead consults exp_flags.do_rapid_start, changing behavior when rapid start is disabled.
* Phase 4D fix note: Mirror the C macro behavior for this condition, likely by not gating the early probe wait on exp_flags.do_rapid_start in this build.
* C source: `picoquic/bbr.c:1793-1814`
* C signature: `void BBRStartProbeBW_DOWN(picoquic_bbr_state_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:1052-1077`
* Rust item: `start_probe_bw_down`

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
        current_time: u64,
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
```
