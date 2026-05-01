# Cross-module consistency report

Generated: 2026-05-01T14:41:37
Files scanned: 20
Total lines: 13291

**How to use this report.**  Read each section.  Where you
see inconsistency that should be reconciled, decide a
policy, then sprinkle `// REVIEW: <instruction>` markers in
the offending files.  Run `scripts/phase1c.py` to apply.

## Trait names by case style

Total traits: 39.  PascalCase: 14, snake_case: 25

Rust convention says traits are `PascalCase`.  Phase 1's
policy was to mirror C typedef names where they're part of
the API contract; that argues for snake_case for callback
traits whose typedef name appears in C source the user
can read.  A mix is fine — but the mix should be principled.

// REVIEW: Always follow the Rust convention here. 

### snake_case (25)

| Trait | File | Line |
|---|---|---|
| `picoquic_stream_data_cb_fn` | `rs/fq/src/picoquic/picoquic.rs` | 616 |
| `picoquic_alpn_select_fn` | `rs/fq/src/picoquic/picoquic.rs` | 630 |
| `picoquic_alpn_select_fn_v2` | `rs/fq/src/picoquic/picoquic.rs` | 636 |
| `picoquic_connection_id_cb_fn` | `rs/fq/src/picoquic/picoquic.rs` | 643 |
| `picoquic_fuzz_fn` | `rs/fq/src/picoquic/picoquic.rs` | 655 |
| `picoquic_verify_sign_cb_fn` | `rs/fq/src/picoquic/picoquic.rs` | 673 |
| `picoquic_verify_certificate_cb_fn` | `rs/fq/src/picoquic/picoquic.rs` | 681 |
| `picoquic_free_verify_certificate_ctx` | `rs/fq/src/picoquic/picoquic.rs` | 694 |
| `picoquic_stream_direct_receive_fn` | `rs/fq/src/picoquic/picoquic.rs` | 702 |
| `picoquic_set_tls_key_provider_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 213 |
| `picoquic_get_private_key_from_file_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 223 |
| `picoquic_set_private_key_from_file_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 229 |
| `picoquic_get_public_key_from_private_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 237 |
| `picoquic_dispose_sign_certificate_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 244 |
| `picoquic_get_certs_from_file_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 252 |
| `picoquic_dispose_certificate_verifier_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 258 |
| `picoquic_get_certificate_verifier_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 281 |
| `picoquic_set_tls_root_certificates_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 288 |
| `picoquic_explain_crypto_error_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 298 |
| `picoquic_clear_crypto_errors_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 304 |
| `picoquic_set_random_provider_in_ctx_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 311 |
| `picoquic_crypto_random_provider_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 318 |
| `picoquic_keyex_from_key_file_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 326 |
| `picoquic_keyex_dispose_t` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 332 |
| `picoquic_unified_logging_t` | `rs/fq/src/picoquic/picoquic_unified_log.rs` | 81 |

### PascalCase (14)

| Trait | File | Line |
|---|---|---|
| `PicoHashOps` | `rs/fq/src/picoquic/picohash.rs` | 56 |
| `CongestionAlgorithm` | `rs/fq/src/picoquic/picoquic.rs` | 827 |
| `SpinBitPolicy` | `rs/fq/src/picoquic/picoquic_internal.rs` | 378 |
| `AutoQlog` | `rs/fq/src/picoquic/picoquic_internal.rs` | 747 |
| `PerformanceLog` | `rs/fq/src/picoquic/picoquic_internal.rs` | 754 |
| `MemLogHook` | `rs/fq/src/picoquic/picoquic_internal.rs` | 769 |
| `PicomaskOps` | `rs/fq/src/picoquic/picoquic_internal.rs` | 3652 |
| `PicoquicPacketLoopCbFn` | `rs/fq/src/picoquic/picoquic_packet_loop.rs` | 327 |
| `PicoquicCustomThreadCreateFn` | `rs/fq/src/picoquic/picoquic_packet_loop.rs` | 411 |
| `PicoquicCustomThreadSetnameFn` | `rs/fq/src/picoquic/picoquic_packet_loop.rs` | 430 |
| `PicoquicCustomThreadDeleteFn` | `rs/fq/src/picoquic/picoquic_packet_loop.rs` | 438 |
| `PicoquicThreadFn` | `rs/fq/src/picoquic/picoquic_utils.rs` | 733 |
| `PicoquictestAqmT` | `rs/fq/src/picoquic/picoquic_utils.rs` | 895 |
| `PicoSplayOps` | `rs/fq/src/picoquic/picosplay.rs` | 53 |

