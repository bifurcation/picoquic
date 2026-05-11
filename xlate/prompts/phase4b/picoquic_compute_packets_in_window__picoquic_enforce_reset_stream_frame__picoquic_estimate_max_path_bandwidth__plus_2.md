# Phase 4B missing implementation batch

## NO SHORTCUTS — do the hard work

Phase 4B implements approved required_missing items.  Past sweeps
produced hundreds of stubs that compile but don't translate the
C body — that is failure, not completion.  The gate flags all
of these patterns identically:

* `todo!()`
* `unimplemented!()`
* `// SKIP:` or any placeholder marker (None / Err(Generic) /
  Ok(()) returns plus an excuse comment) — forbidden
* Fabricated default returns (0, false, None, empty Vec) when
  the C body computes a real value — forbidden
* Half-translated bodies that punt error paths to `todo!()` —
  forbidden

**Nothing in picoquic is fundamentally untranslatable.**  Every
function has a body — translate it.  Concrete mappings:
  * `pthread_create` → `std::thread::spawn`
  * `pipe()` wake-up → `std::sync::mpsc` or `Condvar`
  * `select`/`poll`/`io_uring` → `mio` crate
  * loglib helpers → `log` crate + standard file I/O
  * borrow-checker issues → `&mut self`, `RefCell`, restructure

**Forbidden 'blocker' excuses** (every one is a shortcut):
'out of scope', 'deferred to a later pass', 'multi-threading
not in scope', 'loglib not in scope', 'sub-system not yet
translated', 'needs design thought', 'borrow-checker issue'.

The ONLY acceptable bare `todo!()` is a concrete external
dependency outside our reach (e.g. 'blocked: needs picotls API
not yet exposed in the Rust binding') — and even then, prefer
translating the dependency first.

## Implementation rules

Implement the approved missing Rust functions/items listed below.
Follow `CLAUDE.md`, `TRANSLATE_PLAN.md`, and
`xlate/impl_translation_guide.md`.

* Translate the C behavior faithfully into safe, idiomatic Rust.
* Keep the approved module structure unless implementation proves
  it wrong; if it is wrong, stop and report the needed Phase 4A
  plan amendment.
* Add `/// C: ` references for implemented functions so Phase 4A
  can map them on the next refresh.
* Edit only `rs/fq/`, `scripts/`, `xlate/`, or markdown files
  allowed by the repository instructions.

## Verification — run ONLY these cargo commands

* **`cargo test --no-run`** — compile-only.  Use this as your
  build gate.  Do NOT run `cargo test` (full suite) — it will
  run all 505 tests and take 2+ hours.
* **`cargo test <specific_test_name>`** — running a single
  named test for narrow verification is fine.
* **`cargo fmt`** and **`cargo clippy --tests --all-features
  -- -D warnings`** — final polish.
* **Do NOT prefix with `CARGO_INCREMENTAL=0`** — the allowlist
  blocks env-var prefixes; just run `cargo ...` directly.

Run these after implementing:

```sh
cargo fmt
cargo test --no-run
cargo clippy --tests --all-features -- -D warnings
```

## `picoquic/frames.c:picoquic_compute_packets_in_window`
* C source: `picoquic/frames.c:2974-2995`
* C signature: `uint64_t picoquic_compute_packets_in_window(picoquic_cnx_t *, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_enforce_reset_stream_frame`
* C source: `picoquic/frames.c:252-282`
* C signature: `void picoquic_enforce_reset_stream_frame(picoquic_stream_head_t *, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_estimate_max_path_bandwidth`
* C source: `picoquic/frames.c:2924-2961`
* C signature: `void picoquic_estimate_max_path_bandwidth(picoquic_path_t *, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_queue_data_repeat_node_create`
* C source: `picoquic/frames.c:2129-2136`
* C signature: `picosplay_node_t * picoquic_queue_data_repeat_node_create(void *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_queue_data_repeat_node_value`
* C source: `picoquic/frames.c:2138-2141`
* C signature: `void * picoquic_queue_data_repeat_node_value(picosplay_node_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
