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

## `picoquic/quicctx.c:picoquic_wake_list_init`
* C source: `picoquic/quicctx.c:1499-1503`
* C signature: `void picoquic_wake_list_init(picoquic_quic_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_sack_find_range_below_number`
* C source: `picoquic/sacks.c:156-168`
* C signature: `picoquic_sack_item_t * picoquic_sack_find_range_below_number(picoquic_sack_list_t *, picoquic_sack_item_t *, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_update_sack_list`
* C source: `picoquic/sacks.c:197-256`
* C signature: `int picoquic_update_sack_list(picoquic_sack_list_t *, uint64_t, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_check_idle_timer`
* C source: `picoquic/sender.c:3719-3759`
* C signature: `int picoquic_check_idle_timer(picoquic_cnx_t *, uint64_t *, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_format_new_local_id_as_needed`
* C source: `picoquic/sender.c:2596-2658`
* C signature: `uint8_t * picoquic_format_new_local_id_as_needed(picoquic_cnx_t *, uint8_t *, uint8_t *, uint64_t, uint64_t *, int *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_handle_send_timers`
* C source: `picoquic/sender.c:3905-3932`
* C signature: `int picoquic_handle_send_timers(picoquic_cnx_t *, uint64_t, uint64_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_handle_send_train_statistics`
* C source: `picoquic/sender.c:3959-3979`
* C signature: `void picoquic_handle_send_train_statistics(picoquic_cnx_t *, picoquic_path_t *, size_t, size_t *, size_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_insert_hole_in_send_sequence_if_needed`
* C source: `picoquic/sender.c:1133-1169`
* C signature: `void picoquic_insert_hole_in_send_sequence_if_needed(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_context_t *, uint64_t, uint64_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_is_mtu_probe_needed`
* C source: `picoquic/sender.c:1587-1628`
* C signature: `picoquic_pmtu_discovery_status_enum picoquic_is_mtu_probe_needed(picoquic_cnx_t *, picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_preemptive_retransmit_packet`
* C source: `picoquic/sender.c:1332-1419`
* C signature: `int picoquic_preemptive_retransmit_packet(picoquic_packet_t *, picoquic_cnx_t *, uint8_t *, size_t, size_t *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_mtu_probe`
* C source: `picoquic/sender.c:1630-1647`
* C signature: `size_t picoquic_prepare_mtu_probe(picoquic_cnx_t *, picoquic_path_t *, size_t, size_t, uint8_t *, size_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_next_packet_ex`
* C source: `picoquic/sender.c:4244-4324`
* C signature: `int picoquic_prepare_next_packet_ex(picoquic_quic_t *, uint64_t, uint8_t *, size_t, size_t *, struct sockaddr_storage *, struct sockaddr_storage *, int *, picoquic_connection_id_t *, picoquic_cnx_t **, size_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: complete existing Rust function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: mapped Rust function still contains incomplete markers

## `picoquic/sender.c:picoquic_prepare_packet_0rtt`
* C source: `picoquic/sender.c:1649-1743`
* C signature: `int picoquic_prepare_packet_0rtt(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, int, uint64_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_packet_almost_ready`
* C source: `picoquic/sender.c:2964-3284`
* C signature: `int picoquic_prepare_packet_almost_ready(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_packet_client_init`
* C source: `picoquic/sender.c:1932-2213`
* C signature: `int picoquic_prepare_packet_client_init(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_packet_closing`
* C source: `picoquic/sender.c:2351-2594`
* C signature: `int picoquic_prepare_packet_closing(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_packet_ex`
* C source: `picoquic/sender.c:3981-4181`
* C signature: `int picoquic_prepare_packet_ex(picoquic_cnx_t *, uint64_t, uint8_t *, size_t, size_t *, struct sockaddr_storage *, struct sockaddr_storage *, int *, size_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: complete existing Rust function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: mapped Rust function still contains incomplete markers

## `picoquic/sender.c:picoquic_prepare_packet_ready`
* C source: `picoquic/sender.c:3286-3717`
* C signature: `int picoquic_prepare_packet_ready(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_packet_server_init`
* C source: `picoquic/sender.c:2215-2349`
* C signature: `int picoquic_prepare_packet_server_init(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_segment`
* C source: `picoquic/sender.c:3761-3829`
* C signature: `int picoquic_prepare_segment(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
