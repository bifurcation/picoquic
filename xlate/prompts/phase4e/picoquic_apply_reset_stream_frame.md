# Phase 4E repair confirmed translation mismatches

You are repairing Phase 4D `needs_fix` entries.  Phase 4D
already performed deeper classification and concluded that
these Rust translations need repair.

Rules:

* Edit Rust only.  Do not edit C sources.
* Keep edits limited to the owned Rust file(s) for this batch
  unless a directly related helper in `rs/fq/` must change.
* Preserve safe, idiomatic Rust and existing public API shape
  unless the current shape cannot express the C behavior.
* Do not replace code with stubs, placeholders, fabricated
  defaults, or weaker behavior.
* If deeper repair inspection proves Phase 4D was mistaken,
  report outcome `ok` and do not edit source.
* The driver will run a separate read-only re-triage before
  recording any `fixed` or `ok` result as resolved.
* Report `blocked` only with a concrete human-actionable
  reason.

Owned Rust file(s): `rs/fq/src/internal.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/frames.c:picoquic_apply_reset_stream_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust returns immediately for invalid local unidirectional reset, while C signals the error but continues through find/create and other reset handling before returning NULL.
* Phase 4D analysis: The concern is real. C records the invalid local-unidirectional RESET_STREAM error but still falls through into find/create and reset handling, which can cause additional observable connection-error or stream-state side effects. Rust returns immediately.
* Phase 4D fix note: Preserve C fall-through: after signaling the invalid local-unidirectional reset, keep the eventual return as None but continue through find/create and reset validation/handling.
* C source: `picoquic/frames.c:330-371`
* C signature: `const uint8_t * picoquic_apply_reset_stream_frame(picoquic_cnx_t *, const uint8_t *, uint64_t, uint64_t, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:10257-10332`
* Rust item: `picoquic_apply_reset_stream_frame`

### C body
```c
{
    picoquic_stream_head_t* stream;
    
    if (!IS_BIDIR_STREAM_ID(stream_id) && IS_LOCAL_STREAM_ID(stream_id, cnx->client_mode)) {
        /* the peer cannot send data, and thus cannot reset the stream */
        bytes = NULL;
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_STREAM_STATE_ERROR,
            picoquic_frame_type_reset_stream);
    }
    if ((stream = picoquic_find_or_create_stream(cnx, stream_id, 1)) == NULL) {
        /* Not finding the stream is only an error if the stream
         * was expected to be present, or created on demand. If the
         * stream was already created and then deleted, there is no harm.
         * If the "return NULL" is in a normal scenario, the connection state
         * will remain "ready" or "almost ready"
         */
        if (cnx->cnx_state > picoquic_state_ready) {
            bytes = NULL;  /* error already signaled */
        }
    }
    else if ((stream->fin_received || stream->reset_received) && final_offset != stream->fin_offset) {
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_FINAL_OFFSET_ERROR,
            picoquic_frame_type_reset_stream);
        bytes = NULL;

    }
    else if (picoquic_flow_control_check_stream_offset(cnx, stream, final_offset) != 0) {
        bytes = NULL;  // error already signaled
    }
    else if (!stream->reset_received) {
        stream->reset_received = 1;
        stream->reset_offset = reliable_size;
        stream->remote_error = error_code_64;

        if (stream->consumed_offset >= stream->reset_offset) {
            picoquic_signal_stream_reset(cnx, stream);
        }
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    use crate::stream::{Role, StreamId};

    let local_role = if connection.client_mode {
        Role::Client
    } else {
        Role::Server
    };
    let sid = StreamId(stream_id);
    let mut frame_result = Some(bytes);
    if !sid.is_bidir() && sid.is_local(local_role) {
        connection.connection_error(
            crate::errors::TransportError::StreamStateError as u64,
            crate::frames::FrameType::ResetStream as u64,
        );
        frame_result = None;
    }

    let stream_token = match connection.find_stream(stream_id) {
        Some(token) => token,
        None => match connection.create_missing_streams(stream_id, true) {
            Ok(token) => token,
            Err(_) => {
                if connection.connection_state > State::Ready {
                    return None;
                }
                return frame_result;
            }
        },
    };

    let final_offset_mismatch = connection
        .streams
        .get(stream_token)
        .map(|stream| {
            (stream.fin_received || stream.reset_received) && final_offset != stream.fin_offset
        })
        .unwrap_or(false);
    if final_offset_mismatch {
        connection.connection_error(
            crate::errors::TransportError::FinalOffsetError as u64,
            crate::frames::FrameType::ResetStream as u64,
        );
        return None;
    }

    if flow_control_check_stream_offset_token(connection, stream_token, final_offset) != 0 {
        return None;
    }

    let should_signal = if let Some(stream) = connection.streams.get_mut(stream_token) {
        if !stream.reset_received {
            stream.reset_received = true;
            stream.reset_offset = reliable_size;
            stream.remote_error = error_code_64;
            stream.consumed_offset >= stream.reset_offset
        } else {
            false
        }
    } else {
        false
    };

    if should_signal {
        signal_stream_reset_token(connection, stream_token);
    }

    frame_result
}
```
