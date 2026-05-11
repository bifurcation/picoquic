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

## `picoquic/loss_recovery.c:picoquic_copy_before_retransmit`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body contains all retransmission-copy logic inline; Rust only snapshots old_p and delegates to copy_before_retransmit_snapshot, so equivalence is not visible from this body alone.
* C source: `picoquic/loss_recovery.c:664-782`
* C signature: `int picoquic_copy_before_retransmit(picoquic_packet_t *, picoquic_cnx_t *, uint8_t *, size_t, int *, int *, int, size_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:10945-10968`
* Rust item: `copy_before_retransmit`

### C body
```c
{
    /* check if this is an ACK only packet */
    int ret = 0;
    int frame_is_pure_ack = 0;
    size_t frame_length = 0;
    size_t byte_index = 0; /* Used when parsing the old packet */

    if (old_p->is_mtu_probe) {
        if (old_p->send_path != NULL) {
            /* MTU probe was lost, presumably because of packet too big */
            old_p->send_path->mtu_probe_sent = 0;
            if (!force_queue || force_queue == 2) {
                old_p->send_path->send_mtu_max_tried = old_p->length + old_p->checksum_overhead;
            }
        }
        /* MTU probes should not be retransmitted */
        *packet_is_pure_ack = 1;
        *do_not_detect_spurious = 0;
    }
    else if (old_p->is_ack_trap) {
        *packet_is_pure_ack = 1;
        *do_not_detect_spurious = 1;
    }
    else if (old_p->is_multipath_probe) {
        *packet_is_pure_ack = 0;
        *do_not_detect_spurious = 1;
    }
    else if (old_p->was_preemptively_repeated) {
        *packet_is_pure_ack = 1;
        *do_not_detect_spurious = 1;
    }
    else {
        /* Copy the relevant bytes from one packet to the next */
        byte_index = old_p->offset;

        while (ret == 0 && byte_index < old_p->length) {
            ret = picoquic_skip_frame(&old_p->bytes[byte_index],
                old_p->length - byte_index, &frame_length, &frame_is_pure_ack);

            /* Check whether the data was already acked, which may happen in
            * case of spurious retransmissions */
            if (ret == 0 && frame_is_pure_ack == 0) {
                ret = picoquic_check_frame_needs_repeat(cnx, &old_p->bytes[byte_index],
                    frame_length, old_p->ptype, &frame_is_pure_ack, do_not_detect_spurious, NULL);
            }

            /* Keep track of datagram frames that are possibly lost */
            if (ret == 0 &&
                PICOQUIC_IN_RANGE(old_p->bytes[byte_index], picoquic_frame_type_datagram, picoquic_frame_type_datagram_l) &&
                cnx->callback_fn != NULL) {
                uint8_t frame_id;
                uint64_t content_length;
                uint8_t* content_bytes = &old_p->bytes[byte_index];

                /* Parse and skip type and length */
                content_bytes = picoquic_decode_datagram_frame_header(content_bytes, content_bytes + frame_length,
                    &frame_id, &content_length);
                if (content_bytes != NULL) {
                    ret = (cnx->callback_fn)(cnx, old_p->send_time, content_bytes, (size_t)content_length,
                        picoquic_callback_datagram_lost, cnx->callback_ctx, NULL);
                }
                picoquic_log_app_message(cnx, "Datagram lost, PN=%" PRIu64 ", Sent: %" PRIu64,
                    old_p->sequence_number, old_p->send_time);
            }

            /* Prepare retransmission if needed */
            if (ret == 0) {
                if (!frame_is_pure_ack) {
                    if (PICOQUIC_IN_RANGE(old_p->bytes[byte_index], picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max)) {
                        * add_to_data_repeat_queue = 1;
                    }
                    else {
                        if ((force_queue || frame_length > send_buffer_max_minus_checksum - *length)) {
                            ret = picoquic_queue_misc_frame(cnx, &old_p->bytes[byte_index], frame_length, 0,
                                old_p->pc);
                        }
                        else if (frame_length <= send_buffer_max_minus_checksum - *length) {
                            memcpy(&new_bytes[*length], &old_p->bytes[byte_index], frame_length);
                            *length += frame_length;
                        }
                        else {
                            uint64_t error_frame_type = 0;
                            (void)picoquic_varint_decode(&old_p->bytes[byte_index], frame_length, &error_frame_type);
                            picoquic_log_app_message(cnx, "Cannot copy frame 0x%" PRIu64 ", packet type = %d, force queue = %d, repeat buffer : %zu, previous length : %zu.",
                                error_frame_type, old_p->ptype, force_queue);
                            ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR,
                                error_frame_type, "Cannot copy frame for retransmit");
                        }
                    }
                    *packet_is_pure_ack = 0;
                }
                byte_index += frame_length;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let snapshot = PacketRetransmitSnapshot::from(&*old_p);
    copy_before_retransmit_snapshot(
        &snapshot,
        connection,
        new_bytes,
        send_buffer_max_minus_checksum,
        packet_is_pure_ack,
        do_not_detect_spurious,
        force_queue,
        length,
        add_to_data_repeat_queue,
    )
}
```

