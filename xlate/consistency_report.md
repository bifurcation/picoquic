# Cross-module consistency report

Generated: 2026-05-02T10:10:52
Files scanned: 20
Total lines: 12775

**How to use this report.**  Read each section.  Where you
see inconsistency that should be reconciled, decide a
policy, then sprinkle `// REVIEW: <instruction>` markers in
the offending files.  Run `scripts/phase1c.py` to apply.

## Trait names by case style

Total traits: 39.  PascalCase: 39

Rust convention says traits are `PascalCase`.  Anything
else is a refactor candidate.

### PascalCase (39)

| Trait | File | Line |
|---|---|---|
| `SetTlsKeyProvider` | `rs/fq/src/crypto_provider_api.rs` | 210 |
| `GetPrivateKeyFromFile` | `rs/fq/src/crypto_provider_api.rs` | 220 |
| `SetPrivateKeyFromFile` | `rs/fq/src/crypto_provider_api.rs` | 226 |
| `GetPublicKeyFromPrivate` | `rs/fq/src/crypto_provider_api.rs` | 234 |
| `DisposeSignCertificate` | `rs/fq/src/crypto_provider_api.rs` | 241 |
| `GetCertsFromFile` | `rs/fq/src/crypto_provider_api.rs` | 249 |
| `DisposeCertificateVerifier` | `rs/fq/src/crypto_provider_api.rs` | 255 |
| `GetCertificateVerifier` | `rs/fq/src/crypto_provider_api.rs` | 278 |
| `SetTlsRootCertificates` | `rs/fq/src/crypto_provider_api.rs` | 285 |
| `ExplainCryptoError` | `rs/fq/src/crypto_provider_api.rs` | 295 |
| `ClearCryptoErrors` | `rs/fq/src/crypto_provider_api.rs` | 301 |
| `SetRandomProviderInCtx` | `rs/fq/src/crypto_provider_api.rs` | 308 |
| `CryptoRandomProvider` | `rs/fq/src/crypto_provider_api.rs` | 315 |
| `KeyexFromKeyFile` | `rs/fq/src/crypto_provider_api.rs` | 323 |
| `KeyexDispose` | `rs/fq/src/crypto_provider_api.rs` | 329 |
| `HashOps` | `rs/fq/src/hash.rs` | 57 |
| `SpinBitPolicy` | `rs/fq/src/internal.rs` | 377 |
| `AutoQlog` | `rs/fq/src/internal.rs` | 731 |
| `PerformanceLog` | `rs/fq/src/internal.rs` | 738 |
| `MemLogHook` | `rs/fq/src/internal.rs` | 748 |
| `MaskOps` | `rs/fq/src/internal.rs` | 3458 |
| `StreamDataCb` | `rs/fq/src/lib.rs` | 669 |
| `AlpnSelect` | `rs/fq/src/lib.rs` | 683 |
| `AlpnSelectV2` | `rs/fq/src/lib.rs` | 689 |
| `ConnectionIdCb` | `rs/fq/src/lib.rs` | 696 |
| `Fuzz` | `rs/fq/src/lib.rs` | 708 |
| `VerifySignCb` | `rs/fq/src/lib.rs` | 726 |
| `VerifyCertificateCb` | `rs/fq/src/lib.rs` | 734 |
| `FreeVerifyCertificateCtx` | `rs/fq/src/lib.rs` | 747 |
| `StreamDirectReceive` | `rs/fq/src/lib.rs` | 755 |
| `CongestionAlgorithm` | `rs/fq/src/lib.rs` | 880 |
| `PacketLoopCbFn` | `rs/fq/src/packet_loop.rs` | 317 |
| `CustomThreadCreateFn` | `rs/fq/src/packet_loop.rs` | 401 |
| `CustomThreadSetnameFn` | `rs/fq/src/packet_loop.rs` | 420 |
| `CustomThreadDeleteFn` | `rs/fq/src/packet_loop.rs` | 428 |
| `SplayOps` | `rs/fq/src/splay.rs` | 53 |
| `UnifiedLogging` | `rs/fq/src/unified_log.rs` | 73 |
| `ThreadFn` | `rs/fq/src/utils.rs` | 717 |
| `TestAqm` | `rs/fq/src/utils.rs` | 873 |

