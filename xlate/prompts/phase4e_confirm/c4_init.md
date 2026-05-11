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

## `picoquic/c4.c:c4_init`
* Phase 4C status: `suspect`
* Phase 4C rationale: C allocates or reuses path congestion state, sets is_lost_feedback_notification_required, calls c4_reset, and stores the state pointer; Rust shown only resets an existing state and enters initial.
* Prior Phase 4D analysis: This is a real mismatch. The Rust reset method only corresponds to c4_reset; it does not allocate/store C4 state, apply the init option string, or set lost-feedback notification. The registry currently maps c4 to the baseline control adapter, so C4 init behavior is not wired.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: C4 is now wired through a real congestion-control init path matching C allocation/reuse, option propagation, reset, and lost-feedback subscription behavior.
* Phase 4E fix summary: Added c4_init and C4CongestionControl, wired the c4 registry entry to it, extended alg_init to receive the owning connection, preserved default/init option strings, and stored/reused C4State in path.congestion_alg_state.
* C source: `picoquic/c4.c:625-641`
* C signature: `void c4_init(picoquic_path_t *, const char *, uint64_t)`
* Current Rust source: `rs/fq/src/c4.rs:1002-1018`
* Current Rust item: `c4_init`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    c4_state_t* c4_state = path_x->congestion_alg_state;
    
    if (c4_state == NULL) {
        c4_state = (c4_state_t*)malloc(sizeof(c4_state_t));
    }
    
    if (c4_state != NULL){
        path_x->cnx->is_lost_feedback_notification_required = 1;
        
        c4_reset(c4_state, path_x, option_string);
    }

    path_x->congestion_alg_state = (void*)c4_state;
}
```

### Current Rust body
```rust
) {
    let mut state = path_x
        .congestion_alg_state
        .take()
        .and_then(|boxed| boxed.downcast::<C4State>().ok())
        .map(|boxed| *boxed)
        .unwrap_or_else(|| C4State::zeroed_with_option(None));

    connection.is_lost_feedback_notification_required = true;
    state.reset_with_option(path_x, connection, option_string);
    path_x.congestion_alg_state = Some(Box::new(state));
}
```
