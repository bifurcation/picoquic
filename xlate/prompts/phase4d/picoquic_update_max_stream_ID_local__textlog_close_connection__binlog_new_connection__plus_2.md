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

## `picoquic/frames.c:picoquic_update_max_stream_ID_local`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C only updates after receive completion and, for bidirectional streams, send completion; Rust lacks those completion checks and updates based on stream id thresholds.
* C source: `picoquic/frames.c:4702-4722`
* C signature: `void picoquic_update_max_stream_ID_local(picoquic_cnx_t *, picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/internal.rs:10019-10049`
* Rust item: `update_max_stream_id_local`

### C body
```c
{
    if (cnx->client_mode != IS_CLIENT_STREAM_ID(stream->stream_id) && !stream->max_stream_updated) {
        /* This is a remotely initiated stream */
        if (stream->consumed_offset >= stream->fin_offset && (stream->fin_received || stream->reset_received)) {
            /* Receive is complete */
            if (IS_BIDIR_STREAM_ID(stream->stream_id)) {
                if (stream->fin_sent || stream->reset_sent)
                {
                    /* Sending is complete */
                    stream->max_stream_updated = 1;
                    cnx->max_stream_id_bidir_local_computed += 4;
                }
            } else {
                /* No need to check receive complete on uni directional streams */
                stream->max_stream_updated = 1;
                cnx->max_stream_id_unidir_local_computed += 4;
            }
        }
    }
}
```

### Rust body
```rust
    pub fn update_max_stream_id_local(&mut self, stream: &mut StreamHead) {
        use crate::stream::{Role, StreamId};
        let sid = StreamId(stream.stream_id);
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };
        if sid.is_local(local_role) {
            return;
        }
        if sid.is_bidir() {
            if stream.stream_id >= self.max_stream_id_bidir_local {
                let old = self.max_stream_id_bidir_local;
                self.max_stream_id_bidir_local_computed = self
                    .max_stream_id_bidir_local_computed
                    .max(stream.stream_id.saturating_add(4));
                if self.max_stream_id_bidir_local_computed > old {
                    stream.max_stream_updated = true;
                }
            }
        } else if stream.stream_id >= self.max_stream_id_unidir_local {
            let old = self.max_stream_id_unidir_local;
            self.max_stream_id_unidir_local_computed = self
                .max_stream_id_unidir_local_computed
                .max(stream.stream_id.saturating_add(4));
            if self.max_stream_id_unidir_local_computed > old {
                stream.max_stream_updated = true;
            }
        }
    }
```

## `picoquic/logger.c:textlog_close_connection`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body is effectively a no-op, while Rust calls close_connection on a text logger when present.
* C source: `picoquic/logger.c:2370-2375`
* C signature: `void textlog_close_connection(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/logger.rs:828-831`
* Rust item: `close_connection`

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

## `picoquic/logwriter.c:binlog_new_connection`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C closes/creates the binlog file, builds and stores a filename, enforces log limits, increments open-log count, then writes the event; Rust only returns when f_binlog is None and emits the event to an already-attached file.
* C source: `picoquic/logwriter.c:1013-1089`
* C signature: `void binlog_new_connection(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/binlog.rs:1033-1530`
* Rust item: `new_connection`

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

## `picoquic/loss_recovery.c:picoquic_check_path_mtu_on_losses`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C conditionally increments MTU loss counters, resets path MTU, and logs; Rust only unwraps send_path or returns.
* C source: `picoquic/loss_recovery.c:873-893`
* C signature: `void picoquic_check_path_mtu_on_losses(picoquic_cnx_t *, picoquic_packet_t *, int)`
* Rust source: `rs/fq/src/internal.rs:11067-11074`
* Rust item: `check_path_mtu_on_losses`

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

## `picoquic/packet.c:picoquic_incoming_segment`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is incoming_packet_ex-style segment iteration and max-MTU update, not the C segment-processing body with header decrypt, path lookup, packet-type dispatch, logging, error handling, PN recording, and cleanup.
* C source: `picoquic/packet.c:2073-2387`
* C signature: `int picoquic_incoming_segment(picoquic_quic_t *, uint8_t *, size_t, size_t, size_t *, struct sockaddr *, struct sockaddr *, int, unsigned char, uint64_t, uint64_t, picoquic_connection_id_t *, picoquic_cnx_t **)`
* Rust source: `rs/fq/src/lib.rs:3289-3338`
* Rust item: `incoming_packet_ex`

