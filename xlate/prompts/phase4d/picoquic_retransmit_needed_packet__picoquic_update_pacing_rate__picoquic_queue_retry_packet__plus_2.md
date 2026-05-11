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

## `picoquic/loss_recovery.c:picoquic_retransmit_needed_packet`
* Phase 4C status: `suspect`
* Phase 4C rationale: Most visible branches correspond, but the Rust body gates path retransmission bookkeeping on kept.is_some(), whereas the C body performs that bookkeeping after a non-pure-ACK retransmission without an equivalent visible old_p null check.
* C source: `picoquic/loss_recovery.c:297-533`
* C signature: `size_t picoquic_retransmit_needed_packet(picoquic_cnx_t *, picoquic_packet_context_t *, picoquic_packet_t *, picoquic_packet_context_enum, picoquic_path_t *, uint64_t, uint64_t *, picoquic_packet_t *, size_t, size_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:11568-11820`
* Rust item: `retransmit_needed_packet`

### C body
```c
{
    size_t length = 0;
    *continue_next = 0;

    /* TODO: while packets are pure ACK, drop them from retransmit queue */
    picoquic_path_t* old_path = old_p->send_path; /* should be the path on which the packet was transmitted */

    int is_probably_lost = 0;
    int is_timer_expired = 0;
    uint64_t next_retransmit_time = *next_wake_time;

    length = 0;

    if (old_p->ptype == picoquic_packet_0rtt_protected && cnx->cnx_state < picoquic_state_client_ready_start) {
        /* Special case: 0RTT cannot be acked before handshake is complete */
        *continue_next = 1;
        return 0;
    }
    is_probably_lost = cnx->initial_repeat_needed || old_p->send_path == NULL ||
        picoquic_is_packet_probably_lost(cnx, old_p, current_time, &next_retransmit_time, &is_timer_expired);

    if (is_probably_lost && (old_p->ptype == picoquic_packet_initial || old_p->ptype == picoquic_packet_handshake)) {
        /* Need to verify that there is no coalescing issue! */
        if (old_p->length > send_buffer_max) {
            picoquic_log_app_message(cnx, "Delay retransmission in type %d, seq %" PRIu64 ", buffer too small",
                old_p->ptype, old_p->sequence_number);
            return 0;
        }
    }

    if (is_probably_lost) {
        if (old_p->is_ack_trap) {
            picoquic_dequeue_retransmit_packet(cnx, pkt_ctx, old_p, 1, 0);
            *continue_next = 1;
        }
        else {
            /* check if this is an ACK only packet */
            int packet_is_pure_ack = 1;

            /* Parse the old packet, queue frames for retransmit, perhaps copy some
             * frames into the new packet, dequeue from packet queue and if needed
             * copy to "retransmitted", return old_p = 0 if freed.
             */
            old_p = picoquic_process_lost_packet(cnx, pkt_ctx, old_p, 0, pc, path_x, current_time,
                packet, send_buffer_max, &length, &packet_is_pure_ack, header_length);

            /* If there was no frame copied in the packet, tell the caller to continue the loop. */
            if (old_p == NULL || packet_is_pure_ack) {
                length = 0;
                *continue_next = 1;
            }
            /* Also continue the loop if the packet is zero length. This is a tradeoff, because
             * in some circumstances it may cause an increase in memory consumption.
             * Consider limiting this to cases when "bytes in transit" is larger than "CWIN".
             */
            if (length == 0) {
                *continue_next = 1;
            }
        }
    }
    else if (!is_timer_expired) {
        /*
        * Always retransmit in order. If not this one, then nothing.
        * But make an exception for 0-RTT packets.
        */
        if (old_p->ptype == picoquic_packet_0rtt_protected) {
            *continue_next = 1;
        }
        else {
            if (next_retransmit_time < *next_wake_time) {
                *next_wake_time = next_retransmit_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_LOSS_RECOVERY);
            }
            /* Will not continue */
            *continue_next = 0;
        }
    }
    else if (cnx->cnx_state <= picoquic_state_client_ready_start) {
        /* We do not follow the PTO logic before the connection is complete */
        int packet_is_pure_ack;
        if (old_path != NULL &&
            (((old_p->sequence_number > pkt_ctx->highest_acknowledged || pkt_ctx->highest_acknowledged == UINT64_MAX) &&
            old_p->send_time > old_path->last_loss_event_detected) ||
            old_path->last_loss_event_detected == 0)){
            old_path->nb_retransmit++;
            old_path->last_loss_event_detected = current_time;
        }
        old_p = picoquic_process_lost_packet(cnx, pkt_ctx, old_p, 1, pc, path_x, current_time,
            packet, send_buffer_max, &length, &packet_is_pure_ack, header_length);
        if (old_p == NULL || packet_is_pure_ack) {
            length = 0;
            *continue_next = 1;
        }
    }
    else {
        /* The timer is expired */

        /* Evaluate whether this packet did in fact require an acknowledgement */
        if (!picoquic_is_packet_ack_eliciting(old_p)) {
            /* if the packet did not require an acknowledgement, it can be safely
             * removed from the queue, and processing will move to the next packet.
             */

            /* If ack only packets are lost, bundle a ping next time an ACK is sent on that path */
            if (old_p->send_path != NULL && cnx->is_multipath_enabled) {
                old_p->send_path->is_ack_lost = 1;
            }
            picoquic_count_and_notify_loss(cnx, old_p, 2, current_time);
            picoquic_dequeue_retransmit_packet(cnx, pkt_ctx, old_p, 1, 0);
            length = 0;
            *continue_next = 1;
        }
        else if (old_path->is_pto_required && path_x == old_path) {
            /* A previous iteration of this loop requested a PTO repeat, and
             * the repeat has not happened yet. Just wait.
             */
            length = 0;
            *continue_next = 0;
        }
        else {
            /* We need to send a PTO. */
            int packet_is_pure_ack = 1;
            /* Parse the old packet, queue frames for retransmit, perhaps copy some
            * frames into the new packet, dequeue from packet queue and if needed
            * copy to "retransmitted", return old_p = 0 if freed.
            */
            /* TODO: there may be a special case for multipath, in which the
            * management of repeats is different */
            old_p = picoquic_process_lost_packet(cnx, pkt_ctx, old_p, 1, pc, path_x, current_time,
                packet, send_buffer_max, &length,
                &packet_is_pure_ack, header_length);

            if (packet_is_pure_ack) {
                /* this could happen if there is nothing to copy. */
                length = 0;
                *continue_next = 1;
            }
            else {
                /* we did perform a repetition */
                /* First, keep track of retransmissions per path, in order to
                * manage scheduling in multipath setup */

                if (old_path != NULL) {
                    if (path_x == old_path) {
                        old_path->is_pto_required = 1;
                    }
                    old_path->nb_retransmit++;
                    old_path->last_loss_event_detected = current_time;
                    if (cnx->is_multipath_enabled && cnx->nb_paths > 1) {
                        picoquic_retransmit_path_packet_queue(cnx, pkt_ctx, current_time);
                    }
                    if (old_path->nb_retransmit > 9 &&
                        cnx->cnx_state >= picoquic_state_ready) {
                        /* Max retransmission reached for this path */
                        DBG_PRINTF("%s\n", "Too many data retransmits, abandon path");
                        picoquic_log_app_message(cnx, "%s", "Too many data retransmits (%"PRIu64"), abandon path %" PRIu64,
                            old_path->nb_retransmit, old_path->unique_path_id);

                        if (cnx->is_multipath_enabled) {
                            int all_paths_dubious = 1;
                            for (int path_id = 0; path_id < cnx->nb_paths; path_id++) {
                                if (cnx->path[path_id]->nb_retransmit == 0) {
                                    all_paths_dubious = 0;
                                    break;
                                }
                            }
                            if (!all_paths_dubious) {
                                old_path->first_tuple->challenge_failed = 1;
                                cnx->path_demotion_needed = 1;
                            }
                        }
                        else {
                            old_path->first_tuple->challenge_failed = 1;
                            cnx->path_demotion_needed = 1;
                        }
                    }
                }
                /* Then, manage the total number of retransmissions across all paths. */
                if ((old_path == NULL || old_path->nb_retransmit > 9) &&
                    cnx->cnx_state >= picoquic_state_ready) {
                    /* TODO: only disconnect if there is no other available path */
                    int all_paths_bad = 1;
                    if (cnx->is_multipath_enabled) {
                        for (int path_id = 0; path_id < cnx->nb_paths; path_id++) {
                            if (cnx->path[path_id]->nb_retransmit <= 9) {
                                all_paths_bad = 0;
                                break;
                            }
                        }
                    }
                    if (all_paths_bad) {
                        /*
                        * Max retransmission count was exceeded. Log.
                        */
                        DBG_PRINTF("Too many retransmits of packet number %"PRIu64", disconnect", old_p->sequence_number);
                        picoquic_log_app_message(cnx, "Too many retransmits of packet number %"PRIu64", disconnect", old_p->sequence_number);

                        *continue_next = 0;
                    }
                }
            }
#ifdef TODO_CHECK_IF_PACING_ALSO_NEEDED_IN_ALL_CASES
            if (length <= packet->offset) {
                length = 0;
                packet->length = 0;
                packet->offset = 0;
                if (!packet_is_pure_ack) {
                    /* Pace down the next retransmission so as to not pile up error upon error.
                    * We only do that if theree are enough tokens in the bucket to allow at least
                    * one packet out. Otherwise, there is a risk of creating a waiting loop that
                    * only stops when all queued packets have been processed.
                    */
                    if (path_x->pacing_bucket_nanosec > path_x->pacing_packet_time_nanosec) {
                        path_x->pacing_bucket_nanosec -= path_x->pacing_packet_time_nanosec;
                    }
                }
                /*
                * If the loop is continuing, this means that we need to look
                * at the next candidate packet.
                */
                *continue_next = (timer_based_retransmit == 0);
            }
            else {
                *continue_next = 0;
            }
#endif
        }
    }

    return length;
}
```

