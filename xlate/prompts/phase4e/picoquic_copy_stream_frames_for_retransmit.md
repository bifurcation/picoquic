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

## `picoquic/frames.c:picoquic_copy_stream_frames_for_retransmit`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C loops over queued packets by priority, calls a single-frame retransmit helper, handles dequeue/error state, and updates more_data; Rust copies raw bytes from only the first packet and ignores current_priority.
* Phase 4D analysis: Confirmed mismatch: Rust ignores current_priority, copies raw bytes from one packet, does not call the translated single-frame retransmit helper, and does not dequeue/requeue/error-handle like C.
* Phase 4D fix note: Rewrite the wrapper to loop over first_data_repeat_packet by priority, call picoquic_copy_single_stream_frame_for_retransmit, preserve more_data, dequeue, and corrupted-frame fallback semantics.
* C source: `picoquic/frames.c:2462-2501`
* C signature: `uint8_t * picoquic_copy_stream_frames_for_retransmit(picoquic_cnx_t *, uint8_t *, uint8_t *, uint64_t, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:11279-11302`
* Rust item: `copy_stream_frames_for_retransmit`

### C body
```c
{
    int more_retransmit = 0;
    int packet_dequeued = 0;
    uint8_t* bytes_first = bytes_next;
    picoquic_packet_t* packet = NULL;
    do {
        packet_dequeued = 0;
        packet = picoquic_first_data_repeat_packet(cnx);
        if (packet == NULL) {
            break;
        } else if (packet->data_repeat_priority > current_priority) {
            more_retransmit = 1;
            break;
        }
        else {
            more_retransmit = 0;
            bytes_next = picoquic_copy_single_stream_frame_for_retransmit(cnx, packet, 
                bytes_next, bytes_max, &more_retransmit, &packet_dequeued, is_pure_ack);
        }
    } while (bytes_next != NULL && packet_dequeued /* bytes_first < bytes_next */ && bytes_next < bytes_max);

    /* The call to copy frame can fail if the data in memory is somehow corrupted,
    * which mainly happens if we are engaged in fuzzing. In that case, we 
    * need to generate an internal error, but also let the pointer to
    * a reasonable value */
    if (bytes_next == NULL) {
        (void)picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0, "data frame was fuzzed, cannot be resent");
        bytes_next = bytes_first;
    }

    if (packet_dequeued) {
        more_retransmit = (picoquic_first_data_repeat_packet(cnx) != NULL);
    }

    *more_data |= more_retransmit;

    return bytes_next;
}
```

### Rust body
```rust
    let mut data_available = data_length;
    let mut frame_bytes_index = frame_offset + consumed;
    if packet.data_repeat_index > frame_offset + consumed {
        let already_sent = packet.data_repeat_index - frame_offset - consumed;
        if already_sent <= data_length {
            offset += already_sent as u64;
            frame_bytes_index += already_sent;
            data_available -= already_sent;
        } else {
            offset += data_length as u64;
            frame_bytes_index += data_length;
            data_available = 0;
        }
    }

    let is_needed = connection.is_stream_frame_needed(stream_id, offset, data_available, fin);

    let mut bytes_not_sent = 0usize;
```
