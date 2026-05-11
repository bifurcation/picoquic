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

## `picoquic/quicctx.c:picoquic_verify_proposed_tuple`
* Phase 4C status: `suspect`
* Phase 4C rationale: Bodies mostly follow the same branch structure, but Rust intentionally fails when no local match is found in the Some(peer), None branch, while C's visible post-loop check tests addr_peer rather than whether addr_local was found.
* C source: `picoquic/quicctx.c:2385-2435`
* C signature: `int picoquic_verify_proposed_tuple(picoquic_cnx_t *, const struct sockaddr **, const struct sockaddr **, int *)`
* Rust source: `rs/fq/src/lib.rs:2482-2520`
* Rust item: `verify_proposed_tuple`

### C body
```c
{
    int ret = 0;
    struct sockaddr const* addr_peer = *p_addr_peer;
    struct sockaddr const* addr_local = *p_addr_local;
    int if_index = *p_if_index;

    /* verify that the peer and local addresses are correctly set */
    if (addr_peer == NULL || addr_peer->sa_family == 0) {
        if (addr_local == NULL || addr_local->sa_family == 0) {
            ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
        }
        else {
            /* Find the peer address from existing paths */
            for (int i = 0; i < cnx->nb_paths; i++) {
                if (cnx->path[i]->first_tuple->peer_addr.ss_family == addr_local->sa_family) {
                    addr_peer = (struct sockaddr*)&cnx->path[i]->first_tuple->peer_addr;
                    if_index = cnx->path[i]->first_tuple->if_index;
                    break;
                }
            }
            if (addr_peer == NULL || addr_peer->sa_family == 0) {
                ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
            }
        }
    }
    else if (addr_local == NULL || addr_local->sa_family == 0) {
        /* Find the local address from existing paths */
        for (int i = 0; i < cnx->nb_paths; i++) {
            if (cnx->path[i]->first_tuple->local_addr.ss_family == addr_peer->sa_family) {
                addr_local = (struct sockaddr*)&cnx->path[i]->first_tuple->local_addr;
                if_index = cnx->path[i]->first_tuple->if_index;
                break;
            }
        }
        if (addr_peer == NULL) {
            ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
        }
    }
    else if (addr_peer->sa_family != addr_local->sa_family) {
        ret = PICOQUIC_ERROR_PATH_ADDRESS_FAMILY;
    }

    if (ret == 0) {
        *p_addr_peer = addr_peer;
        *p_addr_local = addr_local;
        *p_if_index = if_index;
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(SocketAddr, SocketAddr, i32), Error> {
        let mut if_index = if_index;
        match (addr_peer, addr_local) {
            (None, None) => Err(Error::Generic),
            (None, Some(local)) => {
                let t = self
                    .paths
                    .iter()
                    .filter_map(|p| p.tuples.first())
                    .find(|t| t.peer_addr.is_ipv4() == local.is_ipv4())
                    .ok_or(Error::Generic)?;
                if_index = t.if_index as i32;
                Ok((t.peer_addr, local, if_index))
            }
            (Some(peer), None) => {
                // C: checks addr_peer == NULL after this loop (copy-paste error; addr_peer
                // is non-NULL in this branch).  Translate defensively: fail if no local found.
                let t = self
                    .paths
                    .iter()
                    .filter_map(|p| p.tuples.first())
                    .find(|t| t.local_addr.is_ipv4() == peer.is_ipv4())
                    .ok_or(Error::Generic)?;
                if_index = t.if_index as i32;
                Ok((peer, t.local_addr, if_index))
            }
            (Some(peer), Some(local)) => {
                if peer.is_ipv4() != local.is_ipv4() {
                    return Err(Error::InvalidArgument);
                }
                Ok((peer, local, if_index))
            }
        }
    }
```

## `picoquic/sender.c:picoquic_create_long_header`
* Phase 4C status: `suspect`
* Phase 4C rationale: Bodies are structurally similar, but C applies do_grease_quic_bit to clear a bit in the first byte while Rust has no visible grease-bit handling.
* C source: `picoquic/sender.c:641-709`
* C signature: `size_t picoquic_create_long_header(picoquic_packet_type_enum, picoquic_connection_id_t *, picoquic_connection_id_t *, int, uint32_t, int, uint64_t, size_t, uint8_t *, uint8_t *, size_t *, size_t *)`
* Rust source: `rs/fq/src/internal.rs:6831-6894`
* Rust item: `create_long_header`

