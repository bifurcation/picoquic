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

## Pair `picoquic/packet.c:picoquic_queue_retry_packet`
C: `picoquic/packet.c:1210-1238 picoquic_queue_retry_packet`
Rust: `rs/fq/src/lib.rs:6035-6067 queue_retry_packet`

### C body
```c
{
    int ret = 0;
    uint8_t token_buffer[256];
    size_t token_size;
    picoquic_connection_id_t s_cid = { 0 };

    picoquic_create_local_cnx_id(quic, &s_cid, ph->dest_cnx_id);


    if (picoquic_prepare_retry_token(quic, addr_from,
        current_time, &ph->dest_cnx_id,
        &s_cid, ph->pn, token_buffer, sizeof(token_buffer), &token_size) != 0) {
        ret = PICOQUIC_ERROR_MEMORY;
    }
    else {
        picoquic_queue_stateless_retry(quic, ph, &s_cid, addr_from, addr_to, if_index_to,
            token_buffer, token_size);
        ret = PICOQUIC_ERROR_RETRY;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut server_cid = ConnectionId::default();
        self.create_local_cnx_id(&mut server_cid, ph.dest_connection_id);
        let mut token_buffer = [0u8; 256];
        match self.prepare_retry_token(
            addr_from,
            current_time,
            &ph.dest_connection_id,
            &server_cid,
            ph.packet_number_full as u32,
            &mut token_buffer,
        ) {
            Ok(token_size) => {
                self.queue_stateless_retry(
                    ph,
                    &server_cid,
                    addr_from,
                    addr_to,
                    if_index_to,
                    &token_buffer[..token_size],
                );
                InternalError::Retry as i32
            }
            Err(error) => Self::parse_error_status(error),
        }
    }
```

## Pair `picoquic/packet.c:picoquic_incoming_client_initial`
C: `picoquic/packet.c:1394-1505 picoquic_incoming_client_initial`
Rust: `rs/fq/src/lib.rs:7384-7510 incoming_client_initial`

