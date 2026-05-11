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

## Pair `picoquic/frames.c:picoquic_format_path_challenge_frame`
C: `picoquic/frames.c:4886-4899 picoquic_format_path_challenge_frame`
Rust: `rs/fq/src/internal.rs:13316-13330 format_path_challenge_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_path_challenge)) != NULL &&
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
    bytes[0] = crate::frames::FrameType::PathChallenge as u8;
    format_64(&mut bytes[1..9], challenge);
    *is_pure_ack = 0;
    Some(&mut bytes[9..])
}
```

## Pair `picoquic/frames.c:picoquic_decode_datagram_frame_header`
C: `picoquic/frames.c:5229-5245 picoquic_decode_datagram_frame_header`
Rust: `rs/fq/src/internal.rs:13975-13991 decode_datagram_frame_header`

### C body
```c
{
    if (bytes != NULL) {
        *frame_id = *bytes++;
        if ((*frame_id) & 1) {
            if ((bytes = (uint8_t *)picoquic_frames_varint_decode(bytes, bytes_max, length)) != NULL &&
                (bytes + *length) > bytes_max) {
                bytes = NULL;
            }
        }
        else {
            *length = bytes_max - bytes;
        }
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    let (&first, mut tail) = bytes.split_first()?;
    *frame_id = first;
    if (first & 1) != 0 {
        tail = frames_varint_decode(tail, length)?;
        if (*length as usize) > tail.len() {
            return None;
        }
    } else {
        *length = tail.len() as u64;
    }
    Some(tail)
}
```

## Pair `picoquic/frames.c:picoquic_provide_datagram_buffer_ex`
C: `picoquic/frames.c:5408-5456 picoquic_provide_datagram_buffer_ex`
Rust: `rs/fq/src/lib.rs:4418-4424 provide_datagram_buffer_ex`

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

## Pair `picoquic/frames.c:picoquic_format_ack_frequency_frame`
C: `picoquic/frames.c:5594-5640 picoquic_format_ack_frequency_frame`
Rust: `rs/fq/src/internal.rs:14010-14072 format_ack_frequency_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    uint64_t seq = cnx->ack_frequency_sequence_local + 1;
    uint64_t ack_gap;
    uint64_t ack_delay_max;
    uint64_t reordering_threshold = (cnx->ack_ignore_order_local) ? 0 : 1;

    /* Compute the desired value of the ack frequency*/
    picoquic_compute_ack_gap_and_delay(cnx, cnx->path[0]->rtt_min, cnx->remote_parameters.min_ack_delay,
        cnx->path[0]->bandwidth_estimate, &ack_gap, &ack_delay_max);
    
    if (ack_gap <= cnx->ack_gap_local &&
        ack_delay_max >= (7*cnx->ack_frequency_delay_local)/8 &&
        ack_delay_max <= (9* cnx->ack_frequency_delay_local) / 8) {
        cnx->is_ack_frequency_updated = 0;
    }
    else {
        if (ack_gap < cnx->ack_gap_local) {
            ack_gap = cnx->ack_gap_local;
        }
        if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_ack_frequency)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, seq)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_gap)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_delay_max)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, reordering_threshold)) != NULL) {
            cnx->ack_frequency_sequence_local = seq;
            cnx->ack_gap_local = ack_gap;
            cnx->ack_frequency_delay_local = ack_delay_max;
            cnx->is_ack_frequency_updated = 0;
            if (ack_gap > cnx->max_ack_gap_local) {
                cnx->max_ack_gap_local = ack_gap;
            }
            if (ack_delay_max < cnx->min_ack_delay_local) {
                cnx->min_ack_delay_local = ack_delay_max;
            }
            if (ack_delay_max > cnx->max_ack_delay_local) {
                cnx->max_ack_delay_local = ack_delay_max;
            }
        }
        else {
            bytes = bytes0;
            *more_data = 1;
        }
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let seq = connection.ack_frequency_sequence_local.wrapping_add(1);
    let mut ack_gap = 0;
    let mut ack_delay_max = 0;
    let (rtt, rate) = connection
        .paths
        .first()
        .map(|p| (p.rtt_min, p.bandwidth_estimate))
        .unwrap_or((INITIAL_RTT, 0));
    connection.compute_ack_gap_and_delay(
        rtt,
        connection.remote_parameters.min_ack_delay.ticks(),
        rate,
        &mut ack_gap,
        &mut ack_delay_max,
    );

    if ack_gap <= connection.ack_gap_local
        && ack_delay_max >= (7 * connection.ack_frequency_delay_local.ticks()) / 8
        && ack_delay_max <= (9 * connection.ack_frequency_delay_local.ticks()) / 8
    {
        connection.is_ack_frequency_updated = false;
        return Some(bytes);
    }

    if ack_gap < connection.ack_gap_local {
        ack_gap = connection.ack_gap_local;
    }
    let reordering_threshold = if connection.ack_ignore_order_local {
        0
    } else {
        1
    };
    let mut off = 0;
    for value in [
        crate::frames::FrameType::AckFrequency as u64,
        seq,
        ack_gap,
        ack_delay_max,
        reordering_threshold,
    ] {
        if !encode_varint_at(bytes, &mut off, value) {
            *more_data = 1;
            return Some(bytes);
        }
    }
    connection.ack_frequency_sequence_local = seq;
    connection.ack_gap_local = ack_gap;
    connection.ack_frequency_delay_local = Duration::from_ticks(ack_delay_max);
    connection.is_ack_frequency_updated = false;
    connection.max_ack_gap_local = connection.max_ack_gap_local.max(ack_gap);
    connection.min_ack_delay_local = connection
        .min_ack_delay_local
        .min(Duration::from_ticks(ack_delay_max));
    connection.max_ack_delay_local = connection
        .max_ack_delay_local
        .max(Duration::from_ticks(ack_delay_max));
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_skip_path_abandon_frame`
C: `picoquic/frames.c:5743-5750 picoquic_skip_path_abandon_frame`
Rust: `rs/fq/src/internal.rs:14851-14855 skip_path_abandon_frame`

