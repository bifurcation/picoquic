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

## Pair `picoquic/frames.c:picoquic_format_misc_frames_in_context`
C: `picoquic/frames.c:4860-4880 picoquic_format_misc_frames_in_context`
Rust: `rs/fq/src/internal.rs:13784-13810 format_misc_frames_in_context`

### C body
```c
{
    picoquic_misc_frame_header_t* misc_frame;
    /* If present, send misc frame */
    while ((misc_frame = picoquic_find_first_misc_frame(cnx, pc)) != NULL) {
        uint8_t* bytes_misc = bytes;
        int frame_is_pure_ack = misc_frame->is_pure_ack;

        bytes = picoquic_format_first_misc_or_dg_frame(bytes, bytes_max, more_data, is_pure_ack,
            misc_frame, &cnx->first_misc_frame, &cnx->last_misc_frame);
        if (bytes <= bytes_misc) {
            break;
        }
        else {
            *is_pure_ack &= frame_is_pure_ack;
        }
    }

    return bytes;
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

## Pair `picoquic/frames.c:picoquic_queue_handshake_done_frame`
C: `picoquic/frames.c:5191-5197 picoquic_queue_handshake_done_frame`
Rust: `rs/fq/src/internal.rs:13878-13886 queue_handshake_done_frame`

### C body
```c
{
    uint8_t frame_buffer = picoquic_frame_type_handshake_done;

    return picoquic_queue_misc_or_dg_frame(cnx, &cnx->first_datagram, &cnx->last_datagram,
            &frame_buffer, 1, 0, picoquic_packet_context_application);
}
```

### Rust body
```rust
    pub fn queue_handshake_done_frame(&mut self) -> Result<(), crate::Error> {
        encode_misc_frame(
            self,
            vec![crate::frames::FrameType::HandshakeDone as u8],
            false,
            PacketContext::Application,
        );
        Ok(())
    }