### C body
```c
{
    int ret = 0;

    if (ret == 0) {
        if ((*pcnx)->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len > 0 &&
            picoquic_compare_connection_id(&ph->dest_cnx_id, &(*pcnx)->path[0]->first_tuple->p_local_cnxid->cnx_id) == 0) {
            (*pcnx)->initial_validated = 1;
        }

        if (!(*pcnx)->initial_validated && (*pcnx)->pkt_ctx[picoquic_packet_context_initial].pending_first != NULL
            && packet_length >= PICOQUIC_ENFORCED_INITIAL_MTU) {
            /* In most cases, receiving more than 1 initial packets before validation indicates that the
             * client is repeating data that it believes is lost. We set the initial_repeat_needed flag
             * to trigger such repetitions. There are exceptions, e.g., clients sending large client hellos
             * that require multiple packets. These exceptions are detected and handled during packet
             * processing. */
            (*pcnx)->initial_repeat_needed = 1;
        }

        if ((*pcnx)->cnx_state == picoquic_state_server_init && 
            ((*pcnx)->quic->server_busy || 
            (*pcnx)->quic->current_number_connections > (*pcnx)->quic->tentative_max_number_connections)) {
            (*pcnx)->local_error = PICOQUIC_TRANSPORT_SERVER_BUSY;
            (*pcnx)->cnx_state = picoquic_state_handshake_failure;
        }
        else if ((*pcnx)->cnx_state == picoquic_state_server_init && 
            (*pcnx)->initial_cnxid.id_len < PICOQUIC_ENFORCED_INITIAL_CID_LENGTH) {
            (*pcnx)->local_error = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
            (*pcnx)->cnx_state = picoquic_state_handshake_failure;
        }
        else if ((*pcnx)->cnx_state < picoquic_state_server_almost_ready) {
            /* Document the incoming addresses */
            if ((*pcnx)->path[0]->first_tuple->local_addr.ss_family == 0 && addr_to != NULL) {
                picoquic_store_addr(&(*pcnx)->path[0]->first_tuple->local_addr, addr_to);
            }
            if ((*pcnx)->path[0]->first_tuple->peer_addr.ss_family == 0 && addr_from != NULL) {
                picoquic_store_addr(&(*pcnx)->path[0]->first_tuple->peer_addr, addr_from);
            }
            (*pcnx)->path[0]->first_tuple->if_index = if_index_to;

            /* decode the incoming frames */
            if (ret == 0) {
                uint64_t highest_ack_before = (*pcnx)->pkt_ctx[picoquic_packet_context_initial].highest_acknowledged;
                ret = picoquic_decode_frames(*pcnx, (*pcnx)->path[0],
                    bytes + ph->offset, ph->payload_length, received_data,
                ph->epoch, addr_from, addr_to, ph->pn64, 0, current_time);
                if ((*pcnx)->pkt_ctx[picoquic_packet_context_initial].highest_acknowledged > highest_ack_before &&
                    (*pcnx)->quic->random_initial > 1) {
                    /* Randomized sequence number was acknowledged. Consider the
                     * connection validated */
                    (*pcnx)->initial_validated = 1;
                }
            }

            /* processing of client initial packet */
            if (ret == 0) {
                int data_consumed = 0;
                /* initialization of context & creation of data */
                ret = picoquic_tls_stream_process(*pcnx, &data_consumed, current_time);
                /* The "initial_repeat_needed" flag is set if multiple initial packets are
                 * received while the connection is not yet validated. In most cases, this indicates
                 * that the client repeated some initial packets, or sent some gratuitous initial
                 * packets, because it believes its own initial packet was lost. The flag forces
                 * immediate retransmission of initial packets. However, there are cases when the
                 * client sent large client hello messages that do not fit on a single packets. In
                 * those cases, the flag should not be set. We detect that by testing whether new
                 * TLS data was received in the packet. */
                if (data_consumed) {
                    (*pcnx)->initial_repeat_needed = 0;
                }
            }
        }
        else if ((*pcnx)->cnx_state < picoquic_state_ready) {
            /* Require an acknowledgement if the packet contains ackable frames */
            picoquic_ignore_incoming_handshake(*pcnx, bytes, ph, current_time);
        }
        else {
            /* Initial keys should have been discarded, treat packet as unexpected */
            ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
        }
    }

    if (ret == PICOQUIC_ERROR_INVALID_TOKEN && (*pcnx)->cnx_state == picoquic_state_handshake_failure) {
        ret = 0;
    }

    if (ret == 0 && (*pcnx)->cnx_state == picoquic_state_handshake_failure && new_context_created) {
        picoquic_queue_immediate_close(*pcnx, current_time);
    }

    if (ret != 0 || (*pcnx)->cnx_state == picoquic_state_disconnected) {
        /* This is bad. If this is an initial attempt, delete the connection */
        if (new_context_created) {
            picoquic_delete_cnx(*pcnx);
            *pcnx = NULL;
            ret = PICOQUIC_ERROR_CONNECTION_DELETED;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> (i32, Option<ConnectionToken>) {
        let server_busy = self.server_busy;
        let over_connection_limit =
            self.current_number_connections > self.tentative_max_number_connections;
        let mut ret = 0;
        let mut queue_close = false;
        let mut delete_created_connection = false;

        {
            let Some(cnx) = self.connections.get_mut(connection) else {
                return (InternalError::UnexpectedPacket as i32, None);
            };

            if cnx
                .path_local_connection_id(0)
                .is_some_and(|cid| !cid.is_empty() && cid == ph.dest_connection_id)
            {
                cnx.initial_validated = true;
            }

            if !cnx.initial_validated
                && !cnx.pkt_ctx[PacketContext::Initial as usize]
                    .pending
                    .is_empty()
                && packet_length >= crate::internal::ENFORCED_INITIAL_MTU
            {
                cnx.initial_repeat_needed = true;
            }

            if cnx.connection_state == State::ServerInit && (server_busy || over_connection_limit) {
                cnx.local_error = TransportError::ServerBusy as u64;
                cnx.connection_state = State::HandshakeFailure;
            } else if cnx.connection_state == State::ServerInit
                && cnx.initial_connection_id.len()
                    < crate::internal::ENFORCED_INITIAL_CID_LENGTH as usize
            {
                cnx.local_error = TransportError::ProtocolViolation as u64;
                cnx.connection_state = State::HandshakeFailure;
            } else if cnx.connection_state < State::ServerAlmostReady {
                if let Some(path) = cnx.paths.get_mut(0)
                    && let Some(tuple) = path.tuples.first_mut()
                {
                    if Connection::socket_addr_is_unspecified(&tuple.local_addr)
                        && let Some(addr) = addr_to
                    {
                        tuple.local_addr = *addr;
                    }
                    if Connection::socket_addr_is_unspecified(&tuple.peer_addr)
                        && let Some(addr) = addr_from
                    {
                        tuple.peer_addr = *addr;
                    }
                    tuple.if_index = if_index_to as core::ffi::c_ulong;
                }

                let highest_ack_before =
                    cnx.pkt_ctx[PacketContext::Initial as usize].highest_acknowledged;
                let payload = Connection::packet_payload(bytes, ph);
                ret = cnx.decode_frames_on_path(
                    0,
                    payload,
                    received_data,
                    ph.epoch,
                    addr_from,
                    addr_to,
                    ph.packet_number_full,
                    0,
                    current_time,
                );
                if cnx.pkt_ctx[PacketContext::Initial as usize].highest_acknowledged
                    > highest_ack_before
                    && cnx.random_initial > 1
                {
                    cnx.initial_validated = true;
                }

                if ret == 0 {
                    let (tls_ret, data_consumed) = cnx.process_tls_stream_status(current_time);
                    ret = tls_ret;
                    if data_consumed > 0 {
                        cnx.initial_repeat_needed = false;
                    }
                }
            } else if cnx.connection_state < State::Ready {
                cnx.ignore_incoming_handshake(bytes, ph, current_time);
            } else {
                ret = InternalError::UnexpectedPacket as i32;
            }

            if ret == InternalError::InvalidToken as i32
                && cnx.connection_state == State::HandshakeFailure
            {
                ret = 0;
            }

            if ret == 0 && cnx.connection_state == State::HandshakeFailure && new_context_created {
                queue_close = true;
            }

            if ret != 0 || cnx.connection_state == State::Disconnected {
                delete_created_connection = new_context_created;
            }
        }

        if queue_close {
            self.queue_immediate_close(connection, current_time);
        }

        if delete_created_connection {
            self.delete_connection(connection);
            (InternalError::ConnectionDeleted as i32, None)
        } else {
            (ret, Some(connection))
        }
    }
```

## Pair `picoquic/packet.c:picoquic_incoming_client_handshake`
C: `picoquic/packet.c:1753-1811 picoquic_incoming_client_handshake`
Rust: `rs/fq/src/lib.rs:7063-7113 incoming_client_handshake`