## `picoquic/pacing.c:picoquic_is_sending_authorized_by_pacing`
* Phase 4C status: `suspect`
* Phase 4C rationale: C passes quic->packet_train_mode and quic into the pacing authorization call; Rust passes constant false and None.
* C source: `picoquic/pacing.c:253-257`
* C signature: `int picoquic_is_sending_authorized_by_pacing(picoquic_cnx_t *, picoquic_path_t *, uint64_t, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:6232-6241`
* Rust item: `is_sending_authorized_by_pacing`

### C body
```c
{
    return picoquic_is_authorized_by_pacing(&path_x->pacing, current_time, next_time, cnx->quic->packet_train_mode,
        cnx->quic);
}
```

### Rust body
```rust
        if let Some(path) = self.paths.get_mut(path_idx) {
            path.pacing
                .is_authorized(current_time, next_time, false, None)
        } else {
```

## `picoquic/packet.c:picoquic_ignore_incoming_handshake`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust adds a ret = -1 guard when skip_frame returns frame_length == 0; the C body does not, so that visible edge case would change whether ack_needed is applied.
* C source: `picoquic/packet.c:1345-1387`
* C signature: `void picoquic_ignore_incoming_handshake(picoquic_cnx_t *, uint8_t *, picoquic_packet_header *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:7020-7058`
* Rust item: `ignore_incoming_handshake`

### C body
```c
{
    /* The data starts at ph->index, and its length
     * is ph->payload_length. */
    int ret = 0;
    size_t byte_index = 0;
    int ack_needed = 0;
    picoquic_packet_context_enum pc;

    if (ph->ptype == picoquic_packet_initial) {
        pc = picoquic_packet_context_initial;
    }
    else if (ph->ptype == picoquic_packet_handshake) {
        pc = picoquic_packet_context_handshake;
    }
    else {
        /* Not expected! */
        return;
    }

    bytes += ph->offset;

    while (ret == 0 && byte_index < ph->payload_length) {
        size_t frame_length = 0;
        int frame_is_pure_ack = 0;
        ret = picoquic_skip_frame(&bytes[byte_index],
            ph->payload_length - byte_index, &frame_length, &frame_is_pure_ack);
        byte_index += frame_length;
        if (frame_is_pure_ack == 0) {
            ack_needed = 1;
        }
    }

    /* If the packet contains ackable data, mark ack needed
     * in the relevant packet context */
    if (ret == 0 && ack_needed) {
        picoquic_set_ack_needed(cnx, current_time, pc, cnx->path[0], 0);
    }
}
```

### Rust body
```rust
    ) {
        let pc = match ph.packet_type {
            crate::internal::PacketType::Initial => PacketContext::Initial,
            crate::internal::PacketType::Handshake => PacketContext::Handshake,
            _ => return,
        };

        let payload = Self::packet_payload(bytes, ph);
        let mut byte_index = 0usize;
        let mut ret = 0i32;
        let mut ack_needed = false;

        while ret == 0 && byte_index < payload.len() {
            let mut frame_length = 0usize;
            let mut frame_is_pure_ack = 0i32;
            ret = crate::internal::skip_frame(
                &payload[byte_index..],
                payload.len() - byte_index,
                &mut frame_length,
                &mut frame_is_pure_ack,
            );
            byte_index = byte_index.saturating_add(frame_length);
            if frame_is_pure_ack == 0 {
                ack_needed = true;
            }
            if ret == 0 && frame_length == 0 {
                ret = -1;
            }
        }

        if ret == 0 && ack_needed {
            self.set_ack_needed_on_path(current_time, pc, 0, 0);
        }
    }
```

## `picoquic/packet.c:picoquic_screen_initial_packet`
* Phase 4C status: `suspect`
* Phase 4C rationale: C rejects has_reserved_bit_set before busy checks and passes AEAD/PN contexts into create_cnx_internal; Rust has no visible reserved-bit check and passes None for those contexts.
* C source: `picoquic/packet.c:85-207`
* C signature: `int picoquic_screen_initial_packet(picoquic_quic_t *, const uint8_t *, size_t, const struct sockaddr *, picoquic_packet_header *, uint64_t, picoquic_cnx_t **, int *, picoquic_stream_data_node_t *)`
* Rust source: `rs/fq/src/lib.rs:5680-5821`
* Rust item: `screen_initial_packet`

