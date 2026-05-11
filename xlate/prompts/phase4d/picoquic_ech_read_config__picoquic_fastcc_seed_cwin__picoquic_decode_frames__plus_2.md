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

## `picoquic/ech.c:picoquic_ech_read_config`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C reads and base64-decodes a config file; Rust body saves a config buffer to a file and is named ech_save_config.
* C source: `picoquic/ech.c:95-124`
* C signature: `int picoquic_ech_read_config(ptls_buffer_t *, const char *)`
* Rust source: `rs/fq/src/lib.rs:4749-4757`
* Rust item: `ech_read_config`

### C body
```c
{
    int ret = 0;
    char buffer[1024];
    FILE* F = picoquic_file_open(config_file_name, "r");
    if (F == NULL) {
        ret = -1;
    }
    else {
        ptls_base64_decode_state_t d_state;
        ptls_base64_decode_init(&d_state);
        while (fgets(buffer, sizeof(buffer), F) != NULL) {
            ret = ptls_base64_decode(buffer, &d_state, config);
        }
        if (d_state.status == PTLS_BASE64_DECODE_DONE || (d_state.status == PTLS_BASE64_DECODE_IN_PROGRESS && d_state.nbc == 0)) {
            ret = 0;
        }
        else {
            ret = PTLS_ERROR_INCORRECT_BASE64;
        }
        F=picoquic_file_close(F);
        if (ret == 0) {
            DBG_PRINTF("Got %zu bytes from %s", config->off, config_file_name);
        }
    }
    return ret;
}
```

### Rust body
```rust
pub fn ech_save_config(config: &[u8], ech_config_file: &str) -> Result<(), Error> {
    crate::ech::ech_save_config_file(config, ech_config_file)
}
```

## `picoquic/fastcc.c:picoquic_fastcc_seed_cwin`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C only raises cwin to bytes_in_flight when alg_state is initial; Rust resets fastcc state and never uses bytes_in_flight or updates cwin that way.
* C source: `picoquic/fastcc.c:85-92`
* C signature: `void picoquic_fastcc_seed_cwin(picoquic_fastcc_state_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/fastcc.rs:103-124`
* Rust item: `fastcc_seed_cwin`

### C body
```c
{
    if (fastcc_state->alg_state == picoquic_fastcc_initial) {
        if (path_x->cwin < bytes_in_flight) {
            path_x->cwin = bytes_in_flight;
        }
    }
}
```

### Rust body
```rust
) {
    let mut state = path_x
        .congestion_alg_state
        .take()
        .and_then(|boxed| boxed.downcast::<FastccState>().ok())
        .unwrap_or_default();
    picoquic_fastcc_reset(&mut state, path_x, current_time);
    path_x.congestion_alg_state = Some(state);
}
```

## `picoquic/frames.c:picoquic_decode_frames`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust omits many visible C branches and epoch/protocol checks, generally skips unknown frames instead of treating them as protocol violations, does not track ack_needed/is_path_probing_packet the same way, and unconditionally updates last_non_path_probing_pn at the end.
* C source: `picoquic/frames.c:6701-6990`
* C signature: `int picoquic_decode_frames(picoquic_cnx_t *, picoquic_path_t *, const uint8_t *, size_t, picoquic_stream_data_node_t *, int, struct sockaddr *, struct sockaddr *, uint64_t, int, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:14323-14477`
* Rust item: `decode_frames`

