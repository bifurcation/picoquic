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

## Pair `picoquic/quicctx.c:picoquic_seed_bandwidth`
C: `picoquic/quicctx.c:4539-4549 picoquic_seed_bandwidth`
Rust: `rs/fq/src/internal.rs:9087-9106 seed_bandwidth`

### C body
```c
{
    cnx->seed_rtt_min = rtt_min;
    cnx->seed_cwin = cwin;
    if (ip_addr_length > PICOQUIC_STORED_IP_MAX) {
        ip_addr_length = PICOQUIC_STORED_IP_MAX;
    }
    memcpy(cnx->seed_ip_addr, ip_addr, ip_addr_length);
    cnx->seed_ip_addr_length = ip_addr_length;
}
```

### Rust body
```rust
        if let Some(path) = self.paths.first_mut() {
            path.rtt_min = if rtt_min.ticks() == 0 {
                path.rtt_min
            } else {
                rtt_min
            };
            if cwin > 0 {
                path.cwin = cwin;
                path.bandwidth_estimate = cwin
                    .saturating_mul(1_000_000)
                    .saturating_div(path.rtt_min.ticks().max(1));
                path.bandwidth_estimate_max =
                    path.bandwidth_estimate_max.max(path.bandwidth_estimate);
            }
            let ip_bytes = stored_ip_bytes(ip_addr);
            let len = ip_bytes.len().min(path.ip_client_remote.len());
            path.ip_client_remote[..len].copy_from_slice(&ip_bytes[..len]);
            path.ip_client_remote_length = len as u8;
        }
```

## Pair `picoquic/quicctx.c:picoquic_current_time`
C: `picoquic/quicctx.c:4569-4612 picoquic_current_time`
Rust: `rs/fq/src/lib.rs:483-498 current_time`

### C body
```c
{
    uint64_t now;
#ifdef _WINDOWS
    FILETIME ft;
    /*
    * The GetSystemTimeAsFileTime API returns  the number
    * of 100-nanosecond intervals since January 1, 1601 (UTC),
    * in FILETIME format.
    */
    GetSystemTimePreciseAsFileTime(&ft);

    /*
    * Convert to plain 64 bit format, without making
    * assumptions about the FILETIME structure alignment.
    */
    now = ft.dwHighDateTime;
    now <<= 32;
    now |= ft.dwLowDateTime;
    /*
    * Convert units from 100ns to 1us
    */
    now /= 10;
    /*
    * Account for microseconds elapsed between 1601 and 1970.
    */
    now -= 11644473600000000ULL;
#elif defined(CLOCK_MONOTONIC)
    /*
    * Use CLOCK_MONOTONIC if exists (more accurate)
    */
    struct timespec currentTime;
    (void)clock_gettime(CLOCK_MONOTONIC, &currentTime);
    now = (currentTime.tv_sec * 1000000ull) + currentTime.tv_nsec / 1000ull;
#else
    struct timeval tv;
    (void)gettimeofday(&tv, NULL);
    now = (tv.tv_sec * 1000000ull) + tv.tv_usec;
#endif
    return now;
}
```

### Rust body
```rust
impl Quic {
    /// Virtual time used by this QUIC context (wall-clock or
    /// simulated, depending on whether a simulated-time pointer was
    /// supplied at creation).  C: `get_quic_time`.
    pub fn time(&self) -> u64 {
        current_time()
    }
}
```

