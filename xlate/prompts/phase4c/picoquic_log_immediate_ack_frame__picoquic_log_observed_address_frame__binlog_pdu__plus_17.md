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

## Pair `picoquic/logwriter.c:picoquic_log_immediate_ack_frame`
C: `picoquic/logwriter.c:485-493 picoquic_log_immediate_ack_frame`
Rust: `rs/fq/src/binlog.rs:628-634 log_immediate_ack_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;

    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* frame type as varint */
    picoquic_binlog_frame(f, bytes_begin, bytes);

    return bytes;
}
```

### Rust body
```rust
fn log_immediate_ack_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let bytes = frames_varint_skip(bytes_in)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:picoquic_log_observed_address_frame`
C: `picoquic/logwriter.c:534-548 picoquic_log_observed_address_frame`
Rust: `rs/fq/src/binlog.rs:662-676 log_observed_address_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    size_t ip_len = ((ftype & 1) == 0) ? 4 : 16;
    size_t data_len = ip_len + 2;


    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Frame type */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* Sequence number */
    bytes = picoquic_log_fixed_skip(bytes, bytes_max, data_len); /* IP address and port */

    picoquic_binlog_frame(f, bytes_begin, bytes);

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let ip_len = if (ftype & 1) == 0 { 4 } else { 16 };
    let data_len = ip_len + 2;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = skip_fixed(bytes, data_len)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```

## Pair `picoquic/logwriter.c:binlog_pdu`
C: `picoquic/logwriter.c:700-722 binlog_pdu`
Rust: `rs/fq/src/binlog.rs:800-832 pdu`

### C body
```c
{
    bytestream_buf stream_msg;
    bytestream* msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);

    /* Common chunk header */
    binlog_compose_event_header(msg, cid, current_time, 0, picoquic_log_event_pdu_sent + receiving);

    /* PDU information */
    bytewrite_addr(msg, addr_peer);
    bytewrite_vint(msg, packet_length);
    bytewrite_addr(msg, addr_local);
    bytewrite_vint(msg, unique_path_id);
    bytewrite_int8(msg, ecn);

    uint8_t head[4] = { 0 };
    picoformat_32(head, (uint32_t)bytestream_length(msg));

    (void)fwrite(head, sizeof(head), 1, f);
    (void)fwrite(bytestream_data(msg), bytestream_length(msg), 1, f);
}
```

### Rust body
```rust
) {
    let mut buf = ByteStreamBuf::default();
    let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
        return;
    };

    let event = if receiving {
        LogEventType::PduRecv
    } else {
        LogEventType::PduSent
    };
    compose_event_header(&mut msg, cid, current_time, 0, event);

    let _ = msg.write_addr(addr_peer);
    let _ = msg.write_varint(packet_length as u64);
    let _ = msg.write_addr(addr_local);
    let _ = msg.write_varint(unique_path_id);
    let _ = msg.write_u8(ecn);

    let payload: Vec<u8> = msg.as_bytes().to_vec();
    drop(msg);
    write_record(f, &payload);
}
```

## Pair `picoquic/logwriter.c:binlog_dropped_packet`
C: `picoquic/logwriter.c:799-818 binlog_dropped_packet`
Rust: `rs/fq/src/binlog.rs:956-1530 dropped_packet`