## Module-level lint allowances

Lints suppressed at module scope across 20 files:

| Lint | Modules using it | Modules NOT using it |
|---|---|---|
| `clippy::boxed_local` | 1: picoquic_packet_loop.rs | 19: bytestream.rs, cc_common.rs, performance_log.rs, picohash.rs, picoquic.rs, picoquic_binlog.rs, picoquic_config.rs, picoquic_crypto_provider_api.rs… |
| `clippy::enum_variant_names` | 3: picoquic.rs, picoquic_config.rs, picoquic_lb.rs | 17: bytestream.rs, cc_common.rs, performance_log.rs, picohash.rs, picoquic_binlog.rs, picoquic_crypto_provider_api.rs, picoquic_internal.rs, picoquic_logger.rs… |
| `clippy::result_unit_err` | 11: bytestream.rs, picoquic.rs, picoquic_binlog.rs, picoquic_crypto_provider_api.rs, picoquic_internal.rs, picoquic_lb.rs, picoquic_packet_loop.rs, picoquic_utils.rs, picoquictest_dualq.rs, picosocks.rs, tls_api.rs | 9: cc_common.rs, performance_log.rs, picohash.rs, picoquic_config.rs, picoquic_logger.rs, picoquic_qlog.rs, picoquic_unified_log.rs, picosplay.rs… |
| `clippy::too_many_arguments` | 5: picoquic_binlog.rs, picoquic_internal.rs, picoquic_packet_loop.rs, picoquic_unified_log.rs, tls_api.rs | 15: bytestream.rs, cc_common.rs, performance_log.rs, picohash.rs, picoquic.rs, picoquic_config.rs, picoquic_crypto_provider_api.rs, picoquic_lb.rs… |
| `non_camel_case_types` | 10: bytestream.rs, picoquic.rs, picoquic_config.rs, picoquic_crypto_provider_api.rs, picoquic_internal.rs, picoquic_lb.rs, picoquic_packet_loop.rs, picoquic_unified_log.rs, picoquic_utils.rs, picoquictest_dualq.rs | 10: cc_common.rs, performance_log.rs, picohash.rs, picoquic_binlog.rs, picoquic_logger.rs, picoquic_qlog.rs, picosocks.rs, picosplay.rs… |
| `non_snake_case` | 2: picoquic_internal.rs, picoquictest_dualq.rs | 18: bytestream.rs, cc_common.rs, performance_log.rs, picohash.rs, picoquic.rs, picoquic_binlog.rs, picoquic_config.rs, picoquic_crypto_provider_api.rs… |
| `non_upper_case_globals` | 4: picoquic.rs, picoquic_internal.rs, picoquic_packet_loop.rs, picoquic_utils.rs | 16: bytestream.rs, cc_common.rs, performance_log.rs, picohash.rs, picoquic_binlog.rs, picoquic_config.rs, picoquic_crypto_provider_api.rs, picoquic_lb.rs… |

Lints used in only some modules are the interesting ones.
Either the lint is appropriate for those modules and not
the others (fine — but worth a line of comment), or the
application is inconsistent.

// REVIEW:
// clippy::boxed_local - These should be fixed.  Don't use unnecessary Box.
// clippy::enum_variant_names - These should be fixed.  Follow the Rust convention.
// clippy::result_unit_err - These should be fixed.  Errors should always be semantic.  Define new error types as required.
// clippy::too_many_arguments - This can be ignored.  `allow` it at the crate level
// non_camel_case_types - This should be fixed.  Follow the Rust convention.
// non_upper_case_globals - This should be fixed.  Follow the Rust convention.

