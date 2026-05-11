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

Owned Rust file(s): `rs/fq/src/binlog.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/logwriter.c:picoquic_log_stream_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: For length-bearing stream frames, C logs through bytes plus extra payload after the parsed length, while Rust computes copy_end from the pre-length head_len and may omit the encoded length bytes.
* Phase 4D analysis: Confirmed real mismatch. C logs through the parsed length varint and then up to 8 payload bytes for length-bearing STREAM frames; Rust uses a head length captured before reading the length, so the length varint consumes or replaces the payload preview budget and zero-length frames omit the length varint entirely.
* Phase 4D fix note: Recompute the consumed header length after read_length for the has-length path, then log header plus encoded length plus payload preview; also preserve C's short erroneous-frame logging behavior if repairing the whole routine.
* C source: `picoquic/logwriter.c:75-136`
* C signature: `const uint8_t * picoquic_log_stream_frame(FILE *, const uint8_t *, const uint8_t *)`
* Rust source: `rs/fq/src/binlog.rs:377-422`
* Rust item: `log_stream_frame`

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

### Rust body
```rust
    }
    let dcid_len = send_buffer[pos] as usize;
    pos += 1;
    if pos + dcid_len > send_buffer.len() {
        ph.packet_type = PacketType::Error;
        return ph;
    }
    let Some(cid) = ConnectionId::clone_from_slice(&send_buffer[pos..pos + dcid_len]) else {
        ph.packet_type = PacketType::Error;
        return ph;
    };
    ph.dest_connection_id = cid;
    pos += dcid_len;

    if pos >= send_buffer.len() {
        ph.packet_type = PacketType::Error;
        return ph;
    }
    let scid_len = send_buffer[pos] as usize;
    pos += 1;
    if pos + scid_len > send_buffer.len() {
        ph.packet_type = PacketType::Error;
        return ph;
    }
    let Some(cid) = ConnectionId::clone_from_slice(&send_buffer[pos..pos + scid_len]) else {
        ph.packet_type = PacketType::Error;
        return ph;
    };
    ph.src_connection_id = cid;
    pos += scid_len;

    if version == 0 {
        ph.packet_type = PacketType::VersionNegotiation;
        ph.offset = pos;
        ph.payload_length_value = send_buffer.len().saturating_sub(pos);
        ph.payload_length = ph.payload_length_value;
        return ph;
    }

    ph.packet_type = crate::internal::parse_long_packet_type(flags, ph.version_index);
    ph.quic_bit_is_zero = (flags & 0x40) == 0;
    ph.spin = false;
    ph.has_spin_bit = false;

    match ph.packet_type {
        PacketType::Initial => {
```
