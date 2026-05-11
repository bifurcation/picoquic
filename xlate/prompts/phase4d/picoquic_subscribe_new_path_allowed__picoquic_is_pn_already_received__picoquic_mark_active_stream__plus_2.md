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

## `picoquic/quicctx.c:picoquic_subscribe_new_path_allowed`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both bodies subscribe on the same transient errors, but C also clears is_notified_that_path_is_allowed in both successful and subscribed cases; Rust does not visibly clear that flag.
* C source: `picoquic/quicctx.c:2344-2368`
* C signature: `int picoquic_subscribe_new_path_allowed(picoquic_cnx_t *, int *)`
* Rust source: `rs/fq/src/lib.rs:2634-2651`
* Rust item: `subscribe_new_path_allowed`

### C body
```c
{
    int ret;
    PICOQUIC_THREAD_CHECK(cnx->quic);
        
    ret = picoquic_check_new_path_allowed(cnx, 0);

    *is_already_allowed = 0;
    if (ret == 0) {
        /* is allowed. Just say so -- get return code. */
        *is_already_allowed = 1;
        cnx->is_subscribed_to_path_allowed = 0;
        cnx->is_notified_that_path_is_allowed = 0;
    }
    else if (ret == PICOQUIC_ERROR_PATH_NOT_READY ||
        ret == PICOQUIC_ERROR_PATH_LIMIT_EXCEEDED ||
        ret == PICOQUIC_ERROR_PATH_ID_BLOCKED ||
        ret == PICOQUIC_ERROR_PATH_CID_BLOCKED) {
        /* transient error. Subscribe to the event and return 0 */
        cnx->is_subscribed_to_path_allowed = 1;
        cnx->is_notified_that_path_is_allowed = 0;
        ret = 0;
    }
    return ret;
}
```

### Rust body
```rust
        match self.check_new_path_allowed(false) {
            Ok(()) => {
                self.is_subscribed_to_path_allowed = false;
                Ok(true)
            }
            Err(Error::Protocol(code))
                if code == InternalError::PathNotReady as u64
                    || code == InternalError::PathLimitExceeded as u64
                    || code == InternalError::PathIdBlocked as u64
                    || code == InternalError::PathCidBlocked as u64 =>
            {
                self.is_subscribed_to_path_allowed = true;
                Ok(false)
            }
            Err(e) => Err(e),
        }
```

## `picoquic/sacks.c:picoquic_is_pn_already_received`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust checks only existing SACK ranges; C first treats packets below ack_horizon as received when horizon_delay is positive.
* C source: `picoquic/sacks.c:170-189`
* C signature: `int picoquic_is_pn_already_received(picoquic_cnx_t *, picoquic_packet_context_enum, picoquic_local_cnxid_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:8015-8043`
* Rust item: `is_pn_already_received`

### C body
```c
{
    int is_received = 0;
    picoquic_sack_list_t* sack_list = picoquic_sack_list_from_cnx_context(cnx, pc, l_cid);

    if (sack_list->horizon_delay > 0 && pn64 < sack_list->ack_horizon) {
        is_received = 1;
    }
    else {
        picoquic_sack_item_t* sack_found = picoquic_sack_find_range_below_number(sack_list, NULL, pn64);
        is_received = (sack_found != NULL && pn64 <= sack_found->end_of_sack_range);
    }
    return is_received;
}
```

### Rust body
```rust
    ) -> bool {
        // C: picoquic_is_pn_already_received — check SACK list for duplicate PN.
        let ack_ctx = &self.ack_ctx[pc as usize];
        // Check: is pn64 covered by any SACK range?
        let mut st_opt = ack_ctx.sack_list.ack_tree.last();
        while let Some(st) = st_opt {
            let item_tok = match ack_ctx.sack_list.ack_tree.get(st).copied() {
                Some(t) => t,
                None => break,
            };
            let item = match ack_ctx.sack_list.sack_items.get(item_tok) {
                Some(i) => i,
                None => break,
            };
            if item.start_of_sack_range <= pn64 && item.end_of_sack_range >= pn64 {
                return true;
            }
            if item.end_of_sack_range < pn64 {
                break;
            }
            st_opt = ack_ctx.sack_list.ack_tree.previous(st);
        }
        false
    }
```

