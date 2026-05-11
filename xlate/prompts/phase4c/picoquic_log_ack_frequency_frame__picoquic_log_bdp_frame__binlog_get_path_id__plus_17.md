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

## Pair `picoquic/logwriter.c:picoquic_log_ack_frequency_frame`
C: `picoquic/logwriter.c:470-483 picoquic_log_ack_frequency_frame`
Rust: `rs/fq/src/binlog.rs:616-626 log_ack_frequency_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;

    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* frame type as varint */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Seq num */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Packet tolerance */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Max ACK delay */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Reordering threshold */

    picoquic_binlog_frame(f, bytes_begin, bytes);

    return bytes;
}
```

### Rust body
```rust
fn log_ack_frequency_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_bdp_frame`
C: `picoquic/logwriter.c:517-532 picoquic_log_bdp_frame`
Rust: `rs/fq/src/binlog.rs:649-660 log_bdp_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    size_t ip_len = 0;

    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Frame type */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Life time */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Bytes in flight */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* min rtt */
    bytes = picoquic_log_length(bytes, bytes_max, &ip_len); /*  IP Address length */
    bytes = picoquic_log_fixed_skip(bytes, bytes_max, ip_len); /* IP address value */

    picoquic_binlog_frame(f, bytes_begin, bytes);

    return bytes;
}
```

### Rust body
```rust
fn log_bdp_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let (ip_len, rest) = read_length(bytes)?;
    bytes = skip_fixed(rest, ip_len)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:binlog_get_path_id`
C: `picoquic/logwriter.c:689-698 binlog_get_path_id`
Rust: `rs/fq/src/binlog.rs:215-228 get_path_id`

### C body
```c
{
    uint64_t path_id = 0;

    if (cnx->is_multipath_enabled && path_x != NULL) {
        path_id = path_x->unique_path_id;
    }

    return path_id;
}
```

### Rust body
```rust
fn append_varint(out: &mut Vec<u8>, value: u64) {
    let mut buf = [0u8; 8];
    let n = varint_encode(&mut buf, value);
    out.extend_from_slice(&buf[..n]);
}
```

## Pair `picoquic/logwriter.c:binlog_packet_ex`
C: `picoquic/logwriter.c:790-797 binlog_packet_ex`
Rust: `rs/fq/src/logger.rs:514-524 packet`

### C body
```c
{
    if (cnx != NULL && cnx->f_binlog != NULL && picoquic_cnx_is_still_logging(cnx)) {
        binlog_packet(cnx->f_binlog, &cnx->initial_cnxid, binlog_get_path_id(cnx, path_x),
            receiving, current_time, ph, bytes, bytes_max);
    }
}
```

### Rust body
```rust
        if !self.is_still_logging() {
            return;
        }
```

## Pair `picoquic/logwriter.c:binlog_packet_lost`
C: `picoquic/logwriter.c:890-918 binlog_packet_lost`
Rust: `rs/fq/src/binlog.rs:997-1530 packet_lost`

