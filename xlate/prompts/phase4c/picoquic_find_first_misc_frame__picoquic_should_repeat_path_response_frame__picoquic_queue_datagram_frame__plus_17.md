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

## Pair `picoquic/frames.c:picoquic_find_first_misc_frame`
C: `picoquic/frames.c:4844-4854 picoquic_find_first_misc_frame`
Rust: `rs/fq/src/internal.rs:13774-13810 find_first_misc_frame`

### C body
```c
{
    picoquic_misc_frame_header_t* misc_frame = cnx->first_misc_frame;

    while (misc_frame != NULL && misc_frame->pc != pc) {
        misc_frame = misc_frame->next_misc_frame;
    }
    return misc_frame;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    // Drain all misc frames whose packet_context matches pc into bytes.
    let mut remaining = bytes;
    loop {
        let front_matches = connection
            .misc_frames
            .front()
            .map(|f| f.packet_context == pc)
            .unwrap_or(false);
        if !front_matches {
            break;
        }
        remaining = format_first_misc_or_dg_frame(
            remaining,
            more_data,
            is_pure_ack,
            &mut connection.misc_frames,
        )?;
    }
    Some(remaining)
}
```

## Pair `picoquic/frames.c:picoquic_should_repeat_path_response_frame`
C: `picoquic/frames.c:5065-5101 picoquic_should_repeat_path_response_frame`
Rust: `rs/fq/src/internal.rs:13349-13361 should_repeat_path_response_frame`

### C body
```c
{
    /* On the client side, challenge responses generally ought to be repeated in order to maximise
    * chances of handshake success. However, doing so on the server side may create a "blowback"
    * in case of attacks, if the initial challenge was set from an unreachable address, or if the
    * source address of the path challenge was forged.
    * If the node has sent several path responses, only the last one ought to be repeated.
    * If the path on which the response was sent is abandoned, there is no need to repeat
    * this frame. If the path is validated, then the response should always be repeated.
    */
    int should_repeat = 0;
    uint64_t response;
    if (picoquic_frames_uint64_decode(bytes + 1, bytes + bytes_max, &response) != NULL) {
        /* malformed frames will not be repeated */
        /* find the path on which the challenge was sent. */
        int path_index = -1;

        for (int i = 0; i < cnx->nb_paths; i++) {
            if (cnx->path[i]->first_tuple->challenge_response == response) {
                path_index = i;
                break;
            }
        }

        if (path_index >= 0 &&
            (cnx->path[path_index]->first_tuple->challenge_verified ||
                (cnx->client_mode && !cnx->path[path_index]->first_tuple->challenge_failed))) {
            should_repeat = 1;
        }
        else {
            should_repeat = 0;
        }
    }

    return should_repeat;
}
```

### Rust body
```rust
    pub fn should_repeat_path_response_frame(&self, bytes: &[u8], bytes_max: usize) -> bool {
        let max = bytes_max.min(bytes.len());
        if max < 9 {
            return false;
        }
        let response = parse_64(&bytes[1..9]);
        self.paths.iter().any(|path| {
            path.tuples.iter().any(|tuple| {
                tuple.challenge_response == response
                    && (tuple.challenge_verified || (self.client_mode && !tuple.challenge_failed))
            })
        })
    }
```

