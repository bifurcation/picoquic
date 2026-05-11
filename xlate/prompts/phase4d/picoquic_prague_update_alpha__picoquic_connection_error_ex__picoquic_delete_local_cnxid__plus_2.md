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

## `picoquic/prague.c:picoquic_prague_update_alpha`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C computes frac, applies suspect/increase/smoothing logic, updates alpha, and logs; Rust body shown is only a trailing else returning 0.
* C source: `picoquic/prague.c:210-249`
* C signature: `void picoquic_prague_update_alpha(picoquic_path_t *, picoquic_prague_state_t *, uint64_t, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/prague.rs:147-159`
* Rust item: `prague_update_alpha`

### C body
```c
{
    uint64_t frac = 0;
    int is_suspect = 0;

    if (delta_ce > 0) {
        frac = (delta_ce * 1024) / (delta_ce + delta_ect1);
    }
    else {
        frac = 0;
    }

    if (pr_state->l4s_update_sent != 0 && frac >= 512 && pr_state->alpha < 128 &&
        current_time - pr_state->recovery_stamp > path_x->smoothed_rtt) {
        /*
         * the epoch lasted more than the RTT. This is most
         * probably due to period of inactivity, then effects of imprecise
         * tuning of pacing's leaky bucket algorithm. Limiting the
         * fraction frac to about 1/8th to avoid too much bad effects. */
        is_suspect = 1;
        frac = 128;
    }

    if (delta_ce > 0 || delta_ect1 > 0) {
        if (frac > pr_state->alpha && (frac >= 512 || is_suspect)) {
            pr_state->alpha = frac;
        }
        else
        {
            uint64_t alpha_shifted = pr_state->alpha << PRAGUE_SHIFT_G;
            alpha_shifted -= pr_state->alpha;
            alpha_shifted += frac;
            pr_state->alpha = alpha_shifted >> PRAGUE_SHIFT_G;
        }
    }
    picoquic_log_app_message(path_x->cnx,
        "Prague: %" PRIu64 ",%d,%d,%d,%" PRIu64 ",%" PRIu64,
        current_time, (int)delta_ect1, (int)delta_ce, (int)pr_state->alpha, path_x->cwin, path_x->rtt_sample);
}
```

### Rust body
```rust
    } else {
        0
    };
```

## `picoquic/quicctx.c:picoquic_connection_error_ex`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C clamps large local_error values, uses state-specific transitions, always returns PICOQUIC_ERROR_DETECTED; Rust lacks the clamp/state distinctions and returns local_error as i32.
* C source: `picoquic/quicctx.c:4991-5019`
* C signature: `int picoquic_connection_error_ex(picoquic_cnx_t *, uint64_t, uint64_t, const char *)`
* Rust source: `rs/fq/src/internal.rs:5728-5743`
* Rust item: `connection_error_ex`

### C body
```c
{
    if (local_error > PICOQUIC_ERROR_CLASS) {
        local_error = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
    }

    if (cnx->cnx_state == picoquic_state_ready || 
        cnx->cnx_state == picoquic_state_client_ready_start || cnx->cnx_state == picoquic_state_server_false_start) {
        cnx->local_error = local_error;
        cnx->local_error_reason = local_reason;
        cnx->cnx_state = picoquic_state_disconnecting;
    } else if (cnx->cnx_state < picoquic_state_server_false_start) {
        if (cnx->cnx_state != picoquic_state_handshake_failure &&
            cnx->cnx_state != picoquic_state_handshake_failure_resend) {
            cnx->local_error = local_error;
            cnx->local_error_reason = local_reason;
            cnx->cnx_state = picoquic_state_handshake_failure;
        }
    }

    cnx->offending_frame_type = frame_type;

    picoquic_log_app_message(cnx, "Protocol error 0x%x, frame %" PRIu64 ", reason: %s",
        local_error, frame_type, (local_reason==NULL)?"?":local_reason);
    DBG_PRINTF("Protocol error 0x%x, frame %" PRIu64 ", reason: %s",
        local_error, frame_type, (local_reason==NULL)?"?":local_reason);

    return PICOQUIC_ERROR_DETECTED;
}
```