### C body
```c
{
    FILE* f = cnx->f_binlog;
    bytestream_buf stream_msg;
    bytestream* msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);

    bytewrite_int32(msg, 0);
    /* Common chunk header */
    binlog_compose_event_header(msg, &cnx->initial_cnxid, current_time, binlog_get_path_id(cnx, path_x),
        picoquic_log_event_packet_dropped);
    /* Event header */
    bytewrite_vint(msg, ph->ptype);
    bytewrite_vint(msg, packet_size);
    bytewrite_vint(msg, err);

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

## Pair `picoquic/logwriter.c:binlog_negotiated_alpn`
C: `picoquic/logwriter.c:921-957 binlog_negotiated_alpn`
Rust: `rs/fq/src/binlog.rs:1014-1530 negotiated_alpn`

### C body
```c
{
    FILE* f = cnx->f_binlog;

    bytestream_buf stream_msg;
    bytestream* msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);
    /* Common chunk header */
    binlog_compose_event_header(msg, &cnx->initial_cnxid, picoquic_get_quic_time(cnx->quic), 0, picoquic_log_event_alpn_update);
    /* Event header */
    bytewrite_vint(msg, is_local);
    bytewrite_vint(msg, sni_len);
    if (sni_len > 0) {
        bytewrite_buffer(msg, sni, sni_len);
    }

    bytewrite_vint(msg, alpn_count);
    if (alpn_count > 0) {
        for (size_t i = 0; i < alpn_count; i++) {
            bytewrite_vint(msg, alpn_list[i].len);
            bytewrite_buffer(msg, alpn_list[i].base, alpn_list[i].len);
        }
    }

    bytewrite_vint(msg, alpn_len);
    if (alpn_len > 0) {
        bytewrite_buffer(msg, alpn, alpn_len);
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

## Pair `picoquic/logwriter.c:binlog_new_connection`
C: `picoquic/logwriter.c:1013-1089 binlog_new_connection`
Rust: `rs/fq/src/binlog.rs:1033-1530 new_connection`

### C body
```c
{
    char const* bin_dir = (cnx->quic->binlog_dir == NULL) ? cnx->quic->qlog_dir : cnx->quic->binlog_dir;

    if (bin_dir == NULL || cnx->quic->bin_log_fns == NULL) {
        return;
    }

    if (cnx->quic->current_number_of_open_logs >= cnx->quic->max_simultaneous_logs) {
        return;
    }

    int ret = 0;

    cnx->f_binlog = picoquic_file_close(cnx->f_binlog);
    
    char cid_name[2 * PICOQUIC_CONNECTION_ID_MAX_SIZE + 1];
    if (picoquic_print_connection_id_hexa(cid_name, sizeof(cid_name), &cnx->initial_cnxid) != 0) {
        ret = -1;
    }

    char log_filename[512];
    if (ret == 0) {
        int sprintf_ret = -1;
        if (cnx->quic->use_unique_log_names) {
            sprintf_ret = picoquic_sprintf(log_filename, sizeof(log_filename), NULL, "%s%s%s.%x.%s.log",
                bin_dir, PICOQUIC_FILE_SEPARATOR, cid_name, cnx->log_unique,
                (cnx->client_mode) ? "client" : "server");
        }
        else {
            sprintf_ret = picoquic_sprintf(log_filename, sizeof(log_filename), NULL, "%s%s%s.%s.log",
                bin_dir, PICOQUIC_FILE_SEPARATOR, cid_name,
                (cnx->client_mode) ? "client" : "server");
        }
        if (sprintf_ret != 0) {
            ret = -1;
        }
        else {
            picoquic_string_free(cnx->binlog_file_name);
            cnx->binlog_file_name = picoquic_string_duplicate(log_filename);
        }
    }

    if (ret == 0) {
        cnx->f_binlog = create_binlog(log_filename, picoquic_get_quic_time(cnx->quic),
           cnx->local_parameters.initial_max_path_id > 0);
        if (cnx->f_binlog == NULL) {
            cnx->binlog_file_name = picoquic_string_free(cnx->binlog_file_name);
            ret = -1;
        }
        else {
            cnx->quic->current_number_of_open_logs++;
        }
    }

    if (ret == 0) {
        bytestream_buf stream_msg;
        bytestream * msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);
        /* Common chunk header */
        binlog_compose_event_header(msg, &cnx->initial_cnxid, cnx->start_time, 0, picoquic_log_event_new_connection);

        bytewrite_int8(msg, cnx->client_mode != 0);
        bytewrite_int32(msg, cnx->proposed_version);
        bytewrite_cid(msg, &cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id);

        /* Algorithms used */
        bytewrite_cstr(msg, cnx->congestion_alg->congestion_algorithm_id);
        bytewrite_vint(msg, cnx->spin_policy);

        bytestream_buf stream_head;
        bytestream * head = bytestream_buf_init(&stream_head, 8);
        bytewrite_int32(head, (uint32_t)bytestream_length(msg));

        (void)fwrite(bytestream_data(head), bytestream_length(head), 1, cnx->f_binlog);
        (void)fwrite(bytestream_data(msg), bytestream_length(msg), 1, cnx->f_binlog);
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

## Pair `picoquic/logwriter.c:picoquic_binlog_message_v`
C: `picoquic/logwriter.c:1237-1272 picoquic_binlog_message_v`
Rust: `rs/fq/src/binlog.rs:1025-1530 message_v`

### C body
```c
{
    if (cnx->f_binlog == NULL) {
        return;
    }
    bytestream_buf stream_msg;
    bytestream* ps_msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);
    size_t message_len;
    char* message_text;
    int written = -1;
    /* Common chunk header */
    binlog_compose_event_header(ps_msg, &cnx->initial_cnxid, picoquic_get_quic_time(cnx->quic), 0, picoquic_log_event_info_message);

    message_text = (char*)(ps_msg->data + ps_msg->ptr);
#ifdef _WINDOWS
    written = vsnprintf_s(message_text,
        ps_msg->size - ps_msg->ptr, _TRUNCATE, fmt, vargs);
    message_len = (written < 0) ? ps_msg->size - ps_msg->ptr - 1 : (size_t)written;
#else
    written = vsnprintf(message_text, ps_msg->size - ps_msg->ptr, fmt, vargs);
    if (written < 0 || (size_t)written >= ps_msg->size - ps_msg->ptr){
        message_len = ps_msg->size - ps_msg->ptr - 1;
    } else {
        message_len = (size_t)written;
    }
#endif
    ps_msg->ptr += message_len;

    bytestream_buf stream_head;
    bytestream* ps_head = bytestream_buf_init(&stream_head, BYTESTREAM_MAX_BUFFER_SIZE);

    bytewrite_int32(ps_head, (uint32_t)bytestream_length(ps_msg));

    (void)fwrite(bytestream_data(ps_head), bytestream_length(ps_head), 1, cnx->f_binlog);
    (void)fwrite(bytestream_data(ps_msg), bytestream_length(ps_msg), 1, cnx->f_binlog);
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

## Pair `picoquic/logwriter.c:picoquic_enable_binlog`
C: `picoquic/logwriter.c:1346-1349 picoquic_enable_binlog`
Rust: `rs/fq/src/binlog.rs:1584-1597 enable_binlog`

### C body
```c
{
    quic->bin_log_fns = &binlog_functions;
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

## Pair `picoquic/loss_recovery.c:picoquic_is_packet_probably_lost`
C: `picoquic/loss_recovery.c:535-644 picoquic_is_packet_probably_lost`
Rust: `rs/fq/src/internal.rs:11222-11348 is_packet_probably_lost`

### C body
```c
{
    uint64_t retransmit_time = UINT64_MAX;
    int64_t delta_seq = 0;
    int64_t delta_sent = 0;
    uint64_t rack_timer_min;
    int is_probably_lost = 0;

    *is_timer_expired = 0;

    if (old_p->ptype == picoquic_packet_0rtt_protected && !cnx->zero_rtt_data_accepted) {
        /* Zero RTT data was not accepted by the peer, the packets are considered lost */
        retransmit_time = current_time;
        is_probably_lost = 1;
    }
    else if (old_p->ptype == picoquic_packet_0rtt_protected && cnx->cnx_state != picoquic_state_ready &&
        cnx->cnx_state != picoquic_state_client_ready_start) {
        /* Set the retransmit time ahead of current time since the connection is not ready */
        retransmit_time = current_time + old_p->send_path->smoothed_rtt + PICOQUIC_RACK_DELAY;
    }
    else {
        picoquic_packet_context_t* pkt_ctx = (cnx->is_multipath_enabled && old_p->pc == picoquic_packet_context_application) ?
            &old_p->send_path->pkt_ctx : &cnx->pkt_ctx[old_p->pc];
        delta_seq = pkt_ctx->highest_acknowledged - old_p->sequence_number;

        if (delta_seq >= 3) {
            /* Last acknowledged packet is ways ahead. That means this packet
            * is most probably lost.
            */
            retransmit_time = current_time;
            is_probably_lost = 1;
        }
        else if (delta_seq > 0) {
            /* Set a timer relative to that last packet */
            int64_t rack_delay = (old_p->send_path->smoothed_rtt >> 2);
            delta_sent = pkt_ctx->latest_time_acknowledged - old_p->send_time;
            if (rack_delay > PICOQUIC_RACK_DELAY / 2) {
                rack_delay = PICOQUIC_RACK_DELAY / 2;
            }
            retransmit_time = old_p->send_time + old_p->send_path->retransmit_timer;
            rack_timer_min = pkt_ctx->highest_acknowledged_time + rack_delay
                - delta_sent + cnx->remote_parameters.max_ack_delay;
            if (retransmit_time > rack_timer_min) {
                retransmit_time = rack_timer_min;
            }
            if (retransmit_time <= current_time || old_p->is_ack_trap) {
                is_probably_lost = 1;
            }
        }
    }
    if (!is_probably_lost) {
        /* Find the last packet in the queue, which may be this one.
        * Compute a timer from the time this last packet was sent.
        * If the timer has elapsed, this packet should be resent,
        * in a timer based manner. If not, set the timer to
        * the specified value. */
        uint64_t retransmit_time_timer;
        picoquic_packet_t* last_packet = picoquic_get_last_packet(cnx, old_p->send_path, old_p->pc);

        if (last_packet == NULL) {
            last_packet = old_p;
        }
        retransmit_time_timer = last_packet->send_time + picoquic_current_retransmit_timer(cnx, old_p->send_path);

        if (current_time >= retransmit_time_timer) {
            if (old_p->send_path->path_is_demoted) {
                /* if the path is demoted, treat this as a simple loss */
                is_probably_lost = 1;
            }
            else {
                /* Do not set the "probably lost" condition, because timers are unreliable */
                *is_timer_expired = 1;
            }
        }
        else if (old_p->send_path->nb_retransmit == 0) {
            /* RACK has failure modes if the sender keeps adding small packets to the
             * retransmit queue. This may push the send time of the "last" packet
             * beyond a reasonable value.
             * In that case, we pick a safe timer based retransmit.
             * The "timer" condition will have consequences on congestion control;
             * we only set it if the packet is ack eliciting.
             */
            uint64_t alt_retransmit_timer = old_p->send_time + 2*picoquic_current_retransmit_timer(cnx, old_p->send_path);

            if (alt_retransmit_timer < last_packet->send_time) {
                retransmit_time_timer = alt_retransmit_timer;
                if (current_time >= retransmit_time_timer) {
                    if (picoquic_is_packet_ack_eliciting(old_p))
                    {
                        *is_timer_expired = 1;
                    }
                    else {
                        is_probably_lost = 1;
                    }
                }
            }
        }

        if (retransmit_time_timer < retransmit_time) {
            retransmit_time = retransmit_time_timer;
        }
    }
    if (*next_retransmit_time > retransmit_time) {
        *next_retransmit_time = retransmit_time;
    }

    return is_probably_lost;
}
```

### Rust body
```rust
    ) -> bool {
        let mut retransmit_time = Instant::from_ticks(u64::MAX);
        let mut is_probably_lost = false;
        *is_timer_expired = false;

        let path_idx = old_p
            .send_path
            .and_then(|send_path| self.path_index_from_token(send_path));
        let Some(path_idx) = path_idx else {
            *next_retransmit_time = current_time;
            return true;
        };
        let path = &self.paths[path_idx];

        if old_p.packet_type == PacketType::ZeroRttProtected && !self.zero_rtt_data_accepted {
            retransmit_time = current_time;
            is_probably_lost = true;
        } else if old_p.packet_type == PacketType::ZeroRttProtected
            && self.connection_state != State::Ready
            && self.connection_state != State::ClientReadyStart
        {
            retransmit_time = Instant::from_ticks(
                current_time
                    .ticks()
                    .saturating_add(path.smoothed_rtt.ticks())
                    .saturating_add(RACK_DELAY.ticks()),
            );
        } else {
            let selection = self.packet_context_for_packet(old_p);
            let pkt_ctx = self.packet_context_state(selection);
            let delta_seq = pkt_ctx
                .highest_acknowledged
                .wrapping_sub(old_p.sequence_number) as i64;

            if delta_seq >= 3 {
                retransmit_time = current_time;
                is_probably_lost = true;
            } else if delta_seq > 0 {
                let mut rack_delay = (path.smoothed_rtt.ticks() >> 2) as i64;
                if rack_delay > (RACK_DELAY.ticks() / 2) as i64 {
                    rack_delay = (RACK_DELAY.ticks() / 2) as i64;
                }
                let delta_sent = pkt_ctx
                    .latest_time_acknowledged
                    .ticks()
                    .wrapping_sub(old_p.send_time.ticks()) as i64;
                retransmit_time = Instant::from_ticks(
                    old_p
                        .send_time
                        .ticks()
                        .saturating_add(path.retransmit_timer.ticks()),
                );
                let mut rack_timer_min = pkt_ctx
                    .highest_acknowledged_time
                    .ticks()
                    .saturating_add(rack_delay.max(0) as u64);
                if delta_sent >= 0 {
                    rack_timer_min = rack_timer_min.saturating_sub(delta_sent as u64);
                } else {
                    rack_timer_min = rack_timer_min.saturating_add((-delta_sent) as u64);
                }
                rack_timer_min =
                    rack_timer_min.saturating_add(self.remote_parameters.max_ack_delay as u64);
                if retransmit_time.ticks() > rack_timer_min {
                    retransmit_time = Instant::from_ticks(rack_timer_min);
                }
                if retransmit_time <= current_time || old_p.is_ack_trap {
                    is_probably_lost = true;
                }
            }
        }

        if !is_probably_lost {
            let selection = self.packet_context_for_packet(old_p);
            let last_packet = self
                .packet_context_state(selection)
                .pending
                .values()
                .next_back()
                .and_then(|token| self.queued_packets.get(*token))
                .map(PacketRetransmitSnapshot::from)
                .unwrap_or(*old_p);
            let current_timer = self.current_retransmit_timer_ticks_for_path(path);
            let mut retransmit_time_timer =
                Instant::from_ticks(last_packet.send_time.ticks().saturating_add(current_timer));

            if current_time >= retransmit_time_timer {
                if path.path_is_demoted {
                    is_probably_lost = true;
                } else {
                    *is_timer_expired = true;
                }
            } else if path.nb_retransmit == 0 {
                let alt_retransmit_timer = Instant::from_ticks(
                    old_p
                        .send_time
                        .ticks()
                        .saturating_add(2u64.saturating_mul(current_timer)),
                );
                if alt_retransmit_timer < last_packet.send_time {
                    retransmit_time_timer = alt_retransmit_timer;
                    if current_time >= retransmit_time_timer {
                        if packet_is_ack_eliciting_from_snapshot(old_p) {
                            *is_timer_expired = true;
                        } else {
                            is_probably_lost = true;
                        }
                    }
                }
            }

            if retransmit_time_timer < retransmit_time {
                retransmit_time = retransmit_time_timer;
            }
        }

        if *next_retransmit_time > retransmit_time {
            *next_retransmit_time = retransmit_time;
        }
        is_probably_lost
    }
