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

## Pair `picoquic/logwriter.c:picoquic_log_path_abandon_frame`
C: `picoquic/logwriter.c:449-457 picoquic_log_path_abandon_frame`
Rust: `rs/fq/src/binlog.rs:593-601 log_path_abandon_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* frame type as varint */
    bytes = picoquic_skip_path_abandon_frame(bytes, bytes_max); /* skip abandon frame */
    picoquic_binlog_frame(f, bytes_begin, bytes);

    return bytes;
}
```

### Rust body
```rust
fn log_path_abandon_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_erroring_frame`
C: `picoquic/logwriter.c:495-503 picoquic_log_erroring_frame`
Rust: `rs/fq/src/binlog.rs:678-682 log_erroring_frame`

### C body
```c
{
    size_t frame_size = bytes_max - bytes;
    size_t copied = (frame_size > 8) ? 8 : frame_size;

    picoquic_binlog_frame(f, bytes, bytes + copied);

    return NULL;
}
```

### Rust body
```rust
fn log_erroring_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let copied = bytes_in.len().min(8);
    append_frame(out, &bytes_in[..copied]);
    None
}
```

## Pair `picoquic/logwriter.c:picoquic_binlog_frames`
C: `picoquic/logwriter.c:550-677 picoquic_binlog_frames`
Rust: `rs/fq/src/binlog.rs:687-691 binlog_frames`

### C body
```c
{
    const uint8_t* bytes_max = bytes + length;

    while (bytes != NULL && bytes < bytes_max) {
        uint64_t ftype= 0;
        size_t ftype_ll = picoquic_varint_decode(bytes, length, &ftype);

        if (ftype_ll == 0) {
            /* Error, incorrect frame type encoding */
            bytes = NULL;
            break;
        }
        else if (ftype < 64 && ftype_ll != 1) {
            /* Error, incorrect frame type encoding */
            bytes = NULL;
            break;
        }

        if (PICOQUIC_IN_RANGE(ftype, picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max)) {
            bytes = picoquic_log_stream_frame(f, bytes, bytes_max);
            continue;
        }

        switch (ftype) {
        case picoquic_frame_type_ack:
        case picoquic_frame_type_ack_ecn:
        case picoquic_frame_type_path_ack:
        case picoquic_frame_type_path_ack_ecn:
            bytes = picoquic_log_ack_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_retire_connection_id:
            bytes = picoquic_log_retire_connection_id_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_path_retire_connection_id:
            bytes = picoquic_log_path_retire_connection_id_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_padding:
        case picoquic_frame_type_ping:
            bytes = picoquic_log_padding(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_reset_stream:
            bytes = picoquic_log_reset_stream_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_reset_stream_at:
            bytes = picoquic_log_reset_stream_at_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_connection_close:
            bytes = picoquic_log_close_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_application_close:
            bytes = picoquic_log_app_close_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_max_data:
            bytes = picoquic_log_max_data_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_max_stream_data:
            bytes = picoquic_log_max_stream_data_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_max_streams_bidir:
        case picoquic_frame_type_max_streams_unidir:
            bytes = picoquic_log_max_stream_id_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_data_blocked:
            bytes = picoquic_log_blocked_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_stream_data_blocked:
            bytes = picoquic_log_stream_blocked_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_streams_blocked_bidir:
        case picoquic_frame_type_streams_blocked_unidir:
            bytes = picoquic_log_streams_blocked_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_new_connection_id:
            bytes = picoquic_log_new_connection_id_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_path_new_connection_id:
            bytes = picoquic_log_path_new_connection_id_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_stop_sending:
            bytes = picoquic_log_stop_sending_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_path_challenge:
        case picoquic_frame_type_path_response:
            bytes = picoquic_log_path_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_crypto_hs:
            bytes = picoquic_log_crypto_hs_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_new_token:
            bytes = picoquic_log_new_token_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_handshake_done:
            bytes = picoquic_log_handshake_done_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_datagram:
        case picoquic_frame_type_datagram_l:
            bytes = picoquic_log_datagram_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_ack_frequency:
            bytes = picoquic_log_ack_frequency_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_immediate_ack:
            bytes = picoquic_log_immediate_ack_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_time_stamp:
            bytes = picoquic_log_time_stamp_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_path_abandon:
            bytes = picoquic_log_path_abandon_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_path_backup:
        case picoquic_frame_type_path_available:
            bytes = picoquic_log_path_available_or_backup_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_bdp:
            bytes = picoquic_log_bdp_frame(f, bytes, bytes_max);
            break;
        case picoquic_frame_type_observed_address_v4:
        case picoquic_frame_type_observed_address_v6:
            bytes = picoquic_log_observed_address_frame(f, bytes, bytes_max, ftype);
            break;
        default:
            bytes = picoquic_log_erroring_frame(f, bytes, bytes_max);
            break;
        }
    }
}
```

### Rust body
```rust
        if bytes.is_empty() {
            return;
        }
```

## Pair `picoquic/logwriter.c:binlog_pdu_ex`
C: `picoquic/logwriter.c:724-732 binlog_pdu_ex`
Rust: `rs/fq/src/logger.rs:456-468 pdu`

### C body
```c
{
    if (cnx != NULL && cnx->f_binlog != NULL && picoquic_cnx_is_still_logging(cnx)) {
        binlog_pdu(cnx->f_binlog, &cnx->initial_cnxid, receiving, current_time, addr_peer, addr_local, packet_length,
            unique_path_id, ecn);
    }
}
```

### Rust body
```rust
        if !self.is_still_logging() {
            return;
        }
```

## Pair `picoquic/logwriter.c:binlog_buffered_packet`
C: `picoquic/logwriter.c:820-838 binlog_buffered_packet`
Rust: `rs/fq/src/binlog.rs:969-1530 buffered_packet`

