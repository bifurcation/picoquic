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

### `stresstest` (5 test(s))
   * C source: `picoquictest/stresstest.c`
   * Rust target: `rs/fq/src/tests/stresstest.rs`
   * Entries:
     - `#[test] fn random_tester()` ← C `random_tester_test`
     - `#[test] fn random_gauss()` ← C `random_gauss_test`
     - `#[test] fn stress()` ← C `stress_test`
     - `#[test] fn fuzz()` ← C `fuzz_test`
     - `#[test] fn fuzz_initial()` ← C `fuzz_initial_test`

### `ticket_store_test` (5 test(s))
   * C source: `picoquictest/ticket_store_test.c`
   * Rust target: `rs/fq/src/tests/ticket_store.rs`
   * Entries:
     - `#[test] fn ticket_store()` ← C `ticket_store_test`
     - `#[test] fn ticket_seed()` ← C `ticket_seed_test`
     - `#[test] fn ticket_seed_from_bdp_frame()` ← C `ticket_seed_from_bdp_frame_test`
     - `#[test] fn token_store()` ← C `token_store_test`
     - `#[test] fn token_reuse_api()` ← C `token_reuse_api_test`

### `tls_api_test` (177 test(s))
   * C source: `picoquictest/tls_api_test.c`
   * Rust target: `rs/fq/src/tests/tls_api.rs`
   * Entries:
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

### `transport_param_test` (4 test(s))
   * C source: `picoquictest/transport_param_test.c`
   * Rust target: `rs/fq/src/tests/transport_param.rs`
   * Entries:
     - `#[test] fn vn_tp()` ← C `vn_tp_test`
     - `#[test] fn transport_param_default()` ← C `transport_param_default_test`
     - `#[test] fn transport_param()` ← C `transport_param_test`
     - `#[test] fn transport_param_log()` ← C `transport_param_log_test`

### `util_test` (7 test(s))
   * C source: `picoquictest/util_test.c`
   * Rust target: `rs/fq/src/tests/util_test.rs`
   * Entries:
     - `#[test] fn connection_id_print()` ← C `util_connection_id_print_test`
     - `#[test] fn connection_id_parse()` ← C `util_connection_id_parse_test`
     - `#[test] fn util_sprintf()` ← C `util_sprintf_test`
     - `#[test] fn util_debug_print()` ← C `util_debug_print_test`
     - `#[test] fn util_uint8_to_str()` ← C `util_uint8_to_str_test`
     - `#[test] fn util_memcmp()` ← C `util_memcmp_test`
     - `#[test] fn threading()` ← C `util_threading_test`

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
    python3 scripts/phase3_check.py stresstest ticket_store_test tls_api_test transport_param_test util_test
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