```

## Pair `picoquic/loss_recovery.c:picoquic_check_path_mtu_on_losses`
C: `picoquic/loss_recovery.c:873-893 picoquic_check_path_mtu_on_losses`
Rust: `rs/fq/src/internal.rs:11067-11074 check_path_mtu_on_losses`

### C body
```c
{
    if (old_p->send_path != NULL &&
        ((old_p->length + old_p->checksum_overhead) == old_p->send_path->send_mtu || timer_based_retransmit) &&
        cnx->cnx_state >= picoquic_state_ready) {
        old_p->send_path->nb_mtu_losses++;
        if (old_p->send_path->nb_mtu_losses > PICOQUIC_MTU_LOSS_THRESHOLD || timer_based_retransmit) {
            size_t old_mtu = old_p->send_path->send_mtu;
            picoquic_reset_path_mtu(old_p->send_path);
            if (old_mtu != old_p->send_path->send_mtu) {
                picoquic_log_app_message(cnx,
                    "Reset path %" PRIu64 " MTU after %" PRIu64 " retransmissions, %" PRIu64 "MTU losses, Timer mode : %d",
                    old_p->send_path->unique_path_id,
                    old_p->send_path->nb_retransmit,
                    old_p->send_path->nb_mtu_losses,
                    timer_based_retransmit);
            }
        }
    }
}
```

### Rust body
```rust
        let Some(send_path) = old_p.send_path else {
            return;
        };
