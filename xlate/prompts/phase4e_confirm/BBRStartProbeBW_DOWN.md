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

## `picoquic/bbr.c:BBRStartProbeBW_DOWN`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust matches most assignments but replaces BBRExpTest(bbr_state, do_rapid_start) with direct exp_flags.do_rapid_start, which may skip visible test behavior.
* Prior Phase 4D analysis: C's BBRExpTest macro is selected before the local BBRExperiment define, and no external BBRExperiment define was found, so this call expands to true in the canonical C build. Rust instead consults exp_flags.do_rapid_start, changing behavior when rapid start is disabled.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Confirmed C `BBRExpTest(..., do_rapid_start)` expands to true in this build, so the early probe wait must not depend on `exp_flags.do_rapid_start`.
* Phase 4E fix summary: Removed the `exp_flags.do_rapid_start` gate from `start_probe_bw_down`; early probe wait now depends only on `probe_probe_bw_quickly`.
* C source: `picoquic/bbr.c:1793-1814`
* C signature: `void BBRStartProbeBW_DOWN(picoquic_bbr_state_t *, picoquic_path_t *, uint64_t)`
* Current Rust source: `rs/fq/src/bbr.rs:1053-1080`
* Current Rust item: `start_probe_bw_down`

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

### Current Rust body
```rust
    ) {
        const BBR_PROBE_BW_DOWN_PACING_GAIN: f64 = 0.9;
        const BBR_PROBE_BW_DOWN_CWND_GAIN: f64 = 2.0;
        self.pacing_gain = BBR_PROBE_BW_DOWN_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_DOWN_CWND_GAIN;
        self.reset_congestion_signals();
        self.bw_probe_up_cnt = u32::MAX; // not growing inflight_hi
        // C's `BBRExpTest` macro is defined before the local `BBRExperiment`,
        // so this condition is only gated by `probe_probe_bw_quickly`.
        if self.probe_probe_bw_quickly {
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
    }
```