### C body
```c
{
    FILE* f = cnx->f_binlog;
    bytestream_buf stream_msg;
    bytestream* msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);

    bytewrite_int32(msg, 0);
    /* Common chunk header */
    binlog_compose_event_header(msg, &cnx->initial_cnxid, current_time, binlog_get_path_id(cnx, path_x),
        picoquic_log_event_packet_buffered);
    /* Event header */
    bytewrite_vint(msg, ptype);
    (void)bytewrite_cstr(msg, "keys_unavailable");

    /* write the frame length at the reserved spot, and save to log file*/
    picoformat_32(msg->data, (uint32_t)(msg->ptr - 4));
    (void)fwrite(bytestream_data(msg), bytestream_length(msg), 1, f);
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

## Pair `picoquic/logwriter.c:binlog_transport_extension`
C: `picoquic/logwriter.c:959-982 binlog_transport_extension`
Rust: `rs/fq/src/binlog.rs:1020-1530 transport_extension`

### C body
```c
{
    FILE* f = cnx->f_binlog;

    bytestream_buf stream_msg;
    bytestream* msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);
    /* Common chunk header */
    binlog_compose_event_header(msg, &cnx->initial_cnxid, picoquic_get_quic_time(cnx->quic), 0, picoquic_log_event_param_update);
    /* Event header */
    bytewrite_vint(msg, is_local);
    bytewrite_vint(msg, param_length);

    if (param_length > 0) {
        bytewrite_buffer(msg, params, param_length);
    }

    bytestream_buf stream_head;
    bytestream* head = bytestream_buf_init(&stream_head, 4);
    bytewrite_int32(head, (uint32_t)bytestream_length(msg));

    (void)fwrite(bytestream_data(head), bytestream_length(head), 1, f);
    (void)fwrite(bytestream_data(msg), bytestream_length(msg), 1, f);
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

## Pair `picoquic/logwriter.c:binlog_close_connection`
C: `picoquic/logwriter.c:1091-1121 binlog_close_connection`
Rust: `rs/fq/src/binlog.rs:1041-1530 close_connection`

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

## Pair `picoquic/logwriter.c:binlog_app_message`
C: `picoquic/logwriter.c:1300-1307 binlog_app_message`
Rust: `rs/fq/src/binlog.rs:1537-1540 binlog_app_message`

### C body
```c
{
    if (cnx->f_binlog != NULL) {
        picoquic_binlog_message_v(cnx, fmt, vargs);
    }
}
```

### Rust body
```rust
    if connection.f_binlog.is_some() {
        Binlog::message_v(connection, args);
    }
```

## Pair `picoquic/loss_recovery.c:picoquic_retransmit_needed`
C: `picoquic/loss_recovery.c:173-201 picoquic_retransmit_needed`
Rust: `rs/fq/src/internal.rs:11861-11909 retransmit_needed`

### C body
```c
{
    size_t length = 0;

    if (pc == picoquic_packet_context_application && cnx->is_multipath_enabled) {
        /* If unique multipath is enabled, should check for retransmission on all paths */
        for (int i=0; i<cnx->nb_paths; i++) {
            if (length == 0) {
                length = picoquic_retransmit_needed_loop(cnx, &cnx->path[i]->pkt_ctx, pc, path_x, current_time,
                    next_wake_time, packet, send_buffer_max, header_length);
            }
            else {
                /* If more retransmission are queued, set the timer appropriately */
                if (cnx->path[i]->pkt_ctx.pending_first != NULL) {
                    picoquic_set_wake_up_from_packet_retransmit(cnx, cnx->path[i]->pkt_ctx.pending_first, current_time, next_wake_time);
                }
            }
        }
    }
    else {
        length = picoquic_retransmit_needed_loop(cnx, &cnx->pkt_ctx[pc], pc, path_x, current_time, next_wake_time,
            packet, send_buffer_max, header_length);
    }

    return (int)length;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut length = 0usize;

        if pc == PacketContext::Application && self.is_multipath_enabled {
            for path_idx in 0..self.paths.len() {
                let selection = PacketContextSelection::Path(path_idx);
                if length == 0 {
                    length = self.retransmit_needed_loop(
                        selection,
                        pc,
                        path_x,
                        current_time,
                        next_wake_time,
                        packet,
                        send_buffer_max,
                        header_length,
                    );
                } else if let Some(old_token) = self.pending_first_token(selection) {
                    self.set_wake_up_from_packet_retransmit(
                        old_token,
                        current_time,
                        next_wake_time,
                    );
                }
            }
        } else {
            length = self.retransmit_needed_loop(
                PacketContextSelection::Connection(pc),
                pc,
                path_x,
                current_time,
                next_wake_time,
                packet,
                send_buffer_max,
                header_length,
            );
        }

        length as i32
    }
```

## Pair `picoquic/loss_recovery.c:picoquic_set_wake_up_from_packet_retransmit`
C: `picoquic/loss_recovery.c:646-662 picoquic_set_wake_up_from_packet_retransmit`
Rust: `rs/fq/src/internal.rs:11537-11549 set_wake_up_from_packet_retransmit`

### C body
```c
{
    uint64_t next_retransmit_time = *next_wake_time;
    int is_timer_expired = 0;
    int is_probably_lost = picoquic_is_packet_probably_lost(cnx, old_p, current_time, &next_retransmit_time,
        &is_timer_expired);

    if (is_probably_lost || is_timer_expired) {
        *next_wake_time = current_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_LOSS_RECOVERY);
    }
    else if (next_retransmit_time < *next_wake_time) {
        *next_wake_time = next_retransmit_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_LOSS_RECOVERY);
    }
}
```

### Rust body
```rust
        else {
            return;
        };
```

## Pair `picoquic/loss_recovery.c:picoquic_count_and_notify_loss`
C: `picoquic/loss_recovery.c:895-932 picoquic_count_and_notify_loss`
Rust: `rs/fq/src/internal.rs:11117-11219 count_and_notify_loss`

