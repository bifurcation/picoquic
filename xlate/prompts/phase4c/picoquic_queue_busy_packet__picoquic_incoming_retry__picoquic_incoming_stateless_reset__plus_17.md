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

## Pair `picoquic/packet.c:picoquic_queue_busy_packet`
C: `picoquic/packet.c:1240-1315 picoquic_queue_busy_packet`
Rust: `rs/fq/src/lib.rs:6069-6078 queue_busy_packet`

### C body
```c
{
    int ret = 0;
    picoquic_connection_id_t s_cid = { 0 };
    picoquic_stateless_packet_t* sp = picoquic_create_stateless_packet(quic);
    void* aead_ctx = NULL;
    void* pn_enc_ctx = NULL;

    if (sp != NULL) {
        uint8_t* bytes = sp->bytes;
        size_t byte_index = 0;
        size_t header_length = 0;
        size_t pn_offset;
        size_t pn_length;
        /* Payload is the encoding of the simples connection close frame */
        uint8_t payload[4] = { picoquic_frame_type_connection_close, PICOQUIC_TRANSPORT_SERVER_BUSY, 0, 0 };
        size_t payload_length = 0;

        picoquic_create_local_cnx_id(quic, &s_cid, ph->dest_cnx_id);


        /* Prepare long header:  Initial */
        byte_index = header_length = picoquic_create_long_header(
            picoquic_packet_initial,
            &ph->srce_cnx_id,
            &s_cid,
            0 /* No grease bit here */,
            ph->vn,
            ph->version_index,
            0, /* Sequence number 0 by default. */
            0,
            NULL,
            bytes,
            &pn_offset,
            &pn_length);

        /* Apply AEAD */
        if (picoquic_get_initial_aead_context(quic, ph->version_index, &ph->dest_cnx_id,
            0 /* is_client=0 */, 1 /* is_enc = 1 */, &aead_ctx, &pn_enc_ctx) == 0) {
            /* Make sure that the payload length is encoded in the header */
            /* Using encryption, the "payload" length also includes the encrypted packet length */
            picoquic_update_payload_length(bytes, pn_offset, header_length - pn_length,
                header_length + sizeof(payload) + picoquic_aead_get_checksum_length(aead_ctx));
            /* Encrypt packet payload */
            payload_length = picoquic_aead_encrypt_generic(bytes + header_length,
                payload, sizeof(payload), 0, bytes, header_length, aead_ctx);
            /* protect the PN */
            picoquic_protect_packet_header(bytes, pn_offset, 0x0F, pn_enc_ctx);
            /* Fill up control fields */
            sp->length = byte_index + payload_length;
            sp->ptype = picoquic_packet_initial;
            picoquic_store_addr(&sp->addr_to, addr_from);
            picoquic_store_addr(&sp->addr_local, addr_to);
            sp->if_index_local = if_index_to;
            sp->cnxid_log64 = picoquic_val64_connection_id(ph->dest_cnx_id);
            /* Queue packet */
            picoquic_queue_stateless_packet(quic, sp);
        }

        if (aead_ctx != NULL) {
            /* Free the AEAD CTX */
            picoquic_aead_free(aead_ctx);
        }

        if (pn_enc_ctx != NULL) {
            /* Free the PN encryption context */
            picoquic_cipher_free(pn_enc_ctx);
        }
    }
    return ret;
}
```

### Rust body
```rust
        let Ok(mut sp) = self.create_stateless_packet() else {
            return InternalError::Memory as i32;
        };
```

## Pair `picoquic/packet.c:picoquic_incoming_retry`
C: `picoquic/packet.c:1521-1613 picoquic_incoming_retry`
Rust: `rs/fq/src/lib.rs:3289-3338 incoming_packet_ex`

