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

## `picoquic/intformat.c:picoquic_decode_varint_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns 1 shifted by the top two bits of the byte; the Rust body shown parses long packet type and version instead.
* C source: `picoquic/intformat.c:85-88`
* C signature: `size_t picoquic_decode_varint_length(uint8_t)`
* Rust source: `rs/fq/src/internal.rs:6481-6526`
* Rust item: `decode_varint_length`

### C body
```c
{
    return ((size_t)1u) << ((byte & 0xC0) >> 6);
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

## `picoquic/logger.c:textlog_dropped_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C logs padding drops specially or logs a decrypted segment; the Rust body shown only checks is_still_logging and returns early.
* C source: `picoquic/logger.c:2261-2284`
* C signature: `void textlog_dropped_packet(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_header *, size_t, int, uint64_t)`
* Rust source: `rs/fq/src/logger.rs:566-576`
* Rust item: `dropped_packet`

### C body
```c
{
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        FILE* F = cnx->quic->F_log;

        if (ret == PICOQUIC_ERROR_PADDING_PACKET) {
            uint64_t log_cnxid64 = 0;

            if (cnx == NULL) {
                log_cnxid64 = picoquic_val64_connection_id(ph->srce_cnx_id);
            }
            else {
                log_cnxid64 = picoquic_val64_connection_id(picoquic_get_logging_cnxid(cnx));
            }
            textlog_prefix_initial_cid64(F, log_cnxid64);
            fprintf(F, "Dropped padding packet, size: %zu.\n", packet_size);
            fprintf(F, "\n");
        }
        else {
            textlog_decrypted_segment(cnx->quic->F_log, 1, cnx, 1, ph, NULL, packet_size, ret);
        }
    }
}
```

### Rust body
```rust
        if !self.is_still_logging() {
            return;
        }
```

## `picoquic/logwriter.c:binlog_packet_ex`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body checks connection/binlog/logging and calls binlog_packet; the Rust body shown only returns early when not logging and contains no packet logging call.
* C source: `picoquic/logwriter.c:790-797`
* C signature: `void binlog_packet_ex(picoquic_cnx_t *, picoquic_path_t *, int, uint64_t, picoquic_packet_header *, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/logger.rs:514-524`
* Rust item: `packet`

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

## `picoquic/loss_recovery.c:picoquic_queue_retransmit_on_ack`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C selects a packet context and loops through pending packets to process lost retransmissions; Rust only calls retransmit_demoted_path.
* C source: `picoquic/loss_recovery.c:1029-1074`
* C signature: `void picoquic_queue_retransmit_on_ack(picoquic_cnx_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:4751-4753`
* Rust item: `queue_retransmit_on_ack`

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

## `picoquic/packet.c:picoquic_incoming_server_handshake`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is incoming_packet_ex-style segment iteration and returns a connection option; C body validates source CID, decodes server handshake frames, may process TLS stream, and returns an integer status.
* C source: `picoquic/packet.c:1704-1752`
* C signature: `int picoquic_incoming_server_handshake(picoquic_cnx_t *, uint8_t *, picoquic_stream_data_node_t *, struct sockaddr *, unsigned long, picoquic_packet_header *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:3289-3338`
* Rust item: `incoming_packet_ex`

### C body
```c
{
    int ret = 0;
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(addr_to);
    UNREFERENCED_PARAMETER(if_index_to);
#endif
    int restricted = cnx->cnx_state != picoquic_state_client_handshake_start;
    
    if (picoquic_compare_connection_id(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id, &ph->srce_cnx_id) != 0) {
        ret = PICOQUIC_ERROR_CNXID_CHECK; /* protocol error */
    }


    if (ret == 0) {
        if (cnx->cnx_state < picoquic_state_ready) {
            /* Accept the incoming frames */

            if (ph->payload_length == 0) {
                /* empty payload! */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
            else {
                ret = picoquic_decode_frames(cnx, cnx->path[0],
                    bytes + ph->offset, ph->payload_length,received_data,
                    ph->epoch, NULL, addr_to, ph->pn64, 0, current_time);
            }

            /* processing of initial packet */
            if (ret == 0 && restricted == 0) {
                ret = picoquic_tls_stream_process(cnx, NULL, current_time);
            }
        }
        else {
            /* Initial keys should have been discarded, treat packet as unexpected */
            ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
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
