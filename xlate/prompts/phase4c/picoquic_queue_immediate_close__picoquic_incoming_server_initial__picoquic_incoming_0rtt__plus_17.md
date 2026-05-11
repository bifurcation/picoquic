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

## Pair `picoquic/packet.c:picoquic_queue_immediate_close`
C: `picoquic/packet.c:1317-1334 picoquic_queue_immediate_close`
Rust: `rs/fq/src/lib.rs:7360-7363 queue_immediate_close`

### C body
```c
{
    picoquic_stateless_packet_t* sp = picoquic_create_stateless_packet(cnx->quic);

    if (sp != NULL) {
        int ret = picoquic_prepare_packet_ex(cnx, current_time, sp->bytes, PICOQUIC_MAX_PACKET_SIZE,
            &sp->length, &sp->addr_to, &sp->addr_local, &sp->if_index_local, NULL);
        if (ret == 0 && sp->length > 0) {
            picoquic_queue_stateless_packet(cnx->quic, sp);
        }
        else {
            picoquic_delete_stateless_packet(sp);
        }
    }
}
```

### Rust body
```rust
        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };
```

## Pair `picoquic/packet.c:picoquic_incoming_server_initial`
C: `picoquic/packet.c:1619-1701 picoquic_incoming_server_initial`
Rust: `rs/fq/src/lib.rs:3289-3338 incoming_packet_ex`

