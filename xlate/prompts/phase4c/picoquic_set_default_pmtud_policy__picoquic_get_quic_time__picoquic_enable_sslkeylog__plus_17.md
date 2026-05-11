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

## Pair `picoquic/quicctx.c:picoquic_set_default_pmtud_policy`
C: `picoquic/quicctx.c:4551-4555 picoquic_set_default_pmtud_policy`
Rust: `rs/fq/src/lib.rs:3238-3246 set_default_pmtud_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_pmtud_policy = pmtud_policy;
}
```

### Rust body
```rust
    pub fn default_callback(&self) -> Option<&dyn StreamDataCallback> {
        self.default_callback_fn.as_deref()
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_quic_time`
C: `picoquic/quicctx.c:4618-4630 picoquic_get_quic_time`
Rust: `rs/fq/src/lib.rs:3158-3253 quic_time`

### C body
```c
{
    uint64_t now;
    PICOQUIC_THREAD_CHECK(quic);
    if (quic->p_simulated_time == NULL) {
        now = picoquic_current_time();
    }
    else {
        now = *quic->p_simulated_time;
    }

    return now;
}
```

### Rust body
```rust
impl Quic {
    /// Borrow the first connection registered with this context.
    pub fn first_connection(&mut self) -> Option<&mut Connection> {
        self.connections.iter_mut().next()
    }

    /// Return the connection that follows the one identified by `current_token`
    /// in arena insertion order, or `None` when `current_token` is the last
    /// live connection.
    ///
    /// C: `picoquic_get_next_cnx` — `cnx->next_in_table`.
    ///
    /// The C intrusive linked list (`next_in_table` / `previous_in_table`) is
    /// replaced by an arena; this method scans forward from the slot after
    /// `current_token.idx` to find the next occupied slot.  Typical usage:
    ///
    /// ```ignore
    /// let mut tok = quic.first_connection().and_then(|c| c.own_token);
    /// while let Some(t) = tok {
    ///     let cnx = quic.connections.get_mut(t).unwrap();
    ///     // ... process cnx ...
    ///     tok = quic.next_cnx(t).and_then(|c| c.own_token);
    /// }
    /// ```
    pub fn next_cnx(&mut self, current_token: ConnectionToken) -> Option<&mut Connection> {
        let next_idx = current_token.slot_idx() + 1;
        self.connections.next_after_idx(next_idx)
    }

    /// Compute the number of microseconds until *any* connection on
    /// this context next needs attention, capped at `delay_max`.
    pub fn next_wake_delay(&self, current_time: Instant, delay_max: i64) -> i64 {
        let now = current_time.ticks();
        // Find the minimum next_wake_time across all connections.
        let earliest = self
            .connections
            .iter()
            .map(|c| c.next_wake_time.ticks())
            .min();
        match earliest {
            None => delay_max,
            Some(t) if t <= now => 0,
            Some(t) => ((t - now) as i64).min(delay_max),
        }
    }

    /// Wall-clock time at which the next event is scheduled.
    pub fn next_wake_time(&self, current_time: Instant) -> u64 {
        let now = current_time.ticks();
        self.connections
            .iter()
            .map(|c| c.next_wake_time.ticks())
            .min()
            .unwrap_or(now)
    }

    /// Return the earliest connection that wakes before `wake_time`,
    /// or `None` if none qualify.
    /// C: `picoquic_get_earliest_cnx_to_wake`.
    pub fn earliest_cnx_to_wake(&mut self, wake_time: Instant) -> Option<&mut Connection> {
        let threshold = wake_time.ticks();
        self.connections
            .iter_mut()
            .filter(|c| c.next_wake_time.ticks() <= threshold)
            .min_by_key(|c| c.next_wake_time.ticks())
    }

    /// Borrow the connection currently advancing through its state
    /// machine, if any (`get_cnx_in_progress` in C).
    pub fn connection_in_progress(&mut self) -> Option<&mut Connection> {
        let tok = self.connection_in_progress?;
        self.connections.get_mut(tok)
    }

    /// Default PMTUD policy applied to new connections.
    pub fn set_default_pmtud_policy(&mut self, pmtud_policy: PmtudPolicy) {
        self.default_pmtud_policy = pmtud_policy;
    }

    /// Borrow the default stream callback installed on this context.
    /// C: `picoquic_get_default_callback_function` — `quic->default_callback_fn`.
    pub fn default_callback(&self) -> Option<&dyn StreamDataCallback> {
        self.default_callback_fn.as_deref()
    }

    /// Borrow the opaque context associated with the default callback.
    /// C: `picoquic_get_default_callback_context` — `quic->default_callback_ctx`.
    pub fn default_callback_ctx(&self) -> Option<&dyn core::any::Any> {
        self.default_callback_ctx.as_deref()
    }
}
```

## Pair `picoquic/quicctx.c:picoquic_enable_sslkeylog`
C: `picoquic/quicctx.c:4652-4657 picoquic_enable_sslkeylog`
Rust: `rs/fq/src/lib.rs:1101-1108 set_sslkeylog_enabled`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->enable_sslkeylog = (enable_sslkeylog != 0);
}
```