### C body
```c
{
    if (timer_based_retransmit < 2) {
        picoquic_log_packet_lost(cnx, old_p->send_path, old_p->ptype, old_p->sequence_number,
            (timer_based_retransmit) ? "timer" : "repeat",
            (old_p->send_path == NULL || old_p->send_path->first_tuple->p_remote_cnxid == NULL) ? NULL : &old_p->send_path->first_tuple->p_remote_cnxid->cnx_id,
            old_p->length, current_time);

        if (!old_p->is_preemptive_repeat) {
            cnx->nb_retransmission_total++;
        }
    }

    if (old_p->send_path != NULL) {
        old_p->send_path->nb_losses_found++;
        if (timer_based_retransmit) {
            old_p->send_path->nb_timer_losses++;
        }
        if ((old_p->send_path->smoothed_rtt != PICOQUIC_INITIAL_RTT ||
            old_p->send_path->rtt_variant != 0) &&
            old_p->send_time > cnx->start_time + old_p->send_path->smoothed_rtt) {
            /* we do not count losses occruring before ready state, because the 
             * timers are not reliable yet */
            old_p->send_path->total_bytes_lost += old_p->length;
        }

        if (cnx->congestion_alg != NULL && cnx->cnx_state >= picoquic_state_ready && old_p->send_path != NULL) {
            picoquic_per_ack_state_t ack_state = { 0 };
            ack_state.pc = old_p->pc;
            ack_state.lost_packet_number = old_p->sequence_number;
            ack_state.nb_bytes_newly_lost = old_p->length;
            cnx->congestion_alg->alg_notify(cnx, old_p->send_path,
                (timer_based_retransmit == 0) ? picoquic_congestion_notification_repeat : picoquic_congestion_notification_timeout,
                &ack_state, current_time);
        }
    }
}
```

### Rust body
```rust
    ) {
        let path_idx = old_p
            .send_path
            .and_then(|send_path| self.path_index_from_token(send_path));
        let dcid = path_idx.and_then(|idx| {
            let path = &self.paths[idx];
            let cid_idx = path
                .tuples
                .first()
                .and_then(|tuple| tuple.remote_connection_id_index)?;
            self.remote_connection_id_stashes
                .iter()
                .find(|stash| stash.unique_path_id == path.unique_path_id)
                .and_then(|stash| stash.connection_ids.get(cid_idx))
                .map(|remote| remote.connection_id)
        });

        if let Some(idx) = path_idx {
            let mut path = self.paths.remove(idx);
            if timer_based_retransmit < 2 {
                crate::logger::Log::packet_lost(
                    self,
                    &mut path,
                    old_p.packet_type,
                    old_p.sequence_number,
                    if timer_based_retransmit != 0 {
                        "timer"
                    } else {
                        "repeat"
                    },
                    dcid.as_ref(),
                    old_p.length,
                    current_time,
                );
            }

            path.nb_losses_found = path.nb_losses_found.saturating_add(1);
            if timer_based_retransmit != 0 {
                path.nb_timer_losses = path.nb_timer_losses.saturating_add(1);
            }
            if (path.smoothed_rtt != INITIAL_RTT || path.rtt_variant.ticks() != 0)
                && old_p.send_time
                    > Instant::from_ticks(
                        self.start_time
                            .ticks()
                            .saturating_add(path.smoothed_rtt.ticks()),
                    )
            {
                path.total_bytes_lost = path.total_bytes_lost.saturating_add(old_p.length as u64);
            }

            let mut cc_notified = false;
            if let Some(cc_alg) = self.congestion_alg
                && self.connection_state >= State::Ready
            {
                let ack_state = PerAckState {
                    pc: old_p.packet_context as i32,
                    lost_packet_number: old_p.sequence_number,
                    nb_bytes_newly_lost: old_p.length as u64,
                    ..PerAckState::default()
                };
                cc_alg.algorithm.alg_notify(
                    self,
                    &mut path,
                    if timer_based_retransmit == 0 {
                        CongestionNotification::Repeat
                    } else {
                        CongestionNotification::Timeout
                    },
                    &ack_state,
                    current_time,
                );
                cc_notified = true;
            }
            self.paths.insert(idx, path);
            if cc_notified {
                self.report_pacing_update(idx);
            }
        } else if timer_based_retransmit < 2 {
            crate::logger::Log::app_message(
                self,
                format_args!(
                    "Packet lost, type {:?}, seq {}, trigger {}",
                    old_p.packet_type,
                    old_p.sequence_number,
                    if timer_based_retransmit != 0 {
                        "timer"
                    } else {
                        "repeat"
                    }
                ),
            );
        }

        if timer_based_retransmit < 2 && !old_p.is_preemptive_repeat {
            self.nb_retransmission_total = self.nb_retransmission_total.saturating_add(1);
        }
    }
```

## Pair `picoquic/loss_recovery.c:picoquic_queue_retransmit_on_ack`
C: `picoquic/loss_recovery.c:1029-1074 picoquic_queue_retransmit_on_ack`
Rust: `rs/fq/src/internal.rs:4751-4753 queue_retransmit_on_ack`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = NULL;
    picoquic_packet_t* old_p;
    uint64_t next_retransmit_time = UINT64_MAX;

    /* If multipath, pick the packet context associated with the current path,
     * else, pick the default 1RTT context */
    if (cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }
    else {
        pkt_ctx = &cnx->pkt_ctx[picoquic_packet_context_application];
    }
    /* For all packets in this context:
    * Check whether it needs retransmit.
    * if yes, call "picoquic_process_lost_packet".
    * else, stop. */
    old_p = pkt_ctx->pending_first;

    /* Call the per packet routine in a loop */
    while (old_p != NULL) {
        picoquic_packet_t* p_next = old_p->packet_next;
        int packet_is_pure_ack = 0;
        size_t header_length = 0;
        int is_timer_expired = 0;
        int is_probably_lost = cnx->initial_repeat_needed || old_p->send_path == NULL ||
            picoquic_is_packet_probably_lost(cnx, old_p, current_time, &next_retransmit_time, &is_timer_expired);
        size_t length = 0;

        if (is_probably_lost) {
            if (old_p->is_ack_trap) {
                picoquic_dequeue_retransmit_packet(cnx, pkt_ctx, old_p, 1, 0);
            }
            else {
                (void)picoquic_process_lost_packet(cnx, pkt_ctx, old_p, is_timer_expired, old_p->pc,
                    path_x, current_time, NULL, 0, &length, &packet_is_pure_ack, &header_length);
            }
            old_p = p_next;
        }
        else {
            break;
        }   
    }
}
```

### Rust body
```rust
    pub fn queue_retransmit_on_ack(&mut self, path_x: &mut Path, current_time: Instant) {
        self.retransmit_demoted_path(path_x, current_time);
    }