### Rust body
```rust
    ) -> usize {
        let Some(old_p) = self
            .queued_packets
            .get(old_token)
            .map(PacketRetransmitSnapshot::from)
        else {
            *continue_next = 1;
            return 0;
        };
        let mut length = 0usize;
        *continue_next = 0;

        let old_path_idx = old_p
            .send_path
            .and_then(|send_path| self.path_index_from_token(send_path));
        if old_p.packet_type == PacketType::ZeroRttProtected
            && self.connection_state < State::ClientReadyStart
        {
            *continue_next = 1;
            return 0;
        }

        let mut is_timer_expired = false;
        let mut next_retransmit_time = *next_wake_time;
        let is_probably_lost = self.initial_repeat_needed
            || old_p.send_path.is_none()
            || self.is_packet_probably_lost(
                &old_p,
                current_time,
                &mut next_retransmit_time,
                &mut is_timer_expired,
            );

        if is_probably_lost
            && (old_p.packet_type == PacketType::Initial
                || old_p.packet_type == PacketType::Handshake)
            && old_p.length > send_buffer_max
        {
            crate::logger::Log::app_message(
                self,
                format_args!(
                    "Delay retransmission in type {:?}, seq {}, buffer too small",
                    old_p.packet_type, old_p.sequence_number
                ),
            );
            return 0;
        }

        if is_probably_lost {
            if old_p.is_ack_trap {
                self.dequeue_retransmit_packet_in_context(selection, old_token, true, false);
                *continue_next = 1;
            } else {
                let mut packet_is_pure_ack = 1;
                let kept = self.process_lost_packet(
                    selection,
                    old_token,
                    0,
                    pc,
                    path_x,
                    current_time,
                    Some(packet),
                    send_buffer_max,
                    &mut length,
                    &mut packet_is_pure_ack,
                    header_length,
                );
                if kept.is_none() || packet_is_pure_ack != 0 {
                    length = 0;
                    *continue_next = 1;
                }
                if length == 0 {
                    *continue_next = 1;
                }
            }
        } else if !is_timer_expired {
            if old_p.packet_type == PacketType::ZeroRttProtected {
                *continue_next = 1;
            } else {
                if next_retransmit_time < *next_wake_time {
                    *next_wake_time = next_retransmit_time;
                }
                *continue_next = 0;
            }
        } else if self.connection_state <= State::ClientReadyStart {
            if let Some(path_idx) = old_path_idx {
                let highest_acknowledged =
                    self.packet_context_state(selection).highest_acknowledged;
                let path = &mut self.paths[path_idx];
                if ((old_p.sequence_number > highest_acknowledged
                    || highest_acknowledged == u64::MAX)
                    && old_p.send_time > path.last_loss_event_detected)
                    || path.last_loss_event_detected.ticks() == 0
                {
                    path.nb_retransmit = path.nb_retransmit.saturating_add(1);
                    path.last_loss_event_detected = current_time;
                }
            }
            let mut packet_is_pure_ack = 1;
            let kept = self.process_lost_packet(
                selection,
                old_token,
                1,
                pc,
                path_x,
                current_time,
                Some(packet),
                send_buffer_max,
                &mut length,
                &mut packet_is_pure_ack,
                header_length,
            );
            if kept.is_none() || packet_is_pure_ack != 0 {
                length = 0;
                *continue_next = 1;
            }
        } else if !self.packet_token_is_ack_eliciting(old_token) {
            if let Some(path_idx) = old_path_idx
                && self.is_multipath_enabled
            {
                self.paths[path_idx].is_ack_lost = true;
            }
            self.count_and_notify_loss(&old_p, 2, current_time);
            self.dequeue_retransmit_packet_in_context(selection, old_token, true, false);
            length = 0;
            *continue_next = 1;
        } else if let Some(path_idx) = old_path_idx {
            if self.paths[path_idx].is_pto_required
                && path_x.unique_path_id == self.paths[path_idx].unique_path_id
            {
                length = 0;
                *continue_next = 0;
            } else {
                let mut packet_is_pure_ack = 1;
                let kept = self.process_lost_packet(
                    selection,
                    old_token,
                    1,
                    pc,
                    path_x,
                    current_time,
                    Some(packet),
                    send_buffer_max,
                    &mut length,
                    &mut packet_is_pure_ack,
                    header_length,
                );

                if packet_is_pure_ack != 0 {
                    length = 0;
                    *continue_next = 1;
                } else {
                    if kept.is_some() {
                        let path_is_same =
                            path_x.unique_path_id == self.paths[path_idx].unique_path_id;
                        if path_is_same {
                            self.paths[path_idx].is_pto_required = true;
                        }
                        self.paths[path_idx].nb_retransmit =
                            self.paths[path_idx].nb_retransmit.saturating_add(1);
                        self.paths[path_idx].last_loss_event_detected = current_time;
                        if self.is_multipath_enabled && self.paths.len() > 1 {
                            self.retransmit_path_packet_queue(selection, current_time);
                        }
                        if self.paths[path_idx].nb_retransmit > 9
                            && self.connection_state >= State::Ready
                        {
                            let nb_retransmit = self.paths[path_idx].nb_retransmit;
                            let unique_path_id = self.paths[path_idx].unique_path_id;
                            crate::logger::Log::app_message(
                                self,
                                format_args!(
                                    "Too many data retransmits ({}), abandon path {}",
                                    nb_retransmit, unique_path_id
                                ),
                            );
                            if self.is_multipath_enabled {
                                let all_paths_dubious =
                                    self.paths.iter().all(|path| path.nb_retransmit != 0);
                                if !all_paths_dubious {
                                    if let Some(tuple) = self.paths[path_idx].tuples.first_mut() {
                                        tuple.challenge_failed = true;
                                    }
                                    self.path_demotion_needed = true;
                                }
                            } else {
                                if let Some(tuple) = self.paths[path_idx].tuples.first_mut() {
                                    tuple.challenge_failed = true;
                                }
                                self.path_demotion_needed = true;
                            }
                        }
                    }

                    if self
                        .paths
                        .get(path_idx)
                        .is_none_or(|path| path.nb_retransmit > 9)
                        && self.connection_state >= State::Ready
                    {
                        let all_paths_bad = if self.is_multipath_enabled {
                            self.paths.iter().all(|path| path.nb_retransmit > 9)
                        } else {
                            true
                        };
                        if all_paths_bad {
                            crate::logger::Log::app_message(
                                self,
                                format_args!(
                                    "Too many retransmits of packet number {}, disconnect",
                                    old_p.sequence_number
                                ),
                            );
                            *continue_next = 0;
                        }
                    }
                }
            }
        } else {
            let mut packet_is_pure_ack = 1;
            let kept = self.process_lost_packet(
                selection,
                old_token,
                1,
                pc,
                path_x,
                current_time,
                Some(packet),
                send_buffer_max,
                &mut length,
                &mut packet_is_pure_ack,
                header_length,
            );
            if kept.is_none() || packet_is_pure_ack != 0 {
                length = 0;
                *continue_next = 1;
            }
        }

        length
    }
```

