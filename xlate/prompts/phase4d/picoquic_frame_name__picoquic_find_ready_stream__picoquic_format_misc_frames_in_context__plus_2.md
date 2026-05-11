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

## `picoquic/frame_names.c:picoquic_frame_name`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns "unknown" for the default case, while Rust returns None for unmatched frame types.
* C source: `picoquic/frame_names.c:25-116`
* C signature: `const char * picoquic_frame_name(uint64_t)`
* Rust source: `rs/fq/src/frames.rs:63-108`
* Rust item: `name`

### C body
```c
{
    if ((int)ftype >= picoquic_frame_type_stream_range_min &&
        (int)ftype <= picoquic_frame_type_stream_range_max) {
        return "stream";
    }

    switch (ftype) {
    case picoquic_frame_type_padding:
        return "padding";
    case picoquic_frame_type_reset_stream:
        return "reset_stream";
    case picoquic_frame_type_reset_stream_at:
        return "reset_stream_at";
    case picoquic_frame_type_connection_close:
    case picoquic_frame_type_application_close:
        return "connection_close";
    case picoquic_frame_type_max_data:
        return "max_data";
    case picoquic_frame_type_max_stream_data:
        return "max_stream_data";
    case picoquic_frame_type_max_streams_bidir:
    case picoquic_frame_type_max_streams_unidir:
        return "max_streams";
    case picoquic_frame_type_ping:
        return "ping";
    case picoquic_frame_type_data_blocked:
        return "data_blocked";
    case picoquic_frame_type_stream_data_blocked:
        return "stream_data_blocked";
    case picoquic_frame_type_streams_blocked_bidir:
    case picoquic_frame_type_streams_blocked_unidir:
        return "streams_blocked";
    case picoquic_frame_type_new_connection_id:
        return "new_connection_id";
    case picoquic_frame_type_path_new_connection_id:
        return "path_new_connection_id";
    case picoquic_frame_type_stop_sending:
        return "stop_sending";
    case picoquic_frame_type_ack:
        return "ack";
    case picoquic_frame_type_path_challenge:
        return "path_challenge";
    case picoquic_frame_type_path_response:
        return "path_response";
    case picoquic_frame_type_crypto_hs:
        return "crypto";
    case picoquic_frame_type_new_token:
        return "new_token";
    case picoquic_frame_type_ack_ecn:
        return "ack";
    case picoquic_frame_type_path_ack:
        return "path_ack";
    case picoquic_frame_type_path_ack_ecn:
        return "path_ack";
    case picoquic_frame_type_retire_connection_id:
        return "retire_connection_id";
    case picoquic_frame_type_path_retire_connection_id:
        return "path_retire_connection_id";
    case picoquic_frame_type_handshake_done:
        return "handshake_done";
    case picoquic_frame_type_datagram:
    case picoquic_frame_type_datagram_l:
        return "datagram";
    case picoquic_frame_type_ack_frequency:
        return "ack_frequency";
    case picoquic_frame_type_immediate_ack:
        return "immediate_ack";
    case picoquic_frame_type_time_stamp:
        return "time_stamp";
    case picoquic_frame_type_path_abandon:
        return "path_abandon";
    case picoquic_frame_type_path_backup:
        return "path_backup";
    case picoquic_frame_type_path_available:
        return "path_available";
    case picoquic_frame_type_bdp:
        return "bdp";
    case picoquic_frame_type_max_path_id:
        return "max_path_id";
    case picoquic_frame_type_paths_blocked:
        return "paths_blocked";
    case picoquic_frame_type_path_cid_blocked:
        return "path_cid_blocked";
    case picoquic_frame_type_observed_address_v4:
        return "observed_address_v4";
    case picoquic_frame_type_observed_address_v6:
        return "observed_address_v6";
    default:
        return "unknown";
    }
}
```

### Rust body
```rust
    pub fn name(frame_type: u64) -> Option<&'static str> {
        if frame_type >= FrameType::StreamRangeMin as u64
            && frame_type <= FrameType::StreamRangeMax as u64
        {
            return Some("stream");
        }
        match frame_type {
            0x00 => Some("padding"),
            0x01 => Some("ping"),
            0x02 | 0x03 => Some("ack"),
            0x04 => Some("reset_stream"),
            0x05 => Some("stop_sending"),
            0x06 => Some("crypto"),
            0x07 => Some("new_token"),
            0x10 => Some("max_data"),
            0x11 => Some("max_stream_data"),
            0x12 | 0x13 => Some("max_streams"),
            0x14 => Some("data_blocked"),
            0x15 => Some("stream_data_blocked"),
            0x16 | 0x17 => Some("streams_blocked"),
            0x18 => Some("new_connection_id"),
            0x19 => Some("retire_connection_id"),
            0x1a => Some("path_challenge"),
            0x1b => Some("path_response"),
            0x1c | 0x1d => Some("connection_close"),
            0x1e => Some("handshake_done"),
            0x1f => Some("immediate_ack"),
            0x24 => Some("reset_stream_at"),
            0x30 | 0x31 => Some("datagram"),
            0x3e | 0x3f => Some("path_ack"),
            0xaf => Some("ack_frequency"),
            757 => Some("time_stamp"),
            0x3e75 => Some("path_abandon"),
            0x3e76 => Some("path_backup"),
            0x3e77 => Some("path_available"),
            0x3e78 => Some("path_new_connection_id"),
            0x3e79 => Some("path_retire_connection_id"),
            0x3e7a => Some("max_path_id"),
            0x3e7b => Some("paths_blocked"),
            0x3e7c => Some("path_cid_blocked"),
            0xebd9 => Some("bdp"),
            0x9f81a6 => Some("observed_address_v4"),
            0x9f81a7 => Some("observed_address_v6"),
            _ => None,
        }
    }
```