## Module-level lint allowances

Lints suppressed at module scope across 20 files:

| Lint | Modules using it | Modules NOT using it |
|---|---|---|
| `clippy::too_many_arguments` | 1: lib.rs | 19: binlog.rs, bytestream.rs, cc_common.rs, config.rs, crypto_provider_api.rs, hash.rs, internal.rs, lb.rs… |
| `non_camel_case_types` | 10: bytestream.rs, config.rs, crypto_provider_api.rs, internal.rs, lb.rs, lib.rs, packet_loop.rs, test_dualq.rs, unified_log.rs, utils.rs | 10: binlog.rs, cc_common.rs, hash.rs, logger.rs, performance_log.rs, qlog.rs, siphash.rs, socks.rs… |
| `non_snake_case` | 2: internal.rs, test_dualq.rs | 18: binlog.rs, bytestream.rs, cc_common.rs, config.rs, crypto_provider_api.rs, hash.rs, lb.rs, lib.rs… |
| `non_upper_case_globals` | 4: internal.rs, lib.rs, packet_loop.rs, utils.rs | 16: binlog.rs, bytestream.rs, cc_common.rs, config.rs, crypto_provider_api.rs, hash.rs, lb.rs, logger.rs… |

Lints used in only some modules are the interesting ones.
Either the lint is appropriate for those modules and not
the others (fine — but worth a line of comment), or the
application is inconsistent.

## Type definitions across modules

No name is defined in more than one module.  Good.

### All `pub struct` / `pub enum` / `pub type` declarations (119 total)

Single definitions are shown collapsed by source file.
Use this to see at a glance which module owns each type.

