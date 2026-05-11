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

## `picoquic/ech.c:picoquic_ech_save_config`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body base64-encodes into an allocated text buffer, writes it plus newline to a file, and frees it; Rust only delegates to another function whose body is not shown.
* C source: `picoquic/ech.c:127-169`
* C signature: `int picoquic_ech_save_config(uint8_t *, size_t, const char *)`
* Rust source: `rs/fq/src/lib.rs:4755-4757`
* Rust item: `ech_save_config`

### C body
```c
{
    int ret = 0;
    int last_err;
    size_t ech_text_size = 0;
    size_t ech_text_len = 0;
    char* ech_text = NULL;

    (void)picoquic_base64_encode(config, config_len, ech_text, ech_text_size, &ech_text_len);
    ech_text_size = ech_text_len + 1;
    ech_text = (char*)malloc(ech_text_size);

    if (ech_text == NULL) {
        DBG_PRINTF("Cannot allocate %d bytes for text buffer", ech_text_size);
        ret = -1;
    }
    else {
        ret = picoquic_base64_encode(config, config_len, ech_text, ech_text_size, &ech_text_len);
    }
    if (ret == 0) {
        FILE* F = picoquic_file_open_ex(file_name, "w", &last_err);
        if (F == NULL) {
            DBG_PRINTF("Cannot open file <%s>, err: %x", file_name, last_err);
            ret = -1;
        }
        else {
            size_t written = fwrite(ech_text, 1, ech_text_size, F);
            if (written != ech_text_size) {
                DBG_PRINTF("Cannot write %d bytes on file <%s>, err= %zu", ech_text, file_name, written);
                ret = -1;
            }
            else {
                (void)fprintf(F, "\n");
                DBG_PRINTF("Wrote %d bytes on file <%s>", ech_text_size + 1, file_name);
            }
            picoquic_file_close(F);
        }
    }
    if (ech_text != NULL) {
        free(ech_text);
    }
    return ret;
}
```

### Rust body
```rust
pub fn ech_save_config(config: &[u8], ech_config_file: &str) -> Result<(), Error> {
    crate::ech::ech_save_config_file(config, ech_config_file)
}
```

## `picoquic/frames.c:picoquic_estimate_path_bandwidth`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust assigns path_x.bandwidth_estimate = bw_estimate before checking bw_estimate > path_x.bandwidth_estimate, making that comparison visibly false after assignment; C has the same visible ordering, but the guarded block is therefore suspicious in both bodies.
* C source: `picoquic/frames.c:2865-2922`
* C signature: `void picoquic_estimate_path_bandwidth(picoquic_cnx_t *, picoquic_path_t *, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, int)`
* Rust source: `rs/fq/src/internal.rs:1104-1171`
* Rust item: `picoquic_estimate_path_bandwidth`

### C body
```c
{
    if (send_time >= path_x->delivered_sent_last) {
        if (path_x->delivered_time_last == 0) {
            /* No estimate yet, need to initialize the variables */
            path_x->delivered_last = path_x->delivered;
            path_x->delivered_time_last = delivery_time;
            path_x->delivered_sent_last = send_time;
        }
        else {
            uint64_t receive_interval = delivery_time - delivered_time_prior;

            if (receive_interval > PICOQUIC_BANDWIDTH_TIME_INTERVAL_MIN) {
                uint64_t delivered = path_x->delivered - delivered_prior;
                uint64_t send_interval = send_time - delivered_sent_prior;
                uint64_t bw_estimate;

                if (send_interval > receive_interval) {
                    receive_interval = send_interval;
                }

                bw_estimate = PICOQUIC_RATE_FROM_BYTES(delivered, receive_interval);

                path_x->bandwidth_estimate = bw_estimate;
                if (!rs_is_path_limited || bw_estimate > path_x->bandwidth_estimate) {
                    if (path_x == cnx->path[0]){
                        if (cnx->is_ack_frequency_negotiated) {
                            /* Compute the desired value of the ack frequency*/
                            uint64_t ack_gap;
                            uint64_t ack_delay_max;
                            picoquic_compute_ack_gap_and_delay(cnx, cnx->path[0]->rtt_min, cnx->remote_parameters.min_ack_delay,
                                bw_estimate, &ack_gap, &ack_delay_max);
                            if (ack_gap != cnx->ack_gap_local) {
                                cnx->is_ack_frequency_updated = 1;
                            }
                        }
                    }
                }

                /* Bandwidth was estimated, update the references */
                path_x->delivered_last = path_x->delivered;
                path_x->delivered_time_last = delivery_time;
                path_x->delivered_sent_last = send_time;
                path_x->delivered_last_packet = delivered_prior;
                path_x->last_bw_estimate_path_limited = rs_is_path_limited;
                if (path_x->delivered_last_packet > path_x->delivered_limited_index) {
                    path_x->delivered_limited_index = 0;
                }
                /* Statistics */
                if (bw_estimate > path_x->bandwidth_estimate_max) {
                    path_x->bandwidth_estimate_max = bw_estimate;
                }
            }
        }
    }
}
```

