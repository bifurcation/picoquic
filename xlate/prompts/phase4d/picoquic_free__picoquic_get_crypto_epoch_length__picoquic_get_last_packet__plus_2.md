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

## `picoquic/quicctx.c:picoquic_free`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C releases/deletes many owned resources and frees quic; Rust only calls self.init().
* C source: `picoquic/quicctx.c:1062-1173`
* C signature: `void picoquic_free(picoquic_quic_t *)`
* Rust source: `rs/fq/src/internal.rs:8647-8649`
* Rust item: `free`

### C body
```c
{
    if (quic != NULL) {
        PICOQUIC_THREAD_DISABLE_CHECK(quic);

        /* delete all the connection contexts -- do this before any other
         * action, as deleting connections may add packets to queues or
         * change connection lists */
        while (quic->cnx_list != NULL) {
            picoquic_delete_cnx(quic->cnx_list);
        }

        /* Delete ECH context if it was created */
        picoquic_release_quic_ech_ctx(quic);

        /* Delete TLS and AEAD cntexts */
        picoquic_delete_retry_protection_contexts(quic);

        if (quic->aead_encrypt_ticket_ctx != NULL) {
            picoquic_aead_free(quic->aead_encrypt_ticket_ctx);
            quic->aead_encrypt_ticket_ctx = NULL;
        }

        if (quic->aead_decrypt_ticket_ctx != NULL) {
            picoquic_aead_free(quic->aead_decrypt_ticket_ctx);
            quic->aead_decrypt_ticket_ctx = NULL;
        }

        if (quic->default_alpn != NULL) {
            free((void*)quic->default_alpn);
            quic->default_alpn = NULL;
        }

        /* delete the stored tickets */
        picoquic_free_tickets(&quic->p_first_ticket);

        /* Delete the stored tokens */
        picoquic_free_tokens(&quic->p_first_token);

        /* Deelete the reused tokens tree */
        picosplay_empty_tree(&quic->token_reuse_tree);

        /* delete packets in pool */
        while (quic->p_first_packet != NULL) {
            picoquic_packet_t * p = quic->p_first_packet->packet_previous;
            free(quic->p_first_packet);
            quic->p_first_packet = p;
            quic->nb_packets_allocated--;
            quic->nb_packets_in_pool--;
        }

        /* delete data nodes in pool */
        while (quic->p_first_data_node != NULL) {
            picoquic_stream_data_node_t* p = quic->p_first_data_node->next_stream_data;
            free(quic->p_first_data_node);
            quic->p_first_data_node = p;
            quic->nb_data_nodes_allocated--;
            quic->nb_data_nodes_in_pool--;
        }

        /* delete all pending stateless packets */
        while (quic->pending_stateless_packet != NULL) {
            picoquic_stateless_packet_t* to_delete = quic->pending_stateless_packet;
            quic->pending_stateless_packet = to_delete->next_packet;
            free(to_delete);
        }

        if (quic->table_cnx_by_id != NULL) {
            picohash_delete(quic->table_cnx_by_id, 0);
        }

        if (quic->table_cnx_by_net != NULL) {
            picohash_delete(quic->table_cnx_by_net, 0);
        }

        if (quic->table_cnx_by_icid != NULL) {
            picohash_delete(quic->table_cnx_by_icid, 0);
        }

        if (quic->table_issued_tickets != NULL) {
            picohash_delete(quic->table_issued_tickets, 1);
        }

        if (quic->table_cnx_by_secret != NULL) {
            picohash_delete(quic->table_cnx_by_secret, 0);
        }

        if (quic->verify_certificate_callback != NULL) {
            picoquic_dispose_verify_certificate_callback(quic);
        }

        /* Delete the picotls context */
        if (quic->tls_master_ctx != NULL) {
            picoquic_master_tlscontext_free(quic);

            free(quic->tls_master_ctx);
            quic->tls_master_ctx = NULL;
        }

        /* Close the logs */
        picoquic_log_close_logs(quic);

        quic->binlog_dir = picoquic_string_free(quic->binlog_dir);
        quic->qlog_dir = picoquic_string_free(quic->qlog_dir);

        if (quic->perflog_fn != NULL) {
            (void)(quic->perflog_fn)(quic, NULL, 1);
        }

        free(quic);
    }
}
```

