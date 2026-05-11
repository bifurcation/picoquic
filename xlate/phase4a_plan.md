# Phase 4A Function Map and Implementation Plan

Status: draft

This plan is generated from `xlate/function_translation_map.json`.
Review the `required_missing`, `expected_omission`, and `blocked`
entries before approving Phase 4B.

## Summary

* `implemented`: 1410
* `required_missing`: 0
* `expected_omission`: 197
* `blocked`: 0

Total in-scope C functions: 1607

## Required Missing Implementations

| C function | C span | Rust span | Action | Proposed destination | Reason |
| --- | --- | --- | --- | --- | --- |
| _none_ | | | | | |


## Blocked Classifications

| C function | C span | Rust span | Action | Proposed destination | Reason |
| --- | --- | --- | --- | --- | --- |
| _none_ | | | | | |


## Expected Omissions

| C function | C span | Rust span | Action | Proposed destination | Reason |
| --- | --- | --- | --- | --- | --- |
| `picoquic_bbr1_delete` | `picoquic/bbr1.c:481-488` | - | `expected_omission` | `rs/fq/src/bbr1.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `ech_dispose_opener_callback` | `picoquic/ech.c:269-278` | - | `expected_omission` | `rs/fq/src/ech.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_flow_control_check_stream_offset` | `picoquic/frames.c:204-228` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_skip_reset_stream_frame` | `picoquic/frames.c:236-250` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_reset_stream_at_frame` | `picoquic/frames.c:459-478` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_new_connection_id_frame` | `picoquic/frames.c:622-636` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_retire_connection_id_frame` | `picoquic/frames.c:864-877` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_new_token_frame` | `picoquic/frames.c:1045-1048` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_stop_sending_frame` | `picoquic/frames.c:1157-1163` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `add_chunk_node` | `picoquic/frames.c:1308-1343` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_queue_data_repeat_delete` | `picoquic/frames.c:2174-2186` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_crypto_stream_from_ptype` | `picoquic/frames.c:2576-2598` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_check_spurious_retransmission` | `picoquic/frames.c:2757-2840` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_dequeue_old_retransmitted_packets` | `picoquic/frames.c:2842-2863` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_find_acked_packet` | `picoquic/frames.c:3232-3251` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_process_ack_of_ack_body` | `picoquic/frames.c:3253-3350` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_range` | `picoquic/frames.c:3852-3918` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_decode_application_close_frame` | `picoquic/frames.c:4447-4473` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_0len_frame` | `picoquic/frames.c:5153-5160` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_datagram_frame` | `picoquic/frames.c:5204-5227` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_ack_frequency_frame` | `picoquic/frames.c:5526-5537` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_immediate_ack_frame` | `picoquic/frames.c:5642-5648` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_time_stamp_frame` | `picoquic/frames.c:5680-5687` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_time_stamp_frame` | `picoquic/frames.c:5689-5694` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_path_abandon_frame` | `picoquic/frames.c:5752-5759` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_path_available_or_backup_frame` | `picoquic/frames.c:5921-5928` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_max_path_id_frame` | `picoquic/frames.c:6028-6033` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_max_path_id_frame` | `picoquic/frames.c:6035-6040` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_paths_blocked_frame` | `picoquic/frames.c:6144-6149` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_paths_blocked_frame` | `picoquic/frames.c:6151-6156` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_path_cid_blocked_frame` | `picoquic/frames.c:6278-6285` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_path_cid_blocked_frame` | `picoquic/frames.c:6287-6294` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_observed_address_frame` | `picoquic/frames.c:6467-6477` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_bdp_frame` | `picoquic/frames.c:6559-6568` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_bdp_frame` | `picoquic/frames.c:6570-6587` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

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

| `textlog_prefix_initial_cid64` | `picoquic/logger.c:52-57` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_address` | `picoquic/logger.c:59-88` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_packet_address` | `picoquic/logger.c:90-117` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

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

| `picoquic_sack_node_value` | `picoquic/sacks.c:35-40` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_item_value` | `picoquic/sacks.c:42-45` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_item_compare` | `picoquic/sacks.c:47-53` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_node_create` | `picoquic/sacks.c:55-58` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_node_delete` | `picoquic/sacks.c:60-66` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_sack_list_free` | `picoquic/sacks.c:449-457` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquictest_sim_link_delete` | `picoquic/sim_link.c:64-78` | - | `expected_omission` | `rs/fq/src/tests/harness.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_network_thread` | `picoquic/sockloop.c:1851-1875` | - | `expected_omission` | `rs/fq/src/packet_loop.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_free_tickets` | `picoquic/ticket_store.c:500-509` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_tls_api_log_versions` | `picoquic/tls_api.c:204-227` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_hpke_cipher_suite` | `picoquic/tls_api.c:294-306` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_hpke_kem` | `picoquic/tls_api.c:308-319` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_verify_certificate_fn` | `picoquic/tls_api.c:339-346` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_keyex_from_key_file_fn` | `picoquic/tls_api.c:360-365` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_cipher_suite_in_ctx` | `picoquic/tls_api.c:391-425` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

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