## `picoquic/pacing.c:picoquic_update_pacing_rate`
* Phase 4C status: `suspect`
* Phase 4C rationale: C passes path_x as the signalled_path argument to update_pacing_parameters; Rust passes None.
* C source: `picoquic/pacing.c:259-264`
* C signature: `void picoquic_update_pacing_rate(picoquic_path_t *, double, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:6093-6096`
* Rust item: `update_pacing_rate`

### C body
```c
{
    picoquic_update_pacing_parameters(&path_x->pacing, pacing_rate,
        quantum, path_x->send_mtu, path_x->smoothed_rtt, path_x);
}
```

### Rust body
```rust
    pub fn update_pacing_rate(&mut self, pacing_rate: f64, quantum: u64) {
        self.pacing
            .update_parameters(pacing_rate, quantum, self.send_mtu, self.smoothed_rtt, None);
    }
```

## `picoquic/packet.c:picoquic_queue_retry_packet`
* Phase 4C status: `suspect`
* Phase 4C rationale: C maps any retry-token preparation failure to PICOQUIC_ERROR_MEMORY, while Rust returns parse_error_status(error).
* C source: `picoquic/packet.c:1210-1238`
* C signature: `int picoquic_queue_retry_packet(picoquic_quic_t *, const struct sockaddr *, const struct sockaddr *, int, picoquic_packet_header *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:6035-6067`
* Rust item: `queue_retry_packet`

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

