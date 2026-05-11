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

## Pair `picoquic/logwriter.c:picoquic_log_path_available_or_backup_frame`
C: `picoquic/logwriter.c:459-467 picoquic_log_path_available_or_backup_frame`
Rust: `rs/fq/src/binlog.rs:603-614 log_path_available_or_backup_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* frame type as varint */
    bytes = picoquic_skip_path_available_or_backup_frame(bytes, bytes_max); /* skip available or backup frame */
    picoquic_binlog_frame(f, bytes_begin, bytes);

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_padding`
C: `picoquic/logwriter.c:505-515 picoquic_log_padding`
Rust: `rs/fq/src/binlog.rs:636-647 log_padding`

### C body
```c
{
    picoquic_binlog_frame(f, bytes, bytes + 1);

    uint8_t ftype = bytes[0];
    while (bytes < bytes_max && bytes[0] == ftype) {
        bytes++;
    }

    return bytes;
}
```

### Rust body
```rust
fn log_padding<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    if bytes_in.is_empty() {
        return None;
    }
    append_frame(out, &bytes_in[..1]);
    let ftype = bytes_in[0];
    let mut idx = 0usize;
    while idx < bytes_in.len() && bytes_in[idx] == ftype {
        idx += 1;
    }
    Some(&bytes_in[idx..])
}
```

## Pair `picoquic/logwriter.c:binlog_compose_event_header`
C: `picoquic/logwriter.c:679-687 binlog_compose_event_header`
Rust: `rs/fq/src/binlog.rs:201-212 compose_event_header`

### C body
```c
{
    /* Common chunk header */
    bytewrite_cid(msg, cid);
    bytewrite_vint(msg, current_time);
    bytewrite_vint(msg, path_id);
    bytewrite_vint(msg, (uint64_t)event_type);
}
```

### Rust body
```rust
) {
    let _ = msg.write_cid(cid);
    let _ = msg.write_varint(current_time.ticks());
    let _ = msg.write_varint(path_id);
    let _ = msg.write_varint(event_type as u64);
}
```

## Pair `picoquic/logwriter.c:binlog_packet`
C: `picoquic/logwriter.c:734-788 binlog_packet`
Rust: `rs/fq/src/binlog.rs:840-907 packet`

### C body
```c
{
    long fpos0 = ftell(f);

    uint8_t head[4] = { 0 };
    (void)fwrite(head, 4, 1, f);

    bytestream_buf stream_msg;
    bytestream* msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);

    /* Common chunk header */
    binlog_compose_event_header(msg, cid, current_time, path_id, picoquic_log_event_packet_sent + receiving);

    /* packet information */
    bytewrite_vint(msg, bytes_max);

    /* packet header */
    bytewrite_int8(msg, (uint8_t)(64*ph->quic_bit_is_zero + 2 * ph->spin + ph->key_phase));
    bytewrite_vint(msg, ph->payload_length);
    bytewrite_vint(msg, ph->ptype);
    bytewrite_vint(msg, ph->pn64);

    bytewrite_cid(msg, &ph->dest_cnx_id);
    bytewrite_cid(msg, &ph->srce_cnx_id);

    if (ph->ptype != picoquic_packet_1rtt_protected &&
        ph->ptype != picoquic_packet_version_negotiation) {
        bytewrite_int32(msg, ph->vn);
    }

    if (ph->ptype == picoquic_packet_initial) {
        bytewrite_vint(msg, ph->token_length);
        bytewrite_buffer(msg, ph->token_bytes, ph->token_length);
    }

    (void)fwrite(bytestream_data(msg), bytestream_length(msg), 1, f);

    /* frame information */
    if (ph->ptype == picoquic_packet_version_negotiation || ph->ptype == picoquic_packet_retry) {
        picoquic_binlog_frame(f, bytes + ph->offset, bytes + bytes_max);
    }
    else if (ph->ptype != picoquic_packet_error) {
        picoquic_binlog_frames(f, bytes + ph->offset, ph->payload_length);
    }

    /* re-write chunk size field */
    long fpos1 = ftell(f);

    picoformat_32(head, (uint32_t)(fpos1 - fpos0 - 4));

    (void)fseek(f, fpos0, SEEK_SET);
    (void)fwrite(head, 4, 1, f);
    (void)fseek(f, 0, SEEK_END);
}
```

