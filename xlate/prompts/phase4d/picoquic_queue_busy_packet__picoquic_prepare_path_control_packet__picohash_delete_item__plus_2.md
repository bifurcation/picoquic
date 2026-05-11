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

## `picoquic/packet.c:picoquic_queue_busy_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C builds, encrypts, protects, fills, and queues a stateless busy packet; Rust only creates a stateless packet and has no visible equivalent work.
* C source: `picoquic/packet.c:1240-1315`
* C signature: `int picoquic_queue_busy_packet(picoquic_quic_t *, const struct sockaddr *, const struct sockaddr *, int, picoquic_packet_header *)`
* Rust source: `rs/fq/src/lib.rs:6069-6078`
* Rust item: `queue_busy_packet`

### C body
```c
{
    int ret = 0;
    picoquic_connection_id_t s_cid = { 0 };
    picoquic_stateless_packet_t* sp = picoquic_create_stateless_packet(quic);
    void* aead_ctx = NULL;
    void* pn_enc_ctx = NULL;

    if (sp != NULL) {
        uint8_t* bytes = sp->bytes;
        size_t byte_index = 0;
        size_t header_length = 0;
        size_t pn_offset;
        size_t pn_length;
        /* Payload is the encoding of the simples connection close frame */
        uint8_t payload[4] = { picoquic_frame_type_connection_close, PICOQUIC_TRANSPORT_SERVER_BUSY, 0, 0 };
        size_t payload_length = 0;

        picoquic_create_local_cnx_id(quic, &s_cid, ph->dest_cnx_id);


        /* Prepare long header:  Initial */
        byte_index = header_length = picoquic_create_long_header(
            picoquic_packet_initial,
            &ph->srce_cnx_id,
            &s_cid,
            0 /* No grease bit here */,
            ph->vn,
            ph->version_index,
            0, /* Sequence number 0 by default. */
            0,
            NULL,
            bytes,
            &pn_offset,
            &pn_length);

        /* Apply AEAD */
        if (picoquic_get_initial_aead_context(quic, ph->version_index, &ph->dest_cnx_id,
            0 /* is_client=0 */, 1 /* is_enc = 1 */, &aead_ctx, &pn_enc_ctx) == 0) {
            /* Make sure that the payload length is encoded in the header */
            /* Using encryption, the "payload" length also includes the encrypted packet length */
            picoquic_update_payload_length(bytes, pn_offset, header_length - pn_length,
                header_length + sizeof(payload) + picoquic_aead_get_checksum_length(aead_ctx));
            /* Encrypt packet payload */
            payload_length = picoquic_aead_encrypt_generic(bytes + header_length,
                payload, sizeof(payload), 0, bytes, header_length, aead_ctx);
            /* protect the PN */
            picoquic_protect_packet_header(bytes, pn_offset, 0x0F, pn_enc_ctx);
            /* Fill up control fields */
            sp->length = byte_index + payload_length;
            sp->ptype = picoquic_packet_initial;
            picoquic_store_addr(&sp->addr_to, addr_from);
            picoquic_store_addr(&sp->addr_local, addr_to);
            sp->if_index_local = if_index_to;
            sp->cnxid_log64 = picoquic_val64_connection_id(ph->dest_cnx_id);
            /* Queue packet */
            picoquic_queue_stateless_packet(quic, sp);
        }

        if (aead_ctx != NULL) {
            /* Free the AEAD CTX */
            picoquic_aead_free(aead_ctx);
        }

        if (pn_enc_ctx != NULL) {
            /* Free the PN encryption context */
            picoquic_cipher_free(pn_enc_ctx);
        }
    }
    return ret;
}
```

### Rust body
```rust
        let Ok(mut sp) = self.create_stateless_packet() else {
            return InternalError::Memory as i32;
        };
```

