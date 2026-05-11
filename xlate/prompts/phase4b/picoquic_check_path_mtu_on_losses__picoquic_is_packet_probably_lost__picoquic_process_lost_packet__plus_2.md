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

## `picoquic/loss_recovery.c:picoquic_check_path_mtu_on_losses`
* C source: `picoquic/loss_recovery.c:873-893`
* C signature: `void picoquic_check_path_mtu_on_losses(picoquic_cnx_t *, picoquic_packet_t *, int)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/loss_recovery.c:picoquic_is_packet_probably_lost`
* C source: `picoquic/loss_recovery.c:535-644`
* C signature: `int picoquic_is_packet_probably_lost(picoquic_cnx_t *, picoquic_packet_t *, uint64_t, uint64_t *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/loss_recovery.c:picoquic_process_lost_packet`
* C source: `picoquic/loss_recovery.c:784-871`
* C signature: `picoquic_packet_t * picoquic_process_lost_packet(picoquic_cnx_t *, picoquic_packet_context_t *, picoquic_packet_t *, int, picoquic_packet_context_enum, picoquic_path_t *, uint64_t, picoquic_packet_t *, size_t, size_t *, int *, size_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/loss_recovery.c:picoquic_retransmit_needed_packet`
* C source: `picoquic/loss_recovery.c:297-533`
* C signature: `size_t picoquic_retransmit_needed_packet(picoquic_cnx_t *, picoquic_packet_context_t *, picoquic_packet_t *, picoquic_packet_context_enum, picoquic_path_t *, uint64_t, uint64_t *, picoquic_packet_t *, size_t, size_t *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/loss_recovery.c:picoquic_retransmit_path_packet_queue`
* C source: `picoquic/loss_recovery.c:969-1008`
* C signature: `void picoquic_retransmit_path_packet_queue(picoquic_cnx_t *, picoquic_packet_context_t *, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
