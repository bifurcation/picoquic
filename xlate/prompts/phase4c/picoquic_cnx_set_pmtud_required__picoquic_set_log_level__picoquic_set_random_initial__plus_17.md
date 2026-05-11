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

## Pair `picoquic/quicctx.c:picoquic_cnx_set_pmtud_required`
C: `picoquic/quicctx.c:4563-4567 picoquic_cnx_set_pmtud_required`
Rust: `rs/fq/src/lib.rs:2881-2893 set_pmtud_required`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->pmtud_policy = (is_pmtud_required) ? picoquic_pmtud_required : picoquic_pmtud_basic;
}
```

### Rust body
```rust
    pub fn tls_is_psk_handshake(&self) -> bool {
        self.psk_cipher_suite_id != 0
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_log_level`
C: `picoquic/quicctx.c:4639-4644 picoquic_set_log_level`
Rust: `rs/fq/src/lib.rs:1083-1087 set_log_level`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    /* Only two level for now: log first 100 packets, or log everything. */
    quic->use_long_log = (log_level > 0) ? 1 : 0;
}
```

### Rust body
```rust
        for connection in self.connections.iter_mut() {
            connection.use_long_log = self.use_long_log;
        }
```

## Pair `picoquic/quicctx.c:picoquic_set_random_initial`
C: `picoquic/quicctx.c:4666-4671 picoquic_set_random_initial`
Rust: `rs/fq/src/lib.rs:1112-1114 set_random_initial`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    /* If set, triggers randomization of initial PN numbers. */
    quic->random_initial = (random_initial > 1) ? 2 : ((random_initial > 0) ? 1 : 0);
}
```

### Rust body
```rust
    pub fn set_random_initial(&mut self, random_initial: i32) {
        self.random_initial = random_initial as u8;
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_connection_id_ttl`
C: `picoquic/quicctx.c:4708-4712 picoquic_set_default_connection_id_ttl`
Rust: `rs/fq/src/lib.rs:1854-1861 set_default_connection_id_ttl`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->local_cnxid_ttl = ttl_usec;
}
```

### Rust body
```rust
    pub fn default_connection_id_ttl(&self) -> u64 {
        self.local_connection_id_ttl
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_alpn_select_fn_v2`
C: `picoquic/quicctx.c:4737-4748 picoquic_set_alpn_select_fn_v2`
Rust: `rs/fq/src/lib.rs:1875-1885 set_alpn_select_fn`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    if (quic->default_alpn != NULL) {
        free((void *)quic->default_alpn);
        quic->default_alpn = NULL;
    }
    quic->alpn_select_fn_v2 = alpn_select_fn;
    if (alpn_select_fn != NULL) {
        quic->alpn_select_fn = NULL;
    }
}
```

### Rust body
```rust
    pub fn set_default_callback(&mut self, callback: Option<Box<dyn StreamDataCallback>>) {
        self.default_callback_fn = callback;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_default_callback_function`
C: `picoquic/quicctx.c:4773-4777 picoquic_get_default_callback_function`
Rust: `rs/fq/src/lib.rs:3244-3252 default_callback`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->default_callback_fn;
}
```

### Rust body
```rust
    pub fn default_callback_ctx(&self) -> Option<&dyn core::any::Any> {
        self.default_callback_ctx.as_deref()
    }
```

## Pair `picoquic/quicctx.c:picoquic_create_misc_frame`
C: `picoquic/quicctx.c:4797-4816 picoquic_create_misc_frame`
Rust: `rs/fq/src/internal.rs:15656-15701 create_misc_frame`

### C body
```c
{
    size_t l_alloc = sizeof(picoquic_misc_frame_header_t) + length;

    if (l_alloc < sizeof(picoquic_misc_frame_header_t)) {
        return NULL;
    }
    else {
        picoquic_misc_frame_header_t* head = (picoquic_misc_frame_header_t*)malloc(l_alloc);
        if (head != NULL) {
            memset(head, 0, sizeof(picoquic_misc_frame_header_t));
            head->length = length;
            head->is_pure_ack = is_pure_ack;
            head->pc = pc;
            memcpy(((uint8_t *)head) + sizeof(picoquic_misc_frame_header_t), bytes, length);
        }
        return head;
    }
}
```

### Rust body
```rust
impl Connection {
    pub fn process_version_upgrade(
        &mut self,
        old_version_index: i32,
        new_version_index: i32,
    ) -> i32 {
        self.rejected_version = if old_version_index >= 0 {
            self.proposed_version
        } else {
            self.rejected_version
        };
        self.version_index = new_version_index;
        let version = match new_version_index {
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
            _ => Version::V1,
        };
        self.proposed_version = version as u32;
        self.desired_version = self.proposed_version;
        self.local_parameters.version_negotiation.current = self.proposed_version;
        0
    }
}
```

## Pair `picoquic/quicctx.c:picoquic_delete_misc_or_dg`
C: `picoquic/quicctx.c:4867-4884 picoquic_delete_misc_or_dg`
Rust: `rs/fq/src/internal.rs:13843-13845 delete_misc_or_dg`

### C body
```c
{
    if (frame->next_misc_frame) {
        frame->next_misc_frame->previous_misc_frame = frame->previous_misc_frame;
    }
    else {
        *last = frame->previous_misc_frame;
    }

    if (frame->previous_misc_frame) {
        frame->previous_misc_frame->next_misc_frame = frame->next_misc_frame;
    }
    else {
        *first = frame->next_misc_frame;
    }

    free(frame);
}
```

### Rust body
```rust
pub fn delete_misc_or_dg(queue: &mut VecDeque<MiscFrameHeader>, index: usize) {
    queue.remove(index);
}
```

## Pair `picoquic/quicctx.c:picoquic_reset_cnx`
C: `picoquic/quicctx.c:4939-4989 picoquic_reset_cnx`
Rust: `rs/fq/src/lib.rs:2363-2410 reset_cnx`

### C body
```c
{
    int ret = 0;

    /* Delete the packets queued for retransmission */
    for (picoquic_packet_context_enum pc = 0;
        pc < picoquic_nb_packet_context; pc++) {
        /* Do not reset the application context, in order to keep the 0-RTT
         * packets, and to keep using the same sequence number space in
         * the new connection */
        if (pc != picoquic_packet_context_application) {
            /* TODO: special case for 0-RTT packets! */
            picoquic_reset_packet_context(cnx, &cnx->pkt_ctx[pc]);
            picoquic_reset_ack_context(&cnx->ack_ctx[pc]);
        }
    }

    /* Reset the crypto stream */
    for (int epoch = 0; epoch < PICOQUIC_NUMBER_OF_EPOCHS; epoch++) {
        picoquic_clear_stream(&cnx->tls_stream[epoch]);
        cnx->tls_stream[epoch].consumed_offset = 0;
        cnx->tls_stream[epoch].fin_offset = 0;
        cnx->tls_stream[epoch].sent_offset = 0;
        /* No need to reset the state flags, are they are not used for the crypto stream */
    }

    for (int k = 0; k < 4; k++) {
        picoquic_crypto_context_free(&cnx->crypto_context[k]);
    }

    picoquic_crypto_context_free(&cnx->crypto_context_new);

    ret = picoquic_setup_initial_traffic_keys(cnx);

    /* Reset the TLS context, Re-initialize the tls connection */
    if (cnx->tls_ctx != NULL) {
        picoquic_tlscontext_free(cnx->tls_ctx, cnx->client_mode);
        cnx->tls_ctx = NULL;
    }

    picoquic_log_new_connection(cnx);

    if (ret == 0) {
        ret = picoquic_tlscontext_create(cnx->quic, cnx);
    }
    if (ret == 0) {
        ret = picoquic_initialize_tls_stream(cnx, current_time);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn reset_cnx(&mut self, current_time: Instant) -> Result<(), Error> {
        for pc in 0..crate::NB_PACKET_CONTEXT {
            if pc != PacketContext::Application as usize {
                let pkt_ctx = &mut self.pkt_ctx[pc];
                pkt_ctx.pending.clear();
                pkt_ctx.retransmitted.clear();
                pkt_ctx.send_sequence = 0;
                pkt_ctx.retransmit_sequence = 0;
                pkt_ctx.next_sequence_hole = 0;
                pkt_ctx.retransmitted_queue_size = 0;
                pkt_ctx.highest_acknowledged = u64::MAX;
                pkt_ctx.latest_time_acknowledged = current_time;
                pkt_ctx.highest_acknowledged_time = current_time;
                self.ack_ctx[pc].reset_ack_context();
            }
        }

        for stream in &mut self.tls_stream {
            stream.clear_stream();
            stream.consumed_offset = 0;
            stream.fin_offset = 0;
            stream.sent_offset = 0;
        }

        fn empty_crypto_context() -> crate::internal::CryptoContext {
            crate::internal::CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            }
        }
        for ctx in &mut self.crypto_context {
            *ctx = empty_crypto_context();
        }
        self.crypto_context_new = empty_crypto_context();

        self.setup_initial_traffic_keys()?;
        self.tls_ctx = None;
        if self.quic_ptr.is_null() {
            return Err(Error::InvalidState);
        }
        // SAFETY: quic_ptr is installed by Quic::create_cnx_internal and the
        // owning Quic outlives every connection stored in its arena.
        let quic = unsafe { &mut *self.quic_ptr };
        self.create_tls_context(quic)?;
        self.initialize_tls_stream(current_time)
    }
```

## Pair `picoquic/quicctx.c:picoquic_start_key_rotation`
C: `picoquic/quicctx.c:5036-5058 picoquic_start_key_rotation`
Rust: `rs/fq/src/lib.rs:2790-2793 start_key_rotation`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    /* Verify that a packet of the previous rotation was acked */
    if (cnx->cnx_state != picoquic_state_ready ||
        cnx->crypto_epoch_sequence >
        picoquic_sack_list_last(&cnx->ack_ctx[picoquic_packet_context_application].sack_list)) {
        ret = PICOQUIC_ERROR_KEY_ROTATION_NOT_READY;
    }
    else {
        ret = picoquic_compute_new_rotated_keys(cnx);
    }

    if (ret == 0) {
        picoquic_apply_rotated_keys(cnx, 1);
        picoquic_crypto_context_free(&cnx->crypto_context_old);
        cnx->crypto_epoch_sequence = cnx->pkt_ctx[picoquic_packet_context_application].send_sequence;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn start_key_rotation(&mut self) -> Result<(), Error> {
        // Complex: initiates TLS key update state machine.
        Err(Error::Generic)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_close_reasons`
C: `picoquic/quicctx.c:5181-5189 picoquic_get_close_reasons`
Rust: `rs/fq/src/lib.rs:4496-4503 close_reasons`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    *local_reason = cnx->local_error;
    *remote_reason = cnx->remote_error;
    *local_application_reason = cnx->application_error;
    *remote_application_reason = cnx->remote_application_error;
}
```

### Rust body
```rust
    pub fn close_reasons(&self) -> (u64, u64, u64, u64) {
        (
            self.local_error,
            self.remote_error,
            self.application_error,
            self.remote_application_error,
        )
    }
```

## Pair `picoquic/quicctx.c:picoquic_cnx_by_id`
C: `picoquic/quicctx.c:5216-5240 picoquic_cnx_by_id`
Rust: `rs/fq/src/internal.rs:5822-5842 connection_by_id`

### C body
```c
{
    picoquic_cnx_t* ret = NULL;
    picohash_item* item;
    picoquic_local_cnxid_t key;

    memset(&key, 0, sizeof(key));
    key.cnx_id = cnx_id;

    item = picohash_retrieve(quic->table_cnx_by_id, &key);

    if (item != NULL) {
        ret = ((picoquic_local_cnxid_t*)item->key)->registered_cnx;
        if (l_cid != NULL) {
            *l_cid = ((picoquic_local_cnxid_t*)item->key);
        }
    }
    else if (l_cid != NULL) {
        *l_cid = NULL;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Option<(ConnectionToken, LocalConnectionIdToken)> {
        let ht = self.connection_by_id.lookup(&cnx_id)?;
        let &conn_tok = self.connection_by_id.get(ht)?;
        let lcid_tok = self.connections.get(conn_tok).and_then(|connection| {
            connection
                .local_connection_id_lists
                .iter()
                .flat_map(|list| list.connection_ids.iter().copied())
                .find(|&tok| {
                    connection
                        .local_connection_ids
                        .get(tok)
                        .map(|l_cid| l_cid.connection_id == cnx_id)
                        .unwrap_or(false)
                })
        })?;
        Some((conn_tok, lcid_tok))
    }
```

## Pair `picoquic/quicctx.c:picoquic_register_congestion_control_algorithms`
C: `picoquic/quicctx.c:5304-5309 picoquic_register_congestion_control_algorithms`
Rust: `rs/fq/src/lib.rs:4554-4558 register_congestion_control_algorithms`

### C body
```c
{
    picoquic_congestion_control_algorithms = alg;
    picoquic_nb_congestion_control_algorithms = nb_algorithms;
}
```

### Rust body
```rust
pub fn register_congestion_control_algorithms(alg: &'static [&'static CongestionAlgorithm]) {
    // Best-effort set: if the registry was already initialised, this is a no-op
    // (OnceLock semantics).
    let _ = CC_ALGORITHM_REGISTRY.set(alg.to_vec());
}
```

## Pair `picoquic/quicctx.c:picoquic_set_default_congestion_algorithm_by_name`
C: `picoquic/quicctx.c:5345-5348 picoquic_set_default_congestion_algorithm_by_name`
Rust: `rs/fq/src/lib.rs:4597-4609 set_default_congestion_algorithm_by_name`

### C body
```c
{
    picoquic_set_default_congestion_algorithm_ex(quic, picoquic_get_congestion_algorithm(alg_name), NULL);
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        match get_congestion_algorithm(alg_name) {
            Some(alg) => {
                self.default_congestion_alg = Some(alg);
                self.default_congestion_alg_option_string = None;
                Ok(())
            }
            None => Err(Error::InvalidArgument),
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_congestion_algorithm_ex`
C: `picoquic/quicctx.c:5372-5393 picoquic_set_congestion_algorithm_ex`
Rust: `rs/fq/src/lib.rs:4622-4629 set_congestion_algorithm_ex`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    if (cnx->congestion_alg != NULL) {
        if (cnx->path != NULL) {
            for (int i = 0; i < cnx->nb_paths; i++) {
                cnx->congestion_alg->alg_delete(cnx->path[i]);
            }
        }
    }

    cnx->congestion_alg = alg;
    cnx->congestion_alg_option_string = alg_option_string;

    if (cnx->congestion_alg != NULL) {
        if (cnx->path != NULL) {
            for (int i = 0; i < cnx->nb_paths; i++) {
                cnx->congestion_alg->alg_init(cnx->path[i], alg_option_string, picoquic_get_quic_time(cnx->quic));
            }
        }
    }
}
```

### Rust body
```rust
    ) {
        self.congestion_alg = Some(alg);
        self.congestion_alg_option_string = alg_option_string.map(|s| s.to_owned());
    }
```

## Pair `picoquic/quicctx.c:picoquic_request_forced_probe_up`
C: `picoquic/quicctx.c:5412-5416 picoquic_request_forced_probe_up`
Rust: `rs/fq/src/lib.rs:4644-4658 request_forced_probe_up`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->is_forced_probe_up_required = request_forced_probe_up;
}
```

### Rust body
```rust
    ) {
        self.pacing_decrease_threshold = decrease_threshold;
        self.pacing_increase_threshold = increase_threshold;
        self.is_pacing_update_requested = true;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_rtt`
C: `picoquic/quicctx.c:5438-5442 picoquic_get_rtt`
Rust: `rs/fq/src/lib.rs:4672-4677 rtt`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->smoothed_rtt;
}
```

### Rust body
```rust
    pub fn rtt(&self) -> u64 {
        self.paths
            .first()
            .map(|p| p.smoothed_rtt.ticks())
            .unwrap_or(0)
```

## Pair `picoquic/quicctx.c:picoquic_set_verify_certificate_callback`
C: `picoquic/quicctx.c:5486-5492 picoquic_set_verify_certificate_callback`
Rust: `rs/fq/src/lib.rs:1890-1901 set_verify_certificate_callback`

### C body
```c
    ptls_verify_certificate_t * cb, picoquic_free_verify_certificate_ctx free_fn) {
    PICOQUIC_THREAD_CHECK(quic);
    picoquic_dispose_verify_certificate_callback(quic);

    picoquic_tls_set_verify_certificate_callback(quic, cb, free_fn);
}
```

### Rust body
```rust
    pub fn set_default_stateless_reset_min_interval(&mut self, min_interval: Duration) {
        self.stateless_reset_min_interval = min_interval;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_application_error`
C: `picoquic/quicctx.c:5514-5518 picoquic_get_application_error`
Rust: `rs/fq/src/lib.rs:4482-4490 application_error`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->remote_application_error;
}
```

### Rust body
```rust
    pub fn remote_application_error(&self) -> u64 {
        self.remote_application_error
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_client_authentication`
C: `picoquic/quicctx.c:5543-5546 picoquic_set_client_authentication`
Rust: `rs/fq/src/lib.rs:1654-1662 set_client_authentication`

### C body
```c
void picoquic_set_client_authentication(picoquic_quic_t* quic, int client_authentication) {
    PICOQUIC_THREAD_CHECK(quic);
    picoquic_tls_set_client_authentication(quic, client_authentication);
}
```

### Rust body
```rust
    pub fn set_use_exporter(&mut self, use_exporter: bool) {
        self.use_exporter = use_exporter;
    }
```
