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

## `picoquic/frames.c:picoquic_copy_stream_frame_for_retransmit`
* Phase 4C status: `suspect`
* Phase 4C rationale: C has a visible cnx == NULL path that keeps the frame needed; Rust always performs connection stream lookup and suppresses retransmit when the stream is missing.
* Phase 4D analysis: C's nullable cnx is meaningful: the C unit test passes NULL to exercise pure frame copying, which bypasses stream lookup. Rust requires a Connection and treats a missing stream as not needed, so the no-connection/missing-stream behavior is not preserved.
* Phase 4D fix note: Represent the nullable connection path in Rust, for example by accepting Option<&mut Connection> or adding a no-connection copy path that skips stream lookup; keep production callers using a connection and update the translated dataqueue copy test accordingly.
* C source: `picoquic/frames.c:2277-2397`
* C signature: `uint8_t * picoquic_copy_stream_frame_for_retransmit(picoquic_cnx_t *, picoquic_packet_t *, uint8_t *, uint8_t *)`
* Rust source: `rs/fq/src/internal.rs:10541-10658`
* Rust item: `copy_stream_frame_for_retransmit`

### C body
```c
{
    uint8_t* frame = packet->bytes + packet->data_repeat_frame;
    size_t frame_length_max = packet->length - packet->data_repeat_frame;
    uint64_t stream_id;
    uint64_t offset;
    size_t data_length;
    size_t consumed;
    size_t bytes_not_sent = 0;
    int fin;

    if (picoquic_parse_stream_header(frame, frame_length_max, &stream_id, &offset, &data_length, &fin, &consumed) != 0) {
        /* Malformed stream frame. Error. */
        bytes_next = NULL;
    }
    else {
        uint8_t* bytes_first = bytes_next;
        /* Need to find out how much is really available, based on the index in the packet */
        size_t data_available = data_length;
        uint8_t* frame_bytes = frame + consumed;
        int is_needed = 1;
        if (packet->data_repeat_index > packet->data_repeat_frame + consumed) {
            size_t already_sent = packet->data_repeat_index - packet->data_repeat_frame - consumed;
            if (already_sent <= data_length) {
                offset += already_sent;
                frame_bytes += already_sent;
                data_available -= already_sent;
            }
            else {
                /* This is really an internal error! */
                offset += data_length;
                frame_bytes += data_length;
                data_available = 0;
            }
        }
        /* Check that these bytes are needed.
         * The code only deletes a stream context if all the stream bytes have been acknowledged,
         * including the FIN flag which is counted as a final octet after the max offset.
         * If the stream is deleted or reset, there is no need to send again any stream data frame for that stream.
         * If all the octets in the frame are acknowledged, including the FIN bit if present, there is
         * also no need to send the frame again.
         */
        if (cnx != NULL) {
            picoquic_stream_head_t* stream = picoquic_find_stream(cnx, stream_id);
            if (stream == NULL || stream->reset_sent || 
                picoquic_check_sack_list(&stream->sack_list, offset, offset + data_available - ((fin) ? 0 : 1))) {
                /* That frame is not needed anymore */
                is_needed = 0;
            }
        }
        if (is_needed) {
            /* Need to check how much can be encoded in the packet:
             * Header (with or without FIN), stream_id, offset, length.
             */
            if ((bytes_next = picoquic_format_stream_frame_header(bytes_next, bytes_max, stream_id, offset)) == NULL ||
                bytes_next == bytes_max) {
                /* Cannot encode anything! -- need to wait for another opportunity */
                bytes_not_sent = data_available;
                bytes_next = bytes_first;
            }
            else {
                uint8_t* before_length = bytes_next;
                if ((bytes_next = picoquic_frames_varint_encode(bytes_next, bytes_max, data_available)) != NULL &&
                    bytes_next + data_available <= bytes_max) {
                    /* Can encode everything in a natural way */
                    *bytes_first |= 2; /* length is present */
                    *bytes_first |= fin; /* fin OK */
                    memcpy(bytes_next, frame_bytes, data_available);
                    bytes_next += data_available;
                }
                else if (before_length + data_available <= bytes_max) {
                    /* everything fits if we remove the length, but we may need to insert initial padding */
                    size_t space_available = bytes_max - before_length;
                    size_t pad_required = space_available - data_available;
                    bytes_next = before_length;
                    *bytes_first |= fin; /* fin OK */
                    if (pad_required > 0) {
                        memmove(bytes_first + pad_required, bytes_first, before_length - bytes_first);
                        for (size_t i = 0; i < pad_required; i++) {
                            bytes_first[i] = 0;
                        }
                        bytes_next += pad_required;
                    }
                    memcpy(bytes_next, frame_bytes, data_available);
                    bytes_next += data_available;
                }
                else {
                    /* buffer is too short -- do not send the FIN bit, do not set the length, just copy bytes */
                    size_t available = bytes_max - before_length;
                    if (available < PICOQUIC_MIN_STREAM_DATA_FRAGMENT) {
                        bytes_not_sent = data_available;
                        bytes_next = bytes_first;
                    }
                    else {
                        bytes_next = before_length;
                        memcpy(bytes_next, frame_bytes, available);
                        bytes_next += available;
                        bytes_not_sent = data_available - available;
                    }
                }
            }
        }

        if (bytes_not_sent == 0) {
            /* Progress frame index to next byte after data frame */
            packet->data_repeat_index = packet->data_repeat_frame + consumed + data_length;
            packet->data_repeat_frame = packet->data_repeat_index;
        }
        else if (bytes_not_sent < data_length) {
            /* Progress index to next byte not sent */
            packet->data_repeat_index = packet->data_repeat_frame + consumed + data_length - bytes_not_sent;
        }
    }

    return bytes_next;
}
```