### Rust body
```rust
) {
    // Buffer the whole record (header + framing) into a `Vec<u8>`,
    // then write 4-byte length + payload.  The C version reserves
    // four bytes via `fseek`/`ftell` because `picoquic_binlog_frames`
    // writes straight to the FILE; the buffered shape is the
    // natural Rust equivalent.
    let mut payload: Vec<u8> = Vec::new();

    {
        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };

        let event = if receiving {
            LogEventType::PacketRecv
        } else {
            LogEventType::PacketSent
        };
        compose_event_header(&mut msg, cid, current_time, path_id, event);

        let _ = msg.write_varint(bytes.len() as u64);

        // Packed header byte: quic_bit_is_zero<<6 | spin<<1 | key_phase.
        let flags: u8 = (if ph.quic_bit_is_zero { 64 } else { 0 })
            + (if ph.spin { 2 } else { 0 })
            + (if ph.key_phase { 1 } else { 0 });
        let _ = msg.write_u8(flags);
        let _ = msg.write_varint(ph.payload_length as u64);
        let _ = msg.write_varint(ph.packet_type as u64);
        let _ = msg.write_varint(ph.packet_number_full);

        let _ = msg.write_cid(&ph.dest_connection_id);
        let _ = msg.write_cid(&ph.src_connection_id);

        if ph.packet_type != PacketType::OneRttProtected
            && ph.packet_type != PacketType::VersionNegotiation
        {
            let _ = msg.write_u32(ph.version);
        }

        if ph.packet_type == PacketType::Initial {
            let _ = msg.write_varint(ph.token_bytes.len() as u64);
            let _ = msg.write_bytes(&ph.token_bytes);
        }

        payload.extend_from_slice(msg.as_bytes());
    }

    if ph.packet_type == PacketType::VersionNegotiation || ph.packet_type == PacketType::Retry {
        let payload_slice = bytes.get(ph.offset..).unwrap_or(&[]);
        append_frame(&mut payload, payload_slice);
    } else if ph.packet_type != PacketType::Error {
        let end = ph.offset.saturating_add(ph.payload_length).min(bytes.len());
        let frames_slice = bytes.get(ph.offset..end).unwrap_or(&[]);
        binlog_frames(&mut payload, frames_slice);
    }

    write_record(f, &payload);
}
```

## Pair `picoquic/logwriter.c:binlog_outgoing_packet`
C: `picoquic/logwriter.c:841-888 binlog_outgoing_packet`
Rust: `rs/fq/src/binlog.rs:980-1530 outgoing_packet`

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

## Pair `picoquic/logwriter.c:binlog_picotls_ticket`
C: `picoquic/logwriter.c:984-1001 binlog_picotls_ticket`
Rust: `rs/fq/src/binlog.rs:915-933 tls_ticket`

### C body
```c
{
    bytestream_buf stream_msg;
    bytestream * msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);
    /* Common chunk header */
    binlog_compose_event_header(msg, &cnx_id, 0, 0, picoquic_log_event_tls_key_update);

    bytewrite_vint(msg, ticket_length);
    bytewrite_buffer(msg, ticket, ticket_length);

    bytestream_buf stream_head;
    bytestream * head = bytestream_buf_init(&stream_head, 8);
    bytewrite_int32(head, (uint32_t)bytestream_length(msg));

    (void)fwrite(bytestream_data(head), bytestream_length(head), 1, f);
    (void)fwrite(bytestream_data(msg), bytestream_length(msg), 1, f);
}
```

### Rust body
```rust
pub fn tls_ticket(f: &mut File, cnx_id: ConnectionId, ticket: &[u8]) {
    let mut buf = ByteStreamBuf::default();
    let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
        return;
    };
    compose_event_header(
        &mut msg,
        &cnx_id,
        Instant::from_ticks(0),
        0,
        LogEventType::TlsKeyUpdate,
    );
    let _ = msg.write_varint(ticket.len() as u64);
    let _ = msg.write_bytes(ticket);

    let payload = msg.as_bytes().to_vec();
    drop(msg);
    write_record(f, &payload);
}
```

## Pair `picoquic/logwriter.c:create_binlog`
C: `picoquic/logwriter.c:1123-1145 create_binlog`
Rust: `rs/fq/src/binlog.rs:169-198 create_binlog`

### C body
```c
{
    FILE* f_binlog = picoquic_file_open(binlog_file, "wb");
    if (f_binlog == NULL) {
        DBG_PRINTF("Cannot open file %s for write.\n", binlog_file);
    }
    else {
        /* Write a header text with version identifier and current date  */
        bytestream_buf stream;
        bytestream* ps = bytestream_buf_init(&stream, 16);
        bytewrite_int32(ps, FOURCC('q', 'l', 'o', 'g'));
        bytewrite_int16(ps, (is_multipath_supported) ? 0x01 : 0); /* flags */
        bytewrite_int16(ps, 0x01); /* version */
        bytewrite_int64(ps, creation_time);

        if (fwrite(bytestream_data(ps), bytestream_length(ps), 1, f_binlog) <= 0) {
            DBG_PRINTF("Cannot write header for file %s.\n", binlog_file);
            f_binlog = picoquic_file_close(f_binlog);
        }
    }

    return f_binlog;
}
```