### C body
```c
{
    const uint8_t *bytes_max = bytes + bytes_maxsize;
    int ack_needed = 0;
    int is_path_probing_packet = 1; /* Will be set to zero if non probing frame received */
    picoquic_packet_context_enum pc = picoquic_context_from_epoch(epoch);
    picoquic_packet_data_t packet_data;

    memset(&packet_data, 0, sizeof(packet_data));

    while (bytes != NULL && bytes < bytes_max) {
        uint8_t first_byte = bytes[0];
        int is_path_probing_frame = 0;

        if (PICOQUIC_IN_RANGE(first_byte, picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max)) {
            if (epoch != picoquic_epoch_0rtt && epoch != picoquic_epoch_1rtt) {
                DBG_PRINTF("Data frame (0x%x), when only TLS stream is expected", first_byte);
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, first_byte);
                bytes = NULL;
                break;
            }

            bytes = picoquic_decode_stream_frame(cnx, bytes, bytes_max, received_data, current_time);
            ack_needed = 1;

        }
        else if (first_byte == picoquic_frame_type_ack) {
            if (epoch == picoquic_epoch_0rtt) {
                DBG_PRINTF("Ack frame (0x%x) not expected in 0-RTT packet", first_byte);
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, first_byte);
                bytes = NULL;
                break;
            }
            bytes = picoquic_decode_ack_frame(cnx, bytes, bytes_max, current_time, epoch, 0, 0, &packet_data);
        }
        else if (first_byte == picoquic_frame_type_ack_ecn) {
            if (epoch == picoquic_epoch_0rtt) {
                DBG_PRINTF("Ack-ECN frame (0x%x) not expected in 0-RTT packet", first_byte);
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, first_byte);
                bytes = NULL;
                break;
            }
            bytes = picoquic_decode_ack_frame(cnx, bytes, bytes_max, current_time, epoch, 1, 0, &packet_data);
        }
        else if (epoch != picoquic_epoch_0rtt && epoch != picoquic_epoch_1rtt && first_byte != picoquic_frame_type_padding
            && first_byte != picoquic_frame_type_ping
            && first_byte != picoquic_frame_type_connection_close
            && first_byte != picoquic_frame_type_crypto_hs) {
            picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, first_byte);
            bytes = NULL;
            break;
        }
        else if (epoch == picoquic_epoch_0rtt && (first_byte == picoquic_frame_type_crypto_hs
            || first_byte == picoquic_frame_type_handshake_done
            || first_byte == picoquic_frame_type_new_token
            || first_byte == picoquic_frame_type_path_response
            || first_byte == picoquic_frame_type_retire_connection_id)) {
            /* From draft-31:
             * Note that it is not possible to send the following frames in 0-RTT
             * packets for various reasons : ACK, CRYPTO, HANDSHAKE_DONE, NEW_TOKEN,
             * PATH_RESPONSE, and RETIRE_CONNECTION_ID.A server MAY treat receipt
             * of these frames in 0 - RTT packets as a connection error of type
             * PROTOCOL_VIOLATION.
             */
            picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, first_byte);
            bytes = NULL;
            break;
        }
        else {
            switch (first_byte) {
            case picoquic_frame_type_padding:
                is_path_probing_frame = 1;
                bytes = picoquic_skip_0len_frame(bytes, bytes_max);
                break;
            case picoquic_frame_type_reset_stream:
                bytes = picoquic_decode_reset_stream_frame(cnx, bytes, bytes_max);
                ack_needed = 1;
                break;
            case picoquic_frame_type_connection_close:
                bytes = picoquic_decode_connection_close_frame(cnx, bytes, bytes_max);
                ack_needed = 0;
                break;
            case picoquic_frame_type_application_close:
                bytes = picoquic_decode_application_close_frame(cnx, bytes, bytes_max);
                ack_needed = 0;
                break;
            case picoquic_frame_type_max_data:
                bytes = picoquic_decode_max_data_frame(cnx, bytes, bytes_max);
                ack_needed = 1;
                break;
            case picoquic_frame_type_max_stream_data:
                bytes = picoquic_decode_max_stream_data_frame(cnx, bytes, bytes_max);
                ack_needed = 1;
                break;
            case picoquic_frame_type_max_streams_bidir:
            case picoquic_frame_type_max_streams_unidir:
                bytes = picoquic_decode_max_streams_frame(cnx, bytes, bytes_max, first_byte);
                ack_needed = 1;
                break;
            case picoquic_frame_type_ping:
                bytes = picoquic_skip_0len_frame(bytes, bytes_max);
                ack_needed = 1;
                break;
            case picoquic_frame_type_data_blocked:
                bytes = picoquic_decode_blocked_frame(cnx, bytes, bytes_max);
                ack_needed = 1;
                break;
            case picoquic_frame_type_stream_data_blocked:
                bytes = picoquic_decode_stream_blocked_frame(cnx, bytes, bytes_max);
                ack_needed = 1;
                break;
            case picoquic_frame_type_streams_blocked_unidir:
            case picoquic_frame_type_streams_blocked_bidir:
                bytes = picoquic_decode_streams_blocked_frame(cnx, bytes, bytes_max, first_byte);
                ack_needed = 1;
                break;
            case picoquic_frame_type_new_connection_id:
                is_path_probing_frame = 1;
                bytes = picoquic_decode_new_connection_id_frame(cnx, bytes, bytes_max, current_time, 0);
                ack_needed = 1;
                break;
            case picoquic_frame_type_stop_sending:
                bytes = picoquic_decode_stop_sending_frame(cnx, bytes, bytes_max);
                ack_needed = 1;
                break;
            case picoquic_frame_type_path_challenge:
                is_path_probing_frame = 1;
                bytes = picoquic_decode_path_challenge_frame(cnx, bytes, bytes_max, 
                    (path_is_not_allocated)?NULL:path_x, addr_from, addr_to);
                break;
            case picoquic_frame_type_path_response:
                is_path_probing_frame = 1;
                bytes = picoquic_decode_path_response_frame(cnx, bytes, bytes_max,
                    (path_is_not_allocated) ? NULL : path_x, current_time);
                break;
            case picoquic_frame_type_crypto_hs:
                bytes = picoquic_decode_crypto_hs_frame(cnx, bytes, bytes_max, received_data, epoch);
                ack_needed = 1;
                break;
            case picoquic_frame_type_new_token:
                bytes = picoquic_decode_new_token_frame(cnx, bytes, bytes_max, addr_to);
                ack_needed = 1;
                break;
            case picoquic_frame_type_retire_connection_id:
                /* the old code point for ACK frames, but this is taken care of in the ACK tests above */
                bytes = picoquic_decode_retire_connection_id_frame(cnx, bytes, bytes_max, path_x, 0);
                ack_needed = 1;
                break;
            case picoquic_frame_type_handshake_done:
                bytes = picoquic_decode_handshake_done_frame(cnx, bytes, current_time);
                ack_needed = 1;
                break;
            case picoquic_frame_type_datagram:
            case picoquic_frame_type_datagram_l:
                /* Datagram carrying packets are acked, but not repeated */
                ack_needed = 1;
                bytes = picoquic_decode_datagram_frame(cnx, path_x, bytes, bytes_max);
                break;
            case picoquic_frame_type_reset_stream_at:
                bytes = picoquic_decode_reset_stream_at_frame(cnx, bytes, bytes_max);
                ack_needed = 1;
                break;
            default: {
                uint64_t frame_id64;
                const uint8_t* bytes0 = bytes;

                if ((bytes = picoquic_frames_varint_decode(bytes, bytes_max, &frame_id64)) != NULL) {
                    if (epoch == picoquic_epoch_0rtt &&
                        frame_id64 != picoquic_frame_type_bdp) {
                        /* By default, extension frames should not be used in 0rtt */
                        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, first_byte);
                        bytes = NULL;
                    }
                    else {
                        switch (frame_id64) {
                        case picoquic_frame_type_ack_frequency:
                            bytes = picoquic_decode_ack_frequency_frame(bytes, bytes_max, cnx);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_immediate_ack:
                            bytes = picoquic_decode_immediate_ack_frame(bytes, bytes_max, cnx, path_x, current_time);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_time_stamp:
                            bytes = picoquic_decode_time_stamp_frame(bytes, bytes_max, cnx, &packet_data);
                            break;
                        case picoquic_frame_type_path_ack: {
                            bytes = picoquic_decode_ack_frame(cnx, bytes0, bytes_max, current_time, epoch, 0, 1, &packet_data);
                            break;
                        }
                        case picoquic_frame_type_path_ack_ecn: {
                            bytes = picoquic_decode_ack_frame(cnx, bytes0, bytes_max, current_time, epoch, 1, 1, &packet_data);
                            break;
                        }
                        case picoquic_frame_type_path_abandon:
                            bytes = picoquic_decode_path_abandon_frame(bytes, bytes_max, cnx, current_time);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_path_backup:
                        case picoquic_frame_type_path_available:
                            bytes = picoquic_decode_path_available_or_backup_frame(bytes, bytes_max, frame_id64, cnx);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_max_path_id:
                            bytes = picoquic_decode_max_path_id_frame(bytes, bytes_max, cnx);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_paths_blocked:
                            bytes = picoquic_decode_paths_blocked_frame(bytes, bytes_max, cnx);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_path_cid_blocked:
                            bytes = picoquic_decode_path_cid_blocked_frame(bytes, bytes_max, cnx);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_path_new_connection_id:
                            is_path_probing_frame = 1;
                            bytes = picoquic_decode_new_connection_id_frame(cnx, bytes0, bytes_max, current_time, 1);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_path_retire_connection_id:
                            bytes = picoquic_decode_retire_connection_id_frame(cnx, bytes0, bytes_max, path_x, 1);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_bdp:
                            if (cnx->client_mode && epoch != picoquic_epoch_1rtt) {
                                DBG_PRINTF("BDP frame (0x%x) is expected in 1-RTT packet", first_byte);
                                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, first_byte);
                                bytes = NULL;
                                break;
                            }
                            if (!cnx->client_mode && epoch != picoquic_epoch_0rtt && epoch != picoquic_epoch_1rtt) {
                                DBG_PRINTF("BDP frame (0x%x) is expected in 0-RTT packet", first_byte);
                                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, first_byte);
                                bytes = NULL;
                                break;
                            }
                            if (cnx->client_mode && cnx->local_parameters.enable_bdp_frame == 0) {
                                DBG_PRINTF("BDP frame (0x%x) not expected", first_byte);
                                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
                                bytes = NULL;
                                break;
                            }

                            bytes = picoquic_decode_bdp_frame(cnx, bytes, bytes_max, current_time, addr_from, path_x);
                            ack_needed = 1;
                            break;
                        case picoquic_frame_type_observed_address_v4:
                        case picoquic_frame_type_observed_address_v6:
                            is_path_probing_frame = 1;
                            ack_needed = 1;
                            bytes = picoquic_decode_observed_address_frame(cnx, bytes, bytes_max, path_x, frame_id64);
                            break;
                        default:
                            /* Not implemented yet! */
                            picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, frame_id64);
                            bytes = NULL;
                            break;
                        }
                    }
                }
                break;
            }
            }
        }
        is_path_probing_packet &= is_path_probing_frame;
    }

    if (bytes != NULL) {
        process_decoded_packet_data(cnx, path_x, epoch, pc, current_time, &packet_data);

        if (ack_needed) {
            cnx->latest_receive_time = current_time;
            picoquic_set_ack_needed(cnx, current_time, pc, path_x, 0);
        }

        if (epoch == picoquic_epoch_1rtt && !is_path_probing_packet && pn64 > path_x->last_non_path_probing_pn) {
            path_x->last_non_path_probing_pn = pn64;
        }
    }

    return bytes != NULL ? 0 : PICOQUIC_ERROR_DETECTED;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut tail = &bytes[..bytes_max.min(bytes.len())];
        while !tail.is_empty() {
            let frame_start = tail;
            let mut frame_type = 0;
            let Some(after_type) = frames_varint_decode(tail, &mut frame_type) else {
                return self.connection_error(0x7, 0);
            };
            match frame_type {
                x if x >= crate::frames::FrameType::StreamRangeMin as u64
                    && x <= crate::frames::FrameType::StreamRangeMax as u64 =>
                {
                    let Some(rest) =
                        decode_stream_frame(self, frame_start, received_data, current_time)
                    else {
                        return self.connection_error(0x7, frame_type);
                    };
                    tail = rest;
                }
                x if x == crate::frames::FrameType::CryptoHs as u64 => {
                    let Some(rest) =
                        decode_crypto_hs_frame(self, frame_start, received_data, epoch)
                    else {
                        return self.connection_error(0x7, frame_type);
                    };
                    tail = rest;
                }
                x if x == crate::frames::FrameType::Ack as u64
                    || x == crate::frames::FrameType::AckEcn as u64
                    || x == crate::frames::FrameType::PathAck as u64
                    || x == crate::frames::FrameType::PathAckEcn as u64 =>
                {
                    let mut num_block = 0;
                    let mut path_id = 0;
                    let mut largest = 0;
                    let mut ack_delay = 0;
                    let mut consumed = 0;
                    if parse_ack_header(
                        frame_start,
                        frame_start.len(),
                        &mut num_block,
                        &mut path_id,
                        &mut largest,
                        &mut ack_delay,
                        &mut consumed,
                        self.remote_parameters.ack_delay_exponent,
                    ) != 0
                    {
                        return self.connection_error(0x7, frame_type);
                    }
                    let mut consumed_total = 0;
                    let mut pure_ack = 0;
                    if skip_frame(
                        frame_start,
                        frame_start.len(),
                        &mut consumed_total,
                        &mut pure_ack,
                    ) != 0
                    {
                        return self.connection_error(0x7, frame_type);
                    }
                    tail = &frame_start[consumed_total..];
                    let _ = path_id;
                    let _ = ack_delay;
                    let _ = largest;
                    let _ = num_block;
                }
                x if x == crate::frames::FrameType::PathChallenge as u64 => {
                    if after_type.len() < 8 {
                        return self.connection_error(0x7, frame_type);
                    }
                    if let Some(tuple) = path_x.tuples.first_mut() {
                        tuple.challenge_response = parse_64(&after_type[..8]);
                        tuple.response_required = true;
                    }
                    tail = &after_type[8..];
                }
                x if x == crate::frames::FrameType::PathResponse as u64 => {
                    if after_type.len() < 8 {
                        return self.connection_error(0x7, frame_type);
                    }
                    let response = parse_64(&after_type[..8]);
                    for tuple in &mut path_x.tuples {
                        if tuple.challenge.contains(&response) {
                            tuple.challenge_verified = true;
                            tuple.challenge_required = false;
                            tuple.challenge_failed = false;
                        }
                    }
                    tail = &after_type[8..];
                }
                x if x == crate::frames::FrameType::Datagram as u64
                    || x == crate::frames::FrameType::DatagramL as u64 =>
                {
                    let mut frame_id = 0;
                    let mut length = 0;
                    let Some(payload) =
                        decode_datagram_frame_header(frame_start, &mut frame_id, &mut length)
                    else {
                        return self.connection_error(0x7, frame_type);
                    };
                    tail = &payload[length as usize..];
                    let _ = frame_id;
                }
                x if x == crate::frames::FrameType::NewConnectionId as u64
                    || x == crate::frames::FrameType::PathNewConnectionId as u64 =>
                {
                    let mut consumed = 0;
                    let mut pure_ack = 0;
                    if skip_frame(frame_start, frame_start.len(), &mut consumed, &mut pure_ack) != 0
                    {
                        return self.connection_error(0x7, frame_type);
                    }
                    tail = &frame_start[consumed..];
                }
                x if x == crate::frames::FrameType::ConnectionClose as u64 => {
                    self.remote_error = 1;
                    self.connection_state = State::ClosingReceived;
                    return 0;
                }
                x if x == crate::frames::FrameType::ApplicationClose as u64 => {
                    self.remote_application_error = 1;
                    self.connection_state = State::ClosingReceived;
                    return 0;
                }
                _ => {
                    let mut consumed = 0;
                    let mut pure_ack = 0;
                    if skip_frame(frame_start, frame_start.len(), &mut consumed, &mut pure_ack) != 0
                        || consumed == 0
                    {
                        return self.connection_error(0x7, frame_type);
                    }
                    tail = &frame_start[consumed..];
                }
            }
        }
        path_x.last_non_path_probing_pn = pn64;
        if let Some(addr) = addr_from {
            path_x.update_peer_addr(Some(addr));
        }
        0
    }
```

