# Phase 4F post-merge semantic revalidation

This is a read-only final-tree audit after worker worktree
merges.  Do not edit files.

For each C/Rust function pair, inspect the C function and the
current merged Rust implementation.  Decide whether the merged
Rust behavior is acceptable as a safe Rust translation of the
C behavior.  Pay particular attention to whether a prior
worker repair may have been lost or distorted during conflict
resolution.

Report:

* `ok` when the current merged Rust behavior is acceptable.
* `needs_fix` when a real C/Rust mismatch remains in the
  current merged tree.
* `blocked` only when a concrete missing external decision or
  dependency prevents classification.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/packet.c:picoquic_incoming_segment`
* Worker baseline outcome: `blocked`
* Worker baseline source: `/private/tmp/picoquic-4e-r2-09`
* Worker analysis: Rust source already has Quic::incoming_segment with the correct C reference and incoming_packet_ex remains mapped to picoquic_incoming_packet_ex. The remaining mismatch is caused by scripts/phase4_common.py hard-coding picoquic_incoming_segment -> incoming_packet_ex, so fixing it requires editing the mapping script or xlate/function_translation_map.json outside this Rust-only owned-file batch.
* Worker fix summary: 
* Worker confirmation outcome: ``
* Worker confirmation analysis: 
* C source: `picoquic/packet.c:2073-2387`
* C signature: `int picoquic_incoming_segment(picoquic_quic_t *, uint8_t *, size_t, size_t, size_t *, struct sockaddr *, struct sockaddr *, int, unsigned char, uint64_t, uint64_t, picoquic_connection_id_t *, picoquic_cnx_t **)`
* Current merged Rust source: `rs/fq/src/lib.rs:8092-8429`
* Current merged Rust item: `incoming_segment`

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

### Current merged Rust body
```rust
    ) -> i32 {
        let mut ph = crate::internal::PacketHeader::default();
        let mut decrypted_data = match self.stream_data_node_alloc() {
            Ok(node) => node,
            Err(_) => return -1,
        };

        let parsed = self.parse_header_and_decrypt_for_segment(
            raw_bytes,
            length,
            packet_length,
            addr_from,
            current_time,
            &mut decrypted_data,
            &mut ph,
            consumed,
        );
        let mut ret = parsed.ret;
        let mut cnx = parsed.connection;
        let new_context_created = parsed.new_context_created;
        let mut is_first_segment = false;
        let mut is_buffered = false;
        let mut path_id = None;
        let mut path_is_not_allocated = 0i32;

        if ret == 0
            && let Some(connection) = cnx
        {
            if ph.packet_type == crate::internal::PacketType::OneRttProtected {
                let lookup = {
                    let Some(cnx_ref) = self.connections.get_mut(connection) else {
                        return -1;
                    };
                    cnx_ref.find_incoming_path(
                        &mut ph,
                        addr_from,
                        addr_to,
                        if_index_to,
                        current_time,
                    )
                };
                match lookup {
                    Ok(lookup) => {
                        path_id = Some(lookup.path_id);
                        path_is_not_allocated = i32::from(lookup.created);
                    }
                    Err(error) => ret = Self::parse_error_status(error),
                }
            } else {
                path_id = Some(0);
            }
        }

        if previous_dest_id.is_empty() {
            *previous_dest_id = ph.dest_connection_id;
            is_first_segment = true;
            *first_cnx = cnx;
            if let Some(connection) = cnx {
                if let Some(cnx_ref) = self.connections.get_mut(connection) {
                    let unique_path_id = path_id
                        .and_then(|idx| cnx_ref.paths.get(idx))
                        .map(|path| path.unique_path_id)
                        .unwrap_or(0);
                    crate::logger::Log::pdu(
                        cnx_ref,
                        true,
                        current_time,
                        addr_from,
                        addr_to,
                        packet_length,
                        unique_path_id,
                        received_ecn,
                    );
                }
            } else {
                self.log_pdu(
                    true,
                    current_time,
                    ph.dest_connection_id.val64(),
                    addr_from,
                    addr_to,
                    packet_length,
                );
            }
        } else {
            if (ret == 0 && *previous_dest_id != ph.dest_connection_id)
                || ret == InternalError::VersionNotSupported as i32
            {
                ret = InternalError::CnxidSegment as i32;
            }
            if ret == InternalError::CnxidSegment as i32
                && *first_cnx != cnx
                && let Some(first) = *first_cnx
                && let Some(first_ref) = self.connections.get_mut(first)
            {
                first_ref.log_dropped_packet_on_path(
                    None,
                    &ph,
                    length,
                    InternalError::PaddingPacket as i32,
                    current_time,
                );
            }
        }

        if ret == InternalError::AeadNotReady as i32
            && let Some(connection) = cnx
            && let Some(cnx_ref) = self.connections.get_mut(connection)
        {
            is_buffered = cnx_ref.incoming_not_decrypted(
                &ph,
                current_time,
                raw_bytes,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
            ) != 0;
        }

        if let Some(connection) = cnx {
            if ret == 0 && ph.packet_type == crate::internal::PacketType::OneRttProtected {
                if ph.payload_length == 0 {
                    if let Some(cnx_ref) = self.connections.get_mut(connection) {
                        ret = cnx_ref.connection_error(TransportError::ProtocolViolation as u64, 0);
                    }
                } else if ph.has_reserved_bit_set
                    && let Some(cnx_ref) = self.connections.get_mut(connection)
                {
                    ret = cnx_ref.connection_error(TransportError::ProtocolViolation as u64, 0);
                }
            }

            if let Some(cnx_ref) = self.connections.get_mut(connection) {
                if ret == 0 {
                    cnx_ref.log_packet_on_path(
                        path_id,
                        true,
                        current_time,
                        &ph,
                        &decrypted_data.data[..decrypted_data.length],
                    );
                } else if is_buffered {
                    cnx_ref.log_buffered_packet_on_path(path_id, ph.packet_type, current_time);
                } else {
                    cnx_ref.log_dropped_packet_on_path(path_id, &ph, length, ret, current_time);
                }
            }
        }

        if ret == InternalError::VersionNotSupported as i32 {
            if packet_length >= crate::internal::ENFORCED_INITIAL_MTU
                && (self.is_port_blocking_disabled || !check_addr_blocked(addr_from))
            {
                self.queue_version_negotiation_packet(addr_from, addr_to, if_index_to, &ph);
            }
        } else if ret == InternalError::RetryNeeded as i32 {
            if packet_length >= crate::internal::ENFORCED_INITIAL_MTU
                && (self.is_port_blocking_disabled || !check_addr_blocked(addr_from))
            {
                ret = self.queue_retry_packet(addr_from, addr_to, if_index_to, &ph, current_time);
            }
        } else if ret == InternalError::ServerBusy as i32 {
            if packet_length >= crate::internal::ENFORCED_INITIAL_MTU
                && (self.is_port_blocking_disabled || !check_addr_blocked(addr_from))
            {
                ret = self.queue_busy_packet(addr_from, addr_to, if_index_to, &ph);
            }
        } else if ret == 0 {
            if let Some(connection) = cnx {
                if let Some(cnx_ref) = self.connections.get_mut(connection) {
                    cnx_ref.quic_bit_received_0 |= ph.quic_bit_is_zero;
                }
                let bytes_len = decrypted_data.length;
                let mut packet_bytes = [0u8; crate::internal::MAX_PACKET_SIZE];
                packet_bytes[..bytes_len].copy_from_slice(&decrypted_data.data[..bytes_len]);
                let mut received_frame_data = crate::internal::StreamDataNode {
                    stream_data_membership: None,
                    offset: 0,
                    data: [0u8; crate::internal::MAX_PACKET_SIZE],
                    length: 0,
                };
                match ph.packet_type {
                    crate::internal::PacketType::VersionNegotiation => {
                        let bytes = &packet_bytes[..bytes_len];
                        ret = self.incoming_version_negotiation(connection, bytes, &ph);
                    }
                    crate::internal::PacketType::Initial => {
                        let bytes = &packet_bytes[..bytes_len];
                        ret = self.dispatch_initial_segment(
                            connection,
                            bytes,
                            packet_length,
                            &mut received_frame_data,
                            addr_from,
                            addr_to,
                            if_index_to,
                            &ph,
                            current_time,
                            new_context_created,
                            is_first_segment,
                            first_cnx,
                        );
                        cnx = *first_cnx;
                    }
                    crate::internal::PacketType::Retry => {
                        ret = self.incoming_retry(connection, raw_bytes, &ph, current_time);
                    }
                    crate::internal::PacketType::Handshake => {
                        let bytes = &packet_bytes[..bytes_len];
                        ret = self.dispatch_handshake_segment(
                            connection,
                            bytes,
                            &mut received_frame_data,
                            addr_to,
                            &ph,
                            current_time,
                        );
                    }
                    crate::internal::PacketType::ZeroRttProtected => {
                        let bytes = &packet_bytes[..bytes_len];
                        ret = self.dispatch_zero_rtt_segment(
                            connection,
                            bytes,
                            &mut received_frame_data,
                            packet_length,
                            &ph,
                            current_time,
                            is_first_segment,
                        );
                    }
                    crate::internal::PacketType::OneRttProtected => {
                        if let Some(cnx_ref) = self.connections.get_mut(connection) {
                            let bytes = &mut packet_bytes[..bytes_len];
                            ret = cnx_ref.incoming_1rtt(
                                path_id.unwrap_or(0),
                                bytes,
                                &mut received_frame_data,
                                &ph,
                                Some(addr_from),
                                Some(addr_to),
                                if_index_to,
                                path_is_not_allocated,
                                current_time,
                            );
                        }
                    }
                    _ => ret = InternalError::Detected as i32,
                }
            } else {
                if !ph.dest_connection_id.is_empty()
                    && (self.is_port_blocking_disabled || !check_addr_blocked(addr_from))
                {
                    self.queue_version_negotiation_packet(addr_from, addr_to, if_index_to, &ph);
                }
                ret = InternalError::Detected as i32;
            }
        } else if ret == InternalError::StatelessReset as i32 {
            if let Some(connection) = cnx {
                ret = self.incoming_stateless_reset(connection);
            }
        } else if ret == InternalError::AeadCheck as i32
            && ph.packet_type == crate::internal::PacketType::Handshake
            && let Some(connection) = cnx
            && let Some(cnx_ref) = self.connections.get_mut(connection)
            && (cnx_ref.connection_state == State::ClientInitSent
                || cnx_ref.connection_state == State::ClientInitResent)
            && !cnx_ref.pkt_ctx[PacketContext::Initial as usize]
                .pending
                .is_empty()
            && cnx_ref
                .paths
                .first()
                .is_some_and(|path| path.nb_retransmit == 0)
            && let Some(first_pending) = cnx_ref.pkt_ctx[PacketContext::Initial as usize]
                .pending
                .values()
                .next()
                .and_then(|packet| cnx_ref.queued_packets.get(*packet))
            && let Some(path) = cnx_ref.paths.first_mut()
        {
            path.retransmit_timer = Duration::from_ticks(
                current_time
                    .ticks()
                    .saturating_sub(first_pending.send_time.ticks()),
            );
        }

        if ret == 0 {
            if let Some(connection) = cnx
                && let Some(cnx_ref) = self.connections.get_mut(connection)
                && cnx_ref.connection_state != State::Disconnected
                && ph.packet_type != crate::internal::PacketType::VersionNegotiation
            {
                cnx_ref.nb_packets_received = cnx_ref.nb_packets_received.saturating_add(1);
                cnx_ref.latest_receive_time = current_time;
                ret = cnx_ref.record_pn_received(
                    ph.packet_context,
                    ph.local_connection_id,
                    ph.packet_number_full,
                    receive_time,
                );
                cnx_ref.ecn_accounting(received_ecn, ph.packet_context, ph.local_connection_id);
            }
            if let Some(connection) = cnx {
                self.reinsert_by_wake_time_token(connection, current_time);
            }
        } else if Self::is_silent_drop_status(ret) {
            let normalized = if Self::silent_drop_status_returns_zero(ret) {
                0
            } else {
                -1
            };
            if let Some(connection) = cnx {
                self.reinsert_by_wake_time_token(connection, current_time);
            }
            ret = normalized;
        } else if ret != 0 {
            ret = -1;
        }

        self.stream_data_node_recycle(decrypted_data);
        ret
    }
```
