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

## `picoquic/logwriter.c:picoquic_log_stream_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: For length-bearing stream frames, C logs through bytes plus extra payload after the parsed length, while Rust computes copy_end from the pre-length head_len and may omit the encoded length bytes.
* Prior Phase 4D analysis: Confirmed real mismatch. C logs through the parsed length varint and then up to 8 payload bytes for length-bearing STREAM frames; Rust uses a head length captured before reading the length, so the length varint consumes or replaces the payload preview budget and zero-length frames omit the length varint entirely.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Confirmed mismatch: length-bearing STREAM logging used the pre-length header length, omitting or undercounting the encoded length varint. Rust now logs through the parsed length varint plus the payload preview and preserves C-style short-frame error logging.
* Phase 4E fix summary: Updated log_stream_frame header-length accounting after read_length, added cautious STREAM parse-error logging, and added focused unit tests for length-bearing, zero-length, and short STREAM frames.
* C source: `picoquic/logwriter.c:75-136`
* C signature: `const uint8_t * picoquic_log_stream_frame(FILE *, const uint8_t *, const uint8_t *)`
* Current Rust source: `rs/fq/src/binlog.rs:533-591`
* Current Rust item: `log_stream_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    uint8_t ftype = bytes[0];
    size_t length = 0;
    uint8_t log_buffer[256];
    int has_length = 0;
    size_t extra_bytes = 8;

    bytes = picoquic_log_fixed_skip(bytes, bytes_max, 1); /* type */
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* stream */

    if ((ftype & 4) != 0) {
        bytes = picoquic_log_varint_skip(bytes, bytes_max); /* offset */
    }

    if (bytes != NULL) {
        if ((ftype & 2) != 0) {
            bytes = picoquic_log_length(bytes, bytes_max, &length); /* length */
            has_length = 1;
        }
        else {
            length = bytes_max - bytes;
        }
    }

    if (bytes != NULL) {
        if (length < extra_bytes) {
            /* Add up to 8 bytes of content that can be documented in the qlog */
            extra_bytes = length;
        }
        if (has_length) {
            picoquic_binlog_frame(f, bytes_begin, bytes + extra_bytes);
        }
        else {
            uint8_t* log_next = log_buffer;
            size_t l_head = bytes - bytes_begin;

            memcpy(log_buffer, bytes_begin, l_head);
            log_next += l_head;
            if ((log_next = picoquic_frames_varint_encode(log_next, log_buffer + 256, length)) != NULL) {
                memcpy(log_next, bytes, extra_bytes);
                log_next += extra_bytes;
                picoquic_binlog_frame(f, log_buffer, log_next);
            }
            else {
                picoquic_binlog_frame(f, log_buffer, log_buffer + l_head);
            }
        }

        bytes = picoquic_log_fixed_skip(bytes, bytes_max, length);
    }
    else {
        /* Cautiously log the beginning of the erroneous frame */
        length = bytes_max - bytes_begin;
        if (length > 26) {
            length = 26;
        }
        picoquic_binlog_frame(f, bytes_begin, bytes_begin + length);
    }
    return bytes;
}
```

### Current Rust body
```rust
fn log_stream_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    if bytes_in.is_empty() {
        return None;
    }
    let ftype = bytes_in[0];
    let mut bytes = match skip_fixed(bytes_in, 1) {
        Some(rest) => rest,
        None => return log_stream_error(out, bytes_begin),
    };
    bytes = match frames_varint_skip(bytes) {
        Some(rest) => rest,
        None => return log_stream_error(out, bytes_begin),
    };
    if (ftype & 4) != 0 {
        bytes = match frames_varint_skip(bytes) {
            Some(rest) => rest,
            None => return log_stream_error(out, bytes_begin),
        };
    }

    let has_length = (ftype & 2) != 0;
    let length: usize;
    if has_length {
        let (l, rest) = match read_length(bytes) {
            Some(v) => v,
            None => return log_stream_error(out, bytes_begin),
        };
        length = l;
        bytes = rest;
    } else {
        length = bytes.len();
    }

    let mut extra_bytes: usize = 8;
    if length < extra_bytes {
        extra_bytes = length;
    }

    if has_length {
        let header_len = bytes_begin.len() - bytes.len();
        let copy_end = header_len + extra_bytes.min(bytes.len());
        append_frame(out, &bytes_begin[..copy_end]);
    } else {
        let head_len = bytes_begin.len() - bytes.len();
        let mut log_buffer = Vec::with_capacity(head_len + 8 + extra_bytes);
        log_buffer.extend_from_slice(&bytes_begin[..head_len]);
        let mut len_buf = [0u8; 8];
        let n = varint_encode(&mut len_buf, length as u64);
        log_buffer.extend_from_slice(&len_buf[..n]);
        let payload_start = head_len;
        let avail = bytes_begin.len().saturating_sub(payload_start);
        let take = extra_bytes.min(avail);
        log_buffer.extend_from_slice(&bytes_begin[payload_start..payload_start + take]);
        append_frame(out, &log_buffer);
    }

    skip_fixed(bytes, length)
}
```