### C body
```c
{
    int ret = 0;
    picoquic_cnx_t* cnx = NULL;
    picoquic_packet_header ph;
    int new_context_created = 0;
    int is_first_segment = 0;
    int is_buffered = 0;
    int path_id = -1;
    int path_is_not_allocated = 0;
    uint8_t* bytes = NULL;
    picoquic_stream_data_node_t* decrypted_data = picoquic_stream_data_node_alloc(quic);

    if (decrypted_data == NULL) {
        return -1;
    }
    /* Parse the header and decrypt the segment */
    ret = picoquic_parse_header_and_decrypt(quic, raw_bytes, length, packet_length, addr_from,
        current_time, decrypted_data, &ph, &cnx, consumed, &new_context_created);
    bytes = decrypted_data->data;

    if (ret == 0 && cnx != NULL) {
        if (ph.ptype == picoquic_packet_1rtt_protected) {
            /* Find the arrival path and update its state */
            ret = picoquic_find_incoming_path(cnx, &ph, addr_from, addr_to, if_index_to, current_time, &path_id);
        }
        else {
            path_id = 0;
        }
    }

    /* Verify that the segment coalescing is for the same destination ID */
    if (picoquic_is_connection_id_null(previous_dest_id)) {
        /* This is the first segment in the incoming packet */
        *previous_dest_id = ph.dest_cnx_id;
        is_first_segment = 1;
        *first_cnx = cnx;


        /* if needed, log that the packet is received */
        if (cnx != NULL) {
            picoquic_log_pdu(cnx, 1, current_time, addr_from, addr_to, packet_length,
                (path_id >= 0) ? cnx->path[path_id]->unique_path_id : 0, received_ecn);
        }
        else {
            picoquic_log_quic_pdu(quic, 1, current_time, picoquic_val64_connection_id(ph.dest_cnx_id),
                addr_from, addr_to, packet_length);
        }
    }
    else {
        if (ret == 0 && picoquic_compare_connection_id(previous_dest_id, &ph.dest_cnx_id) != 0) {
            ret = PICOQUIC_ERROR_CNXID_SEGMENT;
        }
        else if (ret == PICOQUIC_ERROR_VERSION_NOT_SUPPORTED) {
            /* A coalesced packet with unknown version is likely some kind of padding */
            ret = PICOQUIC_ERROR_CNXID_SEGMENT;
        }

        if (ret == PICOQUIC_ERROR_CNXID_SEGMENT && *first_cnx != cnx && *first_cnx != NULL) {
            /* Log the drop segment information in the context of the first connection */
            picoquic_log_dropped_packet(*first_cnx, NULL, &ph, length, PICOQUIC_ERROR_PADDING_PACKET, bytes, current_time);
        }
    }

    /* Store packet if received in advance of encryption keys */
    if (ret == PICOQUIC_ERROR_AEAD_NOT_READY &&
        cnx != NULL) {
        is_buffered = picoquic_incoming_not_decrypted(cnx, &ph, current_time, raw_bytes, length, addr_from, addr_to, if_index_to, received_ecn);
    }

    /* Find the path and if required log the incoming packet */
    if (cnx != NULL) {
        if (ret == 0 && ph.ptype == picoquic_packet_1rtt_protected) {
            if (ph.payload_length == 0) {
                /* empty payload! */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
            else if (ph.has_reserved_bit_set) {
                /* Reserved bits were not set to zero */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
        }

        if (ret == 0) {
            picoquic_log_packet(cnx, (path_id < 0)?NULL:cnx->path[path_id], 1, current_time, &ph, bytes, *consumed);
        }
        else if (is_buffered) {
            picoquic_log_buffered_packet(cnx, (path_id < 0) ? NULL : cnx->path[path_id], ph.ptype, current_time);
        } else {
            picoquic_log_dropped_packet(cnx, (path_id < 0) ? NULL : cnx->path[path_id], &ph, length, ret, bytes, current_time);
        }
    }

    if (ret == PICOQUIC_ERROR_VERSION_NOT_SUPPORTED) {
        /* use the result of parsing to consider version negotiation,
        * but block reflection attacks towards protected ports. */
        if (packet_length >= PICOQUIC_ENFORCED_INITIAL_MTU){
            if (quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from)) {
                picoquic_prepare_version_negotiation(quic, addr_from, addr_to, if_index_to, &ph, raw_bytes);
            }
        }
    } else if (ret == PICOQUIC_ERROR_RETRY_NEEDED) {
        /* Incoming packet could not be processed, need to send a Retry. */
        if (packet_length >= PICOQUIC_ENFORCED_INITIAL_MTU){
            if (quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from)) {
                picoquic_queue_retry_packet(quic, addr_from, addr_to, if_index_to, &ph, current_time);
            }
        }
    } else if (ret == PICOQUIC_ERROR_SERVER_BUSY) {
        /* Incoming packet could not be processed, need to send a Retry. */
        if (packet_length >= PICOQUIC_ENFORCED_INITIAL_MTU){
            if (quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from)) {
                picoquic_queue_busy_packet(quic, addr_from, addr_to, if_index_to, &ph);
            }
        }
    } else if (ret == 0) {
        if (cnx == NULL) {
            /* Unexpected packet. Reject, drop and log. */
            if (!picoquic_is_connection_id_null(&ph.dest_cnx_id) &&
                (quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from))) {
                picoquic_process_unexpected_cnxid(quic, length, addr_from, addr_to, if_index_to, &ph, current_time);
            }
            ret = PICOQUIC_ERROR_DETECTED;
        }
        else {
            cnx->quic_bit_received_0 |= ph.quic_bit_is_zero;
            switch (ph.ptype) {
            case picoquic_packet_version_negotiation:
                ret = picoquic_incoming_version_negotiation(
                    cnx, bytes, length, addr_from, &ph, current_time);
                break;
            case picoquic_packet_initial:
                /* Initial packet: either crypto handshakes or acks. */
                if (ph.has_reserved_bit_set) {
                    ret = PICOQUIC_ERROR_PACKET_HEADER_PARSING;
                } else if ((!cnx->client_mode && picoquic_compare_connection_id(&ph.dest_cnx_id, &cnx->initial_cnxid) == 0) ||
                    picoquic_compare_connection_id(&ph.dest_cnx_id, &cnx->path[0]->first_tuple->p_local_cnxid->cnx_id) == 0) {
                    /* Verify that the source CID matches expectation */
                    if (picoquic_is_connection_id_null(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id)) {
                        cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id = ph.srce_cnx_id;
                    } else if (picoquic_compare_connection_id(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id, &ph.srce_cnx_id) != 0) {
                        DBG_PRINTF("Error wrong srce cnxid (%d), type: %d, epoch: %d, pc: %d, pn: %d\n",
                            cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int)ph.pn);
                        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
                    }
                    if (ret == 0) {
                        if (packet_length < PICOQUIC_ENFORCED_INITIAL_MTU) {
                            if (!cnx->did_receive_short_initial) {
                                picoquic_log_app_message(cnx, "Received unpadded initial, length=%zu", packet_length);
                            }
                            cnx->did_receive_short_initial = 1;
                        }
                        if (cnx->client_mode == 0) {
                            if (is_first_segment) {
                                /* Account for the data received in handshake, but only
                                 * count the packet once. Do not count it again if it is not
                                 * the first segment in packet */
                                cnx->initial_data_received += packet_length;
                            }
                            ret = picoquic_incoming_client_initial(&cnx, bytes, packet_length, decrypted_data,
                                addr_from, addr_to, if_index_to, &ph, current_time, new_context_created);
                            /* Reset the value of first_cnx, as the context may have been deleted */
                            *first_cnx = cnx;
                        }
                        else {
                            /* TODO: this really depends on the current receive epoch */
                            ret = picoquic_incoming_server_initial(cnx, bytes, packet_length,
                                decrypted_data, addr_to, if_index_to, &ph, current_time);
                        }
                    }
                } else {
                    DBG_PRINTF("Error detected (%d), type: %d, epoch: %d, pc: %d, pn: %d\n",
                        cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int)ph.pn);
                    ret = PICOQUIC_ERROR_DETECTED;
                }
                break;
            case picoquic_packet_retry:
                ret = picoquic_incoming_retry(cnx, raw_bytes, &ph, current_time);
                break;
            case picoquic_packet_handshake:
                if (ph.has_reserved_bit_set) {
                    ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
                }
                else if (ph.has_reserved_bit_set) {
                    ret = PICOQUIC_ERROR_PACKET_HEADER_PARSING;
                }
                else if (cnx->client_mode)
                {
                    ret = picoquic_incoming_server_handshake(cnx, bytes, decrypted_data, addr_to, if_index_to, &ph, current_time);
                }
                else
                {
                    ret = picoquic_incoming_client_handshake(cnx, bytes, decrypted_data, &ph, current_time);
                }
                break;
            case picoquic_packet_0rtt_protected:
                if (ph.has_reserved_bit_set) {
                    ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
                }
                else {
                    if (is_first_segment) {
                        /* Account for the data received in handshake, but only
                         * count the packet once. Do not count it again if it is not
                         * the first segment in packet */
                        cnx->initial_data_received += packet_length;
                    }
                    ret = picoquic_incoming_0rtt(cnx, bytes, decrypted_data, &ph, current_time);
                }
                break;
            case picoquic_packet_1rtt_protected:
                ret = picoquic_incoming_1rtt(cnx, path_id, bytes, decrypted_data,
                    &ph, addr_from, addr_to, if_index_to,
                    path_is_not_allocated, current_time);
                break;
            default:
                /* Packet type error. Log and ignore */
                DBG_PRINTF("Unexpected packet type (%d), type: %d, epoch: %d, pc: %d, pn: %d\n",
                    cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int) ph.pn);
                ret = PICOQUIC_ERROR_DETECTED;
                break;
            }
        }
    } else if (ret == PICOQUIC_ERROR_STATELESS_RESET) {
        ret = picoquic_incoming_stateless_reset(cnx);
    }
    else if (ret == PICOQUIC_ERROR_AEAD_CHECK &&
        ph.ptype == picoquic_packet_handshake &&
        cnx != NULL &&
        (cnx->cnx_state == picoquic_state_client_init_sent || cnx->cnx_state == picoquic_state_client_init_resent))
    {
        /* Indicates that the server probably sent initial and handshake but initial was lost */
        if (cnx->pkt_ctx[picoquic_packet_context_initial].pending_first != NULL &&
            cnx->path[0]->nb_retransmit == 0) {
            /* Reset the retransmit timer to start retransmission immediately */
            cnx->path[0]->retransmit_timer = current_time -
                cnx->pkt_ctx[picoquic_packet_context_initial].pending_first->send_time;
        }
    }

    if (ret == 0) {
        if (cnx != NULL && cnx->cnx_state != picoquic_state_disconnected &&
            ph.ptype != picoquic_packet_version_negotiation) {
            cnx->nb_packets_received++;
            cnx->latest_receive_time = current_time;
            /* Mark the sequence number as received */
            ret = picoquic_record_pn_received(cnx, ph.pc, ph.l_cid, ph.pn64, receive_time);
            /* Perform ECN accounting */
            picoquic_ecn_accounting(cnx, received_ecn, ph.pc, ph.l_cid);
        }
        if (cnx != NULL) {
            picoquic_reinsert_by_wake_time(cnx->quic, cnx, current_time);
        }
    } else if (ret == PICOQUIC_ERROR_AEAD_CHECK || ret == PICOQUIC_ERROR_INITIAL_TOO_SHORT ||
        ret == PICOQUIC_ERROR_PACKET_WRONG_VERSION ||
        ret == PICOQUIC_ERROR_INITIAL_CID_TOO_SHORT ||
        ret == PICOQUIC_ERROR_PORT_BLOCKED ||
        ret == PICOQUIC_ERROR_UNEXPECTED_PACKET || 
        ret == PICOQUIC_ERROR_CNXID_CHECK || 
        ret == PICOQUIC_ERROR_RETRY || ret == PICOQUIC_ERROR_DETECTED ||
        ret == PICOQUIC_ERROR_SERVER_BUSY ||
        ret == PICOQUIC_ERROR_CONNECTION_DELETED ||
        ret == PICOQUIC_ERROR_CNXID_SEGMENT ||
        ret == PICOQUIC_ERROR_VERSION_NOT_SUPPORTED ||
        ret == PICOQUIC_ERROR_PACKET_TOO_LONG ||
        ret == PICOQUIC_ERROR_DUPLICATE ||
        ret == PICOQUIC_ERROR_AEAD_NOT_READY ||
        ret == PICOQUIC_ERROR_REDIRECTED) {
        /* Bad packets are dropped silently */
        if (ret == PICOQUIC_ERROR_AEAD_CHECK ||
            ret == PICOQUIC_ERROR_PACKET_WRONG_VERSION ||
            ret == PICOQUIC_ERROR_AEAD_NOT_READY ||
            ret == PICOQUIC_ERROR_PACKET_TOO_LONG ||
            ret == PICOQUIC_ERROR_VERSION_NOT_SUPPORTED ||
            ret == PICOQUIC_ERROR_RETRY ||
            ret == PICOQUIC_ERROR_SERVER_BUSY ||
            ret == PICOQUIC_ERROR_REDIRECTED) {
            ret = 0;
        }
        else {
            ret = -1;
        }
        if (cnx != NULL) {
            picoquic_reinsert_by_wake_time(cnx->quic, cnx, current_time);
        }
    } else if (ret == 1) {
        /* wonder what happened ! */
        DBG_PRINTF("Packet (%d) get ret=1, t: %d, e: %d, pc: %d, pn: %d, l: %zu\n",
            (cnx == NULL) ? -1 : cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int)ph.pn, length);
        ret = -1;
    }
    else if (ret != 0) {
        DBG_PRINTF("Packet (%d) error, t: %d, e: %d, pc: %d, pn: %d, l: %zu, ret : 0x%x\n",
            (cnx == NULL) ? -1 : cnx->client_mode, ph.ptype, ph.epoch, ph.pc, (int)ph.pn, length, ret);
        ret = -1;
    }

    if (decrypted_data != NULL && decrypted_data->bytes == NULL) {
        picoquic_stream_data_node_recycle(decrypted_data);
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