```

## Pair `picoquic/newreno.c:picoquic_newreno_sim_notify`
C: `picoquic/newreno.c:88-171 picoquic_newreno_sim_notify`
Rust: `rs/fq/src/cc_common.rs:462-528 notify`

### C body
```c
{
    switch (notification) {
    case picoquic_congestion_notification_acknowledgement: {
        switch (nr_state->alg_state) {
        case picoquic_newreno_alg_slow_start:
            /* TODO discuss app limited for pure reno too? */
            /* following tests will fail:
             * memlog keylog_test packet_trace ready_to_send ready_to_skip ready_to_zfin ready_to_zero pacing_update
             * quality_update multipath_callback multipath_quality multipath_stream_af
             */
            nr_state->cwin += ack_state->nb_bytes_acknowledged;
            /* nr_state->cwin += picoquic_cc_slow_start_increase(path_x, ack_state->nb_bytes_acknowledged); */

            /* if cnx->cwin exceeds SSTHRESH, exit and go to CA */
            if (nr_state->cwin >= nr_state->ssthresh) {
                nr_state->alg_state = picoquic_newreno_alg_congestion_avoidance;
            }
            break;
        case picoquic_newreno_alg_congestion_avoidance: {
            uint64_t complete_delta = ack_state->nb_bytes_acknowledged * path_x->send_mtu + nr_state->residual_ack;
            nr_state->residual_ack = complete_delta % nr_state->cwin;
            nr_state->cwin += complete_delta / nr_state->cwin;
            break;
        }
        }
        break;
    }
    case picoquic_congestion_notification_ecn_ec:
    case picoquic_congestion_notification_repeat:
    case picoquic_congestion_notification_timeout:
        /* if the loss happened in this period, enter recovery */
        if (nr_state->recovery_sequence <= ack_state->lost_packet_number) {
        /* if (nr_state->recovery_sequence <= ack_state->lost_packet_number) { */
            picoquic_newreno_sim_enter_recovery(nr_state, cnx, path_x, notification, current_time);
        }
        break;
    case picoquic_congestion_notification_spurious_repeat:
        if (!cnx->is_multipath_enabled) {
            if (current_time - nr_state->recovery_start < path_x->smoothed_rtt &&
                nr_state->recovery_sequence > picoquic_cc_get_ack_number(cnx, path_x)) {
                /* If spurious repeat of initial loss detected,
                 * exit recovery and reset threshold to pre-entry cwin.
                 */
                if (nr_state->ssthresh != UINT64_MAX &&
                    nr_state->cwin < 2 * nr_state->ssthresh) {
                    nr_state->cwin = 2 * nr_state->ssthresh;
                    nr_state->alg_state = picoquic_newreno_alg_congestion_avoidance;
                }
            }
        }
        else {
            if (current_time - nr_state->recovery_start < path_x->smoothed_rtt &&
                nr_state->recovery_start > picoquic_cc_get_ack_sent_time(cnx, path_x)) {
                /* If spurious repeat of initial loss detected,
                 * exit recovery and reset threshold to pre-entry cwin.
                 */
                if (nr_state->ssthresh != UINT64_MAX &&
                    nr_state->cwin < 2 * nr_state->ssthresh) {
                    nr_state->cwin = 2 * nr_state->ssthresh;
                    nr_state->alg_state = picoquic_newreno_alg_congestion_avoidance;
                }
            }
        }
        break;
    case picoquic_congestion_notification_reset:
        picoquic_newreno_sim_reset(nr_state);
        break;
    case picoquic_congestion_notification_seed_cwin:
        picoquic_newreno_sim_seed_cwin(nr_state, ack_state->nb_bytes_acknowledged);
        break;
    default:
        /* ignore */
        break;
    }
}
```

### Rust body
```rust
    ) {
        match notification {
            CongestionNotification::Acknowledgement => match self.alg_state {
                NewRenoAlgState::SlowStart => {
                    self.cwin += ack_state.nb_bytes_acknowledged;
                    if self.cwin >= self.ssthresh {
                        self.alg_state = NewRenoAlgState::CongestionAvoidance;
                    }
                }
                NewRenoAlgState::CongestionAvoidance => {
                    let complete_delta = ack_state.nb_bytes_acknowledged * path_x.send_mtu as u64
                        + self.residual_ack;
                    self.residual_ack = complete_delta % self.cwin;
                    self.cwin += complete_delta / self.cwin;
                }
            },
            CongestionNotification::EcnEc
            | CongestionNotification::Repeat
            | CongestionNotification::Timeout
                if self.recovery_sequence <= ack_state.lost_packet_number =>
            {
                crate::newreno::picoquic_newreno_sim_enter_recovery(
                    self,
                    connection,
                    path_x,
                    notification,
                    current_time,
                );
            }
            CongestionNotification::EcnEc
            | CongestionNotification::Repeat
            | CongestionNotification::Timeout => {}
            CongestionNotification::SpuriousRepeat => {
                if !connection.is_multipath_enabled {
                    if current_time.ticks() - self.recovery_start < path_x.smoothed_rtt.ticks()
                        && self.recovery_sequence > connection.ack_number(path_x)
                        && self.ssthresh != u64::MAX
                        && self.cwin < 2 * self.ssthresh
                    {
                        self.cwin = 2 * self.ssthresh;
                        self.alg_state = NewRenoAlgState::CongestionAvoidance;
                    }
                } else if current_time.ticks() - self.recovery_start < path_x.smoothed_rtt.ticks()
                    && self.recovery_start > connection.ack_sent_time(path_x).ticks()
                    && self.ssthresh != u64::MAX
                    && self.cwin < 2 * self.ssthresh
                {
                    self.cwin = 2 * self.ssthresh;
                    self.alg_state = NewRenoAlgState::CongestionAvoidance;
                }
            }
            CongestionNotification::Reset => {
                self.reset();
            }
            CongestionNotification::SeedCwin => {
                self.seed_cwin(ack_state.nb_bytes_acknowledged);
            }
            _ => {}
        }
    }
