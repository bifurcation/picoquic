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

## Pair `picoquic/quicctx.c:picoquic_set_default_idle_timeout`
C: `picoquic/quicctx.c:992-996 picoquic_set_default_idle_timeout`
Rust: `rs/fq/src/lib.rs:1744-1754 set_default_idle_timeout`

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

## Pair `picoquic/quicctx.c:picoquic_get_max_half_open_retry_threshold`
C: `picoquic/quicctx.c:1212-1216 picoquic_get_max_half_open_retry_threshold`
Rust: `rs/fq/src/lib.rs:1184-1192 max_half_open_retry_threshold`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->max_half_open_before_retry;
}
```

### Rust body
```rust
    pub fn set_port_blocking_disabled(&mut self, is_port_blocking_disabled: bool) {
        self.is_port_blocking_disabled = is_port_blocking_disabled;
    }
```