### C body
```c
{
    int ret = 0;
    size_t token_length = 0;
    uint8_t * token = NULL;

    if ((cnx->cnx_state != picoquic_state_client_init_sent && cnx->cnx_state != picoquic_state_client_init_resent) ||
        cnx->original_cnxid.id_len != 0) {
        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
    } else {
        /* Verify that the header is a proper echo of what was sent */
        if (ph->vn != picoquic_supported_versions[cnx->version_index].version) {
            /* Packet that do not match the "echo" checks should be logged and ignored */
            ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
        } else if (ph->pn64 != 0) {
            /* after draft-12, PN is required to be 0 */
            ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
        }
    }

    if (ret == 0) {
        /* Parse the retry frame */
        void * integrity_aead = picoquic_find_retry_protection_context(cnx->quic, cnx->version_index, 0);
        size_t byte_index = ph->offset;
        size_t data_length = ph->offset + ph->payload_length;

        /* Assume that is aead context is null, this is the old format and the 
         * integrity shall be verifed by checking the ODCID */
        if (integrity_aead == NULL) {
            uint8_t odcil = bytes[byte_index++];

            if (odcil != cnx->initial_cnxid.id_len || (size_t)odcil + 1u > ph->payload_length ||
                memcmp(cnx->initial_cnxid.id, &bytes[byte_index], odcil) != 0) {
                /* malformed ODCIL, or does not match initial cid; ignore */
                ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
                picoquic_log_app_message(cnx, "Retry packet rejected: odcid check failed");
            }
            else {
                byte_index += odcil;
            }
        }
        else {
            ret = picoquic_verify_retry_protection(integrity_aead, bytes, &data_length, byte_index, &cnx->initial_cnxid);

            if (ret != 0) {
                picoquic_log_app_message(cnx, "Retry packet rejected: integrity check failed, ret=0x%x", ret);
            }
        }

        if (ret == 0) {
            token_length = data_length - byte_index;

            if (token_length > 0) {
                token = malloc(token_length);
                if (token == NULL) {
                    ret = PICOQUIC_ERROR_MEMORY;
                }
                else {
                    memcpy(token, &bytes[byte_index], token_length);
                }
            }
        }
    }

    if (ret == 0) {
        /* Close the log, because it is keyed by initial_cnxid */
        picoquic_log_close_connection(cnx);
        /* if this is the first reset, reset the original cid */
        if (cnx->original_cnxid.id_len == 0) {
            cnx->original_cnxid = cnx->initial_cnxid;
        }
        /* reset the initial CNX_ID to the version sent by the server */
        cnx->initial_cnxid = ph->srce_cnx_id;

        /* keep a copy of the retry token */
        if (cnx->retry_token != NULL) {
            free(cnx->retry_token);
        }
        cnx->retry_token = token;
        cnx->retry_token_length = (uint16_t)token_length;

        picoquic_reset_cnx(cnx, current_time);

        /* Mark the packet as not required for ack */
        ret = PICOQUIC_ERROR_RETRY;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<Option<&mut Connection>, Error> {
        let packet_length = bytes.len();
        let mut consumed_index = 0usize;
        let mut previous_dest_id = ConnectionId::default();
        let mut first_cnx = None;

        while consumed_index < packet_length {
            let mut consumed = 0usize;
            let ret = self.incoming_segment(
                &mut bytes[consumed_index..],
                packet_length - consumed_index,
                packet_length,
                &mut consumed,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
                current_time,
                current_time,
                &mut previous_dest_id,
                &mut first_cnx,
            );

            if ret == 0 {
                consumed_index = consumed_index.saturating_add(consumed);
                if consumed == 0 {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(token) = first_cnx
            && let Some(cnx) = self.connections.get_mut(token)
            && packet_length > cnx.max_mtu_received
        {
            cnx.max_mtu_received = packet_length;
        }

        Ok(first_cnx.and_then(|token| self.connections.get_mut(token)))
    }
```

## Pair `picoquic/packet.c:picoquic_incoming_stateless_reset`
C: `picoquic/packet.c:1813-1829 picoquic_incoming_stateless_reset`
Rust: `rs/fq/src/lib.rs:3289-3338 incoming_packet_ex`

### C body
```c
{
    /* Stateless reset. The connection should be abandonned */
    if (cnx->cnx_state <= picoquic_state_ready) {
        cnx->remote_error = PICOQUIC_ERROR_STATELESS_RESET;
    }
    if (cnx->callback_fn) {
        (void)(cnx->callback_fn)(cnx, 0, NULL, 0, picoquic_callback_stateless_reset, cnx->callback_ctx, NULL);
    }
    picoquic_connection_disconnect(cnx);

    return PICOQUIC_ERROR_AEAD_CHECK;
}
```

### Rust body
```rust
    ) -> Result<Option<&mut Connection>, Error> {
        let packet_length = bytes.len();
        let mut consumed_index = 0usize;
        let mut previous_dest_id = ConnectionId::default();
        let mut first_cnx = None;

        while consumed_index < packet_length {
            let mut consumed = 0usize;
            let ret = self.incoming_segment(
                &mut bytes[consumed_index..],
                packet_length - consumed_index,
                packet_length,
                &mut consumed,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
                current_time,
                current_time,
                &mut previous_dest_id,
                &mut first_cnx,
            );

            if ret == 0 {
                consumed_index = consumed_index.saturating_add(consumed);
                if consumed == 0 {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(token) = first_cnx
            && let Some(cnx) = self.connections.get_mut(token)
            && packet_length > cnx.max_mtu_received
        {
            cnx.max_mtu_received = packet_length;
        }

        Ok(first_cnx.and_then(|token| self.connections.get_mut(token)))
    }
```