```

## Pair `picoquic/newreno.c:picoquic_newreno_delete`
C: `picoquic/newreno.c:298-305 picoquic_newreno_delete`
Rust: `rs/fq/src/newreno.rs:221-231 alg_delete`

### C body
```c
{
    if (path_x->congestion_alg_state != NULL) {
        free(path_x->congestion_alg_state);
        path_x->congestion_alg_state = NULL;
    }
}
```

### Rust body
```rust
    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|state| state.downcast_ref::<NewrenoState>())
            .map(NewrenoState::observe)
    }
```

## Pair `picoquic/pacing.c:picoquic_is_pacing_blocked`
C: `picoquic/pacing.c:54-60 picoquic_is_pacing_blocked`
Rust: `rs/fq/src/internal.rs:5924-5927 is_blocked`

### C body
```c
{
    return (pacing->bucket_nanosec < pacing->packet_time_nanosec);
}
```

### Rust body
```rust
    pub fn is_blocked(&self) -> bool {
        // C: picoquic_is_pacing_blocked
        self.bucket_nanosec < self.packet_time_nanosec
    }
```

## Pair `picoquic/pacing.c:picoquic_update_pacing_window`
C: `picoquic/pacing.c:187-233 picoquic_update_pacing_window`
Rust: `rs/fq/src/internal.rs:6011-6057 update_window`

### C body
```c
{
    uint64_t rtt_nanosec = smoothed_rtt * 1000;

    if ((cwin < ((uint64_t)send_mtu) * 8) || rtt_nanosec <= 1000) {
        /* Small windows, should only relie on ACK clocking */
        pacing->bucket_max = rtt_nanosec;
        pacing->packet_time_nanosec = 1;
        pacing->packet_time_microsec = 1;

        if (pacing->bucket_nanosec > pacing->bucket_max) {
            pacing->bucket_nanosec = pacing->bucket_max;
        }
    }
    else {
        double pacing_rate = ((double)cwin / (double)rtt_nanosec) * 1000000000.0;
        uint64_t quantum = cwin / 4;

        if (quantum < 2ull * send_mtu) {
            quantum = 2ull * send_mtu;
        }
        else {
            if (slow_start && smoothed_rtt > 4*PICOQUIC_MAX_BANDWIDTH_TIME_INTERVAL_MAX) {
                const uint64_t quantum_min = 0x8000;
                if (quantum  < quantum_min){
                    quantum = quantum_min;
                }
                else {
                    uint64_t quantum2 = (uint64_t)((pacing_rate * PICOQUIC_MAX_BANDWIDTH_TIME_INTERVAL_MAX) / 1000000.0);
                    if (quantum2 > quantum) {
                        quantum = quantum2;
                    }
                }
            }
            else if (quantum > 16ull * send_mtu) {
                quantum = 16ull * send_mtu;
            }

        }

        if (slow_start) {
            pacing_rate *= 1.25;
        }
        picoquic_update_pacing_parameters(pacing, pacing_rate, quantum, send_mtu, smoothed_rtt, signalled_path);
    }
}
```

### Rust body
```rust
    ) {
        // C: picoquic_update_pacing_window
        let rtt_nanosec = smoothed_rtt.ticks().saturating_mul(1000);
        if cwin < (send_mtu as u64).saturating_mul(8) || rtt_nanosec <= 1000 {
            self.bucket_max = rtt_nanosec as i64;
            self.packet_time_nanosec = 1;
            self.packet_time_microsec = crate::Duration::from_ticks(1);
            if self.bucket_nanosec > self.bucket_max {
                self.bucket_nanosec = self.bucket_max;
            }
        } else {
            let mut pacing_rate = cwin as f64 / rtt_nanosec as f64 * 1_000_000_000.0;
            let mut quantum = cwin / 4;
            let two_mtu = (2 * send_mtu) as u64;
            if quantum < two_mtu {
                quantum = two_mtu;
            } else if slow_start != 0 && smoothed_rtt.ticks() > 4 * MAX_BANDWIDTH_TIME_INTERVAL_MAX
            {
                const QUANTUM_MIN: u64 = 0x8000;
                if quantum < QUANTUM_MIN {
                    quantum = QUANTUM_MIN;
                } else {
                    let quantum2 =
                        (pacing_rate * MAX_BANDWIDTH_TIME_INTERVAL_MAX as f64 / 1_000_000.0) as u64;
                    if quantum2 > quantum {
                        quantum = quantum2;
                    }
                }
            } else {
                let max_quantum = (16 * send_mtu) as u64;
                if quantum > max_quantum {
                    quantum = max_quantum;
                }
            }
            if slow_start != 0 {
                pacing_rate *= 1.25;
            }
            self.update_parameters(pacing_rate, quantum, send_mtu, smoothed_rtt, signalled_path);
        }
    }
