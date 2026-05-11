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

## `picoquic/ech.c:picoquic_ech_create_config_from_private_key`
* C source: `picoquic/ech.c:879-925`
* C signature: `int picoquic_ech_create_config_from_private_key(uint8_t **, size_t *, const char *, const char *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: complete existing Rust function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: mapped Rust function still contains incomplete markers

## `picoquic/ech.c:picoquic_ech_create_config_from_public_key`
* C source: `picoquic/ech.c:857-876`
* C signature: `int picoquic_ech_create_config_from_public_key(uint8_t **, size_t *, const char *, const char *)`
* Approved Rust destination: `rs/fq/src/lib.rs`
* Item shape: complete existing Rust function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: mapped Rust function still contains incomplete markers

## `picoquic/ech.c:picoquic_ech_create_config_list_from_config`
* C source: `picoquic/ech.c:761-800`
* C signature: `int picoquic_ech_create_config_list_from_config(ptls_buffer_t *, uint8_t **, size_t *)`
* Approved Rust destination: `rs/fq/src/ech.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/ech.c:picoquic_ech_create_rr_from_binary`
* C source: `picoquic/ech.c:827-842`
* C signature: `int picoquic_ech_create_rr_from_binary(ptls_buffer_t *, ptls_iovec_t, const char *)`
* Approved Rust destination: `rs/fq/src/ech.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/ech.c:picoquic_ech_parse_public_key`
* C source: `picoquic/ech.c:601-653`
* C signature: `int picoquic_ech_parse_public_key(ptls_iovec_t, uint16_t *, ptls_iovec_t *)`
* Approved Rust destination: `rs/fq/src/ech.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