## `picoquic/paths.c:picoquic_prepare_path_control_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C predicts/builds header, prepares challenge/observed-address frames, pads by policy, finalizes/protects packet, updates wake/logging; Rust only writes challenge frames, simple padding, and metadata.
* C source: `picoquic/paths.c:150-232`
* C signature: `int picoquic_prepare_path_control_packet(picoquic_cnx_t *, picoquic_path_t *, picoquic_tuple_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:4424-4459`
* Rust item: `prepare_path_control_packet`

### C body
```c
{
    int ret = 0;
    picoquic_packet_type_enum packet_type = picoquic_packet_1rtt_protected;
    int is_pure_ack = 1;
    size_t header_length = 0;
    size_t length = 0;
    size_t checksum_overhead = picoquic_get_checksum_length(cnx, picoquic_epoch_1rtt);
    size_t send_buffer_min_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;
    uint8_t* bytes = packet->bytes;
    uint8_t* bytes_max = bytes + send_buffer_min_max - checksum_overhead;
    uint8_t* bytes_next;
    int more_data = 0;
    int is_challenge_padding_needed = 0;
    picoquic_packet_context_t* pkt_ctx = (cnx->is_multipath_enabled) ?
        &path_x->pkt_ctx :
        &cnx->pkt_ctx[picoquic_packet_context_application];

    /* TODO: will use the local CID specified for the path. There should be a distinct
    * CID specified for the tuple.
     */
    packet->pc = picoquic_packet_context_application;

    length = picoquic_predict_packet_header_length(
        cnx, packet_type, pkt_ctx);
    packet->ptype = packet_type;
    packet->offset = length;
    header_length = length;
    packet->sequence_number = pkt_ctx->send_sequence;
    packet->send_time = current_time;
    packet->send_path = path_x;
    bytes_next = bytes + length;

    /* If required, prepare challenge and response frames.
     * These frames will be sent immediately, regardless of pacing or flow control.
     */
    bytes_next = picoquic_prepare_tuple_challenge_frames(cnx, path_x, tuple,
        bytes_next, bytes_max, &more_data, &is_pure_ack, &is_challenge_padding_needed,
        current_time, next_wake_time);

    /* Compute the length before pacing block */
    length = bytes_next - bytes;

    if (cnx->is_address_discovery_provider) {
        /* If a new address was learned, prepare an observed address frame */
        /* TODO: tie this code to processing of paths */
        bytes_next = picoquic_prepare_observed_address_frame(bytes_next, bytes_max,
            path_x, tuple, current_time, next_wake_time, &more_data, &is_pure_ack);
    }

    if (ret == 0 && length > header_length) {
        /* Ensure that all packets are properly padded before being sent. */

        if (is_challenge_padding_needed && length < PICOQUIC_ENFORCED_INITIAL_MTU) {
            length = picoquic_pad_to_target_length(bytes, length, (uint32_t)(send_buffer_min_max - checksum_overhead));
        }
        else {
            length = picoquic_pad_to_policy(cnx, bytes, length, (uint32_t)(send_buffer_min_max - checksum_overhead));
        }
    }
    else {
        length = 0;
    }
    packet->length = length;
    picoquic_finalize_and_protect_packet_tuple(cnx, packet,
        ret, length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_min_max,
        path_x, current_time, tuple);

    if (*send_length > 0) {
        *next_wake_time = current_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);

        if (ret == 0 && picoquic_cnx_is_still_logging(cnx)) {
            picoquic_log_cc_dump(cnx, current_time);
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<usize, crate::Error> {
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let mut padding_needed = 0;
        let total = send_buffer.len();
        let tail = self
            .prepare_path_challenge_frames(
                path_x,
                send_buffer,
                &mut more_data,
                &mut is_pure_ack,
                &mut padding_needed,
                current_time,
                next_wake_time,
            )
            .ok_or(crate::Error::BufferTooSmall)?;
        let mut written = total - tail.len();
        if padding_needed != 0 && written < MIN_SEGMENT_SIZE && written < total {
            let target = MIN_SEGMENT_SIZE.min(total);
            send_buffer[written..target].fill(0);
            written = target;
        }
        packet.length = written;
        packet.packet_context = PacketContext::Application;
        packet.packet_type = PacketType::OneRttProtected;
        packet.is_ack_eliciting = is_pure_ack == 0;
        Ok(written)
    }
```