## `picoquic/sender.c:picoquic_mark_active_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: C allows reset_requested when a sack-list check succeeds, but Rust rejects any reset_requested stream. Rust also enqueues output streams instead of visibly reinserting by wake time.
* C source: `picoquic/sender.c:147-178`
* C signature: `int picoquic_mark_active_stream(picoquic_cnx_t *, uint64_t, int, void *)`
* Rust source: `rs/fq/src/lib.rs:3973-4007`
* Rust item: `mark_active_stream`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    stream = picoquic_find_stream_for_writing(cnx, stream_id, &ret);
    if (ret == 0) {
        if (is_active) {
            /* The call only fails if the stream was closed or reset */
            if (!stream->fin_requested && 
                (!stream->reset_requested || picoquic_check_sack_list(&stream->sack_list, 0, stream->reliable_size) == 0) &&
                cnx->callback_fn != NULL) {
                stream->app_stream_ctx = app_stream_ctx;
                if (!stream->is_active) {
                    stream->is_active = 1;
                    picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));
                }
            }
            else {
                ret = PICOQUIC_ERROR_CANNOT_SET_ACTIVE_STREAM;
            }
        }
        else {
            stream->is_active = 0;
            stream->app_stream_ctx = app_stream_ctx;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let stream_token = self.find_stream_for_writing(stream_id)?;
        let has_callback = self.callback_fn.is_some();
        let mut should_enqueue = false;

        {
            let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;

            if is_active {
                if !stream.fin_requested && !stream.reset_requested && has_callback {
                    stream.app_stream_ctx = v_stream_ctx;
                    stream.is_active = true;
                    if !stream.is_output_stream {
                        stream.is_output_stream = true;
                        should_enqueue = true;
                    }
                } else {
                    return Err(Error::Protocol(InternalError::CannotSetActiveStream as u64));
                }
            } else {
                stream.is_active = false;
                stream.app_stream_ctx = v_stream_ctx;
            }
        }

        if should_enqueue {
            enqueue_output_stream_token(self, stream_token);
        }
        Ok(())
    }