### Rust body
```rust
    pub fn is_sslkeylog_enabled(&self) -> bool {
        self.enable_sslkeylog
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_padding_policy`
C: `picoquic/quicctx.c:4681-4686 picoquic_set_padding_policy`
Rust: `rs/fq/src/lib.rs:1123-1144 set_padding_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->padding_minsize_default = padding_min_size;
    quic->padding_multiple_default = padding_multiple;
}
```

### Rust body
```rust
    pub fn set_key_log_file(&mut self, keylog_filename: Option<&str>) {
        // Open or clear the SSL keylog file.  When a path is given, open it
        // for appending; errors are silently ignored (matching the C behaviour
        // of falling back to no logging rather than crashing).
        use std::fs::OpenOptions;
        match keylog_filename {
            Some(path) => {
                if let Ok(f) = OpenOptions::new().create(true).append(true).open(path) {
                    self.f_log = Some(Box::new(f));
                }
            }
            None => {
                self.f_log = None;
            }
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_mtu_max`
C: `picoquic/quicctx.c:4720-4725 picoquic_set_mtu_max`
Rust: `rs/fq/src/lib.rs:1864-1878 set_mtu_max`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->mtu_max = mtu_max;
    quic->default_tp.max_packet_size = mtu_max;
}
```

### Rust body
```rust
    pub fn set_alpn_select_fn(&mut self, alpn_select_fn: Option<Box<dyn AlpnSelect>>) {
        self.default_alpn = None;
        self.alpn_select_fn = alpn_select_fn;
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_stateless_reset_min_interval`
C: `picoquic/quicctx.c:4758-4763 picoquic_set_default_stateless_reset_min_interval`
Rust: `rs/fq/src/lib.rs:1899-1907 set_default_stateless_reset_min_interval`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->stateless_reset_next_time = picoquic_get_quic_time(quic);
    quic->stateless_reset_min_interval = min_interval_usec;
}
```

### Rust body
```rust
    pub fn set_max_simultaneous_logs(&mut self, max_simultaneous_logs: u32) {
        self.max_simultaneous_logs = max_simultaneous_logs;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_callback_function`
C: `picoquic/quicctx.c:4785-4789 picoquic_get_callback_function`
Rust: `rs/fq/src/lib.rs:3100-3108 callback`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->callback_fn;
}
```

### Rust body
```rust
    pub fn callback_ctx(&self) -> Option<&dyn core::any::Any> {
        self.callback_ctx.as_deref()
    }
```

## Pair `picoquic/quicctx.c:picoquic_queue_misc_frame`
C: `picoquic/quicctx.c:4844-4848 picoquic_queue_misc_frame`
Rust: `rs/fq/src/lib.rs:3111-3124 queue_misc_frame`

### C body
```c
{
    return picoquic_queue_misc_or_dg_frame(cnx, &cnx->first_misc_frame, &cnx->last_misc_frame, bytes, length, is_pure_ack, pc);
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        use crate::internal::MiscFrameHeader;
        self.misc_frames.push_back(MiscFrameHeader {
            bytes: bytes.to_vec(),
            packet_context: pc,
            is_pure_ack: is_pure_ack as i32,
        });
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_reset_ack_context`
C: `picoquic/quicctx.c:4893-4902 picoquic_reset_ack_context`
Rust: `rs/fq/src/internal.rs:13871-13873 reset_ack_context`

### C body
```c
{
    picoquic_clear_ack_ctx(ack_ctx);

    picoquic_sack_list_init(&ack_ctx->sack_list);

    ack_ctx->ecn_ect0_total_local = 0;
    ack_ctx->ecn_ect1_total_local = 0;
    ack_ctx->ecn_ce_total_local = 0;
}
```

### Rust body
```rust
    pub fn reset_ack_context(&mut self) {
        self.clear_ack_ctx();
    }
```

## Pair `picoquic/quicctx.c:picoquic_connection_error`
C: `picoquic/quicctx.c:5021-5024 picoquic_connection_error`
Rust: `rs/fq/src/internal.rs:5722-5743 connection_error`

### C body
```c
{
    return picoquic_connection_error_ex(cnx, local_error, frame_type, NULL);
}
```

### Rust body
```rust
    ) -> i32 {
        // C: picoquic_connection_error_ex
        self.local_error = local_error;
        self.offending_frame_type = frame_type;
        self.local_error_reason = local_reason.map(|s| s.to_owned());
        // Move to disconnecting state if not already past that.
        if (self.connection_state as u32) < crate::State::Disconnecting as u32 {
            self.connection_state = crate::State::Disconnecting;
        }
        local_error as i32
    }
```

## Pair `picoquic/quicctx.c:picoquic_delete_cnx`
C: `picoquic/quicctx.c:5072-5173 picoquic_delete_cnx`
Rust: `rs/fq/src/internal.rs:3894-3897 delete_connection`

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

## Pair `picoquic/quicctx.c:picoquic_set_desired_version`
C: `picoquic/quicctx.c:5201-5207 picoquic_set_desired_version`
Rust: `rs/fq/src/lib.rs:2426-2433 set_desired_version`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->desired_version = desired_version;
    cnx->do_version_negotiation = 1;
}
```