### C body
```c
{
    /* Create a long packet */
    size_t length = 0;

    /* The first byte is defined in RFC 9000 as:
    *     Header Form (1) = 1,
    *     Fixed Bit (1) = 1,
    *     Long Packet Type (2),
    *     Type-Specific Bits (4)
    * The packet type is version dependent. In fact, the whole first byte is version
    * dependent, the invariant draft only specifies the "header form" bit = 1 for long
    * header. In version 1, the packet specific bytes are two reserved bytes +
    * sequence number length, always set to 3 in picoquic (i.e., 4 bytes).
    *
    */
    bytes[0] = picoquic_create_long_packet_type(packet_type, version_index);

    if (do_grease_quic_bit) {
        bytes[0] &= 0xBF;
    }

    length = 1;
    picoformat_32(&bytes[length], version);
    length += 4;

    bytes[length++] = dest_cnx_id->id_len;
    length += picoquic_format_connection_id(&bytes[length], PICOQUIC_MAX_PACKET_SIZE - length, *dest_cnx_id);
    bytes[length++] = srce_cnx_id->id_len;
    length += picoquic_format_connection_id(&bytes[length], PICOQUIC_MAX_PACKET_SIZE - length, *srce_cnx_id);

    /* Special case of packet initial -- encode token as part of header */
    if (packet_type == picoquic_packet_initial) {
        length += picoquic_varint_encode(&bytes[length], PICOQUIC_MAX_PACKET_SIZE - length, retry_token_length);
        if (retry_token_length > 0) {
            memcpy(&bytes[length], retry_token, retry_token_length);
            length += retry_token_length;
        }
    }

    if (packet_type == picoquic_packet_retry) {
        /* No payload length and no sequence number for Retry */
        *pn_offset = 0;
        *pn_length = 0;
    }
    else {
        /* Reserve two bytes for payload length */
        bytes[length++] = 0;
        bytes[length++] = 0;
        /* Encode the sequence number */
        *pn_offset = length;
        *pn_length = 4;
        picoformat_32(&bytes[length], (uint32_t)sequence_number);
        length += 4;
    }
    return length;
}
```

### Rust body
```rust
) -> usize {
    // C: picoquic_create_long_header — encode a QUIC long header into `bytes`.
    let is_v2 = version == Version::V2 as u32 || version == Version::V2Draft as u32;
    let first_byte: u8 = match (packet_type, is_v2) {
        (PacketType::Initial, false) => 0xC3,
        (PacketType::ZeroRttProtected, false) => 0xD3,
        (PacketType::Handshake, false) => 0xE3,
        (PacketType::Retry, false) => 0xF0,
        (PacketType::Initial, true) => 0xD3,
        (PacketType::ZeroRttProtected, true) => 0xE3,
        (PacketType::Handshake, true) => 0xF3,
        (PacketType::Retry, true) => 0xC0,
        _ => 0xFF,
    };
    bytes[0] = first_byte;
    let mut length = 1;
    let ver_bytes = version.to_be_bytes();
    bytes[length..length + 4].copy_from_slice(&ver_bytes);
    length += 4;
    // Dest CID
    bytes[length] = dest_cnx_id.len() as u8;
    length += 1;
    let dlen = dest_cnx_id.len();
    bytes[length..length + dlen].copy_from_slice(dest_cnx_id.as_bytes());
    length += dlen;
    // Src CID
    bytes[length] = srce_cnx_id.len() as u8;
    length += 1;
    let slen = srce_cnx_id.len();
    bytes[length..length + slen].copy_from_slice(srce_cnx_id.as_bytes());
    length += slen;
    // Token for Initial
    if packet_type == PacketType::Initial {
        length += varint_encode(&mut bytes[length..], retry_token.len() as u64);
        bytes[length..length + retry_token.len()].copy_from_slice(retry_token);
        length += retry_token.len();
    }
    if packet_type != PacketType::Retry {
        bytes[length] = 0;
        bytes[length + 1] = 0;
        length += 2;
        *pn_offset = length;
        *pn_length = 4;
        let pn32 = sequence_number as u32;
        bytes[length..length + 4].copy_from_slice(&pn32.to_be_bytes());
        length += 4;
    } else {
        *pn_offset = 0;
        *pn_length = 0;
    }
    length
}
```

