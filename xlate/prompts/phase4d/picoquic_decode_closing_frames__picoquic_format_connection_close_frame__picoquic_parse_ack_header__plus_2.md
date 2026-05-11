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

## `picoquic/frames.c:picoquic_decode_closing_frames`
* Phase 4C status: `suspect`
* Phase 4C rationale: The Rust body returns immediately if skip_frame reports ret != 0 or consumed == 0, but returns ret even when consumed == 0 and ret == 0, yielding 0 rather than continuing; the C body has no consumed==0 guard.
* C source: `picoquic/frames.c:7330-7353`
* C signature: `int picoquic_decode_closing_frames(uint8_t *, size_t, int *)`
* Rust source: `rs/fq/src/internal.rs:14943-14973`
* Rust item: `decode_closing_frames`

### C body
```c
{
    int ret = 0;
    size_t byte_index = 0;

    *closing_received = 0;
    while (ret == 0 && byte_index < bytes_max) {
        uint8_t first_byte = bytes[byte_index];

        if (first_byte == picoquic_frame_type_connection_close || first_byte == picoquic_frame_type_application_close) {
            *closing_received = 1;
            break;
        } else {
            size_t consumed = 0;
            int pure_ack = 0;

            ret = picoquic_skip_frame(bytes + byte_index,
                bytes_max - byte_index, &consumed, &pure_ack);
            byte_index += consumed;
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let max = bytes_max.min(bytes.len());
    let mut byte_index = 0;
    *closing_received = 0;
    while byte_index < max {
        let first_byte = bytes[byte_index];
        if first_byte == crate::frames::FrameType::ConnectionClose as u8
            || first_byte == crate::frames::FrameType::ApplicationClose as u8
        {
            *closing_received = 1;
            break;
        }
        let mut consumed = 0;
        let mut pure_ack = 0;
        let ret = skip_frame(
            &bytes[byte_index..max],
            max - byte_index,
            &mut consumed,
            &mut pure_ack,
        );
        if ret != 0 || consumed == 0 {
            return ret;
        }
        byte_index += consumed;
    }
    0
}
```

## `picoquic/frames.c:picoquic_format_connection_close_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C writes the frame type with a uint8 encoder, while Rust uses varint encoding for it; equivalence depends on values not visible in the body.
* C source: `picoquic/frames.c:4377-4393`
* C signature: `uint8_t * picoquic_format_connection_close_frame(picoquic_cnx_t *, uint8_t *, uint8_t *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:12742-12767`
* Rust item: `format_connection_close_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_connection_close)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, cnx->local_error)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, cnx->offending_frame_type)) != NULL &&
        (bytes = picoquic_frames_charz_encode(bytes, bytes_max, cnx->local_error_reason)) != NULL) {
        *is_pure_ack = 0;
    }
    else {
        bytes = bytes0;
        *more_data = 1;
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let reason = connection.local_error_reason.as_deref().unwrap_or("");
    let reason_bytes = reason.as_bytes();
    let mut off = 0;
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::ConnectionClose as u64,
    ) || !encode_varint_at(bytes, &mut off, connection.local_error)
        || !encode_varint_at(bytes, &mut off, connection.offending_frame_type)
        || !encode_varint_at(bytes, &mut off, reason_bytes.len() as u64)
        || bytes.len() < off + reason_bytes.len()
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + reason_bytes.len()].copy_from_slice(reason_bytes);
    off += reason_bytes.len();
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## `picoquic/frames.c:picoquic_parse_ack_header`
* Phase 4C status: `suspect`
* Phase 4C rationale: C parses path_id based on whether the path_id pointer is provided, while Rust decides from decoded frame_type; Rust also uses checked shift for ack_delay.
* C source: `picoquic/frames.c:2712-2755`
* C signature: `int picoquic_parse_ack_header(const uint8_t *, size_t, uint64_t *, uint64_t *, uint64_t *, uint64_t *, size_t *, uint8_t)`
* Rust source: `rs/fq/src/internal.rs:12340-12393`
* Rust item: `parse_ack_header`

### C body
```c
{
    int ret = 0;
    size_t byte_index = picoquic_decode_varint_length(bytes[0]);
    size_t l_largest = 0;
    size_t l_delay = 0;
    size_t l_blocks = 0;
    size_t l_path_id = 0;

    if (path_id != NULL && bytes_max > byte_index) {
        l_path_id = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, path_id);
        byte_index += l_path_id;
    }

    if (bytes_max > byte_index) {
        l_largest = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, largest);
        byte_index += l_largest;
    }

    if (bytes_max > byte_index) {
        l_delay = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, ack_delay);
        *ack_delay <<= ack_delay_exponent;
        byte_index += l_delay;
    }

    if (bytes_max > byte_index) {
        l_blocks = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, num_block);
        byte_index += l_blocks;
    }

    if (l_largest == 0 || l_delay == 0 || l_blocks == 0 || bytes_max < byte_index ||
        (path_id != NULL && l_path_id == 0)) {
        DBG_PRINTF("ack frame fixed header too large: first_byte=0x%02x, bytes_max=%" PRIst,
            bytes[0], bytes_max);
        byte_index = bytes_max;
        ret = -1;
    }

    *consumed = byte_index;
    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let max = bytes_max.min(bytes.len());
    if max == 0 {
        *consumed = 0;
        return -1;
    }

    let mut frame_type = 0;
    let mut off = varint_decode(&bytes[..max], &mut frame_type);
    if off == 0 {
        *consumed = max;
        return -1;
    }

    let is_path_ack = frame_type == crate::frames::FrameType::PathAck as u64
        || frame_type == crate::frames::FrameType::PathAckEcn as u64;
    if is_path_ack {
        let l_path = varint_decode(&bytes[off..max], path_id);
        off += l_path;
        if l_path == 0 {
            *consumed = max;
            return -1;
        }
    } else {
        *path_id = 0;
    }

    let l_largest = varint_decode(&bytes[off..max], largest);
    off += l_largest;
    let l_delay = varint_decode(&bytes[off..max], ack_delay);
    *ack_delay = ack_delay
        .checked_shl(ack_delay_exponent as u32)
        .unwrap_or(0);
    off += l_delay;
    let l_blocks = varint_decode(&bytes[off..max], num_block);
    off += l_blocks;

    if l_largest == 0 || l_delay == 0 || l_blocks == 0 || off > max {
        *consumed = max;
        -1
    } else {
        *consumed = off;
        0
    }
}
```

## `picoquic/frames.c:picoquic_should_repeat_path_response_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C compares the decoded response only against each path's first_tuple; Rust searches all tuples on each path.
* C source: `picoquic/frames.c:5065-5101`
* C signature: `int picoquic_should_repeat_path_response_frame(picoquic_cnx_t *, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/internal.rs:13349-13361`
* Rust item: `should_repeat_path_response_frame`

