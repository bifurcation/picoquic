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

## Pair `picoquic/packet.c:picoquic_ignore_incoming_handshake`
C: `picoquic/packet.c:1345-1387 picoquic_ignore_incoming_handshake`
Rust: `rs/fq/src/lib.rs:7020-7058 ignore_incoming_handshake`

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

## Pair `picoquic/packet.c:picoquic_incoming_server_handshake`
C: `picoquic/packet.c:1704-1752 picoquic_incoming_server_handshake`
Rust: `rs/fq/src/lib.rs:3289-3338 incoming_packet_ex`

### C body
```c
{
    int ret = 0;
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(addr_to);
    UNREFERENCED_PARAMETER(if_index_to);
#endif
    int restricted = cnx->cnx_state != picoquic_state_client_handshake_start;
    
    if (picoquic_compare_connection_id(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id, &ph->srce_cnx_id) != 0) {
        ret = PICOQUIC_ERROR_CNXID_CHECK; /* protocol error */
    }


    if (ret == 0) {
        if (cnx->cnx_state < picoquic_state_ready) {
            /* Accept the incoming frames */

            if (ph->payload_length == 0) {
                /* empty payload! */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
            else {
                ret = picoquic_decode_frames(cnx, cnx->path[0],
                    bytes + ph->offset, ph->payload_length,received_data,
                    ph->epoch, NULL, addr_to, ph->pn64, 0, current_time);
            }

            /* processing of initial packet */
            if (ret == 0 && restricted == 0) {
                ret = picoquic_tls_stream_process(cnx, NULL, current_time);
            }
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

## Pair `picoquic/packet.c:picoquic_ecn_accounting`
C: `picoquic/packet.c:1880-1908 picoquic_ecn_accounting`
Rust: `rs/fq/src/lib.rs:5534-5557 ecn_accounting`

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

## Pair `picoquic/packet.c:picoquic_incoming_packet_ex`
C: `picoquic/packet.c:2389-2430 picoquic_incoming_packet_ex`
Rust: `rs/fq/src/lib.rs:3289-3338 incoming_packet_ex`

### C body
```c
{
    size_t consumed_index = 0;
    int ret = 0;
    picoquic_connection_id_t previous_destid = picoquic_null_connection_id;
    PICOQUIC_THREAD_CHECK(quic);

    while (consumed_index < packet_length) {
        size_t consumed = 0;

        ret = picoquic_incoming_segment(quic, bytes + consumed_index, 
            packet_length - consumed_index, packet_length,
            &consumed, addr_from, addr_to, if_index_to, received_ecn, current_time, current_time,
            &previous_destid, first_cnx);

        if (ret == 0) {
            consumed_index += consumed;
            if (consumed == 0) {
                DBG_PRINTF("%s", "Receive bug, ret = 0 && consumed = 0\n");
                break;
            }
        } else {
            ret = 0;
            break;
        }
    }

    if (*first_cnx != NULL && packet_length > (*first_cnx)->max_mtu_received) {
        (*first_cnx)->max_mtu_received = packet_length;
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

## Pair `picoquic/paths.c:picoquic_prepare_tuple_challenge_frames`
C: `picoquic/paths.c:33-138 picoquic_prepare_tuple_challenge_frames`
Rust: `rs/fq/src/internal.rs:4498-4591 prepare_tuple_challenge_frames`

### C body
```c
{
    if (tuple->challenge_verified == 0 && tuple->challenge_failed == 0) {
        uint64_t next_challenge_time = picoquic_tuple_challenge_time(path_x, tuple, current_time);

        if (next_challenge_time > current_time) {
            if (next_challenge_time < *next_wake_time) {
                *next_wake_time = next_challenge_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
            }
        }
        else {
            uint8_t* bytes_challenge = bytes_next;

            if (tuple->challenge_repeat_count < PICOQUIC_CHALLENGE_REPEAT_MAX) {
                /* When blocked, repeat the path challenge or wait */

                bytes_next = picoquic_format_path_challenge_frame(bytes_next, bytes_max, more_data, is_pure_ack,
                    tuple->challenge[tuple->challenge_repeat_count]);
                if (bytes_next > bytes_challenge) {
                    tuple->challenge_time = current_time;
                    tuple->challenge_repeat_count++;
                    if (!tuple->is_nat_rebinding) {
                        if (cnx->client_mode || ((path_x->bytes_sent + PICOQUIC_ENFORCED_INITIAL_MTU) <= path_x->received)) {
                            *is_challenge_padding_needed = 1;
                        }
                        else {
                            /* Sending a full size packet would defeat the amplification limits, so we take
                             * advantage of the escape clause in RFC 9000, "An endpoint MUST expand datagrams
                             * that contain a PATH_CHALLENGE frame to at least the smallest allowed maximum
                             * datagram size of 1200 bytes, unless the anti-amplification limit for the path
                             * does not permit sending a datagram of this size."
                             */
                            *is_challenge_padding_needed = 0;
                        }
                    }
                    else {
                        /* never pad the packets sent in response to NAT rebinding. */
                        *is_challenge_padding_needed = 0;
                    }
                }

                /* Reset the next challenge time to match the new challenge count */
                next_challenge_time = picoquic_tuple_challenge_time(path_x, tuple, current_time);
                if (next_challenge_time < *next_wake_time) {
                    *next_wake_time = next_challenge_time;
                    SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                }
            }
            else {
                /* This particular tuple failed.
                 * Update its status, and move it to the end of the list.
                 */
                picoquic_tuple_t* next_tuple = path_x->first_tuple;
                picoquic_tuple_t* previous_tuple = NULL;

                tuple->challenge_failed = 1;
                tuple->demotion_time = current_time + (path_x->retransmit_timer << PICOQUIC_CHALLENGE_REPEAT_MAX);

                while (next_tuple != NULL) {
                    if (next_tuple == tuple) {
                        if (previous_tuple == NULL) {
                            path_x->first_tuple = next_tuple->next_tuple;
                            next_tuple = path_x->first_tuple;
                        }
                        else
                        {
                            previous_tuple->next_tuple = next_tuple->next_tuple;
                            next_tuple = previous_tuple->next_tuple;
                        }
                    }
                    else {
                        previous_tuple = next_tuple;
                        next_tuple = next_tuple->next_tuple;

                    }
                }
                if (previous_tuple == NULL) {
                    path_x->first_tuple = tuple;
                }
                else
                {
                    previous_tuple->next_tuple = tuple;
                }
                tuple->next_tuple = NULL;
            }
        }
    }

    if (tuple->response_required) {
        uint8_t* bytes_response = bytes_next;
        if ((bytes_next = picoquic_format_path_response_frame(bytes_response, bytes_max,
            more_data, is_pure_ack, tuple->challenge_response)) > bytes_response) {
            tuple->response_required = 0;
            *is_challenge_padding_needed |= cnx->client_mode || ((path_x->bytes_sent + PICOQUIC_ENFORCED_INITIAL_MTU) <= path_x->received);
        }
    }

    /* TODO: consider adding an address discovery frame. */

    return bytes_next;
}
```

### Rust body
```rust
    ) -> Option<&'a mut [u8]> {
        if tuple_index >= path_x.tuples.len() {
            return Some(bytes);
        }

        let mut tail = bytes;
        let mut active_index = tuple_index;

        if !path_x.tuples[active_index].challenge_verified
            && !path_x.tuples[active_index].challenge_failed
        {
            let mut next_challenge_time =
                tuple_challenge_time(path_x, &path_x.tuples[active_index], current_time);

            if next_challenge_time > current_time {
                if next_challenge_time < *next_wake_time {
                    *next_wake_time = next_challenge_time;
                }
            } else if path_x.tuples[active_index].challenge_repeat_count
                < CHALLENGE_REPEAT_MAX as u8
            {
                let repeat_index = path_x.tuples[active_index].challenge_repeat_count as usize;
                let challenge = path_x.tuples[active_index].challenge[repeat_index];
                let before_len = tail.len();
                tail = format_path_challenge_frame(tail, more_data, is_pure_ack, challenge)?;
                if tail.len() < before_len {
                    let tuple = &mut path_x.tuples[active_index];
                    tuple.challenge_time = current_time;
                    tuple.challenge_repeat_count = tuple.challenge_repeat_count.saturating_add(1);
                    if tuple.is_nat_rebinding == 0 {
                        if self.client_mode
                            || path_x
                                .bytes_sent
                                .saturating_add(ENFORCED_INITIAL_MTU as u64)
                                <= path_x.received
                        {
                            *is_challenge_padding_needed = 1;
                        } else {
                            *is_challenge_padding_needed = 0;
                        }
                    } else {
                        *is_challenge_padding_needed = 0;
                    }
                }

                next_challenge_time =
                    tuple_challenge_time(path_x, &path_x.tuples[active_index], current_time);
                if next_challenge_time < *next_wake_time {
                    *next_wake_time = next_challenge_time;
                }
            } else {
                let mut tuple = path_x.tuples.remove(active_index);
                tuple.challenge_failed = true;
                let demotion_delay = path_x
                    .retransmit_timer
                    .ticks()
                    .checked_shl(CHALLENGE_REPEAT_MAX as u32)
                    .unwrap_or(u64::MAX);
                tuple.demotion_time =
                    Instant::from_ticks(current_time.ticks().saturating_add(demotion_delay));
                path_x.tuples.push(tuple);
                active_index = path_x.tuples.len() - 1;
            }
        }

        if path_x.tuples[active_index].response_required {
            let challenge_response = path_x.tuples[active_index].challenge_response;
            let before_len = tail.len();
            tail = format_path_response_frame(tail, more_data, is_pure_ack, challenge_response)?;
            if tail.len() < before_len {
                path_x.tuples[active_index].response_required = false;
                if self.client_mode
                    || path_x
                        .bytes_sent
                        .saturating_add(ENFORCED_INITIAL_MTU as u64)
                        <= path_x.received
                {
                    *is_challenge_padding_needed = 1;
                }
            }
        }

        Some(tail)
    }
```

## Pair `picoquic/paths.c:picoquic_delete_demoted_tuples`
C: `picoquic/paths.c:259-286 picoquic_delete_demoted_tuples`
Rust: `rs/fq/src/internal.rs:4349-4362 delete_demoted_tuples`

### C body
```c
{
    for (int path_index = 0; path_index < cnx->nb_paths; path_index++) {
        picoquic_path_t* path_x = cnx->path[path_index];
        if (!path_x->path_is_demoted) {
            /* examine each tuple record */
            picoquic_tuple_t* tuple = path_x->first_tuple;
            picoquic_tuple_t* next_tuple;

            while (tuple != NULL && (next_tuple = tuple->next_tuple) != NULL) {
                if (next_tuple->challenge_failed) {
                    if (current_time > next_tuple->demotion_time) {
                        picoquic_delete_tuple(path_x, next_tuple, 0);
                        continue;
                    }
                    else if (*next_wake_time > next_tuple->demotion_time) {
                        *next_wake_time = next_tuple->demotion_time;
                        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                    }
                }
                tuple = next_tuple;
            }
        }
    }
    cnx->tuple_demotion_needed = 0;
}
```

### Rust body
```rust
    pub fn delete_demoted_tuples(&mut self, current_time: Instant, next_wake_time: &mut Instant) {
        let mut retained = Vec::with_capacity(self.paths.len());
        for mut path in self.paths.drain(..) {
            if path.path_is_demoted && path.demotion_time <= current_time {
                path.tuples.clear();
            } else {
                if path.path_is_demoted && path.demotion_time < *next_wake_time {
                    *next_wake_time = path.demotion_time;
                }
                retained.push(path);
            }
        }
        self.paths = retained;
    }
```

## Pair `picoquic/paths.c:picoquic_select_next_path_tuple`
C: `picoquic/paths.c:488-556 picoquic_select_next_path_tuple`
Rust: `rs/fq/src/internal.rs:4627-4650 select_next_path_tuple`

### C body
```c
{
    int nb_available = 0;
    uint64_t min_retransmit = 0;

    *next_path = NULL;
    *next_tuple = NULL;

    /* First check whether path contol messages are needed */
    for (int path_index = 0; path_index < cnx->nb_paths; path_index++)
    {
        if (cnx->path[path_index]->path_is_demoted) {
            continue;
        }
        else if (cnx->is_multipath_enabled && cnx->path[path_index]->first_tuple->challenge_failed && !cnx->path[path_index]->path_abandon_sent) {
            (void)picoquic_abandon_path(cnx, cnx->path[path_index]->unique_path_id, PICOQUIC_TRANSPORT_UNSTABLE_INTERFACE, current_time);
        }
        else if ((*next_tuple = picoquic_check_path_control_needed(cnx, cnx->path[path_index], current_time, next_wake_time)) != NULL) {
            *next_path = cnx->path[path_index];
            (*next_path)->challenger++;
            break;
        }
        else if (cnx->nb_paths > 0 && cnx->path[path_index]->first_tuple->challenge_verified && cnx->path[path_index]->nb_retransmit > 0 &&
            cnx->cnx_state == picoquic_state_ready && cnx->path[path_index]->bytes_in_transit == 0) {
            cnx->path[path_index]->is_multipath_probe_needed = 1;
            *next_path = cnx->path[path_index];
            *next_tuple = (*next_path)->first_tuple;
            (*next_path)->challenger++;
            break;
        }
    }
    if (*next_path != NULL) {
        /* we are done */
    }
    else  if (cnx->nb_paths == 1) {
        /* No choice, just use this path -- this is the default if multipath is not selected. */
        *next_path = cnx->path[0];
        *next_tuple = (*next_path)->first_tuple;
    }
    else if ((nb_available = picoquic_verify_path_available(cnx, next_path, &min_retransmit, current_time)) < 2) {
        /* Only 0 or 1 path to chose from. Just select that. */
        if (*next_path == NULL) {
            *next_path = cnx->path[0];
        }
        *next_tuple = (*next_path)->first_tuple;
    }
    else {
        /* Several paths are available. We will chose from that, looking at
        * available path that can send ACK, or paced data, or congestion
        * controlled data.
         */
        picoquic_sort_available_paths(cnx, current_time, next_wake_time, next_path, min_retransmit, next_tuple);
    }
}
```

### Rust body
```rust
    ) -> Option<(PathToken, usize)> {
        for (path_idx, path) in self.paths.iter_mut().enumerate() {
            if path.path_is_demoted || path.path_abandon_received {
                continue;
            }
            if !path
                .pacing
                .is_authorized(current_time, next_wake_time, false, None)
            {
                continue;
            }
            if let Some(tuple_idx) = path.tuples.iter().position(|tuple| !tuple.challenge_failed) {
                return Some((
                    PathToken::synthetic(path_idx as u32, path_idx as u32),
                    tuple_idx,
                ));
            }
        }
        None
    }
```

## Pair `picoquic/performance_log.c:picoquic_perflog`
C: `picoquic/performance_log.c:225-241 picoquic_perflog`
Rust: `rs/fq/src/tests/tls_api.rs:885-887 perflog`

### C body
```c
{
    int ret = 0;
    picoquic_performance_log_ctx_t* perflog_ctx = (picoquic_performance_log_ctx_t*)quic->v_perflog_ctx;

    if (cnx != NULL) {
        ret = picoquic_perflog_record(cnx, perflog_ctx);
    }

    if (should_delete) {
        picoquic_perflog_free(perflog_ctx);
        quic->v_perflog_ctx = NULL;
        quic->perflog_fn = NULL;
    }

    return ret;
}
```

### Rust body
```rust
fn perflog() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("perflog");
}
```

## Pair `picoquic/performance_log.c:picoquic_perflog_setup`
C: `picoquic/performance_log.c:320-345 picoquic_perflog_setup`
Rust: `rs/fq/src/performance_log.rs:349-363 perflog_setup`

### C body
```c
{
    int ret = 0;
    picoquic_performance_log_ctx_t* perflog_ctx = (picoquic_performance_log_ctx_t*)
        malloc(sizeof(picoquic_performance_log_ctx_t));
    if (perflog_ctx == NULL) {
        ret = -1;
    }
    else {
        memset(perflog_ctx, 0, sizeof(picoquic_performance_log_ctx_t));
        perflog_ctx->perflog_file_name = picoquic_string_duplicate(perflog_file_name);
        if (perflog_ctx->perflog_file_name == NULL) {
            free(perflog_ctx);
            ret = -1;
        } else {
            /* If the file is empty, add a description string, so CSV looks good */
            if (picoquic_perflog_file_is_empty(perflog_file_name)) {
                picoquic_perflog_file_set_header(perflog_file_name);
            }
            /* Program the QUIC context to produce performance logs */
            quic->perflog_fn = picoquic_perflog;
            quic->v_perflog_ctx = (void*)perflog_ctx;
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let path = perflog_file_name.as_ref().to_path_buf();
        if file_is_empty(&path) {
            file_set_header(&path);
        }
        let ctx = PerflogCtx {
            items: Vec::new(),
            perflog_file_name: path,
        };
        self.perflog_fn = Some(Box::new(ctx));
        Ok(())
    }
```

## Pair `picoquic/picohash.c:picohash_insert`
C: `picoquic/picohash.c:84-109 picohash_insert`
Rust: `rs/fq/src/hash.rs:334-355 insert`

### C body
```c
{
    uint64_t hash = hash_table->picohash_hash(key, hash_table->hash_seed);
    uint32_t bin = (uint32_t)(hash % hash_table->nb_bin);
    int ret = 0;
    picohash_item* item;
    
    if (hash_table->picohash_key_to_item == NULL) {
        item = (picohash_item*)malloc(sizeof(picohash_item));
    }
    else {
        item = hash_table->picohash_key_to_item(key);
    }

    if (item == NULL) {
        ret = -1;
    } else {
        item->hash = hash;
        item->key = key;
        item->next_in_bin = hash_table->hash_bin[bin];
        hash_table->hash_bin[bin] = item;
        hash_table->count++;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn insert(&mut self, key: K, value: V) -> Result<(HashToken, Option<V>), Error> {
        let hash = self.hash_key(&key);
        let bin = self.bin_of(hash);

        // Check for existing key.
        let mut cur = self.bins[bin];
        while let Some(idx) = cur {
            if self.slot_key(idx) == &key {
                let old = core::mem::replace(self.slot_value_mut(idx), value);
                return Ok((self.token_of(idx), Some(old)));
            }
            cur = self.slot_next(idx);
        }

        // New entry: prepend to bin chain.
        let idx = self.alloc_slot(hash, key, value)?;
        let old_head = self.bins[bin];
        self.set_next(idx, old_head);
        self.bins[bin] = Some(idx);
        self.len += 1;
        Ok((self.token_of(idx), None))
    }
```

## Pair `picoquic/picohash.c:picohash_siphash`
C: `picoquic/picohash.c:204-219 picohash_siphash`
Rust: `rs/fq/src/siphash.rs:22-38 siphash`

### C body
```c
{
    uint8_t sip_out[8];
    uint64_t hash;
    (void)siphash(bytes, length, hash_seed, sip_out, 8);
    hash =
        (uint64_t)sip_out[0] +
        (((uint64_t)sip_out[1]) << 8) +
        (((uint64_t)sip_out[2]) << 16) +
        (((uint64_t)sip_out[3]) << 24) +
        (((uint64_t)sip_out[4]) << 32) +
        (((uint64_t)sip_out[5]) << 40) +
        (((uint64_t)sip_out[6]) << 48) +
        (((uint64_t)sip_out[7]) << 56);
    return hash;
}
```

### Rust body
```rust
    fn sip_round(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
        *v0 = v0.wrapping_add(*v1);
        *v1 = v1.rotate_left(13);
        *v1 ^= *v0;
        *v0 = v0.rotate_left(32);
        *v2 = v2.wrapping_add(*v3);
        *v3 = v3.rotate_left(16);
        *v3 ^= *v2;
        *v0 = v0.wrapping_add(*v3);
        *v3 = v3.rotate_left(21);
        *v3 ^= *v0;
        *v2 = v2.wrapping_add(*v1);
        *v1 = v1.rotate_left(17);
        *v1 ^= *v2;
        *v2 = v2.rotate_left(32);
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_generate_stream_cipher`
C: `picoquic/picoquic_lb.c:84-101 picoquic_lb_compat_cid_generate_stream_cipher`
Rust: `rs/fq/src/lb.rs:342-373 generate_stream_cipher`

### C body
```c
{
    size_t id_offset = ((size_t)1) + lb_ctx->nonce_length;
    /* Prepare a clear text server ID */
    picoquic_lb_compat_cid_generate_first_byte(quic, lb_ctx, cnx_id_returned);
    memcpy(cnx_id_returned->id + id_offset, lb_ctx->server_id, lb_ctx->server_id_length);
    /* First pass -- obtain intermediate server ID */
    picoquic_lb_compat_cid_one_pass_stream(lb_ctx->cid_encryption_context, cnx_id_returned->id + 1, lb_ctx->nonce_length,
        cnx_id_returned->id + id_offset, lb_ctx->server_id_length);
    /* Second pass -- obtain encrypted nonce */
    picoquic_lb_compat_cid_one_pass_stream(lb_ctx->cid_encryption_context, 
        cnx_id_returned->id + id_offset, lb_ctx->server_id_length,
        cnx_id_returned->id + 1, lb_ctx->nonce_length);
    /* Third pass -- obtain encrypted server-id */
    picoquic_lb_compat_cid_one_pass_stream(lb_ctx->cid_encryption_context, cnx_id_returned->id + 1, lb_ctx->nonce_length,
        cnx_id_returned->id + id_offset, lb_ctx->server_id_length);
}
```

### Rust body
```rust
    fn generate_stream_cipher(&self, quic: &Quic, bytes: &mut [u8]) {
        let id_offset = 1 + self.nonce_length;
        self.set_first_byte(quic, bytes);
        bytes[id_offset..id_offset + self.server_id_length]
            .copy_from_slice(&self.server_id_encoded[..self.server_id_length]);

        let enc = self.cid_encryption_context.as_ref().unwrap();
        Self::one_pass_stream(
            enc,
            bytes,
            1,
            self.nonce_length,
            id_offset,
            self.server_id_length,
        );
        Self::one_pass_stream(
            enc,
            bytes,
            id_offset,
            self.server_id_length,
            1,
            self.nonce_length,
        );
        Self::one_pass_stream(
            enc,
            bytes,
            1,
            self.nonce_length,
            id_offset,
            self.server_id_length,
        );
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_verify_stream_cipher`
C: `picoquic/picoquic_lb.c:163-186 picoquic_lb_compat_cid_verify_stream_cipher`
Rust: `rs/fq/src/lb.rs:402-440 verify_stream_cipher`

### C body
```c
{
    size_t id_offset = ((size_t)1) + lb_ctx->nonce_length;
    uint64_t s_id64 = 0;
    picoquic_connection_id_t target = *cnx_id;
    /* First pass -- obtain intermediate server ID */
    picoquic_lb_compat_cid_one_pass_stream(lb_ctx->cid_encryption_context, target.id + 1, lb_ctx->nonce_length,
        target.id + id_offset, lb_ctx->server_id_length);
    /* Second pass -- obtain nonce */
    picoquic_lb_compat_cid_one_pass_stream(lb_ctx->cid_encryption_context,
        target.id + id_offset, lb_ctx->server_id_length, target.id + 1, lb_ctx->nonce_length);
    /* First pass -- obtain server-id */
    picoquic_lb_compat_cid_one_pass_stream(lb_ctx->cid_encryption_context, target.id + 1, lb_ctx->nonce_length,
        target.id + id_offset, lb_ctx->server_id_length);

    /* decode the server ID */
    for (size_t i = 0; i < lb_ctx->server_id_length; i++) {
        s_id64 <<= 8;
        s_id64 += target.id[id_offset + i];
    }

    return s_id64;
}
```

### Rust body
```rust
    fn verify_stream_cipher(&self, cnx_id: &ConnectionId) -> u64 {
        let id_offset = 1 + self.nonce_length;
        let mut target = [0u8; CONNECTION_ID_MAX_SIZE];
        let len = cnx_id.len();
        target[..len].copy_from_slice(cnx_id.as_bytes());
        let enc = self.cid_encryption_context.as_ref().unwrap();

        Self::one_pass_stream(
            enc,
            &mut target[..len],
            1,
            self.nonce_length,
            id_offset,
            self.server_id_length,
        );
        Self::one_pass_stream(
            enc,
            &mut target[..len],
            id_offset,
            self.server_id_length,
            1,
            self.nonce_length,
        );
        Self::one_pass_stream(
            enc,
            &mut target[..len],
            1,
            self.nonce_length,
            id_offset,
            self.server_id_length,
        );

        let mut s_id64: u64 = 0;
        for i in 0..self.server_id_length {
            s_id64 <<= 8;
            s_id64 += target[id_offset + i] as u64;
        }
        s_id64
    }
```

## Pair `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_config`
C: `picoquic/picoquic_lb.c:389-495 picoquic_lb_compat_cid_config`
Rust: `rs/fq/src/lb.rs:535-611 set_lb_cid_config`

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

## Pair `picoquic/picoquic_ptls_minicrypto.c:set_minicrypto_private_key_from_key_file`
C: `picoquic/picoquic_ptls_minicrypto.c:35-38 set_minicrypto_private_key_from_key_file`
Rust: `rs/fq/src/tls_api.rs:2616-2623 set_minicrypto_private_key_from_key_file`

### C body
```c
{
    return ptls_minicrypto_load_private_key(ctx, keypem);
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

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_init_openssl`
C: `picoquic/picoquic_ptls_openssl.c:71-88 picoquic_init_openssl`
Rust: `rs/fq/src/sys/openssl.rs:283-286 init_openssl`

### C body
```c
{
    if (openssl_is_init == 0) {
        openssl_is_init = 1;
        OpenSSL_add_all_algorithms();
#if !defined(LIBRESSL_VERSION_NUMBER) && OPENSSL_VERSION_NUMBER >= 0x30000000L
        openssl_default_provider = OSSL_PROVIDER_load(NULL, "default");
#else
        ERR_load_crypto_strings();
#if !defined(OPENSSL_NO_ENGINE)
        /* Load all compiled-in ENGINEs */
        ENGINE_load_builtin_engines();
        ENGINE_register_all_ciphers();
        ENGINE_register_all_digests();
#endif
#endif
    }
}
```

### Rust body
```rust
    if !OPENSSL_IS_INIT.swap(true, Ordering::SeqCst) {
        openssl::init();
    }
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_get_public_key_from_key_file`
C: `picoquic/picoquic_ptls_openssl.c:160-199 picoquic_openssl_get_public_key_from_key_file`
Rust: `rs/fq/src/sys/openssl.rs:334-340 get_public_key_from_key_file`

### C body
```c
{
    int ret = 0;
    BIO* bio = BIO_new_file(keypem, "rb");

    *pubkey = NULL;
    *pubkey_length = 0;

    if (bio == NULL) {
        ret = -1;
    }
    else {
        EVP_PKEY* pkey = PEM_read_bio_PrivateKey(bio, NULL, NULL, NULL);
        if (pkey == NULL) {
            ret = -1;
        }
        else {
            int pk_len = i2d_PublicKey(pkey, NULL);
            if (pk_len <= 0 || (*pubkey = (uint8_t*)malloc(pk_len)) == NULL) {
                ret = -1;
            }
            else {
                unsigned char* pk_peek = *pubkey;
                pk_len = (size_t)i2d_PublicKey(pkey, &pk_peek);
                if (pk_len <= 0 || pk_peek != *pubkey + pk_len) {
                    free(*pubkey);
                    ret = -1;
                }
                else {
                    *pubkey_length = (size_t)pk_len;
                }
            }
            EVP_PKEY_free(pkey);
        }
        BIO_free(bio);
    }
    return ret;
}
```

### Rust body
```rust
fn get_public_key_from_key_file(keypem: &str) -> Result<Vec<u8>, crate::Error> {
    let pem = std::fs::read(keypem).map_err(|_| crate::Error::NoSuchFile)?;
    let pkey =
        openssl::pkey::PKey::private_key_from_pem(&pem).map_err(|_| crate::Error::InvalidFile)?;
    pkey.public_key_to_der()
        .map_err(|_| crate::Error::InvalidFile)
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_dispose_certificate_verifier`
C: `picoquic/picoquic_ptls_openssl.c:269-275 picoquic_openssl_dispose_certificate_verifier`
Rust: `rs/fq/src/sys/openssl.rs:413-415 picoquic_openssl_dispose_certificate_verifier`

### C body
```c
void picoquic_openssl_dispose_certificate_verifier(ptls_verify_certificate_t* verifier) {
    ptls_openssl_dispose_verify_certificate((ptls_openssl_verify_certificate_t*)verifier);
    /* The ptls_openssl call does not free the verifier context.
     * We free it here, in order to match the programming pattern of picoquic.
     */
    free(verifier);
}
```

### Rust body
```rust
pub fn picoquic_openssl_dispose_certificate_verifier(verifier: CertificateVerifier) {
    drop(verifier);
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_clear_crypto_errors`
C: `picoquic/picoquic_ptls_openssl.c:334-340 picoquic_openssl_clear_crypto_errors`
Rust: `rs/fq/src/sys/openssl.rs:467-469 clear_crypto_errors`

### C body
```c
{
    ERR_clear_error();
}
```

### Rust body
```rust
pub fn clear_crypto_errors() {
    drop(openssl::error::ErrorStack::get());
}
```

## Pair `picoquic/picoquic_ptls_openssl.c:picoquic_ptls_openssl_log_version`
C: `picoquic/picoquic_ptls_openssl.c:456-464 picoquic_ptls_openssl_log_version`
Rust: `rs/fq/src/sys/openssl.rs:793-809 picoquic_ptls_openssl_log_version`

### C body
```c
{
#ifdef OPENSSL_VERSION_NUMBER
    picoquic_log_app_message(cnx, "OpenSSL source version %x, binary version %x", OPENSSL_VERSION_NUMBER, OpenSSL_version_num());
#endif
#ifdef LIBRESSL_VERSION_NUMBER
    picoquic_log_app_message(cnx, "LibreSSL source version %x, binary version %x", LIBRESSL_VERSION_NUMBER, OpenSSL_version_num());
#endif
}
```

### Rust body
```rust
pub fn picoquic_ptls_openssl_log_version(cnx: &mut Connection) {
    let binary_version = openssl::version::number();
    if let Some(source_version) = libressl_source_version_number() {
        cnx.log_app_message(&alloc::format!(
            "LibreSSL source version {:x}, binary version {:x}",
            source_version,
            binary_version
        ));
    } else {
        let source_version = source_version_number().unwrap_or(binary_version);
        cnx.log_app_message(&alloc::format!(
            "OpenSSL source version {:x}, binary version {:x}",
            source_version,
            binary_version
        ));
    }
}
```