## Pair `picoquic/quicctx.c:picoquic_use_unique_log_names`
C: `picoquic/quicctx.c:4646-4650 picoquic_use_unique_log_names`
Rust: `rs/fq/src/lib.rs:1093-1095 set_use_unique_log_names`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->use_unique_log_names = use_unique_log_names;
}
```

### Rust body
```rust
    pub fn set_use_unique_log_names(&mut self, use_unique_log_names: bool) {
        self.use_unique_log_names = use_unique_log_names;
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_packet_train_mode`
C: `picoquic/quicctx.c:4673-4679 picoquic_set_packet_train_mode`
Rust: `rs/fq/src/lib.rs:1118-1126 set_packet_train_mode`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    /* TODO: consider setting high water mark for pacing. */
    /* If set, wait until pacing bucket is full enough to allow further transmissions. */
    quic->packet_train_mode = (train_mode > 0) ? 1 : 0;
}
```

### Rust body
```rust
    pub fn set_padding_policy(&mut self, padding_min_size: u32, padding_multiple: u32) {
        self.padding_minsize_default = padding_min_size;
        self.padding_multiple_default = padding_multiple;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_default_connection_id_ttl`
C: `picoquic/quicctx.c:4714-4718 picoquic_get_default_connection_id_ttl`
Rust: `rs/fq/src/lib.rs:1859-1866 default_connection_id_ttl`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->local_cnxid_ttl;
}
```

### Rust body
```rust
    pub fn set_mtu_max(&mut self, mtu_max: u32) {
        self.mtu_max = mtu_max;
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_callback`
C: `picoquic/quicctx.c:4750-4756 picoquic_set_default_callback`
Rust: `rs/fq/src/lib.rs:1883-1895 set_default_callback`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_callback_fn = callback_fn;
    quic->default_callback_ctx = callback_ctx;
}
```

### Rust body
```rust
    ) {
        self.tls_callbacks = callback;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_default_callback_context`
C: `picoquic/quicctx.c:4779-4783 picoquic_get_default_callback_context`
Rust: `rs/fq/src/lib.rs:3250-3252 default_callback_ctx`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->default_callback_ctx;
}
```

### Rust body
```rust
    pub fn default_callback_ctx(&self) -> Option<&dyn core::any::Any> {
        self.default_callback_ctx.as_deref()
```

## Pair `picoquic/quicctx.c:picoquic_queue_misc_or_dg_frame`
C: `picoquic/quicctx.c:4818-4842 picoquic_queue_misc_or_dg_frame`
Rust: `rs/fq/src/internal.rs:13816-13829 queue_misc_or_dg_frame`

### C body
```c
{
    int ret = 0;
    picoquic_misc_frame_header_t* misc_frame = picoquic_create_misc_frame(bytes, length, is_pure_ack, pc);

    if (misc_frame == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    } else {
        if (*last == NULL) {
            *first = misc_frame;
            *last = misc_frame;
        }
        else {
            (*last)->next_misc_frame = misc_frame;
            misc_frame->previous_misc_frame = *last;
            *last = misc_frame;
        }
    }

    picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        queue.push_back(MiscFrameHeader {
            bytes: bytes.to_vec(),
            packet_context: pc,
            is_pure_ack: if is_pure_ack { 1 } else { 0 },
        });
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_clear_ack_ctx`
C: `picoquic/quicctx.c:4886-4890 picoquic_clear_ack_ctx`
Rust: `rs/fq/src/internal.rs:13849-13866 clear_ack_ctx`

### C body
```c
{
    picoquic_sack_list_free(&ack_ctx->sack_list);

}
```

### Rust body
```rust
    pub fn clear_ack_ctx(&mut self) {
        self.sack_list.free();
        self.time_stamp_largest_received = crate::Instant::from_ticks(0);
        self.crypto_rotation_sequence = 0;
        self.ecn_ect0_total_local = 0;
        self.ecn_ect1_total_local = 0;
        self.ecn_ce_total_local = 0;
        self.sending_ecn_ack = false;
        for act in &mut self.act {
            act.highest_ack_sent = 0;
            act.highest_ack_sent_time = crate::Instant::from_ticks(0);
            act.time_oldest_unack_packet_received = crate::Instant::from_ticks(0);
            act.ack_needed = false;
            act.ack_after_fin = false;
            act.out_of_order_received = false;
            act.is_immediate_ack_required = false;
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_connection_error_ex`
C: `picoquic/quicctx.c:4991-5019 picoquic_connection_error_ex`
Rust: `rs/fq/src/internal.rs:5728-5743 connection_error_ex`

### C body
```c
{
    if (local_error > PICOQUIC_ERROR_CLASS) {
        local_error = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
    }

    if (cnx->cnx_state == picoquic_state_ready || 
        cnx->cnx_state == picoquic_state_client_ready_start || cnx->cnx_state == picoquic_state_server_false_start) {
        cnx->local_error = local_error;
        cnx->local_error_reason = local_reason;
        cnx->cnx_state = picoquic_state_disconnecting;
    } else if (cnx->cnx_state < picoquic_state_server_false_start) {
        if (cnx->cnx_state != picoquic_state_handshake_failure &&
            cnx->cnx_state != picoquic_state_handshake_failure_resend) {
            cnx->local_error = local_error;
            cnx->local_error_reason = local_reason;
            cnx->cnx_state = picoquic_state_handshake_failure;
        }
    }

    cnx->offending_frame_type = frame_type;

    picoquic_log_app_message(cnx, "Protocol error 0x%x, frame %" PRIu64 ", reason: %s",
        local_error, frame_type, (local_reason==NULL)?"?":local_reason);
    DBG_PRINTF("Protocol error 0x%x, frame %" PRIu64 ", reason: %s",
        local_error, frame_type, (local_reason==NULL)?"?":local_reason);

    return PICOQUIC_ERROR_DETECTED;
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

## Pair `picoquic/quicctx.c:picoquic_delete_sooner_packets`
C: `picoquic/quicctx.c:5060-5070 picoquic_delete_sooner_packets`
Rust: `rs/fq/src/internal.rs:14982-14984 delete_sooner_packets`

### C body
```c
{
    picoquic_stateless_packet_t* packet = cnx->first_sooner;

    while (packet != NULL) {
        picoquic_stateless_packet_t* next_packet = packet->next_packet;
        picoquic_delete_stateless_packet(packet);
        packet = next_packet;
    }
    cnx->first_sooner = NULL;
}
```

### Rust body
```rust
    pub fn delete_sooner_packets(&mut self) {
        self.sooner_stateless.clear();
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_app_wake_time`
C: `picoquic/quicctx.c:5191-5199 picoquic_set_app_wake_time`
Rust: `rs/fq/src/lib.rs:2421-2428 set_app_wake_time`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->app_wake_time = app_wake_time;
    if (cnx->app_wake_time != 0 && cnx->app_wake_time < cnx->next_wake_time) {
        picoquic_reinsert_by_wake_time(cnx->quic, cnx, app_wake_time);
    }
}
```

### Rust body
```rust
    pub fn set_desired_version(&mut self, desired_version: u32) {
        self.desired_version = desired_version;
    }
```

## Pair `picoquic/quicctx.c:picoquic_cnx_by_net`
C: `picoquic/quicctx.c:5242-5256 picoquic_cnx_by_net`
Rust: `rs/fq/src/internal.rs:5846-5851 connection_by_net`

### C body
```c
{
    picoquic_cnx_t* ret = NULL;
    picohash_item* item;
    picoquic_path_t dummy_path_x = { 0 };

    picoquic_store_addr(&dummy_path_x.registered_peer_addr, addr);

    item = picohash_retrieve(quic->table_cnx_by_net, &dummy_path_x);

    if (item != NULL) {
        ret = ((picoquic_path_t*)item->key)->cnx;
    }
    return ret;
}
```

### Rust body
```rust
    pub fn connection_by_net(&mut self, addr: Option<&SocketAddr>) -> Option<ConnectionToken> {
        let addr = addr?;
        let ht = self.connection_by_net.lookup(addr)?;
        let &tok = self.connection_by_net.get(ht)?;
        Some(tok)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_congestion_algorithm`
C: `picoquic/quicctx.c:5311-5327 picoquic_get_congestion_algorithm`
Rust: `rs/fq/src/lib.rs:4568-4610 get_congestion_algorithm`

### C body
```c
{
    picoquic_congestion_algorithm_t const* alg = NULL;

    if (alg_name != NULL && picoquic_congestion_control_algorithms != NULL) {
        for (size_t i = 0; i < picoquic_nb_congestion_control_algorithms; i++) {
            if (strcmp(alg_name, picoquic_congestion_control_algorithms[i]->congestion_algorithm_id) == 0) {
                alg = picoquic_congestion_control_algorithms[i];
                break;
            }
        }
        if (alg == NULL && strcmp(alg_name, "reno") == 0) {
            alg = picoquic_get_congestion_algorithm("newreno");
        }
    }
    return alg;
}
```

### Rust body
```rust
impl Quic {
    /// Set the default congestion-control algorithm applied to new
    /// connections on this context.
    pub fn set_default_congestion_algorithm(&mut self, algo: &'static CongestionAlgorithm) {
        self.default_congestion_alg = Some(algo);
        self.default_congestion_alg_option_string = None;
    }

    /// Same as [`Self::set_default_congestion_algorithm`] but
    /// passes through a per-algorithm options string.
    pub fn set_default_congestion_algorithm_ex(
        &mut self,
        alg: &'static CongestionAlgorithm,
        alg_option_string: Option<&str>,
    ) {
        self.default_congestion_alg = Some(alg);
        self.default_congestion_alg_option_string = alg_option_string.map(|s| s.to_owned());
    }

    /// Convenience: select the default algorithm by name (looking up
    /// in the registry).  Returns `Err` when no registered algorithm
    /// matches `alg_name`.
    pub fn set_default_congestion_algorithm_by_name(
        &mut self,
        alg_name: &str,
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
}
```

## Pair `picoquic/quicctx.c:picoquic_set_optimistic_ack_policy`
C: `picoquic/quicctx.c:5354-5358 picoquic_set_optimistic_ack_policy`
Rust: `rs/fq/src/lib.rs:4434-4441 set_optimistic_ack_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->sequence_hole_pseudo_period = sequence_hole_pseudo_period;
}
```

### Rust body
```rust
    pub fn set_preemptive_repeat_policy(&mut self, do_repeat: bool) {
        self.is_preemptive_repeat_enabled = do_repeat;
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_congestion_algorithm`
C: `picoquic/quicctx.c:5395-5398 picoquic_set_congestion_algorithm`
Rust: `rs/fq/src/lib.rs:4615-4618 set_congestion_algorithm`

### C body
```c
{
    picoquic_set_congestion_algorithm_ex(cnx, alg, NULL);
}
```

### Rust body
```rust
    pub fn set_congestion_algorithm(&mut self, algo: &'static CongestionAlgorithm) {
        self.congestion_alg = Some(algo);
        self.congestion_alg_option_string = None;
    }
```

## Pair `picoquic/quicctx.c:picoquic_subscribe_pacing_rate_updates`
C: `picoquic/quicctx.c:5418-5424 picoquic_subscribe_pacing_rate_updates`
Rust: `rs/fq/src/lib.rs:4650-4663 subscribe_pacing_rate_updates`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->pacing_decrease_threshold = decrease_threshold;
    cnx->pacing_increase_threshold = increase_threshold;
    cnx->is_pacing_update_requested = (decrease_threshold != UINT64_MAX || increase_threshold != UINT64_MAX);
}
```

### Rust body
```rust
    pub fn pacing_rate(&self) -> u64 {
        self.paths.first().map(|p| p.pacing.rate).unwrap_or(0)
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_local_addr`
C: `picoquic/quicctx.c:5444-5457 picoquic_set_local_addr`
Rust: `rs/fq/src/lib.rs:2925-2940 set_local_addr`

### C body
```c
{
    int ret = 0;

    if (cnx != NULL && cnx->path[0] != NULL && cnx->path[0]->first_tuple->local_addr.ss_family == 0) {
        picoquic_store_addr(&cnx->path[0]->first_tuple->local_addr, addr);
        ret = (cnx->path[0]->first_tuple->local_addr.ss_family == 0) ? -1 : 0;
    }
    else {
        ret = -1;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn set_local_addr(&mut self, addr: &SocketAddr) -> Result<(), Error> {
        if let Some(path) = self.paths.first_mut()
            && let Some(tuple) = path.tuples.first_mut()
        {
            if !socket_addr_is_unspecified(&tuple.local_addr) {
                return Err(Error::Generic);
            }
            tuple.local_addr = *addr;
            return if socket_addr_is_unspecified(&tuple.local_addr) {
                Err(Error::Generic)
            } else {
                Ok(())
            };
        }
        Err(Error::InvalidArgument)
    }
```

## Pair `picoquic/quicctx.c:picoquic_is_client`
C: `picoquic/quicctx.c:5494-5498 picoquic_is_client`
Rust: `rs/fq/src/lib.rs:4467-4474 is_client`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->client_mode;
}
```

### Rust body
```rust
    pub fn local_error(&self) -> u64 {
        self.local_error
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_remote_stream_error`
C: `picoquic/quicctx.c:5520-5529 picoquic_get_remote_stream_error`
Rust: `rs/fq/src/lib.rs:4506-4522 remote_stream_error`

### C body
```c
{
    uint64_t remote_error = 0;
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    if ((stream = picoquic_find_stream(cnx, stream_id)) != NULL) {
        remote_error = stream->remote_error;
    }
    return remote_error;
}
```

### Rust body
```rust
    pub fn set_stream_remote_error(&mut self, stream_id: u64, error_code: u64) {
        if let Some(stream) = self.find_stream(stream_id)
            && let Some(stream) = self.streams.get_mut(stream)
        {
            stream.remote_error = error_code;
        }
    }
```
