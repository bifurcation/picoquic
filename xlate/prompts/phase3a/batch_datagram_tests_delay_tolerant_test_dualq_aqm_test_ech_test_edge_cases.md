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

### `datagram_tests` (11 test(s))
   * C source: `picoquictest/datagram_tests.c`
   * Rust target: `rs/fq/src/tests/datagram.rs`
   * Entries:
     - `#[test] fn datagram()` ← C `datagram_test`
     - `#[test] fn datagram_rt()` ← C `datagram_rt_test`
     - `#[test] fn datagram_rt_skip()` ← C `datagram_rt_skip_test`
     - `#[test] fn datagram_rtnew_skip()` ← C `datagram_rtnew_skip_test`
     - `#[test] fn datagram_loss()` ← C `datagram_loss_test`
     - `#[test] fn datagram_size()` ← C `datagram_size_test`
     - `#[test] fn datagram_small()` ← C `datagram_small_test`
     - `#[test] fn datagram_small_new()` ← C `datagram_small_new_test`
     - `#[test] fn datagram_small_packet()` ← C `datagram_small_packet_test`
     - `#[test] fn datagram_too_long_test()` ← C `datagram_too_long_test`
     - `#[test] fn datagram_wifi()` ← C `datagram_wifi_test`

### `delay_tolerant_test` (4 test(s))
   * C source: `picoquictest/delay_tolerant_test.c`
   * Rust target: `rs/fq/src/tests/delay_tolerant.rs`
   * Entries:
     - `#[test] fn dtn_basic()` ← C `dtn_basic_test`
     - `#[test] fn dtn_data()` ← C `dtn_data_test`
     - `#[test] fn dtn_silence()` ← C `dtn_silence_test`
     - `#[test] fn dtn_twenty()` ← C `dtn_twenty_test`

### `dualq_aqm_test` (1 test(s))
   * C source: `picoquictest/dualq_aqm_test.c`
   * Rust target: `rs/fq/src/tests/dualq_aqm.rs`
   * Entries:
     - `#[test] fn dualq_aqm()` ← C `dualq_aqm_test`

### `ech_test` (6 test(s))
   * C source: `picoquictest/ech_test.c`
   * Rust target: `rs/fq/src/tests/ech.rs`
   * Entries:
     - `#[test] fn ech_config()` ← C `ech_config_test`
     - `#[test] fn ech_config_p()` ← C `ech_config_p_test`
     - `#[test] fn ech_e2e()` ← C `ech_e2e_test`
     - `#[test] fn ech_e2e_0rtt()` ← C `ech_e2e_0rtt_test`
     - `#[test] fn ech_grease()` ← C `ech_grease_test`
     - `#[test] fn ech_no_ech()` ← C `ech_no_ech_test`

### `edge_cases` (25 test(s))
   * C source: `picoquictest/edge_cases.c`
   * Rust target: `rs/fq/src/tests/edge_cases.rs`
   * Entries:
     - `#[test] fn error_name()` ← C `error_name_test`
     - `#[test] fn ec00_zero()` ← C `ec00_zero_test`
     - `#[test] fn ec2f_second_flight()` ← C `ec2f_second_flight_nack_test`
     - `#[test] fn eccf_corrupted_fuzz()` ← C `eccf_corrupted_file_fuzz_test`
     - `#[test] fn eca1_amplification_loss()` ← C `eca1_amplification_loss_test`
     - `#[test] fn ecf1_final_loss()` ← C `ecf1_final_loss_test`
     - `#[test] fn ec5c_silly_cid()` ← C `ec5c_silly_cid_test`
     - `#[test] fn ec9a_preemptive_amok()` ← C `ec9a_preemptive_amok_test`
     - `#[test] fn idle_server()` ← C `idle_server_test`
     - `#[test] fn idle_timeout()` ← C `idle_timeout_test`
     - `#[test] fn reset_ack_max()` ← C `reset_ack_max_test`
     - `#[test] fn reset_ack_reset()` ← C `reset_ack_reset_test`
     - `#[test] fn reset_extra_max()` ← C `reset_extra_max_test`
     - `#[test] fn reset_extra_reset()` ← C `reset_extra_reset_test`
     - `#[test] fn reset_extra_stop()` ← C `reset_extra_stop_test`
     - `#[test] fn reset_need_max()` ← C `reset_need_max_test`
     - `#[test] fn reset_need_reset()` ← C `reset_need_reset_test`
     - `#[test] fn reset_need_stop()` ← C `reset_need_stop_test`
     - `#[test] fn reset_loop_test()` ← C `reset_loop_test`
     - `#[test] fn reset_stream_at_basic()` ← C `reset_stream_at_basic_test`
     - `#[test] fn reset_stream_at_limit_test()` ← C `reset_stream_at_limit_test`
     - `#[test] fn reset_stream_at_loss()` ← C `reset_stream_at_loss_test`
     - `#[test] fn initial_pto()` ← C `initial_pto_test`
     - `#[test] fn initial_pto_srv()` ← C `initial_pto_srv_test`
     - `#[test] fn crypto_hs_offset()` ← C `crypto_hs_offset_test`

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
    python3 scripts/phase3_check.py datagram_tests delay_tolerant_test dualq_aqm_test ech_test edge_cases
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
