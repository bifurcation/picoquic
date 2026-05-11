# Phase 4E repair confirmation

This is a read-only re-triage after a Phase 4E repair or
repair-level `ok` claim.  Do not edit files.

For each entry, inspect directly relevant C and Rust context
and decide whether the current Rust translation is now
acceptable.

Report:

* `ok` when the current Rust behavior is acceptable.
* `needs_fix` when a real mismatch remains.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short confirmation conclusion","fix_summary":"remaining mismatch if any, or empty","files_changed":[],"verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/logwriter.c:binlog_new_connection`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C closes/creates the binlog file, builds and stores a filename, enforces log limits, increments open-log count, then writes the event; Rust only returns when f_binlog is None and emits the event to an already-attached file.
* Prior Phase 4D analysis: C opens and names a binlog file, enforces the open-log cap, updates bookkeeping, then writes the event. Rust only writes the event to an already attached file and binlog enablement does not install the backend hook.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Rust now matches the C behavior: binlog new_connection opens and names the per-connection file, enforces the open-log cap, updates bookkeeping, and writes the start event through installed binlog dispatch.
* Phase 4E fix summary: Implemented binlog file creation/bookkeeping in Connection::new_connection and wired Quic::enable_binlog to install the binlog Logger backend.
* C source: `picoquic/logwriter.c:1013-1089`
* C signature: `void binlog_new_connection(picoquic_cnx_t *)`
* Current Rust source: `rs/fq/src/binlog.rs:1202-1763`
* Current Rust item: `new_connection`

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

### Current Rust body
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
        // send_buffer, …, &pcnx, 0)` with `pcnx` pre-set to this
        // connection.  Mirror the outgoing short-header CID-length
        // rule locally before emitting the binlog record.
        let outgoing_short_dcid_len = initial_remote_connection_id(self).len();
        let mut ph = parse_outgoing_header(
            send_buffer,
            outgoing_short_dcid_len,
            self.version_index,
            self.local_parameters.do_grease_quic_bit,
            self.is_loss_bit_enabled_outgoing,
        );

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
        let quic_ptr = self.quic_ptr;
        if quic_ptr.is_null() {
            return;
        }

        let (bin_dir, use_unique_log_names, creation_time) = unsafe {
            // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal`
            // and remains valid while the connection is live.  This block only
            // snapshots context-level logging configuration and checks the
            // open-log cap before the connection-owned file is replaced.
            let quic = &mut *quic_ptr;
            if quic.bin_log_fns.is_none()
                || quic.current_number_of_open_logs >= quic.max_simultaneous_logs
            {
                return;
            }
            let Some(bin_dir) = quic.binlog_dir.as_ref().or(quic.qlog_dir.as_ref()).cloned() else {
                return;
            };
            (bin_dir, quic.use_unique_log_names, quic.time())
        };

        self.f_binlog = None;

        let cid_name = connection_id_hexa(&self.initial_connection_id);
        let role = if self.client_mode { "client" } else { "server" };
        let file_name = if use_unique_log_names {
            format!("{}.{:x}.{}.log", cid_name, self.log_unique, role)
        } else {
            format!("{}.{}.log", cid_name, role)
        };
        let log_filename = bin_dir.join(file_name);
        if log_filename.to_string_lossy().len() >= 512 {
            return;
        }
        self.binlog_file_name = Some(log_filename.clone());

        let Some(f_binlog) = create_binlog(
            &log_filename,
            creation_time,
            self.local_parameters.initial_max_path_id > 0,
        ) else {
            self.binlog_file_name = None;
            return;
        };
        self.f_binlog = Some(f_binlog);

        unsafe {
            // SAFETY: same ownership invariant as above; this mirrors the C
            // context-level open-log accounting increment after successful
            // binlog creation.
            (*quic_ptr).current_number_of_open_logs =
                (*quic_ptr).current_number_of_open_logs.saturating_add(1);
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
        let remote_cid = initial_remote_connection_id(self);

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
        if let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) {
            compose_event_header(&mut msg, &cid, now, 0, LogEventType::ConnectionClose);
            let payload = msg.as_bytes().to_vec();
            drop(msg);

            if let Some(f) = self.f_binlog.as_mut() {
                write_record(f, &payload);
            }
        }

        if let Some(f) = self.f_binlog.as_mut() {
            let _ = f.flush();
        }

        // Close the file before auto-qlog conversion, matching C's
        // `cnx->f_binlog = picoquic_file_close(...)`.
        self.f_binlog = None;
        let quic_ptr = self.quic_ptr;

        if !quic_ptr.is_null() {
            // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal`
            // and remains valid while the connection is live.  Borrow only
            // the disjoint context fields needed for the close hook; the
            // callback receives a shared connection borrow, as required by
            // `AutoQlog::run`.
            unsafe {
                let qlog_dir = &(*quic_ptr).qlog_dir;
                let autoqlog_fn = &mut (*quic_ptr).autoqlog_fn;
                if qlog_dir.is_some()
                    && let Some(autoqlog) = autoqlog_fn.as_mut()
                {
                    let _ = autoqlog.run(self);
                }
            }
        }

        self.binlog_file_name = None;

        if !quic_ptr.is_null() {
            // SAFETY: same ownership invariant as above; this mirrors the C
            // context-level log-accounting decrement after the file was closed.
            unsafe {
                let open_logs = &mut (*quic_ptr).current_number_of_open_logs;
                if *open_logs > 0 {
                    *open_logs -= 1;
                }
            }
        }
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
