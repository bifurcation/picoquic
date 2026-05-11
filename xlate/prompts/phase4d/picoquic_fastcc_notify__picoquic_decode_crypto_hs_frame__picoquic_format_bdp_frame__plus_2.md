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

## `picoquic/fastcc.c:picoquic_fastcc_notify`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only takes congestion_alg_state and returns on None; it omits the C update flag, freeze handling, notification switch, and all congestion logic.
* C source: `picoquic/fastcc.c:162-306`
* C signature: `void picoquic_fastcc_notify(picoquic_cnx_t *, picoquic_path_t *, picoquic_congestion_notification_t, picoquic_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/fastcc.rs:168-179`
* Rust item: `picoquic_fastcc_notify`

### C body
```c
{
    picoquic_fastcc_state_t* fastcc_state = (picoquic_fastcc_state_t*)path_x->congestion_alg_state;
    path_x->is_cc_data_updated = 1;

    if (fastcc_state != NULL) {
        if (fastcc_state->alg_state == picoquic_fastcc_freeze && 
            (current_time > fastcc_state->end_of_freeze ||
                fastcc_state->recovery_sequence <= picoquic_cc_get_ack_number(cnx, path_x))) {
            if (fastcc_state->last_freeze_was_timeout) {
                fastcc_state->alg_state = picoquic_fastcc_initial;
            }
            else {
                fastcc_state->alg_state = picoquic_fastcc_eval;
            }
            fastcc_state->last_freeze_was_not_delay = 0;
            fastcc_state->last_freeze_was_timeout = 0;

            fastcc_state->nb_cc_events = 0;
            fastcc_state->nb_bytes_ack_since_rtt = 0;
        }

        switch (notification) {
        case picoquic_congestion_notification_acknowledgement: 
            if (fastcc_state->alg_state != picoquic_fastcc_freeze) {
                /* Count the bytes since last RTT measurement */
                fastcc_state->nb_bytes_ack_since_rtt += ack_state->nb_bytes_acknowledged;
                /* Compute pacing data. */
                picoquic_update_pacing_data(path_x, 0);
            }
            break;

        case picoquic_congestion_notification_ecn_ec:
            fastcc_notify_congestion(cnx, path_x, fastcc_state, current_time, 0, 0);
            break;
        case picoquic_congestion_notification_repeat:
        case picoquic_congestion_notification_timeout:
            if (picoquic_cc_hystart_loss_test(&fastcc_state->rtt_filter, notification, ack_state->lost_packet_number, PICOQUIC_SMOOTHED_LOSS_THRESHOLD)) {
                fastcc_notify_congestion(cnx, path_x, fastcc_state, current_time, 0,
                    (notification == picoquic_congestion_notification_timeout) ? 1 : 0);
            }
            break;
        case picoquic_congestion_notification_spurious_repeat:
            if (fastcc_state->nb_cc_events > 0) {
                fastcc_state->nb_cc_events--;
            }
            break;
        case picoquic_congestion_notification_rtt_measurement:
        {
            uint64_t delta_rtt = 0;

            picoquic_cc_filter_rtt_min_max(&fastcc_state->rtt_filter, ack_state->rtt_measurement);

            if (fastcc_state->rtt_filter.is_init) {
                /* We use the maximum of the last samples as the candidate for the
                 * min RTT, in order to filter the rtt jitter */
                if (current_time > fastcc_state->end_of_epoch) {
                    /* If end of epoch, reset the min RTT to min of remembered periods,
                     * and roll the period. */
                    fastcc_state->rtt_min = UINT64_MAX;
                    for (int i = FASTCC_NB_PERIOD - 1; i > 0; i--) {
                        fastcc_state->last_rtt_min[i] = fastcc_state->last_rtt_min[i - 1];
                        if (fastcc_state->last_rtt_min[i] > 0 &&
                            fastcc_state->last_rtt_min[i] < fastcc_state->rtt_min) {
                            fastcc_state->rtt_min = fastcc_state->last_rtt_min[i];
                        }
                    }
                    fastcc_state->delay_threshold = picoquic_fastcc_delay_threshold(fastcc_state->rtt_min);
                    fastcc_state->last_rtt_min[0] = fastcc_state->rolling_rtt_min;
                    fastcc_state->rolling_rtt_min = fastcc_state->rtt_filter.sample_max;
                    fastcc_state->end_of_epoch = current_time + FASTCC_PERIOD;
                }
                else if (fastcc_state->rtt_filter.sample_max < fastcc_state->rolling_rtt_min || fastcc_state->rolling_rtt_min == 0) {
                    /* If not end of epoch, update the rolling minimum */
                    fastcc_state->rolling_rtt_min = fastcc_state->rtt_filter.sample_max;
                    if (fastcc_state->rolling_rtt_min < fastcc_state->rtt_min) {
                        fastcc_state->rtt_min = fastcc_state->rolling_rtt_min;
                    }
                }
            }

            if (fastcc_state->alg_state != picoquic_fastcc_freeze) {
                if (ack_state->rtt_measurement < fastcc_state->rtt_min) {
                    fastcc_state->delay_threshold = picoquic_fastcc_delay_threshold(fastcc_state->rtt_min);
                }
                else if (fastcc_state->rtt_min_is_trusted){
                    delta_rtt = ack_state->rtt_measurement - fastcc_state->rtt_min;
                }
                else {
                    fastcc_state->rtt_min = ack_state->rtt_measurement; 
                    fastcc_state->rolling_rtt_min = ack_state->rtt_measurement;
                    fastcc_state->rtt_min_is_trusted = 1;
                    delta_rtt = 0;
                }

                if (delta_rtt < fastcc_state->delay_threshold) {
                    double alpha = 1.0;
                    fastcc_state->nb_cc_events = 0;

                    if (fastcc_state->alg_state != picoquic_fastcc_initial) {
                        alpha -= ((double)delta_rtt / (double)fastcc_state->delay_threshold);
                        alpha *= FASTCC_EVAL_ALPHA;
                    }

                    /* Increase the window if it is not frozen */
                    if (path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                        path_x->cwin += (uint64_t)(alpha * (double)fastcc_state->nb_bytes_ack_since_rtt);
                    }
                    fastcc_state->nb_bytes_ack_since_rtt = 0;
                }
                else {
                    /* May well be congested */
                    fastcc_state->nb_cc_events++;
                    if (fastcc_state->nb_cc_events >= FASTCC_REPEAT_THRESHOLD) {
                        /* Too many events, reduce the window */
                        fastcc_notify_congestion(cnx, path_x, fastcc_state, current_time, 1, 0);
                    }
                }
            }
        }
        break;
        case picoquic_congestion_notification_cwin_blocked:
            break;
        case picoquic_congestion_notification_reset:
            picoquic_fastcc_reset(fastcc_state, path_x, current_time);
            break;
        case picoquic_congestion_notification_seed_cwin:
            picoquic_fastcc_seed_cwin(fastcc_state, path_x, ack_state->nb_bytes_acknowledged);
            break;
        default:
            /* ignore */
            break;
        }
    }
}
```

