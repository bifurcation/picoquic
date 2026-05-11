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

## `picoquic/error_names.c:picoquic_error_name`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown is a test function with cases and assertions, not the error-name mapping function body.
* C source: `picoquic/error_names.c:25-132`
* C signature: `const char * picoquic_error_name(uint64_t)`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1531-1647`
* Rust item: `error_name`

### C body
```c
{
    char const* e_name = "unknown";
    switch (error_code) {
        /* Protocol errors defined in the QUIC spec */
    case PICOQUIC_TRANSPORT_INTERNAL_ERROR: e_name = "internal"; break;
    case PICOQUIC_TRANSPORT_SERVER_BUSY: e_name = "server busy"; break;
    case PICOQUIC_TRANSPORT_FLOW_CONTROL_ERROR: e_name = "flow control"; break;
    case PICOQUIC_TRANSPORT_STREAM_LIMIT_ERROR: e_name = "stream limit"; break;
    case PICOQUIC_TRANSPORT_STREAM_STATE_ERROR: e_name = "stream state"; break;
    case PICOQUIC_TRANSPORT_FINAL_OFFSET_ERROR: e_name = "final offset"; break;
    case PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR: e_name = "frame format"; break;
    case PICOQUIC_TRANSPORT_PARAMETER_ERROR: e_name = "parameter"; break;
    case PICOQUIC_TRANSPORT_CONNECTION_ID_LIMIT_ERROR: e_name = "connection_id limit"; break;
    case PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION: e_name = "protocol violation"; break;
    case PICOQUIC_TRANSPORT_INVALID_TOKEN: e_name = "invalid token"; break;
    case PICOQUIC_TRANSPORT_APPLICATION_ERROR: e_name = "application"; break;
    case PICOQUIC_TRANSPORT_CRYPTO_BUFFER_EXCEEDED: e_name = "crypto buffer exceeded"; break;
    case PICOQUIC_TRANSPORT_KEY_UPDATE_ERROR: e_name = "key update"; break;
    case PICOQUIC_TRANSPORT_AEAD_LIMIT_REACHED: e_name = "aead limit"; break;
    case PICOQUIC_TLS_ALERT_WRONG_ALPN: e_name = "wrong alpn"; break;
    case PICOQUIC_TLS_HANDSHAKE_FAILED: e_name = "tls handshake failed"; break;
    case PICOQUIC_TRANSPORT_VERSION_NEGOTIATION_ERROR: e_name = "version negotiation"; break;
    case PICOQUIC_TRANSPORT_APPLICATION_ABANDON: e_name = "application abandon"; break;
    case PICOQUIC_TRANSPORT_RESOURCE_LIMIT_REACHED: e_name = "resource limit reached"; break;
    case PICOQUIC_TRANSPORT_UNSTABLE_INTERFACE: e_name = "unstable interface"; break;
    case PICOQUIC_TRANSPORT_NO_CID_AVAILABLE: e_name = "no CID available"; break;
        /* Picoquic local error codes. */
    case PICOQUIC_ERROR_DUPLICATE: e_name = "duplicate"; break;
    case PICOQUIC_ERROR_AEAD_CHECK: e_name = "payload_decrypt_error"; break;
    case PICOQUIC_ERROR_UNEXPECTED_PACKET: e_name = "unexpected packet"; break;
    case PICOQUIC_ERROR_MEMORY: e_name = "memory"; break;
    case PICOQUIC_ERROR_CNXID_CHECK: e_name = "connection ID check"; break;
    case PICOQUIC_ERROR_INITIAL_TOO_SHORT: e_name = ""; break;
    case PICOQUIC_ERROR_VERSION_NEGOTIATION_SPOOFED: e_name = "version negotation spoofed"; break;
    case PICOQUIC_ERROR_MALFORMED_TRANSPORT_EXTENSION: e_name = "malformed transport extension"; break;
    case PICOQUIC_ERROR_EXTENSION_BUFFER_TOO_SMALL: e_name = "extension buffer too small"; break;
    case PICOQUIC_ERROR_ILLEGAL_TRANSPORT_EXTENSION: e_name = "illegal transport extension"; break;
    case PICOQUIC_ERROR_CANNOT_RESET_STREAM_ZERO: e_name = "cannot reset the crypto stream"; break;
    case PICOQUIC_ERROR_INVALID_STREAM_ID: e_name = "invalid stream id"; break;
    case PICOQUIC_ERROR_STREAM_ALREADY_CLOSED: e_name = "stream already closed"; break;
    case PICOQUIC_ERROR_FRAME_BUFFER_TOO_SMALL: e_name = "frame buffer too small"; break;
    case PICOQUIC_ERROR_INVALID_FRAME: e_name = "invalid frame"; break;
    case PICOQUIC_ERROR_CANNOT_CONTROL_STREAM_ZERO: e_name = "cannot control the crypto stream"; break;
    case PICOQUIC_ERROR_RETRY: e_name = "retry"; break;
    case PICOQUIC_ERROR_DISCONNECTED: e_name = "disconnected"; break;
    case PICOQUIC_ERROR_DETECTED: e_name = "error detected"; break;
    case PICOQUIC_ERROR_INVALID_TICKET: e_name = "invalid ticket"; break;
    case PICOQUIC_ERROR_INVALID_FILE: e_name = "invalid file"; break;
    case PICOQUIC_ERROR_SEND_BUFFER_TOO_SMALL: e_name = "send buffer too small"; break;
    case PICOQUIC_ERROR_UNEXPECTED_STATE: e_name = "unexpected state"; break;
    case PICOQUIC_ERROR_UNEXPECTED_ERROR: e_name = "unexpected error"; break;
    case PICOQUIC_ERROR_TLS_SERVER_CON_WITHOUT_CERT: e_name = "server configuration without cert"; break;
    case PICOQUIC_ERROR_NO_SUCH_FILE: e_name = "no such file"; break;
    case PICOQUIC_ERROR_STATELESS_RESET: e_name = "stateless reset"; break;
    case PICOQUIC_ERROR_CONNECTION_DELETED: e_name = "connection deleted"; break;
    case PICOQUIC_ERROR_CNXID_SEGMENT: e_name = "connection ID segment error"; break;
    case PICOQUIC_ERROR_CNXID_NOT_AVAILABLE: e_name = "connection ID not available"; break;
    case PICOQUIC_ERROR_MIGRATION_DISABLED: e_name = "migration disabled"; break;
    case PICOQUIC_ERROR_CANNOT_COMPUTE_KEY: e_name = "cannot compute key"; break;
    case PICOQUIC_ERROR_CANNOT_SET_ACTIVE_STREAM: e_name = "cannot set active stream"; break;
    case PICOQUIC_ERROR_CANNOT_CHANGE_ACTIVE_CONTEXT: e_name = "cannot change active context"; break;
    case PICOQUIC_ERROR_INVALID_TOKEN: e_name = "invalid token"; break;
    case PICOQUIC_ERROR_INITIAL_CID_TOO_SHORT: e_name = "initial CID too short"; break;
    case PICOQUIC_ERROR_KEY_ROTATION_NOT_READY: e_name = "key rotation not ready"; break;
    case PICOQUIC_ERROR_AEAD_NOT_READY: e_name = "aead not ready"; break;
    case PICOQUIC_ERROR_NO_ALPN_PROVIDED: e_name = "no ALPN provided"; break;
    case PICOQUIC_ERROR_NO_CALLBACK_PROVIDED: e_name = "no callback provided"; break;
    case PICOQUIC_STREAM_RECEIVE_COMPLETE: e_name = "stream receive complete"; break;
    case PICOQUIC_ERROR_PACKET_HEADER_PARSING: e_name = "packet header parsing"; break;
    case PICOQUIC_ERROR_QUIC_BIT_MISSING: e_name = "QUIC bit missing"; break;
    case PICOQUIC_NO_ERROR_TERMINATE_PACKET_LOOP: e_name = "terminate packet loop (not an error)"; break;
    case PICOQUIC_NO_ERROR_SIMULATE_NAT: e_name = "simulate NAT (not an error)"; break;
    case PICOQUIC_NO_ERROR_SIMULATE_MIGRATION: e_name = "simulate migration (not an error)"; break;
    case PICOQUIC_ERROR_VERSION_NOT_SUPPORTED: e_name = "version not supported"; break;
    case PICOQUIC_ERROR_IDLE_TIMEOUT: e_name = "idle timeout"; break;
    case PICOQUIC_ERROR_REPEAT_TIMEOUT: e_name = "repeat timeout"; break;
    case PICOQUIC_ERROR_HANDSHAKE_TIMEOUT: e_name = "handshake timeout"; break;
    case PICOQUIC_ERROR_SOCKET_ERROR: e_name = "socket"; break;
    case PICOQUIC_ERROR_VERSION_NEGOTIATION: e_name = "version negotiation"; break;
    case PICOQUIC_ERROR_PACKET_TOO_LONG: e_name = "packet too long"; break;
    case PICOQUIC_ERROR_PACKET_WRONG_VERSION: e_name = "wrong version"; break;
    case PICOQUIC_ERROR_PORT_BLOCKED: e_name = "port blocked"; break;
    case PICOQUIC_ERROR_DATAGRAM_TOO_LONG: e_name = "datagram too long"; break;
    case PICOQUIC_ERROR_PATH_ID_INVALID: e_name = "invalid path ID"; break;
    case PICOQUIC_ERROR_RETRY_NEEDED: e_name = "retry needed"; break;
    case PICOQUIC_ERROR_SERVER_BUSY: e_name = "server busy"; break;
    case PICOQUIC_ERROR_PATH_DUPLICATE: e_name = "duplicate path"; break;
    case PICOQUIC_ERROR_PATH_ID_BLOCKED: e_name = "blocked by lack of path ID"; break;
    case PICOQUIC_ERROR_PATH_CID_BLOCKED: e_name = "blocked by lack of CID"; break;
    case PICOQUIC_ERROR_PATH_ADDRESS_FAMILY: e_name = "path address family"; break;
    case PICOQUIC_ERROR_PATH_NOT_READY: e_name = "path not ready"; break;
    case PICOQUIC_ERROR_PATH_LIMIT_EXCEEDED: e_name = "path limit exceeded"; break;
    case PICOQUIC_ERROR_REDIRECTED: e_name = "redirected to proxy (not an error)"; break; /* Not an error: the packet was captured by a proxy, no further processing needed */
    case PICOQUIC_ERROR_PADDING_PACKET: e_name = "padding_packet"; break; /* Random bytes at end of datagram */
    default:
        if (error_code > 0x100 && error_code < 0x200) {
            /* Protocol errors defined in the QUIC spec */
            e_name = "crypto error alert";
        }
        else if (error_code > 0x400 && error_code < 0x500) {
            /* Picoquic error codes */
            e_name = "unknown picoquic error";
        }
        break;
    }
    return e_name;
}
```

### Rust body
```rust
fn error_name() {
    use crate::errors::InternalError;

    let cases: &[(u64, &str)] = &[
        // Protocol (transport / TLS) errors.
        (0x1, "internal"),
        (0x2, "server busy"),
        (0x3, "flow control"),
        (0x4, "stream limit"),
        (0x5, "stream state"),
        (0x6, "final offset"),
        (0x7, "frame format"),
        (0x8, "parameter"),
        (0x9, "connection_id limit"),
        (0xA, "protocol violation"),
        (0xB, "invalid token"),
        (0xC, "application"),
        (0xD, "crypto buffer exceeded"),
        (0xE, "key update"),
        (0xF, "aead limit"),
        (0x178, "wrong alpn"),
        (0x201, "tls handshake failed"),
        (0x11, "version negotiation"),
        (0x3e, "application abandon"),
        (0x3e75, "resource limit reached"),
        (0x3e76, "unstable interface"),
        (0x3e77, "no CID available"),
        // Picoquic-internal codes (0x400+ range).
        (0x401, "duplicate"),
        (0x403, "payload_decrypt_error"),
        (0x404, "unexpected packet"),
        (0x405, "memory"),
        (0x407, "connection ID check"),
        (0x408, ""),
        (0x409, "version negotation spoofed"),
        (0x40A, "malformed transport extension"),
        (0x40B, "extension buffer too small"),
        (0x40C, "illegal transport extension"),
        (0x40D, "cannot reset the crypto stream"),
        (0x40E, "invalid stream id"),
        (0x40F, "stream already closed"),
        (0x410, "frame buffer too small"),
        (0x411, "invalid frame"),
        (0x412, "cannot control the crypto stream"),
        (0x413, "retry"),
        (0x414, "disconnected"),
        (0x415, "error detected"),
        (0x417, "invalid ticket"),
        (0x418, "invalid file"),
        (0x419, "send buffer too small"),
        (0x41A, "unexpected state"),
        (0x41B, "unexpected error"),
        (0x41C, "server configuration without cert"),
        (0x41D, "no such file"),
        (0x41E, "stateless reset"),
        (0x41F, "connection deleted"),
        (0x420, "connection ID segment error"),
        (0x421, "connection ID not available"),
        (0x422, "migration disabled"),
        (0x423, "cannot compute key"),
        (0x424, "cannot set active stream"),
        (0x425, "cannot change active context"),
        (0x426, "invalid token"),
        (0x427, "initial CID too short"),
        (0x428, "key rotation not ready"),
        (0x429, "aead not ready"),
        (0x42A, "no ALPN provided"),
        (0x42B, "no callback provided"),
        (0x42C, "stream receive complete"),
        (0x42D, "packet header parsing"),
        (0x42E, "QUIC bit missing"),
        (0x42F, "terminate packet loop (not an error)"),
        (0x430, "simulate NAT (not an error)"),
        (0x431, "simulate migration (not an error)"),
        (0x432, "version not supported"),
        (0x433, "idle timeout"),
        (0x434, "repeat timeout"),
        (0x435, "handshake timeout"),
        (0x436, "socket"),
        (0x437, "version negotiation"),
        (0x438, "packet too long"),
        (0x439, "wrong version"),
        (0x43A, "port blocked"),
        (0x43B, "datagram too long"),
        (0x43C, "invalid path ID"),
        (0x43D, "retry needed"),
        (0x43E, "server busy"),
        (0x43F, "duplicate path"),
        (0x440, "blocked by lack of path ID"),
        (0x441, "blocked by lack of CID"),
        (0x442, "path address family"),
        (0x443, "path not ready"),
        (0x444, "path limit exceeded"),
        (0x445, "redirected to proxy (not an error)"),
        (0x446, "padding_packet"),
        // CRYPTO_ERROR alert range (default branch).
        (0x101, "crypto error alert"),
        (0x150, "crypto error alert"),
        (0x1FF, "crypto error alert"),
        // Unknown picoquic error range (default branch).
        (0x450, "unknown picoquic error"),
        (0x4FF, "unknown picoquic error"),
        // Truly unknown.
        (0x0, "unknown"),
        (0x200, "unknown"),
        (0x500, "unknown"),
        (0xFFFF_FFFF_FFFF_FFFF, "unknown"),
    ];

    for &(code, expected) in cases {
        let got = InternalError::name(code).unwrap_or("<none>");
        assert_eq!(
            got, expected,
            "error_code=0x{code:x} got={got:?} expected={expected:?}",
        );
    }
}
```

## `picoquic/frames.c:picoquic_compute_ack_gap_and_delay`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust uses different direct formulas and lacks the C correction block based on smoothed RTT, return data rate, ACK transmission time, and final gap cap condition.
* C source: `picoquic/frames.c:3066-3130`
* C signature: `void picoquic_compute_ack_gap_and_delay(picoquic_cnx_t *, uint64_t, uint64_t, uint64_t, uint64_t *, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:8852-8925`
* Rust item: `compute_ack_gap_and_delay`

### C body
```c
{
    uint64_t nb_packets = picoquic_compute_packets_in_window(cnx, data_rate);

    *ack_delay_max = picoquic_compute_ack_delay_max(cnx, rtt, remote_min_ack_delay);
    *ack_gap = picoquic_compute_ack_gap(cnx, data_rate, nb_packets);

    if (2 * cnx->path[0]->smoothed_rtt > 3 * cnx->path[0]->rtt_min) {
        uint64_t return_data_rate = 0;

        /* This code kicks in when the smoothed RTT is larger than 1.5 times the RTT Min.
         * If that is the case, the default computation of ACK gap and ACK delay may
         * be wrong, and a more conservative computation is required.
         * This code assume that ACK gap and ACK delay are already computed using
         * the default algorithms.
         */
        if (cnx->is_ack_frequency_negotiated) {
            return_data_rate = cnx->path[0]->receive_rate_max;
        }
        else {
            return_data_rate = cnx->path[0]->bandwidth_estimate;
        }

        if (nb_packets < 2) {
            nb_packets = 2;
        }
        if (return_data_rate > 0) {
            /* Estimate of ACK size = L2 + IPv6 + UDP + padded ACK */
            const uint64_t ack_size = 12 + 40 + 8 + 55;
            /* Estimate of ACK transmission time *in microseconds */
            uint64_t ack_transmission_time = (ack_size * 1000000) / return_data_rate;
            /* if ACK transmission time > ack delay, perform correction */
            if (ack_transmission_time > * ack_delay_max) {
                *ack_delay_max = ack_transmission_time;
                if (*ack_delay_max > PICOQUIC_ACK_DELAY_MAX) {
                    *ack_delay_max = PICOQUIC_ACK_DELAY_MAX;
                }
            }
            /* if ack gap smaller than ack time fraction of CWIN, perform correction */
            uint64_t rtt_target = (cnx->path[0]->smoothed_rtt + cnx->path[0]->rtt_min) / 2;

            if (!cnx->path[0]->is_ssthresh_initialized) {
                nb_packets /= 2;
            }

            uint64_t nb_ack_per_rtt = (*ack_gap > 0) ? (nb_packets + *ack_gap - 1) / (*ack_gap):nb_packets;
            if (nb_ack_per_rtt * (*ack_delay_max) > rtt_target) {
                uint64_t nb_acks_max = cnx->path[0]->smoothed_rtt / (*ack_delay_max);
                if (nb_acks_max <= 1) {
                    *ack_gap = nb_packets;
                }
                else {
                    uint64_t ack_gap_min = (nb_packets + nb_acks_max - 1) / nb_acks_max;
                    if (*ack_gap < ack_gap_min) {
                        *ack_gap = ack_gap_min;
                    }
                }
            }
        }
    }
    if (cnx->path[0]->rtt_min < *ack_delay_max * 4 && *ack_gap > 32) {
        *ack_gap = 32;
    }
}
```

### Rust body
```rust
    ) {
        let first_path = self.paths.first();
        let rtt_ticks = rtt.ticks();
        let bytes_in_window = data_rate
            .saturating_mul(rtt_ticks)
            .saturating_div(1_000_000);
        let mut nb_packets = (bytes_in_window / MAX_PACKET_SIZE as u64).max(2);

        *ack_delay_max = (rtt_ticks / 4).min(ACK_DELAY_MAX.ticks());
        if !self.is_ack_frequency_negotiated
            && first_path
                .map(|p| !p.is_ssthresh_initialized)
                .unwrap_or(true)
        {
            *ack_delay_max /= 2;
        }
        *ack_delay_max = (*ack_delay_max).max(remote_min_ack_delay);

        if self.is_ack_frequency_negotiated
            && first_path
                .map(|p| !p.is_ssthresh_initialized)
                .unwrap_or(false)
        {
            nb_packets /= 2;
        }

        if let Some(path) = first_path
            && path.rtt_min < Duration::from_ticks(4 * ACK_DELAY_MIN.ticks())
        {
            let mult = if path.rtt_min > ACK_DELAY_MIN {
                (4 * ACK_DELAY_MIN.ticks()) / path.rtt_min.ticks().max(1)
            } else {
                4
            };
            nb_packets = nb_packets.saturating_mul(mult);
        }

        let mut gap = nb_packets.div_ceil(4);
        let mut gap_min = 2;
        if data_rate > BANDWIDTH_MEDIUM {
            gap_min = if first_path
                .map(|p| p.rtt_min > TARGET_RENO_RTT)
                .unwrap_or(false)
            {
                10
            } else {
                4
            };
        }
        if gap < gap_min {
            gap = gap_min;
        } else if gap > 32 {
            let cc_number = self
                .congestion_alg
                .map(|cc| cc.congestion_algorithm_number)
                .unwrap_or(CC_ALGO_NUMBER_NEW_RENO);
            if self.is_multipath_enabled
                || cc_number == CC_ALGO_NUMBER_NEW_RENO
                || cc_number == CC_ALGO_NUMBER_FAST
            {
                gap = 32;
            } else {
                gap = (32 + nb_packets.saturating_sub(128) / 8).min(64);
            }
        }
        *ack_gap = gap;
    }
