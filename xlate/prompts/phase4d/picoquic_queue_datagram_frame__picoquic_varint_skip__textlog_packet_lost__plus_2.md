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

## `picoquic/frames.c:picoquic_queue_datagram_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C validates length limits, formats a datagram frame into a packet buffer, then queues the encoded frame; Rust queues the input bytes directly with no length checks or datagram frame formatting.
* C source: `picoquic/frames.c:5307-5335`
* C signature: `int picoquic_queue_datagram_frame(picoquic_cnx_t *, size_t, const uint8_t *)`
* Rust source: `rs/fq/src/lib.rs:3127-3135`
* Rust item: `queue_datagram_frame`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (length > PICOQUIC_DATAGRAM_QUEUE_CAUTIOUS_LENGTH) {
        if (length > cnx->local_parameters.max_datagram_frame_size ||
            length > cnx->remote_parameters.max_datagram_frame_size ||
            length + 21 + cnx->quic->local_cnxid_length > cnx->path[0]->send_mtu) {
            ret = PICOQUIC_ERROR_DATAGRAM_TOO_LONG;
        }
    }
    if (ret == 0) {
        size_t consumed = 0;
        uint8_t frame_buffer[PICOQUIC_MAX_PACKET_SIZE];
        int more_data = 0;
        int is_pure_ack = 1;
        uint8_t* bytes_next = picoquic_format_datagram_frame(frame_buffer, frame_buffer + sizeof(frame_buffer), &more_data, &is_pure_ack, length, src);

        if ((consumed = bytes_next - frame_buffer) > 0) {
            ret = picoquic_queue_misc_or_dg_frame(cnx, &cnx->first_datagram, &cnx->last_datagram,
                frame_buffer, consumed, 0, picoquic_packet_context_application);
        }
        else {
            ret = PICOQUIC_ERROR_FRAME_BUFFER_TOO_SMALL;
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn queue_datagram_frame(&mut self, bytes: &[u8]) -> Result<(), Error> {
        use crate::internal::MiscFrameHeader;
        self.datagrams.push_back(MiscFrameHeader {
            bytes: bytes.to_vec(),
            packet_context: PacketContext::Application,
            is_pure_ack: 0,
        });
        Ok(())
    }
```

## `picoquic/intformat.c:picoquic_varint_skip`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the decoded varint length from the first byte; the Rust body shown only returns 0 when the slice is empty and lacks the non-empty computation.
* C source: `picoquic/intformat.c:164-167`
* C signature: `size_t picoquic_varint_skip(const uint8_t *)`
* Rust source: `rs/fq/src/internal.rs:6452-6455`
* Rust item: `varint_skip`

### C body
```c
{
    return picoquic_decode_varint_length(bytes[0]);
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return 0;
    }
```

## `picoquic/logger.c:textlog_packet_lost`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C logs packet-lost details after logging checks, while Rust only checks is_still_logging and returns, with no visible packet-lost output.
* C source: `picoquic/logger.c:2308-2326`
* C signature: `void textlog_packet_lost(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_type_enum, uint64_t, const char *, picoquic_connection_id_t *, size_t, uint64_t)`
* Rust source: `rs/fq/src/logger.rs:697-709`
* Rust item: `packet_lost`

### C body
```c
{
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        FILE* F = cnx->quic->F_log;

        textlog_prefix_initial_cid64(F, picoquic_val64_connection_id(picoquic_get_logging_cnxid(cnx)));
        textlog_time(F, cnx, current_time, "T= ", ", ");
        fprintf(F, "Lost packet type %d, path %" PRIu64 ", number %" PRIu64 ", size %zu", ptype,
            path_x->unique_path_id, sequence_number, packet_size);
        if (dcid != NULL) {
            fprintf(F, ", DCID ");
            textlog_connection_id(F, dcid);
        }
        fprintf(F, ", reason: %s\n", trigger);
    }
}
```

### Rust body
```rust
        if !self.is_still_logging() {
            return;
        }
```

## `picoquic/logwriter.c:picoquic_binlog_frames`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C loops, decodes frame types, validates encodings, and dispatches many frame loggers; Rust only returns on empty input and otherwise has no visible implementation.
* C source: `picoquic/logwriter.c:550-677`
* C signature: `void picoquic_binlog_frames(FILE *, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/binlog.rs:687-691`
* Rust item: `binlog_frames`

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

## `picoquic/newreno.c:picoquic_newreno_delete`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C frees and clears congestion_alg_state; Rust body is alg_observe and only reads/downcasts state.
* C source: `picoquic/newreno.c:298-305`
* C signature: `void picoquic_newreno_delete(picoquic_path_t *)`
* Rust source: `rs/fq/src/newreno.rs:221-231`
* Rust item: `alg_delete`

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
