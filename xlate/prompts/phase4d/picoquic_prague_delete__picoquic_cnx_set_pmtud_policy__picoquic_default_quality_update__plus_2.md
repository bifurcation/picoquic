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

## `picoquic/prague.c:picoquic_prague_delete`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C frees and nulls congestion_alg_state; Rust body shown is an unrelated alg_observe implementation.
* C source: `picoquic/prague.c:396-403`
* C signature: `void picoquic_prague_delete(picoquic_path_t *)`
* Rust source: `rs/fq/src/prague.rs:425-435`
* Rust item: `alg_delete`

### C body
```c
{
    if (path_x->congestion_alg_state != NULL) {
        free(path_x->congestion_alg_state);
        path_x->congestion_alg_state = NULL;
    }
}
```

### Rust body
```rust
    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|state| state.downcast_ref::<PragueState>())
            .map(PragueState::observe)
    }
```

## `picoquic/quicctx.c:picoquic_cnx_set_pmtud_policy`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C assigns the provided policy directly; Rust body is named set_pmtud_required and maps a bool to only Required or Basic.
* C source: `picoquic/quicctx.c:4557-4561`
* C signature: `void picoquic_cnx_set_pmtud_policy(picoquic_cnx_t *, picoquic_pmtud_policy_enum)`
* Rust source: `rs/fq/src/lib.rs:2875-2887`
* Rust item: `set_pmtud_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->pmtud_policy = pmtud_policy;
}
```

### Rust body
```rust
    pub fn set_pmtud_required(&mut self, is_pmtud_required: bool) {
        self.pmtud_policy = if is_pmtud_required {
            PmtudPolicy::Required
        } else {
            PmtudPolicy::Basic
        };
    }
```

## `picoquic/quicctx.c:picoquic_default_quality_update`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets pacing/rtt update deltas; Rust sets cwin_max.
* C source: `picoquic/quicctx.c:2762-2767`
* C signature: `void picoquic_default_quality_update(picoquic_quic_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:1719-1728`
* Rust item: `default_quality_update`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->pacing_rate_update_delta = pacing_rate_delta;
    quic->rtt_update_delta = rtt_delta;
}
```

### Rust body
```rust
    pub fn set_cwin_max(&mut self, cwin_max: u64) {
        self.cwin_max = if cwin_max == 0 { u64::MAX } else { cwin_max };
    }
```

## `picoquic/quicctx.c:picoquic_demote_path`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust only marks a path demoted and sets demotion_time to current_time; C computes demotion_time with a timer offset, sets path_demotion_needed, and performs substantial multipath abandon/reordering logic.
* C source: `picoquic/quicctx.c:2052-2116`
* C signature: `void picoquic_demote_path(picoquic_cnx_t *, int, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:4726-4731`
* Rust item: `demote_path`

### C body
```c
{
    if (!cnx->path[path_index]->path_is_demoted) {
        uint64_t demote_timer = cnx->path[path_index]->retransmit_timer;

        if (demote_timer < PICOQUIC_INITIAL_MAX_RETRANSMIT_TIMER &&
            !cnx->is_multipath_enabled) {
            demote_timer = PICOQUIC_INITIAL_MAX_RETRANSMIT_TIMER;
        }

        cnx->path[path_index]->path_is_demoted = 1;
        cnx->path[path_index]->demotion_time = current_time + 3* demote_timer;
        cnx->path_demotion_needed = 1;

        /* TODO: add suspended callback */
        if (cnx->is_multipath_enabled) {
             /* Special case for path 0: we want to reorder the paths so the path[0]
             * is always a valid path.
             */
            if (path_index == 0) {
                int alt_path0 = 0;
                for (int i = 1; i < cnx->nb_paths; i++) {
                    if (cnx->path[i]->first_tuple->p_remote_cnxid != NULL) {
                        alt_path0 = i;
                        break;
                    }
                }
                if (alt_path0 != 0) {
                    picoquic_path_t* path_x = cnx->path[0];
                    cnx->path[0] = cnx->path[alt_path0];
                    cnx->path[alt_path0] = path_x;
                    path_index = alt_path0;
                }
            }
            if (path_index == 0) {
                picoquic_log_app_message(cnx, "Cannot demote path index 0, unique_id %" PRIu64", was reason % " PRIu64,
                    cnx->path[path_index]->unique_path_id, reason);
            }
            else if (!cnx->path[path_index]->path_abandon_sent) {
                uint64_t path_id = cnx->path[path_index]->unique_path_id;
                if (picoquic_queue_path_abandon_frame(cnx, path_id, reason) == 0){
                    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = 
                        picoquic_find_or_create_remote_cnxid_stash(cnx, 
                            cnx->path[path_index]->unique_path_id, 0);
                    if (remote_cnxid_stash != NULL && path_index != 0) {
                        cnx->path[path_index]->first_tuple->p_remote_cnxid = NULL;
                        picoquic_delete_remote_cnxid_stash(cnx, remote_cnxid_stash);
                    }
                    else {
                        DBG_PRINTF("Cannot abandon path[%d]", cnx->path[path_index]->unique_path_id);
                    }
                    picoquic_log_app_message(cnx, "Abandon path, unique_id %" PRIu64", reason % " PRIu64,
                        cnx->path[path_index]->unique_path_id, reason);
                    cnx->path[path_index]->path_abandon_sent = 1;
                } else {
                    picoquic_log_app_message(cnx, "Cannot queue abandon path [%" PRIu64 "]",
                        cnx->path[path_index]->unique_path_id);
                }
            }
        }
    }
}
```

### Rust body
```rust
        if let Some(p) = self.paths.get_mut(idx) {
            p.path_is_demoted = true;
            p.demotion_time = current_time;
        }
```

## `picoquic/quicctx.c:picoquic_find_local_cnxid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body retires a local connection ID by path and sequence instead of searching for and returning a matching local connection ID.
* C source: `picoquic/quicctx.c:4017-4034`
* C signature: `picoquic_local_cnxid_t * picoquic_find_local_cnxid(picoquic_cnx_t *, uint64_t, picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/lib.rs:2965-2977`
* Rust item: `find_local_cnxid`

### C body
```c
{
    picoquic_local_cnxid_t* local_cnxid = NULL;
    picoquic_local_cnxid_list_t* local_cnxid_list = picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 0);
    
    if (local_cnxid_list != NULL && (local_cnxid = local_cnxid_list->local_cnxid_first) != NULL) {
        while (local_cnxid != NULL) {
            if (picoquic_compare_connection_id(&local_cnxid->cnx_id, cnxid) == 0) {
                break;
            }
            else {
                local_cnxid = local_cnxid->next;
            }
        }
    }
    
    return local_cnxid;
}
```

### Rust body
```rust
    pub fn retire_local_cnxid(&mut self, unique_path_id: u64, sequence: u64) {
        self.retire_local_connection_id(unique_path_id, sequence);
    }
```