```

## `picoquic/sender.c:picoquic_prepare_next_packet_ex`
* Phase 4C status: `suspect`
* Phase 4C rationale: Disconnected handling omits body-visible logging/flush behavior from C and returns default output values in cases where C only sets send_length.
* C source: `picoquic/sender.c:4244-4324`
* C signature: `int picoquic_prepare_next_packet_ex(picoquic_quic_t *, uint64_t, uint8_t *, size_t, size_t *, struct sockaddr_storage *, struct sockaddr_storage *, int *, picoquic_connection_id_t *, picoquic_cnx_t **, size_t *)`
* Rust source: `rs/fq/src/lib.rs:3392-3505`
* Rust item: `prepare_next_packet_ex`

### C body
```c
{
    int ret = 0;
    picoquic_stateless_packet_t* sp;
    PICOQUIC_THREAD_CHECK(quic);
    
    sp = picoquic_dequeue_stateless_packet(quic);
    if (p_last_cnx) {
        *p_last_cnx = NULL;
    }

    if (sp != NULL) {
        if (sp->length > send_buffer_max) {
            *send_length = 0;
        }
        else {
            memcpy(send_buffer, sp->bytes, sp->length);
            *send_length = sp->length;
            picoquic_store_addr(p_addr_to, (struct sockaddr*) & sp->addr_to);
            picoquic_store_addr(p_addr_from, (struct sockaddr*) & sp->addr_local);
            *if_index = sp->if_index_local;
            if (log_cid != NULL) {
                *log_cid = sp->initial_cid;
            }
        }
        picoquic_delete_stateless_packet(sp);
    }
    else {
        picoquic_cnx_t* cnx = picoquic_get_earliest_cnx_to_wake(quic, current_time);

        if (cnx == NULL) {
            *send_length = 0;
        }
        else {
            ret = picoquic_prepare_packet_ex(cnx, current_time, send_buffer, send_buffer_max, send_length, p_addr_to, p_addr_from, 
                if_index, send_msg_size);
            if (log_cid != NULL) {
                *log_cid = cnx->initial_cnxid;
            }

            if (ret == PICOQUIC_ERROR_DISCONNECTED) {
                ret = 0;

                picoquic_log_app_message(cnx, "Closed. Retrans= %d, spurious= %d, max sp gap = %d, max sp delay = %d, dg-coal: %f",
                    (int)cnx->nb_retransmission_total, (int)cnx->nb_spurious,
                    (int)cnx->path[0]->max_reorder_gap, (int)cnx->path[0]->max_spurious_rtt,
                    (cnx->nb_trains_sent > 0) ? ((double)cnx->nb_packets_sent / (double)cnx->nb_trains_sent) : 0.0);

                if (quic->F_log != NULL) {
                    fflush(quic->F_log);
                }

                if (cnx->f_binlog != NULL) {
                    fflush(cnx->f_binlog);
                }

                if (cnx->client_mode) {
                    /* Do not unilaterally delete the connection context, as it was set by the application */
                    picoquic_reinsert_by_wake_time(cnx->quic, cnx, UINT64_MAX);
                    SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                }
                else {
                    picoquic_delete_cnx(cnx);
                }
            }
            else {
                if (*if_index == -1) {
                    *if_index = picoquic_get_local_if_index(cnx);
                }
                if (p_last_cnx) {
                    *p_last_cnx = cnx;
                }
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<PreparedPacket<'_>, Error> {
        let default_addr = unspecified_socket_addr();

        if let Some(sp) = self.dequeue_stateless_packet() {
            if sp.length > send_buffer.len() {
                return Ok(PreparedPacket {
                    send_length: 0,
                    addr_to: default_addr,
                    addr_from: default_addr,
                    if_index: -1,
                    log_cid: ConnectionId::default(),
                    last_connection: None,
                    send_msg_size: None,
                });
            }
            send_buffer[..sp.length].copy_from_slice(&sp.bytes[..sp.length]);
            return Ok(PreparedPacket {
                send_length: sp.length,
                addr_to: sp.addr_to,
                addr_from: sp.addr_local,
                if_index: sp.if_index_local,
                log_cid: sp.initial_connection_id,
                last_connection: None,
                send_msg_size: None,
            });
        }

        let token = self
            .connections
            .iter()
            .filter(|cnx| cnx.next_wake_time <= current_time)
            .filter_map(|cnx| cnx.own_token.map(|tok| (tok, cnx.next_wake_time)))
            .min_by_key(|(_, wake)| wake.ticks())
            .map(|(tok, _)| tok);

        let Some(token) = token else {
            return Ok(PreparedPacket {
                send_length: 0,
                addr_to: default_addr,
                addr_from: default_addr,
                if_index: -1,
                log_cid: ConnectionId::default(),
                last_connection: None,
                send_msg_size: None,
            });
        };

        let log_cid = self
            .connections
            .get(token)
            .map(|cnx| cnx.initial_connection_id)
            .unwrap_or_default();
        let prepared = {
            let cnx = self
                .connections
                .get_mut(token)
                .ok_or(Error::InvalidArgument)?;
            cnx.prepare_packet_ex(current_time, send_buffer)
        };

        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(Error::Disconnected) => {
                let is_client = self
                    .connections
                    .get(token)
                    .map(|cnx| cnx.client_mode)
                    .unwrap_or(true);
                if is_client {
                    self.reinsert_by_wake_time_token(token, Instant::from_ticks(u64::MAX));
                } else {
                    self.delete_connection(token);
                }
                return Ok(PreparedPacket {
                    send_length: 0,
                    addr_to: default_addr,
                    addr_from: default_addr,
                    if_index: -1,
                    log_cid,
                    last_connection: None,
                    send_msg_size: None,
                });
            }
            Err(error) => return Err(error),
        };

        let if_index = if prepared.if_index == -1 {
            self.connections
                .get(token)
                .map(|cnx| cnx.local_if_index() as i32)
                .unwrap_or(-1)
        } else {
            prepared.if_index
        };
        let send_length = prepared.send_length;
        let addr_to = prepared.addr_to;
        let addr_from = prepared.addr_from;
        let send_msg_size = prepared.send_msg_size;
        let last_connection = self.connections.get_mut(token);

        Ok(PreparedPacket {
            send_length,
            addr_to,
            addr_from,
            if_index,
            log_cid,
            last_connection,
            send_msg_size,
        })
    }
```

## `picoquic/sender.c:picoquic_queue_for_retransmit`
* Phase 4C status: `suspect`
* Phase 4C rationale: C selects path_x pkt_ctx for multipath 1-RTT, appends to a pending list, and updates bytes_in_transit/pacing for non-ack-trap; Rust inserts into self.pkt_ctx[pc].pending and does not show congestion or pacing updates.
* C source: `picoquic/sender.c:1000-1031`
* C signature: `void picoquic_queue_for_retransmit(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, size_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:5543-5601`
* Rust item: `queue_for_retransmit`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = NULL;
    
    if (packet->ptype == picoquic_packet_1rtt_protected && cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }
    else {
        pkt_ctx = &cnx->pkt_ctx[packet->pc];
    }

    /* Manage the double linked packet list for retransmissions */
    packet->packet_next = NULL;
    if (pkt_ctx->pending_last == NULL) {
        packet->packet_previous = NULL;
        pkt_ctx->pending_first = packet;
    } else {
        packet->packet_previous = pkt_ctx->pending_last;
        packet->packet_previous->packet_next = packet;
    }
    pkt_ctx->pending_last = packet;
    packet->is_queued_for_retransmit = 1;

    if (!packet->is_ack_trap) {
        /* Account for bytes in transit, for congestion control */
        path_x->bytes_in_transit += length;
        path_x->is_cc_data_updated = 1;
        /* Update the pacing data */
        picoquic_update_pacing_after_send(path_x, length, current_time);
    }
}
```

### Rust body
```rust
    ) {
        packet.length = length;
        packet.send_time = current_time;
        packet.send_path = Some(PathToken::synthetic(
            path_x.unique_path_id as u32,
            path_x.unique_path_id as u32,
        ));
        packet.is_queued_for_retransmit = true;
        packet.is_queued_to_path = false;
        let pc = packet.packet_context as usize;
        let sequence = packet.sequence_number;
        if let Ok(token) = self.queued_packets.insert(core::mem::replace(
            packet,
            Packet {
                queue_data_repeat_membership: None,
                send_path: None,
                sequence_number: 0,
                send_time: current_time,
                delivered_prior: 0,
                delivered_time_prior: current_time,
                delivered_sent_prior: 0,
                lost_prior: 0,
                inflight_prior: 0,
                data_repeat_frame: 0,
                data_repeat_index: 0,
                data_repeat_priority: 0,
                data_repeat_stream_id: 0,
                data_repeat_stream_offset: 0,
                data_repeat_stream_data_length: 0,
                length: 0,
                checksum_overhead: 0,
                offset: 0,
                packet_type: PacketType::Error,
                packet_context: PacketContext::Application,
                is_evaluated: false,
                is_ack_eliciting: false,
                is_mtu_probe: false,
                is_multipath_probe: false,
                is_ack_trap: false,
                delivered_app_limited: false,
                sent_cwin_limited: false,
                is_preemptive_repeat: false,
                was_preemptively_repeated: false,
                is_queued_to_path: false,
                is_queued_for_retransmit: false,
                is_queued_for_spurious_detection: false,
                is_queued_for_data_repeat: false,
                bytes: [0u8; MAX_PACKET_SIZE],
            },
        )) {
            self.pkt_ctx[pc].pending.insert(sequence, token);
        }
    }
```