## `picoquic/sender.c:picoquic_predict_packet_header_length`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust delegates to another function after deriving a packet context, while C visibly computes short and long header lengths directly from packet_type, ids, token length, and send sequence.
* C source: `picoquic/sender.c:792-855`
* C signature: `size_t picoquic_predict_packet_header_length(picoquic_cnx_t *, picoquic_packet_type_enum, picoquic_packet_context_t *)`
* Rust source: `rs/fq/src/internal.rs:6930-6955`
* Rust item: `predict_packet_header_length`

### C body
```c
{
    uint32_t header_length = 0;

    /* The only purpose of the test below is to appease the static analyzer, so it
     * wont complain of possible NULL deref. On windows we could use "__assume(cnx != NULL)
     * but the documentation does not say anything about that for GCC and CLANG */
    if (cnx == NULL) {
        return 0;
    }

    if (packet_type == picoquic_packet_1rtt_protected) {
        /* Predict acceptable length of packet number */
        uint8_t pn_l = 4;
        int64_t delta = pkt_ctx->send_sequence;
        if (pkt_ctx->pending_first != NULL) {
            delta -= pkt_ctx->pending_first->sequence_number;
        }
        if (delta < 262144) {
            pn_l = 3;
            if (pkt_ctx->send_sequence < 1024) {
                pn_l = 2;
                if (pkt_ctx->send_sequence < 16) {
                    pn_l = 1;
                }
            }
        }

        /* Compute length of a short packet header */
        header_length = 1 + cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len + pn_l;
    }
    else {
        /* Compute length of a long packet header */
        header_length = 1 + /* version */ 4 + /* cnx_id length bytes */ 2;

        /* add dest-id length */
        if (cnx->client_mode && (packet_type == picoquic_packet_initial ||
            packet_type == picoquic_packet_0rtt_protected)
            && picoquic_is_connection_id_null(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id)) {
            header_length += cnx->initial_cnxid.id_len;
        }
        else {
            header_length += cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len;
        }

        /* add srce-id length */
        header_length += cnx->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len;

        /* add length of payload length and packet number */
        header_length += 2 + 4;

        /* add length of tokens for initial packets */
        if (packet_type == picoquic_packet_initial) {
            uint8_t useless[16];
            header_length += (uint32_t)picoquic_varint_encode(useless, 16, cnx->retry_token_length);
            header_length += (uint32_t)cnx->retry_token_length;
        }
    }

    return header_length;
}
```

### Rust body
```rust
    ) -> usize {
        // Delegate to the PacketContext-based version.
        // Determine which PacketContext this pkt_ctx corresponds to.
        // We can identify it by pointer equality via index.
        let pc = {
            let addr = pkt_ctx as *const PacketContextState;
            let mut found = PacketContext::Application;
            for i in 0..self.pkt_ctx.len() {
                if std::ptr::eq(&self.pkt_ctx[i], addr) {
                    found = match i {
                        0 => PacketContext::Application,
                        1 => PacketContext::Handshake,
                        2 => PacketContext::Initial,
                        _ => PacketContext::Application,
                    };
                    break;
                }
            }
            found
        };
        self.predict_packet_header_length_for_pc(packet_type, pc)
    }
```

## `picoquic/sender.c:picoquic_prepare_packet_server_init`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust delegates most construction to a helper and lacks body-visible initial-context preflight and tls_ready-gated state transition conditions present in C.
* C source: `picoquic/sender.c:2215-2349`
* C signature: `int picoquic_prepare_packet_server_init(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:18265-18336`
* Rust item: `prepare_packet_server_init`