## Type definitions across modules

No name is defined in more than one module.  Good.

### All `pub struct` / `pub enum` / `pub type` declarations (118 total)

Single definitions are shown collapsed by source file.
Use this to see at a glance which module owns each type.

// REVIEW: We should remove all of the `pico` and `picoquic` from all names here,
// struct names, method names, variable names, etc.  Rust namespacing provides
// the separation; we don't need special tags on the names.

| Type | Kind | Source |
|---|---|---|
| `PicoquicCertificateVerifier` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:266` |
| `PicoquicDecryptedRetryToken` | `struct` | `rs/fq/src/picoquic/tls_api.rs:734` |
| `PicoquicInitialAeadContext` | `struct` | `rs/fq/src/picoquic/tls_api.rs:553` |
| `PicoquicVerifiedRetryToken` | `struct` | `rs/fq/src/picoquic/tls_api.rs:773` |
| `PreparedCnxPacket` | `struct` | `rs/fq/src/picoquic/picoquic.rs:1977` |
| `PreparedPacket` | `struct` | `rs/fq/src/picoquic/picoquic.rs:1934` |
| `RecvInfo` | `struct` | `rs/fq/src/picoquic/picosocks.rs:201` |
| `SelectExInfo` | `struct` | `rs/fq/src/picoquic/picosocks.rs:253` |
| `SelectInfo` | `struct` | `rs/fq/src/picoquic/picosocks.rs:231` |
| `ServerAddress` | `struct` | `rs/fq/src/picoquic/picosocks.rs:364` |
| `bytestream` | `struct` | `rs/fq/src/picoquic/bytestream.rs:87` |
| `bytestream_buf` | `struct` | `rs/fq/src/picoquic/bytestream.rs:106` |
| `bytestream_data` | `enum` | `rs/fq/src/picoquic/bytestream.rs:69` |
| `dualq_queue_t` | `struct` | `rs/fq/src/picoquic/picoquictest_dualq.rs:78` |
| `dualq_state_t` | `struct` | `rs/fq/src/picoquic/picoquictest_dualq.rs:102` |
| `packet_loop_system_call_duration_t` | `struct` | `rs/fq/src/picoquic/picoquic_packet_loop.rs:285` |
| `packet_loop_time_check_arg_t` | `struct` | `rs/fq/src/picoquic/picoquic_packet_loop.rs:304` |
| `picohash_item` | `struct` | `rs/fq/src/picoquic/picohash.rs:90` |
| `picohash_table` | `struct` | `rs/fq/src/picoquic/picohash.rs:125` |
| `picoquic_ack_context_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1098` |
| `picoquic_ack_context_track_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1087` |
| `picoquic_aes128_ecb_context_t` | `struct` | `rs/fq/src/picoquic/picoquic_lb.rs:107` |
| `picoquic_alpn_enum` | `enum` | `rs/fq/src/picoquic/picoquic.rs:901` |
| `picoquic_alpn_list_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:914` |
| `picoquic_call_back_event_t` | `enum` | `rs/fq/src/picoquic/picoquic.rs:437` |
| `picoquic_cipher_suites_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:415` |
| `picoquic_cnx_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1346` |
| `picoquic_congestion_algorithm_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:861` |
| `picoquic_congestion_notification_t` | `enum` | `rs/fq/src/picoquic/picoquic.rs:779` |
| `picoquic_connection_id_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:377` |
| `picoquic_crypto_context_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1333` |
| `picoquic_datagram_active_enum` | `enum` | `rs/fq/src/picoquic/picoquic.rs:756` |
| `picoquic_epoch_enum` | `enum` | `rs/fq/src/picoquic/picoquic_internal.rs:313` |
| `picoquic_event_t` | `struct` | `rs/fq/src/picoquic/picoquic_utils.rs:726` |
| `picoquic_file_t` | `struct` | `rs/fq/src/picoquic/picoquic_utils.rs:508` |
| `picoquic_frame_type_enum_t` | `type` | `rs/fq/src/picoquic/picoquic_internal.rs:201` |
| `picoquic_iovec_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:411` |
| `picoquic_issued_ticket_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:711` |
| `picoquic_jitter_mode` | `enum` | `rs/fq/src/picoquic/picoquic_utils.rs:923` |
| `picoquic_load_balancer_cid_context_t` | `struct` | `rs/fq/src/picoquic/picoquic_lb.rs:162` |
| `picoquic_load_balancer_cid_method_enum` | `enum` | `rs/fq/src/picoquic/picoquic_lb.rs:85` |
| `picoquic_load_balancer_config_t` | `struct` | `rs/fq/src/picoquic/picoquic_lb.rs:121` |
| `picoquic_local_cnxid_list_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1124` |
| `picoquic_local_cnxid_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1113` |
| `picoquic_log_event_type` | `enum` | `rs/fq/src/picoquic/picoquic_binlog.rs:132` |
| `picoquic_lossbit_version_enum` | `enum` | `rs/fq/src/picoquic/picoquic.rs:348` |
| `picoquic_min_max_rtt_t` | `struct` | `rs/fq/src/picoquic/cc_common.rs:56` |
| `picoquic_misc_frame_header_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1057` |
| `picoquic_msghdr_t` | `struct` | `rs/fq/src/picoquic/picosocks.rs:99` |
| `picoquic_mutex_t` | `struct` | `rs/fq/src/picoquic/picoquic_utils.rs:718` |
| `picoquic_network_thread_ctx_t` | `struct` | `rs/fq/src/picoquic/picoquic_packet_loop.rs:468` |
| `picoquic_newreno_alg_state_t` | `enum` | `rs/fq/src/picoquic/cc_common.rs:216` |
| `picoquic_newreno_sim_state_t` | `struct` | `rs/fq/src/picoquic/cc_common.rs:225` |
| `picoquic_option_enum_t` | `enum` | `rs/fq/src/picoquic/picoquic_config.rs:69` |
| `picoquic_pacing_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1160` |
| `picoquic_packet_context_enum` | `enum` | `rs/fq/src/picoquic/picoquic.rs:305` |
| `picoquic_packet_context_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1068` |
| `picoquic_packet_data_path_ack_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1624` |
| `picoquic_packet_data_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1638` |
| `picoquic_packet_header` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:340` |
| `picoquic_packet_loop_cb_enum` | `enum` | `rs/fq/src/picoquic/picoquic_packet_loop.rs:246` |
| `picoquic_packet_loop_options_t` | `struct` | `rs/fq/src/picoquic/picoquic_packet_loop.rs:353` |
| `picoquic_packet_loop_param_t` | `struct` | `rs/fq/src/picoquic/picoquic_packet_loop.rs:370` |
| `picoquic_packet_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:468` |
| `picoquic_packet_type_enum` | `enum` | `rs/fq/src/picoquic/picoquic_internal.rs:322` |
| `picoquic_path_quality_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:719` |
| `picoquic_path_status_enum` | `enum` | `rs/fq/src/picoquic/picoquic.rs:358` |
| `picoquic_path_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1198` |
| `picoquic_per_ack_state_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:801` |
| `picoquic_perflog_column_enum` | `enum` | `rs/fq/src/picoquic/performance_log.rs:46` |
| `picoquic_pmtu_discovery_status_enum` | `enum` | `rs/fq/src/picoquic/picoquic_internal.rs:252` |
| `picoquic_pmtud_policy_enum` | `enum` | `rs/fq/src/picoquic/picoquic.rs:317` |
| `picoquic_quic_config_t` | `struct` | `rs/fq/src/picoquic/picoquic_config.rs:143` |
| `picoquic_quic_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:784` |
| `picoquic_registered_token_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:531` |
| `picoquic_remote_cnxid_stash_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1149` |
| `picoquic_remote_cnxid_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1137` |
| `picoquic_sack_item_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:940` |
| `picoquic_sack_list_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:952` |
| `picoquic_sack_range_count_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:948` |
| `picoquic_server_sockets_t` | `struct` | `rs/fq/src/picoquic/picosocks.rs:81` |
| `picoquic_socket_ctx_t` | `struct` | `rs/fq/src/picoquic/picoquic_packet_loop.rs:149` |
| `picoquic_socket_t` | `struct` | `rs/fq/src/picoquic/picosocks.rs:66` |
| `picoquic_spinbit_def_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:393` |
| `picoquic_spinbit_version_enum` | `enum` | `rs/fq/src/picoquic/picoquic.rs:332` |
| `picoquic_state_enum` | `enum` | `rs/fq/src/picoquic/picoquic.rs:228` |
| `picoquic_stateless_packet_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:407` |
| `picoquic_stored_ticket_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:556` |
| `picoquic_stored_token_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:659` |
| `picoquic_stream_data_buffer_argument_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:2998` |
| `picoquic_stream_data_node_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:447` |
| `picoquic_stream_head_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:962` |
| `picoquic_stream_queue_node_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:457` |
| `picoquic_thread_t` | `struct` | `rs/fq/src/picoquic/picoquic_utils.rs:713` |
| `picoquic_tls_ctx_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:557` |
| `picoquic_tp_0rtt_enum` | `enum` | `rs/fq/src/picoquic/picoquic_internal.rs:543` |
| `picoquic_tp_enum` | `type` | `rs/fq/src/picoquic/picoquic.rs:260` |
| `picoquic_tp_preferred_address_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:476` |
| `picoquic_tp_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:513` |
| `picoquic_tp_version_negotiation_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:492` |
| `picoquic_tuple_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:1172` |
| `picoquic_version_parameters_t` | `struct` | `rs/fq/src/picoquic/picoquic_internal.rs:287` |
| `picoquictest_sim_link_t` | `struct` | `rs/fq/src/picoquic/picoquic_utils.rs:949` |
| `picoquictest_sim_packet_t` | `struct` | `rs/fq/src/picoquic/picoquic_utils.rs:876` |
| `picosplay_node_t` | `struct` | `rs/fq/src/picoquic/picosplay.rs:89` |
| `picosplay_tree_t` | `struct` | `rs/fq/src/picoquic/picosplay.rs:127` |
| `ptls_cipher_suite_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:96` |
| `ptls_context_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:121` |
| `ptls_handshake_properties_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:148` |
| `ptls_hpke_cipher_suite_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:108` |
| `ptls_hpke_kem_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:114` |
| `ptls_iovec_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:396` |
| `ptls_key_exchange_algorithm_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:102` |
| `ptls_key_exchange_context_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:154` |
| `ptls_raw_extension_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:142` |
| `ptls_sign_certificate_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:127` |
| `ptls_t` | `struct` | `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs:134` |
| `ptls_verify_certificate_t` | `struct` | `rs/fq/src/picoquic/picoquic.rs:666` |