### C body
```c
{
    int ret = 0;
    void* aead_ctx = NULL;
    void* pn_dec_ctx = NULL;

    /* Create a connection context if the CI is acceptable */
    if (packet_length < PICOQUIC_ENFORCED_INITIAL_MTU) {
        /* Unexpected packet. Reject, drop and log. */
        ret = PICOQUIC_ERROR_INITIAL_TOO_SHORT;
    }
    else if (ph->dest_cnx_id.id_len < PICOQUIC_ENFORCED_INITIAL_CID_LENGTH) {
        /* Initial CID too short -- ignore the packet */
        ret = PICOQUIC_ERROR_INITIAL_CID_TOO_SHORT;
    }
    else if (ph->has_reserved_bit_set) {
        /* Cannot have reserved bit set before negotiation completes */
        ret = PICOQUIC_ERROR_PACKET_HEADER_PARSING;
    }
    else if (quic->enforce_client_only) {
        /* Cannot create a client connection if the context is client only */
        ret = PICOQUIC_ERROR_SERVER_BUSY;
    }
    else if (quic->server_busy ||
        quic->current_number_connections >= quic->tentative_max_number_connections) {
        /* Cannot create a client connection now, send immediate close. */
        ret = PICOQUIC_ERROR_SERVER_BUSY;
    }
    else {
        /* This code assumes that *pcnx is always null when screen initial is called. */
        /* Verify the AEAD checkum */

        if (picoquic_get_initial_aead_context(quic, ph->version_index, &ph->dest_cnx_id,
            0 /* is_client=0 */, 0 /* is_enc = 0 */, &aead_ctx, &pn_dec_ctx) == 0) {
            ret = picoquic_remove_header_protection_inner((uint8_t *)bytes, ph->offset + ph->payload_length,
                decrypted_data->data, ph, pn_dec_ctx, 0 /* is_loss_bit_enabled_incoming */, 0 /* sack_list_last*/);
            if (ret == 0) {
                size_t decrypted_length = picoquic_aead_decrypt_generic(decrypted_data->data + ph->offset,
                    bytes + ph->offset, ph->payload_length, ph->pn64, decrypted_data->data, ph->offset, 
                    aead_ctx);
                if (decrypted_length >= ph->payload_length) {
                    ret = PICOQUIC_ERROR_AEAD_CHECK;
                }
                else {
                    ph->payload_length = (uint16_t)decrypted_length;
                }
            }
        }
        else {
            ret = PICOQUIC_ERROR_MEMORY;
        }

        if (ret == 0) {
            int is_address_blocked = !quic->is_port_blocking_disabled && picoquic_check_addr_blocked(addr_from);
            int is_new_token = 0;
            int has_good_token = 0;
            int has_bad_token = 0;
            picoquic_connection_id_t original_cnxid = { 0 };
            if (ph->token_length > 0) {
                /* If a token is present, verify it. */
                if (picoquic_verify_retry_token(quic, addr_from, current_time,
                    &is_new_token, &original_cnxid, &ph->dest_cnx_id, (uint32_t)ph->pn64,
                    ph->token_bytes, ph->token_length, 1) == 0) {
                    has_good_token = 1;
                }
                else {
                    has_bad_token = 1;
                }
            }

            if (has_bad_token && !is_new_token) {
                /* sending a bad retry token is fatal, sending an old new token is not */
                ret = PICOQUIC_ERROR_INVALID_TOKEN;
            }
            else if (!has_good_token && (quic->force_check_token || quic->max_half_open_before_retry <= quic->current_number_half_open || is_address_blocked)) {
                /* tokens are required before accepting new connections, so ask to queue a retry packet. */
                ret = PICOQUIC_ERROR_RETRY_NEEDED;
            }
            else {
                /* All clear */
                /* Check: what do do with odcid? */
                *pcnx = picoquic_create_cnx_internal(quic, ph->dest_cnx_id, ph->srce_cnx_id, addr_from, current_time, ph->vn,
                    NULL, NULL, 0, aead_ctx, pn_dec_ctx);
                if (*pcnx == NULL) {
                    /* Could not allocate the context */
                    ret = PICOQUIC_ERROR_MEMORY;
                }
                else {
                    *new_ctx_created = 1;
                    if (has_good_token) {
                        (*pcnx)->initial_validated = 1;
                        (void)picoquic_parse_connection_id(original_cnxid.id, original_cnxid.id_len, &(*pcnx)->original_cnxid);
                    }
                    /* Zeroing the pointers aead_ctx and pn_dec_ctx because the underlying object is
                     * now owned by the connection. */
                    aead_ctx = NULL;
                    pn_dec_ctx = NULL;
                }
            }
        }
    }

    if (aead_ctx != NULL) {
        /* Free the AEAD CTX */
        picoquic_aead_free(aead_ctx);
    }

    if (pn_dec_ctx != NULL) {
        /* Free the PN encryption context */
        picoquic_cipher_free(pn_dec_ctx);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> ParsedSegment {
        if packet_length < crate::internal::ENFORCED_INITIAL_MTU {
            return ParsedSegment {
                ret: InternalError::InitialTooShort as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if ph.dest_connection_id.len() < crate::internal::ENFORCED_INITIAL_CID_LENGTH as usize {
            return ParsedSegment {
                ret: InternalError::InitialCidTooShort as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if self.enforce_client_only
            || self.server_busy
            || self.current_number_connections >= self.tentative_max_number_connections
        {
            return ParsedSegment {
                ret: InternalError::ServerBusy as i32,
                connection: None,
                new_context_created: false,
            };
        }

        let initial_context =
            match self.initial_aead_context(ph.version_index, &ph.dest_connection_id, false, false)
            {
                Ok(ctx) => ctx,
                Err(error) => {
                    return ParsedSegment {
                        ret: Self::parse_error_status(error),
                        connection: None,
                        new_context_created: false,
                    };
                }
            };
        let decrypt_ret = decrypt_packet_payload(
            raw_bytes,
            packet_length,
            ph,
            decrypted_data,
            initial_context.pn_enc_ctx.as_ref(),
            initial_context.aead_ctx.as_ref(),
            false,
            0,
        );
        if decrypt_ret != 0 {
            return ParsedSegment {
                ret: decrypt_ret,
                connection: None,
                new_context_created: false,
            };
        }

        let is_address_blocked = !self.is_port_blocking_disabled && check_addr_blocked(addr_from);
        let mut has_good_token = false;
        let mut has_bad_token = false;
        let mut verified_token = None;
        if !ph.token_bytes.is_empty() {
            let token_bytes = ph.token_bytes.clone();
            match self.verify_retry_token(
                addr_from,
                current_time,
                &ph.dest_connection_id,
                ph.packet_number_full as u32,
                &token_bytes,
                true,
            ) {
                Ok(token) => {
                    has_good_token = true;
                    verified_token = Some(token);
                }
                Err(_) => has_bad_token = true,
            }
        }

        if has_bad_token {
            return ParsedSegment {
                ret: InternalError::InvalidToken as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if !has_good_token
            && (self.force_check_token
                || self.max_half_open_before_retry <= self.current_number_half_open
                || is_address_blocked)
        {
            return ParsedSegment {
                ret: InternalError::RetryNeeded as i32,
                connection: None,
                new_context_created: false,
            };
        }

        let connection = match self.create_cnx_internal(
            ph.dest_connection_id,
            ph.src_connection_id,
            Some(addr_from),
            current_time,
            ph.version,
            None,
            None,
            false,
            None,
            None,
        ) {
            Ok(token) => token,
            Err(error) => {
                return ParsedSegment {
                    ret: Self::parse_error_status(error),
                    connection: None,
                    new_context_created: false,
                };
            }
        };

        if let Some(cnx) = self.connections.get_mut(connection) {
            cnx.version_index = ph.version_index;
            cnx.proposed_version = ph.version;
            if let Some(token) = verified_token {
                cnx.initial_validated = true;
                cnx.original_connection_id = token.odcid;
            }
        }

        ParsedSegment {
            ret: 0,
            connection: Some(connection),
            new_context_created: true,
        }
    }
```

## `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_generate_clear`
* Phase 4C status: `suspect`
* Phase 4C rationale: Bodies are similar, but C copies server_id while Rust copies server_id_encoded, which is a visible source-name mismatch.
* C source: `picoquic/picoquic_lb.c:54-59`
* C signature: `void picoquic_lb_compat_cid_generate_clear(picoquic_quic_t *, picoquic_load_balancer_cid_context_t *, picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/lb.rs:332-336`
* Rust item: `generate_clear`

### C body
```c
{
    picoquic_lb_compat_cid_generate_first_byte(quic, lb_ctx, cnx_id_returned);
    memcpy(cnx_id_returned->id + 1, lb_ctx->server_id, lb_ctx->server_id_length);
}
```

### Rust body
```rust
    fn generate_clear(&self, quic: &Quic, bytes: &mut [u8]) {
        self.set_first_byte(quic, bytes);
        bytes[1..1 + self.server_id_length]
            .copy_from_slice(&self.server_id_encoded[..self.server_id_length]);
    }
```