### C body
```c
{
    FILE* f = cnx->f_binlog;

    bytestream_buf stream_msg;
    bytestream* msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);

    bytewrite_int32(msg, 0);
    /* Common chunk header */
    binlog_compose_event_header(msg, &cnx->initial_cnxid, current_time, binlog_get_path_id(cnx, path_x), picoquic_log_event_packet_lost);
    /* Event header */
    bytewrite_vint(msg, ptype);
    bytewrite_vint(msg, sequence_number);
    bytewrite_cstr(msg, trigger);
    if (dcid != NULL) {
        bytewrite_cid(msg, dcid);
    }
    else {
        bytewrite_int8(msg, 0);
    }
    bytewrite_vint(msg, packet_size);

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

## Pair `picoquic/logwriter.c:binlog_picotls_ticket_ex`
C: `picoquic/logwriter.c:1003-1009 binlog_picotls_ticket_ex`
Rust: `rs/fq/src/logger.rs:796-799 tls_ticket`

### C body
```c
{
    if (cnx != NULL && cnx->f_binlog != NULL && picoquic_cnx_is_still_logging(cnx)) {
        binlog_picotls_ticket(cnx->f_binlog, cnx->initial_cnxid, ticket, ticket_length);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().tls_ticket(self, ticket);
        }
```

## Pair `picoquic/logwriter.c:binlog_cc_dump`
C: `picoquic/logwriter.c:1153-1231 binlog_cc_dump`
Rust: `rs/fq/src/binlog.rs:1048-1092 cc_dump`

### C body
```c
{
    bytestream_buf stream_msg;
    bytestream* ps_msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);

    picoquic_packet_context_t* pkt_ctx = &cnx->pkt_ctx[picoquic_packet_context_application];

    if (cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }

    /* Common chunk header */
    /* TODO: understand how to provide per path data -- most probably do a loop on
     * all available paths, and write the data for each path if multipath is enabled.
     * verify that it works for CSV and QLOG formats.
     */
    binlog_compose_event_header(ps_msg, &cnx->initial_cnxid, current_time,
        binlog_get_path_id(cnx, path_x), picoquic_log_event_cc_update);

    bytewrite_vint(ps_msg, pkt_ctx->send_sequence);

    if (pkt_ctx->highest_acknowledged != UINT64_MAX) {
        bytewrite_vint(ps_msg, 1);
        bytewrite_vint(ps_msg, pkt_ctx->highest_acknowledged);
        bytewrite_vint(ps_msg, pkt_ctx->highest_acknowledged_time - cnx->start_time);
        bytewrite_vint(ps_msg, pkt_ctx->latest_time_acknowledged - cnx->start_time);
    }
    else {
        bytewrite_vint(ps_msg, 0);
    }

    bytewrite_vint(ps_msg, path_x->cwin);
    bytewrite_vint(ps_msg, path_x->one_way_delay_sample);
    bytewrite_vint(ps_msg, path_x->rtt_sample);
    bytewrite_vint(ps_msg, path_x->smoothed_rtt);
    bytewrite_vint(ps_msg, path_x->rtt_min);
    bytewrite_vint(ps_msg, path_x->bandwidth_estimate);
    bytewrite_vint(ps_msg, path_x->receive_rate_estimate);
    bytewrite_vint(ps_msg, path_x->send_mtu);
    bytewrite_vint(ps_msg, path_x->pacing.packet_time_microsec);
    if (cnx->is_multipath_enabled) {
        bytewrite_vint(ps_msg, path_x->nb_losses_found);
        bytewrite_vint(ps_msg, path_x->nb_spurious);
    }
    else {
        bytewrite_vint(ps_msg, cnx->nb_retransmission_total);
        bytewrite_vint(ps_msg, cnx->nb_spurious);
    }
    bytewrite_vint(ps_msg, cnx->cwin_blocked);
    bytewrite_vint(ps_msg, cnx->flow_blocked);
    bytewrite_vint(ps_msg, cnx->stream_blocked);

    if (cnx->congestion_alg == NULL) {
        bytewrite_vint(ps_msg, 0);
        bytewrite_vint(ps_msg, 0);
    }
    else {
        uint64_t cc_state = 0;
        uint64_t cc_param = 0;

        if (cnx->path[0]->congestion_alg_state != NULL) {
            cnx->congestion_alg->alg_observe(cnx->path[0], &cc_state, &cc_param);
        }
        bytewrite_vint(ps_msg, cc_state);
        bytewrite_vint(ps_msg, cc_param);
    }

    bytewrite_vint(ps_msg, path_x->peak_bandwidth_estimate);
    bytewrite_vint(ps_msg, path_x->bytes_in_transit);
    bytewrite_vint(ps_msg, path_x->last_bw_estimate_path_limited);

    bytestream_buf stream_head;
    bytestream* ps_head = bytestream_buf_init(&stream_head, BYTESTREAM_MAX_BUFFER_SIZE);

    bytewrite_int32(ps_head, (uint32_t)bytestream_length(ps_msg));

    (void)fwrite(bytestream_data(ps_head), bytestream_length(ps_head), 1, cnx->f_binlog);
    (void)fwrite(bytestream_data(ps_msg), bytestream_length(ps_msg), 1, cnx->f_binlog);
}
```

### Rust body
```rust
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
```

## Pair `picoquic/logwriter.c:picoquic_set_binlog`
C: `picoquic/logwriter.c:1338-1344 picoquic_set_binlog`
Rust: `rs/fq/src/binlog.rs:1556-1566 set_binlog`

### C body
```c
{
    quic->binlog_dir = picoquic_string_free(quic->binlog_dir);
    quic->binlog_dir = picoquic_string_duplicate(binlog_dir);
    quic->bin_log_fns = &binlog_functions;
    return 0;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Mirror C `quic->binlog_dir = strdup(binlog_dir)` (NULL-clear
        // path included): swap out the previous directory and store
        // the new one.
        self.binlog_dir = binlog_dir.map(|p| PathBuf::from(p.as_ref()));
        self.enable_binlog();
        Ok(())
    }
```

## Pair `picoquic/loss_recovery.c:picoquic_retransmit_needed_packet`
C: `picoquic/loss_recovery.c:297-533 picoquic_retransmit_needed_packet`
Rust: `rs/fq/src/internal.rs:11568-11820 retransmit_needed_packet`

### C body
```c
{
    size_t length = 0;
    *continue_next = 0;

    /* TODO: while packets are pure ACK, drop them from retransmit queue */
    picoquic_path_t* old_path = old_p->send_path; /* should be the path on which the packet was transmitted */

    int is_probably_lost = 0;
    int is_timer_expired = 0;
    uint64_t next_retransmit_time = *next_wake_time;

    length = 0;

    if (old_p->ptype == picoquic_packet_0rtt_protected && cnx->cnx_state < picoquic_state_client_ready_start) {
        /* Special case: 0RTT cannot be acked before handshake is complete */
        *continue_next = 1;
        return 0;
    }
    is_probably_lost = cnx->initial_repeat_needed || old_p->send_path == NULL ||
        picoquic_is_packet_probably_lost(cnx, old_p, current_time, &next_retransmit_time, &is_timer_expired);

    if (is_probably_lost && (old_p->ptype == picoquic_packet_initial || old_p->ptype == picoquic_packet_handshake)) {
        /* Need to verify that there is no coalescing issue! */
        if (old_p->length > send_buffer_max) {
            picoquic_log_app_message(cnx, "Delay retransmission in type %d, seq %" PRIu64 ", buffer too small",
                old_p->ptype, old_p->sequence_number);
            return 0;
        }
    }

    if (is_probably_lost) {
        if (old_p->is_ack_trap) {
            picoquic_dequeue_retransmit_packet(cnx, pkt_ctx, old_p, 1, 0);
            *continue_next = 1;
        }
        else {
            /* check if this is an ACK only packet */
            int packet_is_pure_ack = 1;

            /* Parse the old packet, queue frames for retransmit, perhaps copy some
             * frames into the new packet, dequeue from packet queue and if needed
             * copy to "retransmitted", return old_p = 0 if freed.
             */
            old_p = picoquic_process_lost_packet(cnx, pkt_ctx, old_p, 0, pc, path_x, current_time,
                packet, send_buffer_max, &length, &packet_is_pure_ack, header_length);

            /* If there was no frame copied in the packet, tell the caller to continue the loop. */
            if (old_p == NULL || packet_is_pure_ack) {
                length = 0;
                *continue_next = 1;
            }
            /* Also continue the loop if the packet is zero length. This is a tradeoff, because
             * in some circumstances it may cause an increase in memory consumption.
             * Consider limiting this to cases when "bytes in transit" is larger than "CWIN".
             */
            if (length == 0) {
                *continue_next = 1;
            }
        }
    }
    else if (!is_timer_expired) {
        /*
        * Always retransmit in order. If not this one, then nothing.
        * But make an exception for 0-RTT packets.
        */
        if (old_p->ptype == picoquic_packet_0rtt_protected) {
            *continue_next = 1;
        }
        else {
            if (next_retransmit_time < *next_wake_time) {
                *next_wake_time = next_retransmit_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_LOSS_RECOVERY);
            }
            /* Will not continue */
            *continue_next = 0;
        }
    }
    else if (cnx->cnx_state <= picoquic_state_client_ready_start) {
        /* We do not follow the PTO logic before the connection is complete */
        int packet_is_pure_ack;
        if (old_path != NULL &&
            (((old_p->sequence_number > pkt_ctx->highest_acknowledged || pkt_ctx->highest_acknowledged == UINT64_MAX) &&
            old_p->send_time > old_path->last_loss_event_detected) ||
            old_path->last_loss_event_detected == 0)){
            old_path->nb_retransmit++;
            old_path->last_loss_event_detected = current_time;
        }
        old_p = picoquic_process_lost_packet(cnx, pkt_ctx, old_p, 1, pc, path_x, current_time,
            packet, send_buffer_max, &length, &packet_is_pure_ack, header_length);
        if (old_p == NULL || packet_is_pure_ack) {
            length = 0;
            *continue_next = 1;
        }
    }
    else {
        /* The timer is expired */

        /* Evaluate whether this packet did in fact require an acknowledgement */
        if (!picoquic_is_packet_ack_eliciting(old_p)) {
            /* if the packet did not require an acknowledgement, it can be safely
             * removed from the queue, and processing will move to the next packet.
             */

            /* If ack only packets are lost, bundle a ping next time an ACK is sent on that path */
            if (old_p->send_path != NULL && cnx->is_multipath_enabled) {
                old_p->send_path->is_ack_lost = 1;
            }
            picoquic_count_and_notify_loss(cnx, old_p, 2, current_time);
            picoquic_dequeue_retransmit_packet(cnx, pkt_ctx, old_p, 1, 0);
            length = 0;
            *continue_next = 1;
        }
        else if (old_path->is_pto_required && path_x == old_path) {
            /* A previous iteration of this loop requested a PTO repeat, and
             * the repeat has not happened yet. Just wait.
             */
            length = 0;
            *continue_next = 0;
        }
        else {
            /* We need to send a PTO. */
            int packet_is_pure_ack = 1;
            /* Parse the old packet, queue frames for retransmit, perhaps copy some
            * frames into the new packet, dequeue from packet queue and if needed
            * copy to "retransmitted", return old_p = 0 if freed.
            */
            /* TODO: there may be a special case for multipath, in which the
            * management of repeats is different */
            old_p = picoquic_process_lost_packet(cnx, pkt_ctx, old_p, 1, pc, path_x, current_time,
                packet, send_buffer_max, &length,
                &packet_is_pure_ack, header_length);

            if (packet_is_pure_ack) {
                /* this could happen if there is nothing to copy. */
                length = 0;
                *continue_next = 1;
            }
            else {
                /* we did perform a repetition */
                /* First, keep track of retransmissions per path, in order to
                * manage scheduling in multipath setup */

                if (old_path != NULL) {
                    if (path_x == old_path) {
                        old_path->is_pto_required = 1;
                    }
                    old_path->nb_retransmit++;
                    old_path->last_loss_event_detected = current_time;
                    if (cnx->is_multipath_enabled && cnx->nb_paths > 1) {
                        picoquic_retransmit_path_packet_queue(cnx, pkt_ctx, current_time);
                    }
                    if (old_path->nb_retransmit > 9 &&
                        cnx->cnx_state >= picoquic_state_ready) {
                        /* Max retransmission reached for this path */
                        DBG_PRINTF("%s\n", "Too many data retransmits, abandon path");
                        picoquic_log_app_message(cnx, "%s", "Too many data retransmits (%"PRIu64"), abandon path %" PRIu64,
                            old_path->nb_retransmit, old_path->unique_path_id);

                        if (cnx->is_multipath_enabled) {
                            int all_paths_dubious = 1;
                            for (int path_id = 0; path_id < cnx->nb_paths; path_id++) {
                                if (cnx->path[path_id]->nb_retransmit == 0) {
                                    all_paths_dubious = 0;
                                    break;
                                }
                            }
                            if (!all_paths_dubious) {
                                old_path->first_tuple->challenge_failed = 1;
                                cnx->path_demotion_needed = 1;
                            }
                        }
                        else {
                            old_path->first_tuple->challenge_failed = 1;
                            cnx->path_demotion_needed = 1;
                        }
                    }
                }
                /* Then, manage the total number of retransmissions across all paths. */
                if ((old_path == NULL || old_path->nb_retransmit > 9) &&
                    cnx->cnx_state >= picoquic_state_ready) {
                    /* TODO: only disconnect if there is no other available path */
                    int all_paths_bad = 1;
                    if (cnx->is_multipath_enabled) {
                        for (int path_id = 0; path_id < cnx->nb_paths; path_id++) {
                            if (cnx->path[path_id]->nb_retransmit <= 9) {
                                all_paths_bad = 0;
                                break;
                            }
                        }
                    }
                    if (all_paths_bad) {
                        /*
                        * Max retransmission count was exceeded. Log.
                        */
                        DBG_PRINTF("Too many retransmits of packet number %"PRIu64", disconnect", old_p->sequence_number);
                        picoquic_log_app_message(cnx, "Too many retransmits of packet number %"PRIu64", disconnect", old_p->sequence_number);

                        *continue_next = 0;
                    }
                }
            }
#ifdef TODO_CHECK_IF_PACING_ALSO_NEEDED_IN_ALL_CASES
            if (length <= packet->offset) {
                length = 0;
                packet->length = 0;
                packet->offset = 0;
                if (!packet_is_pure_ack) {
                    /* Pace down the next retransmission so as to not pile up error upon error.
                    * We only do that if theree are enough tokens in the bucket to allow at least
                    * one packet out. Otherwise, there is a risk of creating a waiting loop that
                    * only stops when all queued packets have been processed.
                    */
                    if (path_x->pacing_bucket_nanosec > path_x->pacing_packet_time_nanosec) {
                        path_x->pacing_bucket_nanosec -= path_x->pacing_packet_time_nanosec;
                    }
                }
                /*
                * If the loop is continuing, this means that we need to look
                * at the next candidate packet.
                */
                *continue_next = (timer_based_retransmit == 0);
            }
            else {
                *continue_next = 0;
            }
#endif
        }
    }

    return length;
}
```

### Rust body
```rust
    ) -> usize {
        let Some(old_p) = self
            .queued_packets
            .get(old_token)
            .map(PacketRetransmitSnapshot::from)
        else {
            *continue_next = 1;
            return 0;
        };
        let mut length = 0usize;
        *continue_next = 0;

        let old_path_idx = old_p
            .send_path
            .and_then(|send_path| self.path_index_from_token(send_path));
        if old_p.packet_type == PacketType::ZeroRttProtected
            && self.connection_state < State::ClientReadyStart
        {
            *continue_next = 1;
            return 0;
        }

        let mut is_timer_expired = false;
        let mut next_retransmit_time = *next_wake_time;
        let is_probably_lost = self.initial_repeat_needed
            || old_p.send_path.is_none()
            || self.is_packet_probably_lost(
                &old_p,
                current_time,
                &mut next_retransmit_time,
                &mut is_timer_expired,
            );

        if is_probably_lost
            && (old_p.packet_type == PacketType::Initial
                || old_p.packet_type == PacketType::Handshake)
            && old_p.length > send_buffer_max
        {
            crate::logger::Log::app_message(
                self,
                format_args!(
                    "Delay retransmission in type {:?}, seq {}, buffer too small",
                    old_p.packet_type, old_p.sequence_number
                ),
            );
            return 0;
        }

        if is_probably_lost {
            if old_p.is_ack_trap {
                self.dequeue_retransmit_packet_in_context(selection, old_token, true, false);
                *continue_next = 1;
            } else {
                let mut packet_is_pure_ack = 1;
                let kept = self.process_lost_packet(
                    selection,
                    old_token,
                    0,
                    pc,
                    path_x,
                    current_time,
                    Some(packet),
                    send_buffer_max,
                    &mut length,
                    &mut packet_is_pure_ack,
                    header_length,
                );
                if kept.is_none() || packet_is_pure_ack != 0 {
                    length = 0;
                    *continue_next = 1;
                }
                if length == 0 {
                    *continue_next = 1;
                }
            }
        } else if !is_timer_expired {
            if old_p.packet_type == PacketType::ZeroRttProtected {
                *continue_next = 1;
            } else {
                if next_retransmit_time < *next_wake_time {
                    *next_wake_time = next_retransmit_time;
                }
                *continue_next = 0;
            }
        } else if self.connection_state <= State::ClientReadyStart {
            if let Some(path_idx) = old_path_idx {
                let highest_acknowledged =
                    self.packet_context_state(selection).highest_acknowledged;
                let path = &mut self.paths[path_idx];
                if ((old_p.sequence_number > highest_acknowledged
                    || highest_acknowledged == u64::MAX)
                    && old_p.send_time > path.last_loss_event_detected)
                    || path.last_loss_event_detected.ticks() == 0
                {
                    path.nb_retransmit = path.nb_retransmit.saturating_add(1);
                    path.last_loss_event_detected = current_time;
                }
            }
            let mut packet_is_pure_ack = 1;
            let kept = self.process_lost_packet(
                selection,
                old_token,
                1,
                pc,
                path_x,
                current_time,
                Some(packet),
                send_buffer_max,
                &mut length,
                &mut packet_is_pure_ack,
                header_length,
            );
            if kept.is_none() || packet_is_pure_ack != 0 {
                length = 0;
                *continue_next = 1;
            }
        } else if !self.packet_token_is_ack_eliciting(old_token) {
            if let Some(path_idx) = old_path_idx
                && self.is_multipath_enabled
            {
                self.paths[path_idx].is_ack_lost = true;
            }
            self.count_and_notify_loss(&old_p, 2, current_time);
            self.dequeue_retransmit_packet_in_context(selection, old_token, true, false);
            length = 0;
            *continue_next = 1;
        } else if let Some(path_idx) = old_path_idx {
            if self.paths[path_idx].is_pto_required
                && path_x.unique_path_id == self.paths[path_idx].unique_path_id
            {
                length = 0;
                *continue_next = 0;
            } else {
                let mut packet_is_pure_ack = 1;
                let kept = self.process_lost_packet(
                    selection,
                    old_token,
                    1,
                    pc,
                    path_x,
                    current_time,
                    Some(packet),
                    send_buffer_max,
                    &mut length,
                    &mut packet_is_pure_ack,
                    header_length,
                );

                if packet_is_pure_ack != 0 {
                    length = 0;
                    *continue_next = 1;
                } else {
                    if kept.is_some() {
                        let path_is_same =
                            path_x.unique_path_id == self.paths[path_idx].unique_path_id;
                        if path_is_same {
                            self.paths[path_idx].is_pto_required = true;
                        }
                        self.paths[path_idx].nb_retransmit =
                            self.paths[path_idx].nb_retransmit.saturating_add(1);
                        self.paths[path_idx].last_loss_event_detected = current_time;
                        if self.is_multipath_enabled && self.paths.len() > 1 {
                            self.retransmit_path_packet_queue(selection, current_time);
                        }
                        if self.paths[path_idx].nb_retransmit > 9
                            && self.connection_state >= State::Ready
                        {
                            let nb_retransmit = self.paths[path_idx].nb_retransmit;
                            let unique_path_id = self.paths[path_idx].unique_path_id;
                            crate::logger::Log::app_message(
                                self,
                                format_args!(
                                    "Too many data retransmits ({}), abandon path {}",
                                    nb_retransmit, unique_path_id
                                ),
                            );
                            if self.is_multipath_enabled {
                                let all_paths_dubious =
                                    self.paths.iter().all(|path| path.nb_retransmit != 0);
                                if !all_paths_dubious {
                                    if let Some(tuple) = self.paths[path_idx].tuples.first_mut() {
                                        tuple.challenge_failed = true;
                                    }
                                    self.path_demotion_needed = true;
                                }
                            } else {
                                if let Some(tuple) = self.paths[path_idx].tuples.first_mut() {
                                    tuple.challenge_failed = true;
                                }
                                self.path_demotion_needed = true;
                            }
                        }
                    }

                    if self
                        .paths
                        .get(path_idx)
                        .is_none_or(|path| path.nb_retransmit > 9)
                        && self.connection_state >= State::Ready
                    {
                        let all_paths_bad = if self.is_multipath_enabled {
                            self.paths.iter().all(|path| path.nb_retransmit > 9)
                        } else {
                            true
                        };
                        if all_paths_bad {
                            crate::logger::Log::app_message(
                                self,
                                format_args!(
                                    "Too many retransmits of packet number {}, disconnect",
                                    old_p.sequence_number
                                ),
                            );
                            *continue_next = 0;
                        }
                    }
                }
            }
        } else {
            let mut packet_is_pure_ack = 1;
            let kept = self.process_lost_packet(
                selection,
                old_token,
                1,
                pc,
                path_x,
                current_time,
                Some(packet),
                send_buffer_max,
                &mut length,
                &mut packet_is_pure_ack,
                header_length,
            );
            if kept.is_none() || packet_is_pure_ack != 0 {
                length = 0;
                *continue_next = 1;
            }
        }

        length
    }
