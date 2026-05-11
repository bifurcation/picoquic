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

## `picoquic/fastcc.c:picoquic_fastcc_delay_threshold`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a fastcc reset routine, not the delay-threshold function that returns min(rtt_min/8, FASTCC_DELAY_THRESHOLD_MAX).
* C source: `picoquic/fastcc.c:65-72`
* C signature: `uint64_t picoquic_fastcc_delay_threshold(uint64_t)`
* Rust source: `rs/fq/src/fastcc.rs:81-97`
* Rust item: `fastcc_delay_threshold`

### C body
```c
{
    uint64_t delay = rtt_min / 8;
    if (delay > FASTCC_DELAY_THRESHOLD_MAX) {
        delay = FASTCC_DELAY_THRESHOLD_MAX;
    }
    return delay;
}
```

### Rust body
```rust
pub fn picoquic_fastcc_reset(state: &mut FastccState, path_x: &mut Path, current_time: Instant) {
    *state = FastccState::default();
    state.alg_state = FastccAlgState::Initial;
    state.rtt_min = path_x.smoothed_rtt.ticks();
    state.rolling_rtt_min = state.rtt_min;
    state.delay_threshold = fastcc_delay_threshold(state.rtt_min);
    state.end_of_epoch = current_time.ticks().saturating_add(FASTCC_PERIOD);
    path_x.cwin = CWIN_INITIAL;
}
```

## `picoquic/frames.c:picoquic_copy_stream_frames_for_retransmit`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C loops over queued packets by priority, calls a single-frame retransmit helper, handles dequeue/error state, and updates more_data; Rust copies raw bytes from only the first packet and ignores current_priority.
* C source: `picoquic/frames.c:2462-2501`
* C signature: `uint8_t * picoquic_copy_stream_frames_for_retransmit(picoquic_cnx_t *, uint8_t *, uint8_t *, uint64_t, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:10711-10734`
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
) -> Option<&'a mut [u8]> {
    let Some(packet_token) = connection.first_data_repeat_packet() else {
        return Some(bytes);
    };
    let packet = connection.queued_packets.get_mut(packet_token)?;
    let start = packet.data_repeat_frame;
    let end = packet.length.min(packet.bytes.len());
    if start >= end || bytes.len() < end - start {
        return None;
    }
    let len = end - start;
    bytes[..len].copy_from_slice(&packet.bytes[start..end]);
    *is_pure_ack = 0;
    if connection.first_data_repeat_packet().is_some() {
        *more_data = 1;
    }
    Some(&mut bytes[len..])
}
```

## `picoquic/frames.c:picoquic_find_or_create_stream`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a len method returning inner.len(), not find-or-create stream logic.
* C source: `picoquic/frames.c:188-197`
* C signature: `picoquic_stream_head_t * picoquic_find_or_create_stream(picoquic_cnx_t *, uint64_t, int)`
* Rust source: `rs/fq/src/internal.rs:16331-16340`
* Rust item: `new`

### C body
```c
{
    picoquic_stream_head_t* stream = picoquic_find_stream(cnx, stream_id);

    if (stream == NULL) {
        stream = picoquic_create_missing_streams(cnx, stream_id, is_remote);
    }

    return stream;
}
```

### Rust body
```rust
    pub fn len(&self) -> usize {
        self.inner.len()
    }