### Rust body
```rust
    let Some(boxed_state) = path_x.congestion_alg_state.take() else {
        return;
    };
```

## `picoquic/frames.c:picoquic_decode_crypto_hs_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust omits C connection-error signaling, crypto buffer gap check, is-last-frame handling, and queue error propagation.
* C source: `picoquic/frames.c:2541-2574`
* C signature: `const uint8_t * picoquic_decode_crypto_hs_frame(picoquic_cnx_t *, const uint8_t *, const uint8_t *, picoquic_stream_data_node_t *, int)`
* Rust source: `rs/fq/src/internal.rs:12395-12417`
* Rust item: `decode_crypto_hs_frame`

### C body
```c
{
    uint64_t offset;
    uint64_t data_length;
    const uint8_t* data_bytes;

    if ((bytes = picoquic_parse_crypto_hs_frame(bytes, bytes_max, &offset, &data_length, &data_bytes)) == NULL) {
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR, picoquic_frame_type_crypto_hs);
    } else {
        picoquic_stream_head_t* stream = &cnx->tls_stream[epoch];

        if (stream->consumed_offset < offset &&
            stream->consumed_offset + PICOQUIC_MAX_CRYPTO_BUFFER_GAP < offset + data_length) {
            picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_CRYPTO_BUFFER_EXCEEDED, picoquic_frame_type_crypto_hs);
            bytes = NULL;
        }
        else {
            int new_data_available;
            int ret = picoquic_queue_network_input(cnx->quic, &stream->stream_data_tree, stream->consumed_offset,
                offset, data_bytes, (size_t)data_length, picoquic_is_last_stream_frame(bytes + data_length, bytes_max),
                received_data, &new_data_available);

            if (ret != 0) {
                picoquic_connection_error(cnx, (int64_t)ret, picoquic_frame_type_crypto_hs);
                bytes = NULL;
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    let (&frame_type, mut tail) = bytes.split_first()?;
    if frame_type != crate::frames::FrameType::CryptoHs as u8 {
        return None;
    }
    let mut offset = 0;
    tail = frames_varint_decode(tail, &mut offset)?;
    let mut length = 0;
    tail = frames_varint_decode(tail, &mut length)?;
    if tail.len() < length as usize {
        return None;
    }
    let data = &tail[..length as usize];
    if let Some(stream) = connection.tls_stream.get_mut(epoch.clamp(0, 3) as usize) {
        queue_received_stream_data(stream, offset, data, received_data).ok()?;
    }
    Some(&tail[length as usize..])
}
```