### C body
```c
{
    int ret = 0;

    cnx->initial_validated = 1;
    cnx->initial_repeat_needed = 0;

    if (cnx->cnx_state < picoquic_state_server_almost_ready) {
        if (picoquic_compare_connection_id(&ph->srce_cnx_id, &cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id) != 0) {
            ret = PICOQUIC_ERROR_CNXID_CHECK;
        } else {
            /* Accept the incoming frames */
            if (ph->payload_length == 0) {
                /* empty payload! */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
            else {
                ret = picoquic_decode_frames(cnx, cnx->path[0],
                    bytes + ph->offset, ph->payload_length, received_data,
                    ph->epoch, NULL, NULL, ph->pn64, 0, current_time);
            }
            /* processing of client clear text packet */
            if (ret == 0) {
                /* Any successful handshake packet is an explicit ack of initial packets */
                picoquic_implicit_handshake_ack(cnx, picoquic_packet_context_initial, current_time);
                picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_initial]);

                /* If TLS data present, progress the TLS state */
                ret = picoquic_tls_stream_process(cnx, NULL, current_time);

                /* If TLS FIN has been received, the server side handshake is ready */
                if (!cnx->client_mode && cnx->cnx_state < picoquic_state_ready && picoquic_is_tls_complete(cnx)) {
                    picoquic_ready_state_transition(cnx, current_time);
                }
            }
        }
    }
    else if (cnx->cnx_state <= picoquic_state_ready) {
        /* Because the client is never guaranteed to discard handshake keys,
         * we need to keep it for the duration of the connection.
         * Process the incoming frames, ignore them, but 
         * require an acknowledgement if the packet contains ackable frames */
        picoquic_ignore_incoming_handshake(cnx, bytes, ph, current_time);
    } 
    else {
        /* Not expected. Log and ignore. */
        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        self.initial_validated = true;
        self.initial_repeat_needed = false;

        let mut ret = 0;
        if self.connection_state < State::ServerAlmostReady {
            if self.path_remote_connection_id(0) != Some(ph.src_connection_id) {
                ret = InternalError::CnxidCheck as i32;
            } else if ph.payload_length == 0 {
                ret = self.connection_error(TransportError::ProtocolViolation as u64, 0);
            } else {
                let payload = Self::packet_payload(bytes, ph);
                ret = self.decode_frames_on_path(
                    0,
                    payload,
                    received_data,
                    ph.epoch,
                    None,
                    None,
                    ph.packet_number_full,
                    0,
                    current_time,
                );
                if ret == 0 {
                    self.implicit_handshake_ack(PacketContext::Initial, current_time);
                    self.crypto_context[crate::internal::Epoch::Initial as usize].free_handles();
                    let (tls_ret, _) = self.process_tls_stream_status(current_time);
                    ret = tls_ret;
                    if ret == 0
                        && !self.client_mode
                        && self.connection_state < State::Ready
                        && self.is_tls_complete()
                    {
                        self.ready_state_transition(current_time);
                    }
                }
            }
        } else if self.connection_state <= State::Ready {
            self.ignore_incoming_handshake(bytes, ph, current_time);
        } else {
            ret = InternalError::UnexpectedPacket as i32;
        }

        ret
    }
```

## Pair `picoquic/packet.c:picoquic_incoming_1rtt`
C: `picoquic/packet.c:1910-2017 picoquic_incoming_1rtt`
Rust: `rs/fq/src/lib.rs:7167-7303 incoming_1rtt`