```

## Pair `picoquic/loss_recovery.c:picoquic_process_lost_packet`
C: `picoquic/loss_recovery.c:784-871 picoquic_process_lost_packet`
Rust: `rs/fq/src/internal.rs:11359-11470 process_lost_packet`

### C body
```c
{
    int do_not_detect_spurious = 1;
    int add_to_data_repeat_queue = 0;
    uint8_t* new_bytes = NULL;
    size_t checksum_length = 0;
    size_t available_buffer = 0;
    int ret = 0;
    int force_queue = 0;

    /* Manage the path MTU issues */
    picoquic_check_path_mtu_on_losses(cnx, old_p, is_timer_expired);
    /* Report loss to application, update counts */
    picoquic_count_and_notify_loss(cnx, old_p, is_timer_expired, current_time);

    /* Prepare the packet copy */
    *packet_is_pure_ack = 1;

    if (packet == NULL) {
        force_queue = 2;
    } else {
        new_bytes = packet->bytes;
        packet->sequence_number = pkt_ctx->send_sequence;
        packet->send_path = path_x;
        packet->pc = pc;
        packet->ptype = (old_p->ptype == picoquic_packet_0rtt_protected) ? picoquic_packet_1rtt_protected : old_p->ptype;

        *length = picoquic_predict_packet_header_length(cnx, packet->ptype, pkt_ctx);
        packet->offset = *length;
        *header_length = *length;

        switch (packet->ptype) {
        case picoquic_packet_1rtt_protected:
            checksum_length = picoquic_get_checksum_length(cnx, picoquic_epoch_1rtt);
            break;
        case picoquic_packet_initial:
            checksum_length = picoquic_get_checksum_length(cnx, picoquic_epoch_initial);
            break;
        case picoquic_packet_handshake:
            checksum_length = picoquic_get_checksum_length(cnx, picoquic_epoch_handshake);
            break;
        case picoquic_packet_0rtt_protected:
            checksum_length = picoquic_get_checksum_length(cnx, picoquic_epoch_0rtt);
            break;
        default:
            DBG_PRINTF("Trying to retransmit packet type %d", old_p->ptype);
            checksum_length = 0;
            break;
        }
        available_buffer = send_buffer_max - checksum_length;
    }

    ret = picoquic_copy_before_retransmit(old_p, cnx,
        new_bytes,
        available_buffer,
        packet_is_pure_ack,
        &do_not_detect_spurious, force_queue,
        length,
        &add_to_data_repeat_queue);

    if (*length <= *header_length) {
        *length = 0;
    }

    /* If ack only packets are lost, bundle a ping next time an ACK is sent on that path */
    if (old_p->send_path != NULL && cnx->is_multipath_enabled) {
        old_p->send_path->is_ack_lost = 1;
    }

    if (ret != 0) {
        DBG_PRINTF("Copy before retransmit returns %d\n", ret);
    }

    /* Update the number of bytes in transit and remove old packet from queue */
    /* If not pure ack, the packet will be placed in the "retransmitted" queue,
    * in order to enable detection of spurious restransmissions */
    /* Keep track of the path, as "old_p->send_path" will be zeroed when dequeued */
    old_p = picoquic_dequeue_retransmit_packet(cnx, pkt_ctx, old_p, *packet_is_pure_ack & do_not_detect_spurious,
        add_to_data_repeat_queue);

    return old_p;
}
```

### Rust body
```rust
    ) -> Option<PacketToken> {
        let old_p = self
            .queued_packets
            .get(old_token)
            .map(PacketRetransmitSnapshot::from)?;
        let mut do_not_detect_spurious = 1;
        let mut add_to_data_repeat_queue = 0;
        let mut force_queue = 0;
        let mut available_buffer = 0usize;

        self.check_path_mtu_on_losses(&old_p, is_timer_expired);
        self.count_and_notify_loss(&old_p, is_timer_expired, current_time);

        *packet_is_pure_ack = 1;
        if let Some(new_packet) = packet.as_mut() {
            let new_packet = &mut **new_packet;
            new_packet.sequence_number = self.packet_context_state(selection).send_sequence;
            new_packet.send_path = Some(Self::path_token_for_path(path_x));
            new_packet.packet_context = pc;
            new_packet.packet_type = if old_p.packet_type == PacketType::ZeroRttProtected {
                PacketType::OneRttProtected
            } else {
                old_p.packet_type
            };

            *length =
                self.predict_packet_header_length_for_context(new_packet.packet_type, selection);
            new_packet.offset = *length;
            *header_length = *length;
            let checksum_length = match new_packet.packet_type {
                PacketType::OneRttProtected => self.get_checksum_length(Epoch::OneRtt),
                PacketType::Initial => self.get_checksum_length(Epoch::Initial),
                PacketType::Handshake => self.get_checksum_length(Epoch::Handshake),
                PacketType::ZeroRttProtected => self.get_checksum_length(Epoch::ZeroRtt),
                _ => {
                    crate::logger::Log::app_message(
                        self,
                        format_args!("Trying to retransmit packet type {:?}", old_p.packet_type),
                    );
                    0
                }
            };
            available_buffer = send_buffer_max.saturating_sub(checksum_length);
        } else {
            force_queue = 2;
        }

        let ret = if let Some(new_packet) = packet {
            copy_before_retransmit_snapshot(
                &old_p,
                self,
                &mut new_packet.bytes,
                available_buffer,
                packet_is_pure_ack,
                &mut do_not_detect_spurious,
                force_queue,
                length,
                &mut add_to_data_repeat_queue,
            )
        } else {
            let mut empty = [];
            copy_before_retransmit_snapshot(
                &old_p,
                self,
                &mut empty,
                available_buffer,
                packet_is_pure_ack,
                &mut do_not_detect_spurious,
                force_queue,
                length,
                &mut add_to_data_repeat_queue,
            )
        };

        if *length <= *header_length {
            *length = 0;
        }

        if let Some(send_path) = old_p.send_path
            && self.is_multipath_enabled
            && let Some(path_idx) = self.path_index_from_token(send_path)
        {
            self.paths[path_idx].is_ack_lost = true;
        }

        if ret != 0 {
            crate::logger::Log::app_message(
                self,
                format_args!("Copy before retransmit returns {}", ret),
            );
        }

        self.dequeue_retransmit_packet_in_context(
            selection,
            old_token,
            (*packet_is_pure_ack & do_not_detect_spurious) != 0,
            add_to_data_repeat_queue != 0,
        )
    }