## `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_config`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets both cnx_id_callback_fn and cnx_id_callback_ctx on success; Rust sets local_connection_id_length and connection_id_callback_ctx but no visible assignment to connection_id_callback_fn.
* C source: `picoquic/picoquic_lb.c:389-495`
* C signature: `int picoquic_lb_compat_cid_config(picoquic_quic_t *, picoquic_load_balancer_config_t *)`
* Rust source: `rs/fq/src/lb.rs:535-611`
* Rust item: `set_lb_cid_config`

### C body
```c
{
    int ret = 0;

    if (quic->cnx_list != NULL && quic->local_cnxid_length != lb_config->connection_id_length) {
        /* Error. Changing the CID length now will break existing connections */
        ret = -1;
    }
    else if (quic->cnx_id_callback_fn != NULL && quic->cnx_id_callback_ctx != NULL){
        /* Error. Some other CID generation is configured, cannot be changed */
        ret = -1;
    }
    else {
        /* Verify that the method is supported and the parameters are compatible.
         * If valid, configure the connection ID generation */
        if (lb_config->connection_id_length > PICOQUIC_CONNECTION_ID_MAX_SIZE) {
            ret = -1;
        }
        else {
            switch (lb_config->method) {
            case picoquic_load_balancer_cid_clear:
                if (lb_config->server_id_length + 1 > lb_config->connection_id_length) {
                    ret = -1;
                }
                break;
            case picoquic_load_balancer_cid_stream_cipher:
                /* Nonce length must be 8 to 16 bytes, CID should be long enough */
                if (lb_config->nonce_length < 8 || lb_config->nonce_length > 16 ||
                    lb_config->nonce_length + lb_config->server_id_length + 1 > lb_config->connection_id_length) {
                    ret = -1;
                }
                break;
            case picoquic_load_balancer_cid_block_cipher:
                /* CID should include a whole AES-ECB block,
                 * there should be at least 2 bytes available for uniqueness,
                 * zero padding length should be 4 bytes for security */
                if (lb_config->connection_id_length < 17 ||
                    lb_config->server_id_length > 15) {
                    ret = -1;
                }
                break;
            default:
                /* Error, unknown method */
                ret = -1;
                break;
            }
        }
        if (ret == 0) {
            /* Create a copy */
            picoquic_load_balancer_cid_context_t* lb_ctx = (picoquic_load_balancer_cid_context_t*)malloc(sizeof(picoquic_load_balancer_cid_context_t));

            if (lb_ctx == NULL) {
                ret = -1;
            }
            else {
                /* if allocated, create the necessary encryption contexts or variables */
                uint64_t s_id64 = lb_config->server_id64;
                memset(lb_ctx, 0, sizeof(picoquic_load_balancer_cid_context_t));
                lb_ctx->method = lb_config->method;
                lb_ctx->rotation_bits = lb_config->rotation_bits;
                lb_ctx->first_byte_encodes_length = lb_config->first_byte_encodes_length;
                lb_ctx->server_id_length = lb_config->server_id_length;
                lb_ctx->nonce_length = lb_config->nonce_length;
                lb_ctx->connection_id_length = lb_config->connection_id_length;
                lb_ctx->server_id64 = lb_config->server_id64;
                lb_ctx->cid_encryption_context = NULL;
                lb_ctx->cid_decryption_context = NULL;
                /* Compute the server ID bytes and set encryption contexts */
                for (size_t i = 0; i < lb_ctx->server_id_length; i++) {
                    size_t j = lb_ctx->server_id_length - i - 1;
                    lb_ctx->server_id[j] = (uint8_t)s_id64;
                    s_id64 >>= 8;
                }
                if (s_id64 != 0) {
                    /* Server ID not long enough to encode actual value */
                    ret = -1;
                } else if (lb_config->method == picoquic_load_balancer_cid_stream_cipher ||
                    lb_config->method == picoquic_load_balancer_cid_block_cipher) {
                    lb_ctx->cid_encryption_context = picoquic_aes128_ecb_create(1, lb_config->cid_encryption_key);
                    if (lb_ctx->cid_encryption_context == NULL) {
                        ret = -1;
                    }
                    else if (lb_config->method == picoquic_load_balancer_cid_block_cipher) {
                        lb_ctx->cid_decryption_context = picoquic_aes128_ecb_create(0, lb_config->cid_encryption_key);
                        if (lb_ctx->cid_decryption_context == NULL) {
                            picoquic_aes128_ecb_free(lb_ctx->cid_encryption_context);
                            lb_ctx->cid_encryption_context = NULL;
                            ret = -1;
                        }
                    }
                }
                if (ret != 0) {
                    /* if context allocation failed, free the copy */
                    free(lb_ctx);
                    lb_ctx = NULL;
                } else {
                    /* Configure the CID generation */
                    quic->local_cnxid_length = lb_ctx->connection_id_length;
                    quic->cnx_id_callback_fn = picoquic_lb_compat_cid_generate;
                    quic->cnx_id_callback_ctx = (void*)lb_ctx;
                }
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn set_lb_cid_config(&mut self, lb_config: &Config) -> Result<(), Error> {
        if !self.connections.is_empty()
            && self.local_connection_id_length as usize != lb_config.connection_id_length
        {
            return Err(Error::InvalidState);
        }
        if self.connection_id_callback_fn.is_some() && self.connection_id_callback_ctx.is_some() {
            return Err(Error::InvalidState);
        }
        if lb_config.connection_id_length > CONNECTION_ID_MAX_SIZE {
            return Err(Error::InvalidArgument);
        }
        match lb_config.method {
            ConnectionIdMethod::Clear => {
                if lb_config.server_id_length + 1 > lb_config.connection_id_length {
                    return Err(Error::InvalidArgument);
                }
            }
            ConnectionIdMethod::StreamCipher => {
                if lb_config.nonce_length < 8
                    || lb_config.nonce_length > 16
                    || lb_config.nonce_length + lb_config.server_id_length + 1
                        > lb_config.connection_id_length
                {
                    return Err(Error::InvalidArgument);
                }
            }
            ConnectionIdMethod::BlockCipher => {
                if lb_config.connection_id_length < 17 || lb_config.server_id_length > 15 {
                    return Err(Error::InvalidArgument);
                }
            }
        }

        // Encode server_id as big-endian bytes.
        let mut server_id_encoded = [0u8; 16];
        let mut s_id64 = lb_config.server_id;
        for i in 0..lb_config.server_id_length {
            let j = lb_config.server_id_length - i - 1;
            server_id_encoded[j] = s_id64 as u8;
            s_id64 >>= 8;
        }
        if s_id64 != 0 {
            return Err(Error::InvalidArgument);
        }

        let cid_encryption_context = match lb_config.method {
            ConnectionIdMethod::StreamCipher | ConnectionIdMethod::BlockCipher => Some(Box::new(
                Aes128EcbContext::new(true, &lb_config.cid_encryption_key),
            )),
            ConnectionIdMethod::Clear => None,
        };
        let cid_decryption_context = match lb_config.method {
            ConnectionIdMethod::BlockCipher => Some(Box::new(Aes128EcbContext::new(
                false,
                &lb_config.cid_encryption_key,
            ))),
            _ => None,
        };

        let context = ConnectionIdContext {
            method: lb_config.method,
            rotation_bits: lb_config.rotation_bits,
            first_byte_encodes_length: lb_config.first_byte_encodes_length,
            server_id_length: lb_config.server_id_length,
            nonce_length: lb_config.nonce_length,
            connection_id_length: lb_config.connection_id_length,
            server_id: lb_config.server_id,
            server_id_encoded,
            cid_encryption_context,
            cid_decryption_context,
        };

        self.local_connection_id_length = lb_config.connection_id_length as u8;
        self.connection_id_callback_ctx = Some(Box::new(context));
        Ok(())
    }
```

## `picoquic/picoquic_ptls_openssl.c:openssl_keyex_from_key_file`
* Phase 4C status: `suspect`
* Phase 4C rationale: C reads a private key then calls ptls_openssl_create_key_exchange; Rust reads and parses the key but does not visibly perform the same key-exchange creation call.
* C source: `picoquic/picoquic_ptls_openssl.c:342-365`
* C signature: `int openssl_keyex_from_key_file(ptls_key_exchange_context_t **, const char *)`
* Rust source: `rs/fq/src/sys/openssl.rs:313-318`
* Rust item: `openssl_keyex_from_key_file`

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
