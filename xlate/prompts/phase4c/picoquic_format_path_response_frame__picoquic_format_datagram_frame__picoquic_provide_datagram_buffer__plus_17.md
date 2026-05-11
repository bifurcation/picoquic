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

## Pair `picoquic/frames.c:picoquic_format_path_response_frame`
C: `picoquic/frames.c:4989-5002 picoquic_format_path_response_frame`
Rust: `rs/fq/src/internal.rs:13332-13346 format_path_response_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_path_response)) != NULL &&
        (bytes = picoquic_frames_uint64_encode(bytes, bytes_max, challenge)) != NULL) {
        *is_pure_ack = 0;
    }
    else {
        *more_data = 1;
        bytes = bytes0;
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    if bytes.len() < 9 {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[0] = crate::frames::FrameType::PathResponse as u8;
    format_64(&mut bytes[1..9], challenge);
    *is_pure_ack = 0;
    Some(&mut bytes[9..])
}
```

## Pair `picoquic/frames.c:picoquic_format_datagram_frame`
C: `picoquic/frames.c:5288-5305 picoquic_format_datagram_frame`
Rust: `rs/fq/src/internal.rs:13890-13910 format_datagram_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_datagram_l)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, length)) != NULL &&
        bytes + length <= bytes_max) {
        memcpy(bytes, src, length);
        bytes += length;
        *is_pure_ack = 0;
    }
    else {
        *more_data = 1;
        bytes = bytes0;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let payload = src.get(..length)?;
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, crate::frames::FrameType::DatagramL as u64)
        || !encode_varint_at(bytes, &mut off, length as u64)
        || bytes.len() < off + length
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + length].copy_from_slice(payload);
    off += length;
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_provide_datagram_buffer`
C: `picoquic/frames.c:5458-5461 picoquic_provide_datagram_buffer`
Rust: `rs/fq/src/lib.rs:4411-4416 provide_datagram_buffer`

### C body
```c
{
    return picoquic_provide_datagram_buffer_ex(context, length, picoquic_datagram_not_active);
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    provide_stream_data_buffer(context, length, false, false)
}
```

## Pair `picoquic/frames.c:picoquic_format_immediate_ack_frame`
C: `picoquic/frames.c:5669-5677 picoquic_format_immediate_ack_frame`
Rust: `rs/fq/src/internal.rs:14074-14088 format_immediate_ack_frame`

### C body
```c
{
    uint8_t* bytes_0 = bytes;
    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_immediate_ack)) == NULL) {
        bytes = bytes_0;
        *more_data = 1;
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::ImmediateAck as u64,
    ) {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_format_path_abandon_frame`
C: `picoquic/frames.c:5830-5843 picoquic_format_path_abandon_frame`
Rust: `rs/fq/src/internal.rs:14165-14183 format_path_abandon_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_path_abandon)) == NULL ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, path_id)) == NULL ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, reason)) == NULL) {
        bytes = bytes0;
        *more_data = 1;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::PathAbandon as u64,
    ) || !encode_varint_at(bytes, &mut off, path_id)
        || !encode_varint_at(bytes, &mut off, reason)
    {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_skip_path_available_or_backup_frame`
C: `picoquic/frames.c:5912-5919 picoquic_skip_path_available_or_backup_frame`
Rust: `rs/fq/src/internal.rs:14857-14861 skip_path_available_or_backup_frame`

### C body
```c
{
    /* This code assumes that the frame type is already skipped */
    if ((bytes = picoquic_frames_varint_skip(bytes, bytes_max)) != NULL){
        bytes = picoquic_frames_varint_skip(bytes, bytes_max);
    }
    return bytes;
}
```

### Rust body
```rust
pub fn skip_path_available_or_backup_frame(_bytes: &[u8]) -> Option<&[u8]> {
    let mut ignored = 0;
    let bytes = frames_varint_decode(_bytes, &mut ignored)?;
    frames_varint_decode(bytes, &mut ignored)
}
```

## Pair `picoquic/frames.c:picoquic_queue_paths_blocked_frame`
C: `picoquic/frames.c:6128-6142 picoquic_queue_paths_blocked_frame`
Rust: `rs/fq/src/internal.rs:14225-14243 picoquic_queue_paths_blocked_frame`

### C body
```c
{
    /* Call to format will always succeed */
    int ret = 0;
    uint8_t frame_buffer[256];
    int is_pure_ack = 0;
    int more_data = 0;
    uint8_t* bytes_next = picoquic_format_paths_blocked_frame(
        frame_buffer, frame_buffer + sizeof(frame_buffer), cnx->max_path_id_remote, & more_data);
    size_t consumed = bytes_next - frame_buffer;
    ret = picoquic_queue_misc_frame(cnx, frame_buffer, consumed, is_pure_ack,
        picoquic_packet_context_application);
    return ret;
}
```