## `picoquic/picohash.c:picohash_delete_item`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C deletes an item from a generic hash-bin chain and optionally frees memory; Rust updates sack/ack-tree bookkeeping and removes a token from sack_items.
* C source: `picoquic/picohash.c:111-141`
* C signature: `void picohash_delete_item(picohash_table *, picohash_item *, int)`
* Rust source: `rs/fq/src/internal.rs:8148-8166`
* Rust item: `delete_item`

### C body
```c
{
    uint32_t bin = (uint32_t)(item->hash % hash_table->nb_bin);
    picohash_item* previous = hash_table->hash_bin[bin];
    const void* shall_delete = NULL;

    if (previous == item) {
        hash_table->hash_bin[bin] = item->next_in_bin;
        hash_table->count--;
    } else {
        while (previous != NULL) {
            if (previous->next_in_bin == item) {
                previous->next_in_bin = item->next_in_bin;
                hash_table->count--;
                break;
            } else {
                previous = previous->next_in_bin;
            }
        }
    }

    shall_delete = item->key;

    if (hash_table->picohash_key_to_item == NULL) {
        free(item);
    }

    if (delete_key_too) {
        free((void*)shall_delete);
    }
}
```

### Rust body
```rust
    fn delete_item(&mut self, token: SackItemToken) -> Result<(), crate::Error> {
        let (membership, sent_counts) = self
            .sack_items
            .get(token)
            .map(|item| (item.ack_tree_membership, item.nb_times_sent))
            .ok_or(crate::Error::Generic)?;
        for (r, sent) in sent_counts.iter().enumerate() {
            if *sent >= 0 && (*sent as usize) < MAX_ACK_RANGE_REPEAT {
                self.rc[r].range_counts[*sent as usize] -= 1;
            }
        }
        if let Some(membership) = membership {
            self.ack_tree.remove(membership);
        }
        self.sack_items
            .remove(token)
            .map(|_| ())
            .ok_or(crate::Error::Generic)
    }
```

## `picoquic/picosocks.c:picoquic_select`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C forwards to picoquic_select_ex; Rust body shown defines an unrelated ConnectionIdCallback trait method.
* C source: `picoquic/picosocks.c:1248-1260`
* C signature: `int picoquic_select(int *, int, struct sockaddr_storage *, struct sockaddr_storage *, int *, unsigned char *, uint8_t *, int, int64_t, uint64_t *)`
* Rust source: `rs/fq/src/lib.rs:540-553`
* Rust item: `select`

### C body
```c
    uint64_t* current_time) {
    int socket_rank;
    return picoquic_select_ex(sockets, nb_sockets, addr_from, addr_dest, dest_if,
        received_ecn, buffer, buffer_max, delta_t, &socket_rank, current_time);
}
```

### Rust body
```rust
pub trait ConnectionIdCallback {
    fn produce(
        &mut self,
        quic: &mut Quic,
        connection_id_local: ConnectionId,
        connection_id_remote: ConnectionId,
    ) -> ConnectionId;
}
```

## `picoquic/picosplay.c:picosplay_first`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the leftmost node, but Rust function shown is last() and returns rightmost.
* C source: `picoquic/picosplay.c:233-235`
* C signature: `picosplay_node_t * picosplay_first(picosplay_tree_t *)`
* Rust source: `rs/fq/src/splay.rs:508-516`
* Rust item: `first`

### C body
```c
picosplay_node_t* picosplay_first(picosplay_tree_t *tree) {
    return leftmost(tree->root);
}
```

### Rust body
```rust
    pub fn last(&self) -> Option<SplayToken> {
        self.rightmost(self.root).map(|i| self.token_of(i))
    }
```
