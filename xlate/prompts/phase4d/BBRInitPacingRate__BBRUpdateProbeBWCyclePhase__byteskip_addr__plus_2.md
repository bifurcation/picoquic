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

## `picoquic/bbr.c:BBRInitPacingRate`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust body shown is truncated and only shows initial_rtt selection, not nominal bandwidth or pacing_rate assignment.
* C source: `picoquic/bbr.c:932-942`
* C signature: `void BBRInitPacingRate(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:549-552`
* Rust item: `init_pacing_rate`

### C body
```c
{
    /* nominal_bandwidth = InitialCwnd / (SRTT ? SRTT : 1ms); */
    uint64_t initial_rtt = PICOQUIC_INITIAL_RTT; /* 1ms */
    if (path_x->smoothed_rtt != PICOQUIC_INITIAL_RTT || path_x->rtt_variant != 0) {
        initial_rtt = path_x->smoothed_rtt;
    }
    double nominal_bandwidth = ((double)(1000000ull * PICOQUIC_CWIN_INITIAL)) / (double)initial_rtt;
    bbr_state->pacing_rate = BBRStartupPacingGain * nominal_bandwidth;
}
```

### Rust body
```rust
        let initial_rtt = if path_x.smoothed_rtt != INITIAL_RTT || path_x.rtt_variant.ticks() != 0 {
            path_x.smoothed_rtt.ticks()
        } else {
```

## `picoquic/bbr.c:BBRUpdateProbeBWCyclePhase`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust omits the body-visible RTTJitterBufferProbe conditional path-saturation early returns present in C; otherwise the visible ProbeBW state logic matches.
* C source: `picoquic/bbr.c:1850-1923`
* C signature: `void BBRUpdateProbeBWCyclePhase(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:2140-2208`
* Rust item: `update_probe_bw_cycle_phase`

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

## `picoquic/bytestream.c:byteskip_addr`
* Phase 4C status: `suspect`
* Phase 4C rationale: C calls skip of 16+2 even if reading the family failed, preserving possible error-side effects; Rust returns immediately on read_varint error.
* C source: `picoquic/bytestream.c:420-431`
* C signature: `int byteskip_addr(bytestream *)`
* Rust source: `rs/fq/src/bytestream.rs:603-610`
* Rust item: `skip_addr`

### C body
```c
{
    uint64_t family = 0;
    int ret = byteread_vint(s, &family);

    if (ret == 0 && family == AF_INET) {
        ret |= bytestream_skip(s, 4 + 2);
    } else {
        ret |= bytestream_skip(s, 16 + 2);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn skip_addr(&mut self) -> Result<(), Error> {
        let family = self.read_varint()?;
        if family == WIRE_AF_INET {
            self.skip(4 + 2)
        } else {
            self.skip(16 + 2)
        }
    }
```

## `picoquic/c4.c:c4_sensitivity_1024`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust body shows only the final else expression; earlier C branches for low, high, and mid nominal_rate are not visible.
* C source: `picoquic/c4.c:244-260`
* C signature: `uint64_t c4_sensitivity_1024(c4_state_t *)`
* Rust source: `rs/fq/src/c4.rs:131-140`
* Rust item: `sensitivity_1024`

### C body
```c
{
    uint64_t sensitivity = 1024;
    if (c4_state->nominal_rate < 50000) {
        sensitivity = 0;
    }
    else if (c4_state->nominal_rate > 10000000) {
        sensitivity = 1024;
    }
    else if (c4_state->nominal_rate < 1000000) {
        sensitivity = (c4_state->nominal_rate - 50000) * 963 / 950000;
    }
    else {
        sensitivity = 963 + ((c4_state->nominal_rate - 1000000) * 61 / 9000000);
    }
    return sensitivity;
}
```

### Rust body
```rust
        } else {
            963 + ((self.nominal_rate - 1_000_000) * 61 / 9_000_000)
        }
```

## `picoquic/config.c:config_set_string_param`
* Phase 4C status: `suspect`
* Phase 4C rationale: C accepts params[x].param != NULL even when length is 0 and allocates only for length > 0; Rust rejects an empty string with !params[x].is_empty().
* C source: `picoquic/config.c:148-179`
* C signature: `int config_set_string_param(const char **, const option_param_t *, int, int)`
* Rust source: `rs/fq/src/config.rs:664-672`
* Rust item: `config_set_string_param`

### C body
```c
{
    int ret = 0;
    char* p_dup = NULL;

    if (*v != NULL) {
        free((void*)*v);
        *v = NULL;
    }

    if (params != NULL && x >= 0 && x < nb_param && params[x].param != NULL)
    {
        size_t alloc_length = params[x].length + 1;

        if (params[x].length > 0 && alloc_length > params[x].length) {
            p_dup = (char *)malloc(alloc_length);
        }
        if (p_dup != NULL) {
            memcpy(p_dup, params[x].param, params[x].length);
            p_dup[params[x].length] = 0;
            *v = (char const*)p_dup;
        }
        else {
            fprintf(stderr, "Cannot allocate %zu characters\n", params[x].length);
            ret = -1;
        }
    }
    else {
        ret = -1;
    }
    return ret;
}
```

### Rust body
```rust
fn config_set_string_param(v: &mut Option<String>, params: &[&str], x: usize) -> Result<(), Error> {
    *v = None;
    if x < params.len() && !params[x].is_empty() {
        *v = Some(params[x].to_string());
        Ok(())
    } else {
        Err(Error::InvalidArgument)
    }
}
```