## `picoquic/frames.c:picoquic_format_bdp_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body selects ticket-derived values differently for client and server and uses a fixed 24-hour microsecond lifetime; the Rust body uses path fields directly and TOKEN_DELAY_LONG.ticks(), which is visibly different source logic.
* C source: `picoquic/frames.c:6637-6693`
* C signature: `uint8_t * picoquic_format_bdp_frame(picoquic_cnx_t *, uint8_t *, uint8_t *, picoquic_path_t *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:14119-14163`
* Rust item: `format_bdp_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    /* There is no explicit TTL for bdps. We assume they are OK for 24 hours */
    uint64_t lifetime = (uint64_t)(24 * 3600) * ((uint64_t)1000000); 
    uint64_t recon_bytes_in_flight = 0;
    uint64_t recon_min_rtt = 0;
    uint8_t* ip_addr = NULL;
    uint8_t ip_addr_length = 0;

    /* Server sends bdp reflecting current path caracteristics */
    if (!cnx->client_mode) {
        if (path_x->is_ticket_seeded && !path_x->is_bdp_sent) {
            picoquic_issued_ticket_t* server_ticket;
            server_ticket = picoquic_retrieve_issued_ticket(cnx->quic, cnx->issued_ticket_id);
            if (server_ticket != NULL && server_ticket->cwin > 0) {
                recon_bytes_in_flight =  server_ticket->cwin;
                recon_min_rtt = server_ticket->rtt;
                ip_addr = server_ticket->ip_addr;
                ip_addr_length = server_ticket->ip_addr_length;
            }
        }
    }
    else {
        /* Client sends bdp back to the server */
        picoquic_stored_ticket_t* stored_ticket = picoquic_get_stored_ticket(cnx->quic,
            cnx->sni, (uint16_t)strlen(cnx->sni), cnx->alpn, (uint16_t)strlen(cnx->alpn),
            picoquic_supported_versions[cnx->version_index].version, 1, 0);
        if (stored_ticket != NULL) {
            recon_bytes_in_flight = stored_ticket->tp_0rtt[picoquic_tp_0rtt_cwin_remote];
            recon_min_rtt = stored_ticket->tp_0rtt[picoquic_tp_0rtt_rtt_remote];
            /* IP address */
            ip_addr = stored_ticket->ip_addr_client;
            ip_addr_length = stored_ticket->ip_addr_client_length;
        }
    }

    if (recon_bytes_in_flight == 0 ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_bdp)) == NULL || 
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, lifetime)) == NULL || 
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, recon_bytes_in_flight)) == NULL || 
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, recon_min_rtt)) == NULL ||
        (bytes = picoquic_frames_length_data_encode(bytes, bytes_max, ip_addr_length, ip_addr)) == NULL) {
        if (bytes == 0) {
            /* not enough bytes available for the whole frame */
            bytes = bytes0;
            *more_data = 1;
        }
    }
    else {
        *is_pure_ack = 0;
        path_x->is_bdp_sent = 1;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let recon_bytes_in_flight = if path_x.cwin_remote > 0 {
        path_x.cwin_remote
    } else {
        path_x.cwin
    };
    if recon_bytes_in_flight == 0 {
        return Some(bytes);
    }
    let recon_min_rtt = if path_x.rtt_min_remote.ticks() > 0 {
        path_x.rtt_min_remote.ticks()
    } else {
        path_x.rtt_min.ticks()
    };
    let ip_len = path_x.ip_client_remote_length as usize;
    let lifetime = TOKEN_DELAY_LONG.ticks();
    let mut off = 0;
    for value in [
        crate::frames::FrameType::Bdp as u64,
        lifetime,
        recon_bytes_in_flight,
        recon_min_rtt,
        ip_len as u64,
    ] {
        if !encode_varint_at(bytes, &mut off, value) {
            *more_data = 1;
            return Some(bytes);
        }
    }
    if bytes.len() < off + ip_len {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + ip_len].copy_from_slice(&path_x.ip_client_remote[..ip_len]);
    off += ip_len;
    *is_pure_ack = 0;
    path_x.is_bdp_sent = true;
    Some(&mut bytes[off..])
}
```