## Pair `picoquic/packet.c:picoquic_incoming_not_decrypted`
C: `picoquic/packet.c:2019-2067 picoquic_incoming_not_decrypted`
Rust: `rs/fq/src/lib.rs:3289-3338 incoming_packet_ex`

### C body
```c
{
    int buffered = 0;

    if (cnx->cnx_state < picoquic_state_ready) {
        if (cnx->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len > 0 &&
            picoquic_compare_connection_id(&cnx->path[0]->first_tuple->p_local_cnxid->cnx_id, &ph->dest_cnx_id) == 0)
        {
            /* verifying the destination cnx id is a strong hint that the peer is responding.
            * Setting epoch parameter = -1 guarantees the hint is only used if the RTT is not
            * yet known.
            */
            picoquic_update_path_rtt(cnx, cnx->path[0], -1, cnx->start_time, current_time, 0, 0);

            if (length <= PICOQUIC_MAX_PACKET_SIZE &&
                ((ph->ptype == picoquic_packet_handshake && cnx->client_mode) || ph->ptype == picoquic_packet_1rtt_protected)) {
                /* stash a copy of the incoming message for processing once the keys are available */
                picoquic_stateless_packet_t* packet = picoquic_create_stateless_packet(cnx->quic);

                if (packet != NULL) {
                    packet->length = length;
                    packet->ptype = ph->ptype;
                    memcpy(packet->bytes, bytes, length);
                    packet->next_packet = cnx->first_sooner;
                    cnx->first_sooner = packet;
                    picoquic_store_addr(&packet->addr_local, addr_to);
                    picoquic_store_addr(&packet->addr_to, addr_from);
                    packet->if_index_local = if_index_to;
                    packet->received_ecn = received_ecn;
                    packet->receive_time = current_time;
                    buffered = 1;
                }
            }
        }
    }

    return buffered;
}
```

### Rust body
```rust
    ) -> Result<Option<&mut Connection>, Error> {
        let packet_length = bytes.len();
        let mut consumed_index = 0usize;
        let mut previous_dest_id = ConnectionId::default();
        let mut first_cnx = None;

        while consumed_index < packet_length {
            let mut consumed = 0usize;
            let ret = self.incoming_segment(
                &mut bytes[consumed_index..],
                packet_length - consumed_index,
                packet_length,
                &mut consumed,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
                current_time,
                current_time,
                &mut previous_dest_id,
                &mut first_cnx,
            );

            if ret == 0 {
                consumed_index = consumed_index.saturating_add(consumed);
                if consumed == 0 {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(token) = first_cnx
            && let Some(cnx) = self.connections.get_mut(token)
            && packet_length > cnx.max_mtu_received
        {
            cnx.max_mtu_received = packet_length;
        }

        Ok(first_cnx.and_then(|token| self.connections.get_mut(token)))
    }
```

## Pair `picoquic/packet.c:picoquic_process_sooner_packets`
C: `picoquic/packet.c:2449-2516 picoquic_process_sooner_packets`
Rust: `rs/fq/src/internal.rs:14976-14978 process_sooner_packets`

### C body
```c
{
    picoquic_stateless_packet_t* packet = cnx->first_sooner;
    picoquic_stateless_packet_t* previous = NULL;

    cnx->recycle_sooner_needed = 0;

    while (packet != NULL) {
        picoquic_stateless_packet_t* next_packet = packet->next_packet;
        int could_try_now = 1;
        picoquic_epoch_enum epoch = 0;
        switch (packet->ptype) {
        case picoquic_packet_handshake:
            epoch = picoquic_epoch_handshake;
            break;
        case picoquic_packet_1rtt_protected:
            epoch = picoquic_epoch_1rtt;
            break;
        default:
            could_try_now = 0;
            break;
        }

        if (could_try_now &&
            (cnx->crypto_context[epoch].aead_decrypt != NULL || cnx->crypto_context[epoch].pn_dec != NULL))
        {
            size_t consumed_index = 0;
            int ret = 0;
            picoquic_connection_id_t previous_destid = picoquic_null_connection_id;
            picoquic_cnx_t* first_cnx = NULL;


            while (consumed_index < packet->length) {
                size_t consumed = 0;

                ret = picoquic_incoming_segment(cnx->quic, packet->bytes + consumed_index,
                    packet->length - consumed_index, packet->length,
                    &consumed, (struct sockaddr*) & packet->addr_to, (struct sockaddr*) & packet->addr_local, packet->if_index_local,
                    packet->received_ecn, current_time, packet->receive_time, &previous_destid, &first_cnx);

                if (ret == 0 && consumed > 0) {
                    consumed_index += consumed;
                }
                else {
                    break;
                }
            }

            if (ret != 0) {
                DBG_PRINTF("Processing sooner packet type %d returns %d (0x%d)", (int)packet->ptype, ret, ret);
            }

            if (previous == NULL) {
                cnx->first_sooner = packet->next_packet;
            }
            else {
                previous->next_packet = packet->next_packet;
            }
            picoquic_delete_stateless_packet(packet);
        }
        else {
            previous = packet;
        }

        packet = next_packet;
    }
}
```

