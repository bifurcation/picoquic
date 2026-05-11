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

## `picoquic/spinbit.c:picoquic_spinbit_random_outgoing`
* C source: `picoquic/spinbit.c:72-76`
* C signature: `uint8_t picoquic_spinbit_random_outgoing(picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/spinbit.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_get_aes128gcm_sha256`
* C source: `picoquic/tls_api.c:709-713`
* C signature: `ptls_cipher_suite_t * picoquic_get_aes128gcm_sha256(int)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_get_cipher_suite_by_id_v`
* C source: `picoquic/tls_api.c:731-734`
* C signature: `void * picoquic_get_cipher_suite_by_id_v(int, int)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr1.c:BBR1OnTransmit`
* C source: `picoquic/bbr1.c:1083-1086`
* C signature: `void BBR1OnTransmit(picoquic_bbr1_state_t *, uint64_t, int)`
* Approved Rust destination: `rs/fq/src/bbr1.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/paths.c:picoquic_sort_available_paths`
* C source: `picoquic/paths.c:379-486`
* C signature: `void picoquic_sort_available_paths(picoquic_cnx_t *, uint64_t, uint64_t *, picoquic_path_t **, uint64_t, picoquic_tuple_t **)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_uniform_random`
* C source: `picoquic/quicctx.c:5600-5604`
* C signature: `uint64_t picoquic_uniform_random(uint64_t)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_sack_list_first`
* C source: `picoquic/sacks.c:407-412`
* C signature: `uint64_t picoquic_sack_list_first(picoquic_sack_list_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_sack_list_last`
* C source: `picoquic/sacks.c:414-420`
* C signature: `uint64_t picoquic_sack_list_last(picoquic_sack_list_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_preemptive_retransmit_as_needed`
* C source: `picoquic/sender.c:1494-1543`
* C signature: `int picoquic_preemptive_retransmit_as_needed(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_context_enum, uint64_t, uint64_t *, uint8_t *, size_t, size_t *, int *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_get_aes128gcm_v`
* C source: `picoquic/tls_api.c:720-729`
* C signature: `void * picoquic_get_aes128gcm_v(int)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_sack_list_reset`
* C source: `picoquic/sacks.c:439-447`
* C signature: `int picoquic_sack_list_reset(picoquic_sack_list_t *, uint64_t, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/transport.c:picoquic_clear_transport_extensions`
* C source: `picoquic/transport.c:503-532`
* C signature: `void picoquic_clear_transport_extensions(picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_insert_cnx_by_wake_time`
* C source: `picoquic/quicctx.c:1510-1513`
* C signature: `void picoquic_insert_cnx_by_wake_time(picoquic_quic_t *, picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_sack_insert_item`
* C source: `picoquic/sacks.c:89-108`
* C signature: `int picoquic_sack_insert_item(picoquic_sack_list_t *, uint64_t, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
