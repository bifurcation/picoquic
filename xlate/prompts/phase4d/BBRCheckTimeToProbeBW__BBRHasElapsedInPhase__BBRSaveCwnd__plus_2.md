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

## `picoquic/bbr.c:BBRCheckTimeToProbeBW`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is visibly incomplete/truncated and omits the C condition and false return path.
* C source: `picoquic/bbr.c:1780-1791`
* C signature: `int BBRCheckTimeToProbeBW(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:1733-1747`
* Rust item: `check_time_to_probe_bw`

### C body
```c
{
    if (BBRHasElapsedInPhase(bbr_state, bbr_state->bw_probe_wait, current_time) ||
        BBRIsRenoCoexistenceProbeTime(bbr_state, path_x) ||
        (BBRExpTest(bbr_state, do_enter_probeBW_after_limited) && BBRCheckAppLimitedEnded(bbr_state, rs))) {
        BBRStartProbeBW_REFILL(bbr_state, path_x);
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
            self.start_probe_bw_refill(connection, path_x);
            return true;
        }
```

## `picoquic/bbr.c:BBRHasElapsedInPhase`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns current_time > cycle_stamp + interval, while Rust body enters startup resume and mutates state and gains.
* C source: `picoquic/bbr.c:1775-1778`
* C signature: `int BBRHasElapsedInPhase(picoquic_bbr_state_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:466-480`
* Rust item: `has_elapsed_in_phase`

### C body
```c
static int BBRHasElapsedInPhase(picoquic_bbr_state_t* bbr_state, uint64_t interval, uint64_t current_time) {
    return current_time > bbr_state->cycle_stamp + interval;
}
```

### Rust body
```rust
    pub(crate) fn enter_startup_resume(&mut self) {
        self.state = BbrAlgState::StartupResume;
        self.pacing_gain = BBR_STARTUP_RESUME_PACING_GAIN;
        self.cwnd_gain = BBR_STARTUP_RESUME_CWND_GAIN;
    }
```

## `picoquic/bbr.c:BBRSaveCwnd`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is visibly incomplete/truncated after the else and omits the prior_cwnd versus cwin comparison returned by C.
* C source: `picoquic/bbr.c:720-734`
* C signature: `uint64_t BBRSaveCwnd(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:1284-1287`
* Rust item: `save_cwnd`

### C body
```c
{

    if ( !InLossRecovery(bbr_state) && bbr_state->state != picoquic_bbr_alg_probe_rtt) {
        return path_x->cwin;
    }
    else {
        if (bbr_state->prior_cwnd > path_x->cwin) {
            return bbr_state->prior_cwnd;
        }
        else {
            return path_x->cwin;
        }
    }
}
```

### Rust body
```rust
        if !self.in_loss_recovery() && self.state != BbrAlgState::ProbeRtt {
            path_x.cwin
        } else {
```

## `picoquic/bbr1.c:BBR1AfterOneRoundtripInFastRecovery`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C clears packet_conservation; Rust body handles idle restart and pacing rate and does not touch packet_conservation.
* C source: `picoquic/bbr1.c:1107-1110`
* C signature: `void BBR1AfterOneRoundtripInFastRecovery(picoquic_bbr1_state_t *)`
* Rust source: `rs/fq/src/bbr1.rs:219-234`
* Rust item: `after_one_roundtrip_in_fast_recovery`

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

## `picoquic/bbr1.c:BBR1SaveCwnd`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown is only a fragment returning prior_cwnd/else and does not show the C function's w initialization, condition, or final return structure.
* C source: `picoquic/bbr1.c:907-916`
* C signature: `uint64_t BBR1SaveCwnd(picoquic_bbr1_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr1.rs:537-542`
* Rust item: `save_cwnd`

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
