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

## `picoquic/fastcc.c:picoquic_fastcc_delete`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C deletes congestion_alg_state; Rust body shown is alg_observe and only reads state.
* C source: `picoquic/fastcc.c:308-315`
* C signature: `void picoquic_fastcc_delete(picoquic_path_t *)`
* Rust source: `rs/fq/src/fastcc.rs:355-365`
* Rust item: `alg_delete`

### C body
```c
{
    if (path_x->congestion_alg_state != NULL) {
        free(path_x->congestion_alg_state);
        path_x->congestion_alg_state = NULL;
    }
}
```

### Rust body
```rust
    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|s| s.downcast_ref::<FastccState>())
            .map(|state| (state.alg_state as u64, state.rolling_rtt_min))
    }
```

## `picoquic/frames.c:picoquic_create_missing_streams`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only creates streams in a loop; it omits the C direction checks, limit errors, old-stream returns, and unidirectional fin marking.
* C source: `picoquic/frames.c:57-96`
* C signature: `picoquic_stream_head_t * picoquic_create_missing_streams(picoquic_cnx_t *, uint64_t, int)`
* Rust source: `rs/fq/src/internal.rs:9489-9505`
* Rust item: `create_missing_streams`

### C body
```c
{
    /* Verify the stream ID control conditions */
    picoquic_stream_head_t* stream = NULL;
    unsigned int expect_client_stream = cnx->client_mode ^ is_remote;

    if (is_remote && stream_id < cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]) {
        return NULL;
    } else if (IS_CLIENT_STREAM_ID(stream_id) != expect_client_stream){
        /* TODO: not an error if lower than next stream, would be just an old stream. */
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_STREAM_LIMIT_ERROR, 0);
    }
    else if (is_remote && stream_id > (IS_BIDIR_STREAM_ID(stream_id) ? cnx->max_stream_id_bidir_local : cnx->max_stream_id_unidir_local)){
        /* Protocol error, stream ID too high */
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_STREAM_LIMIT_ERROR, 0);
    } 
    else if (stream_id < cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]) {
        /* Stream already closed locally; do not escalate to transport error here. */
        return NULL;
    } else {
        while (stream_id >= cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]) {
            stream = picoquic_create_stream(cnx, cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]);
            if (stream == NULL) {
                picoquic_log_app_message(cnx, "Create stream %" PRIu64 " returns error 0x%x",
                    stream_id, PICOQUIC_TRANSPORT_INTERNAL_ERROR);
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0);
                break;
            }
            else if (!IS_BIDIR_STREAM_ID(stream_id)) {
                if (!IS_LOCAL_STREAM_ID(stream_id, cnx->client_mode)) {
                    /* Mark the stream as already finished in our direction */
                    stream->fin_requested = 1;
                    stream->fin_sent = 1;
                }
            }
        }
    }

    return stream;
}
```

### Rust body
```rust
    ) -> Result<StreamToken, crate::Error> {
        use crate::stream::StreamId;
        let type_idx = (stream_id & 3) as usize;
        let mut first_new_id = self.next_stream_id[type_idx];
        let mut last_tok: Option<StreamToken> = None;
        // Walk forward creating streams until we reach stream_id.
        while first_new_id <= stream_id {
            let tok = self.create_stream(first_new_id)?;
            last_tok = Some(tok);
            first_new_id = StreamId(first_new_id).next_with_same_kind().0;
        }
        last_tok.ok_or(crate::Error::Memory)
    }
```

## `picoquic/frames.c:picoquic_find_ready_stream_path`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a simple first-match filter and omits C priority scheduling, stop/reset handling, flow-control checks, cleanup, and blocked-flag side effects.
* C source: `picoquic/frames.c:1562-1664`
* C signature: `picoquic_stream_head_t * picoquic_find_ready_stream_path(picoquic_cnx_t *, picoquic_path_t *, int)`
* Rust source: `rs/fq/src/internal.rs:9673-9697`
* Rust item: `find_ready_stream_path`