### Rust body
```rust
    pub fn set_rejected_version(&mut self, rejected_version: u32) {
        self.rejected_version = rejected_version;
    }
```

## Pair `picoquic/quicctx.c:picoquic_cnx_by_icid`
C: `picoquic/quicctx.c:5258-5275 picoquic_cnx_by_icid`
Rust: `rs/fq/src/internal.rs:5855-5867 connection_by_icid`

### C body
```c
{
    picoquic_cnx_t* ret = NULL;
    picohash_item* item;
    picoquic_cnx_t dummy_cnx = { 0 };

    picoquic_store_addr(&dummy_cnx.registered_icid_addr, addr);
    dummy_cnx.initial_cnxid = *icid;
    dummy_cnx.quic = quic;

    item = picohash_retrieve(quic->table_cnx_by_icid, &dummy_cnx);

    if (item != NULL) {
        ret = (picoquic_cnx_t*)item->key;
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Option<ConnectionToken> {
        let key = (
            *icid,
            addr.copied().unwrap_or_else(crate::unspecified_socket_addr),
        );
        let ht = self.connection_by_icid.lookup(&key)?;
        let &tok = self.connection_by_icid.get(ht)?;
        Some(tok)
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_congestion_algorithm_ex`
C: `picoquic/quicctx.c:5333-5338 picoquic_set_default_congestion_algorithm_ex`
Rust: `rs/fq/src/lib.rs:4585-4592 set_default_congestion_algorithm_ex`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_congestion_alg = alg;
    quic->default_congestion_alg_option_string = alg_option_string;
}
```

### Rust body
```rust
    ) {
        self.default_congestion_alg = Some(alg);
        self.default_congestion_alg_option_string = alg_option_string.map(|s| s.to_owned());
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_preemptive_repeat_policy`
C: `picoquic/quicctx.c:5360-5364 picoquic_set_preemptive_repeat_policy`
Rust: `rs/fq/src/lib.rs:4439-4448 set_preemptive_repeat_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->is_preemptive_repeat_enabled = (do_repeat) ? 1 : 0;
}
```

### Rust body
```rust
    pub fn set_preemptive_repeat(&mut self, do_repeat: bool) {
        self.is_preemptive_repeat_enabled = do_repeat;
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_priority_limit_for_bypass`
C: `picoquic/quicctx.c:5400-5404 picoquic_set_priority_limit_for_bypass`
Rust: `rs/fq/src/lib.rs:4633-4641 set_priority_limit_for_bypass`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->priority_limit_for_bypass = priority_limit;
}
```

### Rust body
```rust
    pub fn set_feedback_loss_notification(&mut self, should_notify: bool) {
        self.is_lost_feedback_notification_required = should_notify;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_pacing_rate`
C: `picoquic/quicctx.c:5426-5430 picoquic_get_pacing_rate`
Rust: `rs/fq/src/lib.rs:4661-4668 pacing_rate`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->pacing.rate;
}
```

### Rust body
```rust
    pub fn cwin(&self) -> u64 {
        self.paths.first().map(|p| p.cwin).unwrap_or(0)
    }
```

## Pair `picoquic/quicctx.c:picoquic_enable_keep_alive`
C: `picoquic/quicctx.c:5459-5478 picoquic_enable_keep_alive`
Rust: `rs/fq/src/lib.rs:4457-4464 enable_keep_alive`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    if (interval == 0) {
        /* Use the negotiated value */
        uint64_t idle_timeout = cnx->idle_timeout;
        if (idle_timeout == 0) {
            /* Idle timeout is only initialized after parameters are negotiated  */
            idle_timeout = cnx->local_parameters.max_idle_timeout * 1000ull;
        }
        /* Ensure at least 3 PTO*/
        if (idle_timeout < 3 * cnx->path[0]->retransmit_timer) {
            idle_timeout = 3 * cnx->path[0]->retransmit_timer;
        }
        /* set interval to half that value */
        cnx->keep_alive_interval = idle_timeout / 2;
    } else {
        cnx->keep_alive_interval = interval;
    }
}
```

### Rust body
```rust
    pub fn disable_keep_alive(&mut self) {
        self.keep_alive_interval = Duration::from_ticks(0);
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_local_error`
C: `picoquic/quicctx.c:5502-5506 picoquic_get_local_error`
Rust: `rs/fq/src/lib.rs:4472-4479 local_error`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->local_error;
}
```

### Rust body
```rust
    pub fn remote_error(&self) -> u64 {
        self.remote_error
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_data_sent`
C: `picoquic/quicctx.c:5531-5535 picoquic_get_data_sent`
Rust: `rs/fq/src/lib.rs:4525-4532 data_sent`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->data_sent;
}
```

### Rust body
```rust
    pub fn data_received(&self) -> u64 {
        self.data_received
    }
```