### C body
```c
{
    int ret = 0;

    /* Check the packet */
    if (cnx->cnx_state < picoquic_state_client_almost_ready) {
        /* handshake is not complete. Just ignore the packet */
        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
    }
    else if (cnx->cnx_state == picoquic_state_disconnected) {
        /* Connection is disconnected. Just ignore the packet */
        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
    }
    else {
        /* Packet is correct */

        /* TODO: consider treatment of migration during closing mode */

        /* Do not process data in closing or draining modes */
        if (cnx->cnx_state >= picoquic_state_disconnecting) {
            /* only look for closing frames in closing modes */
            if (cnx->cnx_state == picoquic_state_closing || cnx->cnx_state == picoquic_state_disconnecting) {
                int closing_received = 0;

                ret = picoquic_decode_closing_frames(
                    bytes + ph->offset, ph->payload_length, &closing_received);

                if (ret == 0) {
                    if (closing_received) {
                        if (cnx->client_mode) {
                            picoquic_connection_disconnect(cnx);
                        }
                        else {
                            cnx->cnx_state = picoquic_state_draining;
                        }
                    }
                    else {
                        picoquic_set_ack_needed(cnx, current_time, ph->pc, cnx->path[path_id], 0);
                    }
                }
            }
            else {
                /* Just ignore the packets in closing received or draining mode */
                ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
            }
        }
        else if (ret == 0) {
            picoquic_path_t* path_x = cnx->path[path_id];

            path_x->first_tuple->if_index = if_index_to;
            cnx->is_1rtt_received = 1;
            picoquic_spin_function_table[cnx->spin_policy].spinbit_incoming(cnx, path_x, ph);
            /* Accept the incoming frames */
            ret = picoquic_decode_frames(cnx, cnx->path[path_id],
                bytes + ph->offset, ph->payload_length, received_data,
                ph->epoch, addr_from, addr_to, ph->pn64,
                path_is_not_allocated, current_time);

            if (ret == 0) {
                /* Compute receive bandwidth */
                path_x->received += (uint64_t)ph->offset + ph->payload_length +
                    picoquic_get_checksum_length(cnx, picoquic_epoch_1rtt);
                if (path_x->receive_rate_epoch == 0) {
                    path_x->received_prior = cnx->path[path_id]->received;
                    path_x->receive_rate_epoch = current_time;
                }
                else {
                    uint64_t delta = current_time - cnx->path[path_id]->receive_rate_epoch;
                    if (delta > path_x->smoothed_rtt && delta > PICOQUIC_BANDWIDTH_TIME_INTERVAL_MIN) {
                        path_x->receive_rate_estimate = PICOQUIC_RATE_FROM_BYTES(
                            cnx->path[path_id]->received - cnx->path[path_id]->received_prior, delta);
                        path_x->received_prior = cnx->path[path_id]->received;
                        path_x->receive_rate_epoch = current_time;
                        if (path_x->receive_rate_estimate > cnx->path[path_id]->receive_rate_max) {
                            path_x->receive_rate_max = cnx->path[path_id]->receive_rate_estimate;
                            if (path_id == 0 && !cnx->is_ack_frequency_negotiated) {
                                picoquic_compute_ack_gap_and_delay(cnx, cnx->path[0]->rtt_min, PICOQUIC_ACK_DELAY_MIN,
                                    cnx->path[0]->receive_rate_max, &cnx->ack_gap_remote, &cnx->ack_delay_remote);
                            }
                        }
                    }
                }

                /* Processing of TLS messages  */
                ret = picoquic_tls_stream_process(cnx, NULL, current_time);
            }

            if (ret == 0 && picoquic_cnx_is_still_logging(cnx)) {
                picoquic_log_cc_dump(cnx, current_time);
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        if self.connection_state < State::ClientAlmostReady {
            return InternalError::UnexpectedPacket as i32;
        }
        if self.connection_state == State::Disconnected {
            return InternalError::UnexpectedPacket as i32;
        }

        if self.connection_state >= State::Disconnecting {
            if self.connection_state == State::Closing
                || self.connection_state == State::Disconnecting
            {
                let payload = Self::packet_payload_mut(bytes, ph);
                let mut closing_received = 0;
                let ret = crate::internal::decode_closing_frames(
                    payload,
                    payload.len(),
                    &mut closing_received,
                );
                if ret == 0 {
                    if closing_received != 0 {
                        if self.client_mode {
                            self.connection_disconnect();
                        } else {
                            self.connection_state = State::Draining;
                        }
                    } else {
                        self.set_ack_needed_on_path(current_time, ph.packet_context, path_id, 0);
                    }
                }
                ret
            } else {
                InternalError::UnexpectedPacket as i32
            }
        } else {
            if path_id >= self.paths.len() {
                return InternalError::UnexpectedPacket as i32;
            }

            let payload = Self::packet_payload(bytes, ph);
            let mut path = self.paths.remove(path_id);
            if let Some(tuple) = path.tuples.first_mut() {
                tuple.if_index = if_index_to as core::ffi::c_ulong;
            }
            self.is_1rtt_received = true;
            if let Some(policy) =
                crate::internal::SPIN_FUNCTION_TABLE.get(self.spin_policy as usize)
            {
                policy.incoming(self, &mut path, ph);
            }

            let mut ret = self.decode_frames(
                &mut path,
                payload,
                payload.len(),
                received_data,
                ph.epoch as i32,
                addr_from,
                addr_to,
                ph.packet_number_full,
                path_is_not_allocated,
                current_time,
            );

            let mut recompute_ack_frequency = None;
            if ret == 0 {
                path.received = path.received.saturating_add(
                    (ph.offset as u64)
                        .saturating_add(ph.payload_length as u64)
                        .saturating_add(
                            self.get_checksum_length(crate::internal::Epoch::OneRtt) as u64
                        ),
                );
                if path.receive_rate_epoch == 0 {
                    path.received_prior = path.received;
                    path.receive_rate_epoch = current_time.ticks();
                } else {
                    let delta = current_time.ticks().saturating_sub(path.receive_rate_epoch);
                    if delta > path.smoothed_rtt.ticks()
                        && delta > crate::internal::BANDWIDTH_TIME_INTERVAL_MIN
                    {
                        path.receive_rate_estimate = crate::utils::rate_from_bytes(
                            path.received.saturating_sub(path.received_prior),
                            delta,
                        );
                        path.received_prior = path.received;
                        path.receive_rate_epoch = current_time.ticks();
                        if path.receive_rate_estimate > path.receive_rate_max {
                            path.receive_rate_max = path.receive_rate_estimate;
                            if path_id == 0 && !self.is_ack_frequency_negotiated {
                                recompute_ack_frequency =
                                    Some((path.rtt_min, path.receive_rate_max));
                            }
                        }
                    }
                }
            }

            self.paths.insert(path_id, path);

            if let Some((rtt_min, receive_rate_max)) = recompute_ack_frequency {
                let mut ack_gap = self.ack_gap_remote;
                let mut ack_delay = self.ack_delay_remote.ticks();
                self.compute_ack_gap_and_delay(
                    rtt_min,
                    crate::internal::ACK_DELAY_MIN.ticks(),
                    receive_rate_max,
                    &mut ack_gap,
                    &mut ack_delay,
                );
                self.ack_gap_remote = ack_gap;
                self.ack_delay_remote = Duration::from_ticks(ack_delay);
            }

            if ret == 0 {
                let (tls_ret, _) = self.process_tls_stream_status(current_time);
                ret = tls_ret;
            }

            if ret == 0 && self.is_still_logging() {
                crate::logger::Log::cc_dump(self, current_time);
            }

            ret
        }
    }
```

## Pair `picoquic/packet.c:picoquic_incoming_packet`
C: `picoquic/packet.c:2432-2447 picoquic_incoming_packet`
Rust: `rs/fq/src/lib.rs:3264-3282 incoming_packet`

### C body
```c
{
    picoquic_cnx_t* first_cnx = NULL;

    int ret = picoquic_incoming_packet_ex(quic, bytes, packet_length, addr_from, addr_to,
        if_index_to, received_ecn, &first_cnx, current_time);
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        self.incoming_packet_ex(
            bytes,
            addr_from,
            addr_to,
            if_index_to,
            received_ecn,
            current_time,
        )
        .map(|_| ())
    }
```

## Pair `picoquic/paths.c:picoquic_prepare_path_challenge_frames`
C: `picoquic/paths.c:140-148 picoquic_prepare_path_challenge_frames`
Rust: `rs/fq/src/internal.rs:4464-4488 prepare_path_challenge_frames`

### C body
```c
{
    return picoquic_prepare_tuple_challenge_frames(cnx, path_x, path_x->first_tuple,
        bytes_next, bytes_max, more_data, is_pure_ack, is_challenge_padding_needed,
        current_time, next_wake_time);
}
```

### Rust body
```rust
    ) -> Option<&'a mut [u8]> {
        if path_x.tuples.is_empty() {
            Some(bytes)
        } else {
            self.prepare_tuple_challenge_frames(
                path_x,
                0,
                bytes,
                more_data,
                is_pure_ack,
                is_challenge_padding_needed,
                current_time,
                next_wake_time,
            )
        }
    }
```

## Pair `picoquic/paths.c:picoquic_check_path_control_needed`
C: `picoquic/paths.c:288-321 picoquic_check_path_control_needed`
Rust: `rs/fq/src/internal.rs:4599-4622 check_path_control_needed`