### Rust body
```rust
pub fn picoquic_queue_paths_blocked_frame(connection: &mut Connection) -> Result<(), crate::Error> {
    let mut frame_buffer = [0u8; 256];
    let mut more_data = 0;
    let frame_len = frame_buffer.len();
    let bytes_next = format_paths_blocked_frame(
        &mut frame_buffer,
        connection.max_path_id_remote,
        &mut more_data,
    )
    .ok_or(crate::Error::BufferTooSmall)?;
    let consumed = frame_len - bytes_next.len();
    encode_misc_frame(
        connection,
        frame_buffer[..consumed].to_vec(),
        false,
        PacketContext::Application,
    );
    Ok(())
}
```

## Pair `picoquic/frames.c:picoquic_format_observed_address_frame`
C: `picoquic/frames.c:6384-6405 picoquic_format_observed_address_frame`
Rust: `rs/fq/src/internal.rs:14511-14534 format_observed_address_frame`

### C body
```c
{
    size_t l_addr = ((ftype & 1) == 0) ? 4 : 16;
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, ftype)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, sequence_number)) != NULL &&
        bytes + l_addr < bytes_max) {
        memcpy(bytes, addr, l_addr);
        bytes = picoquic_frames_uint16_encode(bytes + l_addr, bytes_max, port);
    }
    else {
        bytes = NULL;
    }
    if (bytes == NULL) {
        *more_data = 1;
        bytes = bytes0;
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let addr_len = if (ftype & 1) == 0 { 4 } else { 16 };
    let mut off = 0;
    if addr.len() < addr_len
        || !encode_varint_at(bytes, &mut off, ftype)
        || !encode_varint_at(bytes, &mut off, sequence_number)
        || bytes.len() < off + addr_len + 2
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + addr_len].copy_from_slice(&addr[..addr_len]);
    off += addr_len;
    format_16(&mut bytes[off..off + 2], port);
    off += 2;
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_decode_frames`
C: `picoquic/frames.c:6701-6990 picoquic_decode_frames`
Rust: `rs/fq/src/internal.rs:14323-14477 decode_frames`

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

## Pair `picoquic/intformat.c:picoformat_16`
C: `picoquic/intformat.c:27-31 picoformat_16`
Rust: `rs/fq/src/utils.rs:952-955 picoformat_16`

### C body
```c
{
    bytes[0] = (uint8_t)(n16 >> 8);
    bytes[1] = (uint8_t)(n16);
}
```

### Rust body
```rust
pub fn picoformat_16(bytes: &mut [u8], n16: u16) {
    bytes[0] = (n16 >> 8) as u8;
    bytes[1] = n16 as u8;
}
```

## Pair `picoquic/intformat.c:picoquic_encode_varint_length`
C: `picoquic/intformat.c:60-83 picoquic_encode_varint_length`
Rust: `rs/fq/src/internal.rs:6461-6471 encode_varint_length`

### C body
```c
{
    if (n64 < 16384) {
        if (n64 < 64) {
            return 1u;
        } else {
            return 2u;
        }
    } else {
        if (n64 < 1073741824) {
            return 4u;
        } else {
            return 8u;
        }
    }
}
```

### Rust body
```rust
pub fn encode_varint_length(n64: u64) -> usize {
    if n64 < 64 {
        1
    } else if n64 < 16_384 {
        2
    } else if n64 < 1_073_741_824 {
        4
    } else {
        8
    }
}
```

## Pair `picoquic/intformat.c:picoquic_varint_decode`
C: `picoquic/intformat.c:136-162 picoquic_varint_decode`
Rust: `rs/fq/src/internal.rs:6396-6400 varint_decode`

### C body
```c
{
    size_t length = 0;
    
    if (max_bytes < 1) {
        *n64 = 0;
    } else {
        length = ((size_t)1) << ((bytes[0] & 0xC0) >> 6);

        if (length > max_bytes) {
            *n64 = 0;
            length = 0;
        }
        else {
            uint64_t v = *bytes++ & 0x3F;

            for (size_t i = 1; i < length; i++) {
                v <<= 8;
                v += *bytes++;
            }

            *n64 = v;
        }
    } 

    return length;
}
```

### Rust body
```rust
    if bytes.is_empty() {
        *n64 = 0;
        return 0;
    }
```

## Pair `picoquic/logger.c:textlog_app_message`
C: `picoquic/logger.c:2216-2221 textlog_app_message`
Rust: `rs/fq/src/logger.rs:438-441 app_message`