```

## Pair `picoquic/loss_recovery.c:picoquic_retransmit_demoted_path`
C: `picoquic/loss_recovery.c:1010-1026 picoquic_retransmit_demoted_path`
Rust: `rs/fq/src/internal.rs:4736-4746 retransmit_demoted_path`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = NULL;

    if (cnx->cnx_state == picoquic_state_ready && cnx->nb_paths > 1) {
        if (cnx->is_multipath_enabled) {
            pkt_ctx = &path_x->pkt_ctx;
        }
        else {
            pkt_ctx = &cnx->pkt_ctx[picoquic_packet_context_application];
        }
        if (pkt_ctx != NULL) {
            picoquic_retransmit_path_packet_queue(cnx, pkt_ctx, current_time);
        }
    }
}
```

### Rust body
```rust
        {
            if let Some(packet) = self.queued_packets.get_mut(token) {
                packet.is_queued_for_retransmit = true;
            }
        }
```

## Pair `picoquic/newreno.c:picoquic_newreno_sim_seed_cwin`
C: `picoquic/newreno.c:73-85 picoquic_newreno_sim_seed_cwin`
Rust: `rs/fq/src/cc_common.rs:536-545 seed_cwin`

### C body
```c
{
    if (nr_state->alg_state == picoquic_newreno_alg_slow_start &&
        nr_state->ssthresh == UINT64_MAX) {
        if (seed_cwin > nr_state->cwin) {
            nr_state->cwin = seed_cwin;
            nr_state->ssthresh = seed_cwin;
            nr_state->alg_state = picoquic_newreno_alg_congestion_avoidance;
        }
    }
}
```

### Rust body
```rust
    fn seed_cwin(&mut self, seed_cwin: u64) {
        if self.alg_state == NewRenoAlgState::SlowStart
            && self.ssthresh == u64::MAX
            && seed_cwin > self.cwin
        {
            self.cwin = seed_cwin;
            self.ssthresh = seed_cwin;
            self.alg_state = NewRenoAlgState::CongestionAvoidance;
        }
    }
```

## Pair `picoquic/newreno.c:picoquic_newreno_notify`
C: `picoquic/newreno.c:207-296 picoquic_newreno_notify`
Rust: `rs/fq/src/newreno.rs:74-85 picoquic_newreno_notify`

### C body
```c
{
    picoquic_newreno_state_t* nr_state = (picoquic_newreno_state_t*)path_x->congestion_alg_state;

    path_x->is_cc_data_updated = 1;

    if (nr_state != NULL) {
        switch (notification) {
        /* RTT measurements will happen before acknowledgement is signalled */
        case picoquic_congestion_notification_acknowledgement:
            if (nr_state->nrss.alg_state == picoquic_newreno_alg_slow_start &&
                nr_state->nrss.ssthresh == UINT64_MAX) {
                /* Increase cwin based on bandwidth estimation. */
                path_x->cwin = picoquic_cc_update_target_cwin_estimation(path_x);
                nr_state->nrss.cwin = path_x->cwin;
            }

            if (path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                /* TODO app limited. */
                picoquic_newreno_sim_notify(&nr_state->nrss, cnx, path_x, notification, ack_state, current_time);
                path_x->cwin = nr_state->nrss.cwin;
            }
            break;
        case picoquic_congestion_notification_seed_cwin:
            picoquic_newreno_sim_notify(&nr_state->nrss, cnx, path_x, notification, ack_state, current_time);
            path_x->cwin = nr_state->nrss.cwin;
            break;
        case picoquic_congestion_notification_ecn_ec:
        case picoquic_congestion_notification_repeat:
        case picoquic_congestion_notification_timeout:
            /* TODO fix test cases first. */
            /* packet_trace qlog_trace qlog_trace_auto qlog_trace_only qlog_trace_ecn l4s_reno pacing_update
             * quality_update app_limited_reno multipath_callback multipath_quality  */
            /* if (picoquic_cc_hystart_loss_test(&nr_state->rtt_filter, notification, ack_state->lost_packet_number,
                PICOQUIC_SMOOTHED_LOSS_THRESHOLD)) { */
                picoquic_newreno_sim_notify(&nr_state->nrss, cnx, path_x, notification, ack_state, current_time);
                path_x->cwin = nr_state->nrss.cwin;
            /* } */
            break;
        case picoquic_congestion_notification_spurious_repeat:
            picoquic_newreno_sim_notify(&nr_state->nrss, cnx, path_x, notification, ack_state, current_time);
            path_x->cwin = nr_state->nrss.cwin;
            path_x->is_ssthresh_initialized = 1;
            break;
        case picoquic_congestion_notification_rtt_measurement:
            if (nr_state->nrss.alg_state == picoquic_newreno_alg_slow_start &&
                nr_state->nrss.ssthresh == UINT64_MAX){

                /* if in slow start, increase the window for long delay RTT */
                if (path_x->rtt_min > PICOQUIC_TARGET_RENO_RTT) {
                    path_x->cwin = picoquic_cc_update_cwin_for_long_rtt(path_x);
                    nr_state->nrss.cwin = path_x->cwin;
                }

                /* HyStart. */
                /* Using RTT increases as signal to get out of initial slow start */
                if (picoquic_cc_hystart_test(&nr_state->rtt_filter, (cnx->is_time_stamp_enabled) ? ack_state->one_way_delay : ack_state->rtt_measurement,
                    cnx->path[0]->pacing.packet_time_microsec, current_time, cnx->is_time_stamp_enabled)) {
                    /* RTT increased too much, get out of slow start! */
                    nr_state->nrss.ssthresh = nr_state->nrss.cwin;
                    nr_state->nrss.alg_state = picoquic_newreno_alg_congestion_avoidance;
                    path_x->cwin = nr_state->nrss.cwin;
                    path_x->is_ssthresh_initialized = 1;
                }
            }
            break;
        case picoquic_congestion_notification_reset:
            picoquic_newreno_reset(nr_state, path_x);
            break;
        default:
            /* ignore */
            break;
        }

        /* Compute pacing data */
        picoquic_update_pacing_data(path_x, nr_state->nrss.alg_state == picoquic_newreno_alg_slow_start &&
            nr_state->nrss.ssthresh == UINT64_MAX);
    }
}
```