### Rust body
```rust
    pub fn free(&mut self) {
        self.init();
    }
```

## `picoquic/quicctx.c:picoquic_get_crypto_epoch_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns crypto_epoch_length_max, while Rust sets pmtud_policy and returns nothing.
* C source: `picoquic/quicctx.c:1024-1028`
* C signature: `uint64_t picoquic_get_crypto_epoch_length(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2870-2877`
* Rust item: `crypto_epoch_length`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->crypto_epoch_length_max;
}
```

### Rust body
```rust
    pub fn set_pmtud_policy(&mut self, pmtud_policy: PmtudPolicy) {
        self.pmtud_policy = pmtud_policy;
    }
```

## `picoquic/quicctx.c:picoquic_get_last_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns a pending_last packet depending on multipath/context; Rust initializes ACK context fields.
* C source: `picoquic/quicctx.c:1700-1704`
* C signature: `picoquic_packet_t * picoquic_get_last_packet(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_context_enum)`
* Rust source: `rs/fq/src/internal.rs:7850-7875`
* Rust item: `get_last_packet`

### C body
```c
{
    return (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) ? path_x->pkt_ctx.pending_last :
        cnx->pkt_ctx[pc].pending_last;
}
```

### Rust body
```rust
impl Connection {
    pub fn init_ack_ctx(&mut self, ack_ctx: &mut AckContext) {
        // C: picoquic_init_ack_ctx
        ack_ctx.sack_list = SackList::new();
        ack_ctx.time_stamp_largest_received = crate::Instant::from_ticks(u64::MAX);
        ack_ctx.act[0].highest_ack_sent = 0;
        ack_ctx.act[0].highest_ack_sent_time = self.start_time;
        ack_ctx.act[0].ack_needed = false;
        ack_ctx.act[1].highest_ack_sent = 0;
        ack_ctx.act[1].highest_ack_sent_time = self.start_time;
        ack_ctx.act[1].ack_needed = false;
    }
}
```

## `picoquic/quicctx.c:picoquic_get_next_wake_time`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns current_time when a stateless packet is pending, otherwise first wake-tree time or UINT64_MAX; Rust ignores pending stateless packets and returns min over all connections or now.
* C source: `picoquic/quicctx.c:1533-1551`
* C signature: `uint64_t picoquic_get_next_wake_time(picoquic_quic_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:3210-3217`
* Rust item: `next_wake_time`

### C body
```c
{
    uint64_t wake_time = UINT64_MAX;
    PICOQUIC_THREAD_CHECK(quic);

    if (quic->pending_stateless_packet != NULL) {
        wake_time = current_time;
    }
    else{
        picoquic_cnx_t* cnx_wake_first = (picoquic_cnx_t*)picoquic_wake_list_node_value(
            picosplay_first(&quic->cnx_wake_tree));

        if (cnx_wake_first != NULL) {
            wake_time = cnx_wake_first->next_wake_time;
        }
    }

    return wake_time;
}
```

### Rust body
```rust
    pub fn next_wake_time(&self, current_time: Instant) -> u64 {
        let now = current_time.ticks();
        self.connections
            .iter()
            .map(|c| c.next_wake_time.ticks())
            .min()
            .unwrap_or(now)
    }
```

## `picoquic/quicctx.c:picoquic_get_server_cnxid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns one of two path connection IDs depending on client_mode; Rust returns initial_connection_id unconditionally and has a different visible function name.
* C source: `picoquic/quicctx.c:4483-4487`
* C signature: `picoquic_connection_id_t picoquic_get_server_cnxid(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3061-3072`
* Rust item: `server_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return (cnx->client_mode) ? cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id : cnx->path[0]->first_tuple->p_local_cnxid->cnx_id;
}
```

### Rust body
```rust
    pub fn logging_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }
```