```

## Pair `picoquic/frames.c:picoquic_format_first_datagram_frame`
C: `picoquic/frames.c:5342-5379 picoquic_format_first_datagram_frame`
Rust: `rs/fq/src/internal.rs:13912-13947 format_first_datagram_frame`

### C body
```c
{
    if (bytes + cnx->first_datagram->length <= bytes_max) {
        bytes = picoquic_format_first_misc_or_dg_frame(bytes, bytes_max, more_data, is_pure_ack,
            cnx->first_datagram, &cnx->first_datagram, &cnx->last_datagram);
    } else {
        int is_sent = 0;
        uint8_t* frame_content = ((uint8_t*)cnx->first_datagram) + sizeof(picoquic_misc_frame_header_t);

        if (frame_content[0] == picoquic_frame_type_datagram_l) {
            /* It might be possible to squeeze the size by removing the length */
            size_t header_length = 1 + picoquic_varint_skip(frame_content + 1);
            size_t data_length = cnx->first_datagram->length - header_length;
            size_t min_length = data_length + 1;

            if (bytes + min_length <= bytes_max) {
                while (bytes + min_length < bytes_max) {
                    *bytes++ = picoquic_frame_type_padding;
                }
                *bytes++ = picoquic_frame_type_datagram;
                memcpy(bytes, frame_content + header_length, data_length);
                bytes += data_length;
                picoquic_delete_misc_or_dg(&cnx->first_datagram, &cnx->last_datagram, cnx->first_datagram);
                is_sent= 1;
            }
        }
        if (!is_sent && is_first_in_packet) {
            picoquic_log_app_message(cnx, "Deleting datagram length %zu, larger than %zu, MTU %zu",
                cnx->first_datagram->length, bytes_max - bytes, cnx->path[0]->send_mtu);
            picoquic_delete_misc_or_dg(&cnx->first_datagram, &cnx->last_datagram, cnx->first_datagram);
        }

        *more_data |= (cnx->first_datagram != NULL);
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let frame = connection.datagrams.pop_front()?;
    if frame.bytes.first().is_some_and(|b| {
        *b == crate::frames::FrameType::Datagram as u8
            || *b == crate::frames::FrameType::DatagramL as u8
    }) {
        let len = frame.bytes.len();
        if bytes.len() < len {
            connection.datagrams.push_front(frame);
            *more_data = 1;
            return Some(bytes);
        }
        bytes[..len].copy_from_slice(&frame.bytes);
        *is_pure_ack = 0;
        return Some(&mut bytes[len..]);
    }
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, crate::frames::FrameType::DatagramL as u64)
        || !encode_varint_at(bytes, &mut off, frame.bytes.len() as u64)
        || bytes.len() < off + frame.bytes.len()
    {
        connection.datagrams.push_front(frame);
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + frame.bytes.len()].copy_from_slice(&frame.bytes);
    off += frame.bytes.len();
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_parse_ack_frequency_frame`
C: `picoquic/frames.c:5539-5550 picoquic_parse_ack_frequency_frame`
Rust: `rs/fq/src/internal.rs:13993-14008 parse_ack_frequency_frame`

### C body
```c
{
    *reordering_threshold = 0;
    if ((bytes = picoquic_frames_varint_decode(bytes, bytes_max, seq)) != NULL &&
        (bytes = picoquic_frames_varint_decode(bytes, bytes_max, packets)) != NULL &&
        (bytes = picoquic_frames_varint_decode(bytes, bytes_max, microsec)) != NULL &&
        (bytes = picoquic_frames_varint_decode(bytes, bytes_max, reordering_threshold)) != NULL){
        *ignore_order = (*reordering_threshold == 0);
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    *reordering_threshold = 0;
    let bytes = frames_varint_decode(bytes, seq)?;
    let bytes = frames_varint_decode(bytes, packets)?;
    let bytes = frames_varint_decode(bytes, microsec)?;
    let bytes = frames_varint_decode(bytes, reordering_threshold)?;
    *ignore_order = u8::from(*reordering_threshold == 0);
    Some(bytes)
}
```

## Pair `picoquic/frames.c:picoquic_encode_time_stamp_length`
C: `picoquic/frames.c:5733-5738 picoquic_encode_time_stamp_length`
Rust: `rs/fq/src/internal.rs:14111-14116 encode_time_stamp_length`

### C body
```c
{
    uint64_t time_stamp = (current_time - cnx->start_time) >> cnx->local_parameters.ack_delay_exponent;

    return (2 + picoquic_encode_varint_length(time_stamp));
}
```

### Rust body
```rust
    pub fn encode_time_stamp_length(&self, current_time: Instant) -> usize {
        let delta = current_time.ticks().saturating_sub(self.start_time.ticks());
        let time_stamp = delta >> self.local_parameters.ack_delay_exponent;
        encode_varint_length(crate::frames::FrameType::TimeStamp as u64)
            + encode_varint_length(time_stamp)
    }
```

## Pair `picoquic/frames.c:picoquic_format_path_available_or_backup_frame`
C: `picoquic/frames.c:5863-5878 picoquic_format_path_available_or_backup_frame`
Rust: `rs/fq/src/internal.rs:14186-14201 format_path_available_or_backup_frame`

### C body
```c
{
    /* This code assumes that the frame type is already skipped */
    uint8_t* bytes0 = bytes;
    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, frame_type)) == NULL ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, path_id)) == NULL ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, sequence)) == NULL) {
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
    for value in [frame_type, path_id, sequence] {
        if !encode_varint_at(bytes, &mut off, value) {
            *more_data = 1;
            return Some(bytes);
        }
    }
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_queue_max_path_id_frame`
C: `picoquic/frames.c:6012-6026 picoquic_queue_max_path_id_frame`
Rust: `rs/fq/src/internal.rs:13453-13471 picoquic_queue_max_path_id_frame`

### C body
```c
{
    /* Frame buffer sized so the code will always succeed */
    int ret = 0;
    uint8_t frame_buffer[256];
    int is_pure_ack = 0;
    int more_data = 0;
    uint8_t* bytes_next = picoquic_format_max_path_id_frame(
        frame_buffer, frame_buffer + sizeof(frame_buffer), cnx->max_path_id_local, & more_data);
    size_t consumed = bytes_next - frame_buffer;
    ret = picoquic_queue_misc_frame(cnx, frame_buffer, consumed, is_pure_ack,
        picoquic_packet_context_application);
    return ret;
}
```

### Rust body
```rust
pub fn picoquic_queue_max_path_id_frame(connection: &mut Connection) -> Result<(), crate::Error> {
    let mut frame_buffer = [0u8; 256];
    let mut more_data = 0;
    let frame_len = frame_buffer.len();
    let bytes_next = format_max_path_id_frame(
        &mut frame_buffer,
        connection.max_path_id_local,
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

## Pair `picoquic/frames.c:picoquic_path_cid_next_sequence_number`
C: `picoquic/frames.c:6239-6255 picoquic_path_cid_next_sequence_number`
Rust: `rs/fq/src/internal.rs:4940-4955 path_cid_next_sequence_number`

### C body
```c
{
    picoquic_remote_cnxid_stash_t* stash = picoquic_find_or_create_remote_cnxid_stash(path_x->cnx, path_x->unique_path_id, 0);
    uint64_t next_sequence_number = 0;

    if (stash != NULL) {
        picoquic_remote_cnxid_t* remote_cnxid = stash->cnxid_stash_first;

        while (remote_cnxid != NULL) {
            if (remote_cnxid->sequence >= next_sequence_number) {
                next_sequence_number = remote_cnxid->sequence + 1;
            }
            remote_cnxid = remote_cnxid->next;
        }
    }
    return next_sequence_number;
}
```

### Rust body
```rust
            .map(|stash| {
                stash
                    .connection_ids
                    .iter()
                    .fold(0u64, |next_sequence_number, remote_cnxid| {
                        if remote_cnxid.sequence >= next_sequence_number {
                            remote_cnxid.sequence.saturating_add(1)
                        } else {
                            next_sequence_number
                        }
                    })
            })
```

## Pair `picoquic/frames.c:picoquic_parse_observed_address_frame`
C: `picoquic/frames.c:6479-6492 picoquic_parse_observed_address_frame`
Rust: `rs/fq/src/internal.rs:14489-14509 parse_observed_address_frame`

### C body
```c
{
    if ((bytes = picoquic_frames_varint_decode(bytes, bytes_max, sequence)) != NULL) {
        size_t l_addr = ((ftype & 1) == 0) ? 4 : 16;

        *addr = bytes;
        if ((bytes = picoquic_frames_fixed_skip(bytes, bytes_max, l_addr)) != NULL) {
            bytes = picoquic_frames_uint16_decode(bytes, bytes_max, port);
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<(ObservedAddress<'a>, &'a [u8])> {
    let mut sequence = 0;
    let bytes = frames_varint_decode(bytes, &mut sequence)?;
    let addr_len = if (ftype & 1) == 0 { 4 } else { 16 };
    if bytes.len() < addr_len + 2 {
        return None;
    }
    let addr = &bytes[..addr_len];
    let port = parse_16(&bytes[addr_len..addr_len + 2]);
    Some((
        ObservedAddress {
            sequence,
            addr,
            port,
        },
        &bytes[addr_len + 2..],
    ))
}
```

## Pair `picoquic/frames.c:picoquic_is_path_challenging_packet`
C: `picoquic/frames.c:7291-7327 picoquic_is_path_challenging_packet`
Rust: `rs/fq/src/internal.rs:14863-14888 is_path_challenging_packet`

### C body
```c
{
    const uint8_t* bytes_max = bytes + bytes_maxsize;
    uint64_t frame_id64;
    int is_challenge = 1;
    int has_challenge = 0;

    /* Only a few frames are path challenging, so we check for those. */
    while (bytes != NULL && bytes < bytes_max) {
        size_t consumed = 0;
        int pure_ack = 0;

        if (picoquic_frames_varint_decode(bytes, bytes_max, &frame_id64) == NULL) {
            break;
        }
        switch (frame_id64) {
        case picoquic_frame_type_path_challenge:
            has_challenge = 1;
            break;
        case picoquic_frame_type_path_response:
        case picoquic_frame_type_padding:
        case picoquic_frame_type_new_connection_id:
        case picoquic_frame_type_path_new_connection_id:
            break;
        default:
            is_challenge = 0;
            break;
        }
        if (picoquic_skip_frame(bytes, bytes_max - bytes, &consumed, &pure_ack) != 0) {
            break;
        }
        else {
            bytes += consumed;
        }
    }
    return (is_challenge && has_challenge);
}
```

### Rust body
```rust
pub fn is_path_challenging_packet(_bytes: &[u8], _bytes_maxsize: usize) -> bool {
    let max = _bytes_maxsize.min(_bytes.len());
    let mut bytes = &_bytes[..max];
    let mut has_challenge = false;
    while !bytes.is_empty() {
        let mut frame_id = 0;
        if frames_varint_decode(bytes, &mut frame_id).is_none() {
            break;
        }
        match frame_id {
            x if x == crate::frames::FrameType::PathChallenge as u64 => has_challenge = true,
            x if x == crate::frames::FrameType::PathResponse as u64
                || x == crate::frames::FrameType::Padding as u64
                || x == crate::frames::FrameType::NewConnectionId as u64
                || x == crate::frames::FrameType::PathNewConnectionId as u64 => {}
            _ => return false,
        }
        let mut consumed = 0;
        let mut pure_ack = 0;
        if skip_frame(bytes, bytes.len(), &mut consumed, &mut pure_ack) != 0 || consumed == 0 {
            break;
        }
        bytes = &bytes[consumed..];
    }
    has_challenge
}
```

## Pair `picoquic/intformat.c:picoformat_32`
C: `picoquic/intformat.c:40-46 picoformat_32`
Rust: `rs/fq/src/utils.rs:967-972 picoformat_32`

### C body
```c
{
    bytes[0] = (uint8_t)(n32 >> 24);
    bytes[1] = (uint8_t)(n32 >> 16);
    bytes[2] = (uint8_t)(n32 >> 8);
    bytes[3] = (uint8_t)(n32);
}
```

### Rust body
```rust
pub fn picoformat_32(bytes: &mut [u8], n32: u32) {
    bytes[0] = (n32 >> 24) as u8;
    bytes[1] = (n32 >> 16) as u8;
    bytes[2] = (n32 >> 8) as u8;
    bytes[3] = n32 as u8;
}
```

## Pair `picoquic/intformat.c:picoquic_varint_encode`
C: `picoquic/intformat.c:90-126 picoquic_varint_encode`
Rust: `rs/fq/src/internal.rs:6345-6350 varint_encode`

### C body
```c
{
    uint8_t* x = bytes;

    if (n64 < 16384) {
        if (n64 < 64) {
            if (max_bytes > 0) {
                *x++ = (uint8_t)(n64);
            }
        } else {
            if (max_bytes >= 2) {
                *x++ = (uint8_t)((n64 >> 8) | 0x40);
                *x++ = (uint8_t)(n64);
            }
        }
    } else if (n64 < 1073741824) {
        if (max_bytes >= 4) {
            *x++ = (uint8_t)((n64 >> 24) | 0x80);
            *x++ = (uint8_t)(n64 >> 16);
            *x++ = (uint8_t)(n64 >> 8);
            *x++ = (uint8_t)(n64);
        }
    } else {
        if (max_bytes >= 8) {
            *x++ = (uint8_t)((n64 >> 56) | 0xC0);
            *x++ = (uint8_t)(n64 >> 48);
            *x++ = (uint8_t)(n64 >> 40);
            *x++ = (uint8_t)(n64 >> 32);
            *x++ = (uint8_t)(n64 >> 24);
            *x++ = (uint8_t)(n64 >> 16);
            *x++ = (uint8_t)(n64 >> 8);
            *x++ = (uint8_t)(n64);
        }
    }

    return (x - bytes);
}
```

### Rust body
```rust
            if bytes.is_empty() {
                return 0;
            }
```

## Pair `picoquic/logger.c:textlog_prefix_initial_cid64`
C: `picoquic/logger.c:52-57 textlog_prefix_initial_cid64`
Rust: `rs/fq/src/tests/util.rs:4189-4197 textlog_prefix_initial_cid64`

### C body
```c
{
    if (log_cnxid64 != 0) {
        fprintf(F, "%016llx: ", (unsigned long long)log_cnxid64);
    }
}
```

### Rust body
```rust
    ) -> std::io::Result<()> {
        if cnx_id != 0 {
            write!(out, "{cnx_id:016x}: ")?;
        }
        Ok(())
    }
```

## Pair `picoquic/logger.c:textlog_buffered_packet`
C: `picoquic/logger.c:2286-2296 textlog_buffered_packet`
Rust: `rs/fq/src/logger.rs:618-621 buffered_packet`

### C body
```c
{
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        FILE* F = cnx->quic->F_log;

        textlog_prefix_initial_cid64(F, picoquic_val64_connection_id(picoquic_get_logging_cnxid(cnx)));
        textlog_time(F, cnx, current_time, "T= ", ", ");
        fprintf(F, "Keys unavailable, buffered packet type %d.\n", ptype);
    }
}
```

### Rust body
```rust
        if !self.is_still_logging() {
            return;
        }
```

## Pair `picoquic/logger.c:textlog_transport_extension`
C: `picoquic/logger.c:2346-2353 textlog_transport_extension`
Rust: `rs/fq/src/logger.rs:776-780 transport_extension`

### C body
```c
{
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        /* TODO: alpn */
        picoquic_textlog_transport_extension(cnx->quic->F_log, cnx, (is_local)?0:1, 1, params, param_length);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .transport_extension(self, is_local, params);
        }
```

## Pair `picoquic/logger.c:picoquic_textlog_close`
C: `picoquic/logger.c:2383-2391 picoquic_textlog_close`
Rust: `rs/fq/src/textlog.rs:85-100 textlog_close`

### C body
```c
{
    if (quic->F_log != NULL && quic->should_close_log) {
        (void)picoquic_file_close(quic->F_log);
    }

    quic->F_log = NULL;
    quic->should_close_log = 0;
}
```

### Rust body
```rust
    pub fn log_app_message_v(&mut self, args: core::fmt::Arguments<'_>) {
        crate::logger::Log::app_message(self, args);
    }
```

## Pair `picoquic/logwriter.c:picoquic_log_varint`
C: `picoquic/logwriter.c:47-51 picoquic_log_varint`
Rust: `rs/fq/src/internal.rs:6419-6434 frames_varint_decode`

### C body
```c
{
    size_t len = (bytes == NULL) ? 0 : picoquic_varint_decode(bytes, bytes_max - bytes, n64);
    return len == 0 ? NULL : bytes + len;
}
```

### Rust body
```rust
pub fn frames_varint_decode<'a>(bytes: &'a [u8], n64: &mut u64) -> Option<&'a [u8]> {
    if bytes.is_empty() {
        return None;
    }
    let length = 1usize << ((bytes[0] & 0xC0) >> 6);
    if length > bytes.len() {
        return None;
    }
    let mut v = (bytes[0] & 0x3F) as u64;
    for b in bytes.iter().take(length).skip(1) {
        v <<= 8;
        v += *b as u64;
    }
    *n64 = v;
    Some(&bytes[length..])
}
```

## Pair `picoquic/logwriter.c:picoquic_log_ack_frame`
C: `picoquic/logwriter.c:138-171 picoquic_log_ack_frame`
Rust: `rs/fq/src/binlog.rs:424-449 log_ack_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    uint64_t ftype = 0;
    uint64_t nb_blocks;

    (void) picoquic_varint_decode(bytes, bytes_max - bytes, &ftype);

    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Logging the frame type, maybe multiple bytes */

    if (ftype == picoquic_frame_type_path_ack || ftype == picoquic_frame_type_path_ack_ecn) {
        bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Log the path_id */
    }

    bytes = picoquic_log_varint_skip(bytes, bytes_max);
    bytes = picoquic_log_varint_skip(bytes, bytes_max);
    bytes = picoquic_log_varint(bytes, bytes_max, &nb_blocks);

    for (uint64_t i = 0; bytes != NULL && i <= nb_blocks; i++) {
        if (i != 0) {
            bytes = picoquic_log_varint_skip(bytes, bytes_max);
        }
        bytes = picoquic_log_varint_skip(bytes, bytes_max);
    }
    
    if (ftype == picoquic_frame_type_ack_ecn || ftype == picoquic_frame_type_path_ack_ecn) {
        bytes = picoquic_log_varint_skip(bytes, bytes_max);
        bytes = picoquic_log_varint_skip(bytes, bytes_max);
        bytes = picoquic_log_varint_skip(bytes, bytes_max);
    }

    picoquic_binlog_frame(f, bytes_begin, bytes);
    return bytes;
}
```

### Rust body
```rust
fn log_ack_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut ftype = 0u64;
    let _ = frames_varint_decode(bytes_in, &mut ftype)?;
    let mut bytes = frames_varint_skip(bytes_in)?;
    if ftype == FrameType::PathAck as u64 || ftype == FrameType::PathAckEcn as u64 {
        bytes = frames_varint_skip(bytes)?;
    }
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let mut nb_blocks = 0u64;
    bytes = frames_varint_decode(bytes, &mut nb_blocks)?;
    bytes = frames_varint_skip(bytes)?;
    for _ in 0..nb_blocks {
        bytes = frames_varint_skip(bytes)?;
        bytes = frames_varint_skip(bytes)?;
    }
    if ftype == FrameType::AckEcn as u64 || ftype == FrameType::PathAckEcn as u64 {
        bytes = frames_varint_skip(bytes)?;
        bytes = frames_varint_skip(bytes)?;
        bytes = frames_varint_skip(bytes)?;
    }
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_path_new_connection_id_frame`
C: `picoquic/logwriter.c:326-342 picoquic_log_path_new_connection_id_frame`
Rust: `rs/fq/src/binlog.rs:510-525 log_path_new_connection_id_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;

    bytes = picoquic_log_varint_skip(bytes, bytes_max);
    bytes = picoquic_log_varint_skip(bytes, bytes_max);
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
fn log_path_new_connection_id_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
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

## Pair `picoquic/logwriter.c:picoquic_log_handshake_done_frame`
C: `picoquic/logwriter.c:407-415 picoquic_log_handshake_done_frame`
Rust: `rs/fq/src/binlog.rs:556-561 log_handshake_done_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1);

    picoquic_binlog_frame(f, bytes_begin, bytes);
    return bytes;
}
```

### Rust body
```rust
fn log_handshake_done_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes = skip_fixed(bytes_in, 1)?;
    let consumed = bytes_in.len() - bytes.len();
    append_frame(out, &bytes_in[..consumed]);
    Some(bytes)
}
```
