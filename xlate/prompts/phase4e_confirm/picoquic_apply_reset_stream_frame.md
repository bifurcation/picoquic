# Phase 4E repair confirmation

This is a read-only re-triage after a Phase 4E repair or
repair-level `ok` claim.  Do not edit files.

For each entry, inspect directly relevant C and Rust context
and decide whether the current Rust translation is now
acceptable.

Report:

* `ok` when the current Rust behavior is acceptable.
* `needs_fix` when a real mismatch remains.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short confirmation conclusion","fix_summary":"remaining mismatch if any, or empty","files_changed":[],"verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/frames.c:picoquic_apply_reset_stream_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust returns immediately for invalid local unidirectional reset, while C signals the error but continues through find/create and other reset handling before returning NULL.
* Prior Phase 4D analysis: The concern is real. C records the invalid local-unidirectional RESET_STREAM error but still falls through into find/create and reset handling, which can cause additional observable connection-error or stream-state side effects. Rust returns immediately.
* Phase 4E claimed outcome: `ok`
* Phase 4E repair analysis: Current Rust already preserves the C fall-through behavior: invalid local unidirectional RESET_STREAM records a connection error and sets the eventual return to None, but still runs stream lookup/create, final-offset validation, flow-control check, and reset handling.
* Phase 4E fix summary: 
* C source: `picoquic/frames.c:330-371`
* C signature: `const uint8_t * picoquic_apply_reset_stream_frame(picoquic_cnx_t *, const uint8_t *, uint64_t, uint64_t, uint64_t, uint64_t)`
* Current Rust source: `rs/fq/src/internal.rs:10257-10332`
* Current Rust item: `picoquic_apply_reset_stream_frame`

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

### Current Rust body
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