### C body
```c
{
    /* examine each tuple record */
    picoquic_tuple_t* tuple = path_x->first_tuple;

    while (tuple != NULL) {
        if (tuple->challenge_failed) {
            if (tuple != path_x->first_tuple && current_time > tuple->demotion_time) {
                cnx->tuple_demotion_needed = 1;
            }
            /* go to next tuple */
        }
        else if (tuple->response_required) {
            /* selected */
            break;
        }
        else if (tuple->challenge_required && !tuple->challenge_verified) {
            uint64_t next_challenge_time = picoquic_tuple_challenge_time(path_x, tuple, current_time);
            if (current_time >= next_challenge_time) {
                break;
            }
            else if (next_challenge_time < *next_wake_time) {
                *next_wake_time = next_challenge_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
            }
        }
        tuple = tuple->next_tuple;
    }
    return tuple;
}
```

### Rust body
```rust
        for tuple_index in 0..path_x.tuples.len() {
            let tuple = &path_x.tuples[tuple_index];
            if tuple.challenge_failed {
                if tuple_index != 0 && current_time > tuple.demotion_time {
                    self.tuple_demotion_needed = true;
                }
            } else if tuple.response_required {
                return Some(tuple_index);
            } else if tuple.challenge_required && !tuple.challenge_verified {
                let next_challenge_time = tuple_challenge_time(path_x, tuple, current_time);
                if current_time >= next_challenge_time {
                    return Some(tuple_index);
                }
                if next_challenge_time < *next_wake_time {
                    *next_wake_time = next_challenge_time;
                }
            }
        }
```

## Pair `picoquic/paths.c:picoquic_find_incoming_path`
C: `picoquic/paths.c:614-747 picoquic_find_incoming_path`
Rust: `rs/fq/src/internal.rs:4376-4396 find_incoming_path`

### C body
```c
{
    int ret = 0;
    picoquic_path_t* path_x = NULL;
    picoquic_tuple_t* tuple = NULL;
    int path_id = (ph->l_cid == NULL) ? 0 : picoquic_find_path_by_unique_id(cnx, ph->l_cid->path_id);

    if (path_id < 0) {
        /* Either this path has not yet been created, or it was already destroyed.
        * The packet decryption was successful, which means that the CID is valid,
        * but on the server side we might have a "probe".
         */
        if (cnx->nb_paths < PICOQUIC_NB_PATH_TARGET &&
            (cnx->quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from)) &&
            picoquic_create_path(cnx, current_time, addr_to, addr_from, if_index_to, ph->l_cid->path_id) > 0) {
            /* if we do create a new path, it should have the right path_id. We cannot
            * assume that paths will be created in the full order, so that means we may
            * have to create "empty" paths in invalid state. Or, more simply,
            * create a path and override the unique path id, which should be OK
            * as that unique ID does not exist.
            * TODO: modify path creation to force path_id, return error if impossible.
             */
            path_id = cnx->nb_paths - 1;
            path_x = cnx->path[path_id];

            /* when creating the path, we need to copy the dest CID and chose
             * destination CID with the matching path ID.
             */
            path_x->first_tuple->p_local_cnxid = picoquic_find_local_cnxid(cnx, path_x->unique_path_id, &ph->dest_cnx_id);
            picoquic_assign_peer_cnxid_to_tuple(cnx, path_x, path_x->first_tuple);
        }
    }
    else
    {
        path_x = cnx->path[path_id];
        tuple = path_x->first_tuple;

        /* If the local CID is not set, set it */
        if (path_x->first_tuple->p_local_cnxid == NULL) {
            path_x->first_tuple->p_local_cnxid = picoquic_find_local_cnxid(cnx, path_x->unique_path_id, &ph->dest_cnx_id);
            if (!cnx->client_mode && cnx->is_multipath_enabled && path_x->first_tuple->challenge_verified) {
                /* If the peer renewed its connection id, the retire connection ID frame may already
                 * have arrived on a separate path. If the server noticed that, it should also renew
                 * its "remote path" ID */
                (void)picoquic_renew_connection_id(cnx, path_id);
            }
        }

        /* Treat the special case of the unkown local address, which should only happen
         * for clients and for the first tuple. */
        if (path_x->first_tuple->local_addr.ss_family == AF_UNSPEC && addr_to->sa_family != AF_UNSPEC) {
            picoquic_store_addr(&cnx->path[path_id]->first_tuple->local_addr, addr_to);
        }

        /* Look for the best match among existing tuples */
        while (tuple != NULL) {
            /* If the addresses match, we are good. */
            if (picoquic_compare_addr(addr_from, (struct sockaddr*)&tuple->peer_addr) == 0 &&
                picoquic_compare_addr(addr_to, (struct sockaddr*)&tuple->local_addr) == 0) {
                break;
            }
            else
            {
                tuple = tuple->next_tuple;
            }
        }
        if (tuple == NULL) {
            /* If the addresses do not match, we have two possibilities:
            * either the creation of a new tuple, or a NAT rebinding on an existing tuple.
            * In all cases, we need to create a new tuple. In the NAT rebinding cases, we
            * need to be a bit more agressive, i.e., immediately promote the new tuple
            * as the default. In fact, we MUST do that if the CID also changed, otherwise
            * we will stumble on a bug if the packet asks to retire the CID.
            *
            * We thus need to distinguish the NAT rebinding case from the non-multipath
            * path-migration. This is bound to be ambiguous, but we can use a simple heuristic:
            *
            * - if multipath is enabled, the old style path migration is supported but
            *   discouraged. It is mostly there to support "migration to a preferred
            *   address", and there is no much harm to always treat that as a NAT
            *   rebinding. Maybe make an exception if the destination address is
            *   one of the preferred addresses.
            * - if multipath is not enabled, check whether this looks like a challenge
            *   for a new address, i.e., it contains a PATH CHALLENGE frame and only
            *   non-path validating packets. If true, treat it as a path migration challenge.
            *   else, treat it as a NAT rebinding.
            */

            if (picoquic_check_cid_for_new_tuple(cnx, path_x->unique_path_id) == 0 &&
                (tuple = picoquic_create_tuple(path_x, addr_to, addr_from, if_index_to)) != NULL &&
                picoquic_assign_peer_cnxid_to_tuple(cnx, path_x, tuple) == 0) {
                picoquic_set_tuple_challenge(tuple, current_time, cnx->quic->use_constant_challenges);
                if (picoquic_compare_connection_id(&path_x->first_tuple->p_local_cnxid->cnx_id, &ph->dest_cnx_id) != 0 &&
                    cnx->is_multipath_enabled) {
                    /* Treat this as a NAT rebinding. */
                    picoquic_tuple_t* old_tuple = path_x->first_tuple;
                    /* We need to replace the first tuple by this tuple. */
                    picoquic_set_first_tuple(path_x, tuple);
                    tuple->challenge_verified = 1;
                    /* set a challenge on the old tuple to recover from spoofed addresses */
                    picoquic_set_tuple_challenge(old_tuple, current_time, cnx->quic->use_constant_challenges);
                    old_tuple->challenge_required = 1;
                    old_tuple->challenge_verified = 0;
                }
                else {
                    /* Treat this a new tuple challenge. */
                    tuple->challenge_required = 1;
                }
            }
            /* TODO: clean up in case of failure. */
        }
        else {
            /* If the addresses do match, but the CID do not, we have a case of CID migration.
             */
            if (tuple == path_x->first_tuple &&
                picoquic_compare_connection_id(&path_x->first_tuple->p_local_cnxid->cnx_id, &ph->dest_cnx_id) != 0) {
                path_x->first_tuple->p_local_cnxid = picoquic_find_local_cnxid(cnx, path_x->unique_path_id, &ph->dest_cnx_id);
                if (cnx->client_mode == 0) {
                    (void)picoquic_renew_connection_id(cnx, path_id);
                }
            }
        }
    }
    *p_path_id = path_id;
    cnx->path[path_id]->last_packet_received_at = current_time;

    return ret;
}
```