### C body
```c
{
    int ret = 0;
    int tls_ready = 0;
    picoquic_epoch_enum epoch = picoquic_epoch_initial;
    picoquic_packet_type_enum packet_type = picoquic_packet_initial;
    picoquic_packet_context_enum pc = picoquic_packet_context_initial;
    size_t checksum_overhead = 8;
    size_t header_length = 0;
    uint8_t* bytes = packet->bytes;
    uint8_t* bytes_max;
    uint8_t* bytes_next;
    size_t length = 0;
    int more_data = 0;
    int is_pure_ack = 1;

    if (*next_wake_time > cnx->start_time + PICOQUIC_MICROSEC_HANDSHAKE_MAX) {
        *next_wake_time = cnx->start_time + PICOQUIC_MICROSEC_HANDSHAKE_MAX;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
    }

    /* The only purpose of the test below is to appease the static analyzer, so it
     * wont complain of possible NULL deref. On windows we could use "__assume(path_x != NULL)"
     * but the documentation does not say anything about that for GCC and CLANG */
    if (path_x == NULL) {
        return PICOQUIC_ERROR_UNEXPECTED_ERROR;
    }

    if (cnx->crypto_context[picoquic_epoch_handshake].aead_encrypt != NULL &&
        cnx->tls_stream[picoquic_epoch_initial].send_queue == NULL) {
        epoch = picoquic_epoch_handshake;
        pc = picoquic_packet_context_handshake;
        packet_type = picoquic_packet_handshake;
    }

    send_buffer_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;

    if (!cnx->initial_validated &&
        (cnx->initial_data_sent + send_buffer_max) > 3 * cnx->initial_data_received){
        /* Sending more data now would break the amplication limit */
        *send_length = 0;
        return 0;
    }

    /* If context is handshake, verify first that there is no need for retransmit or ack
     * on initial context */
    if (pc == picoquic_packet_context_handshake) {
        length = picoquic_prepare_packet_old_context(cnx, picoquic_packet_context_initial,
            path_x, packet, send_buffer_max, current_time, next_wake_time, &header_length);
    }

    if (length == 0) {
        checksum_overhead = picoquic_get_checksum_length(cnx, epoch);
        bytes_max = bytes + send_buffer_max - checksum_overhead;
        tls_ready = (cnx->tls_stream[epoch].send_queue != NULL &&
            cnx->tls_stream[epoch].send_queue->length > cnx->tls_stream[epoch].send_queue->offset);
        length = picoquic_predict_packet_header_length(cnx, packet_type, &cnx->pkt_ctx[pc]);
        packet->ptype = packet_type;
        packet->offset = length;
        header_length = length;
        packet->sequence_number = cnx->pkt_ctx[pc].send_sequence;
        packet->send_time = current_time;
        packet->send_path = path_x;
        packet->pc = pc;
        bytes_next = bytes + length;

        if (((tls_ready || picoquic_find_first_misc_frame(cnx, pc) != NULL)
            && path_x->cwin > path_x->bytes_in_transit && cnx->quic->cwin_max > path_x->bytes_in_transit) 
            || cnx->ack_ctx[pc].act[0].ack_needed) {
            bytes_next = picoquic_format_ack_frame(cnx, bytes_next, bytes_max, &more_data, current_time, pc, 0);
            /* Encode misc frames if present */
            bytes_next = picoquic_format_misc_frames_in_context(cnx, bytes_next, bytes_max,
                &more_data, &is_pure_ack, pc);
            /* Encode the crypto frame if present */
            bytes_next = picoquic_format_crypto_hs_frame(&cnx->tls_stream[epoch],
                bytes_next, bytes_max, &more_data, &is_pure_ack);
            length = bytes_next - bytes;
            *is_initial_sent |= (epoch == picoquic_epoch_initial && !is_pure_ack);

            /* progress the state if the epoch data is all sent */
            if (ret == 0 && tls_ready != 0 && cnx->tls_stream[epoch].send_queue == NULL) {
                if (epoch == picoquic_epoch_handshake && picoquic_tls_client_authentication_activated(cnx->quic) == 0) {
                    picoquic_false_start_transition(cnx, current_time);

                    if (cnx->callback_fn != NULL) {
                        if (cnx->callback_fn(cnx, 0, NULL, 0, picoquic_callback_almost_ready, cnx->callback_ctx, NULL) != 0) {
                            picoquic_log_app_message(cnx, "Callback almost ready returns error 0x%x", PICOQUIC_TRANSPORT_INTERNAL_ERROR);
                            picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0);
                        }
                    }
                }
                else {
                    cnx->cnx_state = picoquic_state_server_handshake;
                }
            }
            packet->length = length;
        }
        else if ((length = picoquic_retransmit_needed(cnx, pc, path_x, current_time, next_wake_time, packet, send_buffer_max, &header_length)) > 0) {
            /* Set the new checksum length */
            checksum_overhead = picoquic_get_checksum_length(cnx, epoch);
            cnx->initial_repeat_needed = 0;
            /* Check whether it makes sens to add an ACK at the end of the retransmission */
            bytes_max = bytes + send_buffer_max - checksum_overhead;
            bytes_next = picoquic_format_ack_frame(cnx, bytes + length, bytes_max, &more_data, current_time, pc, 0);
            length = bytes_next - bytes;
            packet->length = length;
            /* document the send time & overhead */
            packet->send_time = current_time;
            packet->checksum_overhead = checksum_overhead;
        } else {
            length = 0;
            packet->length = 0;
        }
    }

    if (ret == 0 && length == 0 && more_data) {
        *next_wake_time = current_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
    }

    picoquic_finalize_and_protect_packet(cnx, packet,
        ret, length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_max,
        path_x, current_time);

    /* Account for data sent during handshake */
    if (!cnx->initial_validated) {
        cnx->initial_data_sent += *send_length;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let handshake_deadline = Instant::from_ticks(
            self.start_time
                .ticks()
                .saturating_add(MICROSEC_HANDSHAKE_MAX.ticks()),
        );
        if handshake_deadline < *next_wake_time {
            *next_wake_time = handshake_deadline;
        }
        if !self.initial_validated
            && self
                .initial_data_sent
                .saturating_add(send_buffer_max.min(path_x.send_mtu) as u64)
                > 3u64.saturating_mul(self.initial_data_received)
        {
            *send_length = 0;
            return 0;
        }
        let epoch = if self.crypto_context[Epoch::Handshake as usize]
            .aead_encrypt
            .is_some()
            && self.tls_stream[Epoch::Initial as usize]
                .send_queue
                .is_empty()
        {
            Epoch::Handshake
        } else {
            Epoch::Initial
        };
        let ret = self.prepare_handshake_packet_for_epoch(
            path_x,
            packet,
            current_time,
            send_buffer,
            send_buffer_max,
            send_length,
            next_wake_time,
            epoch,
            false,
            is_initial_sent,
        );
        if ret == 0 && *send_length > 0 && !self.initial_validated {
            self.initial_data_sent = self.initial_data_sent.saturating_add(*send_length as u64);
        }
        if ret == 0 && *send_length > 0 && self.tls_stream[epoch as usize].send_queue.is_empty() {
            if epoch == Epoch::Handshake
                && !self
                    .quic_ref()
                    .map(|q| q.client_authentication)
                    .unwrap_or(false)
            {
                self.false_start_transition(current_time);
                if let Some(mut cb) = self.callback_fn.take() {
                    let _ = cb.callback(self, 0, &[], CallbackEvent::AlmostReady, None);
                    self.callback_fn = Some(cb);
                }
            } else {
                self.connection_state = State::ServerHandshake;
            }
        }
        ret
    }
```