## `picoquic/frames.c:picoquic_format_stream_frame_header`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown only checks for an empty buffer and contains none of the C header encoding logic.
* C source: `picoquic/frames.c:1866-1878`
* C signature: `uint8_t * picoquic_format_stream_frame_header(uint8_t *, uint8_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:12254-12261`
* Rust item: `format_stream_frame_header`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_stream_range_min)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream_id)) != NULL) {
        if (offset > 0) {
            *bytes0 |= 4; /* Indicates presence of offset */
            bytes = picoquic_frames_varint_encode(bytes, bytes_max, offset);
        }
    }

    return bytes;
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## `picoquic/frames.c:picoquic_provide_datagram_buffer_ex`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body updates datagram-specific state, validates allowed_space, encodes datagram length/header placement, sets after_data, and returns the data buffer; the Rust body is only a one-line delegation to provide_stream_data_buffer with no visible matching logic.
* C source: `picoquic/frames.c:5408-5456`
* C signature: `uint8_t * picoquic_provide_datagram_buffer_ex(void *, size_t, picoquic_datagram_active_enum)`
* Rust source: `rs/fq/src/lib.rs:4418-4424`
* Rust item: `provide_datagram_buffer_ex`

### C body
```c
{
    picoquic_datagram_buffer_argument_t* data_ctx = (picoquic_datagram_buffer_argument_t*)context;
    uint8_t* buffer = NULL;

    data_ctx->is_active = ((int)is_active) & 1;
    data_ctx->was_called = 1;

    if (!data_ctx->is_old_api) {
        /* We apply the state change at this point, rather than after the return of the
        * callback, so as to minimize "developer surprise". If the  application calls
        * "picoquic_mark_datagram_ready" after this call, the value set by the
        * application will stick.
        * There are two active flag: global, if the application is ready to send datagrams
        * on any stream, and per path, if the application wants to send datagrams
        * again on that path.
        */
        data_ctx->cnx->is_datagram_ready = is_active;
        if (data_ctx->path_x != NULL) {
            data_ctx->path_x->is_datagram_ready = ((int)is_active)>>1;
        }
    }

    if (length > 0 && length <= data_ctx->allowed_space) {
        /* Compute the length of header and length field */
        uint8_t* after_length = picoquic_frames_varint_encode(
            data_ctx->bytes, data_ctx->bytes_max, length);
        if (after_length == NULL || after_length + length > data_ctx->bytes_max) {
            /* Too long! */
            uint8_t* bytes = picoquic_frames_varint_encode(data_ctx->bytes0,
                data_ctx->bytes_max, picoquic_frame_type_datagram);
            uint8_t* tail = bytes + length;
            if (tail < data_ctx->bytes_max) {
                size_t delta = data_ctx->bytes_max - tail;
                memset(data_ctx->bytes0, picoquic_frame_type_padding, delta);
                bytes = picoquic_frames_varint_encode(data_ctx->bytes0 + delta,
                    data_ctx->bytes_max, picoquic_frame_type_datagram);
            }
            data_ctx->after_data = bytes + length;
            buffer = bytes;
        }
        else {
            buffer = after_length;
            data_ctx->after_data = after_length + length;
        }
    }

    return buffer;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    provide_stream_data_buffer(context, length, false, false)
}
```