## Cross-module imports

Items pulled in via `use crate::picoquic::…`, grouped by
source module.  A type imported by many modules but defined
in one place is the healthy pattern; a type imported via
two different source paths is a smell.

| Source module | Items imported | Importers |
|---|---|---|
| `picohash::` | `*` | 1: picoquic_internal.rs |
| `picoquic::` | `*` | 11: cc_common.rs, picoquic_binlog.rs, picoquic_config.rs, picoquic_crypto_provider_api.rs, picoquic_internal.rs, … (6 more) |
| `picoquic::picoquic_connection_id_t` | `*` | 1: bytestream.rs |
| `picoquic::picoquic_quic_t` | `*` | 3: performance_log.rs, picoquic_qlog.rs, picosocks.rs |
| `picoquic_config::picoquic_quic_config_t` | `*` | 1: picoquic_packet_loop.rs |
| `picoquic_internal::` | `*` | 2: picoquic_binlog.rs, picoquic_unified_log.rs |
| `picoquic_internal::picoquic_crypto_context_t` | `*` | 1: tls_api.rs |
| `picoquic_unified_log::picoquic_unified_logging_t` | `*` | 1: picoquic_internal.rs |
| `picoquic_utils::` | `*` | 2: picoquic_packet_loop.rs, picoquictest_dualq.rs |
| `picoquic_utils::picoquic_file_t` | `*` | 1: picoquic_binlog.rs |
| `picosplay::` | `*` | 1: picoquic_internal.rs |