## `picoquic/sim_link.c:picoquic_test_simlink_suspend`
* Phase 4C status: `suspect`
* Phase 4C rationale: For simulate_receive, C only adjusts the leading packets while arrival_time is before the interval end; Rust scans and adjusts all packets.
* C source: `picoquic/sim_link.c:340-373`
* C signature: `void picoquic_test_simlink_suspend(picoquictest_sim_link_t *, uint64_t, int)`
* Rust source: `rs/fq/src/tests/util.rs:517-533`
* Rust item: `suspend`

### C body
```c
{
    picoquictest_sim_packet_t* packet;
    picoquictest_sim_packet_t* first_old;

    if (simulate_receive) {
        /* specify the resume time */
        link->resume_time = time_end_of_interval;
        /* packets scheduled to arrive before the end of the interval are rescheduled to
         * that end of interval time.
         */
        packet = link->first_packet;
        while (packet != NULL && packet->arrival_time < time_end_of_interval) {
            packet->arrival_time = time_end_of_interval;
            packet = packet->next_packet;
        }
    }
    else {
        /* Reset the queue delay to the end of interval */
        link->queue_time = time_end_of_interval;
        /* stash the old queue, and reset the queue pointers */
        first_old = link->first_packet;
        link->first_packet = NULL;
        link->last_packet = NULL;
        /* resubmit all packets at the end of interval time */
        packet = first_old;
        while (packet != NULL) {
            picoquictest_sim_packet_t* next_packet = packet->next_packet;
            packet->next_packet = NULL;
            picoquictest_sim_link_submit(link, packet, time_end_of_interval);
            packet = next_packet;
        }
    }
}
```

### Rust body
```rust
    pub fn suspend(&mut self, time_end_of_interval: Instant, simulate_receive: bool) {
        if simulate_receive {
            self.resume_time = time_end_of_interval;
            let end = time_end_of_interval.ticks();
            for p in &mut self.packets {
                if p.arrival_time.ticks() < end {
                    p.arrival_time = time_end_of_interval;
                }
            }
        } else {
            self.queue_time = time_end_of_interval;
            let old: Vec<TestSimPacket> = self.packets.drain(..).collect();
            for p in old {
                self.submit(p, time_end_of_interval);
            }
        }
    }
```