### C body
```c
{
    int ret = 0;

    if (cnx->cnx_state == picoquic_state_client_init_sent || cnx->cnx_state == picoquic_state_client_init_resent) {
        cnx->cnx_state = picoquic_state_client_handshake_start;
    }

    /* Check the server cnx id */
    if ((!picoquic_is_connection_id_null(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id) || cnx->cnx_state > picoquic_state_client_handshake_start) &&
        picoquic_compare_connection_id(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id, &ph->srce_cnx_id) != 0) {
        ret = PICOQUIC_ERROR_CNXID_CHECK; /* protocol error */
    }

    if (ret == 0) {
        if (cnx->cnx_state <= picoquic_state_client_handshake_start) {
            /* Document local address if not present */
            if (cnx->path[0]->first_tuple->local_addr.ss_family == 0 && addr_to != NULL) {
                picoquic_store_addr(&cnx->path[0]->first_tuple->local_addr, addr_to);
            }
            cnx->path[0]->first_tuple->if_index = if_index_to;
            /* Accept the incoming frames */
            if (ph->payload_length == 0) {
                /* empty payload! */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
            else {
                /* Verify that the packet is long enough */
                if (packet_length < PICOQUIC_ENFORCED_INITIAL_MTU) {
                    size_t byte_index = ph->offset;
                    int ack_needed = 0;
                    int skip_ret = 0;

                    while (skip_ret == 0 && byte_index < ph->offset + ph->payload_length) {
                        size_t frame_length = 0;
                        int frame_is_pure_ack = 0;
                        skip_ret = picoquic_skip_frame(&bytes[byte_index],
                            ph->payload_length - byte_index, &frame_length, &frame_is_pure_ack);
                        byte_index += frame_length;
                        if (frame_is_pure_ack == 0) {
                            ack_needed = 1;
                            break;
                        }
                    }
                    if (ack_needed && cnx->retry_token_length == 0 && cnx->crypto_context[1].aead_encrypt == NULL) {
                        /* perform the test on new paths, but not if resuming an existing path or session */
                        picoquic_log_app_message(cnx, "Server initial too short (%zu bytes)", packet_length);
                        ret = PICOQUIC_ERROR_INITIAL_TOO_SHORT;
                    }
                }

                /* If no error, process the packet */
                if (ret == 0) {
                    ret = picoquic_decode_frames(cnx, cnx->path[0],
                        bytes + ph->offset, ph->payload_length, received_data,
                        ph->epoch, NULL, addr_to, ph->pn64, 0, current_time);
                }
            }
            /* processing of initial packet */
            if (ret == 0) {
                ret = picoquic_tls_stream_process(cnx, NULL, current_time);
            }
        }
        else if (cnx->cnx_state < picoquic_state_ready) {
            /* Require an acknowledgement if the packet contains ackable frames */
            picoquic_ignore_incoming_handshake(cnx, bytes, ph, current_time);
        }
        else {
            /* Initial keys should have been discarded, treat packet as unexpected */
            ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
        }
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

## Pair `picoquic/packet.c:picoquic_incoming_0rtt`
C: `picoquic/packet.c:1835-1877 picoquic_incoming_0rtt`
Rust: `rs/fq/src/lib.rs:7118-7162 incoming_0rtt`

### C body
```c
{
    int ret = 0;

    if (!(picoquic_compare_connection_id(&ph->dest_cnx_id, &cnx->initial_cnxid) == 0 ||
        picoquic_compare_connection_id(&ph->dest_cnx_id, &cnx->path[0]->first_tuple->p_local_cnxid->cnx_id) == 0) ||
        picoquic_compare_connection_id(&ph->srce_cnx_id, &cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id) != 0) {
        ret = PICOQUIC_ERROR_CNXID_CHECK;
    } else if (cnx->cnx_state == picoquic_state_server_almost_ready || 
        cnx->cnx_state == picoquic_state_server_false_start ||
        (cnx->cnx_state == picoquic_state_ready && !cnx->is_1rtt_received)) {
        if (ph->vn != picoquic_supported_versions[cnx->version_index].version) {
            ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
        } else {
            /* Accept the incoming frames */
            if (ph->payload_length == 0) {
                /* empty payload! */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
            else {
                cnx->nb_zero_rtt_received++;
                ret = picoquic_decode_frames(cnx, cnx->path[0],
                    bytes + ph->offset, ph->payload_length, received_data,
                    ph->epoch, NULL, NULL, ph->pn64, 0, current_time);
            }

            if (ret == 0) {
                /* Processing of TLS messages -- EOED */
                ret = picoquic_tls_stream_process(cnx, NULL, current_time);
            }
        }
    } else {
        /* Not expected. Log and ignore. */
        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let dest_matches = ph.dest_connection_id == self.initial_connection_id
            || self.path_local_connection_id(0) == Some(ph.dest_connection_id);
        let src_matches = self.path_remote_connection_id(0) == Some(ph.src_connection_id);

        if !dest_matches || !src_matches {
            return InternalError::CnxidCheck as i32;
        }

        if self.connection_state == State::ServerAlmostReady
            || self.connection_state == State::ServerFalseStart
            || (self.connection_state == State::Ready && !self.is_1rtt_received)
        {
            if ph.version != self.negotiated_version() || ph.payload_length == 0 {
                self.connection_error(TransportError::ProtocolViolation as u64, 0)
            } else {
                self.nb_zero_rtt_received = self.nb_zero_rtt_received.saturating_add(1);
                let payload = Self::packet_payload(bytes, ph);
                let mut ret = self.decode_frames_on_path(
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
                    let (tls_ret, _) = self.process_tls_stream_status(current_time);
                    ret = tls_ret;
                }
                ret
            }
        } else {
            InternalError::UnexpectedPacket as i32
        }
    }
```

## Pair `picoquic/packet.c:picoquic_incoming_segment`
C: `picoquic/packet.c:2073-2387 picoquic_incoming_segment`
Rust: `rs/fq/src/lib.rs:3289-3338 incoming_packet_ex`

### C body
```c
{
    int ret = 0;
    picoquic_cnx_t* cnx = NULL;
    picoquic_packet_header ph;
    int new_context_created = 0;
    int is_first_segment = 0;
    int is_buffered = 0;
    int path_id = -1;
    int path_is_not_allocated = 0;
    uint8_t* bytes = NULL;
    picoquic_stream_data_node_t* decrypted_data = picoquic_stream_data_node_alloc(quic);

    if (decrypted_data == NULL) {
        return -1;
    }
    /* Parse the header and decrypt the segment */
    ret = picoquic_parse_header_and_decrypt(quic, raw_bytes, length, packet_length, addr_from,
        current_time, decrypted_data, &ph, &cnx, consumed, &new_context_created);
    bytes = decrypted_data->data;

    if (ret == 0 && cnx != NULL) {
        if (ph.ptype == picoquic_packet_1rtt_protected) {
            /* Find the arrival path and update its state */
            ret = picoquic_find_incoming_path(cnx, &ph, addr_from, addr_to, if_index_to, current_time, &path_id);
        }
        else {
            path_id = 0;
        }
    }

    /* Verify that the segment coalescing is for the same destination ID */
    if (picoquic_is_connection_id_null(previous_dest_id)) {
        /* This is the first segment in the incoming packet */
        *previous_dest_id = ph.dest_cnx_id;
        is_first_segment = 1;
        *first_cnx = cnx;


        /* if needed, log that the packet is received */
        if (cnx != NULL) {
            picoquic_log_pdu(cnx, 1, current_time, addr_from, addr_to, packet_length,
                (path_id >= 0) ? cnx->path[path_id]->unique_path_id : 0, received_ecn);
        }
        else {
            picoquic_log_quic_pdu(quic, 1, current_time, picoquic_val64_connection_id(ph.dest_cnx_id),
                addr_from, addr_to, packet_length);
        }
    }
    else {
        if (ret == 0 && picoquic_compare_connection_id(previous_dest_id, &ph.dest_cnx_id) != 0) {
            ret = PICOQUIC_ERROR_CNXID_SEGMENT;
        }
        else if (ret == PICOQUIC_ERROR_VERSION_NOT_SUPPORTED) {
            /* A coalesced packet with unknown version is likely some kind of padding */
            ret = PICOQUIC_ERROR_CNXID_SEGMENT;
        }

        if (ret == PICOQUIC_ERROR_CNXID_SEGMENT && *first_cnx != cnx && *first_cnx != NULL) {
            /* Log the drop segment information in the context of the first connection */
            picoquic_log_dropped_packet(*first_cnx, NULL, &ph, length, PICOQUIC_ERROR_PADDING_PACKET, bytes, current_time);
        }
    }

    /* Store packet if received in advance of encryption keys */
    if (ret == PICOQUIC_ERROR_AEAD_NOT_READY &&
        cnx != NULL) {
        is_buffered = picoquic_incoming_not_decrypted(cnx, &ph, current_time, raw_bytes, length, addr_from, addr_to, if_index_to, received_ecn);
    }

    /* Find the path and if required log the incoming packet */
    if (cnx != NULL) {
        if (ret == 0 && ph.ptype == picoquic_packet_1rtt_protected) {
            if (ph.payload_length == 0) {
                /* empty payload! */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
            else if (ph.has_reserved_bit_set) {
                /* Reserved bits were not set to zero */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
        }

        if (ret == 0) {
            picoquic_log_packet(cnx, (path_id < 0)?NULL:cnx->path[path_id], 1, current_time, &ph, bytes, *consumed);
        }
        else if (is_buffered) {
            picoquic_log_buffered_packet(cnx, (path_id < 0) ? NULL : cnx->path[path_id], ph.ptype, current_time);
        } else {
            picoquic_log_dropped_packet(cnx, (path_id < 0) ? NULL : cnx->path[path_id], &ph, length, ret, bytes, current_time);
        }
    }

    if (ret == PICOQUIC_ERROR_VERSION_NOT_SUPPORTED) {
        /* use the result of parsing to consider version negotiation,
        * but block reflection attacks towards protected ports. */
        if (packet_length >= PICOQUIC_ENFORCED_INITIAL_MTU){
            if (quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from)) {
                picoquic_prepare_version_negotiation(quic, addr_from, addr_to, if_index_to, &ph, raw_bytes);
            }
        }
    } else if (ret == PICOQUIC_ERROR_RETRY_NEEDED) {
        /* Incoming packet could not be processed, need to send a Retry. */
        if (packet_length >= PICOQUIC_ENFORCED_INITIAL_MTU){
            if (quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from)) {
                picoquic_queue_retry_packet(quic, addr_from, addr_to, if_index_to, &ph, current_time);
            }
        }
    } else if (ret == PICOQUIC_ERROR_SERVER_BUSY) {
        /* Incoming packet could not be processed, need to send a Retry. */
        if (packet_length >= PICOQUIC_ENFORCED_INITIAL_MTU){
            if (quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from)) {
                picoquic_queue_busy_packet(quic, addr_from, addr_to, if_index_to, &ph);
            }
        }
    } else if (ret == 0) {
        if (cnx == NULL) {
            /* Unexpected packet. Reject, drop and log. */
            if (!picoquic_is_connection_id_null(&ph.dest_cnx_id) &&
                (quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from))) {
                picoquic_process_unexpected_cnxid(quic, length, addr_from, addr_to, if_index_to, &ph, current_time);
            }
            ret = PICOQUIC_ERROR_DETECTED;
        }
        else {
            cnx->quic_bit_received_0 |= ph.quic_bit_is_zero;
            switch (ph.ptype) {
            case picoquic_packet_version_negotiation:
                ret = picoquic_incoming_version_negotiation(
                    cnx, bytes, length, addr_from, &ph, current_time);
                break;
            case picoquic_packet_initial:
                /* Initial packet: either crypto handshakes or acks. */
                if (ph.has_reserved_bit_set) {
                    ret = PICOQUIC_ERROR_PACKET_HEADER_PARSING;
                } else if ((!cnx->client_mode && picoquic_compare_connection_id(&ph.dest_cnx_id, &cnx->initial_cnxid) == 0) ||
                    picoquic_compare_connection_id(&ph.dest_cnx_id, &cnx->path[0]->first_tuple->p_local_cnxid->cnx_id) == 0) {
                    /* Verify that the source CID matches expectation */
                    if (picoquic_is_connection_id_null(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id)) {
                        cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id = ph.srce_cnx_id;
                    } else if (picoquic_compare_connection_id(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id, &ph.srce_cnx_id) != 0) {
                        DBG_PRINTF("Error wrong srce cnxid (%d), type: %d, epoch: %d, pc: %d, pn: %d\n",
                            cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int)ph.pn);
                        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
                    }
                    if (ret == 0) {
                        if (packet_length < PICOQUIC_ENFORCED_INITIAL_MTU) {
                            if (!cnx->did_receive_short_initial) {
                                picoquic_log_app_message(cnx, "Received unpadded initial, length=%zu", packet_length);
                            }
                            cnx->did_receive_short_initial = 1;
                        }
                        if (cnx->client_mode == 0) {
                            if (is_first_segment) {
                                /* Account for the data received in handshake, but only
                                 * count the packet once. Do not count it again if it is not
                                 * the first segment in packet */
                                cnx->initial_data_received += packet_length;
                            }
                            ret = picoquic_incoming_client_initial(&cnx, bytes, packet_length, decrypted_data,
                                addr_from, addr_to, if_index_to, &ph, current_time, new_context_created);
                            /* Reset the value of first_cnx, as the context may have been deleted */
                            *first_cnx = cnx;
                        }
                        else {
                            /* TODO: this really depends on the current receive epoch */
                            ret = picoquic_incoming_server_initial(cnx, bytes, packet_length,
                                decrypted_data, addr_to, if_index_to, &ph, current_time);
                        }
                    }
                } else {
                    DBG_PRINTF("Error detected (%d), type: %d, epoch: %d, pc: %d, pn: %d\n",
                        cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int)ph.pn);
                    ret = PICOQUIC_ERROR_DETECTED;
                }
                break;
            case picoquic_packet_retry:
                ret = picoquic_incoming_retry(cnx, raw_bytes, &ph, current_time);
                break;
            case picoquic_packet_handshake:
                if (ph.has_reserved_bit_set) {
                    ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
                }
                else if (ph.has_reserved_bit_set) {
                    ret = PICOQUIC_ERROR_PACKET_HEADER_PARSING;
                }
                else if (cnx->client_mode)
                {
                    ret = picoquic_incoming_server_handshake(cnx, bytes, decrypted_data, addr_to, if_index_to, &ph, current_time);
                }
                else
                {
                    ret = picoquic_incoming_client_handshake(cnx, bytes, decrypted_data, &ph, current_time);
                }
                break;
            case picoquic_packet_0rtt_protected:
                if (ph.has_reserved_bit_set) {
                    ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
                }
                else {
                    if (is_first_segment) {
                        /* Account for the data received in handshake, but only
                         * count the packet once. Do not count it again if it is not
                         * the first segment in packet */
                        cnx->initial_data_received += packet_length;
                    }
                    ret = picoquic_incoming_0rtt(cnx, bytes, decrypted_data, &ph, current_time);
                }
                break;
            case picoquic_packet_1rtt_protected:
                ret = picoquic_incoming_1rtt(cnx, path_id, bytes, decrypted_data,
                    &ph, addr_from, addr_to, if_index_to,
                    path_is_not_allocated, current_time);
                break;
            default:
                /* Packet type error. Log and ignore */
                DBG_PRINTF("Unexpected packet type (%d), type: %d, epoch: %d, pc: %d, pn: %d\n",
                    cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int) ph.pn);
                ret = PICOQUIC_ERROR_DETECTED;
                break;
            }
        }
    } else if (ret == PICOQUIC_ERROR_STATELESS_RESET) {
        ret = picoquic_incoming_stateless_reset(cnx);
    }
    else if (ret == PICOQUIC_ERROR_AEAD_CHECK &&
        ph.ptype == picoquic_packet_handshake &&
        cnx != NULL &&
        (cnx->cnx_state == picoquic_state_client_init_sent || cnx->cnx_state == picoquic_state_client_init_resent))
    {
        /* Indicates that the server probably sent initial and handshake but initial was lost */
        if (cnx->pkt_ctx[picoquic_packet_context_initial].pending_first != NULL &&
            cnx->path[0]->nb_retransmit == 0) {
            /* Reset the retransmit timer to start retransmission immediately */
            cnx->path[0]->retransmit_timer = current_time -
                cnx->pkt_ctx[picoquic_packet_context_initial].pending_first->send_time;
        }
    }

    if (ret == 0) {
        if (cnx != NULL && cnx->cnx_state != picoquic_state_disconnected &&
            ph.ptype != picoquic_packet_version_negotiation) {
            cnx->nb_packets_received++;
            cnx->latest_receive_time = current_time;
            /* Mark the sequence number as received */
            ret = picoquic_record_pn_received(cnx, ph.pc, ph.l_cid, ph.pn64, receive_time);
            /* Perform ECN accounting */
            picoquic_ecn_accounting(cnx, received_ecn, ph.pc, ph.l_cid);
        }
        if (cnx != NULL) {
            picoquic_reinsert_by_wake_time(cnx->quic, cnx, current_time);
        }
    } else if (ret == PICOQUIC_ERROR_AEAD_CHECK || ret == PICOQUIC_ERROR_INITIAL_TOO_SHORT ||
        ret == PICOQUIC_ERROR_PACKET_WRONG_VERSION ||
        ret == PICOQUIC_ERROR_INITIAL_CID_TOO_SHORT ||
        ret == PICOQUIC_ERROR_PORT_BLOCKED ||
        ret == PICOQUIC_ERROR_UNEXPECTED_PACKET || 
        ret == PICOQUIC_ERROR_CNXID_CHECK || 
        ret == PICOQUIC_ERROR_RETRY || ret == PICOQUIC_ERROR_DETECTED ||
        ret == PICOQUIC_ERROR_SERVER_BUSY ||
        ret == PICOQUIC_ERROR_CONNECTION_DELETED ||
        ret == PICOQUIC_ERROR_CNXID_SEGMENT ||
        ret == PICOQUIC_ERROR_VERSION_NOT_SUPPORTED ||
        ret == PICOQUIC_ERROR_PACKET_TOO_LONG ||
        ret == PICOQUIC_ERROR_DUPLICATE ||
        ret == PICOQUIC_ERROR_AEAD_NOT_READY ||
        ret == PICOQUIC_ERROR_REDIRECTED) {
        /* Bad packets are dropped silently */
        if (ret == PICOQUIC_ERROR_AEAD_CHECK ||
            ret == PICOQUIC_ERROR_PACKET_WRONG_VERSION ||
            ret == PICOQUIC_ERROR_AEAD_NOT_READY ||
            ret == PICOQUIC_ERROR_PACKET_TOO_LONG ||
            ret == PICOQUIC_ERROR_VERSION_NOT_SUPPORTED ||
            ret == PICOQUIC_ERROR_RETRY ||
            ret == PICOQUIC_ERROR_SERVER_BUSY ||
            ret == PICOQUIC_ERROR_REDIRECTED) {
            ret = 0;
        }
        else {
            ret = -1;
        }
        if (cnx != NULL) {
            picoquic_reinsert_by_wake_time(cnx->quic, cnx, current_time);
        }
    } else if (ret == 1) {
        /* wonder what happened ! */
        DBG_PRINTF("Packet (%d) get ret=1, t: %d, e: %d, pc: %d, pn: %d, l: %zu\n",
            (cnx == NULL) ? -1 : cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int)ph.pn, length);
        ret = -1;
    }
    else if (ret != 0) {
        DBG_PRINTF("Packet (%d) error, t: %d, e: %d, pc: %d, pn: %d, l: %zu, ret : 0x%x\n",
            (cnx == NULL) ? -1 : cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int)ph.pn, length, ret);
        ret = -1;
    }

    if (decrypted_data != NULL && decrypted_data->bytes == NULL) {
        picoquic_stream_data_node_recycle(decrypted_data);
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

## Pair `picoquic/packet_names.c:picoquic_packet_type_name`
C: `picoquic/packet_names.c:25-46 picoquic_packet_type_name`
Rust: `rs/fq/src/tests/picolog.rs:175-243 packet_type_name`

### C body
```c
{
    switch (ptype) {
    case picoquic_packet_error:
        return "error";
    case picoquic_packet_version_negotiation:
        return "version_negotiation";
    case picoquic_packet_initial:
        return "initial";
    case picoquic_packet_retry:
        return "retry";
    case picoquic_packet_handshake:
        return "handshake";
    case picoquic_packet_0rtt_protected:
        return "0RTT";
    case picoquic_packet_1rtt_protected:
        return "1RTT";
    case picoquic_packet_type_max:
    default:
        return "unknown";
    }
}
```

### Rust body
```rust
) -> crate::Result<()> {
    let mut packet_count = 0usize;
    fileread_binlog(f_binlog, |s| {
        let cid = s.read_cid()?;
        if &cid != cid_filter {
            return Ok(());
        }

        let time = s.read_varint()?;
        let path_id = s.read_varint()?;
        let event_id = s.read_varint()?;

        match event_id {
            0x0010 => {
                let _client_mode = s.read_u8()?;
                let _proposed_version = s.read_u32()?;
                let _remote_cnxid = s.read_cid()?;
            }
            0x0008 | 0x0009 => {
                let received = event_id == 0x0009;
                let packet_length = s.read_varint()?;
                let ph = read_packet_header(s)?;
                svg_packet_start(
                    out,
                    &mut packet_count,
                    time,
                    path_id,
                    packet_length,
                    &ph,
                    received,
                )
                .map_err(|_| crate::Error::Generic)?;

                while s.remaining() > 0 {
                    let len = s.read_vlen()?;
                    if s.remaining() < len {
                        return Err(crate::Error::BufferTooSmall);
                    }
                    let mut frame_bytes = s.tail()[..len].to_vec();
                    s.skip(len)?;
                    let mut frame = ByteStream::from_slice(&mut frame_bytes);
                    svg_packet_frame(out, &mut frame).map_err(|_| crate::Error::Generic)?;
                }

                writeln!(out, "</text>").map_err(|_| crate::Error::Generic)?;
            }
            _ => {}
        }

        Ok(())
    })
}
```

## Pair `picoquic/paths.c:picoquic_tuple_challenge_time`
C: `picoquic/paths.c:235-257 picoquic_tuple_challenge_time`
Rust: `rs/fq/src/internal.rs:16443-16461 tuple_challenge_time`

### C body
```c
{
    /* "Challenge time" holds the time at which the last challenge was set. We
     * use the value to compute an estimate of the RTT */
    uint64_t next_challenge_time = tuple->challenge_time;

    if (tuple->challenge_repeat_count == 0) {
        next_challenge_time = current_time;
    }
    else {
        if (tuple->challenge_repeat_count >= 2) {
            next_challenge_time += path_x->retransmit_timer << (tuple->challenge_repeat_count - 1);
        }
        else {
            next_challenge_time += PICOQUIC_INITIAL_RETRANSMIT_TIMER;
        }
    }

    return next_challenge_time;
}
```

### Rust body
```rust
pub fn tuple_challenge_time(path: &Path, tuple: &Tuple, current_time: Instant) -> Instant {
    if tuple.challenge_repeat_count == 0 {
        current_time
    } else if tuple.challenge_repeat_count >= 2 {
        let extra = path
            .retransmit_timer
            .ticks()
            .checked_shl(u32::from(tuple.challenge_repeat_count) - 1)
            .unwrap_or(u64::MAX);
        Instant::from_ticks(tuple.challenge_time.ticks().saturating_add(extra))
    } else {
        Instant::from_ticks(
            tuple
                .challenge_time
                .ticks()
                .saturating_add(INITIAL_RETRANSMIT_TIMER.ticks()),
        )
    }
}
```

## Pair `picoquic/paths.c:picoquic_sort_available_paths`
C: `picoquic/paths.c:379-486 picoquic_sort_available_paths`
Rust: `rs/fq/src/internal.rs:16551-16669 picoquic_sort_available_paths`

### C body
```c
{
    int data_path_cwin = -1;
    int data_path_pacing = -1;
    uint64_t pacing_time_next = UINT64_MAX;
    uint64_t last_sent_pacing = UINT64_MAX;
    uint64_t last_sent_cwin = UINT64_MAX;
    int i_min_rtt = -1;
    int is_min_rtt_pacing_ok = 0;
    int is_ack_needed = 0;
    picoquic_stream_head_t* next_stream = picoquic_find_ready_stream(cnx);
    int affinity_path_id = -1;

    /* Several paths are available. We will chose from that.
     */
    for (int path_index = 0; path_index < cnx->nb_paths; path_index++) {
        picoquic_path_t* path_x = cnx->path[path_index];
        /* Clear the nominal ack path flag from all path -- it will be reset to the low RTT path later */
        path_x->is_nominal_ack_path = 0;
        /* Only continue processing if the path is available */
        if (path_x->path_is_backup || !path_x->first_tuple->challenge_verified || path_x->path_is_demoted || path_x->nb_retransmit > min_retransmit) {
            continue;
        }
        /* This path is a candidate for min rtt */
        if (i_min_rtt < 0 ||
            path_x->nb_retransmit < cnx->path[i_min_rtt]->nb_retransmit ||
            (path_x->nb_retransmit == cnx->path[i_min_rtt]->nb_retransmit &&
                path_x->rtt_min < cnx->path[i_min_rtt]->rtt_min)) {
            i_min_rtt = path_index;
            is_min_rtt_pacing_ok = 0;
        }
        path_x->polled++;
        /* Find the best path authorized by pacing and then by congestion control,
         * taking into account affinity, datagrams, etc.
         */
        if (picoquic_is_sending_authorized_by_pacing(cnx, path_x, current_time, &pacing_time_next)) {
            if (path_x->last_sent_time < last_sent_pacing) {
                last_sent_pacing = path_x->last_sent_time;
                data_path_pacing = path_index;
                if (path_index == i_min_rtt) {
                    is_min_rtt_pacing_ok = 1;
                }
            }
            if (path_x->bytes_in_transit < path_x->cwin &&
                path_x->bytes_in_transit < cnx->quic->cwin_max) {
                if (path_x->last_sent_time < last_sent_cwin) {
                    last_sent_cwin = path_x->last_sent_time;
                    data_path_cwin = path_index;
                }
                if (affinity_path_id < 0) {
                    /* we select here the first path that is either ready to send on
                        * the highest priority stream with affinity on this path, or
                        * ready to send datagrams on this path. */
                    if (next_stream != NULL && path_x == next_stream->affinity_path) {
                        affinity_path_id = path_index;
                    }
                    else if (path_x->is_datagram_ready || cnx->is_datagram_ready) {
                        affinity_path_id = path_index;
                    }
                }
            }
            else {
                path_x->congested++;
            }
        }
        else {
            path_x->paced++;
        }
    }
    /* Putting it all together:
     * - fix the nominal ACK path that we erased.
     * - retain the ACK
     */

    if (i_min_rtt >= 0) {
        is_ack_needed = picoquic_is_ack_needed(cnx, current_time, next_wake_time, 0, 0);
        cnx->path[i_min_rtt]->is_nominal_ack_path = 1;
    }

    if (is_ack_needed && is_min_rtt_pacing_ok) {
        *next_path = cnx->path[i_min_rtt];
    }
    else if (data_path_cwin >= 0) {
        /* if there is a path ready to send the most urgent data, select it */
        if (affinity_path_id >= 0) {
            *next_path = cnx->path[affinity_path_id];
        }
        else {
            *next_path = cnx->path[data_path_cwin];
        }
    }
    else if (data_path_pacing >= 0) {
        *next_path = cnx->path[data_path_pacing];
    }
    else {
        /* No path is ready at all. Set the next wake time to the min of current
         * value and next pacing time.
         */
        if (pacing_time_next < *next_wake_time) {
            *next_wake_time = pacing_time_next;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        *next_path = cnx->path[0];
    }
    (*next_path)->selected++;
    *next_tuple = (*next_path)->first_tuple;
}
```

### Rust body
```rust
    ) -> Option<(PathToken, usize)> {
        if self.paths.is_empty() {
            return None;
        }

        let mut data_path_cwin: Option<usize> = None;
        let mut data_path_pacing: Option<usize> = None;
        let mut pacing_time_next = Instant::from_ticks(u64::MAX);
        let mut last_sent_pacing = Instant::from_ticks(u64::MAX);
        let mut last_sent_cwin = Instant::from_ticks(u64::MAX);
        let mut i_min_rtt: Option<usize> = None;
        let mut is_min_rtt_pacing_ok = false;
        let next_stream = self.find_ready_stream();
        let mut affinity_path_id: Option<usize> = None;
        let cwin_max = self.quic_cwin_max();

        for path_index in 0..self.paths.len() {
            {
                let path_x = &mut self.paths[path_index];
                path_x.is_nominal_ack_path = false;
                let challenge_verified = path_x
                    .tuples
                    .first()
                    .is_some_and(|tuple| tuple.challenge_verified);
                if path_x.path_is_backup
                    || !challenge_verified
                    || path_x.path_is_demoted
                    || path_x.nb_retransmit > min_retransmit
                {
                    continue;
                }
            }

            let replace_min_rtt = match i_min_rtt {
                None => true,
                Some(min_idx) => {
                    self.paths[path_index].nb_retransmit < self.paths[min_idx].nb_retransmit
                        || (self.paths[path_index].nb_retransmit
                            == self.paths[min_idx].nb_retransmit
                            && self.paths[path_index].rtt_min < self.paths[min_idx].rtt_min)
                }
            };
            if replace_min_rtt {
                i_min_rtt = Some(path_index);
                is_min_rtt_pacing_ok = false;
            }

            self.paths[path_index].polled = self.paths[path_index].polled.saturating_add(1);

            if self.is_sending_authorized_by_pacing(path_index, current_time, &mut pacing_time_next)
            {
                if self.paths[path_index].last_sent_time < last_sent_pacing {
                    last_sent_pacing = self.paths[path_index].last_sent_time;
                    data_path_pacing = Some(path_index);
                    if Some(path_index) == i_min_rtt {
                        is_min_rtt_pacing_ok = true;
                    }
                }
                if self.paths[path_index].bytes_in_transit < self.paths[path_index].cwin
                    && self.paths[path_index].bytes_in_transit < cwin_max
                {
                    if self.paths[path_index].last_sent_time < last_sent_cwin {
                        last_sent_cwin = self.paths[path_index].last_sent_time;
                        data_path_cwin = Some(path_index);
                    }
                    if affinity_path_id.is_none() {
                        let path_token = PathToken::synthetic(path_index as u32, path_index as u32);
                        let stream_affinity_matches = next_stream
                            .and_then(|stream| self.streams.get(stream))
                            .and_then(|stream| stream.affinity_path)
                            .is_some_and(|affinity| affinity == path_token);
                        if stream_affinity_matches
                            || self.paths[path_index].is_datagram_ready
                            || self.is_datagram_ready
                        {
                            affinity_path_id = Some(path_index);
                        }
                    }
                } else {
                    self.paths[path_index].congested =
                        self.paths[path_index].congested.saturating_add(1);
                }
            } else {
                self.paths[path_index].paced = self.paths[path_index].paced.saturating_add(1);
            }
        }

        let mut is_ack_needed = false;
        if let Some(min_idx) = i_min_rtt {
            is_ack_needed =
                self.is_ack_needed(current_time, next_wake_time, PacketContext::Application, 0);
            self.paths[min_idx].is_nominal_ack_path = true;
        }

        let selected = if is_ack_needed && is_min_rtt_pacing_ok {
            i_min_rtt
        } else if let Some(cwin_idx) = data_path_cwin {
            Some(affinity_path_id.unwrap_or(cwin_idx))
        } else if let Some(pacing_idx) = data_path_pacing {
            Some(pacing_idx)
        } else {
            if pacing_time_next < *next_wake_time {
                *next_wake_time = pacing_time_next;
            }
            Some(0)
        }?;

        self.paths[selected].selected = self.paths[selected].selected.saturating_add(1);
        if self.paths[selected].tuples.is_empty() {
            None
        } else {
            Some((PathToken::synthetic(selected as u32, selected as u32), 0))
        }
    }
```

## Pair `picoquic/performance_log.c:picoquic_perflog_record`
C: `picoquic/performance_log.c:127-210 picoquic_perflog_record`
Rust: `rs/fq/src/performance_log.rs:212-282 record`

### C body
```c
{
    int ret = 0;
    picoquic_performance_log_item_t* perflog_item = (picoquic_performance_log_item_t*)
        malloc(sizeof(picoquic_performance_log_item_t));

    if (perflog_item == NULL) {
        ret = -1;
    }
    else {
        uint64_t start_time = picoquic_get_cnx_start_time(cnx);
        uint64_t close_time = picoquic_get_quic_time(cnx->quic);
        uint64_t duration_usec = close_time - start_time;
        memset(perflog_item, 0, sizeof(picoquic_performance_log_item_t));
        /* Compute the key performance metrics */
        perflog_item->duration_sec = ((double)duration_usec) / 1000000.0;
        if (perflog_item->duration_sec > 0) {
            perflog_item->data_sent = picoquic_get_data_sent(cnx);
            perflog_item->data_received = picoquic_get_data_received(cnx);
            perflog_item->send_mbps = ((double)perflog_item->data_sent) * 8.0 / ((double)duration_usec);
            perflog_item->recv_mbps = ((double)perflog_item->data_received) * 8.0 / ((double)duration_usec);
            /* TODO: nb streams.
            printf("Nb_transactions: %" PRIu64"\n", quicperf_ctx->nb_streams);
            printf("TPS: %f\n", ((double)quicperf_ctx->nb_streams) / duration_sec);
            */
        }
        /* Store identification data */
        perflog_item->alpn = picoquic_string_duplicate(cnx->alpn);
        perflog_item->quic_version = (cnx->version_index >= 0) ?
            picoquic_supported_versions[cnx->version_index].version : 0;
        perflog_item->cnxid = picoquic_get_logging_cnxid(cnx);
        perflog_item->cnx_time_64 = start_time;
        /* Store additional parameters */
        perflog_item->nb_values = PICOQUIC_PERF_LOG_MAX_ITEMS;
        perflog_item->v[picoquic_perflog_is_client] = cnx->client_mode;
        perflog_item->v[picoquic_perflog_nb_packets_received] = cnx->nb_packets_received;
        perflog_item->v[picoquic_perflog_nb_trains_sent] = cnx->nb_trains_sent;
        perflog_item->v[picoquic_perflog_nb_trains_short] = cnx->nb_trains_short;
        perflog_item->v[picoquic_perflog_nb_trains_blocked_cwin] = cnx->nb_trains_blocked_cwin;
        perflog_item->v[picoquic_perflog_nb_trains_blocked_pacing] = cnx->nb_trains_blocked_pacing;
        perflog_item->v[picoquic_perflog_nb_trains_blocked_others] = cnx->nb_trains_blocked_others;
        perflog_item->v[picoquic_perflog_nb_packets_sent] = cnx->nb_packets_sent;
        perflog_item->v[picoquic_perflog_nb_retransmission_total] = cnx->nb_retransmission_total;
        perflog_item->v[picoquic_perflog_nb_spurious] = cnx->nb_spurious;
        perflog_item->v[picoquic_perflog_delayed_ack_option] = cnx->is_ack_frequency_negotiated;
        perflog_item->v[picoquic_perflog_min_ack_delay_remote] = cnx->min_ack_delay_remote;
        perflog_item->v[picoquic_perflog_max_ack_delay_remote] = cnx->max_ack_delay_remote;
        perflog_item->v[picoquic_perflog_max_ack_gap_remote] = cnx->max_ack_gap_remote;
        perflog_item->v[picoquic_perflog_min_ack_delay_local] = cnx->min_ack_delay_local;
        perflog_item->v[picoquic_perflog_max_ack_delay_local] = cnx->max_ack_delay_local;
        perflog_item->v[picoquic_perflog_max_ack_gap_local] = cnx->max_ack_gap_local;
        perflog_item->v[picoquic_perflog_max_mtu_sent] = cnx->max_mtu_sent;
        perflog_item->v[picoquic_perflog_max_mtu_received] = cnx->max_mtu_received;
        perflog_item->v[picoquic_perflog_zero_rtt] = (cnx->nb_zero_rtt_received > 0) || (cnx->nb_zero_rtt_acked > 0);
        if (cnx->path != NULL && cnx->path[0] != NULL) {
            perflog_item->v[picoquic_perflog_srtt] = cnx->path[0]->smoothed_rtt;
            perflog_item->v[picoquic_perflog_minrtt] = cnx->path[0]->rtt_min;
            perflog_item->v[picoquic_perflog_cwin] = cnx->path[0]->cwin;
            perflog_item->v[picoquic_perflog_bwe_max] = cnx->path[0]->bandwidth_estimate_max;
            perflog_item->v[picoquic_perflog_pacing_quantum_max] = cnx->path[0]->pacing.quantum_max;
            perflog_item->v[picoquic_perflog_pacing_rate] = cnx->path[0]->pacing.rate_max;
        }
        if (cnx->congestion_alg != NULL) {
            perflog_item->v[picoquic_perflog_ccalgo] = cnx->congestion_alg->congestion_algorithm_number;
        }
        
        if (perflog_ctx->first == NULL) {
            perflog_ctx->first = perflog_item;
        }

        if (perflog_ctx->last == NULL) {
            perflog_ctx->last = perflog_item;
        }
        else {
            perflog_ctx->last->next = perflog_item;
        }

        if (cnx->quic->cnx_list == cnx && cnx->quic->cnx_last == cnx) {
            ret = picoquic_perflog_save(perflog_ctx);
        }
    }

    return ret;
}
```

### Rust body
```rust
    fn record(&mut self, connection: &Connection) {
        let start_time = connection.start_time.ticks();
        let close_time = connection.quic_time().ticks();
        let duration_usec = close_time.saturating_sub(start_time);
        let duration_sec = (duration_usec as f64) / 1_000_000.0;

        let mut item = PerflogItem {
            duration_sec,
            send_mbps: 0.0,
            recv_mbps: 0.0,
            data_sent: 0,
            data_received: 0,
            quic_version: 0,
            alpn: None,
            cnxid: connection.logging_connection_id(),
            cnx_time_64: start_time,
            v: [0; PERF_LOG_MAX_ITEMS],
        };

        if duration_usec > 0 {
            item.data_sent = connection.data_sent;
            item.data_received = connection.data_received;
            item.send_mbps = (connection.data_sent as f64) * 8.0 / (duration_usec as f64);
            item.recv_mbps = (connection.data_received as f64) * 8.0 / (duration_usec as f64);
        }

        item.alpn = connection.alpn.clone();
        item.quic_version = if connection.version_index >= 0 {
            connection.version_number()
        } else {
            0
        };

        let v = &mut item.v;
        v[PerflogColumn::IsClient as usize] = connection.client_mode as u64;
        v[PerflogColumn::NbPacketsReceived as usize] = connection.nb_packets_received;
        v[PerflogColumn::NbTrainsSent as usize] = connection.nb_trains_sent;
        v[PerflogColumn::NbTrainsShort as usize] = connection.nb_trains_short;
        v[PerflogColumn::NbTrainsBlockedCwin as usize] = connection.nb_trains_blocked_cwin;
        v[PerflogColumn::NbTrainsBlockedPacing as usize] = connection.nb_trains_blocked_pacing;
        v[PerflogColumn::NbTrainsBlockedOthers as usize] = connection.nb_trains_blocked_others;
        v[PerflogColumn::NbPacketsSent as usize] = connection.nb_packets_sent;
        v[PerflogColumn::NbRetransmissionTotal as usize] = connection.nb_retransmission_total;
        v[PerflogColumn::NbSpurious as usize] = connection.nb_spurious;
        v[PerflogColumn::DelayedAckOption as usize] = connection.is_ack_frequency_negotiated as u64;
        v[PerflogColumn::MinAckDelayRemote as usize] = connection.min_ack_delay_remote.ticks();
        v[PerflogColumn::MaxAckDelayRemote as usize] = connection.max_ack_delay_remote.ticks();
        v[PerflogColumn::MaxAckGapRemote as usize] = connection.max_ack_gap_remote;
        v[PerflogColumn::MinAckDelayLocal as usize] = connection.min_ack_delay_local.ticks();
        v[PerflogColumn::MaxAckDelayLocal as usize] = connection.max_ack_delay_local.ticks();
        v[PerflogColumn::MaxAckGapLocal as usize] = connection.max_ack_gap_local;
        v[PerflogColumn::MaxMtuSent as usize] = connection.max_mtu_sent as u64;
        v[PerflogColumn::MaxMtuReceived as usize] = connection.max_mtu_received as u64;
        v[PerflogColumn::ZeroRtt as usize] =
            (connection.nb_zero_rtt_received > 0 || connection.nb_zero_rtt_acked > 0) as u64;

        if let Some(path) = connection.paths.first() {
            v[PerflogColumn::Srtt as usize] = path.smoothed_rtt.ticks();
            v[PerflogColumn::Minrtt as usize] = path.rtt_min.ticks();
            v[PerflogColumn::Cwin as usize] = path.cwin;
            v[PerflogColumn::BweMax as usize] = path.bandwidth_estimate_max;
            v[PerflogColumn::PacingQuantumMax as usize] = path.pacing.quantum_max;
            v[PerflogColumn::PacingRate as usize] = path.pacing.rate_max;
        }

        if let Some(alg) = connection.congestion_alg {
            v[PerflogColumn::Ccalgo as usize] = alg.congestion_algorithm_number as u64;
        }

        self.items.push(item);
    }
```

## Pair `picoquic/performance_log.c:picoquic_perflog_file_set_header`
C: `picoquic/performance_log.c:298-318 picoquic_perflog_file_set_header`
Rust: `rs/fq/src/performance_log.rs:314-322 file_set_header`

### C body
```c
{
    FILE* F = picoquic_file_open(perflog_file_name, "w");

    if (F != NULL) {
        fprintf(F, "Log_v, PQ_v, Duration, Sent, Received, Mpbs_S, Mbps_R");
        fprintf(F, ", QUIC_v, ALPN, CNX_ID, T64");
        /* Print the additional values */
        for (size_t i = 0; i < PICOQUIC_PERF_LOG_MAX_ITEMS; i++) {
            char buf[16];
            char const* s = picoquic_perflog_param_name((picoquic_perflog_column_enum)i);
            if (s == NULL) {
                (void)picoquic_sprintf(buf, sizeof(buf), NULL, "v%zu", i);
                s = buf;
            }
            fprintf(F, ", %s", s);
        }
        fprintf(F, "\n");
        fclose(F);
    }
}
```

### Rust body
```rust
    else {
        return;
    };
```

## Pair `picoquic/picohash.c:picohash_retrieve`
C: `picoquic/picohash.c:67-82 picohash_retrieve`
Rust: `rs/fq/src/hash.rs:359-370 lookup`

### C body
```c
{
    uint64_t hash = hash_table->picohash_hash(key, hash_table->hash_seed);
    uint32_t bin = (uint32_t)(hash % hash_table->nb_bin);
    picohash_item* item = hash_table->hash_bin[bin];

    while (item != NULL) {
        if (hash_table->picohash_compare(key, item->key) == 0) {
            break;
        } else {
            item = item->next_in_bin;
        }
    }

    return item;
}
```

### Rust body
```rust
    pub fn lookup(&self, key: &K) -> Option<HashToken> {
        let hash = self.hash_key(key);
        let bin = self.bin_of(hash);
        let mut cur = self.bins[bin];
        while let Some(idx) = cur {
            if self.slot_key(idx) == key {
                return Some(self.token_of(idx));
            }
            cur = self.slot_next(idx);
        }
        None
    }
```

## Pair `picoquic/picohash.c:picohash_bytes`
C: `picoquic/picohash.c:180-202 picohash_bytes`
Rust: `rs/fq/src/tests/hashtest.rs:140-167 picohash_bytes`

### C body
```c
{
    uint64_t hash =
        ((uint64_t)hash_seed[8]) +
        (((uint64_t)hash_seed[9]) << 8) +
        (((uint64_t)hash_seed[10]) << 16) +
        (((uint64_t)hash_seed[11]) << 24) +
        (((uint64_t)hash_seed[12]) << 32) +
        (((uint64_t)hash_seed[13]) << 40) +
        (((uint64_t)hash_seed[14]) << 48) +
        (((uint64_t)hash_seed[15]) << 56);
    int rotate = 11;

    for (uint32_t i = 0; i < length; i++) {
        hash ^= bytes[i];
        hash ^= hash_seed[i & 15];
        hash ^= (hash << 8);
        hash += (hash >> rotate);
        rotate = (int)(hash & 31) + 11;
    }
    hash ^= (hash >> rotate);
    return hash;
}
```

### Rust body
```rust
fn picohash_bytes() {
    use crate::hash::hash_bytes;

    let mut test = [0u8; 1024];
    let mut k = [0u8; 16];
    hash_test_init(&mut test, &mut k);

    let lengths: [usize; 12] = [1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024];
    let expected: [u64; 12] = [
        0x0301_6721_e32d_7aa7,
        0x6420_8401_ad85_bed5,
        0x4458_7b02_0947_9519,
        0x14a4_8174_8ee6_d77e,
        0x9a44_370f_d1b8_c1ee,
        0x2708_1725_c416_4c1a,
        0x2f1f_325d_a756_df85,
        0x2aa4_fda7_96f9_ffff,
        0x8ded_0692_d703_8037,
        0x7893_f939_9f50_7284,
        0x47a0_65db_eea7_7343,
        0xb543_a5b3_c675_127d,
    ];

    for (i, &len) in lengths.iter().enumerate() {
        let h = hash_bytes(&test[..len], &k);
        assert_eq!(h, expected[i], "picohash_bytes[{i}] for len={len}");
    }
}
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_one_pass_stream`
C: `picoquic/picoquic_lb.c:61-73 picoquic_lb_compat_cid_one_pass_stream`
Rust: `rs/fq/src/lb.rs:462-477 one_pass_stream`

### C body
```c
{
    uint8_t mask[16];
    /* Set the obfuscation value */
    memset(mask, 0, sizeof(mask));
    memcpy(mask, nonce, nonce_length);
    /* Encrypt with ECB */
    picoquic_aes128_ecb_encrypt(enc_ctx, mask, mask, sizeof(mask));
    /* Apply the mask */
    for (size_t i = 0; i < target_length; i++) {
        target[i] ^= mask[i];
    }
}
```

### Rust body
```rust
    ) {
        let mut mask = [0u8; 16];
        let copy_len = nonce_len.min(16);
        mask[..copy_len].copy_from_slice(&bytes[nonce_start..nonce_start + copy_len]);
        enc.process(&mut mask);
        for i in 0..target_len {
            bytes[target_start + i] ^= mask[i];
        }
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_verify_clear`
C: `picoquic/picoquic_lb.c:150-161 picoquic_lb_compat_cid_verify_clear`
Rust: `rs/fq/src/lb.rs:390-398 verify_clear`

### C body
```c
{
    uint64_t s_id64 = 0;

    for (size_t i = 0; i < lb_ctx->server_id_length; i++) {
        s_id64 <<= 8;
        s_id64 += cnx_id->id[i + 1];
    }

    return s_id64;
}
```

### Rust body
```rust
    fn verify_clear(&self, cnx_id: &ConnectionId) -> u64 {
        let bytes = cnx_id.as_bytes();
        let mut s_id64: u64 = 0;
        for i in 0..self.server_id_length {
            s_id64 <<= 8;
            s_id64 += bytes[i + 1] as u64;
        }
        s_id64
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_config_parse`
C: `picoquic/picoquic_lb.c:236-387 picoquic_lb_compat_cid_config_parse`
Rust: `rs/fq/src/lb.rs:146-277 parse`

### C body
```c
{
    int ret = 0;
    size_t parsed = 0;
    size_t s_id_len;
    size_t cid_len = 0;
    size_t nonce_len = 0;
    /**/
    /* separator "-" */
    /* server_id -- string of 2xserver_id_length hex, max 16 */
    /* separator "-" */
    /* cid_encryption_key -- 32 hex digits*/
    memset(lb_config, 0, sizeof(*lb_config));
    if (txt_length < 4) {
        ret = -1;
    }
    else {
        /* rotation_bits: 0, 1 or 2 -- 3 is indefinite */
        if (txt[0] >= '0' && txt[0] <= '3') {
            lb_config->rotation_bits = (unsigned int)(txt[0] - '0');
        }
        else {
            ret = -1;
        }
        /* first_byte_encodes_length: Y or N */
        if (txt[1] == 'Y' || txt[1] == 'y') {
            lb_config->first_byte_encodes_length = 1;
        }
        else if (txt[1] != 'N' && txt[1] != 'n') {
            ret = -1;
        }
        parsed = 2;
        /* CID length as number, default to zero, in which case will be filled from QUIC context.
         * need to be careful because value is stored as uint8_t.
         */
        while (parsed < txt_length && txt[parsed] >= '0' && txt[parsed] <= '9') {
            cid_len *= 10;
            cid_len += txt[parsed] - '0';
            parsed++;
            if (cid_len < 256) {
                lb_config->connection_id_length = (uint8_t)cid_len;
            }
            else {
                ret = -1;
                break;
            }
        }
        /* method: C, S or B -- clear, stream-encrypted or block encrypted */
        if (parsed >= txt_length) {
            ret = -1;
        }
        else if (ret == 0) {
            char c = txt[parsed];
            parsed++;
            switch (c) {
            case 'c':
            case 'C':
                lb_config->method = picoquic_load_balancer_cid_clear;
                break;
            case 's':
            case 'S':
                lb_config->method = picoquic_load_balancer_cid_stream_cipher;
                while (parsed < txt_length && txt[parsed] >= '0' && txt[parsed] <= '9') {
                    nonce_len *= 10;
                    nonce_len += txt[parsed] - '0';
                    parsed++;
                    if (nonce_len < 256) {
                        lb_config->nonce_length = (uint8_t)nonce_len;
                    }
                    else {
                        ret = -1;
                        break;
                    }
                }
                break;
            case 'b':
            case 'B':
                lb_config->method = picoquic_load_balancer_cid_block_cipher;
                break;
            default:
                ret = -1;
                break;
            }
        }
        /* Skip hyphen */
        if (txt[parsed] == '-') {
            parsed++;
        }
        else {
            ret = -1;
        }
    }
    if (txt_length <= parsed) {
        ret = -1;
    }
    else if (ret == 0) {
        /* Parsing S_ID as hex string. */
        uint8_t s_id_bin[8];
        size_t hex_length = 0;
        while (parsed + hex_length < txt_length && txt[parsed + hex_length] != '-') {
            hex_length++;
        }
        s_id_len = picoquic_parse_hexa(txt + parsed, hex_length, s_id_bin, 8);
        if (s_id_len == 0 || s_id_len > 255) {
            ret = 1;
        }
        else {
            lb_config->server_id_length = (uint8_t)s_id_len;
            for (size_t i = 0; i < s_id_len; i++) {
                lb_config->server_id64 <<= 8;
                lb_config->server_id64 |= s_id_bin[i];
            }
        }
        parsed += 2 * s_id_len;
    }
    if (ret == 0 &&
        (lb_config->method == picoquic_load_balancer_cid_stream_cipher ||
            lb_config->method == picoquic_load_balancer_cid_block_cipher)) {
        /* Skip hyphen */
        if (txt[parsed] == '-') {
            parsed++;
        }
        else {
            ret = -1;
        }
        if (ret == 0) {
            if (txt_length < parsed + 32) {
                ret = -1;
            }
            else {
                /* Parse key as 32 bytes string */
                size_t key_length = picoquic_parse_hexa(txt + parsed, txt_length - parsed, lb_config->cid_encryption_key, 16);
                if (key_length != 16) {
                    ret = -1;
                }
                parsed += 2 * key_length;
            }
        }
    }
    if (ret == 0 && parsed != txt_length) {
        ret = -1;
    }

    if (ret == 0 && lb_config->connection_id_length != 0) {
        size_t min_length = 1 + lb_config->server_id_length + lb_config->nonce_length;
        if (lb_config->connection_id_length < min_length ||
            (lb_config->method == picoquic_load_balancer_cid_block_cipher && lb_config->connection_id_length < 17)) {
            ret = -1;
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn parse(txt: &str) -> Result<Self, Error> {
        let bytes = txt.as_bytes();
        if bytes.len() < 4 {
            return Err(Error::InvalidArgument);
        }

        let rotation_bits = match bytes[0] {
            b'0' => RotationBits::Zero,
            b'1' => RotationBits::One,
            b'2' => RotationBits::Two,
            b'3' => RotationBits::Three,
            _ => return Err(Error::InvalidArgument),
        };

        let first_byte_encodes_length = match bytes[1] {
            b'Y' | b'y' => true,
            b'N' | b'n' => false,
            _ => return Err(Error::InvalidArgument),
        };

        let mut config = Config {
            rotation_bits,
            first_byte_encodes_length,
            ..Config::default()
        };

        let mut parsed = 2usize;

        // Optional decimal connection_id_length.
        let mut cid_len: usize = 0;
        while parsed < bytes.len() && bytes[parsed].is_ascii_digit() {
            cid_len = cid_len * 10 + (bytes[parsed] - b'0') as usize;
            parsed += 1;
            if cid_len >= 256 {
                return Err(Error::InvalidArgument);
            }
            config.connection_id_length = cid_len;
        }

        if parsed >= bytes.len() {
            return Err(Error::InvalidArgument);
        }
        let method_char = bytes[parsed];
        parsed += 1;
        match method_char {
            b'c' | b'C' => {
                config.method = ConnectionIdMethod::Clear;
            }
            b's' | b'S' => {
                config.method = ConnectionIdMethod::StreamCipher;
                let mut nonce_len: usize = 0;
                while parsed < bytes.len() && bytes[parsed].is_ascii_digit() {
                    nonce_len = nonce_len * 10 + (bytes[parsed] - b'0') as usize;
                    parsed += 1;
                    if nonce_len >= 256 {
                        return Err(Error::InvalidArgument);
                    }
                    config.nonce_length = nonce_len;
                }
            }
            b'b' | b'B' => {
                config.method = ConnectionIdMethod::BlockCipher;
            }
            _ => return Err(Error::InvalidArgument),
        }

        // Hyphen before server ID.
        if parsed >= bytes.len() || bytes[parsed] != b'-' {
            return Err(Error::InvalidArgument);
        }
        parsed += 1;

        if parsed >= bytes.len() {
            return Err(Error::InvalidArgument);
        }

        // Server ID hex (scan to next '-' or end).
        let hex_len = bytes[parsed..]
            .iter()
            .position(|&b| b == b'-')
            .unwrap_or(bytes.len() - parsed);
        let hex_end = parsed + hex_len;
        let mut s_id_bin = [0u8; 8];
        let s_id_len = parse_hex_bytes(&bytes[parsed..hex_end], &mut s_id_bin);
        if s_id_len == 0 {
            return Err(Error::InvalidArgument);
        }
        config.server_id_length = s_id_len;
        for b in s_id_bin.iter().take(s_id_len) {
            config.server_id <<= 8;
            config.server_id |= *b as u64;
        }
        parsed += 2 * s_id_len;

        // Encryption key for stream/block cipher.
        if matches!(
            config.method,
            ConnectionIdMethod::StreamCipher | ConnectionIdMethod::BlockCipher
        ) {
            if parsed >= bytes.len() || bytes[parsed] != b'-' {
                return Err(Error::InvalidArgument);
            }
            parsed += 1;
            if bytes.len() < parsed + 32 {
                return Err(Error::InvalidArgument);
            }
            let key_len = parse_hex_bytes(&bytes[parsed..], &mut config.cid_encryption_key);
            if key_len != 16 {
                return Err(Error::InvalidArgument);
            }
            parsed += 32;
        }

        if parsed != bytes.len() {
            return Err(Error::InvalidArgument);
        }

        // Validate connection_id_length if explicitly set.
        if config.connection_id_length != 0 {
            let min_length = 1 + config.server_id_length + config.nonce_length;
            if config.connection_id_length < min_length {
                return Err(Error::InvalidArgument);
            }
            if matches!(config.method, ConnectionIdMethod::BlockCipher)
                && config.connection_id_length < 17
            {
                return Err(Error::InvalidArgument);
            }
        }

        Ok(config)
    }
```

## Pair `picoquic/picoquic_ptls_fusion.c:picoquic_ptls_fusion_load`
C: `picoquic/picoquic_ptls_fusion.c:77-84 picoquic_ptls_fusion_load`
Rust: `rs/fq/src/tls_api.rs:2521-2521 ptls_fusion_load`

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
pub fn ptls_fusion_load(_unload: bool) {}
```

## Pair `picoquic/picoquic_ptls_minicrypto.c:picoquic_ptls_minicrypto_load`
C: `picoquic/picoquic_ptls_minicrypto.c:54-83 picoquic_ptls_minicrypto_load`
Rust: `rs/fq/src/tls_api.rs:2528-2531 ptls_minicrypto_load`

### C body
```c
{
    if (unload) {
        picoquic_clear_minicrypto();
    }
    else {
        picoquic_init_minicrypto();

        picoquic_register_ciphersuite(&ptls_minicrypto_aes128gcmsha256, 1);
        picoquic_register_ciphersuite(&ptls_minicrypto_aes256gcmsha384, 1);
        picoquic_register_ciphersuite(&ptls_minicrypto_chacha20poly1305sha256, 1);
        picoquic_register_key_exchange_algorithm(&ptls_minicrypto_secp256r1);
        picoquic_register_key_exchange_algorithm(&ptls_minicrypto_x25519);

        picoquic_register_crypto_random_provider_fn(ptls_minicrypto_random_bytes);

        picoquic_register_tls_key_provider_fn(
            set_minicrypto_private_key_from_key_file,
            NULL, NULL, NULL);
#if 0
        picoquic_register_verify_certificate_fn(picoquic_openssl_get_certificate_verifier,
            picoquic_openssl_dispose_certificate_verifier,
            picoquic_openssl_set_tls_root_certificates);
        picoquic_register_explain_crypto_error_fn(picoquic_open_ssl_explain_crypto_error,
            picoquic_openssl_clear_crypto_errors);
#endif
    }
}
```

### Rust body
```rust
pub fn ptls_minicrypto_load(unload: bool) {
    let mut state = tls_api_state();
    ptls_minicrypto_load_locked(&mut state, i32::from(unload));
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:set_openssl_private_key_from_key_file`
C: `picoquic/picoquic_ptls_openssl.c:138-158 set_openssl_private_key_from_key_file`
Rust: `rs/fq/src/sys/openssl.rs:552-557 set_openssl_private_key_from_key_file`

### C body
```c
{
    int ret = 0;
    BIO* bio = BIO_new_file(keypem, "rb");
    if (bio == NULL) {
        ret = -1;
    }
    else {
        EVP_PKEY* pkey = PEM_read_bio_PrivateKey(bio, NULL, NULL, NULL);
        if (pkey == NULL) {
            ret = -1;
        }
        else {
            ret = set_openssl_sign_certificate_from_key(pkey, ctx);
        }
        BIO_free(bio);
    }
    return ret;
}
```

### Rust body
```rust
fn set_openssl_private_key_from_key_file(keypem: &str) -> Result<SignCertificate, crate::Error> {
    let pem = std::fs::read(keypem).map_err(|_| crate::Error::NoSuchFile)?;
    let key =
        openssl::pkey::PKey::private_key_from_pem(&pem).map_err(|_| crate::Error::InvalidFile)?;
    set_sign_certificate_from_key(Some(key))
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_get_openssl_certificate_verifier`
C: `picoquic/picoquic_ptls_openssl.c:237-267 picoquic_openssl_get_openssl_certificate_verifier`
Rust: `rs/fq/src/sys/openssl.rs:377-400 get_openssl_certificate_verifier`

### C body
```c
{
    ptls_openssl_verify_certificate_t * verifier = (ptls_openssl_verify_certificate_t*)malloc(sizeof(ptls_openssl_verify_certificate_t));
    if (verifier != NULL) {
        X509_STORE* store = X509_STORE_new();

        if (cert_root_file_name != NULL && store != NULL) {
            int file_ret = 0;
            X509_LOOKUP* lookup = X509_STORE_add_lookup(store, X509_LOOKUP_file());
            if ((file_ret = X509_LOOKUP_load_file(lookup, cert_root_file_name, X509_FILETYPE_PEM)) == 1) {
                *is_cert_store_not_empty = 1;
            }
        }
#ifdef PTLS_OPENSSL_VERIFY_CERTIFICATE_ENABLE_OVERRIDE
        ptls_openssl_init_verify_certificate(verifier, store, NULL);
#else
        ptls_openssl_init_verify_certificate(verifier, store);
#endif

        // If we created an instance of the store, release our reference after giving it to the verify_certificate callback.
        // The callback internally increased the reference counter by one.
#if OPENSSL_VERSION_NUMBER > 0x10100000L
        if (store != NULL) {
            X509_STORE_free(store);
        }
#endif
    }
    return verifier;
}
```

### Rust body
```rust
) -> Result<CertificateVerifier, crate::Error> {
    let mut builder =
        openssl::x509::store::X509StoreBuilder::new().map_err(|_| crate::Error::Generic)?;
    let mut is_cert_store_not_empty = false;
    if let Some(file_name) = cert_root_file_name {
        // Load PEM certs from the root CA file and add each to the store.
        // Mirrors X509_STORE_add_lookup + X509_LOOKUP_load_file from the C original.
        if let Ok(pem_data) = std::fs::read(file_name)
            && let Ok(certs) = openssl::x509::X509::stack_from_pem(&pem_data)
        {
            for cert in certs {
                if builder.add_cert(cert).is_ok() {
                    is_cert_store_not_empty = true;
                }
            }
        }
    }
    Ok(CertificateVerifier {
        store: builder.build(),
        is_cert_store_not_empty,
    })
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_open_ssl_explain_crypto_error`
C: `picoquic/picoquic_ptls_openssl.c:321-332 picoquic_open_ssl_explain_crypto_error`
Rust: `rs/fq/src/sys/openssl.rs:451-461 explain_crypto_error`

### C body
```c
{
#if !defined(LIBRESSL_VERSION_NUMBER) && OPENSSL_VERSION_NUMBER >= 0x30000000L
    const char *func = NULL;
    const char *data = NULL;
    int flags=0;
    return (int)ERR_get_error_all(err_file, err_line, &func, &data, &flags);
#else
    return ERR_get_error_line(err_file, err_line);
#endif
}
```

### Rust body
```rust
        .map(|e| CryptoError {
            code: e.code(),
            reason: e.reason().map(str::to_owned),
            library: e.library().map(str::to_owned),
            file: e.file().to_owned(),
            line: e.line(),
        })
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_ptls_openssl_load`
C: `picoquic/picoquic_ptls_openssl.c:406-454 picoquic_ptls_openssl_load`
Rust: `rs/fq/src/sys/openssl.rs:772-789 picoquic_ptls_openssl_load`

### C body
```c
{
    if (unload) {
        if (unload == 1) {
            picoquic_clear_openssl();
        }
    }
    else {
        picoquic_init_openssl();
#ifdef OPENSSL_VERSION_NUMBER
        DBG_PRINTF("Open ssl include version: %x", OPENSSL_VERSION_NUMBER);
#endif
#ifdef LIBRESSL_VERSION_NUMBER
        DBG_PRINTF("LIBRE SSL include version: %x", LIBRESSL_VERSION_NUMBER);
#endif
        DBG_PRINTF("OpenSSL_version_num(): %x", OpenSSL_version_num());

        picoquic_register_ciphersuite(&ptls_openssl_aes128gcmsha256, 1);
        picoquic_register_ciphersuite(&ptls_openssl_aes256gcmsha384, 1);
        picoquic_register_key_exchange_algorithm(&ptls_openssl_secp256r1);
        picoquic_register_hpke_cipher_suite(&picoquic_openssl_hpke_aes128gcmsha256);
        picoquic_register_hpke_cipher_suite(&picoquic_openssl_hpke_aes128gcmsha512);
        picoquic_register_hpke_cipher_suite(&picoquic_openssl_hpke_aes256gcmsha384);
        picoquic_register_hpke_kem(&picoquic_openssl_hpke_kem_p256sha256);
        picoquic_register_hpke_kem(&picoquic_openssl_hpke_kem_p384sha384);

#ifdef PTLS_OPENSSL_HAVE_CHACHA20_POLY1305
        picoquic_register_ciphersuite(&ptls_openssl_chacha20poly1305sha256, 1);
        picoquic_register_key_exchange_algorithm(&ptls_openssl_x25519);
        picoquic_register_hpke_cipher_suite(&picoquic_openssl_hpke_chacha20poly1305sha256);
        picoquic_register_hpke_kem(&picoquic_openssl_hpke_kem_x25519sha256);
#endif
        picoquic_register_tls_key_provider_fn(
            set_openssl_private_key_from_key_file,
            picoquic_openssl_dispose_sign_certificate,
            picoquic_openssl_get_certs_from_file,
            picoquic_openssl_get_public_key_from_key_file);
        picoquic_register_verify_certificate_fn(picoquic_openssl_get_certificate_verifier,
            picoquic_openssl_dispose_certificate_verifier,
            picoquic_openssl_set_tls_root_certificates);
        picoquic_register_explain_crypto_error_fn(picoquic_open_ssl_explain_crypto_error,
            picoquic_openssl_clear_crypto_errors);
        picoquic_register_crypto_random_provider_fn(ptls_openssl_random_bytes);
        picoquic_register_keyex_from_key_file_fn(openssl_keyex_from_key_file, openssl_keyex_dispose);

    }
}
```

### Rust body
```rust
pub fn picoquic_ptls_openssl_load(unload: i32) -> Option<&'static OpenSslProviderRegistration> {
    if unload != 0 {
        if unload == 1 {
            clear_openssl();
        }
        None
    } else {
        init_openssl();
        if let Some(version) = source_version_number() {
            log::debug!("Open ssl include version: {:x}", version);
        }
        if let Some(version) = libressl_source_version_number() {
            log::debug!("LIBRE SSL include version: {:x}", version);
        }
        log::debug!("OpenSSL_version_num(): {:x}", openssl::version::number());
        Some(&OPENSSL_PROVIDER_REGISTRATION)
    }
}
```