## `picoquic/frames.c:picoquic_format_blocked_frames`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C iterates streams and formats per-stream blocked frames with high-priority filtering; Rust only emits a connection-level data-blocked frame.
* C source: `picoquic/frames.c:1782-1805`
* C signature: `uint8_t * picoquic_format_blocked_frames(picoquic_cnx_t *, uint8_t *, uint8_t *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:13495-13505`
* Rust item: `format_blocked_frames`

### C body
```c
{
    picoquic_stream_head_t* stream = picoquic_first_stream(cnx);
    picoquic_stream_head_t* hi_pri_stream = NULL;

    /* Check whether there is a high priority stream declared */
    if (cnx->high_priority_stream_id != UINT64_MAX) {
        hi_pri_stream = picoquic_find_stream(cnx, cnx->high_priority_stream_id);
    }

    /* Look for blocked streams, as long as there is message space available */
    while (stream != NULL) {
        if (hi_pri_stream == NULL || stream == hi_pri_stream) {
            bytes = picoquic_format_one_blocked_frame(cnx, bytes, bytes_max, more_data, is_pure_ack, stream);
            if (*more_data) {
                break;
            }
        }

        stream = picoquic_next_stream(stream);
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    if connection.maxdata_remote <= connection.data_sent && !connection.sent_blocked_frame {
        return format_data_blocked_frame(connection, bytes, more_data, is_pure_ack);
    }
    Some(bytes)
}
```