```

## Pair `picoquic/loss_recovery.c:picoquic_retransmit_path_packet_queue`
C: `picoquic/loss_recovery.c:969-1008 picoquic_retransmit_path_packet_queue`
Rust: `rs/fq/src/internal.rs:11473-11535 retransmit_path_packet_queue`

### C body
```c
{
    picoquic_packet_t* old_p = pkt_ctx->pending_first;
    int ret;

    while (old_p != NULL) {
        picoquic_packet_t* next_packet = old_p->packet_next;
        int packet_is_pure_ack = 1;
        int do_not_detect_spurious = 0;
        int add_to_data_repeat_queue = 0;
        size_t length = 0;

        /* Report loss to application, update counts */
        picoquic_count_and_notify_loss(cnx, old_p, 0, current_time);
        /* Call the copy routine but force it to not put anything in the copy */
        if ((ret = picoquic_copy_before_retransmit(old_p, cnx,
            NULL, 0, &packet_is_pure_ack, &do_not_detect_spurious, 1, &length,
            &add_to_data_repeat_queue)) != 0) {
            DBG_PRINTF("Copy before retransmit returns %d\n", ret);
        }

        /* If ack only packets are lost, bundle a ping next time an ACK is sent on that path */
        if (old_p->send_path != NULL && cnx->is_multipath_enabled) {
            old_p->send_path->is_ack_lost = 1;
        }

        /* Update the number of bytes in transit and remove old packet from queue */
        /* If not pure ack, the packet will be placed in the "retransmitted" queue,
        * in order to enable detection of spurious restransmissions */
        (void)picoquic_dequeue_retransmit_packet(cnx, pkt_ctx, old_p, packet_is_pure_ack & do_not_detect_spurious,
            add_to_data_repeat_queue);

        /* move to next packet */
        old_p = next_packet;
    }

}
```

### Rust body
```rust
    ) {
        let packets: Vec<PacketToken> = self
            .packet_context_state(selection)
            .pending
            .values()
            .copied()
            .collect();

        for old_token in packets {
            if !self.queued_packets.contains(old_token) {
                continue;
            }
            let Some(old_p) = self
                .queued_packets
                .get(old_token)
                .map(PacketRetransmitSnapshot::from)
            else {
                continue;
            };
            let mut packet_is_pure_ack = 1;
            let mut do_not_detect_spurious = 0;
            let mut add_to_data_repeat_queue = 0;
            let mut length = 0usize;

            self.count_and_notify_loss(&old_p, 0, current_time);
            let mut empty = [];
            let ret = copy_before_retransmit_snapshot(
                &old_p,
                self,
                &mut empty,
                0,
                &mut packet_is_pure_ack,
                &mut do_not_detect_spurious,
                1,
                &mut length,
                &mut add_to_data_repeat_queue,
            );
            if ret != 0 {
                crate::logger::Log::app_message(
                    self,
                    format_args!("Copy before retransmit returns {}", ret),
                );
            }

            if let Some(send_path) = old_p.send_path
                && self.is_multipath_enabled
                && let Some(path_idx) = self.path_index_from_token(send_path)
            {
                self.paths[path_idx].is_ack_lost = true;
            }

            self.dequeue_retransmit_packet_in_context(
                selection,
                old_token,
                (packet_is_pure_ack & do_not_detect_spurious) != 0,
                add_to_data_repeat_queue != 0,
            );
        }
    }