### Rust body
```rust
) {
    if send_time < path_x.delivered_sent_last {
        return;
    }

    if path_x.delivered_time_last.ticks() == 0 {
        path_x.delivered_last = path_x.delivered;
        path_x.delivered_time_last = Instant::from_ticks(delivery_time);
        path_x.delivered_sent_last = send_time;
        return;
    }

    let mut receive_interval = delivery_time.saturating_sub(delivered_time_prior);
    if receive_interval <= BANDWIDTH_TIME_INTERVAL_MIN {
        return;
    }

    let delivered = path_x.delivered.saturating_sub(delivered_prior);
    let send_interval = send_time.saturating_sub(delivered_sent_prior);
    if send_interval > receive_interval {
        receive_interval = send_interval;
    }
    let bw_estimate = crate::utils::rate_from_bytes(delivered, receive_interval);

    path_x.bandwidth_estimate = bw_estimate;
    if !rs_is_path_limited || bw_estimate > path_x.bandwidth_estimate {
        let is_first_path = connection
            .paths
            .first()
            .map(|p| p.unique_path_id == path_x.unique_path_id)
            .unwrap_or(false);
        if is_first_path && connection.is_ack_frequency_negotiated {
            let mut ack_gap = 0;
            let mut ack_delay_max = 0;
            connection.compute_ack_gap_and_delay(
                path_x.rtt_min,
                connection.remote_parameters.min_ack_delay.ticks(),
                bw_estimate,
                &mut ack_gap,
                &mut ack_delay_max,
            );
            if ack_gap != connection.ack_gap_local {
                connection.is_ack_frequency_updated = true;
            }
        }
    }

    path_x.delivered_last = path_x.delivered;
    path_x.delivered_time_last = Instant::from_ticks(delivery_time);
    path_x.delivered_sent_last = send_time;
    path_x.delivered_last_packet = delivered_prior;
    path_x.last_bw_estimate_path_limited = rs_is_path_limited;
    if path_x.delivered_last_packet > path_x.delivered_limited_index {
        path_x.delivered_limited_index = 0;
    }
    if bw_estimate > path_x.bandwidth_estimate_max {
        path_x.bandwidth_estimate_max = bw_estimate;
    }
}
```

## `picoquic/frames.c:picoquic_format_max_data_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C uses direct addition for maxdata_local plus increase, while Rust uses saturating_add, which can differ on overflow.
* C source: `picoquic/frames.c:4485-4501`
* C signature: `uint8_t * picoquic_format_max_data_frame(picoquic_cnx_t *, uint8_t *, uint8_t *, int *, int *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:12838-12859`
* Rust item: `format_max_data_frame`

### C body
```c
{
    uint8_t * bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_max_data)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, cnx->maxdata_local + maxdata_increase)) != NULL) {
        cnx->maxdata_local = (cnx->maxdata_local + maxdata_increase);
        *is_pure_ack = 0;
    }
    else {
        *more_data = 1;
        bytes = bytes0;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let new_max = connection.maxdata_local.saturating_add(maxdata_increase);
    if bytes.is_empty() {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[0] = crate::frames::FrameType::MaxData as u8;
    let mut off = 1;
    if !encode_varint_at(bytes, &mut off, new_max) {
        *more_data = 1;
        return Some(bytes);
    }
    connection.maxdata_local = new_max;
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## `picoquic/frames.c:picoquic_queue_handshake_done_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C queues the handshake-done byte through the datagram queue pointers, while Rust calls encode_misc_frame without any body-visible datagram-queue target.
* C source: `picoquic/frames.c:5191-5197`
* C signature: `int picoquic_queue_handshake_done_frame(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:13878-13886`
* Rust item: `queue_handshake_done_frame`

### C body
```c
{
    uint8_t frame_buffer = picoquic_frame_type_handshake_done;

    return picoquic_queue_misc_or_dg_frame(cnx, &cnx->first_datagram, &cnx->last_datagram,
            &frame_buffer, 1, 0, picoquic_packet_context_application);
}
```

### Rust body
```rust
    pub fn queue_handshake_done_frame(&mut self) -> Result<(), crate::Error> {
        encode_misc_frame(
            self,
            vec![crate::frames::FrameType::HandshakeDone as u8],
            false,
            PacketContext::Application,
        );
        Ok(())
    }
```

## `picoquic/logger.c:textlog_transport_extension`
* Phase 4C status: `suspect`
* Phase 4C rationale: C requires both a non-null log file and still-logging check before logging; Rust only checks for a logger_ref and then logs.
* C source: `picoquic/logger.c:2346-2353`
* C signature: `void textlog_transport_extension(picoquic_cnx_t *, int, size_t, uint8_t *)`
* Rust source: `rs/fq/src/logger.rs:776-780`
* Rust item: `transport_extension`

### C body
```c
{
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        /* TODO: alpn */
        picoquic_textlog_transport_extension(cnx->quic->F_log, cnx, (is_local)?0:1, 1, params, param_length);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .transport_extension(self, is_local, params);
        }
```