### Rust body
```rust
    let Some(boxed_state) = path_x.congestion_alg_state.take() else {
        return;
    };
```

## Pair `picoquic/pacing.c:picoquic_update_pacing_bucket`
C: `picoquic/pacing.c:37-52 picoquic_update_pacing_bucket`
Rust: `rs/fq/src/internal.rs:5908-5921 update_bucket`

### C body
```c
{
    if (pacing->bucket_nanosec < -pacing->packet_time_nanosec) {
        pacing->bucket_nanosec = -pacing->packet_time_nanosec;
    }

    if (current_time > pacing->evaluation_time) {
        pacing->bucket_nanosec += (current_time - pacing->evaluation_time) * 1000;
        pacing->evaluation_time = current_time;
        if (pacing->bucket_nanosec > pacing->bucket_max) {
            pacing->bucket_nanosec = pacing->bucket_max;
        }
    }
}
```

### Rust body
```rust
    fn update_bucket(&mut self, current_time: Instant) {
        if self.bucket_nanosec < -self.packet_time_nanosec {
            self.bucket_nanosec = -self.packet_time_nanosec;
        }
        let cur = current_time.ticks();
        let ev = self.evaluation_time.ticks();
        if cur > ev {
            self.bucket_nanosec += ((cur - ev) * 1000) as i64;
            self.evaluation_time = current_time;
            if self.bucket_nanosec > self.bucket_max {
                self.bucket_nanosec = self.bucket_max;
            }
        }
    }
```

## Pair `picoquic/pacing.c:picoquic_update_pacing_parameters`
C: `picoquic/pacing.c:137-180 picoquic_update_pacing_parameters`
Rust: `rs/fq/src/internal.rs:5964-6008 update_parameters`

### C body
```c
{
    double packet_time = (double)send_mtu / pacing_rate;
    double quantum_time = (double)quantum / pacing_rate;
    uint64_t rtt_nanosec = smoothed_rtt * 1000;

    pacing->rate = (uint64_t)pacing_rate;

    if (quantum > pacing->quantum_max) {
        pacing->quantum_max = quantum;
    }
    if (pacing->rate > pacing->rate_max) {
        pacing->rate_max = pacing->rate;
    }

    pacing->packet_time_nanosec = (uint64_t)(packet_time * 1000000000.0);

    if (pacing->packet_time_nanosec <= 0) {
        pacing->packet_time_nanosec = 1;
        pacing->packet_time_microsec = 1;
    }
    else {
        if ((uint64_t)pacing->packet_time_nanosec > rtt_nanosec) {
            pacing->packet_time_nanosec = rtt_nanosec;
        }
        pacing->packet_time_microsec = (pacing->packet_time_nanosec + 999ull) / 1000;
    }

    pacing->bucket_max = (uint64_t)(quantum_time * 1000000000.0);
    if (pacing->bucket_max <= 0) {
        pacing->bucket_max = 16 * pacing->packet_time_nanosec;
    }

    if (pacing->bucket_nanosec > pacing->bucket_max) {
        pacing->bucket_nanosec = pacing->bucket_max;
    }

    if (signalled_path != NULL) {
        picoquic_report_pacing_update(pacing, signalled_path);
    }
}
```

### Rust body
```rust
    ) {
        // C: picoquic_update_pacing_parameters
        let rtt_nanosec = smoothed_rtt.ticks().saturating_mul(1000);
        self.rate = if pacing_rate.is_sign_positive() {
            pacing_rate as u64
        } else {
            0
        };
        if quantum > self.quantum_max {
            self.quantum_max = quantum;
        }
        if self.rate > self.rate_max {
            self.rate_max = self.rate;
        }

        if pacing_rate > 0.0 && send_mtu > 0 {
            let mut packet_time_nanosec = (send_mtu as f64 / pacing_rate * 1_000_000_000.0) as i64;
            if packet_time_nanosec <= 0 {
                packet_time_nanosec = 1;
                self.packet_time_microsec = crate::Duration::from_ticks(1);
            } else {
                if packet_time_nanosec as u64 > rtt_nanosec {
                    packet_time_nanosec = rtt_nanosec as i64;
                }
                self.packet_time_microsec =
                    crate::Duration::from_ticks(((packet_time_nanosec + 999) / 1000) as u64);
            }
            self.packet_time_nanosec = packet_time_nanosec;

            self.bucket_max = (quantum as f64 / pacing_rate * 1_000_000_000.0) as i64;
            if self.bucket_max <= 0 {
                self.bucket_max = 16 * self.packet_time_nanosec;
            }
            if self.bucket_nanosec > self.bucket_max {
                self.bucket_nanosec = self.bucket_max;
            }
        }
    }
```

## Pair `picoquic/pacing.c:picoquic_is_sending_authorized_by_pacing`
C: `picoquic/pacing.c:253-257 picoquic_is_sending_authorized_by_pacing`
Rust: `rs/fq/src/internal.rs:6232-6241 is_sending_authorized_by_pacing`

### C body
```c
{
    return picoquic_is_authorized_by_pacing(&path_x->pacing, current_time, next_time, cnx->quic->packet_train_mode,
        cnx->quic);
}
```

### Rust body
```rust
        if let Some(path) = self.paths.get_mut(path_idx) {
            path.pacing
                .is_authorized(current_time, next_time, false, None)
        } else {
```

## Pair `picoquic/packet.c:picoquic_screen_initial_packet`
C: `picoquic/packet.c:85-207 picoquic_screen_initial_packet`
Rust: `rs/fq/src/lib.rs:5680-5821 screen_initial_packet`