### C body
```c
{
    if (cnx->quic->F_log != NULL) {
        picoquic_txtlog_message_v(cnx->quic, &cnx->initial_cnxid, fmt, vargs);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().app_message(self, args);
        }
```

## Pair `picoquic/logger.c:textlog_packet_lost`
C: `picoquic/logger.c:2308-2326 textlog_packet_lost`
Rust: `rs/fq/src/logger.rs:697-709 packet_lost`

### C body
```c
{
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        FILE* F = cnx->quic->F_log;

        textlog_prefix_initial_cid64(F, picoquic_val64_connection_id(picoquic_get_logging_cnxid(cnx)));
        textlog_time(F, cnx, current_time, "T= ", ", ");
        fprintf(F, "Lost packet type %d, path %" PRIu64 ", number %" PRIu64 ", size %zu", ptype,
            path_x->unique_path_id, sequence_number, packet_size);
        if (dcid != NULL) {
            fprintf(F, ", DCID ");
            textlog_connection_id(F, dcid);
        }
        fprintf(F, ", reason: %s\n", trigger);
    }
}
```

### Rust body
```rust
        if !self.is_still_logging() {
            return;
        }
```

## Pair `picoquic/logger.c:textlog_close_connection`
C: `picoquic/logger.c:2370-2375 textlog_close_connection`
Rust: `rs/fq/src/logger.rs:828-831 close_connection`

### C body
```c
{
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(cnx);
#endif
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().close_connection(self);
        }
```

## Pair `picoquic/logwriter.c:picoquic_log_fixed_skip`
C: `picoquic/logwriter.c:37-40 picoquic_log_fixed_skip`
Rust: `rs/fq/src/binlog.rs:363-375 skip_fixed`

### C body
```c
{
    return bytes == NULL ? NULL : ((bytes += size) <= bytes_max ? bytes : NULL);
}
```

### Rust body
```rust
fn read_length(bytes: &[u8]) -> Option<(usize, &[u8])> {
    let mut n64 = 0u64;
    let rest = frames_varint_decode(bytes, &mut n64)?;
    let n = n64 as usize;
    if (n as u64) != n64 {
        return None;
    }
    Some((n, rest))
}
```

## Pair `picoquic/logwriter.c:picoquic_binlog_frame`
C: `picoquic/logwriter.c:64-73 picoquic_binlog_frame`
Rust: `rs/fq/src/binlog.rs:232-235 append_frame`

### C body
```c
{
    if (bytes != NULL && bytes_max != NULL) {
        size_t len = bytes_max - bytes;
        uint8_t varlen[8];
        size_t l_varlen = picoquic_varint_encode(varlen, 8, len);
        fwrite(varlen, 1, l_varlen, f);
        fwrite(bytes, 1, len, f);
    }
}
```

### Rust body
```rust
fn append_frame(out: &mut Vec<u8>, frame_bytes: &[u8]) {
    append_varint(out, frame_bytes.len() as u64);
    out.extend_from_slice(frame_bytes);
}
```

## Pair `picoquic/logwriter.c:picoquic_log_app_close_frame`
C: `picoquic/logwriter.c:227-239 picoquic_log_app_close_frame`
Rust: `rs/fq/src/binlog.rs:483-492 log_app_close_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    size_t length = 0;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1);
    bytes = picoquic_log_varint_skip(bytes, bytes_max);
    bytes = picoquic_log_length(bytes, bytes_max, &length);
    bytes = picoquic_log_fixed_skip(bytes, bytes_max, length);

    picoquic_binlog_frame(f, bytes_begin, bytes);
    return bytes;
}
```

### Rust body
```rust
fn log_app_close_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    let (length, rest) = read_length(bytes)?;
    bytes = skip_fixed(rest, length)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_path_frame`
C: `picoquic/logwriter.c:381-389 picoquic_log_path_frame`
Rust: `rs/fq/src/binlog.rs:537-542 log_path_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1 + 8);

    picoquic_binlog_frame(f, bytes_begin, bytes);
    return bytes;
}
```

### Rust body
```rust
fn log_path_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes = skip_fixed(bytes_in, 1 + 8)?;
    let consumed = bytes_in.len() - bytes.len();
    append_frame(out, &bytes_in[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_time_stamp_frame`
C: `picoquic/logwriter.c:437-447 picoquic_log_time_stamp_frame`
Rust: `rs/fq/src/binlog.rs:584-591 log_time_stamp_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;

    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* frame type as varint */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* time stamp as varint */

    picoquic_binlog_frame(f, bytes_begin, bytes);

    return bytes;
}
```

### Rust body
```rust
fn log_time_stamp_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```
