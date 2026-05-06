# Phase 3A batch translation: 8 source files

You're translating C test bodies in `picoquictest/` into Rust
`#[test]` bodies under `rs/fq/src/tests/`.  This is a **batch**
invocation — handle every source listed below in one session,
amortising the read of the translation guide / API surface
and the additions to `rs/fq/src/tests/util.rs`.

## What this is

**Test-driven development.**  Tests don't have to *pass* yet.
They must be *expressed* against the Rust API as it is
designed today.  A test that compiles and panics inside
`Quic::new` (or any `todo!()` API) on its first call is a
clean fail.  Do **not** sidestep work by leaving the body as
`todo!()` — translate the C body faithfully and let the panic
happen wherever it naturally does.

## Required reading (in order, once for the whole batch)

1. **`xlate/test_translation_guide.md`** — focused summary of
   the Rust API surface, naming conventions, helper inventory,
   and translation patterns.  Substitutes for grepping
   `lib.rs` / `internal.rs`.
2. `rs/fq/src/tests/util.rs` — test infrastructure; add helpers
   here when more than one source needs them, then re-use
   across this batch.  Keep one canonical name per helper.
3. Module sources (`rs/fq/src/<X>.rs`) only when you need to
   verify a specific signature the guide didn't quote.

Do **not** re-read the guide or `util.rs` once per source —
read each once and use what you remember across sources.

## Sources to translate (this batch)

### `parseheadertest` (4 test(s))
   * C source: `picoquictest/parseheadertest.c`
   * Rust target: `rs/fq/src/tests/parseheadertest.rs`
   * Entries:
     - `#[test] fn parseheader()` ← C `parseheadertest`
     - `#[test] fn incoming_initial()` ← C `incoming_initial_test`
     - `#[test] fn header_length()` ← C `header_length_test`
     - `#[test] fn packet_enc_dec()` ← C `packet_enc_dec_test`

### `picolog_test` (1 test(s))
   * C source: `picoquictest/picolog_test.c`
   * Rust target: `rs/fq/src/tests/picolog.rs`
   * Entries:
     - `#[test] fn picolog_basic()` ← C `picolog_basic_test`

### `picoquic_lb_test` (2 test(s))
   * C source: `picoquictest/picoquic_lb_test.c`
   * Rust target: `rs/fq/src/tests/picoquic_lb.rs`
   * Entries:
     - `#[test] fn cid_for_lb()` ← C `cid_for_lb_test`
     - `#[test] fn cid_for_lb_cli()` ← C `cid_for_lb_cli_test`

### `pn2pn64test` (1 test(s))
   * C source: `picoquictest/pn2pn64test.c`
   * Rust target: `rs/fq/src/tests/pn2pn64test.rs`
   * Entries:
     - `#[test] fn pn2pn64()` ← C `pn2pn64test`

### `qlog_frame_test` (1 test(s))
   * C source: `picoquictest/qlog_frame_test.c`
   * Rust target: `rs/fq/src/tests/qlog_frame.rs`
   * Entries:
     - `#[test] fn qlog_frames()` ← C `qlog_frames_test`

### `qlog_test` (2 test(s))
   * C source: `picoquictest/qlog_test.c`
   * Rust target: `rs/fq/src/tests/qlog.rs`
   * Entries:
     - `#[test] fn qlog_auto()` ← C `qlog_auto_test`
     - `#[test] fn qlog_error()` ← C `qlog_error_test`

### `quic_tester` (2 test(s))
   * C source: `picoquictest/quic_tester.c`
   * Rust target: `rs/fq/src/tests/quic_tester.rs`
   * Entries:
     - `#[test] fn initial_ping()` ← C `initial_ping_test`
     - `#[test] fn initial_ping_ack()` ← C `initial_ping_ack_test`

### `sacktest` (6 test(s))
   * C source: `picoquictest/sacktest.c`
   * Rust target: `rs/fq/src/tests/sacktest.rs`
   * Entries:
     - `#[test] fn ack_sack()` ← C `sacktest`
     - `#[test] fn ack_send()` ← C `sendacktest`
     - `#[test] fn ack_loop()` ← C `sendack_loop_test`
     - `#[test] fn ack_range()` ← C `ackrange_test`
     - `#[test] fn ack_disorder()` ← C `ack_disorder_test`
     - `#[test] fn ack_horizon()` ← C `ack_horizon_test`

## Translation rules

- **Faithful, idiomatic.**  Walk the C body and write the
  equivalent Rust.  Use Rust idioms (`assert_eq!`, `Vec`,
  `Option`, iterators, `?`-propagation).
- **Use the Rust public API as designed.**  If a method
  doesn't exist yet, **add it as a `todo!()` stub** to the
  appropriate source (`rs/fq/src/internal.rs` / `lib.rs`)
  with a `/// C: `picoquic_xxx`` doc-comment.  Match the
  C signature translated to Rust types.  This is the only
  authorized non-test edit.
- **Helpers go in `tests/util.rs`.**  Port `tls_api_init_ctx*`,
  `picoquic_test_set_minimal_cnx`, etc. once and re-use.
- **Golden / fixture files** — copy from `picoquictest/` to
  `rs/fq/tests/fixtures/` and reach via
  `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/<name>")`.
- **No placeholder bodies.**  Every test gets a translated body.
- **No `unsafe`.**  No edits outside `rs/fq/src/tests/`,
  `rs/fq/tests/fixtures/`, or `todo!()` stubs in
  `rs/fq/src/{internal,lib}.rs`.

## Stop spinning

If 5 grep/read tool calls into the same file haven't found
what you need, **stop searching**.  The API doesn't exist.
Add it as a `todo!()` stub and move on.  Aim to start writing
edits within the first 10 tool calls of EACH source.

## Verification

After translating each source, run:
    python3 scripts/phase3_check.py <source_basename>
to confirm every `#[test]` in that source has a non-stub body.
If the check reports stubs, fix them before moving on.

After the whole batch, run:
    python3 scripts/phase3_check.py parseheadertest picolog_test picoquic_lb_test pn2pn64test qlog_frame_test qlog_test quic_tester sacktest
    cd rs/fq && cargo fmt
    cd rs/fq && cargo test --no-run
    cd rs/fq && cargo clippy --tests --all-features -- -D warnings
All four must succeed.  Iterate until they do.

## Process

1. Read the translation guide and `tests/util.rs` once.
2. For each source in the batch:
   a. Read the C source and the Rust target.
   b. Translate every entry (use existing util.rs helpers, add
      new ones when shared across sources).
   c. Run `python3 scripts/phase3_check.py <src>`; iterate.
3. Run the gate (fmt + test --no-run + clippy) once at the end.
4. Report on stdout: a one-paragraph summary per source.