### Rust body
```rust
    pub fn process_sooner_packets(&mut self, _current_time: Instant) {
        self.sooner_stateless.clear();
    }
```

## Pair `picoquic/paths.c:picoquic_prepare_path_control_packet`
C: `picoquic/paths.c:150-232 picoquic_prepare_path_control_packet`
Rust: `rs/fq/src/internal.rs:4424-4459 prepare_path_control_packet`

### C body
```c
{
    int ret = 0;
    picoquic_packet_type_enum packet_type = picoquic_packet_1rtt_protected;
    int is_pure_ack = 1;
    size_t header_length = 0;
    size_t length = 0;
    size_t checksum_overhead = picoquic_get_checksum_length(cnx, picoquic_epoch_1rtt);
    size_t send_buffer_min_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;
    uint8_t* bytes = packet->bytes;
    uint8_t* bytes_max = bytes + send_buffer_min_max - checksum_overhead;
    uint8_t* bytes_next;
    int more_data = 0;
    int is_challenge_padding_needed = 0;
    picoquic_packet_context_t* pkt_ctx = (cnx->is_multipath_enabled) ?
        &path_x->pkt_ctx :
        &cnx->pkt_ctx[picoquic_packet_context_application];

    /* TODO: will use the local CID specified for the path. There should be a distinct
    * CID specified for the tuple.
     */
    packet->pc = picoquic_packet_context_application;

    length = picoquic_predict_packet_header_length(
        cnx, packet_type, pkt_ctx);
    packet->ptype = packet_type;
    packet->offset = length;
    header_length = length;
    packet->sequence_number = pkt_ctx->send_sequence;
    packet->send_time = current_time;
    packet->send_path = path_x;
    bytes_next = bytes + length;

    /* If required, prepare challenge and response frames.
     * These frames will be sent immediately, regardless of pacing or flow control.
     */
    bytes_next = picoquic_prepare_tuple_challenge_frames(cnx, path_x, tuple,
        bytes_next, bytes_max, &more_data, &is_pure_ack, &is_challenge_padding_needed,
        current_time, next_wake_time);

    /* Compute the length before pacing block */
    length = bytes_next - bytes;

    if (cnx->is_address_discovery_provider) {
        /* If a new address was learned, prepare an observed address frame */
        /* TODO: tie this code to processing of paths */
        bytes_next = picoquic_prepare_observed_address_frame(bytes_next, bytes_max,
            path_x, tuple, current_time, next_wake_time, &more_data, &is_pure_ack);
    }

    if (ret == 0 && length > header_length) {
        /* Ensure that all packets are properly padded before being sent. */

        if (is_challenge_padding_needed && length < PICOQUIC_ENFORCED_INITIAL_MTU) {
            length = picoquic_pad_to_target_length(bytes, length, (uint32_t)(send_buffer_min_max - checksum_overhead));
        }
        else {
            length = picoquic_pad_to_policy(cnx, bytes, length, (uint32_t)(send_buffer_min_max - checksum_overhead));
        }
    }
    else {
        length = 0;
    }
    packet->length = length;
    picoquic_finalize_and_protect_packet_tuple(cnx, packet,
        ret, length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_min_max,
        path_x, current_time, tuple);

    if (*send_length > 0) {
        *next_wake_time = current_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);

        if (ret == 0 && picoquic_cnx_is_still_logging(cnx)) {
            picoquic_log_cc_dump(cnx, current_time);
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<usize, crate::Error> {
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let mut padding_needed = 0;
        let total = send_buffer.len();
        let tail = self
            .prepare_path_challenge_frames(
                path_x,
                send_buffer,
                &mut more_data,
                &mut is_pure_ack,
                &mut padding_needed,
                current_time,
                next_wake_time,
            )
            .ok_or(crate::Error::BufferTooSmall)?;
        let mut written = total - tail.len();
        if padding_needed != 0 && written < MIN_SEGMENT_SIZE && written < total {
            let target = MIN_SEGMENT_SIZE.min(total);
            send_buffer[written..target].fill(0);
            written = target;
        }
        packet.length = written;
        packet.packet_context = PacketContext::Application;
        packet.packet_type = PacketType::OneRttProtected;
        packet.is_ack_eliciting = is_pure_ack == 0;
        Ok(written)
    }
```