```

## Pair `picoquic/pacing.c:picoquic_update_pacing_rate`
C: `picoquic/pacing.c:259-264 picoquic_update_pacing_rate`
Rust: `rs/fq/src/internal.rs:6093-6096 update_pacing_rate`

### C body
```c
{
    picoquic_update_pacing_parameters(&path_x->pacing, pacing_rate,
        quantum, path_x->send_mtu, path_x->smoothed_rtt, path_x);
}
```

### Rust body
```rust
    pub fn update_pacing_rate(&mut self, pacing_rate: f64, quantum: u64) {
        self.pacing
            .update_parameters(pacing_rate, quantum, self.send_mtu, self.smoothed_rtt, None);
    }
```

## Pair `picoquic/packet.c:picoquic_parse_long_packet_header`
C: `picoquic/packet.c:210-394 picoquic_parse_long_packet_header`
Rust: `rs/fq/src/internal.rs:6602-6612 parse_long_packet_header_inner`

### C body
```c
{
    int ret = 0;

    const uint8_t* bytes_start = bytes;
    const uint8_t* bytes_max = bytes + length;
    uint8_t flags = 0;

    if ((bytes = picoquic_frames_uint8_decode(bytes, bytes_max, &flags)) == NULL ||
        (bytes = picoquic_frames_uint32_decode(bytes, bytes_max, &ph->vn)) == NULL)
    {
        ret = -1;
    }
    else if (ph->vn != 0) {
        ph->version_index = picoquic_get_version_index(ph->vn);
        if (ph->version_index < 0) {
            DBG_PRINTF("Version is not recognized: 0x%08x\n", ph->vn);
            ph->ptype = picoquic_packet_error;
            ph->pc = 0;
            ret = PICOQUIC_ERROR_VERSION_NOT_SUPPORTED;
        }
    }
    
    if (ret == 0 && (
        (bytes = picoquic_frames_cid_decode(bytes, bytes_max, &ph->dest_cnx_id)) == NULL ||
        (bytes = picoquic_frames_cid_decode(bytes, bytes_max, &ph->srce_cnx_id)) == NULL)) {
        ret = -1;
    }

    if (ret == 0) {
        ph->offset = bytes - bytes_start;

        if (ph->vn == 0) {
            /* VN = zero identifies a version negotiation packet */
            ph->ptype = picoquic_packet_version_negotiation;
            ph->pc = picoquic_packet_context_initial;
            ph->payload_length = (uint16_t)((length > ph->offset) ? length - ph->offset : 0);
            ph->pl_val = ph->payload_length; /* saving the value found in the packet */

            if (*pcnx == NULL && quic != NULL) {
                /* The version negotiation should always include the cnx-id sent by the client */
                if (quic->local_cnxid_length == 0) {
                    *pcnx = picoquic_cnx_by_net(quic, addr_from);
                }
                else if (ph->dest_cnx_id.id_len == quic->local_cnxid_length) {
                    *pcnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
                }
            }
        }
        else {
            size_t payload_length = 0;
            /* If the version is supported now, the format field in the version table
            * describes the encoding. */
            ph->spin = 0;
            ph->has_spin_bit = 0;
            ph->quic_bit_is_zero = (flags & 0x40) == 0;

            /* The first byte is defined in RFC 9000 as:
             *     Header Form (1) = 1,
             *     Fixed Bit (1) = 1,
             *     Long Packet Type (2),
             *     Type-Specific Bits (4)
             * The packet type is version dependent. In fact, the whole first byte is version
             * dependent, the invariant draft only specifies the "header form" bit = 1 for long
             * header. In version 1, the packet specific bytes are two reserved bytes +
             * sequence number length. We assume the same for version 2.
             */
            ph->ptype = picoquic_parse_long_packet_type(flags, ph->version_index);
            switch (ph->ptype) {
            case picoquic_packet_initial: /* Initial */
            {
                /* special case of the initial packets. They contain a retry token between the header
                * and the encrypted payload */
                size_t tok_len = 0;
                bytes = picoquic_frames_varlen_decode(bytes, bytes_max, &tok_len);

                size_t bytes_left = bytes_max - bytes;

                ph->epoch = picoquic_epoch_initial;
                if (bytes == NULL || bytes_left < tok_len) {
                    /* packet is malformed */
                    ph->ptype = picoquic_packet_error;
                    ph->pc = 0;
                    ph->offset = length;
                }
                else {
                    ph->pc = picoquic_packet_context_initial;
                    ph->token_length = tok_len;
                    ph->token_bytes = bytes;
                    bytes += tok_len;
                    ph->offset = bytes - bytes_start;
                }

                break;
            }
            case picoquic_packet_0rtt_protected: /* 0-RTT Protected */
                ph->pc = picoquic_packet_context_application;
                ph->epoch = picoquic_epoch_0rtt;
                break;
            case picoquic_packet_handshake: /* Handshake */
                ph->pc = picoquic_packet_context_handshake;
                ph->epoch = picoquic_epoch_handshake;
                break;
            case picoquic_packet_retry: /* Retry */
            default:
                /* No default branch in this statement, because there are only 4 possible types
                 * parsed in picoquic_parse_long_packet_type */
                ph->pc = picoquic_packet_context_initial;
                ph->epoch = picoquic_epoch_initial;
                break;
            }

            if (ph->ptype == picoquic_packet_retry) {
                /* No segment length or sequence number in retry packets */
                if (length > ph->offset) {
                    payload_length = length - ph->offset;
                }
                else {
                    payload_length = 0;
                    ph->ptype = picoquic_packet_error;
                }
            }
            else if (ph->ptype != picoquic_packet_error) {
                bytes = picoquic_frames_varlen_decode(bytes, bytes_max, &payload_length);

                size_t bytes_left = (bytes_max > bytes) ? bytes_max - bytes : 0;
                if (bytes == NULL || bytes_left < payload_length || ph->version_index < 0) {
                    ph->ptype = picoquic_packet_error;
                    ph->payload_length = (uint16_t)((length > ph->offset) ? length - ph->offset : 0);
                    ph->pl_val = ph->payload_length;
                }
            }

            if (ph->ptype != picoquic_packet_error)
            {
                ph->pl_val = (uint16_t)payload_length;
                ph->payload_length = (uint16_t)payload_length;
                ph->offset = bytes - bytes_start;
                ph->pn_offset = ph->offset;

                /* Retrieve the connection context */
                if (*pcnx == NULL) {
                    if (quic->local_cnxid_length == 0) {
                        *pcnx = picoquic_cnx_by_net(quic, addr_from);
                    }
                    else
                    {
                        if (ph->dest_cnx_id.id_len == quic->local_cnxid_length) {
                            *pcnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
                        }

                        if (*pcnx == NULL && (ph->ptype == picoquic_packet_initial || ph->ptype == picoquic_packet_0rtt_protected)) {
                            *pcnx = picoquic_cnx_by_icid(quic, &ph->dest_cnx_id, addr_from);
                        }
                        else if (*pcnx == NULL) {
                            DBG_PRINTF("Dropped packet of type %d, no connection", ph->ptype);
                        }
                    }
                }

                if (ph->quic_bit_is_zero && *pcnx != NULL && !(*pcnx)->local_parameters.do_grease_quic_bit) {
                    ph->ptype = picoquic_packet_error;
                }
            }
            else {
                /* Try to find the connection context, for logging purpose. */
                if (*pcnx == NULL) {
                    if (quic->local_cnxid_length == 0) {
                        *pcnx = picoquic_cnx_by_net(quic, addr_from);
                    }
                    else if (ph->dest_cnx_id.id_len == quic->local_cnxid_length) {
                        *pcnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
                    }
                }
            }
        }
    }
    return ret;
}
```

### Rust body
```rust
        if bytes.len() < 5 {
            ph.packet_type = PacketType::Error;
            return None;
        }