### Rust body
```rust
        for (path_id, path) in self.paths.iter_mut().enumerate() {
            if path.tuples.iter().any(|tuple| {
                tuple.peer_addr == *addr_from
                    && tuple.local_addr == *addr_to
                    && tuple.if_index == if_index_to as core::ffi::c_ulong
            }) {
                path.last_packet_received_at = current_time;
                return Ok(IncomingPathLookup {
                    path_id,
                    created: false,
                });
            }
        }
```

## Pair `picoquic/performance_log.c:picoquic_perflog_param_name`
C: `picoquic/performance_log.c:243-277 picoquic_perflog_param_name`
Rust: `rs/fq/src/performance_log.rs:75-105 param_name`

### C body
```c
{
    switch (rank) {
    case picoquic_perflog_is_client: return("is_client");
    case picoquic_perflog_nb_packets_received: return("pkt_recv");
    case picoquic_perflog_nb_trains_sent: return("trains_s");
    case picoquic_perflog_nb_trains_short: return("t_short");
    case picoquic_perflog_nb_trains_blocked_cwin: return("tb_cwin");
    case picoquic_perflog_nb_trains_blocked_pacing: return("tb_pacing");
    case picoquic_perflog_nb_trains_blocked_others: return("tb_others");
    case picoquic_perflog_nb_packets_sent: return("pkt_sent");
    case picoquic_perflog_nb_retransmission_total: return("retrans.");
    case picoquic_perflog_nb_spurious: return("spurious");
    case picoquic_perflog_delayed_ack_option: return("delayed_ack_option");
    case picoquic_perflog_min_ack_delay_remote: return("min_ack_delay_remote");
    case picoquic_perflog_max_ack_delay_remote: return("max_ack_delay_remote");
    case picoquic_perflog_max_ack_gap_remote: return("max_ack_gap_remote");
    case picoquic_perflog_min_ack_delay_local: return("min_ack_delay_local");
    case picoquic_perflog_max_ack_delay_local: return("max_ack_delay_local");
    case picoquic_perflog_max_ack_gap_local: return("max_ack_gap_local");
    case picoquic_perflog_max_mtu_sent: return("max_mtu_sent");
    case picoquic_perflog_max_mtu_received: return("max_mtu_received");
    case picoquic_perflog_zero_rtt: return("zero_rtt");
    case picoquic_perflog_srtt: return("srtt");
    case picoquic_perflog_minrtt: return("minrtt");
    case picoquic_perflog_cwin: return("cwin");
    case picoquic_perflog_ccalgo: return("ccalgo");
    case picoquic_perflog_bwe_max: return("bwe_max");
    case picoquic_perflog_pacing_quantum_max: return("p_quantum");
    case picoquic_perflog_pacing_rate: return("p_rate");
    default:
        break;
    }
    return NULL;
}
```

### Rust body
```rust
    pub fn param_name(self) -> &'static str {
        match self {
            PerflogColumn::IsClient => "is_client",
            PerflogColumn::NbPacketsReceived => "pkt_recv",
            PerflogColumn::NbTrainsSent => "trains_s",
            PerflogColumn::NbTrainsShort => "t_short",
            PerflogColumn::NbTrainsBlockedCwin => "tb_cwin",
            PerflogColumn::NbTrainsBlockedPacing => "tb_pacing",
            PerflogColumn::NbTrainsBlockedOthers => "tb_others",
            PerflogColumn::NbPacketsSent => "pkt_sent",
            PerflogColumn::NbRetransmissionTotal => "retrans.",
            PerflogColumn::NbSpurious => "spurious",
            PerflogColumn::DelayedAckOption => "delayed_ack_option",
            PerflogColumn::MinAckDelayRemote => "min_ack_delay_remote",
            PerflogColumn::MaxAckDelayRemote => "max_ack_delay_remote",
            PerflogColumn::MaxAckGapRemote => "max_ack_gap_remote",
            PerflogColumn::MinAckDelayLocal => "min_ack_delay_local",
            PerflogColumn::MaxAckDelayLocal => "max_ack_delay_local",
            PerflogColumn::MaxAckGapLocal => "max_ack_gap_local",
            PerflogColumn::MaxMtuSent => "max_mtu_sent",
            PerflogColumn::MaxMtuReceived => "max_mtu_received",
            PerflogColumn::ZeroRtt => "zero_rtt",
            PerflogColumn::Srtt => "srtt",
            PerflogColumn::Minrtt => "minrtt",
            PerflogColumn::Cwin => "cwin",
            PerflogColumn::Ccalgo => "ccalgo",
            PerflogColumn::BweMax => "bwe_max",
            PerflogColumn::PacingQuantumMax => "p_quantum",
            PerflogColumn::PacingRate => "p_rate",
        }
    }
```

