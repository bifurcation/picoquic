# Phase 4C body-only translation audit

Compare each C/Rust pair using only the function bodies shown
below. Do not infer from dependencies, type definitions, callers,
module context, tests, or external knowledge. This is a cheap
superficial check for obvious inconsistencies.

Return only JSON with this shape:

```json
{"reviews":[{"c_id":"...","status":"ok|suspect|definitely_not_ok","rationale":"body-visible reason"}]}
```

Status meanings:
* `ok`: no obvious body-level concern.
* `suspect`: possible mismatch visible from the bodies.
* `definitely_not_ok`: clear mismatch or placeholder-like code.

## Pair `picoquic/quicctx.c:picoquic_subscribe_to_quality_update_per_path_context`
C: `picoquic/quicctx.c:2721-2727 picoquic_subscribe_to_quality_update_per_path_context`
Rust: `rs/fq/src/lib.rs:2774-2782 subscribe_to_quality_update_per_path_context`

### C body
```c
{
    path_x->pacing_rate_update_delta = pacing_rate_delta;
    path_x->rtt_update_delta = rtt_delta;
    picoquic_refresh_path_quality_thresholds(path_x);
}
```

### Rust body
```rust
    ) {
        self.pacing_rate_update_delta = pacing_rate_delta;
        self.rtt_update_delta = rtt_delta;
        self.refresh_quality_thresholds();
    }
```

## Pair `picoquic/quicctx.c:picoquic_remove_cnxid_from_stash`
C: `picoquic/quicctx.c:3055-3091 picoquic_remove_cnxid_from_stash`
Rust: `rs/fq/src/internal.rs:5158-5174 remove_connection_id_from_stash`

### C body
```c
{
    picoquic_remote_cnxid_t* stashed = NULL;

    if (cnx != NULL && remote_cnxid_stash != NULL && remote_cnxid_stash->cnxid_stash_first != NULL && removed != NULL) {
        stashed = remote_cnxid_stash->cnxid_stash_first;
        /* Verify the value of the previous pointer */
        if (previous != NULL) {
            if (previous->next == removed) {
                stashed = removed;
            }
            else {
                previous = NULL;
            }
        }
        /* If the previous pointer was NULL or invalid, reset it */
        if (previous == NULL) {
            while (stashed != NULL && removed != stashed) {
                previous = stashed;
                stashed = stashed->next;
            }
        }
        /* Actually remove the element from the stash */
        if (stashed != NULL) {
            stashed = stashed->next;
            if (previous == NULL) {
                remote_cnxid_stash->cnxid_stash_first = stashed;
            }
            else {
                previous->next = stashed;
            }
            free(removed);
        }
    }
    return stashed;
}
```

### Rust body
```rust
    ) -> Option<usize> {
        let stash = self.remote_connection_id_stashes.get_mut(stash_index)?;
        if removed_index >= stash.connection_ids.len() {
            return None;
        }
        stash.connection_ids.remove(removed_index);
        // Return the index of the next live entry (same index since we removed one).
        if removed_index < stash.connection_ids.len() {
            Some(removed_index)
        } else {
            None
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_last_stream`
C: `picoquic/quicctx.c:3477-3484 picoquic_last_stream`
Rust: `rs/fq/src/internal.rs:9633-9636 last_stream`

### C body
```c
{
#ifdef TOO_CAUTIOUS
    return picoquic_stream_from_node(picosplay_last(&cnx->stream_tree));
#else
    return (picoquic_stream_head_t *)picosplay_last(&cnx->stream_tree);
#endif
}
```

### Rust body
```rust
    pub fn last_stream(&self) -> Option<StreamToken> {
        let st = self.stream_tree.last()?;
        self.stream_tree.get(st).copied()
    }
```

## Pair `picoquic/quicctx.c:picoquic_retire_local_cnxid`
C: `picoquic/quicctx.c:3965-3985 picoquic_retire_local_cnxid`
Rust: `rs/fq/src/lib.rs:2975-2977 retire_local_cnxid`

### C body
```c
{
    picoquic_local_cnxid_list_t* local_cnxid_list = picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 0);

    if (local_cnxid_list != NULL) {
        picoquic_local_cnxid_t* local_cnxid = local_cnxid_list->local_cnxid_first;

        while (local_cnxid != NULL) {
            if (local_cnxid->sequence == sequence) {
                break;
            }
            else {
                local_cnxid = local_cnxid->next;
            }
        }

        if (local_cnxid != NULL) {
            picoquic_delete_local_cnxid_listed(cnx, local_cnxid_list, local_cnxid);
        }
    }
}
```

### Rust body
```rust
    pub fn retire_local_cnxid(&mut self, unique_path_id: u64, sequence: u64) {
        self.retire_local_connection_id(unique_path_id, sequence);
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_server_cnxid`
C: `picoquic/quicctx.c:4483-4487 picoquic_get_server_cnxid`
Rust: `rs/fq/src/lib.rs:3061-3072 server_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return (cnx->client_mode) ? cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id : cnx->path[0]->first_tuple->p_local_cnxid->cnx_id;
}
```

### Rust body
```rust
    pub fn logging_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }
```
