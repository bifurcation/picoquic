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

## `picoquic/prague.c:picoquic_prague_process_ack`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C updates alpha, congestion window, ssthresh, era limits, and ACK growth; Rust only reads two ECN counters into locals.
* C source: `picoquic/prague.c:251-292`
* C signature: `void picoquic_prague_process_ack(picoquic_cnx_t *, picoquic_path_t *, picoquic_prague_state_t *, picoquic_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/prague.rs:211-221`
* Rust item: `picoquic_prague_process_ack`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = picoquic_prague_get_pkt_ctx(cnx, path_x);
    uint64_t next_sequence = picoquic_cc_get_ack_number(path_x->cnx, path_x);

    if (next_sequence > pr_state->recovery_sequence) {
        /* new period. Update alpha, etc. */
        int64_t delta_ect1 = pkt_ctx->ecn_ect1_total_remote - pr_state->l4s_epoch_ect1;
        int64_t delta_ce = pkt_ctx->ecn_ce_total_remote - pr_state->l4s_epoch_ce;

        if (delta_ect1 >= 0 && delta_ce >= 0 && delta_ce + delta_ect1 > 0 ) {
            /* We are receiving ECN signals, so update alpha and do CWND reduction */
            uint64_t delta_cwin;
            picoquic_prague_update_alpha(path_x, pr_state, delta_ect1, delta_ce, current_time);

            /* Update the ssthresh and the CWIN */
            delta_cwin = (path_x->cwin * pr_state->alpha) / 2048;
            path_x->cwin -= delta_cwin;
            if (path_x->cwin < PICOQUIC_CWIN_MINIMUM) {
                path_x->cwin = PICOQUIC_CWIN_MINIMUM;
            }
            pr_state->ssthresh = path_x->cwin;
        }
        /* reset the era limits */
        picoquic_prague_initialize_era(cnx, path_x, pr_state, current_time);
    }
    /* Increment CWND whether in recovery or not */
    if (pkt_ctx->ecn_ect1_total_remote >= pr_state->l4s_packet_ect1 &&
        pkt_ctx->ecn_ce_total_remote >= pr_state->l4s_packet_ce) {
        uint64_t delta_ect1_ack = pkt_ctx->ecn_ect1_total_remote - pr_state->l4s_packet_ect1;
        uint64_t delta_ce_ack = pkt_ctx->ecn_ce_total_remote - pr_state->l4s_packet_ce;
        uint64_t ack_bytes = ack_state->nb_bytes_acknowledged;
        double frac_not_ce = 1.0;

        if (delta_ce_ack + delta_ect1_ack > 0) {
            frac_not_ce = ((double)delta_ect1_ack) / (double)(delta_ce_ack + delta_ect1_ack);
            ack_bytes = (uint64_t)(frac_not_ce * (double)ack_bytes);
        }
        path_x->cwin += path_x->send_mtu * ack_bytes / path_x->cwin;
    }
}
```

### Rust body
```rust
    let (ecn_ect1_total_remote, ecn_ce_total_remote) = {
        let pkt_ctx = prague_get_pkt_ctx(cnx, path_x);
        (pkt_ctx.ecn_ect1_total_remote, pkt_ctx.ecn_ce_total_remote)
    };
```

## `picoquic/quicctx.c:picoquic_cnx_set_spinbit_policy`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C setter assigns spin_policy from the argument; Rust body is a getter returning spin_policy and takes no policy argument.
* C source: `picoquic/quicctx.c:4533-4537`
* C signature: `void picoquic_cnx_set_spinbit_policy(picoquic_cnx_t *, picoquic_spinbit_version_enum)`
* Rust source: `rs/fq/src/lib.rs:2854-2862`
* Rust item: `set_cnx_spinbit_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->spin_policy = spinbit_policy;
}
```

### Rust body
```rust
    pub fn cnx_spinbit_policy(&self) -> SpinbitVersion {
        self.spin_policy
    }
