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

## `picoquic/bbr.c:BBRInflight`
* C source: `picoquic/bbr.c:909-912`
* C signature: `uint64_t BBRInflight(picoquic_bbr_state_t *, picoquic_path_t *, double)`
* Approved Rust destination: `rs/fq/src/bbr.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr.c:picoquic_bbr_init`
* C source: `picoquic/bbr.c:603-612`
* C signature: `void picoquic_bbr_init(picoquic_path_t *, const char *, uint64_t)`
* Approved Rust destination: `rs/fq/src/bbr.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr.c:picoquic_bbr_reset`
* C source: `picoquic/bbr.c:598-601`
* C signature: `void picoquic_bbr_reset(picoquic_bbr_state_t *, picoquic_path_t *, uint64_t)`
* Approved Rust destination: `rs/fq/src/bbr.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr1.c:BBR1HandleRestartFromIdle`
* C source: `picoquic/bbr1.c:1057-1066`
* C signature: `void BBR1HandleRestartFromIdle(picoquic_bbr1_state_t *, uint64_t, int)`
* Approved Rust destination: `rs/fq/src/bbr1.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr1.c:BBR1OnAllPacketsLost`
* C source: `picoquic/bbr1.c:1091-1095`
* C signature: `void BBR1OnAllPacketsLost(picoquic_bbr1_state_t *, picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/bbr1.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr1.c:BBR1OnEnterFastRecovery`
* C source: `picoquic/bbr1.c:1097-1105`
* C signature: `void BBR1OnEnterFastRecovery(picoquic_bbr1_state_t *, picoquic_path_t *, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/bbr1.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr1.c:picoquic_bbr1_init`
* C source: `picoquic/bbr1.c:469-479`
* C signature: `void picoquic_bbr1_init(picoquic_path_t *, const char *, uint64_t)`
* Approved Rust destination: `rs/fq/src/bbr1.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/config.c:picoquic_get_command_line_option_value`
* C source: `picoquic/config.c:683-720`
* C signature: `int picoquic_get_command_line_option_value(int, const char *, int *, const char **, int, const char *, picoquic_quic_config_t *)`
* Approved Rust destination: `rs/fq/src/config.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/ech.c:picoquic_ech_configure_quic_ctx`
* C source: `picoquic/ech.c:333-366`
* C signature: `int picoquic_ech_configure_quic_ctx(picoquic_quic_t *, const char *, const char *)`
* Approved Rust destination: `rs/fq/src/ech.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_apply_reset_stream_frame`
* C source: `picoquic/frames.c:330-371`
* C signature: `const uint8_t * picoquic_apply_reset_stream_frame(picoquic_cnx_t *, const uint8_t *, uint64_t, uint64_t, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_copy_single_stream_frame_for_retransmit`
* C source: `picoquic/frames.c:2399-2460`
* C signature: `uint8_t * picoquic_copy_single_stream_frame_for_retransmit(picoquic_cnx_t *, picoquic_packet_t *, uint8_t *, uint8_t *, int *, int *, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_estimate_path_bandwidth`
* C source: `picoquic/frames.c:2865-2922`
* C signature: `void picoquic_estimate_path_bandwidth(picoquic_cnx_t *, picoquic_path_t *, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, int)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_queue_data_repeat_adjust`
* C source: `picoquic/frames.c:2203-2252`
* C signature: `int picoquic_queue_data_repeat_adjust(picoquic_cnx_t *, picoquic_packet_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_queue_max_path_id_frame`
* C source: `picoquic/frames.c:6012-6026`
* C signature: `int picoquic_queue_max_path_id_frame(picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_queue_path_cid_blocked_frame`
* C source: `picoquic/frames.c:6257-6276`
* C signature: `int picoquic_queue_path_cid_blocked_frame(picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/frames.c:picoquic_queue_paths_blocked_frame`
* C source: `picoquic/frames.c:6128-6142`
* C signature: `int picoquic_queue_paths_blocked_frame(picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/logwriter.c:binlog_app_message`
* C source: `picoquic/logwriter.c:1300-1307`
* C signature: `void binlog_app_message(picoquic_cnx_t *, const char *, va_list)`
* Approved Rust destination: `rs/fq/src/binlog.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/packet.c:picoquic_remove_header_protection`
* C source: `picoquic/packet.c:617-631`
* C signature: `int picoquic_remove_header_protection(picoquic_cnx_t *, uint8_t *, uint8_t *, picoquic_packet_header *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_verify_stream_cipher`
* C source: `picoquic/picoquic_lb.c:163-186`
* C signature: `uint64_t picoquic_lb_compat_cid_verify_stream_cipher(picoquic_load_balancer_cid_context_t *, const picoquic_connection_id_t *)`
* Approved Rust destination: `rs/fq/src/lb.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/picosocks.c:picoquic_send_through_socket`
* C source: `picoquic/picosocks.c:1262-1271`
* C signature: `int picoquic_send_through_socket(int, struct sockaddr *, struct sockaddr *, int, const char *, int, int *)`
* Approved Rust destination: `rs/fq/src/socks.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
