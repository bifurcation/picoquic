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

## `picoquic/tls_api.c:picoquic_verify_retry_token`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C decrypts and parses the token, validates time, packet number, reuse, and connection IDs; Rust body only rejects tokens longer than 128 bytes.
* C source: `picoquic/tls_api.c:2965-3029`
* C signature: `int picoquic_verify_retry_token(picoquic_quic_t *, const struct sockaddr *, uint64_t, int *, picoquic_connection_id_t *, const picoquic_connection_id_t *, uint32_t, const uint8_t *, size_t, int)`
* Rust source: `rs/fq/src/tls_api.rs:1879-1890`
* Rust item: `verify_retry_token`

### C body
```c
{
    int ret = 0;
    uint8_t text[128];
    size_t text_len = 0;
    picoquic_connection_id_t cid;
    uint64_t token_pn;

    odcid->id_len = 0;

    /* decode the encrypted token */
    if (token_size > sizeof(text)) {
        /* regular tokens produced by picoquic are always short, and a short decoding
        * buffer should be sufficient. If this text fires, it probably because of
        * an attack, or buggy code at the peer. */
        ret = -1;
    }
    else {
        ret = picoquic_server_decrypt_retry_token(quic, addr_peer, is_new_token, token, token_size,
            text, &text_len);
    }

    if (ret == 0) {
        /* Decode the clear text components */
        const uint8_t* bytes = text;
        const uint8_t* bytes_max = text + text_len;
        uint64_t token_time = PICOPARSE_64(text);

        if ((bytes = picoquic_frames_uint64_decode(bytes, bytes_max, &token_time)) != NULL &&
            (bytes = picoquic_frames_cid_decode(bytes, bytes_max, odcid)) != NULL &&
            (bytes = picoquic_frames_cid_decode(bytes, bytes_max, &cid)) != NULL &&
            (bytes = picoquic_frames_varint_decode(bytes, bytes_max, &token_pn)) != NULL) {
            if (token_time < current_time) {
                /* Invalid token, too old */
                ret = -1;
            }
            /* If the PN value is not yet decrypted, setting it to UINT32_MAX
             * bypasses the verification */
            else if (initial_pn != UINT32_MAX && odcid->id_len > 0 && token_pn >= initial_pn) {
                /* Invalid PN number */
                ret = -1;
            }
            else {
                /* Remove old tickets before testing this one. */
                picoquic_registered_token_clear(quic, current_time);
                if (check_reuse && (ret = picoquic_registered_token_check_reuse(quic, token, token_size, token_time)) != 0) {
                    picoquic_log_context_free_app_message(quic, rcid, "Duplicate token test returns %d", ret);
                }
                else if (odcid->id_len > 0 &&
                    picoquic_compare_connection_id(rcid, &cid) != 0) {
                    /* Invalid token, bad rcid */
                    ret = -1;
                }
            }
        }
        else {
            *odcid = picoquic_null_connection_id;
        }
    }

    return ret;
}
```

### Rust body
```rust
        if token.len() > 128 {
            return Err(Error::InvalidArgument);
        }
```

## `picoquic/transport.c:picoquic_transport_param_varint_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C reserves a length byte, encodes a varint, backfills the encoded length, and returns the advanced pointer; Rust body shown only checks for an empty buffer.
* C source: `picoquic/transport.c:43-57`
* C signature: `uint8_t * picoquic_transport_param_varint_encode(uint8_t *, const uint8_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:15004-15007`
* Rust item: `picoquic_transport_param_varint_encode`

### C body
```c
{
    if (bytes + 1 > bytes_max) {
        bytes = NULL;
    }
    else {
        uint8_t* byte_l = bytes++;
        bytes = picoquic_frames_varint_encode(bytes, bytes_max, n64);
        if (bytes != NULL) {
            *byte_l = (uint8_t)((bytes - byte_l) - 1);
        }
    }

    return bytes;
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## `picoquic/util.c:picoquic_connection_id_hash`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C folds bytes beyond the first 8 into the hash, while Rust only copies up to the first 8 bytes and ignores the rest.
* C source: `picoquic/util.c:367-386`
* C signature: `uint64_t picoquic_connection_id_hash(const picoquic_connection_id_t *, const uint8_t *)`
* Rust source: `rs/fq/src/lib.rs:393-405`
* Rust item: `hash_with_seed`

### C body
```c
{
    uint64_t val64 = 0;
    size_t i = 0;

    for (; i < cid->id_len && i < 8; i++) {
        val64 <<= 8;
        val64 += cid->id[i];
    }

    for (; i < cid->id_len; i++) {
        uint64_t top = val64 >> 56;
        val64 <<= 8;
        val64 += cid->id[i];
        val64 += top * 0x10001;
    }

    return val64;
}
```

### Rust body
```rust
    pub fn val64(&self) -> u64 {
        let bytes = self.as_bytes();
        let len = bytes.len().min(8);
        let mut buf = [0u8; 8];
        buf[..len].copy_from_slice(&bytes[..len]);
        u64::from_be_bytes(buf)
    }
```

## `picoquic/util.c:picoquic_frames_varint_skip`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C computes the varint length and skips that many bytes; Rust body only checks for empty input and returns None, with no skip logic shown.
* C source: `picoquic/util.c:795-804`
* C signature: `const uint8_t * picoquic_frames_varint_skip(const uint8_t *, const uint8_t *)`
* Rust source: `rs/fq/src/internal.rs:6438-6441`
* Rust item: `frames_varint_skip`

### C body
```c
{
    if (bytes < bytes_max) {
        uint8_t v_len = VARINT_LEN(bytes);
        return  picoquic_frames_fixed_skip(bytes, bytes_max, v_len);
    }
    else {
        return NULL;
    }
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## `picoquic/bbr.c:BBRAccessEcnPacketContext`
* Phase 4C status: `suspect`
* Phase 4C rationale: C tests non-default path by pointer comparison against connection path[0], while Rust tests unique_path_id != 0; body alone does not prove these identify the same path.
* C source: `picoquic/bbr.c:2244-2260`
* C signature: `picoquic_packet_context_t * BBRAccessEcnPacketContext(picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:2421-2434`
* Rust item: `access_ecn_packet_context`

### C body
```c
{
    /* TODO: ECN counts should be a function of path, not number space! */
    picoquic_packet_context_t* pkt_ctx = &path_x->cnx->pkt_ctx[picoquic_packet_context_application];

    if (path_x->cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }
    else if (path_x != path_x->cnx->path[0]) {
        /* When doing simple multipath, or when preparing transitions,
         * only consider the default path */
        pkt_ctx = NULL;
    }

    return pkt_ctx;
}
```

### Rust body
```rust
) -> Option<&'a PacketContextState> {
    if connection.is_multipath_enabled {
        Some(&path_x.pkt_ctx)
    } else if path_x.unique_path_id != 0 {
        // Non-default path in single-path mode: ECN counts are not
        // tracked per-path here; the caller should use the default path.
        None
    } else {
        Some(&connection.pkt_ctx[PacketContext::Application as usize])
    }
}
```
