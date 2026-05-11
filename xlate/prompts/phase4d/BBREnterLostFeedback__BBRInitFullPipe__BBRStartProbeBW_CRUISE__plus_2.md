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

## `picoquic/bbr.c:BBREnterLostFeedback`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is has_elapsed_in_phase and only compares current_time with cycle_stamp plus interval; it does not enter lost feedback or modify cwin/state fields.
* C source: `picoquic/bbr.c:765-780`
* C signature: `void BBREnterLostFeedback(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:454-468`
* Rust item: `exit_lost_feedback`

### C body
```c
{
    if ((IsInAProbeBWState(bbr_state) || bbr_state->state == picoquic_bbr_alg_drain) &&
        !bbr_state->is_handling_lost_feedback &&
        path_x->cnx->cnx_state == picoquic_state_ready) {
        /* Remembering the old cwin, so the state can be restored when the
        * condition is lifted. */
        bbr_state->cwin_before_lost_feedback = path_x->cwin;
        /* setting the congestion window to exactly the bytes in transit, thus
         * preventing any further transmission until the condition is lifted */
        path_x->cwin = path_x->bytes_in_transit;
        bbr_state->is_handling_lost_feedback = 1;
    }
}
```

### Rust body
```rust
    pub(crate) fn has_elapsed_in_phase(&self, interval: u64, current_time: u64) -> bool {
        current_time > self.cycle_stamp + interval
    }
```

## `picoquic/bbr.c:BBRInitFullPipe`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C clears filled_pipe, full_bw, and full_bw_count, while Rust body is init_lower_bounds and updates bw_lo/inflight_lo instead.
* C source: `picoquic/bbr.c:434-439`
* C signature: `void BBRInitFullPipe(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:524-542`
* Rust item: `init_full_pipe`

### C body
```c
{
    bbr_state->filled_pipe = 0;
    bbr_state->full_bw = 0;
    bbr_state->full_bw_count = 0;
}
```

### Rust body
```rust
    pub fn init_lower_bounds(&mut self, path_x: &Path) {
        if self.bw_lo == u64::MAX {
            self.bw_lo = self.max_bw;
        }
        if self.inflight_lo == u64::MAX {
            self.inflight_lo = path_x.cwin;
        }
    }
```

## `picoquic/bbr.c:BBRStartProbeBW_CRUISE`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets pacing_gain, cwnd_gain, and state; Rust body computes target_inflight and changes none of those fields.
* C source: `picoquic/bbr.c:1816-1821`
* C signature: `void BBRStartProbeBW_CRUISE(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:735-747`
* Rust item: `start_probe_bw_cruise`

### C body
```c
{
    bbr_state->pacing_gain = BBRProbeBwCruisePacingGain;  /* pace at rate */
    bbr_state->cwnd_gain = BBRProbeBwCruiseCwndGain;   /* maintain cwnd */
    bbr_state->state = picoquic_bbr_alg_probe_bw_cruise;
}
```

### Rust body
```rust
    pub fn target_inflight(&self, path_x: &Path) -> u64 {
        self.bdp.min(path_x.cwin)
    }
```

## `picoquic/bbr1.c:BBR1EnterProbeRTT`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C enters ProbeRTT and sets pacing/cwnd gains to 1.0, while the shown Rust body only clears packet_conservation after fast recovery.
* C source: `picoquic/bbr1.c:885-890`
* C signature: `void BBR1EnterProbeRTT(picoquic_bbr1_state_t *)`
* Rust source: `rs/fq/src/bbr1.rs:209-221`
* Rust item: `enter_probe_rtt`

### C body
```c
{
    bbr1_state->state = picoquic_bbr1_alg_probe_rtt;
    bbr1_state->pacing_gain = 1.0;
    bbr1_state->cwnd_gain = 1.0;
}
```

### Rust body
```rust
    pub fn after_one_roundtrip_in_fast_recovery(&mut self) {
        self.packet_conservation = false;
    }
```

## `picoquic/bbr1.c:picoquic_bbr1_notify_congestion`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only contains the repeated-loss early return block; it omits cwin reduction/suspension, loss bookkeeping, congestion_sequence update, and state-specific transitions present in C.
* C source: `picoquic/bbr1.c:1118-1157`
* C signature: `void picoquic_bbr1_notify_congestion(picoquic_bbr1_state_t *, picoquic_cnx_t *, picoquic_path_t *, uint64_t, int)`
* Rust source: `rs/fq/src/bbr1.rs:1161-1174`
* Rust item: `notify_congestion`

### C body
```c
{
    /* Apply filter of last loss */
    if ((bbr1_state->cycle_on_loss || current_time < bbr1_state->loss_interval_start + path_x->smoothed_rtt) &&
        (!is_timeout || bbr1_state->last_loss_was_timeout)) {
        /* filter repeated loss events */
        return;
    }
    if (is_timeout || path_x->cwin < PICOQUIC_CWIN_MINIMUM) {
        if (!bbr1_state->is_suspended) {
            bbr1_state->is_suspended = 1;
            bbr1_state->cwin_before_suspension = path_x->cwin;
        }
        path_x->cwin = PICOQUIC_CWIN_MINIMUM;
    } else {
        path_x->cwin = path_x->cwin / 2;
    }
    bbr1_state->loss_interval_start = current_time;
    bbr1_state->last_loss_was_timeout = is_timeout;
    bbr1_state->congestion_sequence = picoquic_cc_get_sequence_number(cnx, path_x);

    /* Update and check the packet loss rate */
    if (bbr1_state->state == picoquic_bbr1_alg_startup_long_rtt) {
        BBR1ExitStartupLongRtt(bbr1_state, path_x, current_time);
    }
    else if (bbr1_state->state == picoquic_bbr1_alg_startup) {
        bbr1_state->filled_pipe = 1;
        BBR1EnterDrain(bbr1_state, path_x, current_time);
    }
    else {
        bbr1_state->cycle_on_loss = 1;
    }
}
```

### Rust body
```rust
        {
            return;
        }
```