## Pair `picoquic/paths.c:picoquic_verify_path_available`
C: `picoquic/paths.c:323-373 picoquic_verify_path_available`
Rust: `rs/fq/src/internal.rs:16487-16543 verify_path_available`

### C body
```c
{
    int backup_index = -1;
    int nb_available = 0;
    uint64_t best_available_retransmit = UINT64_MAX;
    uint64_t best_backup_retransmit = UINT64_MAX;

    *min_retransmit = 0;

    for (int path_index = 0; path_index < cnx->nb_paths; path_index++) {
        picoquic_path_t* path_x = cnx->path[path_index];
        if (path_x->first_tuple->challenge_verified &&
            !path_x->path_is_demoted) {
            /* Set the congestion algorithm if not already done */
            if (cnx->congestion_alg != NULL && path_x->congestion_alg_state == NULL) {
                cnx->congestion_alg->alg_init(path_x, cnx->congestion_alg_option_string, current_time);
            }
            /* track the available paths */
            if (path_x->path_is_backup) {
                if (backup_index < 0 || path_x->nb_retransmit < best_backup_retransmit) {
                    best_backup_retransmit = path_x->nb_retransmit;
                    backup_index = path_index;
                }
            }
            else
            {
                if (path_x->nb_retransmit < best_available_retransmit) {
                    best_available_retransmit = path_x->nb_retransmit;
                    *next_path = path_x;
                    nb_available = 0;
                }
                nb_available++;
            }
        }
    }
    if (best_available_retransmit > 0 && best_backup_retransmit < best_available_retransmit) {
        cnx->path[backup_index]->path_is_backup = 0;
        *next_path = cnx->path[backup_index];
        nb_available = 1;
        *min_retransmit = best_backup_retransmit;
        /* TODO: some logging. Queue PATH_AVAILABLE frame? */
    }
    else
    {
        *min_retransmit = best_available_retransmit;
    }
    return nb_available;
}
```

### Rust body
```rust
    pub fn verify_path_available(&mut self, current_time: Instant) -> (i32, Option<usize>, u64) {
        let mut backup_index: Option<usize> = None;
        let mut nb_available: i32 = 0;
        let mut best_available_retransmit = u64::MAX;
        let mut best_backup_retransmit = u64::MAX;
        let mut next_path_idx: Option<usize> = None;

        // Extract algorithm and option string before the mutable path loop to
        // avoid overlapping borrows across struct fields.
        let alg = self.congestion_alg;
        let opt_owned = self.congestion_alg_option_string.clone();
        let opt = opt_owned.as_deref();

        for i in 0..self.paths.len() {
            let challenge_verified = self.paths[i]
                .tuples
                .first()
                .is_some_and(|t| t.challenge_verified);
            if !challenge_verified || self.paths[i].path_is_demoted {
                continue;
            }
            if let Some(a) = alg
                && self.paths[i].congestion_alg_state.is_none()
            {
                a.algorithm.alg_init(&mut self.paths[i], opt, current_time);
            }
            let retransmit = self.paths[i].nb_retransmit;
            if self.paths[i].path_is_backup {
                if backup_index.is_none() || retransmit < best_backup_retransmit {
                    best_backup_retransmit = retransmit;
                    backup_index = Some(i);
                }
            } else {
                if retransmit < best_available_retransmit {
                    best_available_retransmit = retransmit;
                    next_path_idx = Some(i);
                    nb_available = 0;
                }
                nb_available += 1;
            }
        }

        let min_retransmit = if best_available_retransmit > 0
            && best_backup_retransmit < best_available_retransmit
        {
            if let Some(bi) = backup_index {
                self.paths[bi].path_is_backup = false;
                next_path_idx = Some(bi);
                nb_available = 1;
            }
            best_backup_retransmit
        } else {
            best_available_retransmit
        };

        (nb_available, next_path_idx, min_retransmit)
    }
```

## Pair `picoquic/performance_log.c:picoquic_perflog_save`
C: `picoquic/performance_log.c:77-125 picoquic_perflog_save`
Rust: `rs/fq/src/performance_log.rs:171-208 save`

