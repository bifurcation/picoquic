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

## `picoquic/tls_api.c:picoquic_tls_stream_process`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C processes only the current TLS read epoch, handles protocol errors, ALPN, early data, send offsets per epoch, and state transitions; Rust drains all epochs into chunks, queues all outbound TLS bytes to epoch 0, and lacks many visible C behaviors.
* C source: `picoquic/tls_api.c:2551-2758`
* C signature: `int picoquic_tls_stream_process(picoquic_cnx_t *, int *, uint64_t)`
* Rust source: `rs/fq/src/tls_api.rs:1056-1115`
* Rust item: `process_tls_stream`

### C body
```c
{
    int ret = 0;
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;
    size_t next_epoch = 0;

    /* Provide indication of current connection for later callbacks */
    cnx->quic->cnx_in_progress = cnx;

    for (size_t epoch = 0; epoch < PICOQUIC_NUMBER_OF_EPOCHS && ret == 0; epoch++) {
        picoquic_stream_head_t* stream = &cnx->tls_stream[epoch];
        picoquic_stream_data_node_t* data = (picoquic_stream_data_node_t*)picosplay_first(&stream->stream_data_tree);
        size_t processed = 0;
        int data_pushed = 0;

        next_epoch = ptls_get_read_epoch(ctx->tls);

        if (epoch != next_epoch) {
            if (epoch > next_epoch) {
                break;
            } else {
                if (data != NULL && data->offset > stream->consumed_offset) {
                    /* Protocol error: data received that could not be read */
#ifdef _DEBUG
                    DBG_PRINTF("Connection error - TLS data at epoch %d, expected %d.\n",
                        epoch, next_epoch);
#endif
                    ret = picoquic_connection_error(cnx,
                        PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
                }
                continue;
            }
        }

        while ((ret == 0 || ret == PTLS_ERROR_IN_PROGRESS) &&
            data != NULL && data->offset <= stream->consumed_offset) {
            struct st_ptls_buffer_t sendbuf;
            size_t start = (size_t)(stream->consumed_offset - data->offset);
            size_t epoch_data = data->length - start;
            size_t send_offset[PICOQUIC_NUMBER_OF_EPOCH_OFFSETS] = { 0, 0, 0, 0, 0 };

            if (data_consumed != NULL) {
                *data_consumed = 1;
            }

            ptls_buffer_init(&sendbuf, "", 0);

            /* Clearing the global error state of the crypto provider before calling handle message.
             * This allows detection of errors during processing. */
            picoquic_clear_crypto_errors();

            ret = ptls_handle_message(ctx->tls, &sendbuf, send_offset, epoch,
                data->bytes + start, epoch_data, &ctx->handshake_properties);

            if ((ret == 0 || ret == PTLS_ERROR_IN_PROGRESS ||
                ret == PTLS_ERROR_STATELESS_RETRY)) {
                for (int i = 0; i < PICOQUIC_NUMBER_OF_EPOCHS; i++) {
                    if (send_offset[i] < send_offset[i + 1]) {
                        data_pushed = 1;
                        ret = picoquic_add_to_tls_stream(cnx,
                            sendbuf.base + send_offset[i], send_offset[i + 1] - send_offset[i], i);
                    }
                }
                if (cnx->client_mode) {
                    if (cnx->alpn == NULL) {
                        const char* alpn = ptls_get_negotiated_protocol(ctx->tls);

                        if (alpn != NULL){
                            cnx->alpn = picoquic_string_duplicate(alpn);

                            picoquic_log_negotiated_alpn(cnx, 0, NULL, 0, (const uint8_t*)alpn, strlen(alpn), NULL, 0);

                            if (cnx->callback_fn != NULL) {
                                cnx->callback_fn(cnx, 0, (uint8_t*)alpn, 0, picoquic_callback_set_alpn, cnx->callback_ctx, NULL);
                            }
                            else {
                                DBG_PRINTF("Negotiated ALPN: %s", alpn);
                            }
                        }
                    }
                    switch (ctx->handshake_properties.client.early_data_acceptance) {
                    case PTLS_EARLY_DATA_REJECTED:
                        cnx->zero_rtt_data_accepted = 0;
                        break;
                    case PTLS_EARLY_DATA_ACCEPTED:
                        cnx->zero_rtt_data_accepted = 1;
                        break;
                    default:
                        break;
                    }
                }
            }
            else {
                picoquic_log_crypto_errors(cnx, ret);
            }

            stream->consumed_offset += epoch_data;
            processed += epoch_data;

            if (start + epoch_data >= data->length) {
                picosplay_delete_hint(&cnx->tls_stream[epoch].stream_data_tree, &data->stream_data_node);
                data = (picoquic_stream_data_node_t*)picosplay_first(&cnx->tls_stream[epoch].stream_data_tree);
            }

            ptls_buffer_dispose(&sendbuf);
        }

        if (processed > 0) {
            if (ret == 0) {
                switch (cnx->cnx_state) {
                case picoquic_state_client_retry_received:
                    /* This is not supposed to happen -- HRR should generate "error in progress" */
                    break;
                case picoquic_state_client_init:
                case picoquic_state_client_init_sent:
                case picoquic_state_client_renegotiate:
                case picoquic_state_client_init_resent:
                case picoquic_state_client_handshake_start:
                    if (ptls_handshake_is_complete(ctx->tls)) {
                        if (cnx->remote_parameters_received == 0) {

#ifdef _DEBUG
                            DBG_PRINTF("%s", "Connection error - no transport parameter received.\n");
#endif
                            ret = picoquic_connection_error(cnx,
                                PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0);
                        }
                        else {
                            if (cnx->crypto_context[3].aead_encrypt != NULL) {
                                picoquic_client_almost_ready_transition(cnx);
                            }
                        }
                    }
                    break;
                case picoquic_state_server_init:
                case picoquic_state_server_handshake:
                    /* If client authentication is activated, the client sends the certificates with its `Finished` packet.
                       The server does not send any further packets, so, we can switch into false start state here.
                    */
                    if (data_pushed == 0 && ((ptls_context_t*)cnx->quic->tls_master_ctx)->require_client_authentication == 1) {
                        picoquic_false_start_transition(cnx, current_time);
                    }
                    else {
                        if (cnx->crypto_context[3].aead_encrypt != NULL) {
                            cnx->cnx_state = picoquic_state_server_almost_ready;
                        }
                    }
                    break;
                case picoquic_state_client_almost_ready:
                case picoquic_state_handshake_failure:
                case picoquic_state_handshake_failure_resend:
                case picoquic_state_client_ready_start:
                case picoquic_state_server_almost_ready:
                case picoquic_state_server_false_start:
                case picoquic_state_ready:
                case picoquic_state_disconnecting:
                case picoquic_state_closing_received:
                case picoquic_state_closing:
                case picoquic_state_draining:
                case picoquic_state_disconnected:
                    break;
                default:
                    DBG_PRINTF("Unexpected connection state: %d\n", cnx->cnx_state);
                    break;
                }
            }
            else if (ret == PTLS_ERROR_IN_PROGRESS && (cnx->cnx_state == picoquic_state_client_init || cnx->cnx_state == picoquic_state_client_init_sent || cnx->cnx_state == picoquic_state_client_init_resent)) {
                /* Extract and install the client 0-RTT key */
#ifdef _DEBUG
                DBG_PRINTF("%s", "Handshake not yet complete.\n");
#endif
            }
            else if (ret == PTLS_ERROR_IN_PROGRESS &&
                (cnx->cnx_state == picoquic_state_server_init ||
                    cnx->cnx_state == picoquic_state_server_handshake))
            {
                if (ptls_handshake_is_complete(ctx->tls))
                {
                    cnx->cnx_state = picoquic_state_server_almost_ready;
                }
            }

            if ((ret == 0 || ret == PTLS_ERROR_IN_PROGRESS || ret == PTLS_ERROR_STATELESS_RETRY)) {
                ret = 0;
            }
            else {
                uint16_t error_code = PICOQUIC_TRANSPORT_INTERNAL_ERROR;

                if (PTLS_ERROR_GET_CLASS(ret) == PTLS_ERROR_CLASS_SELF_ALERT) {
                    error_code = PICOQUIC_TRANSPORT_CRYPTO_ERROR(ret);
                }
#ifdef _DEBUG
                DBG_PRINTF("Handshake failed, ret = 0x%x.\n", ret);
#endif
                (void)picoquic_connection_error(cnx, error_code, 0);
                /* Log the version numbers of SSL/TLS packages to facilitate debugging. */
                picoquic_tls_api_log_versions(cnx);
                ret = 0;
            }
        }
    }

    /* Reset indication of current connection */
    cnx->quic->cnx_in_progress = NULL;


    return ret;
}
```