```

## `picoquic/quicctx.c:picoquic_delete_cnx`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C performs extensive connection cleanup and frees it; Rust body only fetches a connection or returns.
* C source: `picoquic/quicctx.c:5072-5173`
* C signature: `void picoquic_delete_cnx(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:3894-3897`
* Rust item: `delete_connection`

### C body
```c
{
    if (cnx != NULL) {
        PICOQUIC_THREAD_CHECK(cnx->quic);
        if (cnx->memlog_call_back != NULL) {
            cnx->memlog_call_back(cnx, NULL, cnx->memlog_ctx, 1, 0);
        }
        if (cnx->quic->perflog_fn != NULL) {
            (void)(cnx->quic->perflog_fn)(cnx->quic, cnx, 0);
        }

        picoquic_log_close_connection(cnx);

        if (cnx->is_half_open && cnx->quic->current_number_half_open > 0) {
            cnx->quic->current_number_half_open--;
            cnx->is_half_open = 0;
        }

        if (cnx->cnx_state < picoquic_state_disconnected) {
            /* Give the application a chance to clean up its state */
            picoquic_connection_disconnect(cnx);
        }

        if (cnx->alpn != NULL) {
            free((void*)cnx->alpn);
            cnx->alpn = NULL;
        }

        if (cnx->sni != NULL) {
            free((void*)cnx->sni);
            cnx->sni = NULL;
        }

        if (cnx->remote_error_reason != NULL) {
            free((void*)cnx->remote_error_reason);
            cnx->remote_error_reason = NULL;
        }

        if (cnx->retry_token != NULL) {
            free(cnx->retry_token);
            cnx->retry_token = NULL;
        }

        picoquic_delete_sooner_packets(cnx);

        picoquic_remove_cnx_from_list(cnx);
        picoquic_remove_cnx_from_wake_list(cnx);

        for (int i = 0; i < PICOQUIC_NUMBER_OF_EPOCHS; i++) {
            picoquic_crypto_context_free(&cnx->crypto_context[i]);
        }

        picoquic_crypto_context_free(&cnx->crypto_context_new);
        picoquic_crypto_context_free(&cnx->crypto_context_old);

        for (picoquic_packet_context_enum pc = 0;
            pc < picoquic_nb_packet_context; pc++) {
            picoquic_reset_packet_context(cnx, &cnx->pkt_ctx[pc]);
            picoquic_reset_ack_context(&cnx->ack_ctx[pc]);
        }

        while (cnx->first_misc_frame != NULL) {
            picoquic_delete_misc_or_dg(&cnx->first_misc_frame, &cnx->last_misc_frame, cnx->first_misc_frame);
        }

        while (cnx->first_datagram != NULL) {
            picoquic_delete_misc_or_dg(&cnx->first_datagram, &cnx->last_datagram, cnx->first_datagram);
        }

        picosplay_empty_tree(&cnx->queue_data_repeat_tree);

        for (int epoch = 0; epoch < PICOQUIC_NUMBER_OF_EPOCHS; epoch++) {
            picoquic_clear_stream(&cnx->tls_stream[epoch]);
        }

        picosplay_empty_tree(&cnx->stream_tree);

        if (cnx->tls_ctx != NULL) {
            picoquic_tlscontext_free(cnx->tls_ctx, cnx->client_mode);
            cnx->tls_ctx = NULL;
        }

        if (cnx->path != NULL)
        {
            while (cnx->nb_paths > 0) {
                picoquic_dereference_stashed_cnxid(cnx, cnx->path[cnx->nb_paths - 1], 1);
                picoquic_delete_path(cnx, cnx->nb_paths - 1);
            }

            free(cnx->path);
            cnx->path = NULL;
        }

        picoquic_delete_local_cnxid_lists(cnx);
        picoquic_delete_remote_cnxid_stashes(cnx);

        picoquic_unregister_net_icid(cnx);
        picoquic_unregister_net_secret(cnx);

        free(cnx);
    }
}
```

### Rust body
```rust
        let Some(cnx) = self.connections.get(token) else {
            return;
        };
```

## `picoquic/quicctx.c:picoquic_dereference_stashed_cnxid_tuple`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust only takes/clears an optional remote connection ID index and returns if absent; C performs retire queuing, retire_sent updates, possible stashed CID removal, reference decrementing, and nulling.
* C source: `picoquic/quicctx.c:3123-3147`
* C signature: `void picoquic_dereference_stashed_cnxid_tuple(picoquic_cnx_t *, picoquic_path_t *, picoquic_tuple_t *, int)`
* Rust source: `rs/fq/src/internal.rs:5262-5270`
* Rust item: `dereference_stashed_connection_id_tuple`

### C body
```c
{
    if (tuple->p_remote_cnxid != NULL) {
        if (tuple->p_remote_cnxid->nb_path_references <= 1) {
            uint64_t unique_path_id = (cnx->is_multipath_enabled) ? path_x->unique_path_id : 0;
            if (!is_deleting_cnx && !tuple->p_remote_cnxid->retire_sent) {
                /* if this was the last reference, retire the old cnxid */
                if (picoquic_queue_retire_connection_id_frame(cnx, unique_path_id, tuple->p_remote_cnxid->sequence) != 0) {
                    DBG_PRINTF("Could not properly retire CID[%" PRIu64 "]", tuple->p_remote_cnxid->sequence);
                }
                else {
                    tuple->p_remote_cnxid->retire_sent = 1;
                }
            }
            if (is_deleting_cnx || tuple->p_remote_cnxid->retire_acked) {
                /* Delete and perhaps recycle the queued packets */
                (void)picoquic_remove_stashed_cnxid(cnx, path_x->unique_path_id, tuple->p_remote_cnxid, NULL);
            }
        }
        else {
            tuple->p_remote_cnxid->nb_path_references--;
        }
    }
    tuple->p_remote_cnxid = NULL;
}
```

### Rust body
```rust
        let Some(cid_idx) = tuple.remote_connection_id_index.take() else {
            return;
        };
```

## `picoquic/quicctx.c:picoquic_find_or_create_remote_cnxid_stash`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C searches a stash list, optionally allocates/zeroes/appends a new stash, and returns it; Rust body only returns Some(idx), with no visible search, creation, initialization, or linkage.
* C source: `picoquic/quicctx.c:2892-2915`
* C signature: `picoquic_remote_cnxid_stash_t * picoquic_find_or_create_remote_cnxid_stash(picoquic_cnx_t *, uint64_t, int)`
* Rust source: `rs/fq/src/internal.rs:4911-4922`
* Rust item: `find_or_create_remote_connection_id_stash`

### C body
```c
{
    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = cnx->first_remote_cnxid_stash;
    picoquic_remote_cnxid_stash_t** p_previous = &cnx->first_remote_cnxid_stash;

    while (remote_cnxid_stash != NULL && remote_cnxid_stash->unique_path_id != unique_path_id) {
        p_previous = &remote_cnxid_stash->next_stash;
        remote_cnxid_stash = remote_cnxid_stash->next_stash;
    }

    if (remote_cnxid_stash == NULL && do_create) {
        remote_cnxid_stash = (picoquic_remote_cnxid_stash_t*)malloc(sizeof(picoquic_remote_cnxid_stash_t));
        if (remote_cnxid_stash != NULL) {
            memset(remote_cnxid_stash, 0, sizeof(picoquic_remote_cnxid_stash_t));
            remote_cnxid_stash->unique_path_id = unique_path_id;
            *p_previous = remote_cnxid_stash;
        }
    }

    return remote_cnxid_stash;
}
```

### Rust body
```rust
        {
            return Some(idx);
        }
```
