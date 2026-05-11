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

Owned Rust file(s): `rs/fq/src/c4.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/c4.c:c4_init`
* Phase 4C status: `suspect`
* Phase 4C rationale: C allocates or reuses path congestion state, sets is_lost_feedback_notification_required, calls c4_reset, and stores the state pointer; Rust shown only resets an existing state and enters initial.
* Phase 4D analysis: This is a real mismatch. The Rust reset method only corresponds to c4_reset; it does not allocate/store C4 state, apply the init option string, or set lost-feedback notification. The registry currently maps c4 to the baseline control adapter, so C4 init behavior is not wired.
* Phase 4D fix note: Add/wire a real C4 congestion-control adapter/init path that creates or reuses C4State in path.congestion_alg_state, preserves the init option string, calls reset, and ensures lost-feedback notification is enabled when C4 is selected.
* C source: `picoquic/c4.c:625-641`
* C signature: `void c4_init(picoquic_path_t *, const char *, uint64_t)`
* Rust source: `rs/fq/src/c4.rs:834-873`
* Rust item: `reset`

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

### Rust body
```rust
            }
        }
    }

    /// C: `c4_reset` (picoquic/c4.c:517)
    ///
    /// Zeroes the C4 state (preserving `option_string`), then re-initialises
    /// via `enter_initial`.  Called on algorithm reset notifications and during
    /// `c4_init`.
    ///
    /// The C signature carries `option_string` as an explicit parameter so the
    /// caller can supply it directly (e.g. from `c4_init`).  In the Rust
    /// translation the field is owned by [`C4State`], so the method preserves
    /// the existing value — equivalent to the C pattern
    /// `c4_reset(state, path, state->option_string)` used in the Reset
    /// notification path.
    pub fn reset(&mut self, path_x: &mut Path, connection: &Connection) {
        let option_string = self.option_string.take();
        // Zero all fields.
        *self = C4State {
            alg_state: C4AlgState::default(),
            nominal_rate: 0,
            nominal_max_rtt: 0,
            initial_cwnd: 0,
            running_min_rtt: u64::MAX,
            alpha_1024_current: C4_ALPHA_INITIAL,
            alpha_1024_previous: 0,
            nb_packets_in_startup: 0,
            era_sequence: 0,
            nb_cruise_left_before_push: 0,
            seed_cwin: 0,
            seed_rate: 0,
            probe_level: 0,
            nb_eras_no_increase: 0,
            push_rate_old: 0,
            push_alpha: 0,
            era_max_rtt: 0,
            era_min_rtt: 0,
            delay_threshold: 0,
            recent_delay_excess: 0,
```
