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

## `picoquic/frames.c:picoquic_copy_stream_frames_for_retransmit`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C loops over queued packets by priority, calls a single-frame retransmit helper, handles dequeue/error state, and updates more_data; Rust copies raw bytes from only the first packet and ignores current_priority.
* Prior Phase 4D analysis: Confirmed mismatch: Rust ignores current_priority, copies raw bytes from one packet, does not call the translated single-frame retransmit helper, and does not dequeue/requeue/error-handle like C.
* Phase 4E claimed outcome: `ok`
* Phase 4E repair analysis: Current Rust implementation already matches the C wrapper behavior: priority queue loop, single-frame retransmit delegation, dequeue/requeue handling, more_data propagation, and corrupted-frame fallback are present.
* Phase 4E fix summary: 
* C source: `picoquic/frames.c:2462-2501`
* C signature: `uint8_t * picoquic_copy_stream_frames_for_retransmit(picoquic_cnx_t *, uint8_t *, uint8_t *, uint64_t, int *, int *)`
* Current Rust source: `rs/fq/src/internal.rs:11279-11302`
* Current Rust item: `copy_stream_frames_for_retransmit`

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

### Current Rust body
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
