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

## `picoquic/logwriter.c:picoquic_enable_binlog`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C assigns quic->bin_log_fns, while Rust only takes a reference to self.bin_log_fns and leaves it unchanged; the body is placeholder-like.
* Phase 4D analysis: The Rust enable_binlog is a no-op, but the surrounding logger dispatch checks bin_log_fns before emitting binlog events and new connections clone that field from Quic. Unlike C, enabling binlog does not install a binlog backend/vtable, so unified binlog dispatch remains disabled.
* Phase 4D fix note: Make enable_binlog/set_binlog install a real binlog logger backend in bin_log_fns, or adjust the unified logger dispatch so enabled binlog events are reachable without that field.
* C source: `picoquic/logwriter.c:1346-1349`
* C signature: `void picoquic_enable_binlog(picoquic_quic_t *)`
* Rust source: `rs/fq/src/binlog.rs:1584-1597`
* Rust item: `enable_binlog`

### C body
```c
{
    quic->bin_log_fns = &binlog_functions;
}
```

### Rust body
```rust
            // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal`
            // and remains valid while the connection is live.  Borrow only
            // the disjoint context fields needed for the close hook; the
            // callback receives a shared connection borrow, as required by
            // `AutoQlog::run`.
            unsafe {
                let qlog_dir = &(*quic_ptr).qlog_dir;
                let autoqlog_fn = &mut (*quic_ptr).autoqlog_fn;
                if qlog_dir.is_some()
                    && let Some(autoqlog) = autoqlog_fn.as_mut()
                {
                    let _ = autoqlog.run(self);
                }
            }
```