## Pair `picoquic/frames.c:picoquic_queue_datagram_frame`
C: `picoquic/frames.c:5307-5335 picoquic_queue_datagram_frame`
Rust: `rs/fq/src/lib.rs:3127-3135 queue_datagram_frame`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (length > PICOQUIC_DATAGRAM_QUEUE_CAUTIOUS_LENGTH) {
        if (length > cnx->local_parameters.max_datagram_frame_size ||
            length > cnx->remote_parameters.max_datagram_frame_size ||
            length + 21 + cnx->quic->local_cnxid_length > cnx->path[0]->send_mtu) {
            ret = PICOQUIC_ERROR_DATAGRAM_TOO_LONG;
        }
    }
    if (ret == 0) {
        size_t consumed = 0;
        uint8_t frame_buffer[PICOQUIC_MAX_PACKET_SIZE];
        int more_data = 0;
        int is_pure_ack = 1;
        uint8_t* bytes_next = picoquic_format_datagram_frame(frame_buffer, frame_buffer + sizeof(frame_buffer), &more_data, &is_pure_ack, length, src);

        if ((consumed = bytes_next - frame_buffer) > 0) {
            ret = picoquic_queue_misc_or_dg_frame(cnx, &cnx->first_datagram, &cnx->last_datagram,
                frame_buffer, consumed, 0, picoquic_packet_context_application);
        }
        else {
            ret = PICOQUIC_ERROR_FRAME_BUFFER_TOO_SMALL;
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn queue_datagram_frame(&mut self, bytes: &[u8]) -> Result<(), Error> {
        use crate::internal::MiscFrameHeader;
        self.datagrams.push_back(MiscFrameHeader {
            bytes: bytes.to_vec(),
            packet_context: PacketContext::Application,
            is_pure_ack: 0,
        });
        Ok(())
    }
```

## Pair `picoquic/frames.c:picoquic_format_ready_datagram_frame`
C: `picoquic/frames.c:5471-5524 picoquic_format_ready_datagram_frame`
Rust: `rs/fq/src/internal.rs:13949-13973 format_ready_datagram_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_datagram_l)) == NULL ||
        bytes + 16 > bytes_max){
        bytes = bytes0;
        *more_data = 1;
    }
    else {
        /* Compute the length */
        size_t allowed_space = bytes_max - bytes;
        picoquic_datagram_buffer_argument_t datagram_data_context;

        if (allowed_space > cnx->remote_parameters.max_datagram_frame_size) {
            allowed_space = cnx->remote_parameters.max_datagram_frame_size;
        }

        datagram_data_context.cnx = cnx;
        datagram_data_context.path_x = path_x;
        datagram_data_context.bytes0 = bytes0;
        datagram_data_context.bytes = bytes;
        datagram_data_context.bytes_max = bytes_max;
        datagram_data_context.allowed_space = allowed_space;
        datagram_data_context.after_data = bytes0;
        datagram_data_context.is_active = 0;
        datagram_data_context.is_old_api = 0;
        datagram_data_context.was_called = 0;

        if (cnx->callback_fn != NULL && (cnx->callback_fn)(cnx, (cnx->are_path_callbacks_enabled)?path_x->unique_path_id:0, (uint8_t*)&datagram_data_context, allowed_space,
            picoquic_callback_prepare_datagram, cnx->callback_ctx, NULL) != 0) {
            /* something went wrong */
            picoquic_log_app_message(cnx, "Prepare datagram returns error 0x%x", PICOQUIC_TRANSPORT_INTERNAL_ERROR);
            *ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0);
            bytes = bytes0; /* CHECK: SHOULD THIS BE NULL ? */
        }
        else {
            bytes = datagram_data_context.after_data;
            if (bytes > bytes0) {
                *is_pure_ack = 0;
            }

            if (datagram_data_context.is_old_api || !datagram_data_context.was_called) {
                *more_data |= cnx->is_datagram_ready;
            }
            else {
                *more_data |= datagram_data_context.is_active;
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    *ret = 0;
    if !connection.is_datagram_ready && !path_x.is_datagram_ready && connection.datagrams.is_empty()
    {
        return Some(bytes);
    }
    if connection.datagrams.is_empty() {
        connection.is_datagram_ready = false;
        path_x.is_datagram_ready = false;
        return Some(bytes);
    }
    let tail = format_first_datagram_frame(connection, bytes, 0, more_data, is_pure_ack)?;
    if connection.datagrams.is_empty() {
        connection.is_datagram_ready = false;
        path_x.is_datagram_ready = false;
    }
    Some(tail)
}
```

## Pair `picoquic/frames.c:picoquic_format_time_stamp_frame`
C: `picoquic/frames.c:5719-5731 picoquic_format_time_stamp_frame`
Rust: `rs/fq/src/internal.rs:14090-14108 format_time_stamp_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    uint64_t time_stamp = (current_time - cnx->start_time) >> cnx->local_parameters.ack_delay_exponent;

    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_time_stamp)) == NULL ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, time_stamp)) == NULL) {
        bytes = bytes0;
        *more_data = 1;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let delta = current_time
        .ticks()
        .saturating_sub(connection.start_time.ticks());
    let time_stamp = delta >> connection.local_parameters.ack_delay_exponent;
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, crate::frames::FrameType::TimeStamp as u64)
        || !encode_varint_at(bytes, &mut off, time_stamp)
    {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_queue_path_abandon_frame`
C: `picoquic/frames.c:5845-5861 picoquic_queue_path_abandon_frame`
Rust: `rs/fq/src/internal.rs:14299-14319 queue_path_abandon_frame`

### C body
```c
{
    int ret = 0;
    uint8_t buffer[512];
    uint8_t* end_bytes;
    int more_data = 0;
    end_bytes = picoquic_format_path_abandon_frame(buffer, buffer + sizeof(buffer), &more_data,
        unique_path_id, reason);
    if (end_bytes == NULL ||
        picoquic_queue_misc_frame(cnx, buffer, end_bytes - buffer, 0,
            picoquic_packet_context_application) != 0) {
        /* Could not format or could not queue. Internal error. */
        ret = -1;
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let mut frame = [0u8; 512];
        let mut more_data = 0;
        let rest = format_path_abandon_frame(&mut frame, &mut more_data, unique_path_id, reason)
            .ok_or(crate::Error::BufferTooSmall)?;
        let used = 512 - rest.len();
        if more_data != 0 {
            return Err(crate::Error::BufferTooSmall);
        }
        encode_misc_frame(
            self,
            frame[..used].to_vec(),
            false,
            PacketContext::Application,
        );
        Ok(())
    }
```

## Pair `picoquic/frames.c:picoquic_format_max_path_id_frame`
C: `picoquic/frames.c:5999-6010 picoquic_format_max_path_id_frame`
Rust: `rs/fq/src/internal.rs:13435-13448 format_max_path_id_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_max_path_id)) == NULL ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, max_path_id)) == NULL){
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
    if !encode_varint_at(bytes, &mut off, crate::frames::FrameType::MaxPathId as u64)
        || !encode_varint_at(bytes, &mut off, max_path_id)
    {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_format_path_cid_blocked_frame`
C: `picoquic/frames.c:6224-6237 picoquic_format_path_cid_blocked_frame`
Rust: `rs/fq/src/internal.rs:14246-14264 format_path_cid_blocked_frame`

### C body
```c
{
    /* This code assumes that the frame type is already skipped */
    uint8_t* bytes0 = bytes;
    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_path_cid_blocked)) == NULL ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, path_id)) == NULL ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, next_sequence_number)) == NULL) {
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
        crate::frames::FrameType::PathCidBlocked as u64,
    ) || !encode_varint_at(bytes, &mut off, path_id)
        || !encode_varint_at(bytes, &mut off, next_sequence_number)
    {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_prepare_observed_address_frame`
C: `picoquic/frames.c:6409-6465 picoquic_prepare_observed_address_frame`
Rust: `rs/fq/src/internal.rs:14536-14568 prepare_observed_address_frame`

### C body
```c
{
    if (!path_x->observed_addr_acked &&
        tuple->nb_observed_repeat < 4 &&
        tuple->peer_addr.ss_family != AF_UNSPEC) {
        int is_needed = 0;

        if (tuple->nb_observed_repeat == 0) {
            is_needed = 1;
            path_x->observed_sequence_sent = path_x->cnx->observed_number++;
        }
        else {
            uint64_t repeat_time = tuple->observed_time + path_x->retransmit_timer;

            if (repeat_time <= current_time) {
                is_needed = 1;
            }
            else if (*next_wake_time > repeat_time) {
                *next_wake_time = repeat_time;
            }
        }

        if (is_needed) {
            uint64_t ftype = 0;
            uint8_t* ip_addr = NULL;
            uint16_t port = 0;

            if (tuple->peer_addr.ss_family == AF_INET6) {
                struct sockaddr_in6* addr = (struct sockaddr_in6*)&tuple->peer_addr;
                ftype = picoquic_frame_type_observed_address_v6;
                ip_addr = (uint8_t*)&addr->sin6_addr;
                port = ntohs(addr->sin6_port);
            }
            else {
                struct sockaddr_in* addr = (struct sockaddr_in*)&tuple->peer_addr;
                ftype = picoquic_frame_type_observed_address_v4;
                ip_addr = (uint8_t*)&addr->sin_addr;
                port = ntohs(addr->sin_port);
            }

            uint8_t *bytes_next = picoquic_format_observed_address_frame(
                bytes, bytes_max, ftype, path_x->observed_sequence_sent,
                ip_addr, port, more_data);
            if (bytes_next > bytes) {
                *is_pure_ack = 0;
                bytes = bytes_next;
                tuple->nb_observed_repeat += 1;
                tuple->observed_time = current_time;
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let (ftype, addr_bytes) = match tuple.peer_addr.ip() {
        core::net::IpAddr::V4(v4) => (
            crate::frames::FrameType::ObservedAddressV4 as u64,
            v4.octets().to_vec(),
        ),
        core::net::IpAddr::V6(v6) => (
            crate::frames::FrameType::ObservedAddressV6 as u64,
            v6.octets().to_vec(),
        ),
    };
    tuple.observed_time = current_time;
    tuple.nb_observed_repeat += 1;
    format_observed_address_frame(
        bytes,
        ftype,
        tuple.nb_observed_repeat as u64,
        &addr_bytes,
        tuple.peer_addr.port(),
        more_data,
    )
    .inspect(|_| {
        *is_pure_ack = 0;
    })
}
```

## Pair `picoquic/frames.c:picoquic_skip_frame`
C: `picoquic/frames.c:7112-7289 picoquic_skip_frame`
Rust: `rs/fq/src/internal.rs:14578-14849 skip_frame`

### C body
```c
{
    const uint8_t *bytes_max = bytes + bytes_maxsize;
    uint8_t first_byte = bytes[0];

    *pure_ack = 1;

    if (PICOQUIC_IN_RANGE(first_byte, picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max)) {
        *pure_ack = 0;
        bytes = picoquic_skip_stream_frame(bytes, bytes_max);
    } else {
        switch (first_byte) {
        case picoquic_frame_type_ack:
            bytes = picoquic_skip_ack_frame(bytes, bytes_max);
            break;
        case picoquic_frame_type_ack_ecn:
            bytes = picoquic_skip_ack_ecn_frame(bytes, bytes_max);
            break;
        case picoquic_frame_type_padding:
            bytes = picoquic_skip_0len_frame(bytes, bytes_max);
            break;
        case picoquic_frame_type_reset_stream:
            bytes = picoquic_skip_reset_stream_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_connection_close: {
            bytes = picoquic_skip_connection_close_frame(bytes, bytes_max);
            *pure_ack = 1;
            break;
        }
        case picoquic_frame_type_application_close: {
            bytes = picoquic_skip_application_close_frame(bytes, bytes_max);
            *pure_ack = 1;
            break;
        }
        case picoquic_frame_type_max_data:
            bytes = picoquic_frames_varint_skip(bytes+1, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_max_stream_data:
            bytes = picoquic_skip_max_stream_data_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_max_streams_bidir:
        case picoquic_frame_type_max_streams_unidir:
            bytes = picoquic_frames_varint_skip(bytes+1, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_ping:
            bytes = picoquic_skip_0len_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_data_blocked:
            bytes = picoquic_frames_varint_skip(bytes+1, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_stream_data_blocked:
            bytes = picoquic_skip_stream_blocked_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_streams_blocked_bidir:
        case picoquic_frame_type_streams_blocked_unidir:
            bytes = picoquic_frames_varint_skip(bytes+1, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_new_connection_id:
            bytes = picoquic_skip_new_connection_id_frame(bytes, bytes_max, 0);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_stop_sending:
            bytes = picoquic_skip_stop_sending_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_path_challenge:
            bytes = picoquic_frames_fixed_skip(bytes+1, bytes_max, challenge_length);
            break;
        case picoquic_frame_type_path_response:
            bytes = picoquic_frames_fixed_skip(bytes+1, bytes_max, challenge_length);
            break;
        case picoquic_frame_type_crypto_hs:
            bytes = picoquic_skip_crypto_hs_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_new_token:
            bytes = picoquic_skip_new_token_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_retire_connection_id:
            bytes = picoquic_skip_retire_connection_id_frame(bytes, bytes_max, 0);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_handshake_done:
            bytes = bytes + 1;
            *pure_ack = 0;
            break;
        case picoquic_frame_type_datagram:
        case picoquic_frame_type_datagram_l:
            bytes = picoquic_skip_datagram_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_reset_stream_at:
            bytes = picoquic_skip_reset_stream_at_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        default: {
            uint64_t frame_id64;
            const uint8_t * bytes_before_type = bytes;
            if ((bytes = picoquic_frames_varint_decode(bytes, bytes_max, &frame_id64)) != NULL) {
                switch (frame_id64) {
                case picoquic_frame_type_ack_frequency:
                    bytes = picoquic_skip_ack_frequency_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_immediate_ack:
                    bytes = picoquic_skip_immediate_ack_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_time_stamp:
                    bytes = picoquic_skip_time_stamp_frame(bytes, bytes_max);
                    break;
                case picoquic_frame_type_path_ack:
                    bytes = picoquic_skip_ack_frame_maybe_ecn(bytes_before_type, bytes_max, 0, 1);
                    break;
                case picoquic_frame_type_path_ack_ecn:
                    bytes = picoquic_skip_ack_frame_maybe_ecn(bytes_before_type, bytes_max, 1, 1);
                    break;
                case picoquic_frame_type_path_abandon:
                    bytes = picoquic_skip_path_abandon_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_path_backup:
                case picoquic_frame_type_path_available:
                    bytes = picoquic_skip_path_available_or_backup_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_max_path_id:
                    bytes = picoquic_skip_max_path_id_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_paths_blocked:
                    bytes = picoquic_skip_paths_blocked_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_path_cid_blocked:
                    bytes = picoquic_skip_path_cid_blocked_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_bdp:
                    bytes = picoquic_skip_bdp_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_path_new_connection_id:
                    bytes = picoquic_skip_new_connection_id_frame(bytes_before_type, bytes_max, 1);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_path_retire_connection_id:
                    bytes = picoquic_skip_retire_connection_id_frame(bytes_before_type, bytes_max, 1);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_observed_address_v4:
                case picoquic_frame_type_observed_address_v6:
                    bytes = picoquic_skip_observed_address_frame(bytes, bytes_max, frame_id64);
                    *pure_ack = 0;
                    break;
                default:
                    /* Not implemented yet! */
                    bytes = NULL;
                }
            }
            break;
        }
        }
    }

    *consumed = (bytes != NULL) ? bytes_maxsize - (bytes_max - bytes) : bytes_maxsize;

    return bytes == NULL;
}
```

### Rust body
```rust
pub fn skip_frame(bytes: &[u8], bytes_max: usize, consumed: &mut usize, pure_ack: &mut i32) -> i32 {
    let max = bytes_max.min(bytes.len());
    *consumed = 0;
    *pure_ack = 1;
    if max == 0 {
        return -1;
    }

    let mut frame_type = 0;
    let mut tail = match frames_varint_decode(&bytes[..max], &mut frame_type) {
        Some(tail) => tail,
        None => return -1,
    };
    let rest = match frame_type {
        x if x == crate::frames::FrameType::Padding as u64 => {
            *pure_ack = 1;
            let mut off = max - tail.len();
            while off < max && bytes[off] == 0 {
                off += 1;
            }
            &bytes[off..max]
        }
        x if x == crate::frames::FrameType::Ping as u64
            || x == crate::frames::FrameType::HandshakeDone as u64
            || x == crate::frames::FrameType::ImmediateAck as u64 =>
        {
            *pure_ack = 0;
            tail
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
            let mut header_len = 0;
            if parse_ack_header(
                &bytes[..max],
                max,
                &mut num_block,
                &mut path_id,
                &mut largest,
                &mut ack_delay,
                &mut header_len,
                0,
            ) != 0
            {
                return -1;
            }
            tail = &bytes[header_len..max];
            let Some(mut t) = skip_n_varints(tail, 1 + (num_block as usize).saturating_mul(2))
            else {
                return -1;
            };
            if frame_type == crate::frames::FrameType::AckEcn as u64
                || frame_type == crate::frames::FrameType::PathAckEcn as u64
            {
                t = match skip_n_varints(t, 3) {
                    Some(t) => t,
                    None => return -1,
                };
            }
            *pure_ack = 1;
            t
        }
        x if x >= crate::frames::FrameType::StreamRangeMin as u64
            && x <= crate::frames::FrameType::StreamRangeMax as u64 =>
        {
            let mut stream_id = 0;
            let mut offset = 0;
            let mut data_length = 0;
            let mut fin = 0;
            let mut header_len = 0;
            if parse_stream_header(
                &bytes[..max],
                max,
                &mut stream_id,
                &mut offset,
                &mut data_length,
                &mut fin,
                &mut header_len,
            ) != 0
                || header_len.saturating_add(data_length) > max
            {
                return -1;
            }
            *pure_ack = 0;
            &bytes[header_len + data_length..max]
        }
        x if x == crate::frames::FrameType::CryptoHs as u64 => {
            let mut ignored = 0;
            tail = match frames_varint_decode(tail, &mut ignored) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            tail = match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            };
            *pure_ack = 0;
            tail
        }
        x if x == crate::frames::FrameType::NewToken as u64 => {
            let mut length = 0;
            tail = match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            };
            *pure_ack = 0;
            tail
        }
        x if x == crate::frames::FrameType::Datagram as u64 => {
            *pure_ack = 0;
            &[]
        }
        x if x == crate::frames::FrameType::DatagramL as u64 => {
            let mut length = 0;
            *pure_ack = 0;
            match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            }
        }
        x if x == crate::frames::FrameType::PathChallenge as u64
            || x == crate::frames::FrameType::PathResponse as u64 =>
        {
            if tail.len() < 8 {
                return -1;
            }
            &tail[8..]
        }
        x if x == crate::frames::FrameType::ResetStream as u64 => {
            *pure_ack = 0;
            match skip_n_varints(tail, 3) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::ResetStreamAt as u64 => {
            *pure_ack = 0;
            match skip_n_varints(tail, 4) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::StopSending as u64
            || x == crate::frames::FrameType::MaxStreamData as u64
            || x == crate::frames::FrameType::StreamDataBlocked as u64 =>
        {
            *pure_ack = 0;
            match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::MaxData as u64
            || x == crate::frames::FrameType::MaxStreamsBidir as u64
            || x == crate::frames::FrameType::MaxStreamsUnidir as u64
            || x == crate::frames::FrameType::DataBlocked as u64
            || x == crate::frames::FrameType::StreamsBlockedBidir as u64
            || x == crate::frames::FrameType::StreamsBlockedUnidir as u64
            || x == crate::frames::FrameType::RetireConnectionId as u64
            || x == crate::frames::FrameType::MaxPathId as u64
            || x == crate::frames::FrameType::PathsBlocked as u64 =>
        {
            *pure_ack = 0;
            match skip_n_varints(tail, 1) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::TimeStamp as u64 => match skip_n_varints(tail, 1) {
            Some(t) => t,
            None => return -1,
        },
        x if x == crate::frames::FrameType::PathRetireConnectionId as u64
            || x == crate::frames::FrameType::PathAbandon as u64
            || x == crate::frames::FrameType::PathAvailable as u64
            || x == crate::frames::FrameType::PathBackup as u64
            || x == crate::frames::FrameType::PathCidBlocked as u64 =>
        {
            *pure_ack = 0;
            match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::NewConnectionId as u64
            || x == crate::frames::FrameType::PathNewConnectionId as u64 =>
        {
            if frame_type == crate::frames::FrameType::PathNewConnectionId as u64 {
                tail = match skip_n_varints(tail, 1) {
                    Some(t) => t,
                    None => return -1,
                };
            }
            *pure_ack = 0;
            tail = match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            };
            let (&cid_len, t) = match tail.split_first() {
                Some(v) => v,
                None => return -1,
            };
            let skip = cid_len as usize + RESET_SECRET_SIZE;
            if t.len() < skip {
                return -1;
            }
            &t[skip..]
        }
        x if x == crate::frames::FrameType::ConnectionClose as u64 => {
            tail = match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            }
        }
        x if x == crate::frames::FrameType::ApplicationClose as u64 => {
            tail = match skip_n_varints(tail, 1) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            }
        }
        x if x == crate::frames::FrameType::AckFrequency as u64 => {
            *pure_ack = 0;
            match skip_n_varints(tail, 4) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::Bdp as u64 => {
            *pure_ack = 0;
            let mut t = match skip_n_varints(tail, 3) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            t = match frames_varint_decode(t, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            };
            t
        }
        x if x == crate::frames::FrameType::ObservedAddressV4 as u64
            || x == crate::frames::FrameType::ObservedAddressV6 as u64 =>
        {
            *pure_ack = 0;
            match parse_observed_address_frame(tail, frame_type) {
                Some((_, rest)) => rest,
                None => return -1,
            }
        }
        _ => return -1,
    };

    *consumed = max - rest.len();
    0
}
```

## Pair `picoquic/intformat.c:picoformat_24`
C: `picoquic/intformat.c:33-38 picoformat_24`
Rust: `rs/fq/src/utils.rs:959-963 picoformat_24`

### C body
```c
{
    bytes[0] = (uint8_t)(n24 >> 16);
    bytes[1] = (uint8_t)(n24 >> 8);
    bytes[2] = (uint8_t)(n24);
}
```

### Rust body
```rust
pub fn picoformat_24(bytes: &mut [u8], n24: u32) {
    bytes[0] = (n24 >> 16) as u8;
    bytes[1] = (n24 >> 8) as u8;
    bytes[2] = n24 as u8;
}
```

## Pair `picoquic/intformat.c:picoquic_decode_varint_length`
C: `picoquic/intformat.c:85-88 picoquic_decode_varint_length`
Rust: `rs/fq/src/internal.rs:6481-6526 decode_varint_length`

### C body
```c
{
    return ((size_t)1u) << ((byte & 0xC0) >> 6);
}
```

### Rust body
```rust
pub fn parse_long_packet_type(flags: u8, version_index: i32) -> PacketType {
    let type_bits = (flags >> 4) & 3;
    let version = match version_index {
        0 => Version::V1,
        1 => Version::V2,
        2 => Version::V2Draft,
        3 => Version::PostIesg,
        4 => Version::TwentyFirstInterop,
        5 => Version::TwentiethInterop,
        6 => Version::TwentiethPreInterop,
        7 => Version::NineteenthInterop,
        8 => Version::NineteenthBisInterop,
        9 => Version::EighteenthInterop,
        10 => Version::SeventeenthInterop,
        11 => Version::InternalTest2,
        12 => Version::InternalTest1,
        _ => return PacketType::Error,
    };

    match version.parameters().packet_type_version {
        0x0000_0001 => match type_bits {
            0 => PacketType::Initial,
            1 => PacketType::ZeroRttProtected,
            2 => PacketType::Handshake,
            3 => PacketType::Retry,
            _ => PacketType::Error,
        },
        0x6b33_43cf => match type_bits {
            1 => PacketType::Initial,
            2 => PacketType::ZeroRttProtected,
            3 => PacketType::Handshake,
            0 => PacketType::Retry,
            _ => PacketType::Error,
        },
        _ => PacketType::Error,
    }
}
```

## Pair `picoquic/intformat.c:picoquic_varint_skip`
C: `picoquic/intformat.c:164-167 picoquic_varint_skip`
Rust: `rs/fq/src/internal.rs:6452-6455 varint_skip`

### C body
```c
{
    return picoquic_decode_varint_length(bytes[0]);
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return 0;
    }
```

## Pair `picoquic/logger.c:textlog_dropped_packet`
C: `picoquic/logger.c:2261-2284 textlog_dropped_packet`
Rust: `rs/fq/src/logger.rs:566-576 dropped_packet`

### C body
```c
{
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        FILE* F = cnx->quic->F_log;

        if (ret == PICOQUIC_ERROR_PADDING_PACKET) {
            uint64_t log_cnxid64 = 0;

            if (cnx == NULL) {
                log_cnxid64 = picoquic_val64_connection_id(ph->srce_cnx_id);
            }
            else {
                log_cnxid64 = picoquic_val64_connection_id(picoquic_get_logging_cnxid(cnx));
            }
            textlog_prefix_initial_cid64(F, log_cnxid64);
            fprintf(F, "Dropped padding packet, size: %zu.\n", packet_size);
            fprintf(F, "\n");
        }
        else {
            textlog_decrypted_segment(cnx->quic->F_log, 1, cnx, 1, ph, NULL, packet_size, ret);
        }
    }
}
```

### Rust body
```rust
        if !self.is_still_logging() {
            return;
        }
```

## Pair `picoquic/logger.c:textlog_negotiated_alpn`
C: `picoquic/logger.c:2328-2343 textlog_negotiated_alpn`
Rust: `rs/fq/src/logger.rs:755-759 negotiated_alpn`

### C body
```c
{
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(sni);
    UNREFERENCED_PARAMETER(sni_len);
    UNREFERENCED_PARAMETER(alpn);
    UNREFERENCED_PARAMETER(alpn_len);
#endif
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        /* TODO: alpn */
        picoquic_textlog_negotiated_alpn(cnx->quic->F_log, cnx, 
            (is_local) ? 0 : 1, 1, alpn_list, alpn_count);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }
```

## Pair `picoquic/logger.c:textlog_cc_dump`
C: `picoquic/logger.c:2377-2380 textlog_cc_dump`
Rust: `rs/fq/src/logger.rs:846-883 cc_dump`

### C body
```c
{
    textlog_congestion_state(cnx->quic->F_log, cnx, path_x, current_time);
}
```

### Rust body
```rust
    fn cc_dump(&mut self, current_time: Instant) {
        let mut paths = core::mem::take(&mut self.paths);

        if let Some(mut memlog) = self.memlog_call_back.take() {
            if let Some(path0) = paths.first_mut() {
                memlog.callback(self, path0, 0, current_time);
            }
            self.memlog_call_back = Some(memlog);
        }

        if self.is_still_logging() {
            for path_x in &mut paths {
                if !path_x.is_cc_data_updated {
                    continue;
                }

                if let Some(text) = logger_ref(&self.text_log_fns) {
                    text.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.f_binlog.is_some()
                    && let Some(bin) = logger_ref(&self.bin_log_fns)
                {
                    bin.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.qlog_ctx.is_some()
                    && let Some(qlog) = logger_ref(&self.qlog_fns)
                {
                    qlog.borrow_mut().cc_dump(self, path_x, current_time);
                }

                path_x.is_cc_data_updated = false;
            }
        }

        self.paths = paths;
    }
```

## Pair `picoquic/logwriter.c:picoquic_log_varint_skip`
C: `picoquic/logwriter.c:42-45 picoquic_log_varint_skip`
Rust: `rs/fq/src/internal.rs:6438-6441 frames_varint_skip`

### C body
```c
{
    return bytes == NULL ? NULL : (bytes < bytes_max ? picoquic_log_fixed_skip(bytes, bytes_max, VARINT_LEN_T(bytes, size_t)) : NULL);
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## Pair `picoquic/logwriter.c:picoquic_log_stream_frame`
C: `picoquic/logwriter.c:75-136 picoquic_log_stream_frame`
Rust: `rs/fq/src/binlog.rs:377-422 log_stream_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    uint8_t ftype = bytes[0];
    size_t length = 0;
    uint8_t log_buffer[256];
    int has_length = 0;
    size_t extra_bytes = 8;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1); /* type */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* stream */

    if ((ftype & 4) != 0) {
        bytes = picoquic_log_varint_skip(bytes, bytes_max); /* offset */
    }

    if (bytes != NULL) {
        if ((ftype & 2) != 0) {
            bytes = picoquic_log_length(bytes, bytes_max, &length); /* length */
            has_length = 1;
        }
        else {
            length = bytes_max - bytes;
        }
    }

    if (bytes != NULL) {
        if (length < extra_bytes) {
            /* Add up to 8 bytes of content that can be documented in the qlog */
            extra_bytes = length;
        }
        if (has_length) {
            picoquic_binlog_frame(f, bytes_begin, bytes + extra_bytes);
        }
        else {
            uint8_t* log_next = log_buffer;
            size_t l_head = bytes - bytes_begin;

            memcpy(log_buffer, bytes_begin, l_head);
            log_next += l_head;
            if ((log_next = picoquic_frames_varint_encode(log_next, log_buffer + 256, length)) != NULL) {
                memcpy(log_next, bytes, extra_bytes);
                log_next += extra_bytes;
                picoquic_binlog_frame(f, log_buffer, log_next);
            }
            else {
                picoquic_binlog_frame(f, log_buffer, log_buffer + l_head);
            }
        }

        bytes = picoquic_log_fixed_skip(bytes, bytes_max, length);
    }
    else {
        /* Cautiously log the beginning of the erroneous frame */
        length = bytes_max - bytes_begin;
        if (length > 26) {
            length = 26;
        }
        picoquic_binlog_frame(f, bytes_begin, bytes_begin + length);
    }
    return bytes;
}
```

### Rust body
```rust
fn log_stream_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    if bytes_in.is_empty() {
        return None;
    }
    let ftype = bytes_in[0];
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    if (ftype & 4) != 0 {
        bytes = frames_varint_skip(bytes)?;
    }

    let head_len = bytes_begin.len() - bytes.len();
    let has_length = (ftype & 2) != 0;
    let length: usize;
    if has_length {
        let (l, rest) = read_length(bytes)?;
        length = l;
        bytes = rest;
    } else {
        length = bytes.len();
    }

    let mut extra_bytes: usize = 8;
    if length < extra_bytes {
        extra_bytes = length;
    }

    if has_length {
        let copy_end = head_len.saturating_add(extra_bytes).min(bytes_begin.len());
        append_frame(out, &bytes_begin[..copy_end]);
    } else {
        let mut log_buffer = Vec::with_capacity(head_len + 8 + extra_bytes);
        log_buffer.extend_from_slice(&bytes_begin[..head_len]);
        let mut len_buf = [0u8; 8];
        let n = varint_encode(&mut len_buf, length as u64);
        log_buffer.extend_from_slice(&len_buf[..n]);
        let payload_start = head_len;
        let avail = bytes_begin.len().saturating_sub(payload_start);
        let take = extra_bytes.min(avail);
        log_buffer.extend_from_slice(&bytes_begin[payload_start..payload_start + take]);
        append_frame(out, &log_buffer);
    }

    skip_fixed(bytes, length)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_new_connection_id_frame`
C: `picoquic/logwriter.c:309-324 picoquic_log_new_connection_id_frame`
Rust: `rs/fq/src/binlog.rs:494-508 log_new_connection_id_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1);
    bytes = picoquic_log_varint_skip(bytes, bytes_max);
    bytes = picoquic_log_varint_skip(bytes, bytes_max);
    if (bytes != NULL) {
        bytes = picoquic_log_fixed_skip(bytes, bytes_max, ((size_t)1) + bytes[0]);
    }

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, PICOQUIC_RESET_SECRET_SIZE);

    picoquic_binlog_frame(f, bytes_begin, bytes);
    return bytes;
}
```

### Rust body
```rust
fn log_new_connection_id_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    if bytes.is_empty() {
        return None;
    }
    let cid_len = bytes[0] as usize;
    bytes = skip_fixed(bytes, 1 + cid_len)?;
    bytes = skip_fixed(bytes, crate::RESET_SECRET_SIZE)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_crypto_hs_frame`
C: `picoquic/logwriter.c:391-404 picoquic_log_crypto_hs_frame`
Rust: `rs/fq/src/binlog.rs:544-554 log_crypto_hs_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    size_t length = 0;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1);
    bytes = picoquic_log_varint_skip(bytes, bytes_max);
    bytes = picoquic_log_length(bytes, bytes_max, &length);

    picoquic_binlog_frame(f, bytes_begin, bytes);

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, length);
    return bytes;
}
```

### Rust body
```rust
fn log_crypto_hs_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    let (length, rest) = read_length(bytes)?;
    bytes = rest;
    let header_len = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..header_len]);
    bytes = skip_fixed(bytes, length)?;
    Some(bytes)
}
```