// REVIEW: Rather than `*` imports, it would be better to be precise about what
// is imported.  We should not have any re-exports (`pub use`), which should
// prevent the same import via different paths.

## Per-file summary

| File | LOC | Traits | Structs | Enums | Type aliases | Fns | Inner #![allow] |
|---|---:|---:|---:|---:|---:|---:|---:|
| `rs/fq/src/picoquic/bytestream.rs` | 406 | 0 | 2 | 1 | 0 | 38 | 2 |
| `rs/fq/src/picoquic/cc_common.rs` | 256 | 0 | 2 | 1 | 0 | 15 | 0 |
| `rs/fq/src/picoquic/performance_log.rs` | 114 | 0 | 0 | 1 | 0 | 2 | 0 |
| `rs/fq/src/picoquic/picohash.rs` | 275 | 1 | 2 | 0 | 0 | 9 | 0 |
| `rs/fq/src/picoquic/picoquic.rs` | 2437 | 10 | 13 | 10 | 1 | 212 | 4 |
| `rs/fq/src/picoquic/picoquic_binlog.rs` | 367 | 0 | 0 | 1 | 0 | 14 | 2 |
| `rs/fq/src/picoquic/picoquic_config.rs` | 384 | 0 | 1 | 1 | 0 | 9 | 2 |
| `rs/fq/src/picoquic/picoquic_crypto_provider_api.rs` | 598 | 15 | 13 | 0 | 0 | 26 | 2 |
| `rs/fq/src/picoquic/picoquic_internal.rs` | 3679 | 5 | 32 | 4 | 1 | 275 | 5 |
| `rs/fq/src/picoquic/picoquic_lb.rs` | 244 | 0 | 3 | 1 | 0 | 5 | 3 |
| `rs/fq/src/picoquic/picoquic_logger.rs` | 115 | 0 | 0 | 0 | 0 | 4 | 0 |
| `rs/fq/src/picoquic/picoquic_packet_loop.rs` | 773 | 4 | 6 | 1 | 0 | 15 | 5 |
| `rs/fq/src/picoquic/picoquic_qlog.rs` | 38 | 0 | 0 | 0 | 0 | 1 | 0 |
| `rs/fq/src/picoquic/picoquic_unified_log.rs` | 419 | 1 | 0 | 0 | 0 | 16 | 2 |
| `rs/fq/src/picoquic/picoquic_utils.rs` | 1138 | 2 | 6 | 1 | 0 | 93 | 3 |
| `rs/fq/src/picoquic/picoquictest_dualq.rs` | 259 | 0 | 2 | 0 | 0 | 3 | 3 |
| `rs/fq/src/picoquic/picosocks.rs` | 456 | 0 | 7 | 0 | 0 | 20 | 1 |
| `rs/fq/src/picoquic/picosplay.rs` | 256 | 1 | 2 | 0 | 0 | 12 | 0 |
| `rs/fq/src/picoquic/siphash.rs` | 40 | 0 | 0 | 0 | 0 | 1 | 0 |
| `rs/fq/src/picoquic/tls_api.rs` | 1037 | 0 | 3 | 0 | 0 | 53 | 2 |

// REVIEW: There's no reason to have the files be in `src/picoquic` instead of
// `src`.  Move them up a level.

// REVIEW: There should be no type aliases.  And it looks like the type aliases
// that do exist are failures to convert to Rust concepts -- preserving C-style
// enums.  These should be either newtype (e.g., `pub struct FrameType(u64)`) or 
// actual Rust enums.  I would probably prefer the latter.
