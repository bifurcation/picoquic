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

## `picoquic/bbr.c:BBRUpdateProbeBWCyclePhase`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust omits the body-visible RTTJitterBufferProbe conditional path-saturation early returns present in C; otherwise the visible ProbeBW state logic matches.
* Prior Phase 4D analysis: C unconditionally defines RTTJitterBufferProbe and calls BBRCheckPathSaturated in ProbeBwDown and ProbeBwCruise; Rust has the helper but skips these active early-return paths.
* Phase 4E claimed outcome: `ok`
* Phase 4E repair analysis: Current Rust already has the active RTTJitterBufferProbe saturation early returns in the C order for ProbeBwDown and ProbeBwCruise.
* Phase 4E fix summary: 
* C source: `picoquic/bbr.c:1850-1923`
* C signature: `void BBRUpdateProbeBWCyclePhase(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *, uint64_t)`
* Current Rust source: `rs/fq/src/bbr.rs:2144-2216`
* Current Rust item: `update_probe_bw_cycle_phase`

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

### Current Rust body
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
                if self.check_path_saturated(connection, path_x, rs) {
                    return;
                }
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
                if self.check_path_saturated(connection, path_x, rs) {
                    return;
                }
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