```

## Pair `picoquic/newreno.c:picoquic_newreno_sim_enter_recovery`
C: `picoquic/newreno.c:45-71 picoquic_newreno_sim_enter_recovery`
Rust: `rs/fq/src/newreno.rs:23-46 picoquic_newreno_sim_enter_recovery`

### C body
```c
{
    nr_state->ssthresh = nr_state->cwin / 2;
    if (nr_state->ssthresh < PICOQUIC_CWIN_MINIMUM) {
        nr_state->ssthresh = PICOQUIC_CWIN_MINIMUM;
    }

    if (notification == picoquic_congestion_notification_timeout) {
        nr_state->cwin = PICOQUIC_CWIN_MINIMUM;
        nr_state->alg_state = picoquic_newreno_alg_slow_start;
    }
    else {
        nr_state->cwin = nr_state->ssthresh;
        nr_state->alg_state = picoquic_newreno_alg_congestion_avoidance;
    }

    nr_state->recovery_start = current_time;
    nr_state->recovery_sequence = picoquic_cc_get_sequence_number(cnx, path_x);
    nr_state->residual_ack = 0;
}
```

### Rust body
```rust
) {
    nr_state.ssthresh = nr_state.cwin / 2;
    if nr_state.ssthresh < CWIN_MINIMUM {
        nr_state.ssthresh = CWIN_MINIMUM;
    }

    if notification == CongestionNotification::Timeout {
        nr_state.cwin = CWIN_MINIMUM;
        nr_state.alg_state = NewRenoAlgState::SlowStart;
    } else {
        nr_state.cwin = nr_state.ssthresh;
        nr_state.alg_state = NewRenoAlgState::CongestionAvoidance;
    }

    nr_state.recovery_start = current_time.ticks();
    nr_state.recovery_sequence = connection.sequence_number(path_x);
    nr_state.residual_ack = 0;
}
```

## Pair `picoquic/newreno.c:picoquic_newreno_init`
C: `picoquic/newreno.c:189-205 picoquic_newreno_init`
Rust: `rs/fq/src/newreno.rs:60-69 picoquic_newreno_init`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    picoquic_newreno_state_t* nr_state = (picoquic_newreno_state_t*)malloc(sizeof(picoquic_newreno_state_t));
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(current_time);
    UNREFERENCED_PARAMETER(option_string);
#endif

    if (nr_state != NULL) {
        picoquic_newreno_reset(nr_state, path_x);
        path_x->congestion_alg_state = nr_state;
    }
    else {
        path_x->congestion_alg_state = NULL;
    }
}
```