## Pair `picoquic/picohash.c:picohash_create_ex`
C: `picoquic/picohash.c:30-58 picohash_create_ex`
Rust: `rs/fq/src/hash.rs:193-211 with_seed`

### C body
```c
{
    static const uint8_t null_seed[16] = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 };
    picohash_table* t = (picohash_table*)malloc(sizeof(picohash_table));
    size_t items_length = sizeof(picohash_item*) * nb_bin;
    t->hash_bin = NULL;
    if (t != NULL && (items_length / sizeof(picohash_item*)) == nb_bin) {
        t->hash_bin = (picohash_item**)malloc(sizeof(picohash_item*) * nb_bin);
    }

    if (t->hash_bin == NULL) {
        free(t);
        t = NULL;
    } else {
        (void)memset(t->hash_bin, 0, sizeof(picohash_item*) * nb_bin);
        t->nb_bin = nb_bin;
        t->count = 0;
        t->picohash_hash = picohash_hash;
        t->picohash_compare = picohash_compare;
        t->picohash_key_to_item = picohash_key_to_item;
        t->hash_seed = (hash_seed == NULL)? null_seed: hash_seed;
    }

    return t;
}
```

### Rust body
```rust
    fn hash_key(&self, key: &K) -> u64 {
        let mut h = SeedHasher::new(&self.seed);
        key.hash(&mut h);
        h.finish()
    }
```

## Pair `picoquic/picohash.c:picohash_delete_item`
C: `picoquic/picohash.c:111-141 picohash_delete_item`
Rust: `rs/fq/src/internal.rs:8148-8166 delete_item`

### C body
```c
{
    uint32_t bin = (uint32_t)(item->hash % hash_table->nb_bin);
    picohash_item* previous = hash_table->hash_bin[bin];
    const void* shall_delete = NULL;

    if (previous == item) {
        hash_table->hash_bin[bin] = item->next_in_bin;
        hash_table->count--;
    } else {
        while (previous != NULL) {
            if (previous->next_in_bin == item) {
                previous->next_in_bin = item->next_in_bin;
                hash_table->count--;
                break;
            } else {
                previous = previous->next_in_bin;
            }
        }
    }

    shall_delete = item->key;

    if (hash_table->picohash_key_to_item == NULL) {
        free(item);
    }

    if (delete_key_too) {
        free((void*)shall_delete);
    }
}
```

### Rust body
```rust
    fn delete_item(&mut self, token: SackItemToken) -> Result<(), crate::Error> {
        let (membership, sent_counts) = self
            .sack_items
            .get(token)
            .map(|item| (item.ack_tree_membership, item.nb_times_sent))
            .ok_or(crate::Error::Generic)?;
        for (r, sent) in sent_counts.iter().enumerate() {
            if *sent >= 0 && (*sent as usize) < MAX_ACK_RANGE_REPEAT {
                self.rc[r].range_counts[*sent as usize] -= 1;
            }
        }
        if let Some(membership) = membership {
            self.ack_tree.remove(membership);
        }
        self.sack_items
            .remove(token)
            .map(|_| ())
            .ok_or(crate::Error::Generic)
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_generate_first_byte`
C: `picoquic/picoquic_lb.c:42-52 picoquic_lb_compat_cid_generate_first_byte`
Rust: `rs/fq/src/lb.rs:320-327 set_first_byte`

### C body
```c
{
    if (lb_ctx->first_byte_encodes_length){
        cnx_id_returned->id[0] = ((uint8_t)lb_ctx->rotation_bits << 6) | ((uint8_t)quic->local_cnxid_length - 1);
    }
    else {
        cnx_id_returned->id[0] &= 0x3F;
        cnx_id_returned->id[0] |= ((uint8_t)lb_ctx->rotation_bits << 6);
    }
}
```

### Rust body
```rust
    fn set_first_byte(&self, quic: &Quic, bytes: &mut [u8]) {
        if self.first_byte_encodes_length {
            bytes[0] = (self.rotation_bits as u8) << 6 | (quic.local_connection_id_length - 1);
        } else {
            bytes[0] &= 0x3F;
            bytes[0] |= (self.rotation_bits as u8) << 6;
        }
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_generate_block_cipher`
C: `picoquic/picoquic_lb.c:110-121 picoquic_lb_compat_cid_generate_block_cipher`
Rust: `rs/fq/src/lb.rs:379-386 generate_block_cipher`

### C body
```c
{
    picoquic_lb_compat_cid_generate_first_byte(quic, lb_ctx, cnx_id_returned);
    /* Copy the server ID */
    memcpy(cnx_id_returned->id + 1, lb_ctx->server_id, lb_ctx->server_id_length);
    /* encrypt 16 bytes */
    picoquic_aes128_ecb_encrypt(lb_ctx->cid_encryption_context, cnx_id_returned->id + 1, cnx_id_returned->id + 1, 16);
}
```

### Rust body
```rust
    fn generate_block_cipher(&self, quic: &Quic, bytes: &mut [u8]) {
        self.set_first_byte(quic, bytes);
        bytes[1..1 + self.server_id_length]
            .copy_from_slice(&self.server_id_encoded[..self.server_id_length]);
        let enc = self.cid_encryption_context.as_ref().unwrap();
        let block = <&mut [u8; 16]>::try_from(&mut bytes[1..17]).unwrap();
        enc.process(block);
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_verify_block_cipher`
C: `picoquic/picoquic_lb.c:188-205 picoquic_lb_compat_cid_verify_block_cipher`
Rust: `rs/fq/src/lb.rs:444-457 verify_block_cipher`