### Rust body
```rust
    pub fn process_tls_stream(&mut self, current_time: Instant) -> Result<usize, Error> {
        let mut session = self.tls_ctx.take().ok_or(Error::InvalidState)?;
        let mut chunks = Vec::new();
        let mut consumed = 0usize;

        for epoch in 0..NUMBER_OF_EPOCHS {
            let stream = &mut self.tls_stream[epoch];
            while let Some(node_token) = stream.stream_data_tree.first() {
                let data_token = match stream.stream_data_tree.get(node_token).copied() {
                    Some(token) => token,
                    None => {
                        stream.stream_data_tree.remove(node_token);
                        continue;
                    }
                };
                let Some(node) = stream.stream_data_nodes.get(data_token) else {
                    stream.stream_data_tree.remove(node_token);
                    continue;
                };
                if node.offset > stream.consumed_offset {
                    break;
                }
                let start = (stream.consumed_offset - node.offset) as usize;
                if start >= node.length {
                    stream.stream_data_tree.remove(node_token);
                    stream.stream_data_nodes.remove(data_token);
                    continue;
                }
                let data = node.data[start..node.length].to_vec();
                stream.consumed_offset += data.len() as u64;
                consumed += data.len();
                chunks.push(data);
                stream.stream_data_tree.remove(node_token);
                stream.stream_data_nodes.remove(data_token);
            }
        }

        for chunk in &chunks {
            if session.read_handshake(chunk)?
                && let Some(keys) = session.write_handshake(&mut self.tls_sendbuf)
            {
                install_key_pair(&mut self.crypto_context[3], keys);
            }
        }

        if let Some(keys) = session.write_handshake(&mut self.tls_sendbuf) {
            install_key_pair(&mut self.crypto_context[3], keys);
        }

        if !self.tls_sendbuf.is_empty() {
            let out = core::mem::take(&mut self.tls_sendbuf);
            queue_tls_bytes(self, 0, &out)?;
        }

        if !session.is_handshaking() {
            self.ready_state_transition(current_time);
        }
        self.tls_ctx = Some(session);
        Ok(consumed)
    }
```

