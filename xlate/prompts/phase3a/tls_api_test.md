# Phase 3A test-body translation: `tls_api_test`

You're translating the C test bodies in `picoquictest/`
into Rust `#[test]` bodies in `rs/fq/src/tests/tls_api.rs`.

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
2. The C source: `picoquictest/tls_api_test.c`.
3. The Rust target: `rs/fq/src/tests/tls_api.rs` — currently has
   auto-generated stubs (`todo!("<entry_fn>")`) that you
   replace with translations.
4. `rs/fq/src/tests/util.rs` — test infrastructure;
   add helpers here when multiple translations would need
   them.
5. Module sources (`rs/fq/src/<X>.rs`) only when you need to
   verify a specific signature the guide didn't quote.

## Test entries to translate

   - `#[test] fn stateless_blowback()` ← C `test_stateless_blowback`
   - `#[test] fn pn_enc_1rtt()` ← C `pn_enc_1rtt_test`
   - `#[test] fn tls_api_connect()` ← C `tls_api_connect_test`
   - `#[test] fn tls_api()` ← C `tls_api_test`
   - `#[test] fn tls_api_inject_hs_ack()` ← C `tls_api_inject_hs_ack_test`
   - `#[test] fn tls_exporter()` ← C `tls_exporter_test`
   - `#[test] fn null_sni()` ← C `null_sni_test`
   - `#[test] fn silence_test()` ← C `tls_api_silence_test`
   - `#[test] fn version_negotiation()` ← C `tls_api_version_negotiation_test`
   - `#[test] fn version_invariant()` ← C `tls_api_version_invariant_test`
   - `#[test] fn version_negotiation_spoof()` ← C `test_version_negotiation_spoof`
   - `#[test] fn first_loss()` ← C `tls_api_client_first_loss_test`
   - `#[test] fn second_loss()` ← C `tls_api_client_second_loss_test`
   - `#[test] fn SH_loss()` ← C `tls_api_server_first_loss_test`
   - `#[test] fn client_losses()` ← C `tls_api_client_losses_test`
   - `#[test] fn server_losses()` ← C `tls_api_server_losses_test`
   - `#[test] fn many_losses()` ← C `tls_api_many_losses`
   - `#[test] fn ddos_amplification()` ← C `ddos_amplification_test`
   - `#[test] fn ddos_amplification_0rtt()` ← C `ddos_amplification_0rtt_test`
   - `#[test] fn ddos_amplification_8k()` ← C `ddos_amplification_8k_test`
   - `#[test] fn no_ack_frequency()` ← C `no_ack_frequency_test`
   - `#[test] fn immediate_ack()` ← C `immediate_ack_test`
   - `#[test] fn connection_drop()` ← C `connection_drop_test`
   - `#[test] fn vn_compat()` ← C `vn_compat_test`
   - `#[test] fn tls_api_sni()` ← C `tls_api_sni_test`
   - `#[test] fn tls_api_alpn()` ← C `tls_api_alpn_test`
   - `#[test] fn tls_api_wrong_alpn()` ← C `tls_api_wrong_alpn_test`
   - `#[test] fn tls_api_oneway_stream()` ← C `tls_api_oneway_stream_test`
   - `#[test] fn tls_api_q_and_r_stream()` ← C `tls_api_q_and_r_stream_test`
   - `#[test] fn tls_api_q2_and_r2_stream()` ← C `tls_api_q2_and_r2_stream_test`
   - `#[test] fn implicit_ack()` ← C `implicit_ack_test`
   - `#[test] fn stateless_reset()` ← C `stateless_reset_test`
   - `#[test] fn stateless_reset_bad()` ← C `stateless_reset_bad_test`
   - `#[test] fn stateless_reset_client()` ← C `stateless_reset_client_test`
   - `#[test] fn stateless_reset_handshake()` ← C `stateless_reset_handshake_test`
   - `#[test] fn immediate_close()` ← C `immediate_close_test`
   - `#[test] fn tls_api_very_long_stream()` ← C `tls_api_very_long_stream_test`
   - `#[test] fn tls_api_very_long_max()` ← C `tls_api_very_long_max_test`
   - `#[test] fn tls_api_very_long_with_err()` ← C `tls_api_very_long_with_err_test`
   - `#[test] fn tls_api_very_long_congestion()` ← C `tls_api_very_long_congestion_test`
   - `#[test] fn many_short_loss()` ← C `many_short_loss_test`
   - `#[test] fn retry()` ← C `tls_api_retry_test`
   - `#[test] fn retry_large()` ← C `tls_api_retry_large_test`
   - `#[test] fn retry_token()` ← C `tls_retry_token_test`
   - `#[test] fn retry_token_valid()` ← C `tls_retry_token_valid_test`
   - `#[test] fn two_connections()` ← C `tls_api_two_connections_test`
   - `#[test] fn multiple_versions()` ← C `tls_api_multiple_versions_test`
   - `#[test] fn keep_alive()` ← C `keep_alive_test`
   - `#[test] fn integrity_limit()` ← C `integrity_limit_test`
   - `#[test] fn excess_repeat()` ← C `excess_repeat_test`
   - `#[test] fn session_resume()` ← C `session_resume_test`
   - `#[test] fn zero_rtt()` ← C `zero_rtt_test`
   - `#[test] fn zero_rtt_loss()` ← C `zero_rtt_loss_test`
   - `#[test] fn stop_sending()` ← C `stop_sending_test`
   - `#[test] fn stop_sending_loss()` ← C `stop_sending_loss_test`
   - `#[test] fn discard_stream()` ← C `discard_stream_test`
   - `#[test] fn unidir()` ← C `unidir_test`
   - `#[test] fn mtu_discovery()` ← C `mtu_discovery_test`
   - `#[test] fn mtu_blocked()` ← C `mtu_blocked_test`
   - `#[test] fn mtu_delayed()` ← C `mtu_delayed_test`
   - `#[test] fn mtu_required()` ← C `mtu_required_test`
   - `#[test] fn mtu_max()` ← C `mtu_max_test`
   - `#[test] fn mtu_drop_bbr()` ← C `mtu_drop_bbr_test`
   - `#[test] fn mtu_drop_cubic()` ← C `mtu_drop_cubic_test`
   - `#[test] fn mtu_drop_dcubic()` ← C `mtu_drop_dcubic_test`
   - `#[test] fn mtu_drop_fast()` ← C `mtu_drop_fast_test`
   - `#[test] fn mtu_drop_newreno()` ← C `mtu_drop_newreno_test`
   - `#[test] fn red_bbr()` ← C `red_bbr_test`
   - `#[test] fn red_cubic()` ← C `red_cubic_test`
   - `#[test] fn red_dcubic()` ← C `red_dcubic_test`
   - `#[test] fn red_fast()` ← C `red_fast_test`
   - `#[test] fn red_newreno()` ← C `red_newreno_test`
   - `#[test] fn multi_segment()` ← C `multi_segment_test`
   - `#[test] fn heavy_loss()` ← C `heavy_loss_test`
   - `#[test] fn heavy_loss_inter()` ← C `heavy_loss_inter_test`
   - `#[test] fn heavy_loss_total()` ← C `heavy_loss_total_test`
   - `#[test] fn spurious_retransmit()` ← C `spurious_retransmit_test`
   - `#[test] fn tls_zero_share()` ← C `tls_zero_share_test`
   - `#[test] fn bad_certificate()` ← C `bad_certificate_test`
   - `#[test] fn set_verify_certificate_callback_test()` ← C `set_verify_certificate_callback_test`
   - `#[test] fn virtual_time()` ← C `virtual_time_test`
   - `#[test] fn different_params()` ← C `tls_different_params_test`
   - `#[test] fn quant_params()` ← C `tls_quant_params_test`
   - `#[test] fn set_certificate_and_key()` ← C `set_certificate_and_key_test`
   - `#[test] fn request_client_authentication()` ← C `request_client_authentication_test`
   - `#[test] fn bad_client_certificate()` ← C `bad_client_certificate_test`
   - `#[test] fn nat_rebinding()` ← C `nat_rebinding_test`
   - `#[test] fn nat_rebinding_loss()` ← C `nat_rebinding_loss_test`
   - `#[test] fn nat_rebinding_zero()` ← C `nat_rebinding_zero_test`
   - `#[test] fn nat_rebinding_latency()` ← C `nat_rebinding_latency_test`
   - `#[test] fn nat_rebinding_fast()` ← C `fast_nat_rebinding_test`
   - `#[test] fn loss_bit()` ← C `loss_bit_test`
   - `#[test] fn client_error()` ← C `client_error_test`
   - `#[test] fn client_only()` ← C `client_only_test`
   - `#[test] fn zero_rtt_spurious()` ← C `zero_rtt_spurious_test`
   - `#[test] fn zero_rtt_bad_param()` ← C `zero_rtt_bad_param_test`
   - `#[test] fn zero_rtt_retry()` ← C `zero_rtt_retry_test`
   - `#[test] fn zero_rtt_no_coal()` ← C `zero_rtt_no_coal_test`
   - `#[test] fn zero_rtt_many_losses()` ← C `zero_rtt_many_losses_test`
   - `#[test] fn zero_rtt_long()` ← C `zero_rtt_long_test`
   - `#[test] fn zero_rtt_delay()` ← C `zero_rtt_delay_test`
   - `#[test] fn zero_rtt_ech()` ← C `zero_rtt_ech_test`
   - `#[test] fn random_public_tester()` ← C `random_public_tester_test`
   - `#[test] fn cnxid_transmit()` ← C `transmit_cnxid_test`
   - `#[test] fn cnxid_transmit_disable()` ← C `transmit_cnxid_disable_test`
   - `#[test] fn cnxid_transmit_r_before()` ← C `transmit_cnxid_retire_before_test`
   - `#[test] fn cnxid_transmit_r_disable()` ← C `transmit_cnxid_retire_disable_test`
   - `#[test] fn cnxid_transmit_r_early()` ← C `transmit_cnxid_retire_early_test`
   - `#[test] fn probe_api()` ← C `probe_api_test`
   - `#[test] fn migration()` ← C `migration_test`
   - `#[test] fn migration_long()` ← C `migration_test_long`
   - `#[test] fn migration_with_loss()` ← C `migration_test_loss`
   - `#[test] fn migration_zero()` ← C `migration_zero_test`
   - `#[test] fn migration_fail()` ← C `migration_fail_test`
   - `#[test] fn preferred_address()` ← C `preferred_address_test`
   - `#[test] fn preferred_address_dis_mig()` ← C `preferred_address_dis_mig_test`
   - `#[test] fn preferred_address_zero()` ← C `preferred_address_zero_test`
   - `#[test] fn cnxid_renewal()` ← C `cnxid_renewal_test`
   - `#[test] fn retire_cnxid()` ← C `retire_cnxid_test`
   - `#[test] fn not_before_cnxid()` ← C `not_before_cnxid_test`
   - `#[test] fn server_busy()` ← C `server_busy_test`
   - `#[test] fn initial_close()` ← C `initial_close_test`
   - `#[test] fn initial_server_close()` ← C `initial_server_close_test`
   - `#[test] fn new_rotated_key()` ← C `new_rotated_key_test`
   - `#[test] fn key_rotation()` ← C `key_rotation_test`
   - `#[test] fn key_rotation_server()` ← C `key_rotation_auto_server`
   - `#[test] fn key_rotation_client()` ← C `key_rotation_auto_client`
   - `#[test] fn false_migration()` ← C `false_migration_test`
   - `#[test] fn nat_handshake()` ← C `nat_handshake_test`
   - `#[test] fn key_rotation_stress()` ← C `key_rotation_stress_test`
   - `#[test] fn keylog_test()` ← C `keylog_test`
   - `#[test] fn short_initial_cid()` ← C `short_initial_cid_test`
   - `#[test] fn stream_id_max()` ← C `stream_id_max_test`
   - `#[test] fn padding_test()` ← C `padding_test`
   - `#[test] fn padding_null()` ← C `padding_null_test`
   - `#[test] fn padding_zero_min()` ← C `padding_zero_min_test`
   - `#[test] fn packet_trace()` ← C `packet_trace_test`
   - `#[test] fn qlog_trace()` ← C `qlog_trace_test`
   - `#[test] fn qlog_trace_ecn()` ← C `qlog_trace_ecn_test`
   - `#[test] fn qlog_trace_parallel()` ← C `qlog_trace_parallel_test`
   - `#[test] fn qlog_fns()` ← C `qlog_fns_test`
   - `#[test] fn qlog_fns_ecn()` ← C `qlog_fns_ecn_test`
   - `#[test] fn perflog()` ← C `perflog_test`
   - `#[test] fn nat_rebinding_stress()` ← C `rebinding_stress_test`
   - `#[test] fn random_padding()` ← C `random_padding_test`
   - `#[test] fn error_reason()` ← C `error_reason_test`
   - `#[test] fn ready_to_send()` ← C `ready_to_send_test`
   - `#[test] fn ready_to_skip()` ← C `ready_to_skip_test`
   - `#[test] fn ready_to_zfin()` ← C `ready_to_zfin_test`
   - `#[test] fn ready_to_zero()` ← C `ready_to_zero_test`
   - `#[test] fn long_rtt()` ← C `long_rtt_test`
   - `#[test] fn cid_length()` ← C `cid_length_test`
   - `#[test] fn optimistic_ack()` ← C `optimistic_ack_test`
   - `#[test] fn optimistic_hole()` ← C `optimistic_hole_test`
   - `#[test] fn bad_coalesce()` ← C `bad_coalesce_test`
   - `#[test] fn bad_cnxid()` ← C `bad_cnxid_test`
   - `#[test] fn document_addresses()` ← C `document_addresses_test`
   - `#[test] fn migration_disabled()` ← C `migration_disabled_test`
   - `#[test] fn large_client_hello()` ← C `large_client_hello_test`
   - `#[test] fn pacing_update()` ← C `pacing_update_test`
   - `#[test] fn quality_update()` ← C `quality_update_test`
   - `#[test] fn direct_receive()` ← C `direct_receive_test`
   - `#[test] fn af_undef()` ← C `af_undef_test`
   - `#[test] fn initial_race()` ← C `initial_race_test`
   - `#[test] fn chacha20()` ← C `chacha20_test`
   - `#[test] fn cid_quiescence()` ← C `cid_quiescence_test`
   - `#[test] fn client_auth()` ← C `request_client_authentication_test`
   - `#[test] fn client_auth_25519()` ← C `request_client_authentication_25519_test`
   - `#[test] fn client_cert_callback()` ← C `set_verify_certificate_callback_test`
   - `#[test] fn get_hash()` ← C `get_hash_test`
   - `#[test] fn get_tls_errors()` ← C `get_tls_errors_test`
   - `#[test] fn grease_quic_bit()` ← C `grease_quic_bit_test`
   - `#[test] fn grease_quic_bit_one_way()` ← C `grease_quic_bit_one_way_test`
   - `#[test] fn bad_chello()` ← C `bad_chello_test`
   - `#[test] fn pn_random()` ← C `pn_random_test`
   - `#[test] fn port_blocked()` ← C `port_blocked_test`
   - `#[test] fn cnx_ddos()` ← C `cnx_ddos_unit_test`

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

1. Read the C source and the existing `rs/fq/src/tests/tls_api.rs`.
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