### C body
```c
{
    int ret = 0;
    void* aead_ctx = NULL;
    void* pn_dec_ctx = NULL;

    /* Create a connection context if the CI is acceptable */
    if (packet_length < PICOQUIC_ENFORCED_INITIAL_MTU) {
        /* Unexpected packet. Reject, drop and log. */
        ret = PICOQUIC_ERROR_INITIAL_TOO_SHORT;
    }
    else if (ph->dest_cnx_id.id_len < PICOQUIC_ENFORCED_INITIAL_CID_LENGTH) {
        /* Initial CID too short -- ignore the packet */
        ret = PICOQUIC_ERROR_INITIAL_CID_TOO_SHORT;
    }
    else if (ph->has_reserved_bit_set) {
        /* Cannot have reserved bit set before negotiation completes */
        ret = PICOQUIC_ERROR_PACKET_HEADER_PARSING;
    }
    else if (quic->enforce_client_only) {
        /* Cannot create a client connection if the context is client only */
        ret = PICOQUIC_ERROR_SERVER_BUSY;
    }
    else if (quic->server_busy ||
        quic->current_number_connections >= quic->tentative_max_number_connections) {
        /* Cannot create a client connection now, send immediate close. */
        ret = PICOQUIC_ERROR_SERVER_BUSY;
    }
    else {
        /* This code assumes that *pcnx is always null when screen initial is called. */
        /* Verify the AEAD checkum */

        if (picoquic_get_initial_aead_context(quic, ph->version_index, &ph->dest_cnx_id,
            0 /* is_client=0 */, 0 /* is_enc = 0 */, &aead_ctx, &pn_dec_ctx) == 0) {
            ret = picoquic_remove_header_protection_inner((uint8_t *)bytes, ph->offset + ph->payload_length,
                decrypted_data->data, ph, pn_dec_ctx, 0 /* is_loss_bit_enabled_incoming */, 0 /* sack_list_last*/);
            if (ret == 0) {
                size_t decrypted_length = picoquic_aead_decrypt_generic(decrypted_data->data + ph->offset,
                    bytes + ph->offset, ph->payload_length, ph->pn64, decrypted_data->data, ph->offset, 
                    aead_ctx);
                if (decrypted_length >= ph->payload_length) {
                    ret = PICOQUIC_ERROR_AEAD_CHECK;
                }
                else {
                    ph->payload_length = (uint16_t)decrypted_length;
                }
            }
        }
        else {
            ret = PICOQUIC_ERROR_MEMORY;
        }

        if (ret == 0) {
            int is_address_blocked = !quic->is_port_blocking_disabled && picoquic_check_addr_blocked(addr_from);
            int is_new_token = 0;
            int has_good_token = 0;
            int has_bad_token = 0;
            picoquic_connection_id_t original_cnxid = { 0 };
            if (ph->token_length > 0) {
                /* If a token is present, verify it. */
                if (picoquic_verify_retry_token(quic, addr_from, current_time,
                    &is_new_token, &original_cnxid, &ph->dest_cnx_id, (uint32_t)ph->pn64,
                    ph->token_bytes, ph->token_length, 1) == 0) {
                    has_good_token = 1;
                }
                else {
                    has_bad_token = 1;
                }
            }

            if (has_bad_token && !is_new_token) {
                /* sending a bad retry token is fatal, sending an old new token is not */
                ret = PICOQUIC_ERROR_INVALID_TOKEN;
            }
            else if (!has_good_token && (quic->force_check_token || quic->max_half_open_before_retry <= quic->current_number_half_open || is_address_blocked)) {
                /* tokens are required before accepting new connections, so ask to queue a retry packet. */
                ret = PICOQUIC_ERROR_RETRY_NEEDED;
            }
            else {
                /* All clear */
                /* Check: what do do with odcid? */
                *pcnx = picoquic_create_cnx_internal(quic, ph->dest_cnx_id, ph->srce_cnx_id, addr_from, current_time, ph->vn,
                    NULL, NULL, 0, aead_ctx, pn_dec_ctx);
                if (*pcnx == NULL) {
                    /* Could not allocate the context */
                    ret = PICOQUIC_ERROR_MEMORY;
                }
                else {
                    *new_ctx_created = 1;
                    if (has_good_token) {
                        (*pcnx)->initial_validated = 1;
                        (void)picoquic_parse_connection_id(original_cnxid.id, original_cnxid.id_len, &(*pcnx)->original_cnxid);
                    }
                    /* Zeroing the pointers aead_ctx and pn_dec_ctx because the underlying object is
                     * now owned by the connection. */
                    aead_ctx = NULL;
                    pn_dec_ctx = NULL;
                }
            }
        }
    }

    if (aead_ctx != NULL) {
        /* Free the AEAD CTX */
        picoquic_aead_free(aead_ctx);
    }

    if (pn_dec_ctx != NULL) {
        /* Free the PN encryption context */
        picoquic_cipher_free(pn_dec_ctx);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> ParsedSegment {
        if packet_length < crate::internal::ENFORCED_INITIAL_MTU {
            return ParsedSegment {
                ret: InternalError::InitialTooShort as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if ph.dest_connection_id.len() < crate::internal::ENFORCED_INITIAL_CID_LENGTH as usize {
            return ParsedSegment {
                ret: InternalError::InitialCidTooShort as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if self.enforce_client_only
            || self.server_busy
            || self.current_number_connections >= self.tentative_max_number_connections
        {
            return ParsedSegment {
                ret: InternalError::ServerBusy as i32,
                connection: None,
                new_context_created: false,
            };
        }

        let initial_context =
            match self.initial_aead_context(ph.version_index, &ph.dest_connection_id, false, false)
            {
                Ok(ctx) => ctx,
                Err(error) => {
                    return ParsedSegment {
                        ret: Self::parse_error_status(error),
                        connection: None,
                        new_context_created: false,
                    };
                }
            };
        let decrypt_ret = decrypt_packet_payload(
            raw_bytes,
            packet_length,
            ph,
            decrypted_data,
            initial_context.pn_enc_ctx.as_ref(),
            initial_context.aead_ctx.as_ref(),
            false,
            0,
        );
        if decrypt_ret != 0 {
            return ParsedSegment {
                ret: decrypt_ret,
                connection: None,
                new_context_created: false,
            };
        }

        let is_address_blocked = !self.is_port_blocking_disabled && check_addr_blocked(addr_from);
        let mut has_good_token = false;
        let mut has_bad_token = false;
        let mut verified_token = None;
        if !ph.token_bytes.is_empty() {
            let token_bytes = ph.token_bytes.clone();
            match self.verify_retry_token(
                addr_from,
                current_time,
                &ph.dest_connection_id,
                ph.packet_number_full as u32,
                &token_bytes,
                true,
            ) {
                Ok(token) => {
                    has_good_token = true;
                    verified_token = Some(token);
                }
                Err(_) => has_bad_token = true,
            }
        }

        if has_bad_token {
            return ParsedSegment {
                ret: InternalError::InvalidToken as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if !has_good_token
            && (self.force_check_token
                || self.max_half_open_before_retry <= self.current_number_half_open
                || is_address_blocked)
        {
            return ParsedSegment {
                ret: InternalError::RetryNeeded as i32,
                connection: None,
                new_context_created: false,
            };
        }

        let connection = match self.create_cnx_internal(
            ph.dest_connection_id,
            ph.src_connection_id,
            Some(addr_from),
            current_time,
            ph.version,
            None,
            None,
            false,
            None,
            None,
        ) {
            Ok(token) => token,
            Err(error) => {
                return ParsedSegment {
                    ret: Self::parse_error_status(error),
                    connection: None,
                    new_context_created: false,
                };
            }
        };

        if let Some(cnx) = self.connections.get_mut(connection) {
            cnx.version_index = ph.version_index;
            cnx.proposed_version = ph.version;
            if let Some(token) = verified_token {
                cnx.initial_validated = true;
                cnx.original_connection_id = token.odcid;
            }
        }

        ParsedSegment {
            ret: 0,
            connection: Some(connection),
            new_context_created: true,
        }
    }
```