| Type | Kind | Source |
|---|---|---|
| `CertificateVerifier` | `struct` | `rs/fq/src/crypto_provider_api.rs:263` |
| `DecryptedRetryToken` | `struct` | `rs/fq/src/tls_api.rs:710` |
| `Error` | `enum` | `rs/fq/src/lib.rs:101` |
| `FrameType` | `enum` | `rs/fq/src/internal.rs:199` |
| `InitialAeadContext` | `struct` | `rs/fq/src/tls_api.rs:534` |
| `PreparedCnxPacket` | `struct` | `rs/fq/src/lib.rs:1899` |
| `PreparedPacket` | `struct` | `rs/fq/src/lib.rs:1856` |
| `RecvInfo` | `struct` | `rs/fq/src/socks.rs:198` |
| `SelectExInfo` | `struct` | `rs/fq/src/socks.rs:250` |
| `SelectInfo` | `struct` | `rs/fq/src/socks.rs:228` |
| `ServerAddress` | `struct` | `rs/fq/src/socks.rs:361` |
| `Tp` | `enum` | `rs/fq/src/lib.rs:317` |
| `VerifiedRetryToken` | `struct` | `rs/fq/src/tls_api.rs:749` |
| `ack_context_t` | `struct` | `rs/fq/src/internal.rs:1071` |
| `ack_context_track_t` | `struct` | `rs/fq/src/internal.rs:1060` |
| `aes128_ecb_context_t` | `struct` | `rs/fq/src/lb.rs:103` |
| `alpn_enum` | `enum` | `rs/fq/src/lib.rs:948` |
| `alpn_list_t` | `struct` | `rs/fq/src/lib.rs:961` |
| `bytestream` | `struct` | `rs/fq/src/bytestream.rs:87` |
| `bytestream_buf` | `struct` | `rs/fq/src/bytestream.rs:106` |
| `bytestream_data` | `enum` | `rs/fq/src/bytestream.rs:69` |
| `call_back_event_t` | `enum` | `rs/fq/src/lib.rs:494` |
| `cipher_suites_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:410` |
| `cnx_t` | `struct` | `rs/fq/src/internal.rs:1319` |
| `congestion_algorithm_t` | `struct` | `rs/fq/src/lib.rs:909` |
| `congestion_notification_t` | `enum` | `rs/fq/src/lib.rs:832` |
| `connection_id_t` | `struct` | `rs/fq/src/lib.rs:434` |
| `crypto_context_t` | `struct` | `rs/fq/src/internal.rs:1306` |
| `datagram_active_enum` | `enum` | `rs/fq/src/lib.rs:809` |
| `dualq_queue_t` | `struct` | `rs/fq/src/test_dualq.rs:76` |
| `dualq_state_t` | `struct` | `rs/fq/src/test_dualq.rs:100` |
| `epoch_enum` | `enum` | `rs/fq/src/internal.rs:312` |
| `event_t` | `struct` | `rs/fq/src/utils.rs:710` |
| `file_t` | `struct` | `rs/fq/src/utils.rs:503` |
| `hash_item` | `struct` | `rs/fq/src/hash.rs:91` |
| `hash_table` | `struct` | `rs/fq/src/hash.rs:126` |
| `iovec_t` | `struct` | `rs/fq/src/lib.rs:468` |
| `issued_ticket_t` | `struct` | `rs/fq/src/internal.rs:698` |
| `jitter_mode` | `enum` | `rs/fq/src/utils.rs:901` |
| `load_balancer_cid_context_t` | `struct` | `rs/fq/src/lb.rs:157` |
| `load_balancer_cid_method_enum` | `enum` | `rs/fq/src/lb.rs:81` |
| `load_balancer_config_t` | `struct` | `rs/fq/src/lb.rs:117` |
| `local_cnxid_list_t` | `struct` | `rs/fq/src/internal.rs:1097` |
| `local_cnxid_t` | `struct` | `rs/fq/src/internal.rs:1086` |
| `log_event_type` | `enum` | `rs/fq/src/binlog.rs:126` |
| `lossbit_version_enum` | `enum` | `rs/fq/src/lib.rs:405` |
| `min_max_rtt_t` | `struct` | `rs/fq/src/cc_common.rs:54` |
| `misc_frame_header_t` | `struct` | `rs/fq/src/internal.rs:1030` |
| `msghdr_t` | `struct` | `rs/fq/src/socks.rs:99` |
| `mutex_t` | `struct` | `rs/fq/src/utils.rs:702` |
| `network_thread_ctx_t` | `struct` | `rs/fq/src/packet_loop.rs:458` |
| `newreno_alg_state_t` | `enum` | `rs/fq/src/cc_common.rs:204` |
| `newreno_sim_state_t` | `struct` | `rs/fq/src/cc_common.rs:213` |
| `option_enum_t` | `enum` | `rs/fq/src/config.rs:63` |
| `pacing_t` | `struct` | `rs/fq/src/internal.rs:1133` |
| `packet_context_enum` | `enum` | `rs/fq/src/lib.rs:362` |
| `packet_context_t` | `struct` | `rs/fq/src/internal.rs:1041` |
| `packet_data_path_ack_t` | `struct` | `rs/fq/src/internal.rs:1597` |
| `packet_data_t` | `struct` | `rs/fq/src/internal.rs:1611` |
| `packet_header` | `struct` | `rs/fq/src/internal.rs:339` |
| `packet_loop_cb_enum` | `enum` | `rs/fq/src/packet_loop.rs:236` |
| `packet_loop_options_t` | `struct` | `rs/fq/src/packet_loop.rs:343` |
| `packet_loop_param_t` | `struct` | `rs/fq/src/packet_loop.rs:360` |
| `packet_loop_system_call_duration_t` | `struct` | `rs/fq/src/packet_loop.rs:275` |
| `packet_loop_time_check_arg_t` | `struct` | `rs/fq/src/packet_loop.rs:294` |
| `packet_t` | `struct` | `rs/fq/src/internal.rs:455` |
| `packet_type_enum` | `enum` | `rs/fq/src/internal.rs:321` |
| `path_quality_t` | `struct` | `rs/fq/src/lib.rs:772` |
| `path_status_enum` | `enum` | `rs/fq/src/lib.rs:415` |
| `path_t` | `struct` | `rs/fq/src/internal.rs:1171` |
| `per_ack_state_t` | `struct` | `rs/fq/src/lib.rs:854` |
| `perflog_column_enum` | `enum` | `rs/fq/src/performance_log.rs:44` |
| `pmtu_discovery_status_enum` | `enum` | `rs/fq/src/internal.rs:251` |
| `pmtud_policy_enum` | `enum` | `rs/fq/src/lib.rs:374` |
| `ptls_cipher_suite_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:96` |
| `ptls_context_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:121` |
| `ptls_handshake_properties_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:148` |
| `ptls_hpke_cipher_suite_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:108` |
| `ptls_hpke_kem_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:114` |
| `ptls_iovec_t` | `struct` | `rs/fq/src/lib.rs:453` |
| `ptls_key_exchange_algorithm_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:102` |
| `ptls_key_exchange_context_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:154` |
| `ptls_raw_extension_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:142` |
| `ptls_sign_certificate_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:127` |
| `ptls_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:134` |
| `ptls_verify_certificate_t` | `struct` | `rs/fq/src/lib.rs:719` |
| `quic_config_t` | `struct` | `rs/fq/src/config.rs:137` |
| `quic_t` | `struct` | `rs/fq/src/internal.rs:757` |
| `registered_token_t` | `struct` | `rs/fq/src/internal.rs:518` |
| `remote_cnxid_stash_t` | `struct` | `rs/fq/src/internal.rs:1122` |
| `remote_cnxid_t` | `struct` | `rs/fq/src/internal.rs:1110` |
| `sack_item_t` | `struct` | `rs/fq/src/internal.rs:913` |
| `sack_list_t` | `struct` | `rs/fq/src/internal.rs:925` |
| `sack_range_count_t` | `struct` | `rs/fq/src/internal.rs:921` |
| `server_sockets_t` | `struct` | `rs/fq/src/socks.rs:81` |
| `socket_ctx_t` | `struct` | `rs/fq/src/packet_loop.rs:139` |
| `socket_t` | `struct` | `rs/fq/src/socks.rs:66` |
| `spinbit_def_t` | `struct` | `rs/fq/src/internal.rs:387` |
| `spinbit_version_enum` | `enum` | `rs/fq/src/lib.rs:389` |
| `splay_node_t` | `struct` | `rs/fq/src/splay.rs:89` |
| `splay_tree_t` | `struct` | `rs/fq/src/splay.rs:127` |
| `state_enum` | `enum` | `rs/fq/src/lib.rs:287` |
| `stateless_packet_t` | `struct` | `rs/fq/src/internal.rs:401` |
| `stored_ticket_t` | `struct` | `rs/fq/src/internal.rs:543` |
| `stored_token_t` | `struct` | `rs/fq/src/internal.rs:646` |
| `stream_data_buffer_argument_t` | `struct` | `rs/fq/src/internal.rs:2828` |
| `stream_data_node_t` | `struct` | `rs/fq/src/internal.rs:434` |
| `stream_head_t` | `struct` | `rs/fq/src/internal.rs:935` |
| `stream_queue_node_t` | `struct` | `rs/fq/src/internal.rs:444` |
| `test_sim_link_t` | `struct` | `rs/fq/src/utils.rs:927` |
| `test_sim_packet_t` | `struct` | `rs/fq/src/utils.rs:854` |
| `thread_t` | `struct` | `rs/fq/src/utils.rs:697` |
| `tls_ctx_t` | `struct` | `rs/fq/src/crypto_provider_api.rs:541` |
| `tp_0rtt_enum` | `enum` | `rs/fq/src/internal.rs:530` |
| `tp_preferred_address_t` | `struct` | `rs/fq/src/lib.rs:533` |
| `tp_t` | `struct` | `rs/fq/src/lib.rs:570` |
| `tp_version_negotiation_t` | `struct` | `rs/fq/src/lib.rs:549` |
| `tuple_t` | `struct` | `rs/fq/src/internal.rs:1145` |
| `version_parameters_t` | `struct` | `rs/fq/src/internal.rs:286` |

