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

## Pair `picoquic/quicctx.c:picoquic_cnx_set_pmtud_policy`
C: `picoquic/quicctx.c:4557-4561 picoquic_cnx_set_pmtud_policy`
Rust: `rs/fq/src/lib.rs:2875-2887 set_pmtud_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->pmtud_policy = pmtud_policy;
}
```

### Rust body
```rust
    pub fn set_pmtud_required(&mut self, is_pmtud_required: bool) {
        self.pmtud_policy = if is_pmtud_required {
            PmtudPolicy::Required
        } else {
            PmtudPolicy::Basic
        };
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_fuzz`
C: `picoquic/quicctx.c:4632-4637 picoquic_set_fuzz`
Rust: `rs/fq/src/lib.rs:1077-1088 set_fuzz`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->fuzz_fn = fuzz_fn;
    quic->fuzz_ctx = fuzz_ctx;
}
```

### Rust body
```rust
    pub fn set_log_level(&mut self, log_level: i32) {
        self.use_long_log = log_level != 0;
        for connection in self.connections.iter_mut() {
            connection.use_long_log = self.use_long_log;
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_is_sslkeylog_enabled`
C: `picoquic/quicctx.c:4659-4663 picoquic_is_sslkeylog_enabled`
Rust: `rs/fq/src/lib.rs:1106-1108 is_sslkeylog_enabled`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->enable_sslkeylog;
}
```

### Rust body
```rust
    pub fn is_sslkeylog_enabled(&self) -> bool {
        self.enable_sslkeylog
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_connection_id_length`
C: `picoquic/quicctx.c:4688-4706 picoquic_set_default_connection_id_length`
Rust: `rs/fq/src/lib.rs:1839-1850 set_default_connection_id_length`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(quic);

    if (cid_length != quic->local_cnxid_length) {
        if (cid_length > PICOQUIC_CONNECTION_ID_MAX_SIZE) {
            ret = PICOQUIC_ERROR_CNXID_CHECK;
        }
        else if (quic->cnx_list != NULL) {
            ret = PICOQUIC_ERROR_CANNOT_CHANGE_ACTIVE_CONTEXT;
        }
        else {
            quic->local_cnxid_length = cid_length;
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn set_default_connection_id_length(&mut self, cid_length: u8) -> Result<(), Error> {
        if cid_length as usize > CONNECTION_ID_MAX_SIZE {
            return Err(Error::Protocol(InternalError::CnxidCheck as u64));
        }
        if self.current_number_connections > 0 {
            return Err(Error::Protocol(
                InternalError::CannotChangeActiveContext as u64,
            ));
        }
        self.local_connection_id_length = cid_length;
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_alpn_select_fn`
C: `picoquic/quicctx.c:4727-4735 picoquic_set_alpn_select_fn`
Rust: `rs/fq/src/lib.rs:1875-1885 set_alpn_select_fn`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    if (quic->default_alpn != NULL) {
        free((void *)quic->default_alpn);
        quic->default_alpn = NULL;
    }
    quic->alpn_select_fn = alpn_select_fn;
}
```

### Rust body
```rust
    pub fn set_default_callback(&mut self, callback: Option<Box<dyn StreamDataCallback>>) {
        self.default_callback_fn = callback;
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_callback`
C: `picoquic/quicctx.c:4765-4771 picoquic_set_callback`
Rust: `rs/fq/src/lib.rs:3093-3095 set_callback`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->callback_fn = callback_fn;
    cnx->callback_ctx = callback_ctx;
}
```

### Rust body
```rust
    pub fn set_callback(&mut self, callback: Option<Box<dyn StreamDataCallback>>) {
        self.callback_fn = callback;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_callback_context`
C: `picoquic/quicctx.c:4791-4795 picoquic_get_callback_context`
Rust: `rs/fq/src/lib.rs:3106-3124 callback_ctx`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->callback_ctx;
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

## Pair `picoquic/quicctx.c:picoquic_purge_misc_frames_after_ready`
C: `picoquic/quicctx.c:4850-4865 picoquic_purge_misc_frames_after_ready`
Rust: `rs/fq/src/internal.rs:13833-13838 purge_misc_frames_after_ready`

### C body
```c
{
    picoquic_misc_frame_header_t* misc_frame;
    PICOQUIC_THREAD_CHECK(cnx->quic);
        
    misc_frame = cnx->first_misc_frame;

    while (misc_frame != NULL) {
        picoquic_misc_frame_header_t* next_frame = misc_frame->next_misc_frame;

        if (misc_frame->pc != picoquic_packet_context_application) {
            picoquic_delete_misc_or_dg(&cnx->first_misc_frame, &cnx->last_misc_frame, misc_frame);
        }
        misc_frame = next_frame;
    }
}
```

### Rust body
```rust
    pub fn purge_misc_frames_after_ready(&mut self) {
        if self.connection_state == State::Ready {
            self.misc_frames
                .retain(|frame| frame.packet_context == PacketContext::Application);
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_reset_packet_context`
C: `picoquic/quicctx.c:4905-4922 picoquic_reset_packet_context`
Rust: `rs/fq/src/internal.rs:5679-5687 reset_packet_context`

### C body
```c
{
    while (pkt_ctx->pending_last != NULL) {
        (void)picoquic_dequeue_retransmit_packet(cnx, pkt_ctx, pkt_ctx->pending_last, 1, 0);
    }
    
    while (pkt_ctx->retransmitted_newest != NULL) {
        picoquic_dequeue_retransmitted_packet(cnx, pkt_ctx, pkt_ctx->retransmitted_newest);
    }

    pkt_ctx->retransmitted_oldest = NULL;

    /* Reset the ECN data */
    pkt_ctx->ecn_ect0_total_remote = 0;
    pkt_ctx->ecn_ect1_total_remote = 0;
    pkt_ctx->ecn_ce_total_remote = 0;
}
```

### Rust body
```rust
    pub fn reset_packet_context(&mut self, pkt_ctx: &mut PacketContextState) {
        // C: picoquic_reset_packet_context
        pkt_ctx.pending.clear();
        pkt_ctx.retransmitted.clear();
        pkt_ctx.send_sequence = 0;
        pkt_ctx.retransmit_sequence = 0;
        pkt_ctx.next_sequence_hole = 0;
        pkt_ctx.retransmitted_queue_size = 0;
    }
```

## Pair `picoquic/quicctx.c:picoquic_connection_disconnect`
C: `picoquic/quicctx.c:5026-5034 picoquic_connection_disconnect`
Rust: `rs/fq/src/internal.rs:5808-5811 connection_disconnect`

### C body
```c
{
    if (cnx->cnx_state != picoquic_state_disconnected) {
        cnx->cnx_state = picoquic_state_disconnected;
        if (cnx->callback_fn) {
            (void)(cnx->callback_fn)(cnx, 0, NULL, 0, picoquic_callback_close, cnx->callback_ctx, NULL);
        }
    }
}
```

### Rust body
```rust
    pub fn connection_disconnect(&mut self) {
        // C: picoquic_connection_disconnect
        self.connection_state = crate::State::Disconnected;
    }
```

## Pair `picoquic/quicctx.c:picoquic_is_handshake_error`
C: `picoquic/quicctx.c:5175-5179 picoquic_is_handshake_error`
Rust: `rs/fq/src/lib.rs:999-1002 is_handshake_error`

### C body
```c
{
    return ((error_code & 0xFF00) == PICOQUIC_TRANSPORT_CRYPTO_ERROR(0) ||
        error_code == PICOQUIC_TLS_HANDSHAKE_FAILED);
}
```

### Rust body
```rust
pub fn is_handshake_error(error_code: u64) -> bool {
    // TLS handshake errors occupy the range 0x0100..=0x01ff
    (error_code >> 8) == 1
}
```

## Pair `picoquic/quicctx.c:picoquic_set_rejected_version`
C: `picoquic/quicctx.c:5209-5214 picoquic_set_rejected_version`
Rust: `rs/fq/src/lib.rs:2431-2444 set_rejected_version`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->desired_version = rejected_version;
    cnx->do_version_negotiation = 1;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves path creation and network probing.
        Err(Error::Generic)
    }
```

## Pair `picoquic/quicctx.c:picoquic_cnx_by_secret`
C: `picoquic/quicctx.c:5277-5292 picoquic_cnx_by_secret`
Rust: `rs/fq/src/internal.rs:5872-5879 connection_by_secret`

### C body
```c
{
    picoquic_cnx_t* ret = NULL;
    picohash_item* item;
    picoquic_cnx_t dummy_cnx = { 0 };

    picoquic_store_addr(&dummy_cnx.registered_secret_addr, addr);
    memcpy(dummy_cnx.registered_reset_secret, reset_secret, PICOQUIC_RESET_SECRET_SIZE);

    item = picohash_retrieve(quic->table_cnx_by_secret, &dummy_cnx);

    if (item != NULL) {
        ret = ((picoquic_cnx_t*)item->key);
    }
    return ret;
}
```

### Rust body
```rust
        if reset_secret.len() < RESET_SECRET_SIZE {
            return None;
        }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_congestion_algorithm`
C: `picoquic/quicctx.c:5340-5343 picoquic_set_default_congestion_algorithm`
Rust: `rs/fq/src/lib.rs:4578-4581 set_default_congestion_algorithm`

### C body
```c
{
    picoquic_set_default_congestion_algorithm_ex(quic, alg, NULL);
}
```

### Rust body
```rust
    pub fn set_default_congestion_algorithm(&mut self, algo: &'static CongestionAlgorithm) {
        self.default_congestion_alg = Some(algo);
        self.default_congestion_alg_option_string = None;
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_preemptive_repeat_per_cnx`
C: `picoquic/quicctx.c:5366-5370 picoquic_set_preemptive_repeat_per_cnx`
Rust: `rs/fq/src/lib.rs:4446-4454 set_preemptive_repeat`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->is_preemptive_repeat_enabled = (do_repeat) ? 1 : 0;
}
```

### Rust body
```rust
    pub fn preemptive_repeat_count(&self) -> u64 {
        self.nb_preemptive_repeat
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_feedback_loss_notification`
C: `picoquic/quicctx.c:5406-5410 picoquic_set_feedback_loss_notification`
Rust: `rs/fq/src/lib.rs:4639-4646 set_feedback_loss_notification`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->is_lost_feedback_notification_required = should_notify;
}
```

### Rust body
```rust
    pub fn request_forced_probe_up(&mut self, request_forced_probe_up: bool) {
        self.is_forced_probe_up_required = request_forced_probe_up;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_cwin`
C: `picoquic/quicctx.c:5432-5436 picoquic_get_cwin`
Rust: `rs/fq/src/lib.rs:4666-4677 cwin`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->cwin;
}
```

### Rust body
```rust
    pub fn rtt(&self) -> u64 {
        self.paths
            .first()
            .map(|p| p.smoothed_rtt.ticks())
            .unwrap_or(0)
    }
```

## Pair `picoquic/quicctx.c:picoquic_disable_keep_alive`
C: `picoquic/quicctx.c:5480-5484 picoquic_disable_keep_alive`
Rust: `rs/fq/src/lib.rs:4462-4464 disable_keep_alive`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->keep_alive_interval = 0;
}
```

### Rust body
```rust
    pub fn disable_keep_alive(&mut self) {
        self.keep_alive_interval = Duration::from_ticks(0);
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_remote_error`
C: `picoquic/quicctx.c:5508-5512 picoquic_get_remote_error`
Rust: `rs/fq/src/lib.rs:4477-4484 remote_error`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->remote_error;
}
```

### Rust body
```rust
    pub fn application_error(&self) -> u64 {
        self.application_error
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_data_received`
C: `picoquic/quicctx.c:5537-5541 picoquic_get_data_received`
Rust: `rs/fq/src/lib.rs:4530-4538 data_received`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->data_received;
}
```

### Rust body
```rust
    pub fn is_still_logging(&self) -> bool {
        self.nb_packets_logged < LOG_PACKET_MAX_SEQUENCE as u64 || self.use_long_log
    }
```
