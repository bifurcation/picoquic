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

## `picoquic/sender.c:picoquic_preemptive_retransmit_packet`
* Phase 4C status: `suspect`
* Phase 4C rationale: Bodies mostly match, but C advances byte_index by frame_length exactly, while Rust uses frame_length.max(1), which can change behavior when frame_length is zero.
* C source: `picoquic/sender.c:1332-1419`
* C signature: `int picoquic_preemptive_retransmit_packet(picoquic_packet_t *, picoquic_cnx_t *, uint8_t *, size_t, size_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:17204-17280`
* Rust item: `preemptive_retransmit_packet`

### C body
```c
{
    /* check if this is an ACK only packet */
    int ret = 0;
    int frame_is_pure_ack = 0;
    size_t frame_length = 0;
    size_t byte_index = 0; /* Used when parsing the old packet */
    size_t write_index = 0;
    int is_repeated = 1;
    int do_not_detect_spurious = 0;
    int is_preemptive_needed = 0;
    size_t initial_length = *length;
    *has_data = 0;

    if (!old_p->is_mtu_probe &&
        !old_p->is_ack_trap &&
        !old_p->is_multipath_probe) {
        /* Copy the relevant bytes from one packet to the next */
        byte_index = old_p->offset;

        while (ret == 0 && byte_index < old_p->length) {
            ret = picoquic_skip_frame(&old_p->bytes[byte_index],
                old_p->length - byte_index, &frame_length, &frame_is_pure_ack);

            /* Check whether the data was already acked, which may happen in
             * case of spurious retransmissions */
            if (ret == 0 && frame_is_pure_ack == 0) {
                ret = picoquic_check_frame_needs_repeat(cnx, &old_p->bytes[byte_index],
                    frame_length, old_p->ptype, &frame_is_pure_ack, &do_not_detect_spurious, &is_preemptive_needed);
            }

            /* Prepare retransmission if needed */
            if (ret == 0 && !frame_is_pure_ack) {
                if (PICOQUIC_IN_RANGE(old_p->bytes[byte_index], picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max) &&
                    picoquic_is_stream_frame_unlimited(&old_p->bytes[byte_index])) {
                    /* If length is not present, check whether needed */
                    if (write_index + frame_length < send_buffer_max_minus_checksum) {
                        size_t pad_needed = send_buffer_max_minus_checksum - write_index - frame_length;
                        memset(&new_bytes[write_index], picoquic_frame_type_padding, pad_needed);
                        *length += pad_needed;
                        write_index += pad_needed;
                    }
                }
                /* copy the frame */
                if (write_index + frame_length <= send_buffer_max_minus_checksum) {
                    memcpy(&new_bytes[write_index], &old_p->bytes[byte_index], frame_length);
                    write_index += frame_length;
                    *length += frame_length;
                    *has_data = 1;
                }
                else {
                    is_repeated = 0;
                }
            }
            byte_index += frame_length;
        }
    }

    if (*has_data) {
        if (!is_preemptive_needed) {
            /* If the packet does not contain any frame requiring preemptive repeat, do not repeat it. */
            *length = initial_length;
            *has_data = 0;
            is_repeated = 0;
        } else if (is_repeated) {
            old_p->was_preemptively_repeated = 1;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut ret = 0;
        let mut write_index = 0usize;
        let mut is_repeated = 1;
        let mut do_not_detect_spurious = 0;
        let mut is_preemptive_needed = 0;
        let initial_length = *length;
        *has_data = 0;

        if !old_p.is_mtu_probe && !old_p.is_ack_trap && !old_p.is_multipath_probe {
            let mut byte_index = old_p.offset;
            while ret == 0 && byte_index < old_p.length {
                let mut frame_length = 0usize;
                let mut frame_is_pure_ack = 0;
                ret = skip_frame(
                    &old_p.bytes[byte_index..old_p.length],
                    old_p.length - byte_index,
                    &mut frame_length,
                    &mut frame_is_pure_ack,
                );
                if ret == 0 && frame_is_pure_ack == 0 {
                    ret = self.check_frame_needs_repeat(
                        &old_p.bytes[byte_index..old_p.length],
                        frame_length,
                        old_p.packet_type,
                        &mut frame_is_pure_ack,
                        &mut do_not_detect_spurious,
                        &mut is_preemptive_needed,
                    );
                }
                if ret == 0 && frame_is_pure_ack == 0 {
                    if old_p.bytes[byte_index] >= crate::frames::FrameType::StreamRangeMin as u8
                        && old_p.bytes[byte_index] <= crate::frames::FrameType::StreamRangeMax as u8
                        && is_stream_frame_unlimited(&old_p.bytes[byte_index..old_p.length])
                        && write_index + frame_length < send_buffer_max_minus_checksum
                    {
                        let pad_needed =
                            send_buffer_max_minus_checksum - write_index - frame_length;
                        new_bytes[write_index..write_index + pad_needed]
                            .fill(crate::frames::FrameType::Padding as u8);
                        *length = (*length).saturating_add(pad_needed);
                        write_index += pad_needed;
                    }
                    if write_index + frame_length <= send_buffer_max_minus_checksum
                        && write_index + frame_length <= new_bytes.len()
                    {
                        new_bytes[write_index..write_index + frame_length]
                            .copy_from_slice(&old_p.bytes[byte_index..byte_index + frame_length]);
                        write_index += frame_length;
                        *length = (*length).saturating_add(frame_length);
                        *has_data = 1;
                    } else {
                        is_repeated = 0;
                    }
                }
                byte_index = byte_index.saturating_add(frame_length.max(1));
            }
        }

        if *has_data != 0 {
            if is_preemptive_needed == 0 {
                *length = initial_length;
                *has_data = 0;
            } else if is_repeated != 0 {
                old_p.was_preemptively_repeated = true;
            }
        }

        ret
    }
```
