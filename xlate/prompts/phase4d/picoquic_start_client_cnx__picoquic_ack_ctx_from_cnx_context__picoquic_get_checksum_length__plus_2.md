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

## `picoquic/quicctx.c:picoquic_start_client_cnx`
* Phase 4C status: `suspect`
* Phase 4C rationale: C rejects repeated starts, logs, initializes TLS, applies remote/local transport parameters, reinserts by wake time, and returns that result; Rust only sets up initial traffic keys and changes state.
* C source: `picoquic/quicctx.c:4384-4413`
* C signature: `int picoquic_start_client_cnx(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2334-2338`
* Rust item: `start_client`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx->cnx_state != picoquic_state_client_init ||
        cnx->tls_stream[0].sent_offset > 0 ||
        cnx->tls_stream[0].send_queue != NULL) {
        DBG_PRINTF("%s", "picoquic_start_client_cnx called twice.");
        return -1;
    }

    picoquic_log_new_connection(cnx);
        
    ret = picoquic_initialize_tls_stream(cnx, picoquic_get_quic_time(cnx->quic));
    /* A remote session ticket may have been loaded as part of initializing TLS,
     * and remote parameters may have been initialized to the initial value
     * of the previous session. Apply these new parameters. */
    cnx->maxdata_remote = cnx->remote_parameters.initial_max_data;
    cnx->max_stream_id_bidir_remote =
        STREAM_ID_FROM_RANK(cnx->remote_parameters.initial_max_stream_id_bidir, cnx->client_mode, 0);
    cnx->max_stream_id_unidir_remote = 
        STREAM_ID_FROM_RANK(cnx->remote_parameters.initial_max_stream_id_unidir, cnx->client_mode, 1);
    cnx->max_stream_data_remote = cnx->remote_parameters.initial_max_data;
    cnx->max_stream_data_local = cnx->local_parameters.initial_max_stream_data_bidi_local;

    picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));

    return ret;
}
```

### Rust body
```rust
    pub fn start_client(&mut self) -> Result<(), Error> {
        self.setup_initial_traffic_keys()?;
        self.connection_state = State::ClientInitSent;
        Ok(())
    }
```

## `picoquic/sacks.c:picoquic_ack_ctx_from_cnx_context`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust body only shows the multipath/application branch and no visible fallback return of the connection ack context shown in C.
* C source: `picoquic/sacks.c:128-147`
* C signature: `picoquic_ack_context_t * picoquic_ack_ctx_from_cnx_context(picoquic_cnx_t *, picoquic_packet_context_enum, picoquic_local_cnxid_t *)`
* Rust source: `rs/fq/src/internal.rs:8434-8450`
* Rust item: `ack_ctx_from_cnx_context`

### C body
```c
{
    picoquic_ack_context_t* ack_ctx = &cnx->ack_ctx[pc];

    if (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) {
        int path_id = 0;
        if (l_cid != NULL) {
            path_id = picoquic_find_path_by_unique_id(cnx, l_cid->path_id);
        }

        if (path_id >= 0) {
            ack_ctx = &cnx->path[path_id]->ack_ctx;
        }
    }
    return ack_ctx;

}
```

### Rust body
```rust
        if self.is_multipath_enabled && packet_context == PacketContext::Application {
            let path_id = local_connection_id
                .and_then(|tok| self.local_connection_ids.get(tok))
                .map(|l| l.path_id)
                .unwrap_or(0);
            let path_idx = self.paths.iter().position(|p| p.unique_path_id == path_id);
            if let Some(idx) = path_idx {
                return Some(&mut self.paths[idx].ack_ctx);
            }
        }
