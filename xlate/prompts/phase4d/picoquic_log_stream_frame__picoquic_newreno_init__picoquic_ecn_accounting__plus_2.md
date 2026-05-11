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

## `picoquic/logwriter.c:picoquic_log_stream_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: For length-bearing stream frames, C logs through bytes plus extra payload after the parsed length, while Rust computes copy_end from the pre-length head_len and may omit the encoded length bytes.
* C source: `picoquic/logwriter.c:75-136`
* C signature: `const uint8_t * picoquic_log_stream_frame(FILE *, const uint8_t *, const uint8_t *)`
* Rust source: `rs/fq/src/binlog.rs:377-422`
* Rust item: `log_stream_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    uint8_t ftype = bytes[0];
    size_t length = 0;
    uint8_t log_buffer[256];
    int has_length = 0;
    size_t extra_bytes = 8;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1); /* type */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* stream */

    if ((ftype & 4) != 0) {
        bytes = picoquic_log_varint_skip(bytes, bytes_max); /* offset */
    }

    if (bytes != NULL) {
        if ((ftype & 2) != 0) {
            bytes = picoquic_log_length(bytes, bytes_max, &length); /* length */
            has_length = 1;
        }
        else {
            length = bytes_max - bytes;
        }
    }

    if (bytes != NULL) {
        if (length < extra_bytes) {
            /* Add up to 8 bytes of content that can be documented in the qlog */
            extra_bytes = length;
        }
        if (has_length) {
            picoquic_binlog_frame(f, bytes_begin, bytes + extra_bytes);
        }
        else {
            uint8_t* log_next = log_buffer;
            size_t l_head = bytes - bytes_begin;

            memcpy(log_buffer, bytes_begin, l_head);
            log_next += l_head;
            if ((log_next = picoquic_frames_varint_encode(log_next, log_buffer + 256, length)) != NULL) {
                memcpy(log_next, bytes, extra_bytes);
                log_next += extra_bytes;
                picoquic_binlog_frame(f, log_buffer, log_next);
            }
            else {
                picoquic_binlog_frame(f, log_buffer, log_buffer + l_head);
            }
        }

        bytes = picoquic_log_fixed_skip(bytes, bytes_max, length);
    }
    else {
        /* Cautiously log the beginning of the erroneous frame */
        length = bytes_max - bytes_begin;
        if (length > 26) {
            length = 26;
        }
        picoquic_binlog_frame(f, bytes_begin, bytes_begin + length);
    }
    return bytes;
}
```

### Rust body
```rust
fn log_stream_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    if bytes_in.is_empty() {
        return None;
    }
    let ftype = bytes_in[0];
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    if (ftype & 4) != 0 {
        bytes = frames_varint_skip(bytes)?;
    }

    let head_len = bytes_begin.len() - bytes.len();
    let has_length = (ftype & 2) != 0;
    let length: usize;
    if has_length {
        let (l, rest) = read_length(bytes)?;
        length = l;
        bytes = rest;
    } else {
        length = bytes.len();
    }

    let mut extra_bytes: usize = 8;
    if length < extra_bytes {
        extra_bytes = length;
    }

    if has_length {
        let copy_end = head_len.saturating_add(extra_bytes).min(bytes_begin.len());
        append_frame(out, &bytes_begin[..copy_end]);
    } else {
        let mut log_buffer = Vec::with_capacity(head_len + 8 + extra_bytes);
        log_buffer.extend_from_slice(&bytes_begin[..head_len]);
        let mut len_buf = [0u8; 8];
        let n = varint_encode(&mut len_buf, length as u64);
        log_buffer.extend_from_slice(&len_buf[..n]);
        let payload_start = head_len;
        let avail = bytes_begin.len().saturating_sub(payload_start);
        let take = extra_bytes.min(avail);
        log_buffer.extend_from_slice(&bytes_begin[payload_start..payload_start + take]);
        append_frame(out, &log_buffer);
    }

    skip_fixed(bytes, length)
}
```