```

## `picoquic/frames.c:picoquic_estimate_max_path_bandwidth`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown is only a fragment and omits the C send_time guard and bandwidth estimate/update logic.
* C source: `picoquic/frames.c:2924-2961`
* C signature: `void picoquic_estimate_max_path_bandwidth(picoquic_path_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:6130-6136`
* Rust item: `estimate_max_path_bandwidth`

### C body
```c
{
    /* Test whether there is enough time since the last max bandwidth estimate */
    if (send_time >= path_x->max_sample_sent_time) {
        if (path_x->max_sample_sent_time == 0) {
            /* No sample set yet, need to initialize the variables */
            path_x->max_sample_delivered = path_x->delivered;
            path_x->max_sample_acked_time = delivery_time;
            path_x->max_sample_sent_time = send_time;
        }
        else {
            /* Compute a max bandwidth estimate */
            uint64_t receive_interval = delivery_time - path_x->max_sample_acked_time;

            if (receive_interval > PICOQUIC_MAX_BANDWIDTH_TIME_INTERVAL_MIN) {
                uint64_t delivered = path_x->delivered - path_x->max_sample_delivered;
                uint64_t send_interval = send_time - path_x->max_sample_sent_time;
                uint64_t bw_estimate;

                if (send_interval > receive_interval) {
                    receive_interval = send_interval;
                }

                bw_estimate = PICOQUIC_RATE_FROM_BYTES(delivered, receive_interval);
                /* Retain if larger than previous estimate */
                if (bw_estimate > path_x->peak_bandwidth_estimate) {
                    path_x->peak_bandwidth_estimate = bw_estimate;
                }

                /* Change the reference point if estimate duration is long enough */
                path_x->max_sample_delivered = path_x->delivered;
                path_x->max_sample_acked_time = delivery_time;
                path_x->max_sample_sent_time = send_time;
            }
        }
    }
}
```

### Rust body
```rust
            if self.max_sample_sent_time.ticks() == 0 {
                self.max_sample_delivered = self.delivered;
                self.max_sample_acked_time = delivery_time;
                self.max_sample_sent_time = send_time;
            } else {
```

## `picoquic/frames.c:picoquic_format_new_connection_id_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust falls back to the first CID when l_cid is absent, uses a different path id source, copies a stored reset secret instead of creating one for the CID, and marks the CID acked.
* C source: `picoquic/frames.c:593-619`
* C signature: `uint8_t * picoquic_format_new_connection_id_frame(picoquic_cnx_t *, picoquic_local_cnxid_list_t *, uint8_t *, uint8_t *, int *, int *, picoquic_local_cnxid_t *)`
* Rust source: `rs/fq/src/internal.rs:13364-13405`
* Rust item: `format_new_connection_id_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    unsigned int is_mp = cnx->is_multipath_enabled;

    if (l_cid != NULL && l_cid->cnx_id.id_len > 0) {
        if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, 
            (is_mp)?picoquic_frame_type_path_new_connection_id:picoquic_frame_type_new_connection_id)) == NULL ||
            (is_mp && ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, l_cid->path_id)) == NULL)) ||
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, l_cid->sequence)) == NULL ||
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, local_cnxid_list->local_cnxid_retire_before)) == NULL ||
            (bytes = picoquic_frames_cid_encode(bytes, bytes_max, &l_cid->cnx_id)) == NULL ||
            (bytes + PICOQUIC_RESET_SECRET_SIZE) > bytes_max) {
            *more_data = 1;
            bytes = bytes0;
        }
        else {
            *is_pure_ack = 0;
            (void)picoquic_create_cnxid_reset_secret(cnx->quic, &l_cid->cnx_id, bytes);
            bytes += PICOQUIC_RESET_SECRET_SIZE;
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let token = l_cid.or_else(|| local_connection_id_list.connection_ids.first().copied())?;
    let cid = connection.local_connection_ids.get(token)?;
    let frame_type = if connection.is_multipath_enabled {
        crate::frames::FrameType::PathNewConnectionId as u64
    } else {
        crate::frames::FrameType::NewConnectionId as u64
    };
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, frame_type)
        || (connection.is_multipath_enabled
            && !encode_varint_at(bytes, &mut off, local_connection_id_list.unique_path_id))
        || !encode_varint_at(bytes, &mut off, cid.sequence)
        || !encode_varint_at(
            bytes,
            &mut off,
            local_connection_id_list.local_connection_id_retire_before,
        )
        || bytes.len() < off + 1 + cid.connection_id.len() + RESET_SECRET_SIZE
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off] = cid.connection_id.len() as u8;
    off += 1;
    bytes[off..off + cid.connection_id.len()].copy_from_slice(cid.connection_id.as_bytes());
    off += cid.connection_id.len();
    bytes[off..off + RESET_SECRET_SIZE].copy_from_slice(&connection.registered_reset_secret);
    off += RESET_SECRET_SIZE;
    if let Some(cid) = connection.local_connection_ids.get_mut(token) {
        cid.is_acked = true;
    }
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## `picoquic/frames.c:picoquic_process_ack_of_ack_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C parses the ACK header and processes the ACK body; Rust immediately returns -1.
* C source: `picoquic/frames.c:3352-3367`
* C signature: `int picoquic_process_ack_of_ack_frame(picoquic_sack_list_t *, uint8_t *, size_t, size_t *, int)`
* Rust source: `rs/fq/src/internal.rs:8781-8804`
* Rust item: `process_ack_of_ack_frame`

### C body
```c
{
    int ret;
    uint64_t largest;
    uint64_t ack_delay;
    uint64_t num_block;

    ret = picoquic_parse_ack_header(bytes, bytes_max, &num_block, NULL, &largest, &ack_delay, consumed, 0);

    if (ret == 0) {
        ret = picoquic_process_ack_of_ack_body(sack_list, largest, num_block, bytes, bytes_max, consumed, is_ecn);
    }

    return ret;
}
```

### Rust body
```rust
        {
            return -1;
        }
```