### C body
```c
{
    /* On the client side, challenge responses generally ought to be repeated in order to maximise
    * chances of handshake success. However, doing so on the server side may create a "blowback"
    * in case of attacks, if the initial challenge was set from an unreachable address, or if the
    * source address of the path challenge was forged.
    * If the node has sent several path responses, only the last one ought to be repeated.
    * If the path on which the response was sent is abandoned, there is no need to repeat
    * this frame. If the path is validated, then the response should always be repeated.
    */
    int should_repeat = 0;
    uint64_t response;
    if (picoquic_frames_uint64_decode(bytes + 1, bytes + bytes_max, &response) != NULL) {
        /* malformed frames will not be repeated */
        /* find the path on which the challenge was sent. */
        int path_index = -1;

        for (int i = 0; i < cnx->nb_paths; i++) {
            if (cnx->path[i]->first_tuple->challenge_response == response) {
                path_index = i;
                break;
            }
        }

        if (path_index >= 0 &&
            (cnx->path[path_index]->first_tuple->challenge_verified ||
                (cnx->client_mode && !cnx->path[path_index]->first_tuple->challenge_failed))) {
            should_repeat = 1;
        }
        else {
            should_repeat = 0;
        }
    }

    return should_repeat;
}
```

### Rust body
```rust
    pub fn should_repeat_path_response_frame(&self, bytes: &[u8], bytes_max: usize) -> bool {
        let max = bytes_max.min(bytes.len());
        if max < 9 {
            return false;
        }
        let response = parse_64(&bytes[1..9]);
        self.paths.iter().any(|path| {
            path.tuples.iter().any(|tuple| {
                tuple.challenge_response == response
                    && (tuple.challenge_verified || (self.client_mode && !tuple.challenge_failed))
            })
        })
    }
```

## `picoquic/logwriter.c:picoquic_log_path_available_or_backup_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C skips frame type then delegates to skip_path_available_or_backup_frame; Rust skips exactly three varints, which is only body-visibly equivalent if that helper skips two varints.
* C source: `picoquic/logwriter.c:459-467`
* C signature: `const uint8_t * picoquic_log_path_available_or_backup_frame(FILE *, const uint8_t *, const uint8_t *)`
* Rust source: `rs/fq/src/binlog.rs:603-614`
* Rust item: `log_path_available_or_backup_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* frame type as varint */
    bytes = picoquic_skip_path_available_or_backup_frame(bytes, bytes_max); /* skip available or backup frame */
    picoquic_binlog_frame(f, bytes_begin, bytes);

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```