### C body
```c
{
    /* This code assumes that the frame type is already skipped */
    if ((bytes = picoquic_frames_varint_skip(bytes, bytes_max)) != NULL) {
        bytes = picoquic_frames_varint_skip(bytes, bytes_max);
    }
    return bytes;
}
```

### Rust body
```rust
pub fn skip_path_abandon_frame(_bytes: &[u8]) -> Option<&[u8]> {
    let mut ignored = 0;
    let bytes = frames_varint_decode(_bytes, &mut ignored)?;
    frames_varint_decode(bytes, &mut ignored)
}
```

## Pair `picoquic/frames.c:picoquic_queue_path_available_or_backup_frame`
C: `picoquic/frames.c:5880-5910 picoquic_queue_path_available_or_backup_frame`
Rust: `rs/fq/src/internal.rs:14893-14929 queue_path_available_or_backup_frame`

### C body
```c
{
    int ret = 0;

    if (path_x->first_tuple->p_remote_cnxid == NULL) {
        ret = -1;
    }
    else {
        /* Buffer sized so the call to format always succeeds */
        uint8_t frame_buffer[256];
        uint64_t frame_type = (status == picoquic_path_status_available) ?
            picoquic_frame_type_path_available : picoquic_frame_type_path_backup;
        uint64_t sequence = cnx->status_sequence_to_send_next++;
        uint64_t path_id = (cnx->is_multipath_enabled)?
            path_x->unique_path_id :
            path_x->first_tuple->p_remote_cnxid->sequence;
        int is_pure_ack = 0;
        int more_data = 0;
        uint8_t* bytes_next = picoquic_format_path_available_or_backup_frame(
            frame_buffer, frame_buffer + sizeof(frame_buffer), frame_type, path_id, sequence, &more_data);
        size_t consumed = bytes_next - frame_buffer;
        ret = picoquic_queue_misc_frame(cnx, frame_buffer, consumed, is_pure_ack,
                picoquic_packet_context_application);
        if (ret == 0) {
            path_x->status_sequence_sent_last = sequence;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let frame_type = if status == PathStatus::Available {
            crate::frames::FrameType::PathAvailable as u64
        } else {
            crate::frames::FrameType::PathBackup as u64
        };
        let sequence = self.status_sequence_to_send_next;
        self.status_sequence_to_send_next = self.status_sequence_to_send_next.saturating_add(1);
        let path_id = path_x.unique_path_id;
        let mut frame = [0u8; 256];
        let frame_len = frame.len();
        let mut more_data = 0;
        let tail = format_path_available_or_backup_frame(
            &mut frame,
            frame_type,
            path_id,
            sequence,
            &mut more_data,
        )
        .ok_or(crate::Error::BufferTooSmall)?;
        if more_data != 0 {
            return Err(crate::Error::BufferTooSmall);
        }
        let used = frame_len - tail.len();
        path_x.status_sequence_sent_last = sequence;
        encode_misc_frame(
            self,
            frame[..used].to_vec(),
            false,
            PacketContext::Application,
        );
        Ok(())
    }
```