```

## `picoquic/frames.c:picoquic_format_ready_datagram_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C encodes a datagram frame type and invokes a prepare-datagram callback path; Rust only emits an already queued datagram frame and clears readiness flags when the queue is empty.
* C source: `picoquic/frames.c:5471-5524`
* C signature: `uint8_t * picoquic_format_ready_datagram_frame(picoquic_cnx_t *, picoquic_path_t *, uint8_t *, uint8_t *, int *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:13949-13973`
* Rust item: `format_ready_datagram_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_datagram_l)) == NULL ||
        bytes + 16 > bytes_max){
        bytes = bytes0;
        *more_data = 1;
    }
    else {
        /* Compute the length */
        size_t allowed_space = bytes_max - bytes;
        picoquic_datagram_buffer_argument_t datagram_data_context;

        if (allowed_space > cnx->remote_parameters.max_datagram_frame_size) {
            allowed_space = cnx->remote_parameters.max_datagram_frame_size;
        }

        datagram_data_context.cnx = cnx;
        datagram_data_context.path_x = path_x;
        datagram_data_context.bytes0 = bytes0;
        datagram_data_context.bytes = bytes;
        datagram_data_context.bytes_max = bytes_max;
        datagram_data_context.allowed_space = allowed_space;
        datagram_data_context.after_data = bytes0;
        datagram_data_context.is_active = 0;
        datagram_data_context.is_old_api = 0;
        datagram_data_context.was_called = 0;

        if (cnx->callback_fn != NULL && (cnx->callback_fn)(cnx, (cnx->are_path_callbacks_enabled)?path_x->unique_path_id:0, (uint8_t*)&datagram_data_context, allowed_space,
            picoquic_callback_prepare_datagram, cnx->callback_ctx, NULL) != 0) {
            /* something went wrong */
            picoquic_log_app_message(cnx, "Prepare datagram returns error 0x%x", PICOQUIC_TRANSPORT_INTERNAL_ERROR);
            *ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0);
            bytes = bytes0; /* CHECK: SHOULD THIS BE NULL ? */
        }
        else {
            bytes = datagram_data_context.after_data;
            if (bytes > bytes0) {
                *is_pure_ack = 0;
            }

            if (datagram_data_context.is_old_api || !datagram_data_context.was_called) {
                *more_data |= cnx->is_datagram_ready;
            }
            else {
                *more_data |= datagram_data_context.is_active;
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    *ret = 0;
    if !connection.is_datagram_ready && !path_x.is_datagram_ready && connection.datagrams.is_empty()
    {
        return Some(bytes);
    }
    if connection.datagrams.is_empty() {
        connection.is_datagram_ready = false;
        path_x.is_datagram_ready = false;
        return Some(bytes);
    }
    let tail = format_first_datagram_frame(connection, bytes, 0, more_data, is_pure_ack)?;
    if connection.datagrams.is_empty() {
        connection.is_datagram_ready = false;
        path_x.is_datagram_ready = false;
    }
    Some(tail)
}
```

## `picoquic/frames.c:picoquic_process_ack_of_stream_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a test helper with packet fixtures and repeat checks, not stream-header parsing and SACK update logic.
* C source: `picoquic/frames.c:3676-3704`
* C signature: `int picoquic_process_ack_of_stream_frame(picoquic_cnx_t *, uint8_t *, size_t, size_t *)`
* Rust source: `rs/fq/src/tests/skip_frame.rs:644-688`
* Rust item: `stream_ack_test_one`

### C body
```c
{
    int ret;
    int fin;
    size_t data_length;
    uint64_t stream_id;
    uint64_t offset;
    picoquic_stream_head_t* stream = NULL;

    /* skip stream frame */
    ret = picoquic_parse_stream_header(bytes, bytes_max,
        &stream_id, &offset, &data_length, &fin, consumed);

    if (ret == 0) {
        *consumed += data_length;

        /* record the ack range for the stream */
        stream = picoquic_find_stream(cnx, stream_id);
        if (stream != NULL) {
            (void)picoquic_update_sack_list(&stream->sack_list,
                offset, offset + data_length - ((fin) ? 0 : 1), 0);

            picoquic_delete_stream_if_closed(cnx, stream);
        }
    }

    return ret;
}
```

### Rust body
```rust
fn stream_ack_test_one(quic: &mut Quic) -> crate::Result<()> {
    let packets: &[(&[u8], bool)] = &[
        (&[0x0b, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0x0b, 4, 0], true),
        (&[0x0a, 12, 8, 0, 1, 2, 3, 4, 5, 6, 7], true),
        (&[0x0f, 16, 32, 8, 1, 2, 3, 4, 5, 6, 7, 8], true),
        (&[0x09, 20, 4, 0, 0, 0, 0, 1, 2, 3, 4, 5], false),
    ];
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    for stream_id in [0, 4, 8, 12, 16, 20] {
        let _ = cnx.create_stream(stream_id)?;
    }
    for (packet, should_ack) in packets {
        let mut byte_index = 0usize;
        while byte_index < packet.len() {
            let mut consumed = 0usize;
            let mut pure_ack = 0i32;
            if skip_frame(
                &packet[byte_index..],
                packet.len() - byte_index,
                &mut consumed,
                &mut pure_ack,
            ) != 0
            {
                return Err(crate::Error::InvalidFrame);
            }
            let mut no_need_to_repeat = 0;
            let mut do_not_detect_spurious = 0;
            let mut is_preemptive_needed = 0;
            let ret = cnx.check_frame_needs_repeat(
                &packet[byte_index..byte_index + consumed],
                consumed,
                crate::internal::PacketType::OneRttProtected,
                &mut no_need_to_repeat,
                &mut do_not_detect_spurious,
                &mut is_preemptive_needed,
            );
            if ret != 0 || (*should_ack && pure_ack != 0) {
                return Err(crate::Error::Generic);
            }
            byte_index += consumed;
        }
    }
    Ok(())
}
```