### Rust body
```rust
) -> Option<File> {
    let path = binlog_file.as_ref();
    let mut f_binlog = match File::create(path) {
        Ok(file) => file,
        Err(_) => {
            log::debug!("Cannot open file {} for write.", path.display());
            return None;
        }
    };

    let mut buf = ByteStreamBuf::default();
    let mut stream = buf.stream(16)?;
    let flags: u16 = if is_multipath_supported { 0x01 } else { 0 };
    let header_result = stream
        .write_u32(crate::fourcc(b'q', b'l', b'o', b'g'))
        .and_then(|()| stream.write_u16(flags))
        .and_then(|()| stream.write_u16(0x01))
        .and_then(|()| stream.write_u64(creation_time));

    if header_result.is_err() || f_binlog.write_all(stream.as_bytes()).is_err() {
        log::debug!("Cannot write header for file {}.", path.display());
        return None;
    }

    Some(f_binlog)
}
```

## Pair `picoquic/logwriter.c:binlog_close`
C: `picoquic/logwriter.c:1309-1315 binlog_close`
Rust: `rs/fq/src/binlog.rs:1576-1597 binlog_close`

### C body
```c
{
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(quic);
#endif
}
```

### Rust body
```rust
    pub fn enable_binlog(&mut self) {
        // C: `quic->bin_log_fns = &binlog_functions;` installs the
        // unified-logging vtable.  In Rust the per-event binlog
        // dispatch lives directly on the [`Binlog`] trait
        // (implemented for [`Connection`]); the unified [`Logger`]
        // dispatch path on `Quic` is a no-op for an unconfigured
        // backend per `Connection::log_new_connection`.  Leaving
        // `bin_log_fns` as `None` matches the established pattern
        // in `set_qlog` / `set_textlog` and avoids an empty-shell
        // trait object — `is_still_logging` keys off `f_binlog`,
        // which is the source of truth for whether a record-writing
        // call should fire.
        let _ = &self.bin_log_fns;
    }
```

## Pair `picoquic/loss_recovery.c:picoquic_retransmit_needed_loop`
C: `picoquic/loss_recovery.c:203-223 picoquic_retransmit_needed_loop`
Rust: `rs/fq/src/internal.rs:11822-11858 retransmit_needed_loop`

### C body
```c
{
    int continue_next = 1;
    size_t length = 0;
    picoquic_packet_t* old_p = pkt_ctx->pending_first;

    /* Call the per packet routine in a loop */
    while (old_p != 0 && continue_next) {
        picoquic_packet_t* p_next = old_p->packet_next;

        length = picoquic_retransmit_needed_packet(cnx, pkt_ctx, old_p, pc, path_x, current_time,
            next_wake_time, packet, send_buffer_max, header_length, &continue_next);
        old_p = p_next;
    }
    /* TODO: manage the pto flag for the path. */

    return length;
}
```

### Rust body
```rust
    ) -> usize {
        let mut continue_next = 1;
        let mut length = 0usize;
        let mut old_p = self.pending_first_token(selection);

        while let Some(old_token) = old_p {
            if continue_next == 0 {
                break;
            }
            let p_next = self.pending_next_token(selection, old_token);
            length = self.retransmit_needed_packet(
                selection,
                old_token,
                pc,
                path_x,
                current_time,
                next_wake_time,
                packet,
                send_buffer_max,
                header_length,
                &mut continue_next,
            );
            old_p = p_next.filter(|token| self.queued_packets.contains(*token));
        }

        length
    }
```

## Pair `picoquic/loss_recovery.c:picoquic_copy_before_retransmit`
C: `picoquic/loss_recovery.c:664-782 picoquic_copy_before_retransmit`
Rust: `rs/fq/src/internal.rs:10945-10968 copy_before_retransmit`