## `picoquic/frames.c:picoquic_is_tls_stream_ready`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown contains unrelated StreamHead methods and no visible TLS stream readiness loop or return equivalent.
* C source: `picoquic/frames.c:2508-2524`
* C signature: `int picoquic_is_tls_stream_ready(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:9718-9763`
* Rust item: `is_tls_stream_ready`

### C body
```c
{
    int ret = 0;

    for (int epoch = 0; epoch < 4; epoch++) {
        picoquic_stream_head_t* stream = &cnx->tls_stream[epoch];

        if (stream->send_queue != NULL &&
            stream->send_queue->length > stream->send_queue->offset &&
            cnx->crypto_context[epoch].aead_encrypt != NULL) {
            ret = 1;
            break;
        }
    }

    return ret;
}
```

### Rust body
```rust
impl StreamHead {
    /// True when `stream` has finished sending and receiving all data.
    /// C: `picoquic_is_stream_closed`.
    pub fn is_stream_closed(&self, client_mode: bool) -> bool {
        use crate::stream::{Role, StreamId};
        let sid = StreamId(self.stream_id);
        let local_role = if client_mode {
            Role::Client
        } else {
            Role::Server
        };
        if sid.is_bidir() {
            ((self.fin_requested && self.fin_sent) || (self.reset_requested && self.reset_sent))
                && ((self.fin_received && self.fin_signalled)
                    || (self.reset_received && self.reset_signalled))
        } else if sid.is_local(local_role) {
            // Unidir from local host.
            (self.fin_requested && self.fin_sent) || (self.reset_requested && self.reset_sent)
        } else {
            // Unidir from remote.
            (self.fin_received && self.fin_signalled)
                || (self.reset_received && self.reset_signalled)
        }
    }

    /// Mark the stream as reset and drop all queued send data beyond
    /// `reliable_size` bytes.  Data already in the queue with
    /// `offset < reliable_size` is kept so it can be sent before the
    /// RESET_STREAM frame.
    ///
    /// C: `picoquic_enforce_reset_stream_frame` (`frames.c:252-282`)
    pub fn enforce_reset_stream_frame(&mut self, reliable_size: u64) {
        self.reset_sent = true;
        // send_queue is ordered by offset; find the first node that lies
        // at or beyond reliable_size and truncate everything from there.
        let keep = self
            .send_queue
            .partition_point(|node| node.offset < reliable_size);
        self.send_queue.truncate(keep);
    }
}
```