## `picoquic/newreno.c:picoquic_newreno_init`
* Phase 4C status: `suspect`
* Phase 4C rationale: The C body allocates a fresh state and sets congestion_alg_state to NULL on allocation failure; the Rust body reuses or defaults state and always stores Some state, with no visible failure path.
* C source: `picoquic/newreno.c:189-205`
* C signature: `void picoquic_newreno_init(picoquic_path_t *, const char *, uint64_t)`
* Rust source: `rs/fq/src/newreno.rs:60-69`
* Rust item: `picoquic_newreno_init`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    picoquic_newreno_state_t* nr_state = (picoquic_newreno_state_t*)malloc(sizeof(picoquic_newreno_state_t));
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(current_time);
    UNREFERENCED_PARAMETER(option_string);
#endif

    if (nr_state != NULL) {
        picoquic_newreno_reset(nr_state, path_x);
        path_x->congestion_alg_state = nr_state;
    }
    else {
        path_x->congestion_alg_state = NULL;
    }
}
```

### Rust body
```rust
fn picoquic_newreno_init(path_x: &mut Path, _option_string: Option<&str>, _current_time: Instant) {
    let mut state = path_x
        .congestion_alg_state
        .take()
        .and_then(|boxed| boxed.downcast::<NewrenoState>().ok())
        .map(|boxed| *boxed)
        .unwrap_or_default();
    picoquic_newreno_reset(&mut state, path_x);
    path_x.congestion_alg_state = Some(Box::new(state));
}
```

## `picoquic/packet.c:picoquic_ecn_accounting`
* Phase 4C status: `suspect`
* Phase 4C rationale: C starts with ack_ctx = &cnx->ack_ctx[pc] and only switches lookup for application multipath; Rust only updates when ack_ctx_from_cnx_context returns Some, with no visible default ack context path.
* C source: `picoquic/packet.c:1880-1908`
* C signature: `void picoquic_ecn_accounting(picoquic_cnx_t *, unsigned char, picoquic_packet_context_enum, picoquic_local_cnxid_t *)`
* Rust source: `rs/fq/src/lib.rs:5534-5557`
* Rust item: `ecn_accounting`

### C body
```c
{
    picoquic_ack_context_t* ack_ctx = &cnx->ack_ctx[pc];
    
    if (pc == picoquic_packet_context_application && cnx->is_multipath_enabled) {
        ack_ctx = picoquic_ack_ctx_from_cnx_context(cnx, pc, l_cid);
    }

    switch (received_ecn & 0x03) {
    case 0x00:
        break;
    case PICOQUIC_ECN_ECT_1: /* ECN_ECT_1 */
        ack_ctx->ecn_ect1_total_local++;
        ack_ctx->sending_ecn_ack |= 1;
        break;
    case PICOQUIC_ECN_ECT_0: /* ECN_ECT_0 */
        ack_ctx->ecn_ect0_total_local++;
        ack_ctx->sending_ecn_ack |= 1;
        break;
    case PICOQUIC_ECN_CE: /* ECN_CE */
        ack_ctx->ecn_ce_total_local++;
        ack_ctx->sending_ecn_ack |= 1;
        break;
    }
}
```

### Rust body
```rust
        if let Some(ack_ctx) = self.ack_ctx_from_cnx_context(packet_context, local_connection_id) {
            match received_ecn & 0x03 {
                0x00 => {}
                ECN_ECT_1 => {
                    ack_ctx.ecn_ect1_total_local = ack_ctx.ecn_ect1_total_local.saturating_add(1);
                    ack_ctx.sending_ecn_ack = true;
                }
                ECN_ECT_0 => {
                    ack_ctx.ecn_ect0_total_local = ack_ctx.ecn_ect0_total_local.saturating_add(1);
                    ack_ctx.sending_ecn_ack = true;
                }
                ECN_CE => {
                    ack_ctx.ecn_ce_total_local = ack_ctx.ecn_ce_total_local.saturating_add(1);
                    ack_ctx.sending_ecn_ack = true;
                }
                _ => {}
            }
        }
