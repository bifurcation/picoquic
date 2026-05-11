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

## `picoquic/tls_api.c:picoquic_tlscontext_trim_after_handshake`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust clears tls_sendbuf, while C frees ext_data and alpn_vec fields.
* C source: `picoquic/tls_api.c:2118-2133`
* C signature: `void picoquic_tlscontext_trim_after_handshake(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1026-1029`
* Rust item: `trim_tls_context_after_handshake`

### C body
```c
{
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;

    if (ctx->ext_data != NULL) {
        free(ctx->ext_data);
        ctx->ext_data = NULL;
        ctx->ext_data_size = 0;
    }

    if (ctx->alpn_vec != NULL) {
        free(ctx->alpn_vec);
        ctx->alpn_vec = NULL;
        ctx->alpn_vec_size = 0;
    }
}
```

### Rust body
```rust
    pub fn trim_tls_context_after_handshake(&mut self) {
        self.tls_sendbuf.clear();
        self.tls_sendbuf.shrink_to_fit();
    }
```

## `picoquic/transport.c:picoquic_transport_param_type_varint_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C encodes both tp_type and n64 and returns the advanced pointer; Rust snippet only encodes tp_type and shows no visible encoding of n64 or final return.
* C source: `picoquic/transport.c:59-66`
* C signature: `uint8_t * picoquic_transport_param_type_varint_encode(uint8_t *, const uint8_t *, picoquic_tp_enum, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:15040-15048`
* Rust item: `picoquic_transport_param_type_varint_encode`

### C body
```c
{
    if (bytes != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, tp_type)) != NULL) {
        bytes = picoquic_transport_param_varint_encode(bytes, bytes_max, n64);
    }
    return bytes;
}
```

### Rust body
```rust
    if !encode_varint_at(bytes, &mut offset, tp_type as u64) {
        return None;
    }
```

## `picoquic/util.c:picoquic_addr_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns address structure lengths by family, while Rust body is store_addr and only copies an optional SocketAddr.
* C source: `picoquic/util.c:541-550`
* C signature: `int picoquic_addr_length(const struct sockaddr *)`
* Rust source: `rs/fq/src/utils.rs:518-534`
* Rust item: `addr_length`

### C body
```c
{
    int len = 0;
    if (addr->sa_family == AF_INET) {
        len = (int)sizeof(struct sockaddr_in);
    } else if (addr->sa_family == AF_INET6) {
        len = (int)sizeof(struct sockaddr_in6);
    }
    return len;
}
```

### Rust body
```rust
pub fn store_addr(addr: Option<&SocketAddr>) -> Option<SocketAddr> {
    addr.copied()
}
```

## `picoquic/util.c:picoquic_frames_varint_encode_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C computes the encoded length from a numeric value, while Rust computes a decoded length from the top bits of a byte.
* C source: `picoquic/util.c:911-929`
* C signature: `size_t picoquic_frames_varint_encode_length(uint64_t)`
* Rust source: `rs/fq/src/internal.rs:6475-6483`
* Rust item: `frames_varint_encode_length`

### C body
```c
{
    size_t len = 8;

    if (n64 < 16384) {
        if (n64 < 64) {
            len = 1;
        }
        else {
            len = 2;
        }
    }
    else if (n64 < 1073741824) {
        len = 4;
    }

    return len;
}
```

### Rust body
```rust
pub fn decode_varint_length(byte: u8) -> usize {
    1usize << ((byte & 0xC0) >> 6)
}
```

## `picoquic/util.c:picoquic_string_duplicate`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C duplicates a string when non-null; Rust body sets up a performance log file and context.
* C source: `picoquic/util.c:73-84`
* C signature: `char * picoquic_string_duplicate(const char *)`
* Rust source: `rs/fq/src/performance_log.rs:349-363`
* Rust item: `perflog_setup`

### C body
```c
{
    char* str = NULL;

    if (original != NULL) {
        size_t len = strlen(original);

        str = picoquic_string_create(original, len);
    }

    return str;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let path = perflog_file_name.as_ref().to_path_buf();
        if file_is_empty(&path) {
            file_set_header(&path);
        }
        let ctx = PerflogCtx {
            items: Vec::new(),
            perflog_file_name: path,
        };
        self.perflog_fn = Some(Box::new(ctx));
        Ok(())
    }
```