### C body
```c
{
    /* check if this is an ACK only packet */
    int ret = 0;
    int frame_is_pure_ack = 0;
    size_t frame_length = 0;
    size_t byte_index = 0; /* Used when parsing the old packet */

    if (old_p->is_mtu_probe) {
        if (old_p->send_path != NULL) {
            /* MTU probe was lost, presumably because of packet too big */
            old_p->send_path->mtu_probe_sent = 0;
            if (!force_queue || force_queue == 2) {
                old_p->send_path->send_mtu_max_tried = old_p->length + old_p->checksum_overhead;
            }
        }
        /* MTU probes should not be retransmitted */
        *packet_is_pure_ack = 1;
        *do_not_detect_spurious = 0;
    }
    else if (old_p->is_ack_trap) {
        *packet_is_pure_ack = 1;
        *do_not_detect_spurious = 1;
    }
    else if (old_p->is_multipath_probe) {
        *packet_is_pure_ack = 0;
        *do_not_detect_spurious = 1;
    }
    else if (old_p->was_preemptively_repeated) {
        *packet_is_pure_ack = 1;
        *do_not_detect_spurious = 1;
    }
    else {
        /* Copy the relevant bytes from one packet to the next */
        byte_index = old_p->offset;

        while (ret == 0 && byte_index < old_p->length) {
            ret = picoquic_skip_frame(&old_p->bytes[byte_index],
                old_p->length - byte_index, &frame_length, &frame_is_pure_ack);

            /* Check whether the data was already acked, which may happen in
            * case of spurious retransmissions */
            if (ret == 0 && frame_is_pure_ack == 0) {
                ret = picoquic_check_frame_needs_repeat(cnx, &old_p->bytes[byte_index],
                    frame_length, old_p->ptype, &frame_is_pure_ack, do_not_detect_spurious, NULL);
            }

            /* Keep track of datagram frames that are possibly lost */
            if (ret == 0 &&
                PICOQUIC_IN_RANGE(old_p->bytes[byte_index], picoquic_frame_type_datagram, picoquic_frame_type_datagram_l) &&
                cnx->callback_fn != NULL) {
                uint8_t frame_id;
                uint64_t content_length;
                uint8_t* content_bytes = &old_p->bytes[byte_index];

                /* Parse and skip type and length */
                content_bytes = picoquic_decode_datagram_frame_header(content_bytes, content_bytes + frame_length,
                    &frame_id, &content_length);
                if (content_bytes != NULL) {
                    ret = (cnx->callback_fn)(cnx, old_p->send_time, content_bytes, (size_t)content_length,
                        picoquic_callback_datagram_lost, cnx->callback_ctx, NULL);
                }
                picoquic_log_app_message(cnx, "Datagram lost, PN=%" PRIu64 ", Sent: %" PRIu64,
                    old_p->sequence_number, old_p->send_time);
            }

            /* Prepare retransmission if needed */
            if (ret == 0) {
                if (!frame_is_pure_ack) {
                    if (PICOQUIC_IN_RANGE(old_p->bytes[byte_index], picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max)) {
                        * add_to_data_repeat_queue = 1;
                    }
                    else {
                        if ((force_queue || frame_length > send_buffer_max_minus_checksum - *length)) {
                            ret = picoquic_queue_misc_frame(cnx, &old_p->bytes[byte_index], frame_length, 0,
                                old_p->pc);
                        }
                        else if (frame_length <= send_buffer_max_minus_checksum - *length) {
                            memcpy(&new_bytes[*length], &old_p->bytes[byte_index], frame_length);
                            *length += frame_length;
                        }
                        else {
                            uint64_t error_frame_type = 0;
                            (void)picoquic_varint_decode(&old_p->bytes[byte_index], frame_length, &error_frame_type);
                            picoquic_log_app_message(cnx, "Cannot copy frame 0x%" PRIu64 ", packet type = %d, force queue = %d, repeat buffer : %zu, previous length : %zu.",
                                error_frame_type, old_p->ptype, force_queue);
                            ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR,
                                error_frame_type, "Cannot copy frame for retransmit");
                        }
                    }
                    *packet_is_pure_ack = 0;
                }
                byte_index += frame_length;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let snapshot = PacketRetransmitSnapshot::from(&*old_p);
    copy_before_retransmit_snapshot(
        &snapshot,
        connection,
        new_bytes,
        send_buffer_max_minus_checksum,
        packet_is_pure_ack,
        do_not_detect_spurious,
        force_queue,
        length,
        add_to_data_repeat_queue,
    )
}
```

## Pair `picoquic/loss_recovery.c:picoquic_is_packet_ack_eliciting`
C: `picoquic/loss_recovery.c:934-967 picoquic_is_packet_ack_eliciting`
Rust: `rs/fq/src/internal.rs:10766-10776 is_packet_ack_eliciting`

