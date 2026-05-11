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

## `picoquic/frames.c:picoquic_queue_path_available_or_backup_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body returns an error if p_remote_cnxid is NULL and chooses path_id from either unique_path_id or the remote connection ID sequence depending on multipath; the Rust body has no NULL-equivalent check and always uses unique_path_id.
* C source: `picoquic/frames.c:5880-5910`
* C signature: `int picoquic_queue_path_available_or_backup_frame(picoquic_cnx_t *, picoquic_path_t *, picoquic_path_status_enum)`
* Rust source: `rs/fq/src/internal.rs:14893-14929`
* Rust item: `queue_path_available_or_backup_frame`

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

## `picoquic/logger.c:textlog_buffered_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C logs prefix, time, and buffered packet type when logging is active; Rust body shown only returns early if not still logging and has no visible logging output.
* C source: `picoquic/logger.c:2286-2296`
* C signature: `void textlog_buffered_packet(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_type_enum, uint64_t)`
* Rust source: `rs/fq/src/logger.rs:618-621`
* Rust item: `buffered_packet`

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

## `picoquic/logwriter.c:binlog_close`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body is an empty close function, while the Rust body shown is enable_binlog with different behavior and comments about enabling logging.
* C source: `picoquic/logwriter.c:1309-1315`
* C signature: `void binlog_close(picoquic_quic_t *)`
* Rust source: `rs/fq/src/binlog.rs:1576-1597`
* Rust item: `binlog_close`

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

## `picoquic/logwriter.c:picoquic_log_fixed_skip`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C fixed-skip advances by a supplied size, while the shown Rust body reads a varint length and returns that length with the remaining slice.
* C source: `picoquic/logwriter.c:37-40`
* C signature: `const uint8_t * picoquic_log_fixed_skip(const uint8_t *, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/binlog.rs:363-375`
* Rust item: `skip_fixed`

### C body
```c
{
    return bytes == NULL ? NULL : ((bytes += size) <= bytes_max ? bytes : NULL);
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

## `picoquic/packet.c:picoquic_incoming_not_decrypted`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C conditionally buffers undecrypted packets for later and returns a buffered flag; Rust is a generic incoming-packet loop and does not visibly buffer anything.
* C source: `picoquic/packet.c:2019-2067`
* C signature: `int picoquic_incoming_not_decrypted(picoquic_cnx_t *, picoquic_packet_header *, uint64_t, uint8_t *, size_t, struct sockaddr *, struct sockaddr *, int, unsigned char)`
* Rust source: `rs/fq/src/lib.rs:3289-3338`
* Rust item: `incoming_packet_ex`

### C body
```c
{
    int buffered = 0;

    if (cnx->cnx_state < picoquic_state_ready) {
        if (cnx->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len > 0 &&
            picoquic_compare_connection_id(&cnx->path[0]->first_tuple->p_local_cnxid->cnx_id, &ph->dest_cnx_id) == 0)
        {
            /* verifying the destination cnx id is a strong hint that the peer is responding.
            * Setting epoch parameter = -1 guarantees the hint is only used if the RTT is not
            * yet known.
            */
            picoquic_update_path_rtt(cnx, cnx->path[0], -1, cnx->start_time, current_time, 0, 0);

            if (length <= PICOQUIC_MAX_PACKET_SIZE &&
                ((ph->ptype == picoquic_packet_handshake && cnx->client_mode) || ph->ptype == picoquic_packet_1rtt_protected)) {
                /* stash a copy of the incoming message for processing once the keys are available */
                picoquic_stateless_packet_t* packet = picoquic_create_stateless_packet(cnx->quic);

                if (packet != NULL) {
                    packet->length = length;
                    packet->ptype = ph->ptype;
                    memcpy(packet->bytes, bytes, length);
                    packet->next_packet = cnx->first_sooner;
                    cnx->first_sooner = packet;
                    picoquic_store_addr(&packet->addr_local, addr_to);
                    picoquic_store_addr(&packet->addr_to, addr_from);
                    packet->if_index_local = if_index_to;
                    packet->received_ecn = received_ecn;
                    packet->receive_time = current_time;
                    buffered = 1;
                }
            }
        }
    }

    return buffered;
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