## `picoquic/transport.c:picoquic_transport_param_type_flag_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C encodes the parameter type and zero length; Rust body only returns None.
* C source: `picoquic/transport.c:68-75`
* C signature: `uint8_t * picoquic_transport_param_type_flag_encode(uint8_t *, const uint8_t *, picoquic_tp_enum)`
* Rust source: `rs/fq/src/internal.rs:15055-15064`
* Rust item: `picoquic_transport_param_type_flag_encode`

### C body
```c
{
    if (bytes != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, tp_type)) != NULL) {
        bytes = picoquic_frames_varint_encode(bytes, bytes_max, 0);
    }
    return bytes;
}
```

### Rust body
```rust
    {
        return None;
    }
```

## `picoquic/util.c:get_debug_suspended`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns debug_suspended; Rust body is debug_printf, writes a message when not suspended, and returns nothing.
* C source: `picoquic/util.c:116-119`
* C signature: `int get_debug_suspended(void)`
* Rust source: `rs/fq/src/utils.rs:141-159`
* Rust item: `get_debug_suspended`

### C body
```c
{
    return debug_suspended;
}
```

### Rust body
```rust
pub fn debug_printf(msg: &str) {
    if DEBUG_SUSPENDED.with(|s| s.get()) {
        return;
    }
    DEBUG_OUT.with(|o| {
        if let Some(w) = o.borrow_mut().as_mut() {
            let _ = w.write_str(msg);
        }
    });
}
```