### Rust body
```rust
fn picoquic_newreno_init(path_x: &mut Path, _option_string: Option<&str>, _current_time: Instant) {
    let mut state = path_x
        .congestion_alg_state
        .take()
        .and_then(|boxed| boxed.downcast::<NewrenoState>().ok())
        .map(|boxed| *boxed)
        .unwrap_or_default();
    picoquic_newreno_reset(&mut state, path_x);
    path_x.congestion_alg_state = Some(Box::new(state));
}
```

## Pair `picoquic/pacing.c:picoquic_pacing_init`
C: `picoquic/pacing.c:27-35 picoquic_pacing_init`
Rust: `rs/fq/src/internal.rs:5899-5905 init`

### C body
```c
{
    pacing->evaluation_time = current_time;
    pacing->bucket_nanosec = 16;
    pacing->bucket_max = 16;
    pacing->packet_time_nanosec = 1;
    pacing->packet_time_microsec = 1;
}
```

### Rust body
```rust
    pub fn init(&mut self, current_time: Instant) {
        self.evaluation_time = current_time;
        self.bucket_nanosec = 16;
        self.bucket_max = 16;
        self.packet_time_nanosec = 1;
        self.packet_time_microsec = crate::Duration::from_ticks(1);
    }
```

## Pair `picoquic/pacing.c:picoquic_report_pacing_update`
C: `picoquic/pacing.c:107-135 picoquic_report_pacing_update`
Rust: `rs/fq/src/internal.rs:6220-6225 report_pacing_update`

### C body
```c
{
    picoquic_cnx_t* cnx = path_x->cnx;

    if (cnx->is_pacing_update_requested && path_x == cnx->path[0] &&
        cnx->callback_fn != NULL) {
        if ((pacing->rate > cnx->pacing_rate_signalled &&
            (pacing->rate - cnx->pacing_rate_signalled >= cnx->pacing_increase_threshold)) ||
            (pacing->rate < cnx->pacing_rate_signalled &&
                (cnx->pacing_rate_signalled - pacing->rate > cnx->pacing_decrease_threshold))){
            (void)cnx->callback_fn(cnx, pacing->rate, NULL, 0, picoquic_callback_pacing_changed, cnx->callback_ctx, NULL);
            cnx->pacing_rate_signalled = pacing->rate;
        }
    }
    if (cnx->is_path_quality_update_requested &&
        cnx->callback_fn != NULL) {
        /* TODO: add a function "export path quality" */
        /* TODO: remember previous signalled value for change tests */
        if (path_x->smoothed_rtt < path_x->rtt_threshold_low ||
            path_x->smoothed_rtt > path_x->rtt_threshold_high ||
            pacing->rate < path_x->pacing_rate_threshold_low ||
            pacing->rate > path_x->pacing_rate_threshold_high) {
            (void)cnx->callback_fn(cnx, path_x->unique_path_id, NULL, 0, picoquic_callback_path_quality_changed, cnx->callback_ctx, path_x->app_path_ctx);
            picoquic_refresh_path_quality_thresholds(path_x);
        }
    }
}
```

### Rust body
```rust
        if path_index < self.paths.len() {
            let mut path = self.paths.remove(path_index);
            self.report_pacing_update_for_path(&mut path, path_index == 0);
            self.paths.insert(path_index, path);
        }
```

## Pair `picoquic/pacing.c:picoquic_update_pacing_after_send`
C: `picoquic/pacing.c:247-251 picoquic_update_pacing_after_send`
Rust: `rs/fq/src/internal.rs:6086-6089 update_pacing_after_send`

### C body
```c
{
    picoquic_update_pacing_data_after_send(&path_x->pacing, length, path_x->send_mtu, current_time);
}
```

