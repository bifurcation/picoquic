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

## `picoquic/intformat.c:picoquic_varint_decode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only handles the empty-input case and lacks the visible C logic for computing length, checking max bytes, decoding bytes, assigning n64, and returning decoded length.
* C source: `picoquic/intformat.c:136-162`
* C signature: `size_t picoquic_varint_decode(const uint8_t *, size_t, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:6396-6400`
* Rust item: `varint_decode`

### C body
```c
{
    size_t length = 0;
    
    if (max_bytes < 1) {
        *n64 = 0;
    } else {
        length = ((size_t)1) << ((bytes[0] & 0xC0) >> 6);

        if (length > max_bytes) {
            *n64 = 0;
            length = 0;
        }
        else {
            uint64_t v = *bytes++ & 0x3F;

            for (size_t i = 1; i < length; i++) {
                v <<= 8;
                v += *bytes++;
            }

            *n64 = v;
        }
    } 

    return length;
}
```

### Rust body
```rust
    if bytes.is_empty() {
        *n64 = 0;
        return 0;
    }
```

## `picoquic/logger.c:textlog_new_connection`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body is effectively empty, while the Rust body visibly calls a text logger new_connection callback.
* C source: `picoquic/logger.c:2363-2368`
* C signature: `void textlog_new_connection(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/logger.rs:814-817`
* Rust item: `new_connection`

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
            text.borrow_mut().new_connection(self);
        }
```

## `picoquic/logwriter.c:binlog_pdu_ex`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C conditionally calls binlog_pdu with packet metadata; Rust only checks is_still_logging and returns on false, with no visible logging call.
* C source: `picoquic/logwriter.c:724-732`
* C signature: `void binlog_pdu_ex(picoquic_cnx_t *, int, uint64_t, const struct sockaddr *, const struct sockaddr *, size_t, uint64_t, unsigned char)`
* Rust source: `rs/fq/src/logger.rs:456-468`
* Rust item: `pdu`

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

## `picoquic/loss_recovery.c:picoquic_retransmit_demoted_path`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C checks ready state/path count, selects a packet context, and retransmits the path packet queue; Rust only marks one queued packet for retransmit.
* C source: `picoquic/loss_recovery.c:1010-1026`
* C signature: `void picoquic_retransmit_demoted_path(picoquic_cnx_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:4736-4746`
* Rust item: `retransmit_demoted_path`

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

## `picoquic/packet.c:picoquic_incoming_server_initial`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a generic incoming-packet loop over segments and lacks the C body's server-initial state transition, connection ID checks, payload validation, frame decoding, and TLS processing.
* C source: `picoquic/packet.c:1619-1701`
* C signature: `int picoquic_incoming_server_initial(picoquic_cnx_t *, uint8_t *, size_t, picoquic_stream_data_node_t *, struct sockaddr *, unsigned long, picoquic_packet_header *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:3289-3338`
* Rust item: `incoming_packet_ex`

### C body
```c
{
    int ret = 0;

    if (cnx->cnx_state == picoquic_state_client_init_sent || cnx->cnx_state == picoquic_state_client_init_resent) {
        cnx->cnx_state = picoquic_state_client_handshake_start;
    }

    /* Check the server cnx id */
    if ((!picoquic_is_connection_id_null(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id) || cnx->cnx_state > picoquic_state_client_handshake_start) &&
        picoquic_compare_connection_id(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id, &ph->srce_cnx_id) != 0) {
        ret = PICOQUIC_ERROR_CNXID_CHECK; /* protocol error */
    }

    if (ret == 0) {
        if (cnx->cnx_state <= picoquic_state_client_handshake_start) {
            /* Document local address if not present */
            if (cnx->path[0]->first_tuple->local_addr.ss_family == 0 && addr_to != NULL) {
                picoquic_store_addr(&cnx->path[0]->first_tuple->local_addr, addr_to);
            }
            cnx->path[0]->first_tuple->if_index = if_index_to;
            /* Accept the incoming frames */
            if (ph->payload_length == 0) {
                /* empty payload! */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
            else {
                /* Verify that the packet is long enough */
                if (packet_length < PICOQUIC_ENFORCED_INITIAL_MTU) {
                    size_t byte_index = ph->offset;
                    int ack_needed = 0;
                    int skip_ret = 0;

                    while (skip_ret == 0 && byte_index < ph->offset + ph->payload_length) {
                        size_t frame_length = 0;
                        int frame_is_pure_ack = 0;
                        skip_ret = picoquic_skip_frame(&bytes[byte_index],
                            ph->payload_length - byte_index, &frame_length, &frame_is_pure_ack);
                        byte_index += frame_length;
                        if (frame_is_pure_ack == 0) {
                            ack_needed = 1;
                            break;
                        }
                    }
                    if (ack_needed && cnx->retry_token_length == 0 && cnx->crypto_context[1].aead_encrypt == NULL) {
                        /* perform the test on new paths, but not if resuming an existing path or session */
                        picoquic_log_app_message(cnx, "Server initial too short (%zu bytes)", packet_length);
                        ret = PICOQUIC_ERROR_INITIAL_TOO_SHORT;
                    }
                }

                /* If no error, process the packet */
                if (ret == 0) {
                    ret = picoquic_decode_frames(cnx, cnx->path[0],
                        bytes + ph->offset, ph->payload_length, received_data,
                        ph->epoch, NULL, addr_to, ph->pn64, 0, current_time);
                }
            }
            /* processing of initial packet */
            if (ret == 0) {
                ret = picoquic_tls_stream_process(cnx, NULL, current_time);
            }
        }
        else if (cnx->cnx_state < picoquic_state_ready) {
            /* Require an acknowledgement if the packet contains ackable frames */
            picoquic_ignore_incoming_handshake(cnx, bytes, ph, current_time);
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