### C body
```c
{
    int ret = 0;
    FILE* F = picoquic_file_open(perflog_ctx->perflog_file_name, "a");

    if (F == NULL) {
        ret = -1;
    }
    else {
        while (perflog_ctx->first != NULL) {
            char cnxid_str[513];
            picoquic_performance_log_item_t* perflog_item = perflog_ctx->first;
            perflog_ctx->first = perflog_item->next;
            if (perflog_ctx->first == NULL) {
                perflog_ctx->last = NULL;
            }
            /* Print version identifiers */
            fprintf(F, "%d, %s, ", PICOQUIC_PER_LOG_VERSION, PICOQUIC_VERSION);
            /* Print the key performance data */
            fprintf(F, "%f, %" PRIu64 ", %" PRIu64 ", %f, %f",
                perflog_item->duration_sec,
                perflog_item->data_sent,
                perflog_item->data_received,
                perflog_item->send_mbps,
                perflog_item->recv_mbps);
            /* TODO: nb streams.
            printf("Nb_transactions: %" PRIu64"\n", quicperf_ctx->nb_streams);
            printf("TPS: %f\n", ((double)quicperf_ctx->nb_streams) / duration_sec);
            */
            /* Print identification data */
            if (picoquic_print_connection_id_hexa(cnxid_str, sizeof(cnxid_str), &perflog_item->cnxid) != 0) {
                cnxid_str[0] = 0;
            }
            fprintf(F, ", 0x%x, %s, 0x%s, %" PRIu64,
                perflog_item->quic_version,
                (perflog_item->alpn == NULL) ? "" : perflog_item->alpn,
                cnxid_str, perflog_item->cnx_time_64);

            /* Print the additional values */
            for (size_t i = 0; i < perflog_item->nb_values; i++) {
                fprintf(F, ", %"PRIu64, perflog_item->v[i]);
            }
            fprintf(F, "\n");
            picoquic_perflog_item_free(perflog_item);
        }
        (void)picoquic_file_close(F);
    }
    return ret;
}
```

### Rust body
```rust
    fn save(&mut self) -> Result<(), Error> {
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.perflog_file_name)
            .map_err(|_| Error::Generic)?;
        for item in self.items.drain(..) {
            let mut cnxid_str = String::new();
            if print_connection_id_hexa(&mut cnxid_str, &item.cnxid).is_err() {
                cnxid_str.clear();
            }
            write!(f, "{}, {}, ", PER_LOG_VERSION, crate::VERSION).map_err(|_| Error::Generic)?;
            write!(
                f,
                "{:.6}, {}, {}, {:.6}, {:.6}",
                item.duration_sec,
                item.data_sent,
                item.data_received,
                item.send_mbps,
                item.recv_mbps,
            )
            .map_err(|_| Error::Generic)?;
            write!(
                f,
                ", 0x{:x}, {}, 0x{}, {}",
                item.quic_version,
                item.alpn.as_deref().unwrap_or(""),
                cnxid_str,
                item.cnx_time_64,
            )
            .map_err(|_| Error::Generic)?;
            for v in item.v.iter() {
                write!(f, ", {}", v).map_err(|_| Error::Generic)?;
            }
            writeln!(f).map_err(|_| Error::Generic)?;
        }
        Ok(())
    }
```

## Pair `picoquic/performance_log.c:picoquic_perflog_file_is_empty`
C: `picoquic/performance_log.c:279-296 picoquic_perflog_file_is_empty`
Rust: `rs/fq/src/performance_log.rs:305-309 file_is_empty`

### C body
```c
{
    int is_empty = 0;
    FILE* F = picoquic_file_open(perflog_file_name, "rb");
    if (F == NULL) {
        is_empty = 1;
    }
    else {
        long sz;
        fseek(F, 0, SEEK_END);
        sz = ftell(F);
        if (sz == 0) {
            is_empty = 1;
        }
        (void)picoquic_file_close(F);
    }
    return (is_empty);
}
```

### Rust body
```rust
    match std::fs::metadata(perflog_file_name) {
        Ok(md) => md.len() == 0,
        Err(_) => true,
    }
```

## Pair `picoquic/picohash.c:picohash_create`
C: `picoquic/picohash.c:60-65 picohash_create`
Rust: `rs/fq/src/packet_loop.rs:395-411 create`

### C body
```c
{
    return picohash_create_ex(nb_bin, picohash_hash, picohash_compare, NULL, NULL);
}
```

### Rust body
```rust
pub trait CustomThreadSetnameFn {
    /// Apply `thread_name` to the *current* thread.  Called from
    /// inside the thread after it starts, per the C convention.
    fn set_name(&mut self, thread_name: &str);
}
```

## Pair `picoquic/picohash.c:picohash_delete`
C: `picoquic/picohash.c:155-178 picohash_delete`
Rust: `rs/fq/src/lib.rs:2416-2423 delete`

### C body
```c
{
    if (hash_table->count > 0) {
        for (uint32_t i = 0; i < hash_table->nb_bin; i++) {
            picohash_item* item = hash_table->hash_bin[i];
            while (item != NULL) {
                picohash_item* tmp = item;
                const void* key_to_delete = tmp->key;

                item = item->next_in_bin;

                if (hash_table->picohash_key_to_item == NULL) {
                    free(tmp);
                }
                if (delete_key_too) {
                    free((void*)key_to_delete);
                }
            }
        }
    }

    free(hash_table->hash_bin);
    free(hash_table);
}
```