## Pair `picoquic/frames.c:picoquic_format_paths_blocked_frame`
C: `picoquic/frames.c:6114-6126 picoquic_format_paths_blocked_frame`
Rust: `rs/fq/src/internal.rs:14204-14220 format_paths_blocked_frame`

### C body
```c
{
    /* This code assumes that the frame type is already skipped */
    uint8_t* bytes0 = bytes;
    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_paths_blocked)) == NULL ||
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
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::PathsBlocked as u64,
    ) || !encode_varint_at(bytes, &mut off, max_path_id)
    {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_queue_path_cid_blocked_frame`
C: `picoquic/frames.c:6257-6276 picoquic_queue_path_cid_blocked_frame`
Rust: `rs/fq/src/internal.rs:14270-14294 picoquic_queue_path_cid_blocked_frame`

### C body
```c
{
    /* Call to format will always succeed */
    int ret = 0;
    uint8_t frame_buffer[256];
    int is_pure_ack = 0;
    int more_data = 0;
    uint64_t next_sequence_number = picoquic_path_cid_next_sequence_number(path_x);
    uint8_t* bytes_next = picoquic_format_path_cid_blocked_frame(
        frame_buffer, frame_buffer + sizeof(frame_buffer), path_x->unique_path_id, 
        next_sequence_number, &more_data);
    size_t consumed = bytes_next - frame_buffer;
    ret = picoquic_queue_misc_frame(path_x->cnx, frame_buffer, consumed, is_pure_ack,
        picoquic_packet_context_application);
    if (ret == 0) {
        path_x->sending_path_cid_blocked_frame = 1;
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<(), crate::Error> {
    let mut frame_buffer = [0u8; 256];
    let mut more_data = 0;
    let next_sequence_number = connection.path_cid_next_sequence_number(path_x);
    let frame_len = frame_buffer.len();
    let bytes_next = format_path_cid_blocked_frame(
        &mut frame_buffer,
        path_x.unique_path_id,
        next_sequence_number,
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
    path_x.sending_path_cid_blocked_frame = true;
    Ok(())
}
```

## Pair `picoquic/frames.c:picoquic_format_bdp_frame`
C: `picoquic/frames.c:6637-6693 picoquic_format_bdp_frame`
Rust: `rs/fq/src/internal.rs:14119-14163 format_bdp_frame`

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

## Pair `picoquic/frames.c:picoquic_decode_closing_frames`
C: `picoquic/frames.c:7330-7353 picoquic_decode_closing_frames`
Rust: `rs/fq/src/internal.rs:14943-14973 decode_closing_frames`

### C body
```c
{
    int ret = 0;
    size_t byte_index = 0;

    *closing_received = 0;
    while (ret == 0 && byte_index < bytes_max) {
        uint8_t first_byte = bytes[byte_index];

        if (first_byte == picoquic_frame_type_connection_close || first_byte == picoquic_frame_type_application_close) {
            *closing_received = 1;
            break;
        } else {
            size_t consumed = 0;
            int pure_ack = 0;

            ret = picoquic_skip_frame(bytes + byte_index,
                bytes_max - byte_index, &consumed, &pure_ack);
            byte_index += consumed;
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let max = bytes_max.min(bytes.len());
    let mut byte_index = 0;
    *closing_received = 0;
    while byte_index < max {
        let first_byte = bytes[byte_index];
        if first_byte == crate::frames::FrameType::ConnectionClose as u8
            || first_byte == crate::frames::FrameType::ApplicationClose as u8
        {
            *closing_received = 1;
            break;
        }
        let mut consumed = 0;
        let mut pure_ack = 0;
        let ret = skip_frame(
            &bytes[byte_index..max],
            max - byte_index,
            &mut consumed,
            &mut pure_ack,
        );
        if ret != 0 || consumed == 0 {
            return ret;
        }
        byte_index += consumed;
    }
    0
}
```

## Pair `picoquic/intformat.c:picoformat_64`
C: `picoquic/intformat.c:48-58 picoformat_64`
Rust: `rs/fq/src/utils.rs:976-985 picoformat_64`

