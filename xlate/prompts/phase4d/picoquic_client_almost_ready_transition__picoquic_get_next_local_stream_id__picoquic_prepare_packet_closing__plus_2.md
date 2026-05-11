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

## `picoquic/sender.c:picoquic_client_almost_ready_transition`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets the connection state to client_almost_ready; the shown Rust body does not set the state.
* C source: `picoquic/sender.c:2685-2696`
* C signature: `void picoquic_client_almost_ready_transition(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:7691-7714`
* Rust item: `client_almost_ready_transition`

### C body
```c
{
    cnx->cnx_state = picoquic_state_client_almost_ready;
    /* If client, make sure that 0-RTT packets are in correct context */
    if (cnx->is_multipath_enabled) {
        picoquic_packet_context_t* o_pkt_ctx = &cnx->pkt_ctx[0];
        picoquic_packet_context_t* n_pkt_ctx = &cnx->path[0]->pkt_ctx;

        *n_pkt_ctx = *o_pkt_ctx;
        picoquic_init_packet_ctx(cnx, o_pkt_ctx, picoquic_packet_context_application);
    }
}
```

### Rust body
```rust
        if self.is_multipath_enabled && !self.paths.is_empty() {
            let app_ctx = core::mem::replace(
                &mut self.pkt_ctx[PacketContext::Application as usize],
                PacketContextState {
                    send_sequence: 0,
                    next_sequence_hole: 0,
                    retransmit_sequence: 0,
                    highest_acknowledged: u64::MAX,
                    latest_time_acknowledged: self.start_time,
                    highest_acknowledged_time: self.start_time,
                    pending: BTreeMap::new(),
                    retransmitted: BTreeMap::new(),
                    preemptive_repeat_seq: None,
                    retransmitted_queue_size: 0,
                    ecn_ect0_total_remote: 0,
                    ecn_ect1_total_remote: 0,
                    ecn_ce_total_remote: 0,
                    ack_of_ack_requested: false,
                },
            );
            self.paths[0].pkt_ctx = app_ctx;
        }
```

## `picoquic/sender.c:picoquic_get_next_local_stream_id`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns next_stream_id indexed by stream type, while the Rust body shown returns primary_path_rtt_max.
* C source: `picoquic/sender.c:414-428`
* C signature: `uint64_t picoquic_get_next_local_stream_id(picoquic_cnx_t *, int)`
* Rust source: `rs/fq/src/lib.rs:4833-4841`
* Rust item: `get_next_local_stream_id`

### C body
```c
{
    /* This code could be written as:
     * int stream_type_id = ((cnx->client_mode ^ 1) | ((is_unidir) ? 2 : 0)); 
     * but Visual Studio produces an obnoxious error message about
     * mixing bitwise or and logical or. */
    int stream_type_id = cnx->client_mode ^ 1;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (is_unidir) {
        stream_type_id |= 2;
    }

    return cnx->next_stream_id[stream_type_id];     
}
```

### Rust body
```rust
    pub fn primary_path_rtt_max(&self) -> u64 {
        self.paths.first().map(|p| p.rtt_max.ticks()).unwrap_or(0)
    }
```

## `picoquic/sender.c:picoquic_prepare_packet_closing`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust collapses several closing-state branches: ClosingReceived wake/state/frame behavior and Closing retransmission timing/ack_needed behavior visibly differ from C.
* C source: `picoquic/sender.c:2351-2594`
* C signature: `int picoquic_prepare_packet_closing(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:18469-18668`
* Rust item: `prepare_packet_closing`