## `picoquic/frames.c:picoquic_find_ready_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body delegates to find_ready_stream_path with NULL and 0; Rust body implements its own stream scan, so equivalence is not visible from the provided bodies.
* C source: `picoquic/frames.c:1666-1669`
* C signature: `picoquic_stream_head_t * picoquic_find_ready_stream(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:9701-9714`
* Rust item: `find_ready_stream`

### C body
```c
{
    return picoquic_find_ready_stream_path(cnx, NULL, 0);
}
```

### Rust body
```rust
        self.output_streams.iter().copied().find(|tok| {
            self.streams.get(*tok).is_some_and(|stream| {
                let control_frame_ready = (stream.stop_sending_requested
                    && !stream.stop_sending_sent)
                    || (stream.reset_requested && !stream.reset_sent);
                let data_ready = self.maxdata_remote > self.data_sent
                    && stream.sent_offset < stream.maxdata_remote
                    && (stream.is_active
                        || !stream.send_queue.is_empty()
                        || (stream.fin_requested && !stream.fin_sent));
                control_frame_ready || data_ready
            })
        })
```

## `picoquic/frames.c:picoquic_format_misc_frames_in_context`
* Phase 4C status: `suspect`
* Phase 4C rationale: C repeatedly finds the first misc frame matching pc and only ANDs is_pure_ack after bytes advances; Rust only processes while the front frame matches pc and relies on the formatter result.
* C source: `picoquic/frames.c:4860-4880`
* C signature: `uint8_t * picoquic_format_misc_frames_in_context(picoquic_cnx_t *, uint8_t *, uint8_t *, int *, int *, picoquic_packet_context_enum)`
* Rust source: `rs/fq/src/internal.rs:13784-13810`
* Rust item: `format_misc_frames_in_context`

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

## `picoquic/frames.c:picoquic_queue_new_token_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C silently returns 0 if formatting produces no bytes, while Rust returns BufferTooSmall on formatting failure or more_data and then always returns Ok after queueing.
* C source: `picoquic/frames.c:1029-1043`
* C signature: `int picoquic_queue_new_token_frame(picoquic_cnx_t *, uint8_t *, size_t)`
* Rust source: `rs/fq/src/internal.rs:13575-13589`
* Rust item: `queue_new_token_frame`

### C body
```c
{
    int ret = 0;
    int more_data = 0;
    int is_pure_ack = 1;
    uint8_t frame_buffer[258];
    uint8_t* bytes = picoquic_format_new_token_frame(frame_buffer, frame_buffer + sizeof(frame_buffer), &more_data, &is_pure_ack, token, token_length);

    if (bytes > frame_buffer) {
        ret = picoquic_queue_misc_frame(cnx, frame_buffer, bytes - frame_buffer, 1,
            picoquic_packet_context_application);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn queue_new_token_frame(&mut self, token: &[u8]) -> Result<(), crate::Error> {
        let capacity = 1 + encode_varint_length(token.len() as u64) + token.len();
        let mut frame = vec![0u8; capacity];
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let tail_len = format_new_token_frame(&mut frame, &mut more_data, &mut is_pure_ack, token)
            .map(|tail| tail.len())
            .ok_or(crate::Error::BufferTooSmall)?;
        if more_data != 0 {
            return Err(crate::Error::BufferTooSmall);
        }
        frame.truncate(capacity - tail_len);
        encode_misc_frame(self, frame, true, PacketContext::Application);
        Ok(())
    }
```

## `picoquic/logwriter.c:binlog_close_connection`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both log ConnectionClose, flush, and clear the file handle/name, but C also invokes autoqlog_fn and decrements current_number_of_open_logs; Rust comments that these are not done here.
* C source: `picoquic/logwriter.c:1091-1121`
* C signature: `void binlog_close_connection(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/binlog.rs:1041-1530`
* Rust item: `close_connection`

### C body
```c
{
    FILE * f = cnx->f_binlog;
    if (f == NULL) {
        return;
    }

    bytestream_buf stream_msg;
    bytestream * msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);
    /* Common chunk header */
    binlog_compose_event_header(msg, &cnx->initial_cnxid, picoquic_get_quic_time(cnx->quic), 0, picoquic_log_event_connection_close);

    bytestream_buf stream_head;
    bytestream * head = bytestream_buf_init(&stream_head, 8);
    bytewrite_int32(head, (uint32_t)bytestream_length(msg));

    (void)fwrite(bytestream_data(head), bytestream_length(head), 1, f);
    (void)fwrite(bytestream_data(msg), bytestream_length(msg), 1, f);

    fflush(f);

    cnx->f_binlog = picoquic_file_close(cnx->f_binlog);

    if (cnx->quic->qlog_dir != NULL && cnx->quic->autoqlog_fn != NULL) {
        (void)cnx->quic->autoqlog_fn(cnx);
    }
    cnx->binlog_file_name = picoquic_string_free(cnx->binlog_file_name);
    if (cnx->quic->current_number_of_open_logs > 0) {
        cnx->quic->current_number_of_open_logs--;
    }
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
