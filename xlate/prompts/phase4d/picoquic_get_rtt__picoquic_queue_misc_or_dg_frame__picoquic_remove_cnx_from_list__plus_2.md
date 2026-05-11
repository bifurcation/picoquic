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

## `picoquic/quicctx.c:picoquic_get_rtt`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns path[0] smoothed RTT directly; Rust returns first path RTT ticks or 0 when absent, adding absent-path behavior not visible in C.
* C source: `picoquic/quicctx.c:5438-5442`
* C signature: `uint64_t picoquic_get_rtt(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4672-4677`
* Rust item: `rtt`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->smoothed_rtt;
}
```

### Rust body
```rust
    pub fn rtt(&self) -> u64 {
        self.paths
            .first()
            .map(|p| p.smoothed_rtt.ticks())
            .unwrap_or(0)
```

## `picoquic/quicctx.c:picoquic_queue_misc_or_dg_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both queue a misc frame, but C handles allocation failure, first/last linkage, wake-time reinsertion, and returns a memory error; Rust just push_back's and always returns Ok.
* C source: `picoquic/quicctx.c:4818-4842`
* C signature: `int picoquic_queue_misc_or_dg_frame(picoquic_cnx_t *, picoquic_misc_frame_header_t **, picoquic_misc_frame_header_t **, const uint8_t *, size_t, int, picoquic_packet_context_enum)`
* Rust source: `rs/fq/src/internal.rs:13816-13829`
* Rust item: `queue_misc_or_dg_frame`

### C body
```c
{
    int ret = 0;
    picoquic_misc_frame_header_t* misc_frame = picoquic_create_misc_frame(bytes, length, is_pure_ack, pc);

    if (misc_frame == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    } else {
        if (*last == NULL) {
            *first = misc_frame;
            *last = misc_frame;
        }
        else {
            (*last)->next_misc_frame = misc_frame;
            misc_frame->previous_misc_frame = *last;
            *last = misc_frame;
        }
    }

    picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        queue.push_back(MiscFrameHeader {
            bytes: bytes.to_vec(),
            packet_context: pc,
            is_pure_ack: if is_pure_ack { 1 } else { 0 },
        });
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_remove_cnx_from_list`
* Phase 4C status: `suspect`
* Phase 4C rationale: C updates linked-list previous/next/head/tail fields before unregistering and decrementing; Rust only unregisters and decrements a count.
* C source: `picoquic/quicctx.c:1450-1469`
* C signature: `void picoquic_remove_cnx_from_list(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2167-2173`
* Rust item: `remove_cnx_from_list`

### C body
```c
{
    if (cnx->next_in_table == NULL) {
        cnx->quic->cnx_last = cnx->previous_in_table;
    } else {
        cnx->next_in_table->previous_in_table = cnx->previous_in_table;
    }

    if (cnx->previous_in_table == NULL) {
        cnx->quic->cnx_list = cnx->next_in_table;
    }
    else {
        cnx->previous_in_table->next_in_table = cnx->next_in_table;
    }

    picoquic_unregister_net_icid(cnx);
    picoquic_unregister_net_secret(cnx);

    cnx->quic->current_number_connections--;
}
```

### Rust body
```rust
    pub(crate) fn remove_cnx_from_list(&mut self, connection: ConnectionToken) {
        self.unregister_net_icid(connection);
        self.unregister_net_secret(connection);
        if self.current_number_connections > 0 {
            self.current_number_connections -= 1;
        }
    }
```

## `picoquic/quicctx.c:picoquic_set_default_congestion_algorithm_by_name`
* Phase 4C status: `suspect`
* Phase 4C rationale: C delegates with a possibly null lookup result and no visible error return; Rust returns an error when the name is not found.
* C source: `picoquic/quicctx.c:5345-5348`
* C signature: `void picoquic_set_default_congestion_algorithm_by_name(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/lib.rs:4597-4609`
* Rust item: `set_default_congestion_algorithm_by_name`

### C body
```c
{
    picoquic_set_default_congestion_algorithm_ex(quic, picoquic_get_congestion_algorithm(alg_name), NULL);
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        match get_congestion_algorithm(alg_name) {
            Some(alg) => {
                self.default_congestion_alg = Some(alg);
                self.default_congestion_alg_option_string = None;
                Ok(())
            }
            None => Err(Error::InvalidArgument),
        }
    }
```

## `picoquic/quicctx.c:picoquic_set_path_status`
* Phase 4C status: `suspect`
* Phase 4C rationale: C marks backup for any status other than available and queues a path status frame; Rust only sets backup when status is Backup and has no visible frame-queueing call.
* C source: `picoquic/quicctx.c:2801-2810`
* C signature: `int picoquic_set_path_status(picoquic_cnx_t *, uint64_t, picoquic_path_status_enum)`
* Rust source: `rs/fq/src/lib.rs:2575-2590`
* Rust item: `set_path_status`

### C body
```c
{
    int ret = 0;
    int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        cnx->path[path_id]->path_is_backup = (status != picoquic_path_status_available);
        ret = picoquic_queue_path_available_or_backup_frame(cnx, cnx->path[path_id], status);
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        if let Some(path) = self
            .paths
            .iter_mut()
            .find(|p| p.unique_path_id == unique_path_id)
        {
            path.path_is_backup = status == PathStatus::Backup;
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }
```