### C body
```c
{
    int ret = 0;
    /* TODO: manage multiple streams. */
    picoquic_packet_type_enum packet_type = 0;
    size_t checksum_overhead = 8;
    size_t header_length = 0;
    uint8_t* bytes = packet->bytes;
    uint8_t* bytes_max;
    uint8_t* bytes_next;
    int more_data = 0;
    size_t length = 0;
    int is_pure_ack = 1;
    picoquic_packet_context_enum pc = picoquic_packet_context_application;
    picoquic_packet_context_t * pkt_ctx;
    picoquic_epoch_enum epoch = picoquic_epoch_1rtt;

    /* The only purpose of the test below is to appease the static analyzer, so it
     * wont complain of possible NULL deref. On windows we could use "__assume(path_x != NULL)"
     * but the documentation does not say anything about that for GCC and CLANG */
    if (path_x == NULL) {
        return PICOQUIC_ERROR_UNEXPECTED_ERROR;
    }

    send_buffer_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;

    /* Prepare header -- depend on connection state */
    /* TODO: 0-RTT work. */
    switch (cnx->cnx_state) {
    case picoquic_state_handshake_failure:
        /* TODO: check whether closing can be requested in "initial" mode */
        if (cnx->crypto_context[picoquic_epoch_handshake].aead_encrypt != NULL &&
            picoquic_sack_list_first(&cnx->ack_ctx[picoquic_packet_context_handshake].sack_list) != UINT64_MAX) {
            pc = picoquic_packet_context_handshake;
            packet_type = picoquic_packet_handshake;
            epoch = picoquic_epoch_handshake;
        }
        else {
            pc = picoquic_packet_context_initial;
            packet_type = picoquic_packet_initial;
        }
        break;
    case picoquic_state_handshake_failure_resend:
        pc = picoquic_packet_context_handshake;
        packet_type = picoquic_packet_handshake;
        epoch = picoquic_epoch_handshake;
        break;
    case picoquic_state_disconnecting:
        packet_type = picoquic_packet_1rtt_protected;
        break;
    case picoquic_state_closing_received:
        packet_type = picoquic_packet_1rtt_protected;
        break;
    case picoquic_state_closing:
        packet_type = picoquic_packet_1rtt_protected;
        break;
    case picoquic_state_draining:
        packet_type = picoquic_packet_1rtt_protected;
        break;
    case picoquic_state_disconnected:
        ret = PICOQUIC_ERROR_DISCONNECTED;
        break;
    default:
        ret = -1;
        break;
    }

    /* At this stage, we don't try to retransmit any old packet, whether in
     * the current context or in previous contexts. */

    if (packet_type == picoquic_packet_1rtt_protected && cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }
    else {
        pkt_ctx = &cnx->pkt_ctx[pc];
    }

    checksum_overhead = picoquic_get_checksum_length(cnx, epoch);
    packet->pc = pc;
    bytes_max = bytes + send_buffer_max - checksum_overhead;

    if (ret == 0 && cnx->cnx_state == picoquic_state_closing_received) {
        /* Send a closing frame, move to draining state */
        uint64_t exit_time = cnx->latest_progress_time + 3 * path_x->retransmit_timer;

        length = picoquic_predict_packet_header_length(cnx, packet_type, pkt_ctx);
        bytes_next = bytes + length;
        packet->ptype = packet_type;
        packet->offset = length;
        header_length = length;
        packet->sequence_number = pkt_ctx->send_sequence;
        packet->send_time = current_time;
        packet->send_path = path_x;

        /* Send the disconnect frame */
        bytes_next = picoquic_format_connection_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
        length = bytes_next - bytes;
        cnx->last_close_sent = current_time;
        cnx->cnx_state = picoquic_state_draining;
        *next_wake_time = exit_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
    } else if (ret == 0 && cnx->cnx_state == picoquic_state_closing) {
        /* if more than 3*RTO is elapsed, move to disconnected */
        uint64_t exit_time = cnx->latest_progress_time + 3 * path_x->retransmit_timer;
        uint64_t next_close_time = cnx->last_close_sent + path_x->smoothed_rtt;

        if (current_time >= exit_time) {
            picoquic_connection_disconnect(cnx);
            *next_wake_time = current_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        else if (current_time >= next_close_time) {
            uint64_t delta_t = path_x->rtt_min;
            uint64_t next_time = 0;

            if (delta_t * 2 < path_x->retransmit_timer) {
                delta_t = path_x->retransmit_timer / 2;
            }
            /* if more than N packet received, repeat and erase */
            if (cnx->ack_ctx[pc].act[0].ack_needed) {
                length = picoquic_predict_packet_header_length(
                    cnx, packet_type, pkt_ctx);
                packet->ptype = packet_type;
                packet->offset = length;
                header_length = length;
                packet->sequence_number = pkt_ctx->send_sequence;
                packet->send_time = current_time;
                packet->send_path = path_x;
                bytes_next = bytes + length;

                /* Resend the disconnect frame */
                if (cnx->local_error == 0) {
                    bytes_next = picoquic_format_application_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
                } else {
                    bytes_next = picoquic_format_connection_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
                }
                length = bytes_next - bytes;
                cnx->ack_ctx[pc].act[0].ack_needed = 0;
                cnx->ack_ctx[pc].act[0].out_of_order_received = 0;
                cnx->last_close_sent = current_time;
            }
            next_time = current_time + delta_t;
            if (next_time > exit_time) {
                next_time = exit_time;
            }

            *next_wake_time = next_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        else {
            if (next_close_time > exit_time) {
                next_close_time = exit_time;
            }
            if (*next_wake_time > next_close_time) {
                *next_wake_time = next_close_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
            }
        }
    } else if (ret == 0 && cnx->cnx_state == picoquic_state_draining) {
        /* Nothing is ever sent in the draining state */
        /* if more than 3*RTO is elapsed, move to disconnected */
        uint64_t exit_time = cnx->latest_progress_time + 3 * path_x->retransmit_timer;

        if (current_time >= exit_time) {
            picoquic_connection_disconnect(cnx);
            *next_wake_time = current_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        else {
            *next_wake_time = exit_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        length = 0;
    } else if (ret == 0 && (cnx->cnx_state == picoquic_state_disconnecting || 
        cnx->cnx_state == picoquic_state_handshake_failure || 
        cnx->cnx_state == picoquic_state_handshake_failure_resend)) {

        length = picoquic_predict_packet_header_length(
            cnx, packet_type, pkt_ctx);
        bytes_next = bytes + length;
        packet->ptype = packet_type;
        packet->offset = length;
        header_length = length;
        packet->sequence_number = pkt_ctx->send_sequence;
        packet->send_time = current_time;
        packet->send_path = path_x;

        /* send either app close or connection close, depending on error code */
        uint64_t delta_t = path_x->rtt_min;

        if (2 * delta_t < path_x->retransmit_timer) {
            delta_t = path_x->retransmit_timer / 2;
        }

        /* add a final ack so receiver gets clean state */
        bytes_next = picoquic_format_ack_frame(cnx, bytes_next, bytes_max, &more_data, current_time, pc, 0);

        /* Send the disconnect frame */
        if (cnx->local_error == 0) {
            bytes_next = picoquic_format_application_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
        }
        else {
            bytes_next = picoquic_format_connection_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
        }
        length = bytes_next - bytes;

        if (cnx->cnx_state == picoquic_state_handshake_failure) {
            if (pc == picoquic_packet_context_initial &&
                cnx->crypto_context[2].aead_encrypt != NULL) {
                cnx->cnx_state = picoquic_state_handshake_failure_resend;
            }
            else {
                picoquic_connection_disconnect(cnx);
            }
        }
        else if (cnx->cnx_state == picoquic_state_handshake_failure_resend) {
            picoquic_connection_disconnect(cnx);
        }
        else {
            cnx->cnx_state = picoquic_state_closing;
        }
        cnx->latest_progress_time = current_time;
        cnx->last_close_sent = current_time;
        *next_wake_time = current_time + delta_t;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        cnx->ack_ctx[pc].act[0].ack_needed = 0;
    }
    else {
        length = 0;
    }

    if (length > 0 && packet->ptype == picoquic_packet_initial && cnx->client_mode) {
        length = picoquic_pad_to_target_length(bytes, length, send_buffer_max - checksum_overhead);
    }

    picoquic_finalize_and_protect_packet(cnx, packet,
        ret, length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_max,
        path_x, current_time);

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut ret = 0;
        let mut pc = PacketContext::Application;
        let mut packet_type = PacketType::OneRttProtected;
        let mut epoch = Epoch::OneRtt;
        match self.connection_state {
            State::HandshakeFailure => {
                if self.crypto_context[Epoch::Handshake as usize]
                    .aead_encrypt
                    .is_some()
                    && !self.ack_ctx[PacketContext::Handshake as usize]
                        .sack_list
                        .is_empty()
                {
                    pc = PacketContext::Handshake;
                    packet_type = PacketType::Handshake;
                    epoch = Epoch::Handshake;
                } else {
                    pc = PacketContext::Initial;
                    packet_type = PacketType::Initial;
                    epoch = Epoch::Initial;
                }
            }
            State::HandshakeFailureResend => {
                pc = PacketContext::Handshake;
                packet_type = PacketType::Handshake;
                epoch = Epoch::Handshake;
            }
            State::Disconnecting | State::ClosingReceived | State::Closing | State::Draining => {}
            State::Disconnected => ret = crate::errors::InternalError::Disconnected as i32,
            _ => ret = crate::errors::InternalError::UnexpectedState as i32,
        }

        let checksum_overhead = self.get_checksum_length(epoch);
        let send_buffer_max = send_buffer_max.min(path_x.send_mtu);
        let bytes_limit = send_buffer_max
            .saturating_sub(checksum_overhead)
            .min(packet.bytes.len());
        let mut header_length = self.predict_packet_header_length_for_pc(packet_type, pc);
        let mut length = 0usize;
        let mut more_data = 0;
        let mut is_pure_ack = 1;

        if ret == 0
            && matches!(
                self.connection_state,
                State::ClosingReceived
                    | State::Closing
                    | State::Disconnecting
                    | State::HandshakeFailure
                    | State::HandshakeFailureResend
            )
        {
            if self.connection_state == State::Closing {
                let exit_time = self
                    .latest_progress_time
                    .ticks()
                    .saturating_add(3u64.saturating_mul(path_x.retransmit_timer.ticks()));
                let next_close_time = self
                    .last_close_sent
                    .ticks()
                    .saturating_add(path_x.smoothed_rtt.ticks());
                if current_time.ticks() >= exit_time {
                    self.connection_disconnect();
                    self.set_sender_wake_now(next_wake_time, current_time);
                } else if current_time.ticks() < next_close_time {
                    *next_wake_time = Instant::from_ticks(next_close_time.min(exit_time));
                }
            }
            if self.connection_state != State::Disconnected
                && self.connection_state != State::Draining
            {
                length = header_length;
                packet.packet_type = packet_type;
                packet.offset = header_length;
                packet.sequence_number =
                    if packet_type == PacketType::OneRttProtected && self.is_multipath_enabled {
                        path_x.pkt_ctx.send_sequence
                    } else {
                        self.pkt_ctx[pc as usize].send_sequence
                    };
                packet.send_time = current_time;
                packet.send_path = Some(Self::path_token_for_path(path_x));
                packet.packet_context = pc;
                if length <= bytes_limit {
                    let mut offset = length;
                    if matches!(
                        self.connection_state,
                        State::Disconnecting
                            | State::HandshakeFailure
                            | State::HandshakeFailureResend
                    ) {
                        let tail_len = {
                            let tail = &mut packet.bytes[offset..bytes_limit];
                            match format_ack_frame(self, tail, &mut more_data, current_time, pc, 0)
                            {
                                Some(next) => next.len(),
                                None => tail.len(),
                            }
                        };
                        offset = bytes_limit.saturating_sub(tail_len);
                    }
                    let tail_len = {
                        let tail = &mut packet.bytes[offset..bytes_limit];
                        let next = if self.local_error == 0 {
                            format_application_close_frame(
                                self,
                                tail,
                                &mut more_data,
                                &mut is_pure_ack,
                            )
                        } else {
                            format_connection_close_frame(
                                self,
                                tail,
                                &mut more_data,
                                &mut is_pure_ack,
                            )
                        };
                        match next {
                            Some(next) => next.len(),
                            None => {
                                ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                                tail.len()
                            }
                        }
                    };
                    length = bytes_limit.saturating_sub(tail_len);
                }
                self.last_close_sent = current_time;
                match self.connection_state {
                    State::ClosingReceived => self.connection_state = State::Draining,
                    State::HandshakeFailure => {
                        if pc == PacketContext::Initial
                            && self.crypto_context[Epoch::Handshake as usize]
                                .aead_encrypt
                                .is_some()
                        {
                            self.connection_state = State::HandshakeFailureResend;
                        } else {
                            self.connection_disconnect();
                        }
                    }
                    State::HandshakeFailureResend => self.connection_disconnect(),
                    State::Disconnecting => self.connection_state = State::Closing,
                    _ => {}
                }
                self.latest_progress_time = current_time;
                let mut delta_t = path_x.rtt_min.ticks();
                if delta_t.saturating_mul(2) < path_x.retransmit_timer.ticks() {
                    delta_t = path_x.retransmit_timer.ticks() / 2;
                }
                *next_wake_time = Instant::from_ticks(current_time.ticks().saturating_add(delta_t));
                self.ack_ctx[pc as usize].act[0].ack_needed = false;
            }
        } else if ret == 0 && self.connection_state == State::Draining {
            let exit_time = self
                .latest_progress_time
                .ticks()
                .saturating_add(3u64.saturating_mul(path_x.retransmit_timer.ticks()));
            if current_time.ticks() >= exit_time {
                self.connection_disconnect();
                self.set_sender_wake_now(next_wake_time, current_time);
            } else {
                *next_wake_time = Instant::from_ticks(exit_time);
            }
            header_length = 0;
            length = 0;
        }

        if length > 0 && packet.packet_type == PacketType::Initial && self.client_mode {
            length = pad_to_target_length(
                &mut packet.bytes,
                length,
                send_buffer_max.saturating_sub(checksum_overhead),
            );
        }
        self.finalize_and_protect_packet(
            packet,
            ret,
            length,
            header_length,
            checksum_overhead,
            send_length,
            send_buffer,
            send_buffer_max,
            path_x,
            current_time,
        );
        ret
    }
```

