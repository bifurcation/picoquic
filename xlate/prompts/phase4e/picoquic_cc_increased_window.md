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

## `picoquic/cc_common.c:picoquic_cc_increased_window`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is unrelated stream limit frame encoding and returns a byte slice, while C computes and returns a congestion window from previous_window and rtt_min.
* Phase 4D analysis: Confirmed mismatch: Rust Connection::cc_increased_window returns the first path cwin and ignores previous_window and rtt_min; C doubles below the Reno RTT target or scales previous_window by min(rtt_min, satellite target) / Reno target.
* Phase 4D fix note: Replace Connection::cc_increased_window with the C formula using previous_window and the first path rtt_min, capped at TARGET_SATELLITE_RTT for long RTTs.
* C source: `picoquic/cc_common.c:291-304`
* C signature: `uint64_t picoquic_cc_increased_window(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:12888-12941`
* Rust item: `cc_increased_window`

### C body
```c
{
    uint64_t new_window;
    if (cnx->path[0]->rtt_min <= PICOQUIC_TARGET_RENO_RTT) {
        new_window = previous_window * 2;
    }
    else {
        double w = (double)previous_window;
        w /= (double)PICOQUIC_TARGET_RENO_RTT;
        w *= (cnx->path[0]->rtt_min > PICOQUIC_TARGET_SATELLITE_RTT) ? PICOQUIC_TARGET_SATELLITE_RTT : (double)cnx->path[0]->rtt_min;
        new_window = (uint64_t)w;
    }
    return new_window;
}
```

### Rust body
```rust
}

pub fn format_max_streams_frame_if_needed<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if connection.max_stream_id_bidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_bidir
        > connection.max_stream_id_bidir_local
    {
        let new_bidir = connection.max_stream_id_bidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_bidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsBidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_bidir).rank()) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.max_stream_id_bidir_local = new_bidir;
        *is_pure_ack = 0;
    }

    if connection.max_stream_id_unidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_unidir
        > connection.max_stream_id_unidir_local
    {
        let new_unidir = connection.max_stream_id_unidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_unidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsUnidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_unidir).rank()) {
            *more_data = 1;
            return Some(bytes);
```
