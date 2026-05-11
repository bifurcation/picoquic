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

## `picoquic/quicctx.c:picoquic_probe_new_path`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C forwards to picoquic_probe_new_path_ex; Rust is a placeholder-like stub returning Err(Error::Generic).
* C source: `picoquic/quicctx.c:2552-2556`
* C signature: `int picoquic_probe_new_path(picoquic_cnx_t *, const struct sockaddr *, const struct sockaddr *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:2436-2444`
* Rust item: `probe_new_path`

### C body
```c
{
    return picoquic_probe_new_path_ex(cnx, addr_peer, addr_local, 0, current_time, 0);
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves path creation and network probing.
        Err(Error::Generic)
    }
```

## `picoquic/quicctx.c:picoquic_remove_not_before_from_stash`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C marks entries for removal, queues retire frames, conditionally deletes acknowledged entries, and renews or demotes paths; Rust only retains entries with sequence >= not_before and returns a removal count.
* C source: `picoquic/quicctx.c:3154-3234`
* C signature: `uint64_t picoquic_remove_not_before_from_stash(picoquic_cnx_t *, picoquic_remote_cnxid_stash_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:5292-5309`
* Rust item: `remove_not_before_from_stash`

### C body
```c
{
    uint64_t ret = 0;
    if (cnxid_stash != NULL) {

        picoquic_remote_cnxid_t* next_stash = cnxid_stash->cnxid_stash_first;
        picoquic_remote_cnxid_t* previous_stash = NULL;

        while (ret == 0 && next_stash != NULL) {
            next_stash->needs_removal |= (next_stash->sequence < not_before);
            if (next_stash->needs_removal && next_stash->nb_path_references == 0) {
                if (!next_stash->retire_sent) {
                    ret = picoquic_queue_retire_connection_id_frame(cnx, cnxid_stash->unique_path_id, next_stash->sequence);
                    if (ret == 0) {
                        next_stash->retire_sent = 1;
                    }
                }
                if (ret == 0 && next_stash->retire_acked) {
                    next_stash = picoquic_remove_cnxid_from_stash(cnx, cnxid_stash, next_stash, previous_stash);
                }
                else {
                    previous_stash = next_stash;
                    next_stash = next_stash->next;
                }
            }
            else {
                previous_stash = next_stash;
                next_stash = next_stash->next;
            }
        }

        /* We need to stop transmitting data to the old CID. But we cannot just delete
        * the correspondng paths,because there may be some data in transit. We must
        * also ensure that at least one default path migrates successfully to a
        * valid CID. As long as new CID are available, we can simply replace the
        * old one by a new one. If no CID is available, the old path should be marked
        * as failing, and thus scheduled for deletion after a time-out */

        if (cnx->is_multipath_enabled) {
            int path_id = picoquic_find_path_by_unique_id(cnx, cnxid_stash->unique_path_id);
            if (path_id >= 0) {
                if (cnx->path[path_id]->first_tuple->p_remote_cnxid->sequence < not_before &&
                    cnx->path[path_id]->first_tuple->p_remote_cnxid->cnx_id.id_len > 0 &&
                    !cnx->path[path_id]->path_is_demoted) {
                    ret = picoquic_renew_connection_id(cnx, path_id);
                    if (ret != 0) {
                        DBG_PRINTF("Renew CNXID returns %x\n", ret);
                        if (path_id == 0) {
                            ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
                        }
                        else {
                            ret = 0;
                            picoquic_demote_path(cnx, path_id, current_time, 0);
                        }
                    }
                }
            }
        }
        else {
            for (int i = 0; ret == 0 && i < cnx->nb_paths; i++) {
                if (cnx->path[i]->first_tuple->p_remote_cnxid->sequence < not_before &&
                    cnx->path[i]->first_tuple->p_remote_cnxid->cnx_id.id_len > 0 &&
                    !cnx->path[i]->path_is_demoted) {
                    ret = picoquic_renew_connection_id(cnx, i);
                    if (ret != 0) {
                        DBG_PRINTF("Renew CNXID returns %x\n", ret);
                        if (i == 0) {
                            ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
                        }
                        else {
                            ret = 0;
                            picoquic_demote_path(cnx, i, current_time, 0);
                        }
                    }
                }
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> u64 {
        // Remove all CIDs whose sequence < not_before.
        let mut removed = 0u64;
        connection_id_stash.connection_ids.retain(|r| {
            if r.sequence < not_before {
                removed += 1;
                false
            } else {
                true
            }
        });
        removed
    }
```

## `picoquic/quicctx.c:picoquic_set_alpn_select_fn`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C clears default_alpn and sets alpn_select_fn; Rust sets default_callback_fn.
* C source: `picoquic/quicctx.c:4727-4735`
* C signature: `void picoquic_set_alpn_select_fn(picoquic_quic_t *, picoquic_alpn_select_fn)`
* Rust source: `rs/fq/src/lib.rs:1875-1885`
* Rust item: `set_alpn_select_fn`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    if (quic->default_alpn != NULL) {
        free((void *)quic->default_alpn);
        quic->default_alpn = NULL;
    }
    quic->alpn_select_fn = alpn_select_fn;
}
```

### Rust body
```rust
    pub fn set_default_callback(&mut self, callback: Option<Box<dyn StreamDataCallback>>) {
        self.default_callback_fn = callback;
    }
```

## `picoquic/quicctx.c:picoquic_set_default_callback`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C assigns default_callback_fn and default_callback_ctx; Rust body shown assigns tls_callbacks only.
* C source: `picoquic/quicctx.c:4750-4756`
* C signature: `void picoquic_set_default_callback(picoquic_quic_t *, picoquic_stream_data_cb_fn, void *)`
* Rust source: `rs/fq/src/lib.rs:1883-1895`
* Rust item: `set_default_callback`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_callback_fn = callback_fn;
    quic->default_callback_ctx = callback_ctx;
}
```

### Rust body
```rust
    ) {
        self.tls_callbacks = callback;
    }
```

## `picoquic/quicctx.c:picoquic_set_default_stateless_reset_min_interval`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets stateless reset timing fields; Rust body sets max_simultaneous_logs.
* C source: `picoquic/quicctx.c:4758-4763`
* C signature: `void picoquic_set_default_stateless_reset_min_interval(picoquic_quic_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:1899-1907`
* Rust item: `set_default_stateless_reset_min_interval`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->stateless_reset_next_time = picoquic_get_quic_time(quic);
    quic->stateless_reset_min_interval = min_interval_usec;
}
```

### Rust body
```rust
    pub fn set_max_simultaneous_logs(&mut self, max_simultaneous_logs: u32) {
        self.max_simultaneous_logs = max_simultaneous_logs;
    }
```