## `picoquic/sender.c:picoquic_set_path_addresses_from_tuple`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust only writes p_addr_to; C also writes p_addr_from and if_index when provided.
* C source: `picoquic/sender.c:3832-3846`
* C signature: `void picoquic_set_path_addresses_from_tuple(picoquic_tuple_t *, struct sockaddr_storage *, struct sockaddr_storage *, int *)`
* Rust source: `rs/fq/src/internal.rs:4117-4125`
* Rust item: `picoquic_set_path_addresses_from_tuple`

### C body
```c
{
    if (p_addr_to != NULL) {
        picoquic_store_addr(p_addr_to, (struct sockaddr*)&tuple->peer_addr);
    }

    if (p_addr_from != NULL) {
        picoquic_store_addr(p_addr_from, (struct sockaddr*)&tuple->local_addr);
    }

    if (if_index != NULL) {
        *if_index = tuple->if_index;
    }
}
```

### Rust body
```rust
    if let Some(addr_to) = p_addr_to {
        *addr_to = tuple.peer_addr;
    }
```

## `picoquic/sockloop.c:picoquic_start_network_thread`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C delegates to the custom network thread starter; Rust creates a thread that only waits on a channel, with no body-visible packet loop startup.
* C source: `picoquic/sockloop.c:1814-1818`
* C signature: `picoquic_network_thread_ctx_t * picoquic_start_network_thread(picoquic_quic_t *, picoquic_packet_loop_param_t *, picoquic_packet_loop_cb_fn, void *, int *)`
* Rust source: `rs/fq/src/packet_loop.rs:1358-1415`
* Rust item: `spawn`