### C body
```c
{
    /* check if this is an ACK eliciting packet */
    int is_ack_eliciting = 0;

    if (packet->is_evaluated) {
        is_ack_eliciting = packet->is_ack_eliciting;
    } else {
        /* Trap packets are never supposed to elicit an ACK. */
        /* For other packets, we need to look at the frames inside. */
        if (!packet->is_ack_trap) {
            size_t frame_length = 0;
            size_t byte_index = packet->offset;

            while (byte_index < packet->length) {
                int frame_is_pure_ack = 0;
                if (picoquic_skip_frame(&packet->bytes[byte_index],
                    packet->length - byte_index, &frame_length, &frame_is_pure_ack) != 0) {
                    /* Malformed packet. Ignore it. Do not expect an ack */
                    break;
                }
                if (!frame_is_pure_ack) {
                    is_ack_eliciting = 1;
                    break;
                }
                byte_index += frame_length;
            }
        }
        packet->is_ack_eliciting = is_ack_eliciting;
        packet->is_evaluated = 1;
    }

    return is_ack_eliciting;
}
```

### Rust body
```rust
fn is_packet_ack_eliciting(packet: &mut Packet) -> bool {
    if packet.is_evaluated {
        return packet.is_ack_eliciting;
    }

    let snapshot = PacketRetransmitSnapshot::from(&*packet);
    let is_ack_eliciting = packet_is_ack_eliciting_from_snapshot(&snapshot);
    packet.is_ack_eliciting = is_ack_eliciting;
    packet.is_evaluated = true;
    is_ack_eliciting
}
```

## Pair `picoquic/newreno.c:picoquic_newreno_sim_reset`
C: `picoquic/newreno.c:36-43 picoquic_newreno_sim_reset`
Rust: `rs/fq/src/cc_common.rs:450-454 reset`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    memset(nrss, 0, sizeof(picoquic_newreno_sim_state_t));
    nrss->alg_state = picoquic_newreno_alg_slow_start;
    nrss->ssthresh = UINT64_MAX;
    nrss->cwin = PICOQUIC_CWIN_INITIAL;
}
```

### Rust body
```rust
    pub fn reset(&mut self) {
        *self = Self::default();
        self.ssthresh = u64::MAX;
        self.cwin = CWIN_INITIAL;
    }
```

## Pair `picoquic/newreno.c:picoquic_newreno_reset`
C: `picoquic/newreno.c:182-187 picoquic_newreno_reset`
Rust: `rs/fq/src/newreno.rs:51-55 picoquic_newreno_reset`

### C body
```c
{
    memset(nr_state, 0, sizeof(picoquic_newreno_state_t));
    picoquic_newreno_sim_reset(&nr_state->nrss);
    path_x->cwin = nr_state->nrss.cwin;
}
```

### Rust body
```rust
fn picoquic_newreno_reset(nr_state: &mut NewrenoState, path_x: &mut Path) {
    *nr_state = NewrenoState::default();
    nr_state.nrss.reset();
    path_x.cwin = nr_state.nrss.cwin;
}
```

## Pair `picoquic/newreno.c:picoquic_newreno_observe`
C: `picoquic/newreno.c:309-314 picoquic_newreno_observe`
Rust: `rs/fq/src/newreno.rs:191-199 observe`

### C body
```c
{
    picoquic_newreno_state_t* nr_state = (picoquic_newreno_state_t*)path_x->congestion_alg_state;
    *cc_state = (uint64_t)nr_state->nrss.alg_state;
    *cc_param = (nr_state->nrss.ssthresh == UINT64_MAX) ? 0 : nr_state->nrss.ssthresh;
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        let cc_state = self.nrss.alg_state as u64;
        let cc_param = if self.nrss.ssthresh == u64::MAX {
            0
        } else {
            self.nrss.ssthresh
        };
        (cc_state, cc_param)
    }