## `picoquic/util.c:picoquic_frames_varint_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C contains full variable-length integer encoding, while the Rust body shown only checks for an empty buffer and appears incomplete.
* C source: `picoquic/util.c:932-983`
* C signature: `uint8_t * picoquic_frames_varint_encode(uint8_t *, const uint8_t *, uint64_t)`
* Rust source: `rs/fq/src/utils.rs:813-817`
* Rust item: `frames_varint_encode`

### C body
```c
{
    if (n64 < 16384) {
        if (n64 < 64) {
            if (bytes + 1 <= bytes_max) {
                *bytes++ = (uint8_t)(n64);
            }
            else {
                bytes = NULL;
            }
        }
        else {
            if (bytes + 2 <= bytes_max) {
                *bytes++ = (uint8_t)((n64 >> 8) | 0x40);
                *bytes++ = (uint8_t)(n64);
            }
            else {
                bytes = NULL;
            }
        }
    }
    else if (n64 < 1073741824) {
        if (bytes + 4 <= bytes_max) {
            *bytes++ = (uint8_t)((n64 >> 24) | 0x80);
            *bytes++ = (uint8_t)(n64 >> 16);
            *bytes++ = (uint8_t)(n64 >> 8);
            *bytes++ = (uint8_t)(n64);
        }
        else {
            bytes = NULL;
        }
    }
    else {
        if (bytes + 8 <= bytes_max) {
            *bytes++ = (uint8_t)((n64 >> 56) | 0xC0);
            *bytes++ = (uint8_t)(n64 >> 48);
            *bytes++ = (uint8_t)(n64 >> 40);
            *bytes++ = (uint8_t)(n64 >> 32);
            *bytes++ = (uint8_t)(n64 >> 24);
            *bytes++ = (uint8_t)(n64 >> 16);
            *bytes++ = (uint8_t)(n64 >> 8);
            *bytes++ = (uint8_t)(n64);
        }
        else {
            bytes = NULL;
        }
    }

    return bytes;
}
```

### Rust body
```rust
        if bytes.is_empty() {
            return None;
        }
```

## `picoquic/util.c:picoquic_string_create`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C allocates/copies up to len and null-terminates; Rust body only handles None by returning an empty String and shows no copy of Some(original).
* C source: `picoquic/util.c:45-71`
* C signature: `char * picoquic_string_create(const char *, size_t)`
* Rust source: `rs/fq/src/utils.rs:252-255`
* Rust item: `string_create`

### C body
```c
{
    size_t allocated = len + 1;
    char * str = NULL;

    /* tests to protect against integer overflow */
    if (allocated > 0) {
        str = (char*)malloc(allocated);

        if (str != NULL) {
            if (original == NULL || len == 0) {
                str[0] = 0;
            }
            else if (allocated > len) {
                memcpy(str, original, len);
                str[allocated - 1] = 0;
            }
            else {
                /* This could happen only in case of integer overflow */
                free(str);
                str = NULL;
            }
        }
    }

    return str;
}
```

### Rust body
```rust
    let Some(src) = original else {
        return String::new();
    };
```