### C body
```c
{
    return picoquic_start_custom_network_thread(quic, param, NULL, NULL, NULL, NULL, loop_callback, loop_callback_ctx, ret);
}
```

### Rust body
```rust
    ) -> Result<Box<Self>, OsError> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut thread_ctx = Box::new(NetworkThreadCtx::default());
        thread_ctx.param = Some(Box::new(param));
        thread_ctx.loop_callback = loop_callback;
        thread_ctx.thread_delete_fn = thread_delete_fn;
        thread_ctx.thread_setname_fn = thread_setname_fn;
        thread_ctx.thread_name = thread_name.map(str::to_owned);
        thread_ctx.wake_up_pipe_fd = [-1, -1];
        thread_ctx.wake_up_sender = Some(sender);
        thread_ctx.wake_up_receiver = None;
        thread_ctx.wake_up_defined = true;
        thread_ctx.is_threaded = true;

        let name_for_thread = thread_ctx.thread_name.clone();
        let thread_fn: Box<dyn FnOnce() + Send + 'static> = Box::new(move || {
            if let Some(name) = name_for_thread {
                let _ = name;
            }
            while receiver.recv().is_ok() {}
        });

        let handle = if let Some(mut create_fn) = thread_create_fn {
            create_fn.create(thread_fn)?
        } else if let Some(name) = thread_ctx.thread_name.clone() {
            std::thread::Builder::new()
                .name(name)
                .spawn(thread_fn)
                .map_err(|error| OsError(error.raw_os_error().unwrap_or(-1)))?
        } else {
            internal_thread_create(thread_fn)?
        };
        thread_ctx.pthread = Some(handle);
        thread_ctx.thread_is_ready = true;
        let _ = quic.time();
        Ok(thread_ctx)
    }
```
