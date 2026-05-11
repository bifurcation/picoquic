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

## `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_clear_crypto_errors`
* Phase 4C status: `suspect`
* Phase 4C rationale: C calls ERR_clear_error; Rust calls ErrorStack::get() and drops the result, which is not visibly the same clear operation from the body alone.
* C source: `picoquic/picoquic_ptls_openssl.c:334-340`
* C signature: `void picoquic_openssl_clear_crypto_errors(void)`
* Rust source: `rs/fq/src/sys/openssl.rs:467-469`
* Rust item: `clear_crypto_errors`

### C body
```c
{
    ERR_clear_error();
}
```

### Rust body
```rust
pub fn clear_crypto_errors() {
    drop(openssl::error::ErrorStack::get());
}
```

## `picoquic/picosocks.c:picoquic_set_key_log_file_from_env`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks whether key logging is enabled and reads SSLKEYLOGFILE before setting the file; Rust snippet only calls set_key_log_file on an already available path, with surrounding condition not visible.
* C source: `picoquic/picosocks.c:1369-1396`
* C signature: `void picoquic_set_key_log_file_from_env(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:1154-1159`
* Rust item: `set_key_log_file_from_env`

### C body
```c
{
#ifdef PICOQUIC_WITHOUT_SSLKEYLOG
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(quic);
#endif /* WINDOWS*/
#else
    if (picoquic_is_sslkeylog_enabled(quic)) {
        char* keylog_filename = NULL;

#ifdef _WINDOWS
        size_t len;

        if (_dupenv_s(&keylog_filename, &len, "SSLKEYLOGFILE") != 0 ||
            keylog_filename == NULL) {
            return;
        }
#else
        keylog_filename = getenv("SSLKEYLOGFILE");
        if (keylog_filename == NULL) {
            return;
        }
#endif

        picoquic_set_key_log_file(quic, keylog_filename);
    }
#endif /* PICOQUIC_WITHOUT_SSLKEYLOG */
}
```

### Rust body
```rust
        {
            self.set_key_log_file(Some(&path));
        }
```

## `picoquic/quicctx.c:picoquic_add_output_streams`
* Phase 4C status: `suspect`
* Phase 4C rationale: C filters by old/new limits plus local-stream and bidirectional/unidirectional checks; Rust body shown only filters by stream id range.
* C source: `picoquic/quicctx.c:3619-3636`
* C signature: `void picoquic_add_output_streams(picoquic_cnx_t *, uint64_t, uint64_t, unsigned int)`
* Rust source: `rs/fq/src/internal.rs:9646-9659`
* Rust item: `add_output_streams`

### C body
```c
{
    uint64_t old_rank = STREAM_RANK_FROM_ID(old_limit);
    uint64_t first_new_id = STREAM_ID_FROM_RANK(old_rank + 1ull, cnx->client_mode, !is_bidir);
    picoquic_stream_head_t* stream = picoquic_find_stream(cnx, first_new_id );

    while (stream) {
        if (stream->stream_id > old_limit) {
            if (stream->stream_id > new_limit) {
                break;
            }
            if (IS_LOCAL_STREAM_ID(stream->stream_id, cnx->client_mode) && IS_BIDIR_STREAM_ID(stream->stream_id) == is_bidir) {
                picoquic_insert_output_stream(cnx, stream);
            }
        }
        stream = picoquic_next_stream(stream);
    }
}
```

### Rust body
```rust
            .filter_map(|s| {
                if s.stream_id > old_limit && s.stream_id <= new_limit {
                    s.stream_tree_membership
                        .and_then(|st| self.stream_tree.get(st).copied())
                } else {
                    None
                }
            })
```

## `picoquic/quicctx.c:picoquic_context_from_epoch`
* Phase 4C status: `suspect`
* Phase 4C rationale: C shows both valid and invalid epoch returns; Rust snippet shows only the valid branch and an unfinished else.
* C source: `picoquic/quicctx.c:382-392`
* C signature: `picoquic_packet_context_enum picoquic_context_from_epoch(int)`
* Rust source: `rs/fq/src/internal.rs:2160-2169`
* Rust item: `context_from_epoch`

### C body
```c
{
    static picoquic_packet_context_enum const pc[4] = {
        picoquic_packet_context_initial,
        picoquic_packet_context_application,
        picoquic_packet_context_handshake,
        picoquic_packet_context_application
    };

    return (epoch >= 0 && epoch < 4) ? pc[epoch] : 0;
}
```

### Rust body
```rust
    if (0..4).contains(&epoch) {
        PC[epoch as usize]
    } else {
```

## `picoquic/quicctx.c:picoquic_delete_remote_cnxid_stash`
* Phase 4C status: `suspect`
* Phase 4C rationale: C removes every cnxid from the stash before unlinking and freeing it; Rust only swap_removes the stash index, with no visible per-cnxid removal equivalent.
* C source: `picoquic/quicctx.c:3248-3269`
* C signature: `void picoquic_delete_remote_cnxid_stash(picoquic_cnx_t *, picoquic_remote_cnxid_stash_t *)`
* Rust source: `rs/fq/src/internal.rs:5314-5317`
* Rust item: `delete_remote_connection_id_stash`

### C body
```c
{
    picoquic_remote_cnxid_stash_t* previous = cnx->first_remote_cnxid_stash;

    while (cnxid_stash->cnxid_stash_first != NULL) {
        picoquic_remove_cnxid_from_stash(cnx, cnxid_stash, cnxid_stash->cnxid_stash_first, NULL);
    }

    if (previous == cnxid_stash) {
        cnx->first_remote_cnxid_stash = cnxid_stash->next_stash;
    }
    else {
        while (previous != NULL) {
            if (previous->next_stash == cnxid_stash) {
                previous->next_stash = cnxid_stash->next_stash;
                break;
            }
            previous = previous->next_stash;
        }
    }
    free(cnxid_stash);
}
```

### Rust body
```rust
        if stash_index < self.remote_connection_id_stashes.len() {
            self.remote_connection_id_stashes.swap_remove(stash_index);
        }
```