```

## Pair `picoquic/packet.c:picoquic_remove_header_protection_inner`
C: `picoquic/packet.c:526-615 picoquic_remove_header_protection_inner`
Rust: `rs/fq/src/internal.rs:7079-7128 remove_header_protection_inner`

### C body
```c
{
    int ret = 0;

    if (pn_enc != NULL)
    {
        /* The header length is not yet known, will only be known after the sequence number is decrypted */
        size_t mask_length = 5;
        size_t sample_offset = ph->pn_offset + 4;
        size_t sample_size = picoquic_pn_iv_size(pn_enc);
        uint8_t mask_bytes[5] = { 0, 0, 0, 0, 0 };

        if (sample_offset + sample_size > length)
        {
            /* return an error */
            /* Invalid packet format. Avoid crash! */
            ph->pn = 0xFFFFFFFF;
            ph->pnmask = 0xFFFFFFFF00000000ull;
            ph->offset = ph->pn_offset;

            DBG_PRINTF("Invalid packet length, type: %d, epoch: %d, pc: %d, pn-offset: %d, length: %d\n",
                ph->ptype, ph->epoch, ph->pc, (int)ph->pn_offset, (int)length);
        }
        else
        {   /* Decode */
            uint8_t first_byte = bytes[0];
            uint8_t first_mask = ((first_byte & 0x80) == 0x80) ? 0x0F : (is_loss_bit_enabled_incoming)?0x07:0x1F;
            uint8_t pn_l;
            uint32_t pn_val = 0;

            memcpy(decrypted_bytes, bytes, ph->pn_offset);
            picoquic_pn_encrypt(pn_enc, bytes + sample_offset, mask_bytes, mask_bytes, mask_length);
            /* Decode the first byte */
            first_byte ^= (mask_bytes[0] & first_mask);
            pn_l = (first_byte & 3) + 1;
            ph->pnmask = (0xFFFFFFFFFFFFFFFFull);
            decrypted_bytes[0] = first_byte;

            /* Packet encoding is 1 to 4 bytes */
            for (uint8_t i = 1; i <= pn_l; i++) {
                pn_val <<= 8;
                decrypted_bytes[ph->offset] = bytes[ph->offset]^mask_bytes[i];
                pn_val += decrypted_bytes[ph->offset++];
                ph->pnmask <<= 8;
            }

            ph->pn = pn_val;
            ph->payload_length -= pn_l;
            /* Only set the key phase byte if short header */
            if (ph->ptype == picoquic_packet_1rtt_protected) {
                ph->key_phase = ((first_byte >> 2) & 1);
            }

            /* Build a packet number to 64 bits */
            ph->pn64 = picoquic_get_packet_number64(sack_list_last, ph->pnmask, ph->pn);

            /* Check the reserved bits */
            if ((first_byte & 0x80) == 0) {
                ph->has_reserved_bit_set = !is_loss_bit_enabled_incoming && (first_byte & 0x18) != 0;
            }
            else{
                ph->has_reserved_bit_set = (first_byte & 0x0c) != 0;
            }
        }
    }
    else {
        /* The pn_enc algorithm was not initialized. Avoid crash! */
        ph->pn = 0xFFFFFFFF;
        ph->pnmask = 0xFFFFFFFF00000000ull;
        ph->offset = ph->pn_offset;
        ph->pn64 = 0xFFFFFFFFFFFFFFFFull;

        DBG_PRINTF("PN dec not ready, type: %d, epoch: %d, pc: %d, pn: %d\n",
            ph->ptype, ph->epoch, ph->pc, (int)ph->pn);

        ret = PICOQUIC_ERROR_AEAD_NOT_READY;
    }

    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let length = length.min(bytes.len());
    if ph.packet_number_offset >= length {
        return -1;
    }
    let sample_offset = ph.packet_number_offset.saturating_add(4);
    if sample_offset + 16 > length {
        return -1;
    }
    let mut sample = [0u8; 16];
    sample.copy_from_slice(&bytes[sample_offset..sample_offset + 16]);
    let mask = pn_enc.mask(sample);
    let first_mask = if (bytes[0] & 0x80) != 0 { 0x0f } else { 0x1f };
    bytes[0] ^= mask[0] & first_mask;

    if is_loss_bit_enabled_incoming && (bytes[0] & 0x80) == 0 {
        ph.has_loss_bits = true;
        ph.loss_bit_l = (bytes[0] & 0x08) != 0;
        ph.loss_bit_q = (bytes[0] & 0x10) != 0;
    }

    let pn_length = ((bytes[0] & 0x03) + 1) as usize;
    if ph.packet_number_offset + pn_length > length {
        return -1;
    }
    let mut truncated = 0u32;
    for i in 0..pn_length {
        let b = bytes[ph.packet_number_offset + i] ^ mask[i + 1];
        bytes[ph.packet_number_offset + i] = b;
        truncated = (truncated << 8) | b as u32;
    }
    ph.packet_number_truncated = truncated;
    ph.packet_number_mask = if pn_length == 4 {
        u32::MAX as u64
    } else {
        (1u64 << (8 * pn_length)) - 1
    };
    ph.packet_number_full = get_packet_number64(sack_list_last, ph.packet_number_mask, truncated);
    let copy_len = length.min(decrypted_bytes.len());
    decrypted_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);
    0
}
```

## Pair `picoquic/packet.c:picoquic_incoming_version_negotiation`
C: `picoquic/packet.c:904-986 picoquic_incoming_version_negotiation`
Rust: `rs/fq/src/lib.rs:3289-3338 incoming_packet_ex`

### C body
```c
{
    int ret = 0;
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(addr_from);
    UNREFERENCED_PARAMETER(current_time);
#endif

    /* Check the connection state */
    if (cnx->cnx_state != picoquic_state_client_init_sent) {
        /* This is an unexpected packet. Log and drop.*/
        DBG_PRINTF("Unexpected VN packet (%d), state %d, type: %d, epoch: %d, pc: %d, pn: %d\n",
            cnx->client_mode, cnx->cnx_state, ph->ptype, ph->epoch, ph->pc, (int)ph->pn);
    } else if (picoquic_compare_connection_id(&ph->dest_cnx_id, &cnx->path[0]->first_tuple->p_local_cnxid->cnx_id) != 0 || ph->vn != 0) {
        /* Packet destination ID does not match local CID, should be logged and ignored */
        DBG_PRINTF("VN packet (%d), does not pass echo test.\n", cnx->client_mode);
        ret = PICOQUIC_ERROR_DETECTED;
    }
    else if (picoquic_compare_connection_id(&ph->srce_cnx_id, &cnx->initial_cnxid) != 0 || ph->vn != 0) {
        /* Packet destination ID does not match initial DCID, should be logged and ignored */
        DBG_PRINTF("VN packet (%d), does not pass echo test.\n", cnx->client_mode);
        ret = PICOQUIC_ERROR_DETECTED;
    } else {
        /* Add DOS resilience */
        const uint8_t * v_bytes = bytes + ph->offset;
        const uint8_t* bytes_max = bytes + length;
        int nb_vn = 0;
        while (v_bytes < bytes_max) {
            uint32_t vn = 0;
            if ((v_bytes = picoquic_frames_uint32_decode(v_bytes, bytes_max, &vn)) == NULL){
                DBG_PRINTF("VN packet (%d), length %zu, coding error after %d version numbers.\n",
                    cnx->client_mode, length, nb_vn);
                ret = PICOQUIC_ERROR_DETECTED;
                break;
            } else if (vn == cnx->proposed_version || vn == 0) {
                DBG_PRINTF("VN packet (%d), proposed_version[%d] = 0x%08x.\n", cnx->client_mode, nb_vn, vn);
                ret = PICOQUIC_ERROR_DETECTED;
                break;
            }
            else if (picoquic_get_version_index(vn) >= 0){
                /* The VN packet proposes a valid version that is locally supported */
                nb_vn++;
            }
        }
        if (ret == 0) {
            if (nb_vn == 0) {
                DBG_PRINTF("VN packet (%d), does not propose any interesting version.\n", cnx->client_mode);
                ret = PICOQUIC_ERROR_DETECTED;
            }
            else {
                /* Signal VN to the application */
                if (cnx->callback_fn && length > ph->offset) {
                    (void)(cnx->callback_fn)(cnx, 0, bytes + ph->offset, length - ph->offset,
                        picoquic_callback_version_negotiation, cnx->callback_ctx, NULL);
                }
                /* TODO: consider rewriting the version negotiation code */
                DBG_PRINTF("%s", "Disconnect upon receiving version negotiation.\n");
                cnx->remote_error = PICOQUIC_ERROR_VERSION_NEGOTIATION;
                picoquic_connection_disconnect(cnx);
                ret = 0;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<Option<&mut Connection>, Error> {
        let packet_length = bytes.len();
        let mut consumed_index = 0usize;
        let mut previous_dest_id = ConnectionId::default();
        let mut first_cnx = None;

        while consumed_index < packet_length {
            let mut consumed = 0usize;
            let ret = self.incoming_segment(
                &mut bytes[consumed_index..],
                packet_length - consumed_index,
                packet_length,
                &mut consumed,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
                current_time,
                current_time,
                &mut previous_dest_id,
                &mut first_cnx,
            );

            if ret == 0 {
                consumed_index = consumed_index.saturating_add(consumed);
                if consumed == 0 {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(token) = first_cnx
            && let Some(cnx) = self.connections.get_mut(token)
            && packet_length > cnx.max_mtu_received
        {
            cnx.max_mtu_received = packet_length;
        }

        Ok(first_cnx.and_then(|token| self.connections.get_mut(token)))
    }
```
