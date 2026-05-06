# Phase 3A batch translation: 5 source files

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

### `l4s_test` (6 test(s))
   * C source: `picoquictest/l4s_test.c`
   * Rust target: `rs/fq/src/tests/l4s.rs`
   * Entries:
     - `#[test] fn l4s_reno()` ← C `l4s_reno_test`
     - `#[test] fn l4s_prague()` ← C `l4s_prague_test`
     - `#[test] fn l4s_prague_updown()` ← C `l4s_prague_updown_test`
     - `#[test] fn l4s_bbr()` ← C `l4s_bbr_test`
     - `#[test] fn l4s_c4()` ← C `l4s_c4_test`
     - `#[test] fn l4s_bbr_updown()` ← C `l4s_bbr_updown_test`

### `mbedtls_test` (7 test(s))
   * C source: `picoquictest/mbedtls_test.c`
   * Rust target: `rs/fq/src/tests/mbedtls.rs`
   * Entries:
     - `#[test] fn mbedtls()` ← C `mbedtls_test`
     - `#[test] fn mbedtls_crypto()` ← C `mbedtls_crypto_test`
     - `#[test] fn mbedtls_load_key()` ← C `mbedtls_load_key_test`
     - `#[test] fn mbedtls_load_key_fail()` ← C `mbedtls_load_key_fail_test`
     - `#[test] fn mbedtls_retrieve_pubkey()` ← C `mbedtls_retrieve_pubkey_test`
     - `#[test] fn mbedtls_sign_verify()` ← C `mbedtls_sign_verify_test`
     - `#[test] fn mbedtls_configure()` ← C `mbedtls_configure_test`

### `mediatest` (11 test(s))
   * C source: `picoquictest/mediatest.c`
   * Rust target: `rs/fq/src/tests/mediatest.rs`
   * Entries:
     - `#[test] fn mediatest_video()` ← C `mediatest_video_test`
     - `#[test] fn mediatest_video_audio()` ← C `mediatest_video_audio_test`
     - `#[test] fn mediatest_video_data_audio()` ← C `mediatest_video_data_audio_test`
     - `#[test] fn mediatest_video2_down()` ← C `mediatest_video2_down_test`
     - `#[test] fn mediatest_video2_back()` ← C `mediatest_video2_back_test`
     - `#[test] fn mediatest_video2_probe()` ← C `mediatest_video2_probe_test`
     - `#[test] fn mediatest_wifi()` ← C `mediatest_wifi_test`
     - `#[test] fn mediatest_worst()` ← C `mediatest_worst_test`
     - `#[test] fn mediatest_no_coal()` ← C `mediatest_no_coal_test`
     - `#[test] fn mediatest_suspension()` ← C `mediatest_suspension_test`
     - `#[test] fn mediatest_suspension2()` ← C `mediatest_suspension2_test`

### `memlog_test` (1 test(s))
   * C source: `picoquictest/memlog_test.c`
   * Rust target: `rs/fq/src/tests/memlog.rs`
   * Entries:
     - `#[test] fn memlog()` ← C `memlog_test`

### `minicrypto_test` (2 test(s))
   * C source: `picoquictest/minicrypto_test.c`
   * Rust target: `rs/fq/src/tests/minicrypto.rs`
   * Entries:
     - `#[test] fn minicrypto()` ← C `minicrypto_test`
     - `#[test] fn minicrypto_is_last()` ← C `minicrypto_is_last_test`

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
    python3 scripts/phase3_check.py l4s_test mbedtls_test mediatest memlog_test minicrypto_test
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
