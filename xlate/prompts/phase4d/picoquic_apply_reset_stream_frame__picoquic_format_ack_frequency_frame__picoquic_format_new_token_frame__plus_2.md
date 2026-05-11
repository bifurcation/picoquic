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

## `picoquic/frames.c:picoquic_apply_reset_stream_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust returns immediately for invalid local unidirectional reset, while C signals the error but continues through find/create and other reset handling before returning NULL.
* C source: `picoquic/frames.c:330-371`
* C signature: `const uint8_t * picoquic_apply_reset_stream_frame(picoquic_cnx_t *, const uint8_t *, uint64_t, uint64_t, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:10257-10331`
* Rust item: `picoquic_apply_reset_stream_frame`

### C body
```c
{
    picoquic_stream_head_t* stream;
    
    if (!IS_BIDIR_STREAM_ID(stream_id) && IS_LOCAL_STREAM_ID(stream_id, cnx->client_mode)) {
        /* the peer cannot send data, and thus cannot reset the stream */
        bytes = NULL;
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_STREAM_STATE_ERROR,
            picoquic_frame_type_reset_stream);
    }
    if ((stream = picoquic_find_or_create_stream(cnx, stream_id, 1)) == NULL) {
        /* Not finding the stream is only an error if the stream
         * was expected to be present, or created on demand. If the
         * stream was already created and then deleted, there is no harm.
         * If the "return NULL" is in a normal scenario, the connection state
         * will remain "ready" or "almost ready"
         */
        if (cnx->cnx_state > picoquic_state_ready) {
            bytes = NULL;  /* error already signaled */
        }
    }
    else if ((stream->fin_received || stream->reset_received) && final_offset != stream->fin_offset) {
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_FINAL_OFFSET_ERROR,
            picoquic_frame_type_reset_stream);
        bytes = NULL;

    }
    else if (picoquic_flow_control_check_stream_offset(cnx, stream, final_offset) != 0) {
        bytes = NULL;  // error already signaled
    }
    else if (!stream->reset_received) {
        stream->reset_received = 1;
        stream->reset_offset = reliable_size;
        stream->remote_error = error_code_64;

        if (stream->consumed_offset >= stream->reset_offset) {
            picoquic_signal_stream_reset(cnx, stream);
        }
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    use crate::stream::{Role, StreamId};

    let local_role = if connection.client_mode {
        Role::Client
    } else {
        Role::Server
    };
    let sid = StreamId(stream_id);
    if !sid.is_bidir() && sid.is_local(local_role) {
        connection.connection_error(
            crate::errors::TransportError::StreamStateError as u64,
            crate::frames::FrameType::ResetStream as u64,
        );
        return None;
    }

    let stream_token = match connection.find_stream(stream_id) {
        Some(token) => token,
        None => match connection.create_missing_streams(stream_id, true) {
            Ok(token) => token,
            Err(_) => {
                if connection.connection_state > State::Ready {
                    return None;
                }
                return Some(bytes);
            }
        },
    };

    let final_offset_mismatch = connection
        .streams
        .get(stream_token)
        .map(|stream| {
            (stream.fin_received || stream.reset_received) && final_offset != stream.fin_offset
        })
        .unwrap_or(false);
    if final_offset_mismatch {
        connection.connection_error(
            crate::errors::TransportError::FinalOffsetError as u64,
            crate::frames::FrameType::ResetStream as u64,
        );
        return None;
    }

    if flow_control_check_stream_offset_token(connection, stream_token, final_offset) != 0 {
        return None;
    }

    let should_signal = if let Some(stream) = connection.streams.get_mut(stream_token) {
        if !stream.reset_received {
            stream.reset_received = true;
            stream.reset_offset = reliable_size;
            stream.remote_error = error_code_64;
            stream.consumed_offset >= stream.reset_offset
        } else {
            false
        }
    } else {
        false
    };

    if should_signal {
        signal_stream_reset_token(connection, stream_token);
    }

    Some(bytes)
}
```

## `picoquic/frames.c:picoquic_format_ack_frequency_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: On encode failure the C body restores the original pointer before returning, while the Rust body may have partially written bytes and returns the original slice with more_data set; visible side effects differ on failure.
* C source: `picoquic/frames.c:5594-5640`
* C signature: `uint8_t * picoquic_format_ack_frequency_frame(picoquic_cnx_t *, uint8_t *, uint8_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:14010-14072`
* Rust item: `format_ack_frequency_frame`

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