### Rust body
```rust
            *no_need_to_repeat = 1;
            return 0;
        }
        match self
            .find_stream(stream_id)
            .and_then(|token| self.streams.get(token))
        {
            Some(stream) if !(stream.fin_received || stream.reset_received) => {}
            _ => *no_need_to_repeat = 1,
        }
        0
    }

    fn check_reset_stream_at_needs_repeat(
        &mut self,
        bytes: &[u8],
        no_need_to_repeat: &mut i32,
    ) -> i32 {
        let mut stream_id = 0;
        let mut reliable_size = 0;
        let tail = frames_varint_skip(bytes)
            .and_then(|tail| frames_varint_decode(tail, &mut stream_id))
            .and_then(frames_varint_skip)
            .and_then(frames_varint_skip)
            .and_then(|tail| frames_varint_decode(tail, &mut reliable_size));
        if tail.is_none() {
            return -1;
        }
        match self
            .find_stream(stream_id)
            .and_then(|token| self.streams.get(token))
        {
            Some(stream) if !stream.reset_acked => {}
            _ => *no_need_to_repeat = 1,
        }
        0
    }

    fn path_available_or_backup_frame_need_repeat(
        &self,
        bytes: &[u8],
        no_need_to_repeat: &mut i32,
    ) -> i32 {
        *no_need_to_repeat = 0;
        let Some((path_id, sequence, _)) = Self::parse_two_varints(bytes) else {
            *no_need_to_repeat = 1;
            return 0;
        };
        let path_index = self.find_path_by_unique_id(path_id);
        if path_index < 0 {
            *no_need_to_repeat = 1;
        } else if let Some(path) = self.paths.get(path_index as usize)
            && (path.status_sequence_sent_last != sequence || path.path_is_demoted)
        {
            *no_need_to_repeat = 1;
        }
        0
    }

    fn max_path_id_frame_needs_repeat(&self, bytes: &[u8], no_need_to_repeat: &mut i32) -> i32 {
        *no_need_to_repeat = 0;
        let mut max_path_id = 0;
        if frames_varint_decode(bytes, &mut max_path_id).is_none()
            || max_path_id <= self.max_path_id_local
            || max_path_id <= self.max_path_id_acknowledged
        {
            *no_need_to_repeat = 1;
        }
        0
    }

    fn paths_blocked_frame_needs_repeat(&self, bytes: &[u8], no_need_to_repeat: &mut i32) -> i32 {
        *no_need_to_repeat = 0;
        let mut max_path_id = 0;
        if frames_varint_decode(bytes, &mut max_path_id).is_none()
            || max_path_id <= self.max_path_id_remote
            || max_path_id <= self.paths_blocked_acknowledged
        {
            *no_need_to_repeat = 1;
        }
        0
    }

    fn path_cid_blocked_frame_needs_repeat(
        &self,
        bytes: &[u8],
        no_need_to_repeat: &mut i32,
    ) -> i32 {
        *no_need_to_repeat = 0;
        let Some((unique_path_id, next_sequence_number, _)) = Self::parse_two_varints(bytes) else {
            *no_need_to_repeat = 1;
            return 0;
        };
        let path_index = self.find_path_by_unique_id(unique_path_id);
        if path_index < 0 {
            *no_need_to_repeat = 1;
        } else if let Some(path) = self.paths.get(path_index as usize)
            && (!path.sending_path_cid_blocked_frame
                || self.path_cid_next_sequence_number(path) > next_sequence_number)
        {
            *no_need_to_repeat = 1;
        }
        0
    }

    pub fn check_frame_needs_repeat(
        &mut self,
        bytes: &[u8],
        bytes_max: usize,
        p_type: PacketType,
        no_need_to_repeat: &mut i32,
        do_not_detect_spurious: &mut i32,
        is_preemptive_needed: &mut i32,
    ) -> i32 {
```
