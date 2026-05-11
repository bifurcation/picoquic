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

## `picoquic/quicctx.c:picoquic_process_version_upgrade`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust always updates version fields and returns 0, while C only succeeds for same version or supported upgrade and may return -1; C also conditionally frees/reinstalls initial keys.
* C source: `picoquic/quicctx.c:5572-5598`
* C signature: `int picoquic_process_version_upgrade(picoquic_cnx_t *, int, int)`
* Rust source: `rs/fq/src/internal.rs:15669-15700`
* Rust item: `process_version_upgrade`

### C body
```c
{
    int ret = -1;
    /* Check whether upgrade is supported */
    if (new_version_index == old_version_index) {
        /* not an upgrade, nothing to do. */
        ret = 0;
    } else if (picoquic_supported_versions[new_version_index].upgrade_from != NULL) {
        int i = 0;

        while (picoquic_supported_versions[new_version_index].upgrade_from[i] != 0) {
            if (picoquic_supported_versions[new_version_index].upgrade_from[i] ==
                picoquic_supported_versions[old_version_index].version) {
                /* Supported */
                ret = 0;
                if (cnx != NULL) {
                    /* Install the new keys */
                    cnx->version_index = new_version_index;
                    picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_initial]);
                    ret = picoquic_setup_initial_traffic_keys(cnx);
                    break;
                }
            }
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        self.rejected_version = if old_version_index >= 0 {
            self.proposed_version
        } else {
            self.rejected_version
        };
        self.version_index = new_version_index;
        let version = match new_version_index {
            0 => Version::V1,
            1 => Version::V2,
            2 => Version::V2Draft,
            3 => Version::PostIesg,
            4 => Version::TwentyFirstInterop,
            5 => Version::TwentiethInterop,
            6 => Version::TwentiethPreInterop,
            7 => Version::NineteenthInterop,
            8 => Version::NineteenthBisInterop,
            9 => Version::EighteenthInterop,
            10 => Version::SeventeenthInterop,
            11 => Version::InternalTest2,
            12 => Version::InternalTest1,
            _ => Version::V1,
        };
        self.proposed_version = version as u32;
        self.desired_version = self.proposed_version;
        self.local_parameters.version_negotiation.current = self.proposed_version;
        0
    }
```

## `picoquic/quicctx.c:picoquic_reorder_output_stream`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust removes and reinserts any output stream unconditionally, while C only does so when neighboring priority comparisons show the stream is out of order.
* C source: `picoquic/quicctx.c:3587-3604`
* C signature: `void picoquic_reorder_output_stream(picoquic_cnx_t *, picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/internal.rs:9615-9622`
* Rust item: `reorder_output_stream`

### C body
```c
{
    if (stream->is_output_stream) {
        if ((stream->previous_output_stream != NULL &&
            picoquic_compare_stream_priority(stream, stream->previous_output_stream) < 0) ||
            (stream->next_output_stream != NULL &&
                picoquic_compare_stream_priority(stream, stream->next_output_stream) > 0)) {
            picoquic_remove_output_stream(cnx, stream);
            stream->is_output_stream = 0;
            picoquic_insert_output_stream(cnx, stream);
        }
    }
}
```

### Rust body
```rust
    pub fn reorder_output_stream(&mut self, stream: &mut StreamHead) {
        if stream.is_output_stream {
            // Simple re-insert: remove then add back at the end.
            self.remove_output_stream(stream);
            stream.is_output_stream = false;
            self.insert_output_stream(stream);
        }
    }
```

## `picoquic/quicctx.c:picoquic_set_client_authentication`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets TLS client authentication; Rust sets use_exporter, an unrelated field.
* C source: `picoquic/quicctx.c:5543-5546`
* C signature: `void picoquic_set_client_authentication(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1654-1662`
* Rust item: `set_client_authentication`

### C body
```c
void picoquic_set_client_authentication(picoquic_quic_t* quic, int client_authentication) {
    PICOQUIC_THREAD_CHECK(quic);
    picoquic_tls_set_client_authentication(quic, client_authentication);
}
```

### Rust body
```rust
    pub fn set_use_exporter(&mut self, use_exporter: bool) {
        self.use_exporter = use_exporter;
    }
```

## `picoquic/quicctx.c:picoquic_set_default_idle_timeout`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets default_tp.max_idle_timeout from idle_timeout_ms; Rust body clones an entire TransportParameters into local_parameters and has no idle_timeout_ms assignment.
* C source: `picoquic/quicctx.c:992-996`
* C signature: `void picoquic_set_default_idle_timeout(picoquic_quic_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:1744-1754`
* Rust item: `set_default_idle_timeout`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_tp.max_idle_timeout = idle_timeout_ms;
}
```

### Rust body
```rust
    pub fn set_transport_parameters(&mut self, tp: &TransportParameters) {
        self.local_parameters = tp.clone();
    }
```

## `picoquic/quicctx.c:picoquic_set_fuzz`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets fuzz function and context; Rust body sets log level and propagates logging state to connections.
* C source: `picoquic/quicctx.c:4632-4637`
* C signature: `void picoquic_set_fuzz(picoquic_quic_t *, picoquic_fuzz_fn, void *)`
* Rust source: `rs/fq/src/lib.rs:1077-1088`
* Rust item: `set_fuzz`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->fuzz_fn = fuzz_fn;
    quic->fuzz_ctx = fuzz_ctx;
}
```

### Rust body
```rust
    pub fn set_log_level(&mut self, log_level: i32) {
        self.use_long_log = log_level != 0;
        for connection in self.connections.iter_mut() {
            connection.use_long_log = self.use_long_log;
        }
    }
```