## `picoquic/frames.c:picoquic_format_new_token_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body performs the encoding directly; Rust body only delegates to another format_new_token_frame not shown in the provided body.
* C source: `picoquic/frames.c:1013-1027`
* C signature: `uint8_t * picoquic_format_new_token_frame(uint8_t *, uint8_t *, int *, int *, uint8_t *, size_t)`
* Rust source: `rs/fq/src/internal.rs:13542-13549`
* Rust item: `format_new_token_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes; 
    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_new_token)) != NULL &&
        (bytes = picoquic_frames_length_data_encode(bytes, bytes_max, token_length, token)) != NULL) {
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
        format_new_token_frame(bytes, more_data, is_pure_ack, token)
    }
```

## `picoquic/frames.c:picoquic_queue_path_cid_blocked_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: The C body sets sending_path_cid_blocked_frame only if queueing returns 0, while the Rust body sets it unconditionally after encode_misc_frame and has no visible queue failure check.
* C source: `picoquic/frames.c:6257-6276`
* C signature: `int picoquic_queue_path_cid_blocked_frame(picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:14270-14294`
* Rust item: `picoquic_queue_path_cid_blocked_frame`

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

## `picoquic/logwriter.c:binlog_outgoing_packet`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust uses an in-line parse_outgoing_header instead of the C body's picoquic_parse_packet_header call; the rest of the visible packet-number, offset, checksum, and packet logging flow is similar.
* C source: `picoquic/logwriter.c:841-888`
* C signature: `void binlog_outgoing_packet(picoquic_cnx_t *, picoquic_path_t *, uint8_t *, uint64_t, size_t, size_t, uint8_t *, size_t, uint64_t)`
* Rust source: `rs/fq/src/binlog.rs:980-1530`
* Rust item: `outgoing_packet`

### C body
```c
{
    FILE* f = cnx->f_binlog;

    picoquic_cnx_t* pcnx = cnx;
    picoquic_packet_header ph;
    size_t checksum_length = 16;
    struct sockaddr_in default_addr;

    const picoquic_connection_id_t * cnxid = (cnx != NULL) ? &cnx->initial_cnxid : &picoquic_null_connection_id;

    memset(&default_addr, 0, sizeof(struct sockaddr_in));
    default_addr.sin_family = AF_INET;

    picoquic_parse_packet_header((cnx == NULL) ? NULL : cnx->quic, send_buffer, send_length,
        ((cnx == NULL || cnx->path[0] == NULL) ? (struct sockaddr *)&default_addr :
        (struct sockaddr *)&cnx->path[0]->first_tuple->local_addr), &ph, &pcnx, 0);

    if (cnx != NULL) {
        picoquic_epoch_enum epoch = (ph.ptype == picoquic_packet_1rtt_protected) ? picoquic_epoch_1rtt :
            ((ph.ptype == picoquic_packet_0rtt_protected) ? picoquic_epoch_0rtt :
            ((ph.ptype == picoquic_packet_handshake) ? picoquic_epoch_handshake : picoquic_epoch_initial));
        if (cnx->crypto_context[epoch].aead_encrypt != NULL) {
            checksum_length = picoquic_get_checksum_length(cnx, epoch);
        }
    }

    ph.pn64 = sequence_number;
    ph.pn = (uint32_t)ph.pn64;
    if (ph.ptype != picoquic_packet_retry) {
        if (ph.pn_offset != 0) {
            ph.offset = ph.pn_offset + pn_length;
            ph.payload_length -= pn_length;
        }
    }
    if (ph.ptype != picoquic_packet_version_negotiation) {
        if (ph.payload_length > checksum_length) {
            ph.payload_length -= (uint16_t)checksum_length;
        }
        else {
            ph.payload_length = 0;
        }
    }

    binlog_packet(f, cnxid, binlog_get_path_id(cnx, path_x),  0, current_time, &ph, bytes, length);
}
```