## Pair `picoquic/packet.c:picoquic_get_packet_number64`
C: `picoquic/packet.c:500-524 picoquic_get_packet_number64`
Rust: `rs/fq/src/internal.rs:7062-7077 get_packet_number64`

### C body
```c
{
    uint64_t expected = highest + 1;
    uint64_t not_mask_plus_one = (~mask) + 1;
    uint64_t pn64 = (expected & mask) | pn;

    if (pn64 < expected) {
        uint64_t delta1 = expected - pn64;
        uint64_t delta2 = not_mask_plus_one - delta1;
        if (delta2 < delta1) {
            pn64 += not_mask_plus_one;
        }
    } else {
        uint64_t delta1 = pn64 - expected;
        uint64_t delta2 = not_mask_plus_one - delta1;

        if (delta2 <= delta1 && (pn64 & mask) > 0) {
            /* Out of sequence packet from previous roll */
            pn64 -= not_mask_plus_one;
        }
    }

    return pn64;
}
```

### Rust body
```rust
pub fn get_packet_number64(highest: u64, mask: u64, pn: u32) -> u64 {
    // C: picoquic_get_packet_number64 — reconstruct full 64-bit PN from truncated `pn`
    // using `highest` (last seen full PN) and `mask` (which bits are carried).
    // The truncated PN has (mask+1) possible values; pick the one closest to highest.
    let candidate_base = highest & !mask;
    let candidate = candidate_base | (pn as u64 & mask);
    // Adjust if candidate is too far from highest.
    let half_window = (mask + 1).div_ceil(2);
    if candidate + half_window < highest {
        candidate + mask + 1
    } else if candidate > highest + half_window && candidate > mask {
        candidate - (mask + 1)
    } else {
        candidate
    }
}
```

## Pair `picoquic/packet.c:picoquic_parse_header_and_decrypt`
C: `picoquic/packet.c:770-902 picoquic_parse_header_and_decrypt`
Rust: `rs/fq/src/internal.rs:7762-7829 parse_header_and_decrypt`

