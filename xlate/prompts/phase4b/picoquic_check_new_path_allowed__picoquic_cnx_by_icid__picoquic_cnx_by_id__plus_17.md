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

## `picoquic/quicctx.c:picoquic_check_new_path_allowed`
* C source: `picoquic/quicctx.c:2300-2342`
* C signature: `int picoquic_check_new_path_allowed(picoquic_cnx_t *, int)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_cnx_by_icid`
* C source: `picoquic/quicctx.c:5258-5275`
* C signature: `picoquic_cnx_t * picoquic_cnx_by_icid(picoquic_quic_t *, picoquic_connection_id_t *, const struct sockaddr *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_cnx_by_id`
* C source: `picoquic/quicctx.c:5216-5240`
* C signature: `picoquic_cnx_t * picoquic_cnx_by_id(picoquic_quic_t *, picoquic_connection_id_t, struct st_picoquic_local_cnxid_t **)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_cnx_by_net`
* C source: `picoquic/quicctx.c:5242-5256`
* C signature: `picoquic_cnx_t * picoquic_cnx_by_net(picoquic_quic_t *, const struct sockaddr *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_cnx_by_secret`
* C source: `picoquic/quicctx.c:5277-5292`
* C signature: `picoquic_cnx_t * picoquic_cnx_by_secret(picoquic_quic_t *, const uint8_t *, const struct sockaddr *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_create`
* C source: `picoquic/quicctx.c:633-775`
* C signature: `picoquic_quic_t * picoquic_create(uint32_t, const char *, const char *, const char *, const char *, picoquic_stream_data_cb_fn, void *, picoquic_connection_id_cb_fn, void *, uint8_t[16], uint64_t, uint64_t *, const char *, const uint8_t *, size_t)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: complete existing Rust function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: mapped Rust function still contains incomplete markers

## `picoquic/quicctx.c:picoquic_create_cnx_internal`
* C source: `picoquic/quicctx.c:4039-4348`
* C signature: `picoquic_cnx_t * picoquic_create_cnx_internal(picoquic_quic_t *, picoquic_connection_id_t, picoquic_connection_id_t, const struct sockaddr *, uint64_t, uint32_t, const char *, const char *, char, void *, void *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: complete existing Rust function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: mapped Rust function still contains incomplete markers

## `picoquic/quicctx.c:picoquic_create_local_cnxid`
* C source: `picoquic/quicctx.c:3795-3866`
* C signature: `picoquic_local_cnxid_t * picoquic_create_local_cnxid(picoquic_cnx_t *, uint64_t, picoquic_connection_id_t *, uint64_t)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_create_random_cnx_id`
* C source: `picoquic/quicctx.c:1630-1639`
* C signature: `void picoquic_create_random_cnx_id(picoquic_quic_t *, picoquic_connection_id_t *, uint8_t)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_find_local_cnxid`
* C source: `picoquic/quicctx.c:4017-4034`
* C signature: `picoquic_local_cnxid_t * picoquic_find_local_cnxid(picoquic_cnx_t *, uint64_t, picoquic_connection_id_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_get_path_quality_from_context`
* C source: `picoquic/quicctx.c:2678-2699`
* C signature: `void picoquic_get_path_quality_from_context(picoquic_path_t *, picoquic_path_quality_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_notify_destination_unreachable`
* C source: `picoquic/quicctx.c:2217-2244`
* C signature: `void picoquic_notify_destination_unreachable(picoquic_cnx_t *, uint64_t, struct sockaddr *, struct sockaddr *, int, int)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: complete existing Rust function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: mapped Rust function still contains incomplete markers

## `picoquic/quicctx.c:picoquic_register_net_id`
* C source: `picoquic/quicctx.c:1292-1310`
* C signature: `int picoquic_register_net_id(picoquic_quic_t *, picoquic_cnx_t *, picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_remove_stashed_cnxid`
* C source: `picoquic/quicctx.c:3093-3100`
* C signature: `picoquic_remote_cnxid_t * picoquic_remove_stashed_cnxid(picoquic_cnx_t *, uint64_t, picoquic_remote_cnxid_t *, picoquic_remote_cnxid_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_reset_cnx`
* C source: `picoquic/quicctx.c:4939-4989`
* C signature: `int picoquic_reset_cnx(picoquic_cnx_t *, uint64_t)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_retire_local_cnxid`
* C source: `picoquic/quicctx.c:3965-3985`
* C signature: `void picoquic_retire_local_cnxid(picoquic_cnx_t *, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_set_verify_certificate_callback`
* C source: `picoquic/quicctx.c:5486-5492`
* C signature: `void picoquic_set_verify_certificate_callback(picoquic_quic_t *, ptls_verify_certificate_t *, picoquic_free_verify_certificate_ctx)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_subscribe_to_quality_update_per_path_context`
* C source: `picoquic/quicctx.c:2721-2727`
* C signature: `void picoquic_subscribe_to_quality_update_per_path_context(picoquic_path_t *, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_unregister_net_icid`
* C source: `picoquic/quicctx.c:1356-1363`
* C signature: `void picoquic_unregister_net_icid(picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/quicctx.c:picoquic_unregister_net_id`
* C source: `picoquic/quicctx.c:1280-1290`
* C signature: `void picoquic_unregister_net_id(picoquic_cnx_t *, picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
