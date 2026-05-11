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

## `picoquic/quicctx.c:picoquic_get_cnx_state`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns connection state; Rust sets padding policy.
* C source: `picoquic/quicctx.c:4501-4505`
* C signature: `picoquic_state_enum picoquic_get_cnx_state(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2835-2843`
* Rust item: `state`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->cnx_state;
}
```

### Rust body
```rust
    pub fn set_padding_policy(&mut self, padding_multiple: u32, padding_minsize: u32) {
        self.padding_multiple = padding_multiple;
        self.padding_minsize = padding_minsize;
    }
```

## `picoquic/quicctx.c:picoquic_get_default_path_quality`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C reads default path quality from path[0]; Rust finds a path by id and subscribes to quality updates instead.
* C source: `picoquic/quicctx.c:2715-2719`
* C signature: `void picoquic_get_default_path_quality(picoquic_cnx_t *, picoquic_path_quality_t *)`
* Rust source: `rs/fq/src/lib.rs:2704-2732`
* Rust item: `default_path_quality`

### C body
```c
{
    picoquic_path_t* path_x = cnx->path[0];
    picoquic_get_path_quality_from_context(path_x, quality);
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
            path.subscribe_to_quality_update_per_path_context(pacing_rate_delta, rtt_delta);
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }
```

## `picoquic/quicctx.c:picoquic_get_logging_cnxid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns initial_cnxid; Rust returns start_time ticks.
* C source: `picoquic/quicctx.c:4489-4493`
* C signature: `picoquic_connection_id_t picoquic_get_logging_cnxid(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3070-3077`
* Rust item: `logging_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->initial_cnxid;
}
```

### Rust body
```rust
    pub fn start_time(&self) -> u64 {
        self.start_time.ticks()
    }
```

## `picoquic/quicctx.c:picoquic_get_remote_stream_error`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C finds a stream and returns its remote_error or 0; Rust body shown sets a stream remote_error and returns nothing.
* C source: `picoquic/quicctx.c:5520-5529`
* C signature: `uint64_t picoquic_get_remote_stream_error(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:4506-4522`
* Rust item: `remote_stream_error`

### C body
```c
{
    uint64_t remote_error = 0;
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    if ((stream = picoquic_find_stream(cnx, stream_id)) != NULL) {
        remote_error = stream->remote_error;
    }
    return remote_error;
}
```

### Rust body
```rust
    pub fn set_stream_remote_error(&mut self, stream_id: u64, error_code: u64) {
        if let Some(stream) = self.find_stream(stream_id)
            && let Some(stream) = self.streams.get_mut(stream)
        {
            stream.remote_error = error_code;
        }
    }
```

## `picoquic/quicctx.c:picoquic_is_0rtt_available`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C checks whether the 0-RTT crypto encrypt context exists; Rust checks whether backlog queues are empty.
* C source: `picoquic/quicctx.c:4513-4517`
* C signature: `int picoquic_is_0rtt_available(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3080-3088`
* Rust item: `is_0rtt_available`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return (cnx->crypto_context[picoquic_epoch_0rtt].aead_encrypt == NULL) ? 0 : 1;
}
```

### Rust body
```rust
    pub fn is_backlog_empty(&self) -> bool {
        self.nb_bytes_queued == 0 && self.misc_frames.is_empty() && self.output_streams.is_empty()
    }
```