```

## Pair `picoquic/pacing.c:picoquic_is_authorized_by_pacing`
C: `picoquic/pacing.c:62-105 picoquic_is_authorized_by_pacing`
Rust: `rs/fq/src/internal.rs:5933-5961 is_authorized`

### C body
```c
{
    int ret = 1;

    picoquic_update_pacing_bucket(pacing, current_time);

    if (pacing->bucket_nanosec < pacing->packet_time_nanosec) {
        uint64_t next_pacing_time;
        int64_t bucket_required;

        if (packet_train_mode || pacing->bandwidth_pause) {
            bucket_required = pacing->bucket_max;

            if (bucket_required > 10 * pacing->packet_time_nanosec) {
                bucket_required = 10 * pacing->packet_time_nanosec;
            }

            bucket_required -= pacing->bucket_nanosec;
        }
        else {
            bucket_required = pacing->packet_time_nanosec - pacing->bucket_nanosec;
        }

        next_pacing_time = current_time + 1 + bucket_required / 1000;
        if (next_pacing_time < *next_time) {
            pacing->bandwidth_pause = 0;
            *next_time = next_pacing_time;
            if (quic != NULL) {
                SET_LAST_WAKE(quic, PICOQUIC_SENDER);
            }
        }
        ret = 0;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> bool {
        self.update_bucket(current_time);
        if self.bucket_nanosec < self.packet_time_nanosec {
            let bucket_required = if packet_train_mode || self.bandwidth_pause != 0 {
                let mut br = self.bucket_max;
                if br > 10 * self.packet_time_nanosec {
                    br = 10 * self.packet_time_nanosec;
                }
                br - self.bucket_nanosec
            } else {
                self.packet_time_nanosec - self.bucket_nanosec
            };
            let next_pacing_ticks = current_time.ticks() + 1 + (bucket_required as u64) / 1000;
            let next_pacing = crate::Instant::from_ticks(next_pacing_ticks);
            if next_pacing_ticks < next_time.ticks() {
                self.bandwidth_pause = 0;
                *next_time = next_pacing;
            }
            false
        } else {
            true
        }
    }
```

## Pair `picoquic/pacing.c:picoquic_update_pacing_data_after_send`
C: `picoquic/pacing.c:235-245 picoquic_update_pacing_data_after_send`
Rust: `rs/fq/src/internal.rs:6060-6068 update_after_send`

### C body
```c
{
    uint64_t packet_time_nanosec;

    picoquic_update_pacing_bucket(pacing, current_time);
    packet_time_nanosec = ((pacing->packet_time_nanosec * (uint64_t)length) + (send_mtu - 1)) / send_mtu;
    pacing->bucket_nanosec -= packet_time_nanosec;
}
```

### Rust body
```rust
    pub fn update_after_send(&mut self, length: usize, send_mtu: usize, current_time: Instant) {
        // C: picoquic_update_pacing_data_after_send
        self.update_bucket(current_time);
        if send_mtu > 0 {
            let packet_time_nanosec = (self.packet_time_nanosec.max(0) as u128 * length as u128)
                .div_ceil(send_mtu as u128);
            self.bucket_nanosec -= packet_time_nanosec.min(i64::MAX as u128) as i64;
        }
    }
```

## Pair `picoquic/pacing.c:picoquic_update_pacing_data`
C: `picoquic/pacing.c:265-270 picoquic_update_pacing_data`
Rust: `rs/fq/src/internal.rs:6074-6082 update_pacing_data`

### C body
```c
{
    picoquic_update_pacing_window(&path_x->pacing, slow_start, path_x->cwin, path_x->send_mtu, path_x->smoothed_rtt,
        path_x);
}
```

### Rust body
```rust
    pub fn update_pacing_data(&mut self, slow_start: i32) {
        self.pacing.update_window(
            slow_start,
            self.cwin,
            self.send_mtu,
            self.smoothed_rtt,
            None,
        );
    }
```

## Pair `picoquic/packet.c:picoquic_parse_short_packet_header`
C: `picoquic/packet.c:396-473 picoquic_parse_short_packet_header`
Rust: `rs/fq/src/internal.rs:6749-6827 parse_short_packet_header_inner`

### C body
```c
{
    int ret = 0;
    /* If this is a short header, it should be possible to retrieve the connection
     * context. This depends on whether the quic context requires cnx_id or not.
     */
    uint8_t cnxid_length = (receiving == 0 && *pcnx != NULL) ? (*pcnx)->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len : quic->local_cnxid_length;
    ph->pc = picoquic_packet_context_application;
    ph->pl_val = 0; /* No actual payload length in short headers */

    if ((int)length >= 1 + cnxid_length) {
        /* We can identify the connection by its ID */
        ph->offset = (size_t)1 + picoquic_parse_connection_id(bytes + 1, cnxid_length, &ph->dest_cnx_id);
        /* TODO: should consider using combination of CNX ID and ADDR_FROM */
        if (*pcnx == NULL)
        {
            if (quic->local_cnxid_length > 0) {
                *pcnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
            }
            else {
                *pcnx = picoquic_cnx_by_net(quic, addr_from);
            }
        }
    }
    else {
        ph->ptype = picoquic_packet_error;
        ph->offset = length;
        ph->payload_length = 0;
    }

    if (*pcnx != NULL) {
        int has_loss_bit = (receiving && (*pcnx)->is_loss_bit_enabled_incoming) || ((!receiving && (*pcnx)->is_loss_bit_enabled_outgoing));
        ph->epoch = picoquic_epoch_1rtt;
        ph->version_index = (*pcnx)->version_index;
        ph->quic_bit_is_zero = (bytes[0] & 0x40) == 0;

        if (!ph->quic_bit_is_zero ||(*pcnx)->local_parameters.do_grease_quic_bit) {
            /* We do not check the quic bit if the local endpoint advertised greasing. */
            ph->ptype = picoquic_packet_1rtt_protected;
        } else {
            /* Check for QUIC bit failed! */
            ph->ptype = picoquic_packet_error;
        }

        ph->has_spin_bit = 1;
        ph->spin = (bytes[0] >> 5) & 1;
        ph->pn_offset = ph->offset;
        ph->pn = 0;
        ph->pnmask = 0;
        ph->key_phase = ((bytes[0] >> 2) & 1); /* Initialize here so that simple tests with unencrypted headers can work */

        if (has_loss_bit) {
            ph->has_loss_bits = 1;
            ph->loss_bit_L = (bytes[0] >> 3) & 1;
            ph->loss_bit_Q = (bytes[0] >> 4) & 1;
        }
        if (length < ph->offset || ph->ptype == picoquic_packet_error) {
            ret = -1;
            ph->payload_length = 0;
        }
        else {
            ph->payload_length = (uint16_t)(length - ph->offset);
        }
    }
    else {
        /* This may be a packet to a forgotten connection, or a packet bound to a proxied connection */
        ph->ptype = picoquic_packet_1rtt_protected;
        ph->payload_length = (uint16_t)((length > ph->offset) ? length - ph->offset : 0);
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Option<ConnectionToken> {
        let cnxid_length = self.local_connection_id_length as usize;
        ph.packet_context = PacketContext::Application;
        ph.payload_length_value = 0;

        if bytes.len() < 1 + cnxid_length {
            ph.packet_type = PacketType::Error;
            ph.offset = bytes.len();
            ph.payload_length = 0;
            return None;
        }

        let dcid = ConnectionId::clone_from_slice(&bytes[1..1 + cnxid_length]).unwrap_or_default();
        ph.dest_connection_id = dcid;
        ph.offset = 1 + cnxid_length;
        ph.packet_number_offset = ph.offset;

        // Lookup connection
        let conn_tok = if cnxid_length > 0 {
            let cid = ph.dest_connection_id;
            self.connection_by_id(cid).map(|(tok, _)| tok)
        } else {
            self.connection_by_net(addr_from)
        };

        ph.epoch = Epoch::OneRtt;
        ph.quic_bit_is_zero = (bytes[0] & 0x40) == 0;

        // Check QUIC bit (allow grease mode)
        let do_grease = conn_tok
            .and_then(|tok| self.connections.get(tok))
            .map(|c| c.local_parameters.do_grease_quic_bit)
            .unwrap_or(false);

        if !ph.quic_bit_is_zero || do_grease {
            ph.packet_type = PacketType::OneRttProtected;
        } else {
            ph.packet_type = PacketType::Error;
        }

        ph.has_spin_bit = true;
        ph.spin = (bytes[0] >> 5) & 1 != 0;
        ph.key_phase = ((bytes[0] >> 2) & 1) != 0;
        ph.packet_number_mask = 0;
        ph.packet_number_truncated = 0;

        if conn_tok.is_some() {
            let is_loss_bit = conn_tok
                .and_then(|tok| self.connections.get(tok))
                .map(|c| c.is_loss_bit_enabled_incoming || c.is_loss_bit_enabled_outgoing)
                .unwrap_or(false);
            if is_loss_bit {
                ph.has_loss_bits = true;
                ph.loss_bit_l = (bytes[0] >> 3) & 1 != 0;
                ph.loss_bit_q = (bytes[0] >> 4) & 1 != 0;
            }
        }

        ph.payload_length = if bytes.len() > ph.offset {
            bytes.len() - ph.offset
        } else {
            0
        };

        // Set version_index from connection if found
        if let Some(tok) = conn_tok
            && let Some(cnx) = self.connections.get(tok)
        {
            ph.version_index = cnx.version_index;
        }

        conn_tok
    }
```

## Pair `picoquic/packet.c:picoquic_remove_header_protection`
C: `picoquic/packet.c:617-631 picoquic_remove_header_protection`
Rust: `rs/fq/src/internal.rs:7135-7162 picoquic_remove_header_protection`

### C body
```c
{
    int ret = 0;
    size_t length = ph->offset + ph->payload_length; /* this may change after decrypting the PN */
    void * pn_enc = cnx->crypto_context[ph->epoch].pn_dec;

    picoquic_sack_list_t* sack_list = picoquic_sack_list_from_cnx_context(cnx, ph->pc, ph->l_cid);
    ret = picoquic_remove_header_protection_inner(bytes, length, decrypted_bytes, ph,
        pn_enc, cnx->is_loss_bit_enabled_incoming, picoquic_sack_list_last(sack_list));

    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let length = ph.offset.saturating_add(ph.payload_length);
    let epoch = ph.epoch as usize;
    let Some(pn_dec) = connection.crypto_context[epoch].pn_dec.as_deref() else {
        ph.packet_number_truncated = 0xffff_ffff;
        ph.packet_number_mask = 0xffff_ffff_0000_0000;
        ph.offset = ph.packet_number_offset;
        ph.packet_number_full = u64::MAX;
        return crate::errors::InternalError::AeadNotReady as i32;
    };
    let sack_list_last = connection.ack_ctx[ph.packet_context as usize]
        .sack_list
        .first();
    remove_header_protection_inner(
        bytes,
        length,
        decrypted_bytes,
        ph,
        pn_dec,
        connection.is_loss_bit_enabled_incoming,
        sack_list_last,
    )
}
```

## Pair `picoquic/packet.c:picoquic_prepare_version_negotiation`
C: `picoquic/packet.c:994-1075 picoquic_prepare_version_negotiation`
Rust: `rs/fq/src/internal.rs:643-653 prepare_version_negotiation`

### C body
```c
{
    picoquic_cnx_t* cnx = NULL;
    uint8_t dcid_length = original_bytes[5];
    uint8_t * dcid = original_bytes + 6;
    uint8_t scid_length = original_bytes[6 + dcid_length];
    uint8_t* scid = original_bytes + 6 + dcid_length + 1;

    /* Verify that this is not a spurious error by checking whether a connection context
     * already exists */
    if (dcid_length <= PICOQUIC_CONNECTION_ID_MAX_SIZE) {
        (void) picoquic_parse_connection_id(dcid, dcid_length, &ph->dest_cnx_id);
        if (ph->dest_cnx_id.id_len == quic->local_cnxid_length) {
            if (quic->local_cnxid_length == 0) {
                cnx = picoquic_cnx_by_net(quic, addr_from);
            }
            else {
                cnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
            }
        }
        if (cnx == NULL) {
            cnx = picoquic_cnx_by_icid(quic, &ph->dest_cnx_id, addr_from);
        }
    }

    /* If no connection context exists, send back a version negotiation */
    if (cnx == NULL) {
        picoquic_stateless_packet_t* sp = picoquic_create_stateless_packet(quic);

        if (sp != NULL) {
            uint8_t* bytes = sp->bytes;
            size_t byte_index = 0;
            uint32_t rand_vn;

            /* Packet type set to random value for version negotiation */
            picoquic_public_random(bytes + byte_index, 1);
            bytes[byte_index++] |= 0x80;
            /* Set the version number to zero */
            picoformat_32(bytes + byte_index, 0);
            byte_index += 4;

            /* Copy the connection identifiers */
            bytes[byte_index++] = scid_length;
            memcpy(bytes + byte_index, scid, scid_length);
            byte_index += scid_length;
            bytes[byte_index++] = dcid_length;
            memcpy(bytes + byte_index, dcid, dcid_length);
            byte_index += dcid_length;

            /* Set the payload to the list of versions */
            for (size_t i = 0; i < picoquic_nb_supported_versions; i++) {
                picoformat_32(bytes + byte_index, picoquic_supported_versions[i].version);
                byte_index += 4;
            }
            /* Add random reserved value as grease, but be careful to not match proposed version */
            do {
                rand_vn = (((uint32_t)picoquic_public_random_64()) & 0xF0F0F0F0) | 0x0A0A0A0A;
            } while (rand_vn == ph->vn);
            picoformat_32(bytes + byte_index, rand_vn);
            byte_index += 4;

            /* Set length and addresses, and queue. */
            sp->length = byte_index;
            picoquic_store_addr(&sp->addr_to, addr_from);
            picoquic_store_addr(&sp->addr_local, addr_to);
            sp->if_index_local = if_index_to;
            sp->initial_cid = ph->dest_cnx_id;
            sp->cnxid_log64 = picoquic_val64_connection_id(sp->initial_cid);
            sp->ptype = picoquic_packet_version_negotiation;

            picoquic_log_quic_pdu(quic, 1, picoquic_get_quic_time(quic), 0, addr_to, addr_from, sp->length);

            picoquic_queue_stateless_packet(quic, sp);
        }
    }
}
```

### Rust body
```rust
        if original_bytes.len() < 7 {
            return;
        }
```