### Rust body
```rust
impl Binlog for Connection {
    fn dropped_packet(
        &mut self,
        path_x: &mut Path,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    ) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        // Reserve four bytes for the chunk size; patch afterwards
        // — mirrors the C source.
        let _ = msg.write_u32(0);
        compose_event_header(
            &mut msg,
            &cid,
            current_time,
            path_id,
            LogEventType::PacketDropped,
        );
        let _ = msg.write_varint(ph.packet_type as u64);
        let _ = msg.write_varint(packet_size as u64);
        let _ = msg.write_varint(err as u64);

        let body_len = (msg.len().saturating_sub(4)) as u32;
        let mut payload = msg.as_bytes().to_vec();
        drop(msg);
        payload[..4].copy_from_slice(&body_len.to_be_bytes());

        if let Some(f) = self.f_binlog.as_mut() {
            let _ = f.write_all(&payload);
        }
    }

    fn buffered_packet(&mut self, path_x: &mut Path, ptype: PacketType, current_time: Instant) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        let _ = msg.write_u32(0);
        compose_event_header(
            &mut msg,
            &cid,
            current_time,
            path_id,
            LogEventType::PacketBuffered,
        );
        let _ = msg.write_varint(ptype as u64);
        let _ = msg.write_str("keys_unavailable");

        let body_len = (msg.len().saturating_sub(4)) as u32;
        let mut payload = msg.as_bytes().to_vec();
        drop(msg);
        payload[..4].copy_from_slice(&body_len.to_be_bytes());
        if let Some(f) = self.f_binlog.as_mut() {
            let _ = f.write_all(&payload);
        }
    }

    fn outgoing_packet(
        &mut self,
        path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: Instant,
    ) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);

        // The C body calls `picoquic_parse_packet_header(cnx->quic,
        // send_buffer, …)` to recover the header from the encrypted
        // wire form.  `parse_packet_header` lives on [`Quic`] and
        // `Connection` carries no back-pointer; do a minimal in-line
        // parse so we can still emit a useful record.
        let mut ph = parse_outgoing_header(send_buffer);

        let checksum_length: usize = {
            let epoch = match ph.packet_type {
                PacketType::OneRttProtected => crate::internal::Epoch::OneRtt,
                PacketType::ZeroRttProtected => crate::internal::Epoch::ZeroRtt,
                PacketType::Handshake => crate::internal::Epoch::Handshake,
                _ => crate::internal::Epoch::Initial,
            };
            if self.crypto_context[epoch as usize].aead_encrypt.is_some() {
                self.get_checksum_length(epoch)
            } else {
                16
            }
        };

        ph.packet_number_full = sequence_number;
        ph.packet_number_truncated = sequence_number as u32;
        if ph.packet_type != PacketType::Retry && ph.packet_number_offset != 0 {
            ph.offset = ph.packet_number_offset + pn_length;
            ph.payload_length = ph.payload_length.saturating_sub(pn_length);
        }
        if ph.packet_type != PacketType::VersionNegotiation {
            if ph.payload_length > checksum_length {
                ph.payload_length -= checksum_length;
            } else {
                ph.payload_length = 0;
            }
        }

        if let Some(f) = self.f_binlog.as_mut() {
            packet(f, &cid, path_id, false, current_time, &ph, bytes);
        }
    }

    fn packet_lost(
        &mut self,
        path_x: &mut Path,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: Instant,
    ) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        let _ = msg.write_u32(0);
        compose_event_header(
            &mut msg,
            &cid,
            current_time,
            path_id,
            LogEventType::PacketLost,
        );
        let _ = msg.write_varint(ptype as u64);
        let _ = msg.write_varint(sequence_number);
        let _ = msg.write_str(trigger);
        match dcid {
            Some(d) => {
                let _ = msg.write_cid(d);
            }
            None => {
                let _ = msg.write_u8(0);
            }
        }
        let _ = msg.write_varint(packet_size as u64);

        let body_len = (msg.len().saturating_sub(4)) as u32;
        let mut payload = msg.as_bytes().to_vec();
        drop(msg);
        payload[..4].copy_from_slice(&body_len.to_be_bytes());
        if let Some(f) = self.f_binlog.as_mut() {
            let _ = f.write_all(&payload);
        }
    }

    fn negotiated_alpn(&mut self, is_local: bool, sni: &[u8], alpn: &[u8], alpn_list: &[&[u8]]) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let now = self.quic_time();

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        compose_event_header(&mut msg, &cid, now, 0, LogEventType::AlpnUpdate);
        let _ = msg.write_varint(if is_local { 1 } else { 0 });
        let _ = msg.write_varint(sni.len() as u64);
        if !sni.is_empty() {
            let _ = msg.write_bytes(sni);
        }
        let _ = msg.write_varint(alpn_list.len() as u64);
        for entry in alpn_list {
            let _ = msg.write_varint(entry.len() as u64);
            let _ = msg.write_bytes(entry);
        }
        let _ = msg.write_varint(alpn.len() as u64);
        if !alpn.is_empty() {
            let _ = msg.write_bytes(alpn);
        }

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }

    fn transport_extension(&mut self, is_local: bool, params: &[u8]) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let now = self.quic_time();

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        compose_event_header(&mut msg, &cid, now, 0, LogEventType::ParamUpdate);
        let _ = msg.write_varint(if is_local { 1 } else { 0 });
        let _ = msg.write_varint(params.len() as u64);
        if !params.is_empty() {
            let _ = msg.write_bytes(params);
        }

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }

    fn message_v(&mut self, args: core::fmt::Arguments<'_>) {
        if self.f_binlog.is_none() {
            return;
        }

        let cid = self.initial_connection_id;
        let now = self.quic_time();
        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };

        compose_event_header(&mut msg, &cid, now, 0, LogEventType::InfoMessage);

        let message = args.to_string();
        let max_len = msg.remaining().saturating_sub(1);
        let message_len = message.len().min(max_len);
        let _ = msg.write_bytes(&message.as_bytes()[..message_len]);

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }

    fn new_connection(&mut self) {
        // The C body opens a new file using `cnx->quic->binlog_dir`.
        // `Connection` carries no back-pointer to its `Quic`, so the
        // file-open path lives on the Quic side; here we only emit
        // the `new_connection` record when the file handle is
        // already attached, mirroring the C `bin_log_fns == NULL`
        // short-circuit.
        if self.f_binlog.is_none() {
            return;
        }

        let cid = self.initial_connection_id;
        let start_time = self.start_time;
        let client_mode = self.client_mode;
        let proposed_version = self.proposed_version;
        let cc_id: &'static str = self
            .congestion_alg
            .map(|a| a.congestion_algorithm_id)
            .unwrap_or("");
        let spin_policy = self.spin_policy as u64;
        let remote_cid: ConnectionId = self
            .paths
            .first()
            .and_then(|p| p.tuples.first())
            .and_then(|t| t.remote_connection_id_index)
            .and_then(|idx| {
                self.remote_connection_id_stashes.iter().find_map(|stash| {
                    stash
                        .connection_ids
                        .iter()
                        .find(|r| (r.sequence as usize) == idx)
                        .map(|r| r.connection_id)
                })
            })
            .unwrap_or_default();

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        compose_event_header(&mut msg, &cid, start_time, 0, LogEventType::NewConnection);
        let _ = msg.write_u8(if client_mode { 1 } else { 0 });
        let _ = msg.write_u32(proposed_version);
        let _ = msg.write_cid(&remote_cid);
        let _ = msg.write_str(cc_id);
        let _ = msg.write_varint(spin_policy);

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }

    fn close_connection(&mut self) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let now = self.quic_time();

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            self.f_binlog = None;
            self.binlog_file_name = None;
            return;
        };
        compose_event_header(&mut msg, &cid, now, 0, LogEventType::ConnectionClose);
        let payload = msg.as_bytes().to_vec();
        drop(msg);

        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
            let _ = f.flush();
        }

        // Drop the file handle and release the file-name bookkeeping.
        // The C body also calls `quic->autoqlog_fn(cnx)` and
        // decrements `quic->current_number_of_open_logs`; both
        // require a back-pointer to `Quic` we don't carry here, so
        // those bookkeeping hooks fire on the Quic side at teardown.
        self.f_binlog = None;
        self.binlog_file_name = None;
    }

    fn cc_dump(&mut self, path_x: &mut Path, current_time: Instant) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);
        let is_multipath = self.is_multipath_enabled;

        // Snapshot the packet-context fields up front — multipath
        // chooses path_x's pkt_ctx, single-path uses the connection's
        // application-context entry.
        let (
            send_sequence,
            highest_acknowledged,
            highest_acknowledged_time,
            latest_time_acknowledged,
        ) = if is_multipath {
            (
                path_x.pkt_ctx.send_sequence,
                path_x.pkt_ctx.highest_acknowledged,
                path_x.pkt_ctx.highest_acknowledged_time,
                path_x.pkt_ctx.latest_time_acknowledged,
            )
        } else {
            let ctx = &self.pkt_ctx[crate::PacketContext::Application as usize];
            (
                ctx.send_sequence,
                ctx.highest_acknowledged,
                ctx.highest_acknowledged_time,
                ctx.latest_time_acknowledged,
            )
        };

        let path_cwin = path_x.cwin;
        let one_way = path_x.one_way_delay_sample.ticks();
        let rtt_sample = path_x.rtt_sample.ticks();
        let smoothed_rtt = path_x.smoothed_rtt.ticks();
        let rtt_min = path_x.rtt_min.ticks();
        let bw = path_x.bandwidth_estimate;
        let rate = path_x.receive_rate_estimate;
        let mtu = path_x.send_mtu as u64;
        let pacing_pkt_time = path_x.pacing.packet_time_microsec.ticks();
        let nb_losses_found = path_x.nb_losses_found;
        let nb_spurious_path = path_x.nb_spurious;
        let peak_bw = path_x.peak_bandwidth_estimate;
        let bytes_in_transit = path_x.bytes_in_transit;
        let limited = path_x.last_bw_estimate_path_limited;

        let nb_retx_total = self.nb_retransmission_total;
        let nb_spurious_cnx = self.nb_spurious;
        let cwin_blocked = self.cwin_blocked;
        let flow_blocked = self.flow_blocked;
        let stream_blocked = self.stream_blocked;
        let start_time = self.start_time.ticks();

        // Optional CC observation: paths[0] gets queried for its
        // congestion_alg_state, mirroring the C body.
        let cc_obs: Option<(u64, u64)> = self.congestion_alg.and_then(|alg| {
            self.paths.first().and_then(|p0| {
                if p0.congestion_alg_state.is_some() {
                    alg.algorithm.alg_observe(p0)
                } else {
                    None
                }
            })
        });

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        compose_event_header(
            &mut msg,
            &cid,
            current_time,
            path_id,
            LogEventType::CcUpdate,
        );

        let _ = msg.write_varint(send_sequence);
        if highest_acknowledged != u64::MAX {
            let _ = msg.write_varint(1);
            let _ = msg.write_varint(highest_acknowledged);
            let _ = msg.write_varint(highest_acknowledged_time.ticks().saturating_sub(start_time));
            let _ = msg.write_varint(latest_time_acknowledged.ticks().saturating_sub(start_time));
        } else {
            let _ = msg.write_varint(0);
        }

        let _ = msg.write_varint(path_cwin);
        let _ = msg.write_varint(one_way);
        let _ = msg.write_varint(rtt_sample);
        let _ = msg.write_varint(smoothed_rtt);
        let _ = msg.write_varint(rtt_min);
        let _ = msg.write_varint(bw);
        let _ = msg.write_varint(rate);
        let _ = msg.write_varint(mtu);
        let _ = msg.write_varint(pacing_pkt_time);
        if is_multipath {
            let _ = msg.write_varint(nb_losses_found);
            let _ = msg.write_varint(nb_spurious_path);
        } else {
            let _ = msg.write_varint(nb_retx_total);
            let _ = msg.write_varint(nb_spurious_cnx);
        }
        let _ = msg.write_varint(if cwin_blocked { 1 } else { 0 });
        let _ = msg.write_varint(if flow_blocked { 1 } else { 0 });
        let _ = msg.write_varint(if stream_blocked { 1 } else { 0 });

        match cc_obs {
            Some((cc_state, cc_param)) => {
                let _ = msg.write_varint(cc_state);
                let _ = msg.write_varint(cc_param);
            }
            None => {
                let _ = msg.write_varint(0);
                let _ = msg.write_varint(0);
            }
        }

        let _ = msg.write_varint(peak_bw);
        let _ = msg.write_varint(bytes_in_transit);
        let _ = msg.write_varint(if limited { 1 } else { 0 });

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }
}
```