```

## `picoquic/packet.c:picoquic_remove_header_protection`
* Phase 4C status: `suspect`
* Phase 4C rationale: C passes pn_dec directly to the inner helper; Rust returns an AeadNotReady error and mutates packet-number fields if pn_dec is absent, which is extra body-visible behavior.
* C source: `picoquic/packet.c:617-631`
* C signature: `int picoquic_remove_header_protection(picoquic_cnx_t *, uint8_t *, uint8_t *, picoquic_packet_header *)`
* Rust source: `rs/fq/src/internal.rs:7135-7162`
* Rust item: `picoquic_remove_header_protection`

### C body
```c
{
    int ret = 0;
    size_t length = ph->offset + ph->payload_length; /* this may change after decrypting the PN */
    void * pn_enc = cnx->crypto_context[ph->epoch].pn_dec;

    picoquic_sack_list_t* sack_list = picoquic_sack_list_from_cnx_context(cnx, ph->pc, ph->l_cid);
    ret = picoquic_remove_header_protection_inner(bytes, length, decrypted_bytes, ph,
        pn_enc, cnx->is_loss_bit_enabled_incoming, picoquic_sack_list_last(sack_list));

    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let length = ph.offset.saturating_add(ph.payload_length);
    let epoch = ph.epoch as usize;
    let Some(pn_dec) = connection.crypto_context[epoch].pn_dec.as_deref() else {
        ph.packet_number_truncated = 0xffff_ffff;
        ph.packet_number_mask = 0xffff_ffff_0000_0000;
        ph.offset = ph.packet_number_offset;
        ph.packet_number_full = u64::MAX;
        return crate::errors::InternalError::AeadNotReady as i32;
    };
    let sack_list_last = connection.ack_ctx[ph.packet_context as usize]
        .sack_list
        .first();
    remove_header_protection_inner(
        bytes,
        length,
        decrypted_bytes,
        ph,
        pn_dec,
        connection.is_loss_bit_enabled_incoming,
        sack_list_last,
    )
}
```

## `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_config_free`
* Phase 4C status: `suspect`
* Phase 4C rationale: C clears only when the callback function matches the LB generator and context is non-null, while Rust clears based only on context type.
* C source: `picoquic/picoquic_lb.c:497-515`
* C signature: `void picoquic_lb_compat_cid_config_free(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lb.rs:618-627`
* Rust item: `clear_lb_cid_config`

### C body
```c
{
    if (quic->cnx_id_callback_fn == picoquic_lb_compat_cid_generate &&
        quic->cnx_id_callback_ctx != NULL) {
        picoquic_load_balancer_cid_context_t* lb_ctx = (picoquic_load_balancer_cid_context_t*)quic->cnx_id_callback_ctx;
        /* Release the encryption contexts so as to avoid memory leaks */
        if (lb_ctx->cid_encryption_context != NULL) {
            picoquic_aes128_ecb_free(lb_ctx->cid_encryption_context);
        }
        if (lb_ctx->cid_decryption_context != NULL) {
            picoquic_aes128_ecb_free(lb_ctx->cid_decryption_context);
        }
        /* Free the data */
        free(lb_ctx);
        /* Reset the Quic context */
        quic->cnx_id_callback_fn = NULL;
        quic->cnx_id_callback_ctx = NULL;
    }
}
```

### Rust body
```rust
    pub fn clear_lb_cid_config(&mut self) {
        let is_lb = self
            .connection_id_callback_ctx
            .as_ref()
            .is_some_and(|c| c.is::<ConnectionIdContext>());
        if is_lb {
            self.connection_id_callback_ctx = None;
            self.connection_id_callback_fn = None;
        }
    }
```