### Rust body
```rust
    ) -> i32 {
        // C: picoquic_connection_error_ex
        self.local_error = local_error;
        self.offending_frame_type = frame_type;
        self.local_error_reason = local_reason.map(|s| s.to_owned());
        // Move to disconnecting state if not already past that.
        if (self.connection_state as u32) < crate::State::Disconnecting as u32 {
            self.connection_state = crate::State::Disconnecting;
        }
        local_error as i32
    }
```

## `picoquic/quicctx.c:picoquic_delete_local_cnxid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C finds the local CID list and deletes the given listed CID; Rust simply returns initial_connection_id.
* C source: `picoquic/quicctx.c:3927-3932`
* C signature: `void picoquic_delete_local_cnxid(picoquic_cnx_t *, picoquic_local_cnxid_t *)`
* Rust source: `rs/fq/src/lib.rs:2943-2945`
* Rust item: `local_connection_id`

### C body
```c
{
    picoquic_local_cnxid_list_t* local_cnxid_list = picoquic_find_or_create_local_cnxid_list(cnx, l_cid->path_id, 0);

    picoquic_delete_local_cnxid_listed(cnx, local_cnxid_list, l_cid);
}
```

### Rust body
```rust
    pub fn local_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }
```

## `picoquic/quicctx.c:picoquic_enable_keep_alive`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C enables keep-alive and computes or assigns an interval; Rust body disables keep-alive by setting the interval to zero.
* C source: `picoquic/quicctx.c:5459-5478`
* C signature: `void picoquic_enable_keep_alive(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:4457-4464`
* Rust item: `enable_keep_alive`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    if (interval == 0) {
        /* Use the negotiated value */
        uint64_t idle_timeout = cnx->idle_timeout;
        if (idle_timeout == 0) {
            /* Idle timeout is only initialized after parameters are negotiated  */
            idle_timeout = cnx->local_parameters.max_idle_timeout * 1000ull;
        }
        /* Ensure at least 3 PTO*/
        if (idle_timeout < 3 * cnx->path[0]->retransmit_timer) {
            idle_timeout = 3 * cnx->path[0]->retransmit_timer;
        }
        /* set interval to half that value */
        cnx->keep_alive_interval = idle_timeout / 2;
    } else {
        cnx->keep_alive_interval = interval;
    }
}
```

### Rust body
```rust
    pub fn disable_keep_alive(&mut self) {
        self.keep_alive_interval = Duration::from_ticks(0);
    }
```

## `picoquic/quicctx.c:picoquic_find_path_by_address`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C searches paths and updates partial_match; Rust shown body only returns -1 when both addresses are none.
* C source: `picoquic/quicctx.c:2153-2201`
* C signature: `int picoquic_find_path_by_address(picoquic_cnx_t *, const struct sockaddr *, const struct sockaddr *, int *)`
* Rust source: `rs/fq/src/internal.rs:4796-4805`
* Rust item: `find_path_by_address`

### C body
```c
{
    int path_id = -1;
    int is_null_from = 0;
    struct sockaddr_storage null_addr;

    *partial_match = -1;

    if (addr_peer != NULL || addr_local != NULL) {
        if (addr_peer == NULL || addr_local == NULL) {
            memset(&null_addr, 0, sizeof(struct sockaddr_storage));
            if (addr_peer == NULL) {
                addr_peer = (struct sockaddr*) & null_addr;
            }
            else {
                addr_local = (struct sockaddr*) & null_addr;
            }
            is_null_from = 1;
        }
        else if (addr_local->sa_family == 0) {
            is_null_from = 1;
        }

        /* Find whether an existing path matches the  pair of addresses */
        for (int i = 0; i < cnx->nb_paths; i++) {
            if (picoquic_compare_addr((struct sockaddr*) & cnx->path[i]->first_tuple->peer_addr,
                addr_peer) == 0) {
                if (cnx->path[i]->first_tuple->local_addr.ss_family == 0) {
                    *partial_match = i;
                }
                else if (picoquic_compare_addr((struct sockaddr*) & cnx->path[i]->first_tuple->local_addr,
                    addr_local) == 0) {
                    path_id = i;
                    break;
                }
            }

            if (path_id < 0 && is_null_from) {
                path_id = *partial_match;
                *partial_match = -1;
            }
        }
    }

    return path_id;
}
```

### Rust body
```rust
        if addr_peer.is_none() && addr_local.is_none() {
            return -1;
        }
```