### C body
```c
{
    picoquic_stream_head_t* first_stream = cnx->first_output_stream;
    picoquic_stream_head_t* stream = first_stream;
    picoquic_stream_head_t* found_stream = NULL;


    /* Look for a ready stream */
    while (stream != NULL) {
        int has_data = 0;
        picoquic_stream_head_t* next_stream = stream->next_output_stream;

        if (next_stream != NULL && is_coalesced && next_stream->is_not_coalesced) {
            stream = next_stream->next_output_stream;
            continue;
        }

        if (found_stream != NULL && stream->stream_priority > found_stream->stream_priority) {
            /* All the streams at that priority level have been examined,
             * the current selection is validated */
            break;
        }

        /* The tests for "have data" should excatly replicate the tests in
         * the formating of a stream frame */
        if (stream->stop_sending_requested && !stream->stop_sending_sent) {
            /* will send a stop sending frame.
            * this takes precedence over FIFO vs round-robin processing */
            found_stream = stream;
            has_data = 1;
            break;
        }
        else if (stream->reset_sent) {
            /* No data will be sent after a reset */
            has_data = 0;
        }
        else if (stream->reset_requested && 
            (stream->reliable_size == 0 || picoquic_check_sack_list(&stream->sack_list, 0, stream->reliable_size))) {
            /* will queue a reset frame --
            * this takes precedence over FIFO vs round-robin processing */
            found_stream = stream;
            has_data = 1;
            break;
        }
        else if (cnx->maxdata_remote > cnx->data_sent && stream->sent_offset < stream->maxdata_remote && (stream->is_active ||
            (stream->send_queue != NULL && stream->send_queue->length > stream->send_queue->offset) ||
            (stream->fin_requested && !stream->fin_sent))) {
            has_data = 1;
        }
        else {
            has_data = 0;
        }

        /* implement affinity scheduling */
        if (has_data && path_x != NULL && stream->affinity_path != path_x && stream->affinity_path != NULL) {
            /* Only consider the streams that meet path affinity requirements */
            has_data = 0;
        }
        
        if (has_data) {
            /* Check that this stream is actually available for sending data */
            if (stream->sent_offset == 0) {
                if (IS_CLIENT_STREAM_ID(stream->stream_id) == cnx->client_mode) {
                    if (stream->stream_id > ((IS_BIDIR_STREAM_ID(stream->stream_id)) ? cnx->max_stream_id_bidir_remote : cnx->max_stream_id_unidir_remote)) {
                        has_data = 0;
                    }
                }
            }
            if (has_data) {
                /* Something can be sent */
                if ((stream->stream_priority & 1) != 0) {
                    /* This priority level requests FIFO processing, so we return the first available stream */
                    found_stream = stream;
                    break;
                }
                else if (found_stream == NULL || stream->last_time_data_sent < found_stream->last_time_data_sent) {
                    /* Select this stream, but need to check if another stream should go before in round robin order */
                    found_stream = stream;
                }
            }
        }
        else if (((stream->fin_requested && stream->fin_sent) || (stream->reset_requested && stream->reset_sent)) && (!stream->stop_sending_requested || stream->stop_sending_sent)) {
            /* If stream is exhausted, remove from output list */
            picoquic_remove_output_stream(cnx, stream);

            picoquic_delete_stream_if_closed(cnx, stream);
        }
        else {
            if (stream->is_active ||
                (stream->send_queue != NULL && stream->send_queue->length > stream->send_queue->offset)) {
                if (stream->sent_offset >= stream->maxdata_remote) {
                    cnx->stream_blocked = 1;
                }
                else if (cnx->maxdata_remote <= cnx->data_sent) {
                    cnx->flow_blocked = 1;
                }
            }
        }
        stream = next_stream;
    }

    return found_stream;
}
```

### Rust body
```rust
        self.output_streams.iter().copied().find(|tok| {
            self.streams.get(*tok).is_some_and(|stream| {
                let path_ok = stream
                    .affinity_path
                    .map(|affinity| {
                        affinity
                            == PathToken::synthetic(
                                path_x.unique_path_id as u32,
                                path_x.unique_path_id as u32,
                            )
                    })
                    .unwrap_or(true);
                let coalescing_ok = is_coalesced || !stream.is_not_coalesced;
                path_ok
                    && coalescing_ok
                    && (!stream.send_queue.is_empty()
                        || (stream.fin_requested && !stream.fin_sent)
                        || stream.is_active)
            })
        })
```

## `picoquic/frames.c:picoquic_format_required_max_stream_data_frames`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust iterates max_stream_updated streams and emits current maxdata_local, while C scans all streams using fin/app/reset/consumed-offset conditions, increases the window, and clears max_stream_data_needed.
* C source: `picoquic/frames.c:4631-4658`
* C signature: `uint8_t * picoquic_format_required_max_stream_data_frames(picoquic_cnx_t *, uint8_t *, uint8_t *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:12795-12836`
* Rust item: `format_required_max_stream_data_frames`

### C body
```c
{
    uint8_t* bytes0;
    picoquic_stream_head_t* stream = picoquic_first_stream(cnx);

    while (stream != NULL) {
        if (!stream->fin_received && !stream->use_app_flow_control) {
            uint64_t new_window = picoquic_cc_increased_window(cnx, stream->maxdata_local);

            if (!stream->reset_received && 2 * stream->consumed_offset > stream->maxdata_local) {
                bytes0 = bytes;

                if ((bytes = picoquic_format_max_stream_data_frame(cnx, stream, bytes, bytes_max, more_data, is_pure_ack, stream->maxdata_local + new_window)) == bytes0) {
                    /* not enough space for this frame. */
                    break;
                }
            }
        }
        stream = picoquic_next_stream(stream);
    }

    if (stream == NULL) {
        cnx->max_stream_data_needed = 0;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let tokens: Vec<_> = connection
        .streams
        .iter()
        .filter_map(|stream| {
            stream
                .max_stream_updated
                .then_some(stream.stream_tree_membership)
                .flatten()
                .and_then(|st| connection.stream_tree.get(st).copied())
        })
        .collect();
    for tok in tokens {
        let Some(stream) = connection.streams.get_mut(tok) else {
            continue;
        };
        if !stream.max_stream_updated {
            continue;
        }
        let mut off = 0;
        if !encode_varint_at(
            bytes,
            &mut off,
            crate::frames::FrameType::MaxStreamData as u64,
        ) || !encode_varint_at(bytes, &mut off, stream.stream_id)
            || !encode_varint_at(bytes, &mut off, stream.maxdata_local)
        {
            *more_data = 1;
            return Some(bytes);
        }
        stream.maxdata_local_acked = stream.maxdata_local;
        stream.max_stream_updated = false;
        *is_pure_ack = 0;
        bytes = &mut bytes[off..];
    }
    Some(bytes)
}
```

## `picoquic/frames.c:picoquic_provide_datagram_buffer`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C delegates to provide_datagram_buffer_ex with datagram_not_active, while Rust delegates to provide_stream_data_buffer with stream-style boolean arguments.
* C source: `picoquic/frames.c:5458-5461`
* C signature: `uint8_t * picoquic_provide_datagram_buffer(void *, size_t)`
* Rust source: `rs/fq/src/lib.rs:4411-4416`
* Rust item: `provide_datagram_buffer`

### C body
```c
{
    return picoquic_provide_datagram_buffer_ex(context, length, picoquic_datagram_not_active);
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    provide_stream_data_buffer(context, length, false, false)
}
```
