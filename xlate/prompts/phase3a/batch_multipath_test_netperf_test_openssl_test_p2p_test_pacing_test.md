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

### `multipath_test` (41 test(s))
   * C source: `picoquictest/multipath_test.c`
   * Rust target: `rs/fq/src/tests/multipath.rs`
   * Entries:
     - `#[test] fn migration_controlled()` ← C `migration_controlled_test`
     - `#[test] fn migration_mtu_drop()` ← C `migration_mtu_drop_test`
     - `#[test] fn monopath_basic()` ← C `monopath_basic_test`
     - `#[test] fn monopath_hole()` ← C `monopath_hole_test`
     - `#[test] fn monopath_keep_alive()` ← C `monopath_keep_alive_test`
     - `#[test] fn monopath_rotation()` ← C `monopath_rotation_test`
     - `#[test] fn monopath_0rtt()` ← C `monopath_0rtt_test`
     - `#[test] fn monopath_0rtt_loss()` ← C `monopath_0rtt_loss_test`
     - `#[test] fn multipath_aead()` ← C `multipath_aead_test`
     - `#[test] fn multipath_basic()` ← C `multipath_basic_test`
     - `#[test] fn multipath_drop_first()` ← C `multipath_drop_first_test`
     - `#[test] fn multipath_drop_second()` ← C `multipath_drop_second_test`
     - `#[test] fn multipath_fail()` ← C `multipath_fail_test`
     - `#[test] fn multipath_ab1()` ← C `multipath_ab1_test`
     - `#[test] fn multipath_sat_plus()` ← C `multipath_sat_plus_test`
     - `#[test] fn multipath_renew()` ← C `multipath_renew_test`
     - `#[test] fn multipath_rotation()` ← C `multipath_rotation_test`
     - `#[test] fn multipath_break1()` ← C `multipath_break1_test`
     - `#[test] fn multipath_socket_error()` ← C `multipath_socket_error_test`
     - `#[test] fn multipath_socket0_error()` ← C `multipath_socket0_error_test`
     - `#[test] fn multipath_abandon()` ← C `multipath_abandon_test`
     - `#[test] fn multipath_back0()` ← C `multipath_back0_test`
     - `#[test] fn multipath_back1()` ← C `multipath_back1_test`
     - `#[test] fn multipath_nat()` ← C `multipath_nat_test`
     - `#[test] fn multipath_nat_challenge()` ← C `multipath_nat_challenge_test`
     - `#[test] fn multipath_perf()` ← C `multipath_perf_test`
     - `#[test] fn multipath_callback()` ← C `multipath_callback_test`
     - `#[test] fn multipath_quality()` ← C `multipath_quality_test`
     - `#[test] fn multipath_stream_af()` ← C `multipath_stream_af_test`
     - `#[test] fn multipath_datagram()` ← C `multipath_datagram_test`
     - `#[test] fn multipath_dg_af()` ← C `multipath_dg_af_test`
     - `#[test] fn multipath_backup()` ← C `multipath_backup_test`
     - `#[test] fn multipath_standup()` ← C `multipath_standup_test`
     - `#[test] fn multipath_discovery()` ← C `multipath_discovery_test`
     - `#[test] fn multipath_keep_alive()` ← C `multipath_keep_alive_test`
     - `#[test] fn multipath_just_one()` ← C `multipath_just_one_test`
     - `#[test] fn multipath_break_both()` ← C `multipath_break_both_test`
     - `#[test] fn multipath_qlog()` ← C `multipath_qlog_test`
     - `#[test] fn multipath_tunnel()` ← C `multipath_tunnel_test`
     - `#[test] fn monopath_0rtt()` ← C `monopath_0rtt_test`
     - `#[test] fn monopath_0rtt_loss()` ← C `monopath_0rtt_loss_test`

### `netperf_test` (3 test(s))
   * C source: `picoquictest/netperf_test.c`
   * Rust target: `rs/fq/src/tests/netperf.rs`
   * Entries:
     - `#[test] fn netperf_basic()` ← C `netperf_basic_test`
     - `#[test] fn netperf_bbr()` ← C `netperf_bbr_test`
     - `#[test] fn nat_attack()` ← C `nat_attack_test`

### `openssl_test` (1 test(s))
   * C source: `picoquictest/openssl_test.c`
   * Rust target: `rs/fq/src/tests/openssl.rs`
   * Entries:
     - `#[test] fn openssl_cert()` ← C `openssl_cert_test`

### `p2p_test` (1 test(s))
   * C source: `picoquictest/p2p_test.c`
   * Rust target: `rs/fq/src/tests/p2p.rs`
   * Entries:
     - `#[test] fn address_discovery()` ← C `address_discovery_test`

### `pacing_test` (7 test(s))
   * C source: `picoquictest/pacing_test.c`
   * Rust target: `rs/fq/src/tests/pacing.rs`
   * Entries:
     - `#[test] fn pacing()` ← C `pacing_test`
     - `#[test] fn pacing_repeat()` ← C `pacing_repeat_test`
     - `#[test] fn pacing_bbr()` ← C `pacing_bbr_test`
     - `#[test] fn pacing_cubic()` ← C `pacing_cubic_test`
     - `#[test] fn pacing_dcubic()` ← C `pacing_dcubic_test`
     - `#[test] fn pacing_fast()` ← C `pacing_fast_test`
     - `#[test] fn pacing_newreno()` ← C `pacing_newreno_test`

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
    python3 scripts/phase3_check.py multipath_test netperf_test openssl_test p2p_test pacing_test
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