### Rust body
```rust
    pub fn update_pacing_after_send(&mut self, length: usize, current_time: Instant) {
        self.pacing
            .update_after_send(length, self.send_mtu, current_time);
    }
```

## Pair `picoquic/packet.c:picoquic_parse_long_packet_type`
C: `picoquic/packet.c:38-83 picoquic_parse_long_packet_type`
Rust: `rs/fq/src/internal.rs:6490-6526 parse_long_packet_type`

### C body
```c
{
    picoquic_packet_type_enum pt = picoquic_packet_error;

    switch (picoquic_supported_versions[version_index].packet_type_version) {
    case PICOQUIC_V1_VERSION:
        switch ((flags >> 4) & 3) {
        case 0: /* Initial */
            pt = picoquic_packet_initial;
            break;
        case 1: /* 0-RTT Protected */
            pt = picoquic_packet_0rtt_protected;
            break;
        case 2: /* Handshake */
            pt = picoquic_packet_handshake;
            break;
        case 3: /* Retry */
            pt = picoquic_packet_retry;
            break;
        }
        break;
    case PICOQUIC_V2_VERSION:
        /* Initial packets use a packet type field of 0b01. */
        /* 0-RTT packets use a packet type field of 0b10. */
        /* Handshake packets use a packet type field of 0b11. */
        /* Retry packets use a packet type field of 0b00.*/
        switch ((flags >> 4) & 3) {
        case 1: /* Initial */
            pt = picoquic_packet_initial;
            break;
        case 2: /* 0-RTT Protected */
            pt = picoquic_packet_0rtt_protected;
            break;
        case 3: /* Handshake */
            pt = picoquic_packet_handshake;
            break;
        case 0: /* Retry */
            pt = picoquic_packet_retry;
            break;
        }
        break;
    default:
        break;
    }
    return pt;
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

## Pair `picoquic/packet.c:picoquic_parse_packet_header`
C: `picoquic/packet.c:475-497 picoquic_parse_packet_header`
Rust: `rs/fq/src/internal.rs:6578-6600 parse_packet_header`

### C body
```c
{
    int ret = 0;

    /* Initialize the PH structure to zero, but version index to -1 (error) */
    memset(ph, 0, sizeof(picoquic_packet_header));
    ph->version_index = -1;

    /* Is this a long header or a short header? -- in any case, we need at least 17 bytes */
    if ((bytes[0] & 0x80) == 0x80) {
        ret = picoquic_parse_long_packet_header(quic, bytes, length, addr_from, ph, pcnx);
    } else {
        ret = picoquic_parse_short_packet_header(quic, bytes, length, addr_from, ph, pcnx, receiving);
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<Option<ConnectionToken>, crate::Error> {
        *ph = PacketHeader::default();
        ph.version_index = -1;

        if bytes.is_empty() {
            return Err(crate::Error::InvalidArgument);
        }

        let conn_tok = if (bytes[0] & 0x80) == 0x80 {
            // Long header
            self.parse_long_packet_header_inner(bytes, addr_from, ph)
        } else {
            // Short header
            self.parse_short_packet_header_inner(bytes, addr_from, ph, receiving)
        };
        Ok(conn_tok)
    }
```

## Pair `picoquic/packet.c:picoquic_remove_packet_protection`
C: `picoquic/packet.c:633-768 picoquic_remove_packet_protection`
Rust: `rs/fq/src/internal.rs:7257-7271 remove_packet_protection`

### C body
```c
{
    size_t decoded;
    int ret = 0;

    /* verify that the packet is new */
    if (already_received != NULL) {
        if (picoquic_is_pn_already_received(cnx, ph->pc, ph->l_cid, ph->pn64) != 0) {
            /* Set error type: already received */
            *already_received = 1;
        }
        else {
            *already_received = 0;
        }
    }

    if (ph->epoch == picoquic_epoch_1rtt) {
        int need_integrity_check = 1;
        picoquic_ack_context_t* ack_ctx = picoquic_ack_ctx_from_cnx_context(cnx, picoquic_packet_context_application, ph->l_cid);

        /* Manage key rotation */
        if (ph->key_phase == cnx->key_phase_dec) {
            /* AEAD Decrypt */
            if (cnx->is_multipath_enabled && ph->ptype == picoquic_packet_1rtt_protected) {
                decoded = picoquic_aead_decrypt_mp(decoded_bytes + ph->offset,
                    bytes + ph->offset,
                    ph->payload_length, 
                    ph->l_cid->path_id, ph->pn64, decoded_bytes, ph->offset,
                    cnx->crypto_context[picoquic_epoch_1rtt].aead_decrypt);
            } else {
                decoded = picoquic_aead_decrypt_generic(decoded_bytes + ph->offset,
                    bytes + ph->offset, ph->payload_length, ph->pn64, decoded_bytes, ph->offset, 
                    cnx->crypto_context[picoquic_epoch_1rtt].aead_decrypt);
            }
            if (decoded <= ph->payload_length && ph->pn64 < ack_ctx->crypto_rotation_sequence) {
                ack_ctx->crypto_rotation_sequence = ph->pn64;
            }
        }
        else if ((ack_ctx->crypto_rotation_sequence == UINT64_MAX && current_time <= cnx->crypto_rotation_time_guard) ||
            ph->pn64 < ack_ctx->crypto_rotation_sequence) {
            /* This packet claims to be encoded with the old key */
            if (current_time > cnx->crypto_rotation_time_guard) {
                /* Too late. Ignore the packet. Could be some kind of attack. */
                decoded = ph->payload_length + 1;
                need_integrity_check = 0;
            }
            else if (cnx->crypto_context_old.aead_decrypt != NULL) {
                if (cnx->is_multipath_enabled) {
                    decoded = picoquic_aead_decrypt_mp(decoded_bytes + ph->offset, bytes + ph->offset, ph->payload_length,
                        ph->l_cid->path_id, ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context_old.aead_decrypt);
                }
                else {
                    decoded = picoquic_aead_decrypt_generic(decoded_bytes + ph->offset, bytes + ph->offset, ph->payload_length,
                        ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context_old.aead_decrypt);
                }
            }
            else {
                /* old context is either not yet available, or already removed */
                decoded = ph->payload_length + 1;
                need_integrity_check = 0;
            }
        }
        else {
            /* TODO: check that this is larger than last received with current key */
            /* These could only be a new key */
            if (cnx->crypto_context_new.aead_decrypt == NULL &&
                cnx->crypto_context_new.aead_encrypt == NULL) {
                /* If the new context was already computed, don't do it again */
                ret = picoquic_compute_new_rotated_keys(cnx);
            }
            /* if decoding succeeds, the rotation should be validated */
            if (ret == 0 && cnx->crypto_context_new.aead_decrypt != NULL) {
                if (cnx->is_multipath_enabled) {
                    decoded = picoquic_aead_decrypt_mp(decoded_bytes + ph->offset, bytes + ph->offset, ph->payload_length,
                        ph->l_cid->path_id, ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context_new.aead_decrypt);

                }
                else {
                    decoded = picoquic_aead_decrypt_generic(decoded_bytes + ph->offset,
                        bytes + ph->offset, ph->payload_length, ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context_new.aead_decrypt);
                }
                if (decoded <= ph->payload_length) {
                    /* Rotation only if the packet was correctly decrypted with the new key */
                    cnx->crypto_rotation_time_guard = current_time + cnx->path[0]->retransmit_timer;
                    if (cnx->is_multipath_enabled) {
                        for (int i=0; i < cnx->nb_paths; i++){
                            cnx->path[i]->ack_ctx.crypto_rotation_sequence = UINT64_MAX;
                        }
                    }
                    ack_ctx->crypto_rotation_sequence = ph->pn64;
                    picoquic_apply_rotated_keys(cnx, 0);
                    cnx->nb_crypto_key_rotations++;

                    if (cnx->crypto_context_new.aead_encrypt != NULL) {
                        /* If that move was not already validated, move to the new encryption keys */
                        picoquic_apply_rotated_keys(cnx, 1);
                    }
                }
            }
            else {
                /* new context could not be computed  */
                decoded = ph->payload_length + 1;
                need_integrity_check = 0;
            }
        }

        if (need_integrity_check && decoded > ph->payload_length) {
            cnx->crypto_failure_count++;
            if (cnx->crypto_failure_count > picoquic_aead_integrity_limit(cnx->crypto_context[picoquic_epoch_1rtt].aead_decrypt)) {
                picoquic_log_app_message(cnx, "AEAD Integrity limit reached after 0x%" PRIx64 " failed decryptions.", cnx->crypto_failure_count);
                (void)picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_AEAD_LIMIT_REACHED, 0);
            }
        }
    }
    else {
        /* TODO: get rid of handshake some time after handshake complete */
        /* For all the other epochs, there is a single crypto context and no key rotation */
        if (cnx->crypto_context[ph->epoch].aead_decrypt != NULL) {
            decoded = picoquic_aead_decrypt_generic(decoded_bytes + ph->offset,
                bytes + ph->offset, ph->payload_length, ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context[ph->epoch].aead_decrypt);
        }
        else {
            decoded = ph->payload_length + 1;
        }
    }

    /* by conventions, values larger than input indicate error */
    return decoded;
}
```

### Rust body
```rust
        if let Some(already_received) = already_received {
            *already_received = self.is_pn_already_received(
                ph.packet_context,
                ph.local_connection_id,
                ph.packet_number_full,
            );
        }
```

## Pair `picoquic/packet.c:picoquic_process_unexpected_cnxid`
C: `picoquic/packet.c:1077-1138 picoquic_process_unexpected_cnxid`
Rust: `rs/fq/src/internal.rs:769-832 process_unexpected_cnxid`

### C body
```c
{
    if (length > PICOQUIC_RESET_PACKET_MIN_SIZE && 
        ph->ptype == picoquic_packet_1rtt_protected &&
        quic->stateless_reset_next_time <= current_time) {
        picoquic_stateless_packet_t* sp = picoquic_create_stateless_packet(quic);
        if (sp != NULL) {
            size_t pad_size = length - PICOQUIC_RESET_SECRET_SIZE - 2;
            uint8_t* bytes = sp->bytes;
            size_t byte_index = 0;

            if (pad_size > PICOQUIC_RESET_PACKET_MIN_SIZE - PICOQUIC_RESET_SECRET_SIZE - 1) {
                pad_size -= (size_t)picoquic_public_uniform_random(pad_size - (PICOQUIC_RESET_PACKET_MIN_SIZE - PICOQUIC_RESET_SECRET_SIZE - 1));
            }

            /* Packet type set to short header, randomize the 5 lower bits */
            bytes[byte_index++] = 0x40 | (uint8_t)(picoquic_public_random_64() & 0x3F);

            /* Add the random bytes */
            picoquic_public_random(bytes + byte_index, pad_size);
            byte_index += pad_size;
            /* Add the public reset secret */
            (void)picoquic_create_cnxid_reset_secret(quic, &ph->dest_cnx_id, bytes + byte_index);
            byte_index += PICOQUIC_RESET_SECRET_SIZE;
            sp->length = byte_index;
            sp->ptype = picoquic_packet_1rtt_protected;
            picoquic_store_addr(&sp->addr_to, addr_from);
            picoquic_store_addr(&sp->addr_local, addr_to);
            sp->if_index_local = if_index_to;
            sp->initial_cid = ph->dest_cnx_id;
            sp->cnxid_log64 = picoquic_val64_connection_id(sp->initial_cid);

            picoquic_log_context_free_app_message(quic, &sp->initial_cid, "Unexpected connection ID, sending stateless reset.\n");

            picoquic_queue_stateless_packet(quic, sp);
            quic->stateless_reset_next_time = current_time + quic->stateless_reset_min_interval;
        }
    }
}
```

### Rust body
```rust
    ) {
        if length <= crate::RESET_PACKET_MIN_SIZE
            || ph.packet_type != PacketType::OneRttProtected
            || self.stateless_reset_next_time > current_time
        {
            return;
        }

        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };

        let min_pad = crate::RESET_PACKET_MIN_SIZE - crate::RESET_SECRET_SIZE - 1;
        let mut pad_size = length.saturating_sub(crate::RESET_SECRET_SIZE + 2);
        if pad_size > min_pad {
            pad_size -= public_uniform_random((pad_size - min_pad) as u64) as usize;
        }

        if 1 + pad_size + crate::RESET_SECRET_SIZE > sp.bytes.len() {
            return;
        }

        let mut byte_index = 0usize;
        sp.bytes[byte_index] = 0x40 | (crate::public_random_64() as u8 & 0x3f);
        byte_index += 1;

        public_random(&mut sp.bytes[byte_index..byte_index + pad_size]);
        byte_index += pad_size;

        let mut reset_secret = [0u8; RESET_SECRET_SIZE];
        if self
            .create_connection_id_reset_secret(&ph.dest_connection_id, &mut reset_secret)
            .is_err()
        {
            return;
        }
        sp.bytes[byte_index..byte_index + RESET_SECRET_SIZE].copy_from_slice(&reset_secret);
        byte_index += RESET_SECRET_SIZE;

        sp.length = byte_index;
        sp.packet_type = PacketType::OneRttProtected;
        sp.addr_to = *addr_from;
        sp.addr_local = *addr_to;
        sp.if_index_local = if_index_to;
        sp.initial_connection_id = ph.dest_connection_id;
        sp.connection_id_log64 = sp.initial_connection_id.val64();

        self.log_app_message(
            &sp.initial_connection_id,
            format_args!("Unexpected connection ID, sending stateless reset.\n"),
        );

        self.queue_stateless_packet(sp);
        self.stateless_reset_next_time =
            instant_after_delay(current_time, self.stateless_reset_min_interval);
    }
```
