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

## `picoquic/picosplay.c:zig`
* C source: `picoquic/picosplay.c:59-62`
* C signature: `void zig(picosplay_node_t *)`
* Approved Rust destination: `rs/fq/src/splay.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/picosplay.c:zigzag`
* C source: `picoquic/picosplay.c:72-78`
* C signature: `void zigzag(picosplay_node_t *)`
* Approved Rust destination: `rs/fq/src/splay.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/picosplay.c:zigzig`
* C source: `picoquic/picosplay.c:64-70`
* C signature: `void zigzig(picosplay_node_t *, picosplay_node_t *)`
* Approved Rust destination: `rs/fq/src/splay.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_clear_path_data`
* C source: `picoquic/quicctx.c:1885-1899`
* C signature: `void picoquic_clear_path_data(picoquic_cnx_t *, picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_dereference_stashed_cnxid`
* C source: `picoquic/quicctx.c:3149-3152`
* C signature: `void picoquic_dereference_stashed_cnxid(picoquic_cnx_t *, picoquic_path_t *, int)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_remove_cnx_from_list`
* C source: `picoquic/quicctx.c:1450-1469`
* C signature: `void picoquic_remove_cnx_from_list(picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_remove_cnx_from_wake_list`
* C source: `picoquic/quicctx.c:1505-1508`
* C signature: `void picoquic_remove_cnx_from_wake_list(picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_check_sack_list`
* C source: `picoquic/sacks.c:323-340`
* C signature: `int picoquic_check_sack_list(picoquic_sack_list_t *, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_sack_first_item`
* C source: `picoquic/sacks.c:68-72`
* C signature: `picoquic_sack_item_t * picoquic_sack_first_item(picoquic_sack_list_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_sack_last_item`
* C source: `picoquic/sacks.c:74-77`
* C signature: `picoquic_sack_item_t * picoquic_sack_last_item(picoquic_sack_list_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sacks.c:picoquic_sack_list_first_range`
* C source: `picoquic/sacks.c:422-428`
* C signature: `picoquic_sack_item_t * picoquic_sack_list_first_range(picoquic_sack_list_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_close_ex`
* C source: `picoquic/sender.c:4201-4222`
* C signature: `int picoquic_close_ex(picoquic_cnx_t *, uint64_t, const char *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_handle_send_paths`
* C source: `picoquic/sender.c:3934-3957`
* C signature: `void picoquic_handle_send_paths(picoquic_cnx_t *, uint64_t, uint64_t *, picoquic_path_t **, picoquic_tuple_t **, struct sockaddr_storage *, struct sockaddr_storage *, int *, size_t, size_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_is_pkt_ctx_backlog_empty`
* C source: `picoquic/sender.c:1274-1308`
* C signature: `int picoquic_is_pkt_ctx_backlog_empty(picoquic_packet_context_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_preemptive_retransmit_in_context`
* C source: `picoquic/sender.c:1421-1492`
* C signature: `int picoquic_preemptive_retransmit_in_context(picoquic_cnx_t *, picoquic_packet_context_t *, uint64_t, uint64_t, uint64_t *, uint8_t *, size_t, size_t *, int *, int *, int)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_datagram_ready`
* C source: `picoquic/sender.c:2777-2801`
* C signature: `uint8_t * picoquic_prepare_datagram_ready(picoquic_cnx_t *, picoquic_path_t *, uint8_t *, uint8_t *, int, int *, int *, int *, int *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_packet_old_context`
* C source: `picoquic/sender.c:1771-1833`
* C signature: `size_t picoquic_prepare_packet_old_context(picoquic_cnx_t *, picoquic_packet_context_enum, picoquic_path_t *, picoquic_packet_t *, size_t, uint64_t, uint64_t *, size_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_prepare_server_address_migration`
* C source: `picoquic/sender.c:1868-1930`
* C signature: `int picoquic_prepare_server_address_migration(picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sender.c:picoquic_set_path_addresses_from_tuple`
* C source: `picoquic/sender.c:3832-3846`
* C signature: `void picoquic_set_path_addresses_from_tuple(picoquic_tuple_t *, struct sockaddr_storage *, struct sockaddr_storage *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sim_link.c:picoquictest_sim_link_jitter`
* C source: `picoquic/sim_link.c:230-247`
* C signature: `uint64_t picoquictest_sim_link_jitter(picoquictest_sim_link_t *)`
* Approved Rust destination: `rs/fq/src/tests/harness.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
