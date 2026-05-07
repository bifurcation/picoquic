# Phase 4A Function Map and Implementation Plan

Status: draft

This plan is generated from `xlate/function_translation_map.json`.
Review the `required_missing`, `expected_omission`, and `blocked`
entries before approving Phase 4B.

## Summary

* `implemented`: 1156
* `required_missing`: 172
* `expected_omission`: 279
* `blocked`: 0

Total in-scope C functions: 1607

## Required Missing Implementations

| C function | C span | Rust span | Action | Proposed destination | Reason |
| --- | --- | --- | --- | --- | --- |
| `picoquic_bbr_reset` | `picoquic/bbr.c:598-601` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr_init` | `picoquic/bbr.c:603-612` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRInflight` | `picoquic/bbr.c:909-912` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr1_init` | `picoquic/bbr1.c:469-479` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1HandleRestartFromIdle` | `picoquic/bbr1.c:1057-1066` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1OnTransmit` | `picoquic/bbr1.c:1083-1086` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1OnAllPacketsLost` | `picoquic/bbr1.c:1091-1095` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1OnEnterFastRecovery` | `picoquic/bbr1.c:1097-1105` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_command_line_option_value` | `picoquic/config.c:683-720` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_configure_quic_ctx` | `picoquic/ech.c:333-366` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_apply_reset_stream_frame` | `picoquic/frames.c:330-371` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_data_repeat_adjust` | `picoquic/frames.c:2203-2252` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_copy_single_stream_frame_for_retransmit` | `picoquic/frames.c:2399-2460` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_estimate_path_bandwidth` | `picoquic/frames.c:2865-2922` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_max_path_id_frame` | `picoquic/frames.c:6012-6026` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_paths_blocked_frame` | `picoquic/frames.c:6128-6142` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_path_cid_blocked_frame` | `picoquic/frames.c:6257-6276` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `binlog_app_message` | `picoquic/logwriter.c:1300-1307` | - | `required_missing` | `rs/fq/src/binlog.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_screen_initial_packet` | `picoquic/packet.c:85-207` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_header_protection` | `picoquic/packet.c:617-631` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_packet_protection` | `picoquic/packet.c:633-768` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_incoming_version_negotiation` | `picoquic/packet.c:904-986` | rs/fq/src/lib.rs:2719-2730 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_prepare_version_negotiation` | `picoquic/packet.c:994-1075` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_process_unexpected_cnxid` | `picoquic/packet.c:1077-1138` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_stateless_retry` | `picoquic/packet.c:1144-1208` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_retry_packet` | `picoquic/packet.c:1210-1238` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_busy_packet` | `picoquic/packet.c:1240-1315` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_incoming_retry` | `picoquic/packet.c:1521-1613` | rs/fq/src/lib.rs:2719-2730 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_server_initial` | `picoquic/packet.c:1619-1701` | rs/fq/src/lib.rs:2719-2730 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_server_handshake` | `picoquic/packet.c:1704-1752` | rs/fq/src/lib.rs:2719-2730 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_stateless_reset` | `picoquic/packet.c:1813-1829` | rs/fq/src/lib.rs:2719-2730 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_ecn_accounting` | `picoquic/packet.c:1880-1908` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_incoming_not_decrypted` | `picoquic/packet.c:2019-2067` | rs/fq/src/lib.rs:2719-2730 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_segment` | `picoquic/packet.c:2073-2387` | rs/fq/src/lib.rs:2719-2730 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_packet_ex` | `picoquic/packet.c:2389-2430` | rs/fq/src/lib.rs:2719-2730 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_packet` | `picoquic/packet.c:2432-2447` | rs/fq/src/lib.rs:2703-2714 `incoming_packet` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_prepare_tuple_challenge_frames` | `picoquic/paths.c:33-138` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_check_path_control_needed` | `picoquic/paths.c:288-321` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sort_available_paths` | `picoquic/paths.c:379-486` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_generate_clear` | `picoquic/picoquic_lb.c:54-59` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_generate_stream_cipher` | `picoquic/picoquic_lb.c:84-101` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_generate_block_cipher` | `picoquic/picoquic_lb.c:110-121` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_verify_stream_cipher` | `picoquic/picoquic_lb.c:163-186` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_verify_block_cipher` | `picoquic/picoquic_lb.c:188-205` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ptls_minicrypto_load` | `picoquic/picoquic_ptls_minicrypto.c:54-83` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `set_openssl_private_key_from_key_file` | `picoquic/picoquic_ptls_openssl.c:138-158` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_openssl_get_certificate_verifier` | `picoquic/picoquic_ptls_openssl.c:277-292` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ptls_openssl_load` | `picoquic/picoquic_ptls_openssl.c:406-454` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ptls_openssl_log_version` | `picoquic/picoquic_ptls_openssl.c:456-464` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_socket_set_ecn_options_ex` | `picoquic/picosocks.c:93-223` | - | `required_missing` | `rs/fq/src/socks.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_socket_set_ecn_options` | `picoquic/picosocks.c:225-228` | - | `required_missing` | `rs/fq/src/socks.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_send_through_socket` | `picoquic/picosocks.c:1262-1271` | - | `required_missing` | `rs/fq/src/socks.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `zig` | `picoquic/picosplay.c:59-62` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `zigzig` | `picoquic/picosplay.c:64-70` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `zigzag` | `picoquic/picosplay.c:72-78` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picosplay_new_tree` | `picoquic/picosplay.c:90-97` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_init` | `picoquic/prague.c:126-142` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_reset_l3s` | `picoquic/prague.c:156-163` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_reset` | `picoquic/prague.c:166-170` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_initialize_era` | `picoquic/prague.c:172-184` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_enter_recovery` | `picoquic/prague.c:186-208` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_update_alpha` | `picoquic/prague.c:210-249` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_process_ack` | `picoquic/prague.c:251-292` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_process_start_ack` | `picoquic/prague.c:294-317` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_notify` | `picoquic/prague.c:320-394` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_create` | `picoquic/quicctx.c:633-775` | rs/fq/src/lib.rs:1311-1500 `new` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_unregister_net_id` | `picoquic/quicctx.c:1280-1290` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_register_net_id` | `picoquic/quicctx.c:1292-1310` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_unregister_net_icid` | `picoquic/quicctx.c:1356-1363` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_unregister_net_secret` | `picoquic/quicctx.c:1365-1372` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_cnx_from_list` | `picoquic/quicctx.c:1450-1469` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_wake_list_init` | `picoquic/quicctx.c:1499-1503` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_cnx_from_wake_list` | `picoquic/quicctx.c:1505-1508` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_insert_cnx_by_wake_time` | `picoquic/quicctx.c:1510-1513` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_create_random_cnx_id` | `picoquic/quicctx.c:1630-1639` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_clear_path_data` | `picoquic/quicctx.c:1885-1899` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_notify_destination_unreachable` | `picoquic/quicctx.c:2217-2244` | rs/fq/src/lib.rs:2817-2826 `notify_destination_unreachable` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_notify_destination_unreachable_by_cnxid` | `picoquic/quicctx.c:2246-2262` | rs/fq/src/lib.rs:2833-2843 `notify_destination_unreachable_by_connection_id` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_check_new_path_allowed` | `picoquic/quicctx.c:2300-2342` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_path_quality_from_context` | `picoquic/quicctx.c:2678-2699` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_subscribe_to_quality_update_per_path_context` | `picoquic/quicctx.c:2721-2727` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_stashed_cnxid` | `picoquic/quicctx.c:3093-3100` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_dereference_stashed_cnxid` | `picoquic/quicctx.c:3149-3152` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_mark_direct_receive_stream` | `picoquic/quicctx.c:3703-3759` | rs/fq/src/lib.rs:2926-2933 `mark_direct_receive_stream` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_create_local_cnxid` | `picoquic/quicctx.c:3795-3866` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_retire_local_cnxid` | `picoquic/quicctx.c:3965-3985` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_find_local_cnxid` | `picoquic/quicctx.c:4017-4034` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_create_cnx_internal` | `picoquic/quicctx.c:4039-4348` | rs/fq/src/internal.rs:2730-3463 `create_cnx_internal` | `required_missing` | `rs/fq/src/internal.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_get_default_callback_context` | `picoquic/quicctx.c:4779-4783` | rs/fq/src/lib.rs:2689-2731 `default_callback_ctx` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_reset_cnx` | `picoquic/quicctx.c:4939-4989` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_cnx_by_id` | `picoquic/quicctx.c:5216-5240` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_cnx_by_net` | `picoquic/quicctx.c:5242-5256` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_cnx_by_icid` | `picoquic/quicctx.c:5258-5275` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_cnx_by_secret` | `picoquic/quicctx.c:5277-5292` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_rtt` | `picoquic/quicctx.c:5438-5442` | rs/fq/src/lib.rs:3560-3587 `rtt` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_set_verify_certificate_callback` | `picoquic/quicctx.c:5486-5492` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_remote_stream_error` | `picoquic/quicctx.c:5520-5529` | rs/fq/src/lib.rs:3401-3410 `remote_stream_error` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_uniform_random` | `picoquic/quicctx.c:5600-5604` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_first_item` | `picoquic/sacks.c:68-72` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_last_item` | `picoquic/sacks.c:74-77` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_insert_item` | `picoquic/sacks.c:89-108` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_find_range_below_number` | `picoquic/sacks.c:156-168` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_update_sack_list` | `picoquic/sacks.c:197-256` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_check_sack_list` | `picoquic/sacks.c:323-340` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_first` | `picoquic/sacks.c:407-412` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_last` | `picoquic/sacks.c:414-420` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_first_range` | `picoquic/sacks.c:422-428` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_reset` | `picoquic/sacks.c:439-447` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_stream_not_coalesced` | `picoquic/sender.c:180-192` | rs/fq/src/lib.rs:2997-3004 `set_stream_not_coalesced` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_set_stream_priority` | `picoquic/sender.c:213-226` | rs/fq/src/lib.rs:3007-3014 `set_stream_priority` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_mark_high_priority_stream` | `picoquic/sender.c:228-243` | rs/fq/src/lib.rs:3018-3025 `mark_high_priority_stream` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_reset_stream_at` | `picoquic/sender.c:379-407` | rs/fq/src/lib.rs:3192-3200 `reset_stream_at` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_reset_stream` | `picoquic/sender.c:408-412` | rs/fq/src/lib.rs:3185-3188 `reset_stream` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_stop_sending` | `picoquic/sender.c:430-458` | rs/fq/src/lib.rs:3263-3266 `stop_sending` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_discard_stream` | `picoquic/sender.c:460-490` | rs/fq/src/lib.rs:3270-3277 `discard_stream` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_insert_hole_in_send_sequence_if_needed` | `picoquic/sender.c:1133-1169` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_is_pkt_ctx_backlog_empty` | `picoquic/sender.c:1274-1308` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_preemptive_retransmit_packet` | `picoquic/sender.c:1332-1419` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_preemptive_retransmit_in_context` | `picoquic/sender.c:1421-1492` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_preemptive_retransmit_as_needed` | `picoquic/sender.c:1494-1543` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_is_mtu_probe_needed` | `picoquic/sender.c:1587-1628` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_mtu_probe` | `picoquic/sender.c:1630-1647` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_0rtt` | `picoquic/sender.c:1649-1743` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_old_context` | `picoquic/sender.c:1771-1833` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_server_address_migration` | `picoquic/sender.c:1868-1930` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_client_init` | `picoquic/sender.c:1932-2213` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_server_init` | `picoquic/sender.c:2215-2349` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_closing` | `picoquic/sender.c:2351-2594` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_new_local_id_as_needed` | `picoquic/sender.c:2596-2658` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_datagram_ready` | `picoquic/sender.c:2777-2801` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_stream_and_datagrams` | `picoquic/sender.c:2848-2962` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_almost_ready` | `picoquic/sender.c:2964-3284` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_ready` | `picoquic/sender.c:3286-3717` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_check_idle_timer` | `picoquic/sender.c:3719-3759` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_segment` | `picoquic/sender.c:3761-3829` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_path_addresses_from_tuple` | `picoquic/sender.c:3832-3846` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_handle_send_timers` | `picoquic/sender.c:3905-3932` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_handle_send_paths` | `picoquic/sender.c:3934-3957` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_handle_send_train_statistics` | `picoquic/sender.c:3959-3979` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_ex` | `picoquic/sender.c:3981-4181` | rs/fq/src/lib.rs:2795-2813 `prepare_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_prepare_packet` | `picoquic/sender.c:4183-4194` | rs/fq/src/lib.rs:2806-2826 `prepare_packet` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_close_ex` | `picoquic/sender.c:4201-4222` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_next_packet_ex` | `picoquic/sender.c:4244-4324` | rs/fq/src/lib.rs:2759-2777 `prepare_next_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquictest_sim_link_wifi_jitter` | `picoquic/sim_link.c:201-228` | - | `required_missing` | `rs/fq/src/tests/harness.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquictest_sim_link_jitter` | `picoquic/sim_link.c:230-247` | - | `required_missing` | `rs/fq/src/tests/harness.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_packet_loop_open_socket` | `picoquic/sockloop.c:363-462` | - | `required_missing` | `rs/fq/src/packet_loop.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_packet_loop_poll` | `picoquic/sockloop.c:907-992` | - | `required_missing` | `rs/fq/src/packet_loop.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_spinbit_random_outgoing` | `picoquic/spinbit.c:72-76` | - | `required_missing` | `rs/fq/src/spinbit.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_update_stored_ticket` | `picoquic/ticket_store.c:529-571` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_validate_bdp_seed` | `picoquic/timing.c:90-114` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_cipher_suite_by_id` | `picoquic/tls_api.c:435-449` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_aes128gcm_sha256` | `picoquic/tls_api.c:709-713` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_aes128gcm_v` | `picoquic/tls_api.c:720-729` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_cipher_suite_by_id_v` | `picoquic/tls_api.c:731-734` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_crypto_uniform_random` | `picoquic/tls_api.c:778-788` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_public_random_seed` | `picoquic/tls_api.c:859-866` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_public_random` | `picoquic/tls_api.c:868-880` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_public_uniform_random` | `picoquic/tls_api.c:882-892` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_key_from_secret` | `picoquic/tls_api.c:1342-1361` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_compute_initial_secrets` | `picoquic/tls_api.c:1478-1498` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_server_setup_ticket_aead_contexts` | `picoquic/tls_api.c:2414-2442` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_server_encrypt_retry_token` | `picoquic/tls_api.c:2849-2886` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_varint_decode` | `picoquic/transport.c:31-41` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_varint_encode` | `picoquic/transport.c:43-57` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_type_varint_encode` | `picoquic/transport.c:59-66` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_type_flag_encode` | `picoquic/transport.c:68-75` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_cid_encode` | `picoquic/transport.c:77-85` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_cid_decode` | `picoquic/transport.c:87-96` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_encode_transport_preferred_address_address` | `picoquic/transport.c:98-128` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_decode_transport_preferred_address_address` | `picoquic/transport.c:130-162` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_encode_transport_param_version_negotiation` | `picoquic/transport.c:173-224` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_clear_transport_extensions` | `picoquic/transport.c:503-532` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |


## Blocked Classifications

| C function | C span | Rust span | Action | Proposed destination | Reason |
| --- | --- | --- | --- | --- | --- |
| _none_ | | | | | |


## Expected Omissions

| C function | C span | Rust span | Action | Proposed destination | Reason |
| --- | --- | --- | --- | --- | --- |
| `picoquic_bbr_delete` | `picoquic/bbr.c:616-623` | - | `expected_omission` | `rs/fq/src/bbr.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_bbr1_delete` | `picoquic/bbr1.c:481-488` | - | `expected_omission` | `rs/fq/src/bbr1.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `c4_delete` | `picoquic/c4.c:1111-1118` | - | `expected_omission` | `rs/fq/src/c4.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `cubic_delete` | `picoquic/cubic.c:551-558` | - | `expected_omission` | `rs/fq/src/cubic.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `ech_dispose_opener_callback` | `picoquic/ech.c:269-278` | - | `expected_omission` | `rs/fq/src/ech.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_release_quic_ech_ctx` | `picoquic/ech.c:375-389` | - | `expected_omission` | `rs/fq/src/ech.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_is_stream_acked` | `picoquic/frames.c:117-132` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_flow_control_check_stream_offset` | `picoquic/frames.c:204-228` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_skip_reset_stream_frame` | `picoquic/frames.c:236-250` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_reset_stream_frame` | `picoquic/frames.c:373-394` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_reset_stream_frame` | `picoquic/frames.c:396-425` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_reset_stream_needs_repeat` | `picoquic/frames.c:427-449` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_reset_stream_at_frame` | `picoquic/frames.c:459-478` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_reset_stream_at_frame` | `picoquic/frames.c:507-526` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_reset_stream_at_frame` | `picoquic/frames.c:528-558` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_reset_stream_at_needs_repeat` | `picoquic/frames.c:560-587` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_new_connection_id_frame` | `picoquic/frames.c:622-636` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_new_connection_id_frame` | `picoquic/frames.c:638-659` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_new_connection_id_frame` | `picoquic/frames.c:661-727` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_new_cid_frame` | `picoquic/frames.c:729-771` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_new_cid_needs_repeat` | `picoquic/frames.c:773-810` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_retire_connection_id_frame` | `picoquic/frames.c:864-877` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_retire_connection_id_frame` | `picoquic/frames.c:879-898` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_retire_connection_id_frame` | `picoquic/frames.c:900-932` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_retire_connection_id_needs_repeat` | `picoquic/frames.c:938-970` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_retire_connection_id_frame` | `picoquic/frames.c:972-1007` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_new_token_frame` | `picoquic/frames.c:1045-1048` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_new_token_frame` | `picoquic/frames.c:1050-1081` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_stop_sending_frame` | `picoquic/frames.c:1115-1155` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_stop_sending_frame` | `picoquic/frames.c:1157-1163` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_stop_sending_needs_repeat` | `picoquic/frames.c:1166-1192` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_stream_data_chunk_callback` | `picoquic/frames.c:1262-1289` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_stream_data_callback` | `picoquic/frames.c:1291-1306` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `add_chunk_node` | `picoquic/frames.c:1308-1343` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_stream_network_input` | `picoquic/frames.c:1407-1517` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_is_last_stream_frame` | `picoquic/frames.c:1519-1525` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_queue_data_repeat_delete` | `picoquic/frames.c:2174-2186` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_parse_crypto_hs_frame` | `picoquic/frames.c:2526-2538` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_crypto_stream_from_ptype` | `picoquic/frames.c:2576-2598` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_process_ack_of_crypto_frame` | `picoquic/frames.c:2600-2624` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_crypto_frame_needs_repeat` | `picoquic/frames.c:2626-2652` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_spurious_retransmission` | `picoquic/frames.c:2757-2840` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_dequeue_old_retransmitted_packets` | `picoquic/frames.c:2842-2863` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `process_decoded_packet_data` | `picoquic/frames.c:3174-3230` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_find_acked_packet` | `picoquic/frames.c:3232-3251` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_process_ack_of_ack_body` | `picoquic/frames.c:3253-3350` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_path_ack_frame` | `picoquic/frames.c:3372-3410` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_range` | `picoquic/frames.c:3852-3918` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_decode_ack_frame` | `picoquic/frames.c:3920-4071` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_connection_close_frame` | `picoquic/frames.c:4395-4424` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_application_close_frame` | `picoquic/frames.c:4447-4473` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_max_data_frame` | `picoquic/frames.c:4503-4515` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_max_data_frame` | `picoquic/frames.c:4517-4539` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_max_stream_data_frame` | `picoquic/frames.c:4568-4600` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_max_stream_data_frame` | `picoquic/frames.c:4602-4629` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_max_streams_frame` | `picoquic/frames.c:4724-4761` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_max_streams_frame` | `picoquic/frames.c:4763-4791` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_max_streams_frame_needs_repeat` | `picoquic/frames.c:4793-4821` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_challenge_frame` | `picoquic/frames.c:4902-4987` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_response_frame` | `picoquic/frames.c:5004-5063` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_blocked_frame` | `picoquic/frames.c:5106-5113` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_stream_blocked_frame` | `picoquic/frames.c:5116-5131` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_streams_blocked_frame` | `picoquic/frames.c:5134-5150` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_0len_frame` | `picoquic/frames.c:5153-5160` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_handshake_done_frame` | `picoquic/frames.c:5162-5189` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_datagram_frame` | `picoquic/frames.c:5204-5227` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_datagram_frame` | `picoquic/frames.c:5247-5286` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_ack_frequency_frame` | `picoquic/frames.c:5526-5537` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_ack_frequency_frame` | `picoquic/frames.c:5552-5592` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_immediate_ack_frame` | `picoquic/frames.c:5642-5648` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_immediate_ack_frame` | `picoquic/frames.c:5650-5667` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_time_stamp_frame` | `picoquic/frames.c:5680-5687` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_time_stamp_frame` | `picoquic/frames.c:5689-5694` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_time_stamp_frame` | `picoquic/frames.c:5696-5717` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_path_abandon_frame` | `picoquic/frames.c:5752-5759` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_abandon_frame` | `picoquic/frames.c:5761-5828` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_path_available_or_backup_frame` | `picoquic/frames.c:5921-5928` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_available_or_backup_frame` | `picoquic/frames.c:5930-5970` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_path_available_or_backup_frame_need_repeat` | `picoquic/frames.c:5972-5997` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_max_path_id_frame` | `picoquic/frames.c:6028-6033` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_max_path_id_frame` | `picoquic/frames.c:6035-6040` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_max_path_id_frame` | `picoquic/frames.c:6042-6066` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_max_path_id_frame_needs_repeat` | `picoquic/frames.c:6068-6088` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_max_path_id_frame` | `picoquic/frames.c:6091-6112` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_paths_blocked_frame` | `picoquic/frames.c:6144-6149` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_paths_blocked_frame` | `picoquic/frames.c:6151-6156` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_paths_blocked_frame` | `picoquic/frames.c:6158-6176` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_paths_blocked_frame_needs_repeat` | `picoquic/frames.c:6178-6198` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_paths_blocked_frame` | `picoquic/frames.c:6201-6222` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_path_cid_blocked_frame` | `picoquic/frames.c:6278-6285` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_path_cid_blocked_frame` | `picoquic/frames.c:6287-6294` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_cid_blocked_frame` | `picoquic/frames.c:6296-6315` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_path_cid_blocked_frame_needs_repeat` | `picoquic/frames.c:6317-6353` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_path_cid_blocked_frame` | `picoquic/frames.c:6355-6380` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_observed_address_frame` | `picoquic/frames.c:6467-6477` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_observed_address_frame` | `picoquic/frames.c:6494-6535` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_observed_address_frame` | `picoquic/frames.c:6537-6553` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_bdp_frame` | `picoquic/frames.c:6559-6568` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_bdp_frame` | `picoquic/frames.c:6570-6587` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_bdp_frame` | `picoquic/frames.c:6589-6635` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_stream_frame` | `picoquic/frames.c:6992-7008` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_crypto_hs_frame` | `picoquic/frames.c:7014-7020` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_connection_close_frame` | `picoquic/frames.c:7022-7033` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_application_close_frame` | `picoquic/frames.c:7035-7044` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_ack_frame_maybe_ecn` | `picoquic/frames.c:7047-7082` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_ack_frame` | `picoquic/frames.c:7084-7086` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_ack_ecn_frame` | `picoquic/frames.c:7088-7090` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_max_stream_data_frame` | `picoquic/frames.c:7095-7101` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_stream_blocked_frame` | `picoquic/frames.c:7103-7109` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `textlog_time` | `picoquic/logger.c:41-50` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_address` | `picoquic/logger.c:59-88` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_packet_address` | `picoquic/logger.c:90-117` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_ptype_name` | `picoquic/logger.c:193-224` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_frame_names` | `picoquic/logger.c:226-363` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_connection_id` | `picoquic/logger.c:365-372` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_packet_header` | `picoquic/logger.c:374-435` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_negotiation_packet` | `picoquic/logger.c:437-453` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_retry_packet` | `picoquic/logger.c:455-491` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_stream_frame` | `picoquic/logger.c:493-521` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_ack_frame` | `picoquic/logger.c:523-663` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_reset_stream_frame` | `picoquic/logger.c:665-697` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_reset_stream_at_frame` | `picoquic/logger.c:699-738` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_stop_sending_frame` | `picoquic/logger.c:740-765` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_reason_text` | `picoquic/logger.c:767-786` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_generic_close_frame` | `picoquic/logger.c:788-848` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_max_data_frame` | `picoquic/logger.c:855-873` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_max_stream_data_frame` | `picoquic/logger.c:875-897` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_max_stream_id_frame` | `picoquic/logger.c:899-917` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_blocked_frame` | `picoquic/logger.c:919-940` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_stream_blocked_frame` | `picoquic/logger.c:942-962` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_streams_blocked_frame` | `picoquic/logger.c:964-981` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_new_connection_id_frame` | `picoquic/logger.c:983-1049` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_retire_connection_id_frame` | `picoquic/logger.c:1051-1085` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_new_token_frame` | `picoquic/logger.c:1087-1118` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_path_frame` | `picoquic/logger.c:1120-1143` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_crypto_hs_frame` | `picoquic/logger.c:1145-1181` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_datagram_frame` | `picoquic/logger.c:1183-1225` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_ack_frequency_frame` | `picoquic/logger.c:1227-1261` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_immediate_ack_frame` | `picoquic/logger.c:1263-1276` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_time_stamp_frame` | `picoquic/logger.c:1277-1307` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_path_abandon_frame` | `picoquic/logger.c:1309-1342` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_path_available_or_backup_frame` | `picoquic/logger.c:1344-1376` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_max_path_id_frame` | `picoquic/logger.c:1378-1406` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_paths_blocked_frame` | `picoquic/logger.c:1408-1437` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_path_cid_blocked_frame` | `picoquic/logger.c:1439-1470` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_bdp_frame` | `picoquic/logger.c:1473-1516` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_observed_address_frame` | `picoquic/logger.c:1518-1551` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_frames` | `picoquic/logger.c:1553-1740` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_decrypted_segment` | `picoquic/logger.c:1742-1798` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_outgoing_segment` | `picoquic/logger.c:1800-1850` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_transport_extension` | `picoquic/logger.c:1949-1956` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_negotiated_alpn` | `picoquic/logger.c:1958-1985` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_congestion_state` | `picoquic/logger.c:1987-2000` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_tls_ticket` | `picoquic/logger.c:2016-2110` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_picotls_ticket` | `picoquic/logger.c:2124-2189` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_txtlog_message_v` | `picoquic/logger.c:2195-2207` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `txtlog_context_free_app_message` | `picoquic/logger.c:2209-2214` | - | `expected_omission` | `rs/fq/src/logger.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `textlog_quic_pdu` | `picoquic/logger.c:2223-2234` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_pdu_ex` | `picoquic/logger.c:2236-2250` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_packet` | `picoquic/logger.c:2252-2259` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_tls_ticket` | `picoquic/logger.c:2355-2361` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_log_reset_stream_frame` | `picoquic/logwriter.c:173-184` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_reset_stream_at_frame` | `picoquic/logwriter.c:186-198` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_stop_sending_frame` | `picoquic/logwriter.c:200-210` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_max_data_frame` | `picoquic/logwriter.c:241-250` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_max_stream_data_frame` | `picoquic/logwriter.c:252-262` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_max_stream_id_frame` | `picoquic/logwriter.c:264-273` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_blocked_frame` | `picoquic/logwriter.c:275-284` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_stream_blocked_frame` | `picoquic/logwriter.c:286-296` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_streams_blocked_frame` | `picoquic/logwriter.c:298-307` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_retire_connection_id_frame` | `picoquic/logwriter.c:344-353` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_path_retire_connection_id_frame` | `picoquic/logwriter.c:355-365` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_perflog_item_free` | `picoquic/performance_log.c:69-75` | - | `expected_omission` | `rs/fq/src/performance_log.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_perflog_free` | `picoquic/performance_log.c:212-223` | - | `expected_omission` | `rs/fq/src/performance_log.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picohash_delete_key` | `picoquic/picohash.c:143-153` | - | `expected_omission` | `rs/fq/src/hash.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_openssl_dispose_sign_certificate` | `picoquic/picoquic_ptls_openssl.c:202-207` | - | `expected_omission` | `rs/fq/src/sys/openssl.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_openssl_dispose_certificate_verifier` | `picoquic/picoquic_ptls_openssl.c:269-275` | - | `expected_omission` | `rs/fq/src/sys/openssl.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `openssl_keyex_dispose` | `picoquic/picoquic_ptls_openssl.c:367-371` | - | `expected_omission` | `rs/fq/src/sys/openssl.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_prague_delete` | `picoquic/prague.c:396-403` | - | `expected_omission` | `rs/fq/src/prague.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_local_cnxid_hash` | `picoquic/quicctx.c:268-273` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_local_cnxid_compare` | `picoquic/quicctx.c:275-281` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_local_cnxid_to_item` | `picoquic/quicctx.c:283-288` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_id_hash` | `picoquic/quicctx.c:290-296` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_local_netid_to_item` | `picoquic/quicctx.c:298-303` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_id_compare` | `picoquic/quicctx.c:306-312` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_icid_hash` | `picoquic/quicctx.c:314-325` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_icid_compare` | `picoquic/quicctx.c:327-337` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_icid_to_item` | `picoquic/quicctx.c:339-344` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_secret_hash` | `picoquic/quicctx.c:346-357` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_secret_compare` | `picoquic/quicctx.c:359-373` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_secret_to_item` | `picoquic/quicctx.c:375-380` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_issued_ticket_hash` | `picoquic/quicctx.c:403-408` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_issued_ticket_compare` | `picoquic/quicctx.c:410-417` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_issued_ticket_key_to_item` | `picoquic/quicctx.c:419-424` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_delete_issued_ticket` | `picoquic/quicctx.c:461-483` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_registered_token_compare` | `picoquic/quicctx.c:528-549` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_registered_token_value` | `picoquic/quicctx.c:557-560` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_registered_token_delete` | `picoquic/quicctx.c:562-566` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_stateless_packet` | `picoquic/quicctx.c:1226-1229` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_wake_list_node_value` | `picoquic/quicctx.c:1473-1476` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_wake_list_compare` | `picoquic/quicctx.c:1478-1484` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_wake_list_create_node` | `picoquic/quicctx.c:1486-1489` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_wake_list_delete_node` | `picoquic/quicctx.c:1491-1497` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_remote_cnxid_stashes` | `picoquic/quicctx.c:3271-3276` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_stream_data_node_compare` | `picoquic/quicctx.c:3339-3344` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_data_node_create` | `picoquic/quicctx.c:3346-3349` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_data_node_value` | `picoquic/quicctx.c:3352-3355` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_data_node_delete` | `picoquic/quicctx.c:3370-3375` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_stream_node_compare` | `picoquic/quicctx.c:3410-3414` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_node_create` | `picoquic/quicctx.c:3416-3419` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_node_value` | `picoquic/quicctx.c:3422-3425` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_node_delete` | `picoquic/quicctx.c:3448-3455` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_local_cnxid_listed` | `picoquic/quicctx.c:3868-3925` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_sack_node_value` | `picoquic/sacks.c:35-40` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_item_value` | `picoquic/sacks.c:42-45` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_item_compare` | `picoquic/sacks.c:47-53` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_node_create` | `picoquic/sacks.c:55-58` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_node_delete` | `picoquic/sacks.c:60-66` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_sack_delete_item` | `picoquic/sacks.c:109-119` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_sack_list_free` | `picoquic/sacks.c:449-457` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquictest_sim_link_delete` | `picoquic/sim_link.c:64-78` | - | `expected_omission` | `rs/fq/src/tests/harness.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_network_thread` | `picoquic/sockloop.c:1851-1875` | - | `expected_omission` | `rs/fq/src/packet_loop.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_free_tickets` | `picoquic/ticket_store.c:500-509` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_tls_api_init_providers` | `picoquic/tls_api.c:143-181` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_api_zero` | `picoquic/tls_api.c:183-202` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_api_log_versions` | `picoquic/tls_api.c:204-227` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_api_unload` | `picoquic/tls_api.c:238-245` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_ciphersuite` | `picoquic/tls_api.c:258-274` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_key_exchange_algorithm` | `picoquic/tls_api.c:276-292` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_hpke_cipher_suite` | `picoquic/tls_api.c:294-306` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_hpke_kem` | `picoquic/tls_api.c:308-319` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_tls_key_provider_fn` | `picoquic/tls_api.c:321-337` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_verify_certificate_fn` | `picoquic/tls_api.c:339-346` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_explain_crypto_error_fn` | `picoquic/tls_api.c:348-353` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_crypto_random_provider_fn` | `picoquic/tls_api.c:355-358` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_keyex_from_key_file_fn` | `picoquic/tls_api.c:360-365` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_cipher_suite_list` | `picoquic/tls_api.c:367-389` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_cipher_suite_in_ctx` | `picoquic/tls_api.c:391-425` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_key_exchange_in_ctx` | `picoquic/tls_api.c:549-571` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_random_provider_in_ctx` | `picoquic/tls_api.c:594-598` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_dispose_sign_certificate` | `picoquic/tls_api.c:616-629` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_get_certificate_verifier` | `picoquic/tls_api.c:643-654` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_dispose_certificate_verifier` | `picoquic/tls_api.c:656-664` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_log_crypto_errors` | `picoquic/tls_api.c:750-762` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_get_quic_extension_id` | `picoquic/tls_api.c:894-917` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_collect_extensions_cb` | `picoquic/tls_api.c:925-936` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_set_extensions` | `picoquic/tls_api.c:938-962` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_collected_extensions_cb` | `picoquic/tls_api.c:969-996` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_client_hello_call_back` | `picoquic/tls_api.c:1006-1069` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_server_encrypt_ticket_call_back` | `picoquic/tls_api.c:1085-1198` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_client_save_ticket_call_back` | `picoquic/tls_api.c:1205-1235` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_get_simulated_time_cb` | `picoquic/tls_api.c:1237-1244` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_dispose_verify_certificate_callback` | `picoquic/tls_api.c:1259-1277` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_tls_set_verify_certificate_callback` | `picoquic/tls_api.c:1279-1289` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_aes128_ecb_free` | `picoquic/tls_api.c:1332-1335` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_update_traffic_key_callback` | `picoquic/tls_api.c:1390-1419` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_update_traffic_key_callback` | `picoquic/tls_api.c:1421-1431` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `free_certificates_list` | `picoquic/tls_api.c:1863-1872` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_log_event_call_back` | `picoquic/tls_api.c:2002-2019` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_free_log_event` | `picoquic/tls_api.c:2021-2037` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_tlscontext_free` | `picoquic/tls_api.c:2086-2115` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_aead_free` | `picoquic/tls_api.c:2382-2385` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_cipher_free` | `picoquic/tls_api.c:2387-2390` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_one_retry_protection_context` | `picoquic/tls_api.c:3154-3165` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_free_tokens` | `picoquic/token_store.c:350-359` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_string_free` | `picoquic/util.c:86-93` | - | `expected_omission` | `rs/fq/src/utils.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_file_open_ex` | `picoquic/util.c:726-750` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_file_open` | `picoquic/util.c:751-754` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_file_close` | `picoquic/util.c:756-763` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_create_thread` | `picoquic/util.c:1127-1139` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_wait_thread` | `picoquic/util.c:1141-1152` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_delete_thread` | `picoquic/util.c:1154-1174` | - | `expected_omission` | `rs/fq/src/utils.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_mutex` | `picoquic/util.c:1190-1200` | - | `expected_omission` | `rs/fq/src/utils.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_lock_mutex` | `picoquic/util.c:1202-1214` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_unlock_mutex` | `picoquic/util.c:1216-1227` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_create_event` | `picoquic/util.c:1229-1247` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_delete_event` | `picoquic/util.c:1249-1259` | - | `expected_omission` | `rs/fq/src/utils.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_signal_event` | `picoquic/util.c:1261-1275` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_wait_for_event` | `picoquic/util.c:1277-1303` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