## Cross-module imports

Items pulled in via `use crate::…`, grouped by
source module.  A type imported by many modules but defined
in one place is the healthy pattern; a type imported via
two different source paths is a smell.

| Source module | Items imported | Importers |
|---|---|---|
| `Error` | `*` | 14: binlog.rs, bytestream.rs, config.rs, crypto_provider_api.rs, hash.rs, … (9 more) |
| `config::quic_config_t` | `*` | 1: packet_loop.rs |
| `connection_id_t` | `*` | 1: bytestream.rs |
| `crypto_provider_api::ptls_cipher_suite_t` | `*` | 1: tls_api.rs |
| `hash` | `hash_item`, `hash_table` | 1: internal.rs |
| `internal` | `cnx_t`, `packet_header`, `packet_type_enum`, `path_t`, `quic_t` | 3: binlog.rs, lib.rs, unified_log.rs |
| `internal::crypto_context_t` | `*` | 1: tls_api.rs |
| `quic_t` | `*` | 3: performance_log.rs, qlog.rs, socks.rs |
| `splay` | `splay_node_t`, `splay_tree_t` | 1: internal.rs |
| `unified_log::UnifiedLogging` | `*` | 1: internal.rs |
| `utils` | `TestAqm`, `ThreadFn`, `test_sim_link_t`, `test_sim_packet_t`, `thread_t` | 2: packet_loop.rs, test_dualq.rs |
| `utils::file_t` | `*` | 1: binlog.rs |