### C body
```c
{
    bytes[0] = (uint8_t)(n64 >> 56);
    bytes[1] = (uint8_t)(n64 >> 48);
    bytes[2] = (uint8_t)(n64 >> 40);
    bytes[3] = (uint8_t)(n64 >> 32);
    bytes[4] = (uint8_t)(n64 >> 24);
    bytes[5] = (uint8_t)(n64 >> 16);
    bytes[6] = (uint8_t)(n64 >> 8);
    bytes[7] = (uint8_t)(n64);
}
```

### Rust body
```rust
pub fn picoformat_64(bytes: &mut [u8], n64: u64) {
    bytes[0] = (n64 >> 56) as u8;
    bytes[1] = (n64 >> 48) as u8;
    bytes[2] = (n64 >> 40) as u8;
    bytes[3] = (n64 >> 32) as u8;
    bytes[4] = (n64 >> 24) as u8;
    bytes[5] = (n64 >> 16) as u8;
    bytes[6] = (n64 >> 8) as u8;
    bytes[7] = n64 as u8;
}
```

## Pair `picoquic/intformat.c:picoquic_varint_encode_16`
C: `picoquic/intformat.c:128-134 picoquic_varint_encode_16`
Rust: `rs/fq/src/internal.rs:6388-6391 varint_encode_16`

### C body
```c
{
    uint8_t* x = bytes;
    
    *x++ = (uint8_t)(((n16 >> 8) | 0x40)&0x7F);
    *x++ = (uint8_t)(n16);
}
```

### Rust body
```rust
pub fn varint_encode_16(bytes: &mut [u8], n16: u16) {
    bytes[0] = (((n16 >> 8) | 0x40) & 0x7F) as u8;
    bytes[1] = n16 as u8;
}
```

## Pair `picoquic/logger.c:picoquic_textlog_transport_extension_content`
C: `picoquic/logger.c:1852-1947 picoquic_textlog_transport_extension_content`
Rust: `rs/fq/src/tests/util.rs:4199-4301 textlog_transport_extension_content`

### C body
```c
{
    int ret = 0;
    size_t byte_index = 0;

    if (bytes_max < 256)
    {
        if (ret == 0)
        {
            size_t extensions_size = bytes_max;
            size_t extensions_end;

            extensions_end = byte_index + extensions_size;

            if (log_cnxid != 0) {
                textlog_prefix_initial_cid64(F, cnx_id_64);
            }
            fprintf(F, "    Extension list (%d bytes):\n",
                (uint32_t)extensions_size);
            while (ret == 0 && byte_index < extensions_end) {
                uint64_t extension_type = 0;
                uint64_t extension_length = 0;
                size_t ll_type = 0;
                size_t ll_length = 0;

                ll_type = picoquic_varint_decode(bytes + byte_index, extensions_end - byte_index, &extension_type);
                byte_index += ll_type;
                ll_length = picoquic_varint_decode(bytes + byte_index, extensions_end - byte_index, &extension_length);
                byte_index += ll_length;

                if (ll_type == 0 || ll_length == 0 || byte_index + extension_length > extensions_end) {
                    if (log_cnxid != 0) {
                        textlog_prefix_initial_cid64(F, cnx_id_64);
                    }
                    fprintf(F, "        Malformed extension -- only %d bytes avaliable for type and length.\n",
                        (int)(extensions_end - byte_index));
                    ret = -1;
                }
                else {
                    if (log_cnxid != 0) {
                        textlog_prefix_initial_cid64(F, cnx_id_64);
                    }
                    fprintf(F, "        Extension type: %" PRIu64 " (%s), length %d%s",
                        extension_type, picoquic_tp_name((picoquic_tp_enum)extension_type), (int)extension_length,
                        (extension_length == 0) ? "" : ", ");

                    if (byte_index + extension_length > extensions_end) {
                        if (log_cnxid != 0) {
                            textlog_prefix_initial_cid64(F, cnx_id_64);
                        }
                        fprintf(F, "Malformed extension, only %d bytes available.\n", (int)(extensions_end - byte_index));
                        ret = -1;
                    }
                    else {
                        for (uint64_t i = 0; i < extension_length; i++) {
                            fprintf(F, "%02x", bytes[byte_index++]);
                        }
                        fprintf(F, "\n");
                    }
                }
            }
        }

        if (ret == 0 && byte_index < bytes_max) {
            if (log_cnxid != 0) {
                textlog_prefix_initial_cid64(F, cnx_id_64);
            }
            fprintf(F, "    Remaining bytes (%d)\n", (uint32_t)(bytes_max - byte_index));
        }
    }
    else {
        if (log_cnxid != 0) {
            textlog_prefix_initial_cid64(F, cnx_id_64);
        }
        fprintf(F, "Received transport parameter TLS extension (%d bytes):\n", (uint32_t)bytes_max);
        if (log_cnxid != 0) {
            textlog_prefix_initial_cid64(F, cnx_id_64);
        }
        fprintf(F, "    First bytes (%d):\n", (uint32_t)(bytes_max - byte_index));
    }

    if (ret == 0)
    {
        while (byte_index < bytes_max && byte_index < 128) {
            if (log_cnxid != 0) {
                textlog_prefix_initial_cid64(F, cnx_id_64);
            }
            fprintf(F, "        ");
            for (int i = 0; i < 32 && byte_index < bytes_max && byte_index < 128; i++) {
                fprintf(F, "%02x", bytes[byte_index++]);
            }
            fprintf(F, "\n");
        }
    }
}
```

