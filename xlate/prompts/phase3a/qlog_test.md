# Phase 3A test-body translation: `qlog_test`

You're translating the C test bodies in `picoquictest/`
into Rust `#[test]` bodies in `rs/fq/src/tests/qlog.rs`.

## What this is

**Test-driven development.**  The tests don't have to *pass*
yet.  They have to be *expressed* against the Rust API as it
is designed today (in `rs/fq/src/`).  Phase 4 fills in API
bodies; once it does, your tests light up.  A test that
compiles and panics inside `Quic::new` (or any other
currently-`todo!()` API) on its first call is a clean fail.
**That is fine.**  Do **not** sidestep work by leaving the
body as `todo!()` — translate the C body faithfully and let
the panic happen wherever it naturally does.

## Required reading (in order)

1. **`xlate/test_translation_guide.md`** — focused summary of
   the Rust API surface, naming conventions, helper
   inventory, and translation patterns.  Substitutes for
   grepping `lib.rs` / `internal.rs`; only fall back to
   reading those files when this guide doesn't have the
   answer.
2. The C source: `picoquictest/qlog_test.c`.
3. The Rust target: `rs/fq/src/tests/qlog.rs` — currently has
   auto-generated stubs (`todo!("<entry_fn>")`) that you
   replace with translations.
4. `rs/fq/src/tests/util.rs` — test infrastructure;
   add helpers here when multiple translations would need
   them.
5. Module sources (`rs/fq/src/<X>.rs`) only when you need to
   verify a specific signature the guide didn't quote.

## Test entries to translate

   - `#[test] fn qlog_auto()` ← C `qlog_auto_test`
   - `#[test] fn qlog_error()` ← C `qlog_error_test`

## Translation rules

- **Faithful, idiomatic.**  Walk the C body and write the
  equivalent Rust.  Use Rust idioms (`assert_eq!`, `Vec`,
  `Option`, iterators, `?`-propagation) — don't transcribe
  the C control flow with goto-style ret tracking.
- **Use the public API, not internal field access.**  C
  tests routinely poke `cnx->path[0]->first_tuple->...`;
  the Rust translation uses methods.  If the method doesn't
  exist yet, that's OK — call the *intended* method and let
  it panic; this documents the API Phase 4 must supply.
- **Helpers go in `tests/util.rs`.**  C uses
  `picoquic_test_set_minimal_cnx`, `tls_api_init_ctx*`,
  `picoquic_test_random*`, simulator helpers, etc.  Port
  the helper bodies into `tests/util.rs` once and have
  every translation that needs them call through.
- **Golden / fixture files** — if the C test reads a binlog
  reference or qlog template, copy the file to
  `rs/fq/tests/fixtures/` (creating the directory if needed)
  and reach it via `concat!(env!("CARGO_MANIFEST_DIR"),
  "/tests/fixtures/<name>")`.
- **Do not skip with placeholder bodies.**  `todo!("…")`
  in your test body is acceptable *only* when the C body
  itself is a no-op or a build-flag-gated stub (`#if 0`).
  Every other test gets a translated body.
- **Compile is the gate, not pass.**  After your edits run
  `cd rs/fq && cargo test --no-run` to verify the test
  compiles.  Don't run `cargo test` itself — most translated
  tests will fail with `todo!()` panics, which is expected.
- **`cargo fmt` and `cargo clippy --tests --all-features --
  -D warnings`** must also pass.

## Process

1. Read the C source and the existing `rs/fq/src/tests/qlog.rs`.
2. Read whichever Rust modules expose the API the C body
   exercises (use `Glob` / `Grep` to navigate).
3. If common helpers are needed (Quic context creation,
   simulator setup, certificate fixture paths), put them
   in `rs/fq/src/tests/util.rs`.
4. Translate every test in the entry list above.  Keep
   the existing module's `//!` doc-comment header (or
   write a better one) describing what this file covers.
5. Validate:
     cd rs/fq && cargo fmt
     cd rs/fq && cargo test --no-run
     cd rs/fq && cargo clippy --tests --all-features -- -D warnings
   Iterate until all three are clean.
6. Report on stdout: a one-paragraph summary naming each
   translated entry and any helpers added to `tests/util.rs`.

## Constraints

- Edit / write under `rs/fq/src/tests/` and `rs/fq/tests/`
  (for fixtures) only — **with one exception**: if a test
  body needs to call an API method that doesn't yet exist
  on a public Rust type (e.g. `Connection::record_pn_received`
  isn't there but the test calls it), **add the method as a
  `todo!()` stub** in the appropriate source file (typically
  `rs/fq/src/internal.rs` or `rs/fq/src/lib.rs`).  Match the
  shape of the C function being translated — same parameter
  list (translated to Rust types), `Result<…, Error>` for
  fallible C `int` returns, etc.  Add a doc comment `/// C:
  `picoquic_xxx``.  This grows the API surface incrementally
  as tests demand it; Phase 4 then fills the bodies.
- That's the **only** API-source change allowed.  Do not
  rename existing items, change existing signatures, or
  reshape existing types.  If an existing method has the
  wrong signature, document it on stdout and use a wrapper
  helper in `tests/util.rs`.
- Do **not** modify `Cargo.toml` or anything outside `rs/fq/`.
- Do **not** introduce `unsafe`.

## Stop spinning

If 5 grep/read tool calls into the same file haven't found
what you need, **stop searching**.  The API doesn't exist.
Add it as a `todo!()` stub per the rule above and move on.
Time spent re-grepping `internal.rs` is time not spent
translating.  Aim to start writing edits within the first
10 tool calls.
