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

## `picoquic/ech.c:picoquic_ech_configure_quic_ctx`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets client ciphers/kems and only initializes opener when private_key_file is non-null; Rust sets ech_client_enabled and errors if private_key_file is Some but config_file_name is None.
* C source: `picoquic/ech.c:333-366`
* C signature: `int picoquic_ech_configure_quic_ctx(picoquic_quic_t *, const char *, const char *)`
* Rust source: `rs/fq/src/ech.rs:941-957`
* Rust item: `picoquic_ech_configure_quic_ctx`

### C body
```c
{
    int ret = 0;
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;
    PICOQUIC_THREAD_CHECK(quic);

    picoquic_release_quic_ech_ctx(quic);
    ctx->ech.client.ciphers = picoquic_hpke_cipher_suites;
    ctx->ech.client.kems = picoquic_hpke_kems;
    if (private_key_file != NULL) {
        ech_opener_callback_t* ech_cb = NULL;
        if ((ret = ech_init_opener_callback(&ech_cb, private_key_file, config_file_name)) == 0) {
            ctx->ech.server.create_opener = &ech_cb->super;
            ctx->ech.server.retry_configs.base = ech_cb->config.base;
            ctx->ech.server.retry_configs.len = ech_cb->config.off;
        }
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    picoquic_release_quic_ech_ctx(quic);
    quic.ech_client_enabled = true;

    if let Some(private_key_file) = private_key_file {
        let config_file_name = config_file_name.ok_or(Error::InvalidArgument)?;
        let opener = ech_init_opener(private_key_file, config_file_name)?;
        quic.ech_server_retry_config = Some(opener.config.clone());
        quic.ech_opener = Some(opener);
    }

    Ok(())
}
```

## `picoquic/frames.c:picoquic_encode_time_stamp_length`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns 2 plus the encoded timestamp length, while Rust sums encoded frame-type length and encoded timestamp length.
* C source: `picoquic/frames.c:5733-5738`
* C signature: `size_t picoquic_encode_time_stamp_length(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:14111-14116`
* Rust item: `encode_time_stamp_length`

### C body
```c
{
    uint64_t time_stamp = (current_time - cnx->start_time) >> cnx->local_parameters.ack_delay_exponent;

    return (2 + picoquic_encode_varint_length(time_stamp));
}
```

### Rust body
```rust
    pub fn encode_time_stamp_length(&self, current_time: Instant) -> usize {
        let delta = current_time.ticks().saturating_sub(self.start_time.ticks());
        let time_stamp = delta >> self.local_parameters.ack_delay_exponent;
        encode_varint_length(crate::frames::FrameType::TimeStamp as u64)
            + encode_varint_length(time_stamp)
    }
```

## `picoquic/frames.c:picoquic_format_first_misc_or_dg_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets more_data when the frame does not fit and updates is_pure_ack on success; Rust returns None on no fit and does not visibly set more_data or update is_pure_ack.
* C source: `picoquic/frames.c:4827-4842`
* C signature: `uint8_t * picoquic_format_first_misc_or_dg_frame(uint8_t *, uint8_t *, int *, int *, picoquic_misc_frame_header_t *, picoquic_misc_frame_header_t **, picoquic_misc_frame_header_t **)`
* Rust source: `rs/fq/src/internal.rs:13752-13769`
* Rust item: `format_first_misc_or_dg_frame`

### C body
```c
{
    if (bytes + misc_frame->length > bytes_max) {
        *more_data = 1;
    } else {
        uint8_t* frame = ((uint8_t*)misc_frame) + sizeof(picoquic_misc_frame_header_t);
        memcpy(bytes, frame, misc_frame->length);
        bytes += misc_frame->length;
        *is_pure_ack &= misc_frame->is_pure_ack;
        picoquic_delete_misc_or_dg(first, last, misc_frame);
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    // Consume the head of the queue and copy its bytes into the output buffer.
    let frame = queue.pop_front()?;
    let len = frame.bytes.len();
    if bytes.len() >= len {
        bytes[..len].copy_from_slice(&frame.bytes);
        Some(&mut bytes[len..])
    } else {
        // Frame doesn't fit — put it back and return None.
        queue.push_front(frame);
        None
    }
}
```

## `picoquic/frames.c:picoquic_queue_data_repeat_node_value`
* Phase 4C status: `suspect`
* Phase 4C rationale: C always derives a packet pointer from the node by pointer arithmetic; Rust performs a tree lookup that can return None.
* C source: `picoquic/frames.c:2138-2141`
* C signature: `void * picoquic_queue_data_repeat_node_value(picosplay_node_t *)`
* Rust source: `rs/fq/src/internal.rs:5714-5716`
* Rust item: `queue_data_repeat_node_value`

### C body
```c
{
    return (void*)((char*)node - offsetof(struct st_picoquic_packet_t, queue_data_repeat_node));
}
```

### Rust body
```rust
    pub fn queue_data_repeat_node_value(&self, splay_tok: SplayToken) -> Option<PacketToken> {
        self.queue_data_repeat_tree.get(splay_tok).copied()
    }
```

## `picoquic/logger.c:textlog_negotiated_alpn`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks both a log file and still-logging state, flips is_local to a 0/1 remote flag, and passes alpn_count; Rust only checks for logger_ref and passes is_local, sni, alpn, and alpn_list directly.
* C source: `picoquic/logger.c:2328-2343`
* C signature: `void textlog_negotiated_alpn(picoquic_cnx_t *, int, const uint8_t *, size_t, const uint8_t *, size_t, const ptls_iovec_t *, size_t)`
* Rust source: `rs/fq/src/logger.rs:755-759`
* Rust item: `negotiated_alpn`

### C body
```c
{
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(sni);
    UNREFERENCED_PARAMETER(sni_len);
    UNREFERENCED_PARAMETER(alpn);
    UNREFERENCED_PARAMETER(alpn_len);
#endif
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        /* TODO: alpn */
        picoquic_textlog_negotiated_alpn(cnx->quic->F_log, cnx, 
            (is_local) ? 0 : 1, 1, alpn_list, alpn_count);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }
```
