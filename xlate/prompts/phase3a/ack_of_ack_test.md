# Phase 3A test-body translation: `ack_of_ack_test`

You're translating the C test bodies in `picoquictest/`
into Rust `#[test]` bodies in `rs/fq/src/tests/ack_of_ack.rs`.

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

## Required reading

1. The C source: `picoquictest/ack_of_ack_test.c`.
2. The Rust target: `rs/fq/src/tests/ack_of_ack.rs` — currently has
   auto-generated stubs (`todo!("<entry_fn>")`) that you
   replace with translations.
3. The Rust API surface you'll be calling against:
   - `rs/fq/src/lib.rs` — public crate API (Quic / Connection
     constructors, callbacks, error codes).
   - `rs/fq/src/internal.rs` — internal types (Path,
     PacketHeader, sack lists, frame helpers, etc.).
   - `rs/fq/src/tls.rs` — TLS trait surface.
   - `rs/fq/src/tests/util.rs` — test infrastructure (sim
     link, deterministic RNG, certificate paths).  Add
     helpers here if multiple translations need them.
   - `rs/fq/src/tests/dualq.rs` — DualQ AQM test infra.
   - Other `rs/fq/src/<X>.rs` modules per the test's topic
     (e.g. `bytestream.rs`, `splay.rs`, `errors.rs`,
     `frames.rs`, `tp.rs`, `stream.rs`, `lb.rs`).
4. `TRANSLATE_PLAN.md` — Phase 3 section.
5. `CLAUDE.md` — project conventions.

## Test entries to translate

   - `#[test] fn ack_of_ack()` ← C `ack_of_ack_test`

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

1. Read the C source and the existing `rs/fq/src/tests/ack_of_ack.rs`.
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
  (for fixtures) only.  You may also edit
  `rs/fq/src/tests/util.rs` to add helpers.
- Do **not** edit any non-test source under `rs/fq/src/`.
  If a translated test needs an API that doesn't yet exist,
  call the intended method anyway — the resulting compile
  error or `todo!()` panic is the spec for Phase 4.
  (If you genuinely cannot express the test without adding
  a method, surface it on stdout and skip that entry — the
  human reviewer adjusts the API on the next iteration.)
- Do **not** modify `Cargo.toml` or anything outside `rs/fq/`.
- Do **not** introduce `unsafe`.