## Per-file summary

| File | LOC | Traits | Structs | Enums | Type aliases | Fns | Inner #![allow] |
|---|---:|---:|---:|---:|---:|---:|---:|
| `rs/fq/src/binlog.rs` | 349 | 0 | 0 | 1 | 0 | 14 | 0 |
| `rs/fq/src/bytestream.rs` | 406 | 0 | 2 | 1 | 0 | 38 | 1 |
| `rs/fq/src/cc_common.rs` | 244 | 0 | 2 | 1 | 0 | 15 | 0 |
| `rs/fq/src/config.rs` | 369 | 0 | 1 | 1 | 0 | 9 | 1 |
| `rs/fq/src/crypto_provider_api.rs` | 582 | 15 | 13 | 0 | 0 | 26 | 1 |
| `rs/fq/src/hash.rs` | 263 | 1 | 2 | 0 | 0 | 9 | 0 |
| `rs/fq/src/internal.rs` | 3485 | 5 | 32 | 5 | 0 | 275 | 3 |
| `rs/fq/src/lb.rs` | 238 | 0 | 3 | 1 | 0 | 5 | 1 |
| `rs/fq/src/lib.rs` | 2337 | 10 | 13 | 12 | 0 | 211 | 3 |
| `rs/fq/src/logger.rs` | 104 | 0 | 0 | 0 | 0 | 4 | 0 |
| `rs/fq/src/packet_loop.rs` | 751 | 4 | 6 | 1 | 0 | 14 | 2 |
| `rs/fq/src/performance_log.rs` | 104 | 0 | 0 | 1 | 0 | 2 | 0 |
| `rs/fq/src/qlog.rs` | 37 | 0 | 0 | 0 | 0 | 1 | 0 |
| `rs/fq/src/siphash.rs` | 38 | 0 | 0 | 0 | 0 | 1 | 0 |
| `rs/fq/src/socks.rs` | 453 | 0 | 7 | 0 | 0 | 20 | 0 |
| `rs/fq/src/splay.rs` | 250 | 1 | 2 | 0 | 0 | 12 | 0 |
| `rs/fq/src/test_dualq.rs` | 257 | 0 | 2 | 0 | 0 | 3 | 2 |
| `rs/fq/src/tls_api.rs` | 1003 | 0 | 3 | 0 | 0 | 53 | 0 |
| `rs/fq/src/unified_log.rs` | 402 | 1 | 0 | 0 | 0 | 16 | 1 |
| `rs/fq/src/utils.rs` | 1103 | 2 | 6 | 1 | 0 | 91 | 2 |