```

## `picoquic/sender.c:picoquic_get_checksum_length`
* Phase 4C status: `suspect`
* Phase 4C rationale: C queries the AEAD checksum length when present and defaults to 16 otherwise; Rust always returns 16.
* C source: `picoquic/sender.c:857-872`
* C signature: `size_t picoquic_get_checksum_length(picoquic_cnx_t *, picoquic_epoch_enum)`
* Rust source: `rs/fq/src/internal.rs:6976-6982`
* Rust item: `get_checksum_length`

### C body
```c
{
    size_t ret = 16;

    if (cnx->crypto_context[epoch].aead_encrypt != NULL) {
        ret = picoquic_aead_get_checksum_length(cnx->crypto_context[epoch].aead_encrypt);
    }
    else {
        DBG_PRINTF("Try getting checksum for empty context, epoch %d", epoch);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn get_checksum_length(&self, is_cleartext_mode: Epoch) -> usize {
        // C: picoquic_get_checksum_length — returns the AEAD tag length.
        // Cleartext (initial epoch) uses 16 bytes (AES-128-GCM tag).
        // All other epochs also use 16 bytes in practice.
        let _ = is_cleartext_mode;
        16
    }
```

## `picoquic/sender.c:picoquic_prepare_mtu_probe`
* Phase 4C status: `suspect`
* Phase 4C rationale: C writes PING then memset padding and returns probe_length - checksum_length without guard checks; Rust adds bounds guards and may return a shorter value for small lengths.
* C source: `picoquic/sender.c:1630-1647`
* C signature: `size_t picoquic_prepare_mtu_probe(picoquic_cnx_t *, picoquic_path_t *, size_t, size_t, uint8_t *, size_t)`
* Rust source: `rs/fq/src/internal.rs:17177-17198`
* Rust item: `prepare_mtu_probe`

### C body
```c
{
    size_t probe_length = picoquic_next_mtu_probe_length(cnx, path_x);
    size_t length = header_length;

    if (probe_length > bytes_max) {
        probe_length = bytes_max;
    }

    bytes[length++] = picoquic_frame_type_ping;
    memset(&bytes[length], 0, probe_length - checksum_length - length);

    return probe_length - checksum_length;
}
```

### Rust body
```rust
    ) -> usize {
        let mut probe_length = self.next_mtu_probe_length(path_x, self.quic_mtu_max());
        probe_length = probe_length.min(bytes_max).min(bytes.len());
        if probe_length <= checksum_length
            || header_length >= probe_length.saturating_sub(checksum_length)
        {
            return header_length.min(probe_length.saturating_sub(checksum_length));
        }
        let payload_end = probe_length - checksum_length;
        bytes[header_length] = crate::frames::FrameType::Ping as u8;
        if header_length + 1 < payload_end {
            bytes[header_length + 1..payload_end].fill(crate::frames::FrameType::Padding as u8);
        }
        payload_end
    }
```

## `picoquic/sender.c:picoquic_protect_packet`
* Phase 4C status: `suspect`
* Phase 4C rationale: C creates the header, pads for PN sample, handles loss bit, fuzzing, multipath AEAD, logging, and PN offset from header creation; Rust encrypts an existing header/payload with simplified offsets and lacks several visible steps.
* C source: `picoquic/sender.c:897-994`
* C signature: `size_t picoquic_protect_packet(picoquic_cnx_t *, picoquic_packet_type_enum, uint8_t *, uint64_t, size_t, size_t, uint8_t *, size_t, void *, void *, picoquic_path_t *, picoquic_tuple_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:7009-7059`
* Rust item: `protect_packet`

### C body
```c
{
    size_t send_length;
    size_t h_length;
    size_t pn_offset = 0;
    size_t pn_length = 0;
    size_t aead_checksum_length = picoquic_aead_get_checksum_length(aead_context);
    size_t pn_iv_size = picoquic_pn_iv_size(pn_enc);
    size_t pn_sample_start;
    size_t pn_sample_end;
    uint8_t first_mask = 0x0F;

    if (tuple == NULL) {
        tuple = path_x->first_tuple;
    }

    /* Create the packet header just before encrypting the content */
    h_length = picoquic_create_packet_header(cnx, ptype,
        sequence_number, path_x, tuple, header_length, send_buffer, &pn_offset, &pn_length);

    if (h_length != header_length) {
#ifdef HUNTING_FOR_BUFFER_OVERFLOW
        char* x = NULL;
        *x++;
#endif
        picoquic_log_app_message(cnx, "BUFFER OVERFLOW? Packet header prediction fails, %zu instead of %zu\n", h_length, header_length);
    }

    // https://datatracker.ietf.org/doc/html/rfc9001#section-5.4.2
    // ensure there are enough iv bytes for pn encryption
    pn_sample_start = pn_offset + 4;
    pn_sample_end = pn_sample_start + pn_iv_size;
    length = picoquic_pad_to_target_length(bytes, length, pn_sample_end - aead_checksum_length); // discount aead checksum length added later

    if (ptype == picoquic_packet_1rtt_protected) {
        if (cnx->is_loss_bit_enabled_outgoing) {
            first_mask = 0x07;
            path_x->q_square++;
            if ((path_x->q_square & PICOQUIC_LOSS_BIT_Q_HALF_PERIOD) != 0) {
                send_buffer[0] |= 0x10;
            }
            if (path_x->nb_losses_found > path_x->nb_losses_reported) {
                send_buffer[0] |= 0x08;
                path_x->nb_losses_reported++;
            }
        }
        else {
            first_mask = 0x1F;
        }
    }

    /* Make sure that the payload length is encoded in the header */
    /* Using encryption, the "payload" length also includes the encrypted packet length */
    picoquic_update_payload_length(send_buffer, pn_offset, h_length - pn_length, length + aead_checksum_length);

    /* If fuzzing is required, apply it */
    if (cnx->quic->fuzz_fn != NULL) {
        if (h_length == header_length) {
            memcpy(bytes, send_buffer, header_length);
        }
        length = cnx->quic->fuzz_fn(cnx->quic->fuzz_ctx, cnx, bytes,
            send_buffer_max - aead_checksum_length, length, header_length);
        if (h_length == header_length) {
            memcpy(send_buffer, bytes, header_length);
        }
    }

    /* Encrypt the packet */
    if (cnx->is_multipath_enabled && ptype == picoquic_packet_1rtt_protected) {
        send_length = picoquic_aead_encrypt_mp(send_buffer + /* header_length */ h_length,
            bytes + header_length, length - header_length, path_x->unique_path_id,
            sequence_number, send_buffer, /* header_length */ h_length, aead_context);
    }
    else {
        send_length = picoquic_aead_encrypt_generic(send_buffer + /* header_length */ h_length,
            bytes + header_length, length - header_length,
            sequence_number, send_buffer, /* header_length */ h_length, aead_context);
    }

    send_length += /* header_length */ h_length;

    /* if needed, log the segment before header protection is applied */
    picoquic_log_outgoing_packet(cnx, path_x,
        bytes, sequence_number, pn_length, length,
        send_buffer, send_length, current_time);

    /* Next, encrypt the PN -- The sample is located after the pn_offset */
    picoquic_protect_packet_header(send_buffer, pn_offset, first_mask, pn_enc);

    return send_length;
}
```

### Rust body
```rust
    ) -> usize {
        if header_length > length || length > bytes.len() || header_length > send_buffer_max {
            return 0;
        }
        let header = &bytes[..header_length];
        let mut payload = bytes[header_length..length].to_vec();
        aead_context.encrypt(sequence_number, header, &mut payload);
        let packet_length = header_length + payload.len();
        if packet_length > send_buffer_max || packet_length > send_buffer.len() {
            return 0;
        }
        send_buffer[..header_length].copy_from_slice(header);
        send_buffer[header_length..packet_length].copy_from_slice(&payload);
        update_payload_length(
            send_buffer,
            header_length.saturating_sub(4),
            header_length.saturating_sub(4),
            packet_length,
        );
        let first_mask = if (send_buffer[0] & 0x80) != 0 {
            0x0f
        } else {
            0x1f
        };
        let pn_offset = if header_length >= 4 {
            header_length - 4
        } else {
            header_length
        };
        protect_packet_header(
            &mut send_buffer[..packet_length],
            pn_offset,
            first_mask,
            pn_enc,
        );
        packet_length
    }
```
