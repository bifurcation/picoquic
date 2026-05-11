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

After implementing, run the relevant narrow tests if known, then:

```sh
cargo fmt
CARGO_INCREMENTAL=0 cargo test --no-run
CARGO_INCREMENTAL=0 cargo clippy --tests --all-features -- -D warnings
```

## `picoquic/bbr.c:BBREnterStartup`
* C source: `picoquic/bbr.c:2061-2067`
* C signature: `void BBREnterStartup(picoquic_bbr_state_t *, picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/bbr.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr.c:BBREnterStartupLongRTT`
* C source: `picoquic/bbr.c:2080-2101`
* C signature: `void BBREnterStartupLongRTT(picoquic_bbr_state_t *, picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/bbr.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr.c:BBREnterStartupResume`
* C source: `picoquic/bbr.c:1972-1980`
* C signature: `void BBREnterStartupResume(picoquic_bbr_state_t *)`
* Approved Rust destination: `rs/fq/src/bbr.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr.c:BBRExitLostFeedback`
* C source: `picoquic/bbr.c:782-788`
* C signature: `void BBRExitLostFeedback(picoquic_bbr_state_t *, picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/bbr.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/bbr.c:BBRHasElapsedInPhase`
* C source: `picoquic/bbr.c:1775-1778`
* C signature: `int BBRHasElapsedInPhase(picoquic_bbr_state_t *, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/bbr.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
