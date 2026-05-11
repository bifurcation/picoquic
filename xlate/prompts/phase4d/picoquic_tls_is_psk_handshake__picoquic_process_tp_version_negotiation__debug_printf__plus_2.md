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

## `picoquic/tls_api.c:picoquic_tls_is_psk_handshake`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns whether the TLS handshake is PSK; Rust body returns a peer socket address.
* C source: `picoquic/tls_api.c:2153-2158`
* C signature: `int picoquic_tls_is_psk_handshake(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2891-2904`
* Rust item: `tls_is_psk_handshake`

### C body
```c
{
    /* int ret = cnx->is_psk_handshake; */
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return ptls_is_psk_handshake(((picoquic_tls_ctx_t*)(cnx->tls_ctx))->tls);
}
```

### Rust body
```rust
    pub fn peer_addr(&self) -> SocketAddr {
        self.paths
            .first()
            .and_then(|p| p.tuples.first())
            .map(|t| t.peer_addr)
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap())
    }
```

## `picoquic/transport.c:picoquic_process_tp_version_negotiation`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C client-mode scans proposed versions after the current version, while Rust client-mode sets negotiated_vn to current and returns immediately; Rust scans candidates in the opposite extension_mode branch.
* C source: `picoquic/transport.c:226-281`
* C signature: `const uint8_t * picoquic_process_tp_version_negotiation(const uint8_t *, const uint8_t *, int, uint32_t, uint32_t *, int *, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:15270-15306`
* Rust item: `process_tp_version_negotiation`

### C body
```c
{
    uint32_t current;

    *negotiated_vn = 0;
    *negotiated_index = -1;
    *vn_error = 0;

    if ((bytes = picoquic_frames_uint32_decode(bytes, bytes_max, &current)) == NULL) {
        *vn_error = PICOQUIC_TRANSPORT_PARAMETER_ERROR;
    } else {
        if (current != envelop_vn) {
            /* Packet was tempered with */
            *vn_error = PICOQUIC_TRANSPORT_VERSION_NEGOTIATION_ERROR;
            bytes = NULL;
        }
        else if (extension_mode == 0) {
            /* Processing the client extensions */
            while (bytes < bytes_max) {
                uint32_t proposed;
                if ((bytes = picoquic_frames_uint32_decode(bytes, bytes_max, &proposed)) == NULL) {
                    /* Decoding error */
                    *vn_error = PICOQUIC_TRANSPORT_PARAMETER_ERROR;
                    break;
                }
                else {
                    /* Select the first version proposed by the client that is locally supported,
                     * and is deemed compatible with the current version
                     */
                    int this_rank = picoquic_get_version_index(proposed);
                    if (this_rank >= 0) {
                        *negotiated_vn = proposed;
                        *negotiated_index = this_rank;
                        break;
                    }
                }
            }
        }
        else {
            /* Processing the server extensions */
            /* TODO: Check whether the chosen version corresponds to something the client wanted */
            /* TODO: Check whether the chosen version is officially supported, could be reused in 0-RTT */
            while (bytes < bytes_max) {
                uint32_t proposed;
                if ((bytes = picoquic_frames_uint32_decode(bytes, bytes_max, &proposed)) == NULL) {
                    /* Decoding error */
                    *vn_error = PICOQUIC_TRANSPORT_PARAMETER_ERROR;
                    break;
                }
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    *negotiated_vn = 0;
    *negotiated_index = -1;
    *vn_error = 0;
    if bytes.len() < 4 || !bytes.len().is_multiple_of(4) {
        *vn_error = 0x7;
        return None;
    }
    let current = u32::from_be_bytes(bytes[0..4].try_into().ok()?);
    if current != envelop_vn {
        *vn_error = 0xA;
        return None;
    }
    if extension_mode == 0 {
        *negotiated_vn = current;
        *negotiated_index = Version::try_from_wire(current).map(|_| 0).unwrap_or(-1);
        return Some(&bytes[bytes.len()..]);
    }
    for (idx, chunk) in bytes[4..].chunks_exact(4).enumerate() {
        let candidate = u32::from_be_bytes(chunk.try_into().ok()?);
        if Version::try_from_wire(candidate).is_some() {
            *negotiated_vn = candidate;
            *negotiated_index = idx as i32;
            return Some(&bytes[bytes.len()..]);
        }
    }
    *negotiated_vn = current;
    *negotiated_index = Version::try_from_wire(current).map(|_| 0).unwrap_or(-1);
    Some(&bytes[bytes.len()..])
}
```

## `picoquic/util.c:debug_printf`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C conditionally formats and writes debug output; Rust body only returns early when debug is suspended.
* C source: `picoquic/util.c:129-162`
* C signature: `void debug_printf(const char *)`
* Rust source: `rs/fq/src/utils.rs:150-153`
* Rust item: `debug_printf`

### C body
```c
{
#if 1
    if (debug_suspended == 0 && debug_out != NULL) {
#else
    if (debug_suspended == 0 && (debug_out != NULL || debug_callback != NULL)) {
#endif
        if (debug_out) {
            va_list args;
            va_start(args, fmt);
            vfprintf(debug_out, fmt, args);
            va_end(args);
#if 1
        }
#else
        } else {
            char message[1024];
            size_t message_length;
            va_list args;
            va_start(args, fmt);
            vsnprintf(message, sizeof(message), fmt, args);
            va_end(args);
            message_length = strnlen(message, sizeof(message));
            if (message_length > 0) {
                // Strip any trailing newline
                if (message[message_length - 1] == '\n') {
                    message[message_length - 1] = '\0';
                }
            }
            debug_callback(message, debug_callback_argp);
        }
#endif
    }
}
```

### Rust body
```rust
    if DEBUG_SUSPENDED.with(|s| s.get()) {
        return;
    }
```

## `picoquic/util.c:picoquic_frames_uint64_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C writes eight big-endian bytes and advances; Rust body only checks length and has no encoding logic shown.
* C source: `picoquic/util.c:1041-1058`
* C signature: `uint8_t * picoquic_frames_uint64_encode(uint8_t *, const uint8_t *, uint64_t)`
* Rust source: `rs/fq/src/utils.rs:902-905`
* Rust item: `frames_uint64_encode`

### C body
```c
{
    if (bytes + sizeof(n) > bytes_max) {
        bytes = NULL;
    }
    else {
        *bytes++ = (uint8_t)(n >> 56);
        *bytes++ = (uint8_t)(n >> 48);
        *bytes++ = (uint8_t)(n >> 40);
        *bytes++ = (uint8_t)(n >> 32);
        *bytes++ = (uint8_t)(n >> 24);
        *bytes++ = (uint8_t)(n >> 16);
        *bytes++ = (uint8_t)(n >> 8);
        *bytes++ = (uint8_t)n;
    }
    return (bytes);

}
```

### Rust body
```rust
    if bytes.len() < 8 {
        return None;
    }
```

## `picoquic/util.c:picoquic_is_connection_id_null`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns whether id_len is zero; Rust body hashes self bytes with a seed and returns u64.
* C source: `picoquic/util.c:348-351`
* C signature: `int picoquic_is_connection_id_null(const picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/lib.rs:388-395`
* Rust item: `is_empty`

### C body
```c
{
    return (cnx_id->id_len == 0) ? 1 : 0;
}
```

### Rust body
```rust
    pub fn hash_with_seed(&self, seed: &[u8; 16]) -> u64 {
        crate::siphash::siphash(self.as_bytes(), seed)
    }
```