### C body
```c
{
    uint8_t decoded[16];
    uint64_t s_id64 = 0;

    /* decrypt 16 bytes */
    picoquic_aes128_ecb_encrypt(lb_ctx->cid_decryption_context, decoded, cnx_id->id + 1, 16);
    /* Decode the server ID */
    if (s_id64 == 0) {
        for (size_t i = 0; i < lb_ctx->server_id_length; i++) {
            s_id64 <<= 8;
            s_id64 += decoded[i];
        }
    }

    return s_id64;
}
```

### Rust body
```rust
    fn verify_block_cipher(&self, cnx_id: &ConnectionId) -> u64 {
        let bytes = cnx_id.as_bytes();
        let mut decoded = [0u8; 16];
        decoded.copy_from_slice(&bytes[1..17]);
        let dec = self.cid_decryption_context.as_ref().unwrap();
        dec.process(&mut decoded);

        let mut s_id64: u64 = 0;
        for b in decoded.iter().take(self.server_id_length) {
            s_id64 <<= 8;
            s_id64 += *b as u64;
        }
        s_id64
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_config_free`
C: `picoquic/picoquic_lb.c:497-515 picoquic_lb_compat_cid_config_free`
Rust: `rs/fq/src/lb.rs:618-627 clear_lb_cid_config`

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

## Pair `picoquic/picoquic_ptls_minicrypto.c:picoquic_clear_minicrypto`
C: `picoquic/picoquic_ptls_minicrypto.c:43-46 picoquic_clear_minicrypto`
Rust: `rs/fq/src/tls_api.rs:2629-2647 clear_minicrypto`

### C body
```c
{
    /* Nothing for now */
}
```

### Rust body
```rust
fn ip_addr_from_stored_bytes(bytes: &[u8]) -> Option<core::net::IpAddr> {
    match bytes.len() {
        0 => Some(core::net::IpAddr::V4(core::net::Ipv4Addr::UNSPECIFIED)),
        4 => Some(core::net::IpAddr::V4(core::net::Ipv4Addr::new(
            bytes[0], bytes[1], bytes[2], bytes[3],
        ))),
        16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(bytes);
            Some(core::net::IpAddr::V6(core::net::Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_clear_openssl`
C: `picoquic/picoquic_ptls_openssl.c:90-108 picoquic_clear_openssl`
Rust: `rs/fq/src/sys/openssl.rs:295-297 clear_openssl`

### C body
```c
{
    if (openssl_is_init) {
#if !defined(LIBRESSL_VERSION_NUMBER) && OPENSSL_VERSION_NUMBER >= 0x30000000L
        if (openssl_default_provider != NULL) {
            (void)OSSL_PROVIDER_unload(openssl_default_provider);
            openssl_default_provider = NULL;
        }
#else
#if !defined(OPENSSL_NO_ENGINE)
        /* Free allocations from engines ENGINEs */
        ENGINE_cleanup();
#endif
        ERR_free_strings();
#endif
        EVP_cleanup();
        openssl_is_init = 0;
    }
}
```

### Rust body
```rust
pub fn clear_openssl() {
    OPENSSL_IS_INIT.store(false, Ordering::SeqCst);
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_dispose_sign_certificate`
C: `picoquic/picoquic_ptls_openssl.c:202-207 picoquic_openssl_dispose_sign_certificate`
Rust: `rs/fq/src/sys/openssl.rs:546-548 picoquic_openssl_dispose_sign_certificate`

### C body
```c
{
    ptls_openssl_dispose_sign_certificate((ptls_openssl_sign_certificate_t*)cert);
}
```

### Rust body
```rust
pub fn picoquic_openssl_dispose_sign_certificate(signer: SignCertificate) {
    drop(signer);
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_get_certificate_verifier`
C: `picoquic/picoquic_ptls_openssl.c:277-292 picoquic_openssl_get_certificate_verifier`
Rust: `rs/fq/src/sys/openssl.rs:419-427 picoquic_openssl_get_certificate_verifier`

### C body
```c
{
    ptls_verify_certificate_t* verify_cert = NULL;
    ptls_openssl_verify_certificate_t* verifier = picoquic_openssl_get_openssl_certificate_verifier(cert_root_file_name,
        is_cert_store_not_empty);

    if (verifier == NULL) {
        free_certificate_verifier_fn = NULL;
    }
    else {
        verify_cert = &verifier->super;
        *free_certificate_verifier_fn = picoquic_openssl_dispose_certificate_verifier;
    }
    return verify_cert;
}
```

### Rust body
```rust
) -> Option<CertificateVerifierRegistration> {
    let verifier = get_openssl_certificate_verifier(cert_root_file_name).ok()?;
    Some(CertificateVerifierRegistration {
        verifier,
        free_certificate_verifier_fn: picoquic_openssl_dispose_certificate_verifier,
    })
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:openssl_keyex_from_key_file`
C: `picoquic/picoquic_ptls_openssl.c:342-365 openssl_keyex_from_key_file`
Rust: `rs/fq/src/sys/openssl.rs:313-318 openssl_keyex_from_key_file`

### C body
```c
{
    int ret = 0;
    BIO* bio = BIO_new_file(keypem, "rb");
    *p_keyex = NULL;
    if (bio == NULL) {
        ret = -1;
    }
    else {
        EVP_PKEY* pkey = PEM_read_bio_PrivateKey(bio, NULL, NULL, NULL);
        if (pkey == NULL) {
            ret = -1;
        }
        else {
            ret = ptls_openssl_create_key_exchange(p_keyex, pkey);
            assert(ret == 0 && "failed to setup private key");
            EVP_PKEY_free(pkey);
        }
        BIO_free(bio);
    }
    return ret;
}
```

### Rust body
```rust
pub fn openssl_keyex_from_key_file(keypem: &str) -> Result<KeyExchangeContext, crate::Error> {
    let pem = std::fs::read(keypem).map_err(|_| crate::Error::NoSuchFile)?;
    let key =
        openssl::pkey::PKey::private_key_from_pem(&pem).map_err(|_| crate::Error::InvalidFile)?;
    Ok(KeyExchangeContext { key })
}
```