### C body
```c
{
    /* Parse the clear text header. Ret == 0 means an incorrect packet that could not be parsed */
    int already_received = 0;
    size_t decoded_length = 0;
    int ret = picoquic_parse_packet_header(quic, bytes, length, addr_from, ph, pcnx, 1);

    *new_ctx_created = 0;

    if (ret == 0) {
        if (ph->ptype != picoquic_packet_version_negotiation &&
            ph->ptype != picoquic_packet_retry && ph->ptype != picoquic_packet_error) {
            length = ph->offset + ph->payload_length;
            *consumed = length;

            if (*pcnx != NULL) {
                if (!(*pcnx)->client_mode && ph->ptype == picoquic_packet_initial && packet_length < PICOQUIC_ENFORCED_INITIAL_MTU) {
                    /* Unexpected packet. Reject, drop and log. */
                    ret = PICOQUIC_ERROR_INITIAL_TOO_SHORT;
                }
                /* Test whether we need to do a version upgrade */
                else if (ph->version_index != (*pcnx)->version_index) {
                    if ((*pcnx)->client_mode &&
                        (*pcnx)->cnx_state < picoquic_state_client_almost_ready &&
                        ph->version_index >= 0 &&
                        picoquic_supported_versions[ph->version_index].version == (*pcnx)->desired_version) {
                        /* The server already accepted the version upgrade */
                        ret = picoquic_process_version_upgrade(*pcnx, (*pcnx)->version_index, ph->version_index);
                    }
                    else {
                        ret = PICOQUIC_ERROR_PACKET_WRONG_VERSION;
                    }
                }
                else {
                    /* Remove header protection at this point -- values of bytes will not change */
                    ret = picoquic_remove_header_protection(*pcnx, (uint8_t*)bytes, decrypted_data->data, ph);

                    if (ret == 0) {
                        decoded_length = picoquic_remove_packet_protection(*pcnx, (uint8_t*)bytes,
                            decrypted_data->data, ph, current_time, &already_received);
                    }
                    else {
                        decoded_length = ph->payload_length + 1;
                    }

                    if (decoded_length > (length - ph->offset)) {
                        if (ph->ptype == picoquic_packet_1rtt_protected &&
                            length >= PICOQUIC_RESET_PACKET_MIN_SIZE &&
                            memcmp(bytes + length - PICOQUIC_RESET_SECRET_SIZE,
                                (*pcnx)->path[0]->first_tuple->p_remote_cnxid->reset_secret, PICOQUIC_RESET_SECRET_SIZE) == 0) {
                            ret = PICOQUIC_ERROR_STATELESS_RESET;
                            picoquic_log_app_message(*pcnx, "Decrypt error, matching reset secret, ret = %d", ret);
                        }
                        else {
                            if (ret != PICOQUIC_ERROR_AEAD_NOT_READY) {
                                ret = PICOQUIC_ERROR_AEAD_CHECK;
                            }
                            if (*new_ctx_created) {
                                picoquic_delete_cnx(*pcnx);
                                *pcnx = NULL;
                                *new_ctx_created = 0;
                            }
                        }
                    }
                    else if (already_received != 0) {
                        ret = PICOQUIC_ERROR_DUPLICATE;
                    }
                    else {
                        ph->payload_length = (uint16_t)decoded_length;
                    }
                }
            }
            else {
                if (ph->ptype != picoquic_packet_version_negotiation &&
                    ph->ptype != picoquic_packet_retry && ph->ptype != picoquic_packet_error) {
                    /* Redirect if proxy available -- function returns 0 if the packet was *not* intercepted */
                    if (quic->picomask_fns != NULL) {
                        ret = (quic->picomask_fns->picomask_redirect_fn)(quic->picomask_ctx,
                            bytes, packet_length, addr_from, consumed);
                    }
                    if (ret == 0) {
                        /* If packet was not redirected, it might be an initial packet
                         * for a new connection or a stateless redirect. Any other type
                         * should be treated as an error.
                         */
                        if (ph->ptype == picoquic_packet_initial) {
                            /* Screening the packet for protection against DOS. If successful, this
                             * will decrypt the initial packet and create a new connection context.
                             */
                            ret = picoquic_screen_initial_packet(quic, bytes, packet_length, addr_from, ph, current_time, pcnx,
                                new_ctx_created, decrypted_data);
                        }
                        else if (ph->ptype == picoquic_packet_1rtt_protected)
                        {
                            /* This may be a stateless reset.
                             * We test the address + putative reset secret pair against the hash table
                             * of registered secrets. If there is a match, the corresponding connection is
                             * found and the packet is marked as Stateless Reset */
                            if (length >= PICOQUIC_RESET_PACKET_MIN_SIZE) {
                                *pcnx = picoquic_cnx_by_secret(quic, bytes + length - PICOQUIC_RESET_SECRET_SIZE, addr_from);
                                if (*pcnx != NULL) {
                                    ret = PICOQUIC_ERROR_STATELESS_RESET;
                                    picoquic_log_app_message(*pcnx, "Found connection from reset secret, ret = %d", ret);
                                }
                            }
                        }
                        else {
                            ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
                        }
                    }
                }
            }
        }
        else {
            /* Clear text packet. Copy content to decrypted data */
            memmove(decrypted_data->data, bytes, length);
            *consumed = length;
        }
    }
    
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(Option<ConnectionToken>, bool), crate::Error> {
        let packet_length = packet_length.min(bytes.len());
        let conn_tok = self.parse_packet_header(&bytes[..packet_length], addr_from, ph, true)?;
        let Some(conn_tok) = conn_tok else {
            *consumed = ph.offset.min(packet_length);
            return Ok((None, false));
        };
        let mut packet = bytes[..packet_length].to_vec();
        let cnx = self
            .connections
            .get(conn_tok)
            .ok_or(crate::Error::Generic)?;
        let epoch = ph.epoch as usize;
        let pn_dec = cnx.crypto_context[epoch]
            .pn_dec
            .as_deref()
            .ok_or(crate::Error::Tls)?;
        let aead = cnx.crypto_context[epoch]
            .aead_decrypt
            .as_deref()
            .ok_or(crate::Error::Tls)?;
        let mut header_copy = [0u8; MAX_PACKET_SIZE];
        let is_loss_bit = cnx.is_loss_bit_enabled_incoming;
        let largest = cnx.ack_ctx[ph.packet_context as usize].sack_list.first();
        if remove_header_protection_inner(
            &mut packet,
            packet_length,
            &mut header_copy,
            ph,
            pn_dec,
            is_loss_bit,
            largest,
        ) != 0
        {
            return Err(crate::Error::Tls);
        }
        let pn_length = ((packet[0] & 0x03) + 1) as usize;
        let header_end = ph.packet_number_offset.saturating_add(pn_length);
        let cipher_end = if ph.payload_length > 0 {
            ph.packet_number_offset
                .saturating_add(ph.payload_length)
                .min(packet_length)
        } else {
            packet_length
        };
        if header_end > cipher_end || cipher_end > packet.len() {
            return Err(crate::Error::InvalidArgument);
        }
        let header = packet[..header_end].to_vec();
        let mut payload = packet[header_end..cipher_end].to_vec();
        aead.decrypt(ph.packet_number_full, &header, &mut payload)?;
        let len = payload.len().min(decrypted_data.data.len());
        decrypted_data.stream_data_membership = None;
        decrypted_data.offset = 0;
        decrypted_data.length = len;
        decrypted_data.data[..len].copy_from_slice(&payload[..len]);
        *consumed = cipher_end;
        Ok((Some(conn_tok), false))
    }
```

## Pair `picoquic/packet.c:picoquic_queue_stateless_retry`
C: `picoquic/packet.c:1144-1208 picoquic_queue_stateless_retry`
Rust: `rs/fq/src/lib.rs:5976-5987 queue_stateless_retry`

### C body
```c
{
    picoquic_stateless_packet_t* sp = picoquic_create_stateless_packet(quic);
    void * integrity_aead = picoquic_find_retry_protection_context(quic, ph->version_index, 1);
    size_t checksum_length = (integrity_aead == NULL) ? 0 : picoquic_aead_get_checksum_length(integrity_aead);

    if (sp != NULL) {
        uint8_t* bytes = sp->bytes;
        size_t byte_index = 0;
        size_t header_length = 0;
        size_t pn_offset;
        size_t pn_length;

        byte_index = header_length = picoquic_create_long_header(
            picoquic_packet_retry,
            &ph->srce_cnx_id,
            s_cid,
            0 /* No grease bit here */,
            ph->vn,
            ph->version_index,
            0, /* Sequence number is not used */
            retry_token_length,
            retry_token,
            bytes,
            &pn_offset,
            &pn_length);

        /* Add the token to the payload. */
        if (byte_index + retry_token_length < PICOQUIC_MAX_PACKET_SIZE) {
            memcpy(bytes + byte_index, retry_token, retry_token_length);
            byte_index += retry_token_length;
        }

        /* In the old drafts, there is no header protection and the sender copies the ODCID
         * in the packet. In the recent draft, the ODCID is not sent but
         * is verified as part of integrity checksum */
        if (integrity_aead == NULL) {
            bytes[byte_index++] = ph->dest_cnx_id.id_len;
            byte_index += picoquic_format_connection_id(bytes + byte_index,
                PICOQUIC_MAX_PACKET_SIZE - byte_index - checksum_length, ph->dest_cnx_id);
        }
        else {
            /* Encode the retry integrity protection if required. */
            byte_index = picoquic_encode_retry_protection(integrity_aead, bytes, PICOQUIC_MAX_PACKET_SIZE, byte_index, &ph->dest_cnx_id);
        }

        sp->length = byte_index;

        sp->ptype = picoquic_packet_retry;

        picoquic_store_addr(&sp->addr_to, addr_from);
        picoquic_store_addr(&sp->addr_local, addr_to);
        sp->if_index_local = if_index_to;
        sp->cnxid_log64 = picoquic_val64_connection_id(ph->dest_cnx_id);

        picoquic_queue_stateless_packet(quic, sp);
    }
}
```

### Rust body
```rust
        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };
```