### Rust body
```rust
    ) -> std::io::Result<()> {
        let mut ret = false;
        let mut byte_index = 0usize;

        if bytes.len() < 256 {
            let extensions_end = bytes.len();
            if log_cnxid {
                textlog_prefix_initial_cid64(out, cnx_id)?;
            }
            writeln!(out, "    Extension list ({} bytes):", bytes.len())?;
            while !ret && byte_index < extensions_end {
                let mut extension_type = 0u64;
                let mut extension_length = 0u64;
                let ll_type = crate::internal::varint_decode(
                    &bytes[byte_index..extensions_end],
                    &mut extension_type,
                );
                byte_index += ll_type;
                let ll_length = crate::internal::varint_decode(
                    &bytes[byte_index..extensions_end],
                    &mut extension_length,
                );
                byte_index += ll_length;

                if ll_type == 0
                    || ll_length == 0
                    || byte_index + extension_length as usize > extensions_end
                {
                    if log_cnxid {
                        textlog_prefix_initial_cid64(out, cnx_id)?;
                    }
                    writeln!(
                        out,
                        "        Malformed extension -- only {} bytes avaliable for type and length.",
                        extensions_end - byte_index
                    )?;
                    ret = true;
                } else {
                    if log_cnxid {
                        textlog_prefix_initial_cid64(out, cnx_id)?;
                    }
                    let name =
                        crate::tp::TransportParameter::name(extension_type).unwrap_or("unknown");
                    write!(
                        out,
                        "        Extension type: {extension_type} ({name}), length {}{}",
                        extension_length,
                        if extension_length == 0 { "" } else { ", " }
                    )?;
                    let end = byte_index + extension_length as usize;
                    for b in &bytes[byte_index..end] {
                        write!(out, "{b:02x}")?;
                    }
                    byte_index = end;
                    writeln!(out)?;
                }
            }

            if !ret && byte_index < bytes.len() {
                if log_cnxid {
                    textlog_prefix_initial_cid64(out, cnx_id)?;
                }
                writeln!(out, "    Remaining bytes ({})", bytes.len() - byte_index)?;
            }
        } else {
            if log_cnxid {
                textlog_prefix_initial_cid64(out, cnx_id)?;
            }
            writeln!(
                out,
                "Received transport parameter TLS extension ({} bytes):",
                bytes.len()
            )?;
            if log_cnxid {
                textlog_prefix_initial_cid64(out, cnx_id)?;
            }
            writeln!(out, "    First bytes ({}):", bytes.len() - byte_index)?;
        }

        if !ret {
            while byte_index < bytes.len() && byte_index < 128 {
                if log_cnxid {
                    textlog_prefix_initial_cid64(out, cnx_id)?;
                }
                write!(out, "        ")?;
                for _ in 0..32 {
                    if byte_index >= bytes.len() || byte_index >= 128 {
                        break;
                    }
                    write!(out, "{:02x}", bytes[byte_index])?;
                    byte_index += 1;
                }
                writeln!(out)?;
            }
        }

        Ok(())
    }
```

## Pair `picoquic/logger.c:textlog_outgoing_packet`
C: `picoquic/logger.c:2298-2306 textlog_outgoing_packet`
Rust: `rs/fq/src/logger.rs:643-654 outgoing_packet`

### C body
```c
{
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        textlog_outgoing_segment(cnx->quic->F_log, 1,
            cnx, bytes, sequence_number, length, send_buffer, send_length, pn_length);
    }
}
```

### Rust body
```rust
        if !self.is_still_logging() {
            return;
        }
```

