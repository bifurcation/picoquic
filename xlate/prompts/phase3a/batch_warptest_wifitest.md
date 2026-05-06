# Phase 3A batch translation: 2 source files

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

### `warptest` (5 test(s))
   * C source: `picoquictest/warptest.c`
   * Rust target: `rs/fq/src/tests/warptest.rs`
   * Entries:
     - `#[test] fn warptest_video()` ← C `warptest_video_test`
     - `#[test] fn warptest_video_audio()` ← C `warptest_video_audio_test`
     - `#[test] fn warptest_video_data_audio()` ← C `warptest_video_data_audio_test`
     - `#[test] fn warptest_worst()` ← C `warptest_worst_test`
     - `#[test] fn warptest_param()` ← C `warptest_param_test`

### `wifitest` (14 test(s))
   * C source: `picoquictest/wifitest.c`
   * Rust target: `rs/fq/src/tests/wifitest.rs`
   * Entries:
     - `#[test] fn wifi_bbr()` ← C `wifi_bbr_test`
     - `#[test] fn wifi_bbr_hard()` ← C `wifi_bbr_hard_test`
     - `#[test] fn wifi_bbr_long()` ← C `wifi_bbr_long_test`
     - `#[test] fn wifi_bbr_many()` ← C `wifi_bbr_many_test`
     - `#[test] fn wifi_bbr_shadow()` ← C `wifi_bbr_shadow_test`
     - `#[test] fn wifi_bbr1()` ← C `wifi_bbr1_test`
     - `#[test] fn wifi_bbr1_hard()` ← C `wifi_bbr1_hard_test`
     - `#[test] fn wifi_bbr1_long()` ← C `wifi_bbr1_long_test`
     - `#[test] fn wifi_cubic()` ← C `wifi_cubic_test`
     - `#[test] fn wifi_cubic_hard()` ← C `wifi_cubic_hard_test`
     - `#[test] fn wifi_cubic_long()` ← C `wifi_cubic_long_test`
     - `#[test] fn wifi_reno()` ← C `wifi_reno_test`
     - `#[test] fn wifi_reno_hard()` ← C `wifi_reno_hard_test`
     - `#[test] fn wifi_reno_long()` ← C `wifi_reno_long_test`

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
    python3 scripts/phase3_check.py warptest wifitest
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