### Rust body
```rust
    pub fn set_app_wake_time(&mut self, app_wake_time: Instant) {
        self.app_wake_time = app_wake_time;
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_generate_clear`
C: `picoquic/picoquic_lb.c:54-59 picoquic_lb_compat_cid_generate_clear`
Rust: `rs/fq/src/lb.rs:332-336 generate_clear`

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

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_generate`
C: `picoquic/picoquic_lb.c:123-148 picoquic_lb_compat_cid_generate`
Rust: `rs/fq/src/lb.rs:492-503 generate`

### C body
```c
{
    picoquic_load_balancer_cid_context_t* lb_ctx = (picoquic_load_balancer_cid_context_t*)cnx_id_cb_data;
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(cnx_id_local);
    UNREFERENCED_PARAMETER(cnx_id_remote);
#endif
    switch (lb_ctx->method) {
    case picoquic_load_balancer_cid_clear:
        picoquic_lb_compat_cid_generate_clear(quic, lb_ctx, cnx_id_returned);
        break;
    case picoquic_load_balancer_cid_stream_cipher:
        picoquic_lb_compat_cid_generate_stream_cipher(quic, lb_ctx, cnx_id_returned);
        break;
    case picoquic_load_balancer_cid_block_cipher:
        picoquic_lb_compat_cid_generate_block_cipher(quic, lb_ctx, cnx_id_returned);
        break;
    default:
        /* Error, unknown method */
        break;
    }
}
```

### Rust body
```rust
    pub fn generate(&mut self, quic: &Quic, nonce: &ConnectionId) -> ConnectionId {
        let mut cid = *nonce;
        {
            let bytes = cid.as_bytes_mut();
            match self.method {
                ConnectionIdMethod::Clear => self.generate_clear(quic, bytes),
                ConnectionIdMethod::StreamCipher => self.generate_stream_cipher(quic, bytes),
                ConnectionIdMethod::BlockCipher => self.generate_block_cipher(quic, bytes),
            }
        }
        cid
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_verify`
C: `picoquic/picoquic_lb.c:207-234 picoquic_lb_compat_cid_verify`
Rust: `rs/fq/src/internal.rs:16145-16154 lb_verify_cid`

### C body
```c
{
    picoquic_load_balancer_cid_context_t* lb_ctx = (picoquic_load_balancer_cid_context_t*)cnx_id_cb_data;
    uint64_t server_id64;

    if (cnx_id->id_len != lb_ctx->connection_id_length) {
        server_id64 = UINT64_MAX;
    }
    else {
        switch (lb_ctx->method) {
        case picoquic_load_balancer_cid_clear:
            server_id64 = picoquic_lb_compat_cid_verify_clear(lb_ctx, cnx_id);
            break;
        case picoquic_load_balancer_cid_stream_cipher:
            server_id64 = picoquic_lb_compat_cid_verify_stream_cipher(lb_ctx, cnx_id);
            break;
        case picoquic_load_balancer_cid_block_cipher:
            server_id64 = picoquic_lb_compat_cid_verify_block_cipher(lb_ctx, cnx_id);
            break;
        default:
            /* Error, unknown method */
            server_id64 = UINT64_MAX;
            break;
        }
    }

    return server_id64;
}
```

### Rust body
```rust
    pub fn lb_verify_cid(&mut self, cid: &ConnectionId) -> Option<u64> {
        let mut ctx_box = self.connection_id_callback_ctx.take()?;
        let result = if let Some(ctx) = ctx_box.downcast_mut::<crate::lb::ConnectionIdContext>() {
            ctx.verify(cid)
        } else {
            None
        };
        self.connection_id_callback_ctx = Some(ctx_box);
        result
    }
```

## Pair `picoquic/picoquic_mbedtls.c:picoquic_mbedtls_load`
C: `picoquic/picoquic_mbedtls.c:27-42 picoquic_mbedtls_load`
Rust: `rs/fq/src/sys/mod.rs:28-28 mbedtls_load`

### C body
```c
{
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(unload);
#endif
    /* Nothing to do, as the module is not loaded. */
}
```

### Rust body
```rust
pub fn mbedtls_load(_unload: bool) {}
```

## Pair `picoquic/picoquic_ptls_minicrypto.c:picoquic_init_minicrypto`
C: `picoquic/picoquic_ptls_minicrypto.c:48-51 picoquic_init_minicrypto`
Rust: `rs/fq/src/tls_api.rs:2608-2623 init_minicrypto`

### C body
```c
{
    /* Nothing for now */
}
```

### Rust body
```rust
pub fn set_minicrypto_private_key_from_key_file(keypem: &str) -> Result<(), Error> {
    let contents = std::fs::read_to_string(keypem).map_err(|_| Error::NoSuchFile)?;
    if contents.contains("-----BEGIN ") && contents.contains("PRIVATE KEY-----") {
        Ok(())
    } else {
        Err(Error::InvalidFile)
    }
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:set_openssl_sign_certificate_from_key`
C: `picoquic/picoquic_ptls_openssl.c:110-136 set_openssl_sign_certificate_from_key`
Rust: `rs/fq/src/sys/openssl.rs:532-542 set_sign_certificate_from_key`

### C body
```c
{
    int ret = 0;
    ptls_openssl_sign_certificate_t* signer;

    signer = (ptls_openssl_sign_certificate_t*)malloc(sizeof(ptls_openssl_sign_certificate_t));

    if (signer == NULL || pkey == NULL) {
        ret = -1;
    }
    else {
        ret = ptls_openssl_init_sign_certificate(signer, pkey);
        ctx->sign_certificate = &signer->super;
    }

    if (pkey != NULL) {
        EVP_PKEY_free(pkey);
    }

    if (ret != 0 && signer != NULL) {
        free(signer);
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<SignCertificate, crate::Error> {
    let key = key.ok_or(crate::Error::Generic)?;
    // ptls_openssl_init_sign_certificate stores the key pointer in the
    // signer struct and sets `ctx->sign_certificate`.  In Rust we simply
    // move the key into the struct — no additional initialisation is
    // needed because the signing algorithm is chosen dynamically by
    // OpenSSL when a signature is requested.
    Ok(SignCertificate { key })
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_get_certs_from_file`
C: `picoquic/picoquic_ptls_openssl.c:210-235 picoquic_openssl_get_certs_from_file`
Rust: `rs/fq/src/sys/openssl.rs:346-350 get_certs_from_file`

### C body
```c
{
    BIO* bio_key = BIO_new_file(file_name, "rb");
    size_t const max_count = 16;
    ptls_iovec_t* chain = malloc(sizeof(ptls_iovec_t) * max_count);
    *count = 0;
    if (chain != NULL) {
        X509* cert = NULL;
        memset(chain, 0, sizeof(ptls_iovec_t) * max_count);
        /* Load cert and convert to DER */
        while (*count < max_count && (cert = PEM_read_bio_X509(bio_key, NULL, NULL, NULL)) != NULL) {
            int length = i2d_X509(cert, NULL);
            unsigned char* cert_der = (unsigned char*)malloc(length);
            unsigned char* tmp = cert_der;
            i2d_X509(cert, &tmp);
            X509_free(cert);
            chain[*count] = ptls_iovec_init(cert_der, length);
            *count += 1;
        }
    }
    BIO_free(bio_key);
    return chain;
}
```

### Rust body
```rust
    let pem = match std::fs::read(file_name) {
        Ok(data) => data,
        Err(_) => return Vec::new(),
    };
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_set_tls_root_certificates`
C: `picoquic/picoquic_ptls_openssl.c:298-319 picoquic_openssl_set_tls_root_certificates`
Rust: `rs/fq/src/sys/openssl.rs:501-506 picoquic_openssl_set_tls_root_certificates`

### C body
```c
{
    ptls_openssl_verify_certificate_t* verify_ctx = (ptls_openssl_verify_certificate_t*)ctx->verify_certificate;

    for (size_t i = 0; i < count; ++i) {
        uint8_t* cert_i_base = certs[i].base;
        X509* cert = d2i_X509(NULL, (const uint8_t**)&cert_i_base, (long)certs[i].len);

        if (cert == NULL) {
            return -1;
        }

        if (X509_STORE_add_cert(verify_ctx->cert_store, cert) == 0) {
            X509_free(cert);
            return -2;
        }

        X509_free(cert);
    }

    return 0;
}
```

### Rust body
```rust
) -> Result<(), crate::Error> {
    verifier.set_root_certificates(certs)
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:openssl_keyex_dispose`
C: `picoquic/picoquic_ptls_openssl.c:367-371 openssl_keyex_dispose`
Rust: `rs/fq/src/sys/openssl.rs:322-324 openssl_keyex_dispose`

### C body
```c
{
    ptls_iovec_t dummy = ptls_iovec_init(NULL, 0);
    keyex->on_exchange(&keyex, 1, NULL, dummy);
}
```

### Rust body
```rust
pub fn openssl_keyex_dispose(keyex: KeyExchangeContext) {
    drop(keyex);
}
```