## Pair `picoquic/logger.c:textlog_new_connection`
C: `picoquic/logger.c:2363-2368 textlog_new_connection`
Rust: `rs/fq/src/logger.rs:814-817 new_connection`

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
            text.borrow_mut().new_connection(self);
        }
```

## Pair `picoquic/logger.c:picoquic_set_textlog`
C: `picoquic/logger.c:2414-2442 picoquic_set_textlog`
Rust: `rs/fq/src/textlog.rs:46-76 set_textlog`

### C body
```c
{
    int ret = 0;
    FILE* F_log;

    picoquic_textlog_close(quic);

    if (textlog_file != NULL) {
        if (strcmp(textlog_file, "-") == 0) {
            quic->F_log = stdout;
            quic->should_close_log = 0;
        }
        else {
            F_log = picoquic_file_open(textlog_file, "w");
            if (F_log == NULL) {
                DBG_PRINTF("Cannot create log file <%s>\n", textlog_file);
                ret = -1;
            }
            else {
                quic->F_log = F_log;
                quic->should_close_log = 1;
            }
        }

        quic->text_log_fns = &textlog_functions;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        self.textlog_close();

        let Some(path) = textlog_file else {
            return Ok(());
        };
        let path = path.as_ref();

        if path == Path::new("-") {
            self.f_log = Some(Box::new(std::io::stdout()));
            self.should_close_log = false;
        } else {
            match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
            {
                Ok(f) => {
                    self.f_log = Some(Box::new(f));
                    self.should_close_log = true;
                }
                Err(_) => return Err(Error::NoSuchFile),
            }
        }

        Ok(())
    }
```

## Pair `picoquic/logwriter.c:picoquic_log_length`
C: `picoquic/logwriter.c:53-62 picoquic_log_length`
Rust: `rs/fq/src/binlog.rs:367-375 read_length`

### C body
```c
{
    uint64_t n64 = 0;
    size_t len = 0;
    if (bytes != NULL) {
        len = picoquic_varint_decode(bytes, bytes_max - bytes, &n64);
    }
    *nsz = (size_t)n64;
    return (len == 0 || *nsz != n64) ? NULL : bytes + len;
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

## Pair `picoquic/logwriter.c:picoquic_log_close_frame`
C: `picoquic/logwriter.c:212-225 picoquic_log_close_frame`
Rust: `rs/fq/src/binlog.rs:471-481 log_close_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    size_t length = 0;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1);
    bytes = picoquic_log_varint_skip(bytes, bytes_max);
    bytes = picoquic_log_varint_skip(bytes, bytes_max);
    bytes = picoquic_log_length(bytes, bytes_max, &length);
    bytes = picoquic_log_fixed_skip(bytes, bytes_max, length);

    picoquic_binlog_frame(f, bytes_begin, bytes);
    return bytes;
}
```

### Rust body
```rust
fn log_close_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let (length, rest) = read_length(bytes)?;
    bytes = skip_fixed(rest, length)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_new_token_frame`
C: `picoquic/logwriter.c:367-379 picoquic_log_new_token_frame`
Rust: `rs/fq/src/binlog.rs:527-535 log_new_token_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    size_t length = 0;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1);
    bytes = picoquic_log_length(bytes, bytes_max, &length);

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, length);

    picoquic_binlog_frame(f, bytes_begin, bytes);
    return bytes;
}
```

### Rust body
```rust
fn log_new_token_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    let (length, rest) = read_length(bytes)?;
    bytes = skip_fixed(rest, length)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_datagram_frame`
C: `picoquic/logwriter.c:417-435 picoquic_log_datagram_frame`
Rust: `rs/fq/src/binlog.rs:563-582 log_datagram_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    uint8_t ftype = bytes[0];
    size_t length = 0;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1);

    if (ftype & 1) {
        bytes = picoquic_log_length(bytes, bytes_max, &length);
    } else {
        length = bytes_max - bytes;
    }

    picoquic_binlog_frame(f, bytes_begin, bytes);

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, length);
    return bytes;
}
```

### Rust body
```rust
fn log_datagram_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    if bytes_in.is_empty() {
        return None;
    }
    let ftype = bytes_in[0];
    let mut bytes = skip_fixed(bytes_in, 1)?;
    let length: usize;
    if ftype & 1 != 0 {
        let (l, rest) = read_length(bytes)?;
        length = l;
        bytes = rest;
    } else {
        length = bytes.len();
    }
    let header_len = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..header_len]);
    bytes = skip_fixed(bytes, length)?;
    Some(bytes)
}
```
