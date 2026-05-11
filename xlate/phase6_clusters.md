# Phase 6 failure clusters

Source: `xlate/phase6_runs/20260511-072921.log`
Pending failures: 397  (classified into 275 clusters, 1 unclassified)

| Size | Location | Message | Representative |
| ---- | -------- | ------- | -------------- |
| 29 | `src/tests/multipath.rs:1039:5` | server connection not accepted during multipath handshake | `tests::multipath::multipath_ab1` |
| 17 | `src/tests/satellite.rs:161:6` | scenario completed: Generic | `tests::satellite::satellite_basic` |
| 11 | `src/tests/datagram.rs:494:63` | datagram_test_one: Generic | `tests::datagram::datagram` |
| 10 | `src/tests/congestion.rs:501:10` | scenario body: Generic | `tests::congestion::bdp_basic` |
| 9 | `src/tests/congestion.rs:213:6` | scenario body: Generic | `tests::congestion::bbr` |
| 8 | `src/tests/sockloop.rs:824:36` | sockloop_test_one: Generic | `tests::sockloop::sockloop_basic` |
| 7 | `src/tests/congestion.rs:345:6` | scenario body: Generic | `tests::congestion::bbr_asym100` |
| 5 | `src/tests/cpu_limited.rs:145:6` | scenario body: Generic | `tests::cpu_limited::limited_batch` |
| 5 | `src/tests/util.rs:912:14` | server connection not yet accepted | `tests::tls_api::nat_rebinding` |
| 4 | `src/tests/app_limited.rs:48:56` | newreno cc algo | `tests::app_limited::app_limited_bbr` |
| 4 | `src/tests/delay_tolerant.rs:128:6` | DTN scenario: Generic | `tests::delay_tolerant::dtn_basic` |
| 4 | `src/tests/ech.rs:82:34` | ech_e2e_test_one: Generic | `tests::ech::ech_e2e` |
| 4 | `src/tests/high_latency.rs:248:70` | client ready: Generic | `tests::high_latency::high_latency_basic` |
| 4 | `src/tests/multipath.rs:1464:10` | body connect: Generic | `tests::multipath::monopath_basic` |
| 4 | `src/tests/util.rs:7088:5` | client was not ready after mTLS client-auth loop | `tests::tls_api::client_auth` |
| 3 | `src/tests/cert_verify.rs:37:5` | assertion `left == right` failed: expected TLS handshake success=true, got lo... | `tests::cert_verify::cert_verify_null` |
| 3 | `src/tests/congestion.rs:243:81` | connect: Generic | `tests::congestion::bbr1_long` |
| 3 | `src/tests/l4s.rs:131:6` | tls_api_one_scenario_body_ex: Generic | `tests::l4s::l4s_bbr_updown` |
| 3 | `src/tests/pacing.rs:342:13` | scenario body verify: Generic; finished=false, completion_time=61560000000, t... | `tests::pacing::pacing_bbr` |
| 2 | `src/tests/ack_frequency.rs:114:10` | client connection ready: Generic | `tests::ack_frequency::ackfrq_basic` |
| 2 | `src/tests/multipath.rs:217:70` | client ready: Generic | `tests::multipath::migration_controlled` |
| 2 | `src/tests/netperf.rs:125:70` | client ready: Generic | `tests::netperf::netperf_basic` |
| 1 | `src/tests/cleartext_aead.rs:212:10` | server connection | `tests::cleartext_aead::clear_text_aead` |
| 1 | `src/tests/cleartext_aead.rs:472:10` | server connection | `tests::cleartext_aead::cleartext_pn_enc` |
| 1 | `src/tests/cnx_creation.rs:265:17` | connection 1 (odd) should still exist | `tests::cnx_creation::create_cnx` |
| 1 | `src/tests/cnxstress.rs:907:40` | loop step: Generic | `tests::cnxstress::cnx_limit` |
| 1 | `src/tests/cnxstress.rs:966:49` | cnx_stress_do_test: Generic | `tests::cnxstress::cnx_stress` |
| 1 | `src/tests/config.rs:505:14` | create_and_configure | `tests::config::config_quic` |
| 1 | `src/tests/config.rs:590:5` | assertion `left == right` failed | `tests::config::config_usage` |
| 1 | `src/tests/congestion.rs:617:6` | scenario body: Generic | `tests::congestion::blackhole` |
| 1 | `src/tests/congestion.rs:675:6` | scenario body: Generic | `tests::congestion::app_limit_cc` |
| 1 | `src/tests/congestion.rs:738:6` | scenario body: Generic | `tests::congestion::cwin_max` |
| 1 | `src/tests/edge_cases.rs:1604:34` | crypto_hs_offset_one: Generic | `tests::edge_cases::crypto_hs_offset` |
| 1 | `src/tests/edge_cases.rs:1614:66` | edge_case_prepare: Generic | `tests::edge_cases::ec00_zero` |
| 1 | `src/tests/edge_cases.rs:1630:10` | edge_case_prepare: Generic | `tests::edge_cases::ec2f_second_flight` |
| 1 | `src/tests/edge_cases.rs:1683:73` | edge_case_prepare: Generic | `tests::edge_cases::eca1_amplification_loss` |
| 1 | `src/tests/edge_cases.rs:1693:68` | edge_case_prepare: Generic | `tests::edge_cases::ecf1_final_loss` |
| 1 | `src/tests/edge_cases.rs:1716:10` | edge_case_prepare: Generic | `tests::edge_cases::ec5c_silly_cid` |
| 1 | `src/tests/edge_cases.rs:1730:72` | edge_case_prepare: Generic | `tests::edge_cases::ec9a_preemptive_amok` |
| 1 | `src/tests/edge_cases.rs:1751:58` | case 1: Generic | `tests::edge_cases::idle_timeout` |
| 1 | `src/tests/edge_cases.rs:1766:52` | case 1: Generic | `tests::edge_cases::idle_server` |
| 1 | `src/tests/edge_cases.rs:1793:42` | start_client: Generic | `tests::edge_cases::initial_pto` |
| 1 | `src/tests/edge_cases.rs:1829:42` | start_client: Generic | `tests::edge_cases::initial_pto_srv` |
| 1 | `src/tests/edge_cases.rs:1877:56` | reset_ack_max: Generic | `tests::edge_cases::reset_ack_max` |
| 1 | `src/tests/edge_cases.rs:1883:52` | reset_ack_reset: Generic | `tests::edge_cases::reset_ack_reset` |
| 1 | `src/tests/edge_cases.rs:1889:58` | reset_extra_max: Generic | `tests::edge_cases::reset_extra_max` |
| 1 | `src/tests/edge_cases.rs:1895:54` | reset_extra_reset: Generic | `tests::edge_cases::reset_extra_reset` |
| 1 | `src/tests/edge_cases.rs:1901:53` | reset_extra_stop: Generic | `tests::edge_cases::reset_extra_stop` |
| 1 | `src/tests/edge_cases.rs:1907:57` | reset_need_max: Generic | `tests::edge_cases::reset_need_max` |
| 1 | `src/tests/edge_cases.rs:1913:53` | reset_need_reset: Generic | `tests::edge_cases::reset_need_reset` |
| 1 | `src/tests/edge_cases.rs:1919:52` | reset_need_stop: Generic | `tests::edge_cases::reset_need_stop` |
| 1 | `src/tests/edge_cases.rs:1950:42` | start_client: Generic | `tests::edge_cases::reset_loop_test` |
| 1 | `src/tests/edge_cases.rs:2076:56` | reset_stream_at_basic: Generic | `tests::edge_cases::reset_stream_at_basic` |
| 1 | `src/tests/edge_cases.rs:2082:56` | reset_stream_at_limit: Generic | `tests::edge_cases::reset_stream_at_limit_test` |
| 1 | `src/tests/edge_cases.rs:2088:55` | reset_stream_at_loss: Generic | `tests::edge_cases::reset_stream_at_loss` |
| 1 | `src/tests/edge_cases.rs:2207:9` | assertion `left == right` failed: error_code=0x0 got="<none>" expected="unknown" | `tests::edge_cases::error_name` |
| 1 | `src/tests/flow_control.rs:369:47` | bbr algorithm | `tests::flow_control::flow_control` |
| 1 | `src/tests/getter.rs:177:42` | start client: Generic | `tests::getter::getter` |
| 1 | `src/tests/l4s.rs:155:54` | newreno cc algo | `tests::l4s::l4s_reno` |
| 1 | `src/tests/l4s.rs:162:53` | prague cc algo | `tests::l4s::l4s_prague` |
| 1 | `src/tests/l4s.rs:178:50` | bbr cc algo | `tests::l4s::l4s_bbr` |
| 1 | `src/tests/mediatest.rs:1426:46` | mediatest_video: Generic | `tests::mediatest::mediatest_video` |
| 1 | `src/tests/mediatest.rs:1439:51` | mediatest_video_audio: Generic | `tests::mediatest::mediatest_video_audio` |
| 1 | `src/tests/mediatest.rs:1453:55` | mediatest_video_data_audio: Generic | `tests::mediatest::mediatest_video_data_audio` |
| 1 | `src/tests/mediatest.rs:1471:51` | mediatest_video2_down: Generic | `tests::mediatest::mediatest_video2_down` |
| 1 | `src/tests/mediatest.rs:1489:51` | mediatest_video2_back: Generic | `tests::mediatest::mediatest_video2_back` |
| 1 | `src/tests/mediatest.rs:1506:52` | mediatest_video2_probe: Generic | `tests::mediatest::mediatest_video2_probe` |
| 1 | `src/tests/mediatest.rs:1530:45` | mediatest_wifi: Generic | `tests::mediatest::mediatest_wifi` |
| 1 | `src/tests/mediatest.rs:1544:46` | mediatest_worst: Generic | `tests::mediatest::mediatest_worst` |
| 1 | `src/tests/mediatest.rs:1560:47` | mediatest_no_coal: Generic | `tests::mediatest::mediatest_no_coal` |
| 1 | `src/tests/mediatest.rs:1582:51` | mediatest_suspension: Generic | `tests::mediatest::mediatest_suspension` |
| 1 | `src/tests/mediatest.rs:1601:52` | mediatest_suspension2: Generic | `tests::mediatest::mediatest_suspension2` |
| 1 | `src/tests/memlog.rs:126:16` | memlog scenario: Generic | `tests::memlog::memlog` |
| 1 | `src/tests/minicrypto.rs:50:6` | tls_api_init_ctx_ex2 | `tests::minicrypto::minicrypto` |
| 1 | `src/tests/multipath.rs:1518:29` | monopath_0rtt: Generic | `tests::multipath::monopath_0rtt` |
| 1 | `src/tests/multipath.rs:1528:29` | monopath_0rtt_2: Generic | `tests::multipath::monopath_0rtt_2` |
| 1 | `src/tests/multipath.rs:1541:33` | monopath_0rtt_loss fails at packet #1 | `tests::multipath::monopath_0rtt_loss` |
| 1 | `src/tests/multipath.rs:1555:33` | monopath_0rtt_loss_2 fails at packet #1 | `tests::multipath::monopath_0rtt_loss_2` |
| 1 | `src/tests/multipath.rs:887:46` | start client: Generic | `tests::multipath::multipath_qlog` |
| 1 | `src/tests/netperf.rs:285:42` | start client: Generic | `tests::netperf::nat_attack` |
| 1 | `src/tests/p2p.rs:83:10` | wait client ready: Generic | `tests::p2p::address_discovery` |
| 1 | `src/tests/pacing.rs:260:44` | cc algorithm | `tests::pacing::pacing_dcubic` |
| 1 | `src/tests/pacing.rs:342:13` | scenario body verify: Generic; finished=false, completion_time=61560000000, t... | `tests::pacing::pacing_fast` |
| 1 | `src/tests/parseheadertest.rs:453:9` | assertion `left == right` failed: epoch mismatch at 5 | `tests::parseheadertest::parseheader` |
| 1 | `src/tests/parseheadertest.rs:542:5` | expected a new connection to be created | `tests::parseheadertest::incoming_initial` |
| 1 | `src/tests/parseheadertest.rs:789:14` | initial enc_dec: Generic | `tests::parseheadertest::packet_enc_dec` |
| 1 | `src/tests/parseheadertest.rs:933:6` | create connection | `tests::parseheadertest::header_length` |
| 1 | `src/tests/sacktest.rs:232:9` | attempt to subtract with overflow | `tests::sacktest::ack_send` |
| 1 | `src/tests/skip_frame.rs:3048:5` | connections created before set_textlog must dispatch through the installed te... | `tests::skip_frame::logger` |
| 1 | `src/tests/skip_frame.rs:3744:13` | assertion `left == right` failed: parse frame <streams_blocked_bidir> | `tests::skip_frame::frames_parse` |
| 1 | `src/tests/skip_frame.rs:3863:30` | frames_format: Generic | `tests::skip_frame::frames_format` |
| 1 | `src/tests/skip_frame.rs:4382:10` | convert overflow binlog to qlog: Generic | `tests::skip_frame::app_message_overflow` |
| 1 | `src/tests/skip_frame.rs:4472:23` | binlog_test: Generic | `tests::skip_frame::binlog` |
| 1 | `src/tests/socket.rs:168:54` | ping-pong on port 12345: Generic | `tests::socket::sockets` |
| 1 | `src/tests/socket.rs:183:35` | ECN on IPv6: Generic | `tests::socket::socket_ecn` |
| 1 | `src/tests/spinbit.rs:146:65` | spinbit: Generic | `tests::spinbit::spinbit` |
| 1 | `src/tests/spinbit.rs:152:69` | spinbit_random: Generic | `tests::spinbit::spinbit_random` |
| 1 | `src/tests/spinbit.rs:158:69` | spinbit_randclient: Generic | `tests::spinbit::spinbit_randclient` |
| 1 | `src/tests/spinbit.rs:164:67` | spinbit_null: Generic | `tests::spinbit::spinbit_null` |
| 1 | `src/tests/stream0_frame.rs:722:60` | tlstest_v1 | `tests::stream0_frame::tlsstreamframe` |
| 1 | `src/tests/stresstest.rs:1769:5` | fuzzer never mutated packet bytes | `tests::stresstest::fuzz` |
| 1 | `src/tests/stresstest.rs:1796:5` | initial fuzzer did not finish the skip-frame mutation pass | `tests::stresstest::fuzz_initial` |
| 1 | `src/tests/ticket_store.rs:17:29` | ticket_seed: Generic | `tests::ticket_store::ticket_seed` |
| 1 | `src/tests/ticket_store.rs:26:29` | ticket_seed_from_bdp_frame: Generic | `tests::ticket_store::ticket_seed_from_bdp_frame` |
| 1 | `src/tests/ticket_store.rs:295:5` | assertion failed: ctx_too_late.qclient.stored_tickets.is_empty() | `tests::ticket_store::ticket_store` |
| 1 | `src/tests/ticket_store.rs:355:35` | Token[2] already used? Generic | `tests::ticket_store::token_reuse_api` |
| 1 | `src/tests/ticket_store.rs:495:5` | expired tokens should be dropped during late reload | `tests::ticket_store::token_store` |
| 1 | `src/tests/tls_api.rs:1054:10` | scenario verify: Generic | `tests::tls_api::af_undef` |
| 1 | `src/tests/tls_api.rs:1089:5` | server connection should exist | `tests::tls_api::bad_certificate` |
| 1 | `src/tests/tls_api.rs:1164:6` | post bad chello scenario: Generic | `tests::tls_api::bad_chello` |
| 1 | `src/tests/tls_api.rs:1219:5` | assertion `left == right` failed | `tests::tls_api::bad_client_certificate` |
| 1 | `src/tests/tls_api.rs:1250:81` | connect: Generic | `tests::tls_api::bad_cnxid` |
| 1 | `src/tests/tls_api.rs:1330:10` | bad_coalesce: Generic | `tests::tls_api::bad_coalesce` |
| 1 | `src/tests/tls_api.rs:1356:10` | chacha20: Generic | `tests::tls_api::chacha20` |
| 1 | `src/tests/tls_api.rs:1371:53` | cid_length(0): Generic | `tests::tls_api::cid_length` |
| 1 | `src/tests/tls_api.rs:1391:70` | client ready: Generic | `tests::tls_api::cid_quiescence` |
| 1 | `src/tests/tls_api.rs:1499:5` | assertion `left == right` failed: certificate verification callback count | `tests::tls_api::client_cert_callback` |
| 1 | `src/tests/tls_api.rs:1513:53` | client_error(stream): Generic | `tests::tls_api::client_error` |
| 1 | `src/tests/tls_api.rs:1561:36` | cnx_ddos: Generic | `tests::tls_api::cnx_ddos` |
| 1 | `src/tests/tls_api.rs:1641:5` | assertion `left != right` failed: local CNX ID did not change to a new value | `tests::tls_api::cnxid_renewal` |
| 1 | `src/tests/tls_api.rs:1652:50` | cnxid_transmit: Generic | `tests::tls_api::cnxid_transmit` |
| 1 | `src/tests/tls_api.rs:1660:49` | cnxid_transmit_disable: Generic | `tests::tls_api::cnxid_transmit_disable` |
| 1 | `src/tests/tls_api.rs:1668:49` | cnxid_transmit_r_before: Generic | `tests::tls_api::cnxid_transmit_r_before` |
| 1 | `src/tests/tls_api.rs:1676:48` | cnxid_transmit_r_disable: Generic | `tests::tls_api::cnxid_transmit_r_disable` |
| 1 | `src/tests/tls_api.rs:1684:49` | cnxid_transmit_r_early: Generic | `tests::tls_api::cnxid_transmit_r_early` |
| 1 | `src/tests/tls_api.rs:1911:13` | connection_drop case 0 failed: target=ClientInitSent, target_is_client=true, ... | `tests::tls_api::connection_drop` |
| 1 | `src/tests/tls_api.rs:1925:39` | ddos_amplification: Generic | `tests::tls_api::ddos_amplification` |
| 1 | `src/tests/tls_api.rs:1933:39` | ddos_amplification_0rtt: Generic | `tests::tls_api::ddos_amplification_0rtt` |
| 1 | `src/tests/tls_api.rs:1941:39` | ddos_amplification_8k: Generic | `tests::tls_api::ddos_amplification_8k` |
| 1 | `src/tests/tls_api.rs:1973:6` | different_params: Generic | `tests::tls_api::different_params` |
| 1 | `src/tests/tls_api.rs:1991:81` | connect: Generic | `tests::tls_api::direct_receive` |
| 1 | `src/tests/tls_api.rs:2030:40` | discard_stream: Generic | `tests::tls_api::discard_stream` |
| 1 | `src/tests/tls_api.rs:2075:6` | document_addresses scenario: Generic | `tests::tls_api::document_addresses` |
| 1 | `src/tests/tls_api.rs:2286:33` | excess_repeat(newreno): Generic | `tests::tls_api::excess_repeat` |
| 1 | `src/tests/tls_api.rs:2577:29` | false_migration initial target_client=true: Generic | `tests::tls_api::false_migration` |
| 1 | `src/tests/tls_api.rs:2753:37` | grease_quic_bit: Generic | `tests::tls_api::grease_quic_bit` |
| 1 | `src/tests/tls_api.rs:2761:36` | grease_quic_bit_one_way: Generic | `tests::tls_api::grease_quic_bit_one_way` |
| 1 | `src/tests/tls_api.rs:2770:40` | heavy_loss: Generic | `tests::tls_api::heavy_loss` |
| 1 | `src/tests/tls_api.rs:2778:40` | heavy_loss_inter: Generic | `tests::tls_api::heavy_loss_inter` |
| 1 | `src/tests/tls_api.rs:2786:40` | heavy_loss_total: Generic | `tests::tls_api::heavy_loss_total` |
| 1 | `src/tests/tls_api.rs:2883:30` | immediate_ack: Generic | `tests::tls_api::immediate_ack` |
| 1 | `src/tests/tls_api.rs:2944:32` | immediate_close: Generic | `tests::tls_api::immediate_close` |
| 1 | `src/tests/tls_api.rs:2957:52` | implicit_ack_ready: Generic | `tests::tls_api::implicit_ack` |
| 1 | `src/tests/tls_api.rs:3020:5` | server connection deleted, cannot verify error code | `tests::tls_api::initial_close` |
| 1 | `src/tests/tls_api.rs:3102:5` | server did not queue its first packet | `tests::tls_api::initial_race` |
| 1 | `src/tests/tls_api.rs:3147:5` | server connection not accepted before close; client_state=ClientInitSent, c_t... | `tests::tls_api::initial_server_close` |
| 1 | `src/tests/tls_api.rs:3315:29` | keep_alive_on: Generic | `tests::tls_api::keep_alive` |
| 1 | `src/tests/tls_api.rs:3325:30` | key_rotation: Generic | `tests::tls_api::key_rotation` |
| 1 | `src/tests/tls_api.rs:3335:38` | key_rotation_client: Generic | `tests::tls_api::key_rotation_client` |
| 1 | `src/tests/tls_api.rs:3343:39` | key_rotation_server: Generic | `tests::tls_api::key_rotation_server` |
| 1 | `src/tests/tls_api.rs:3540:6` | keylog q_and_r scenario: Generic | `tests::tls_api::keylog_test` |
| 1 | `src/tests/tls_api.rs:3587:6` | large_client_hello q_and_r scenario: Generic | `tests::tls_api::large_client_hello` |
| 1 | `src/tests/tls_api.rs:3633:6` | long_rtt: Generic | `tests::tls_api::long_rtt` |
| 1 | `src/tests/tls_api.rs:3745:13` | loss_bit wait-ready client=0 server=0: Generic | `tests::tls_api::loss_bit` |
| 1 | `src/tests/tls_api.rs:4019:33` | many_losses random mask 0=0x9024423006603000: Generic | `tests::tls_api::many_losses` |
| 1 | `src/tests/tls_api.rs:4179:32` | many_short_loss: Generic | `tests::tls_api::many_short_loss` |
| 1 | `src/tests/tls_api.rs:4187:62` | migration: Generic | `tests::tls_api::migration` |
| 1 | `src/tests/tls_api.rs:4211:6` | migration_disabled q_and_r scenario: Generic | `tests::tls_api::migration_disabled` |
| 1 | `src/tests/tls_api.rs:4246:31` | migration_fail: Generic | `tests::tls_api::migration_fail` |
| 1 | `src/tests/tls_api.rs:4254:64` | migration_long: Generic | `tests::tls_api::migration_long` |
| 1 | `src/tests/tls_api.rs:4262:65` | migration_with_loss: Generic | `tests::tls_api::migration_with_loss` |
| 1 | `src/tests/tls_api.rs:4276:49` | migration_zero: Generic | `tests::tls_api::migration_zero` |
| 1 | `src/tests/tls_api.rs:4291:76` | mtu_blocked: Generic | `tests::tls_api::mtu_blocked` |
| 1 | `src/tests/tls_api.rs:4306:76` | mtu_delayed: Generic | `tests::tls_api::mtu_delayed` |
| 1 | `src/tests/tls_api.rs:4320:74` | mtu_discovery: Generic | `tests::tls_api::mtu_discovery` |
| 1 | `src/tests/tls_api.rs:4328:45` | mtu_drop_bbr: Generic | `tests::tls_api::mtu_drop_bbr` |
| 1 | `src/tests/tls_api.rs:4336:47` | mtu_drop_cubic: Generic | `tests::tls_api::mtu_drop_cubic` |
| 1 | `src/tests/tls_api.rs:4344:47` | mtu_drop_dcubic: Generic | `tests::tls_api::mtu_drop_dcubic` |
| 1 | `src/tests/tls_api.rs:4352:46` | mtu_drop_fast: Generic | `tests::tls_api::mtu_drop_fast` |
| 1 | `src/tests/tls_api.rs:4360:49` | mtu_drop_newreno: Generic | `tests::tls_api::mtu_drop_newreno` |
| 1 | `src/tests/tls_api.rs:4374:77` | mtu_max: Generic | `tests::tls_api::mtu_max` |
| 1 | `src/tests/tls_api.rs:4389:77` | mtu_required: Generic | `tests::tls_api::mtu_required` |
| 1 | `src/tests/tls_api.rs:4508:33` | multi_segment(newreno): Generic | `tests::tls_api::multi_segment` |
| 1 | `src/tests/tls_api.rs:4531:33` | multiple_versions ver=0x6b3343cf: Generic | `tests::tls_api::multiple_versions` |
| 1 | `src/tests/tls_api.rs:4655:33` | nat_handshake(0): Generic | `tests::tls_api::nat_handshake` |
| 1 | `src/tests/tls_api.rs:4806:31` | nat_rebinding_fast: Generic | `tests::tls_api::nat_rebinding_fast` |
| 1 | `src/tests/tls_api.rs:4977:33` | nat_rebinding_stress: Generic | `tests::tls_api::nat_rebinding_stress` |
| 1 | `src/tests/tls_api.rs:5155:28` | new_rotated_key: Protocol(1526726660) | `tests::tls_api::new_rotated_key` |
| 1 | `src/tests/tls_api.rs:5212:29` | no_ack_frequency(1): Generic | `tests::tls_api::no_ack_frequency` |
| 1 | `src/tests/tls_api.rs:5390:29` | not_before_cnxid: Protocol(1527775233) | `tests::tls_api::not_before_cnxid` |
| 1 | `src/tests/tls_api.rs:5408:35` | optimistic_ack: Generic | `tests::tls_api::optimistic_ack` |
| 1 | `src/tests/tls_api.rs:5417:36` | optimistic_hole: Generic | `tests::tls_api::optimistic_hole` |
| 1 | `src/tests/tls_api.rs:5426:26` | pacing_update: Generic | `tests::tls_api::pacing_update` |
| 1 | `src/tests/tls_api.rs:5686:25` | packet_trace: Generic | `tests::tls_api::packet_trace` |
| 1 | `src/tests/tls_api.rs:5855:29` | padding_null: Protocol(1526878719) | `tests::tls_api::padding_null` |
| 1 | `src/tests/tls_api.rs:5863:31` | padding_test: Protocol(1526903295) | `tests::tls_api::padding_test` |
| 1 | `src/tests/tls_api.rs:5871:30` | padding_zero_min: Protocol(1526903295) | `tests::tls_api::padding_zero_min` |
| 1 | `src/tests/tls_api.rs:5894:10` | pn_enc_1rtt application aead: Generic | `tests::tls_api::pn_enc_1rtt` |
| 1 | `src/tests/tls_api.rs:6077:31` | pn_random initial-only: Generic | `tests::tls_api::pn_random` |
| 1 | `src/tests/tls_api.rs:6126:9` | unexpected one-rtt: server did not respond to unblocked source 1.1.1.1:0 | `tests::tls_api::port_blocked` |
| 1 | `src/tests/tls_api.rs:6331:46` | preferred_address: Generic | `tests::tls_api::preferred_address` |
| 1 | `src/tests/tls_api.rs:6340:45` | preferred_address_dis_mig: Generic | `tests::tls_api::preferred_address_dis_mig` |
| 1 | `src/tests/tls_api.rs:6348:45` | preferred_address_zero: Generic | `tests::tls_api::preferred_address_zero` |
| 1 | `src/tests/tls_api.rs:6385:5` | Only 1 CID created on client. | `tests::tls_api::probe_api` |
| 1 | `src/tests/tls_api.rs:6521:26` | qlog_fns: Generic | `tests::tls_api::qlog_fns` |
| 1 | `src/tests/tls_api.rs:6529:29` | qlog_fns_ecn: Generic | `tests::tls_api::qlog_fns_ecn` |
| 1 | `src/tests/tls_api.rs:6538:35` | qlog_trace: Generic | `tests::tls_api::qlog_trace` |
| 1 | `src/tests/tls_api.rs:6546:38` | qlog_trace_ecn: Generic | `tests::tls_api::qlog_trace_ecn` |
| 1 | `src/tests/tls_api.rs:6554:34` | qlog_trace_parallel: Generic | `tests::tls_api::qlog_trace_parallel` |
| 1 | `src/tests/tls_api.rs:6598:6` | quality_update scenario: Generic | `tests::tls_api::quality_update` |
| 1 | `src/tests/tls_api.rs:6683:10` | quant_params: Generic | `tests::tls_api::quant_params` |
| 1 | `src/tests/tls_api.rs:6695:58` | random_padding_128: Generic | `tests::tls_api::random_padding` |
| 1 | `src/tests/tls_api.rs:6826:31` | ready_to_send: Generic | `tests::tls_api::ready_to_send` |
| 1 | `src/tests/tls_api.rs:6834:31` | ready_to_skip: Generic | `tests::tls_api::ready_to_skip` |
| 1 | `src/tests/tls_api.rs:6842:31` | ready_to_zero: Generic | `tests::tls_api::ready_to_zero` |
| 1 | `src/tests/tls_api.rs:6850:31` | ready_to_zfin: Generic | `tests::tls_api::ready_to_zfin` |
| 1 | `src/tests/tls_api.rs:6858:42` | red_bbr: Generic | `tests::tls_api::red_bbr` |
| 1 | `src/tests/tls_api.rs:6866:44` | red_cubic: Generic | `tests::tls_api::red_cubic` |
| 1 | `src/tests/tls_api.rs:6874:45` | red_dcubic: Generic | `tests::tls_api::red_dcubic` |
| 1 | `src/tests/tls_api.rs:6882:43` | red_fast: Generic | `tests::tls_api::red_fast` |
| 1 | `src/tests/tls_api.rs:6890:46` | red_newreno: Generic | `tests::tls_api::red_newreno` |
| 1 | `src/tests/tls_api.rs:6930:5` | Only 1 cids created on client. | `tests::tls_api::retire_cnxid` |
| 1 | `src/tests/tls_api.rs:7069:33` | retry_large: Generic | `tests::tls_api::retry_large` |
| 1 | `src/tests/tls_api.rs:7301:5` | simulated time 120760039 | `tests::tls_api::server_busy` |
| 1 | `src/tests/tls_api.rs:7385:9` | session_resume pass 0: no ticket received | `tests::tls_api::session_resume` |
| 1 | `src/tests/tls_api.rs:7454:5` | client did not reach ready state: client=Some(ClientInitSent) server=None | `tests::tls_api::set_certificate_and_key` |
| 1 | `src/tests/tls_api.rs:7487:33` | short_initial_cid(8): Generic | `tests::tls_api::short_initial_cid` |
| 1 | `src/tests/tls_api.rs:7603:5` | first stateless reset was not sent at T=0 | `tests::tls_api::stateless_blowback` |
| 1 | `src/tests/tls_api.rs:7705:70` | client ready: Generic | `tests::tls_api::stateless_reset` |
| 1 | `src/tests/tls_api.rs:7809:5` | client did not remain ready after bogus stateless reset | `tests::tls_api::stateless_reset_bad` |
| 1 | `src/tests/tls_api.rs:7827:5` | server connection was not accepted after connection loop | `tests::tls_api::stateless_reset_client` |
| 1 | `src/tests/tls_api.rs:7957:41` | stop_sending: Generic | `tests::tls_api::stop_sending` |
| 1 | `src/tests/tls_api.rs:7965:40` | stop_sending_loss: Generic | `tests::tls_api::stop_sending_loss` |
| 1 | `src/tests/tls_api.rs:7997:6` | stream_id_max: Generic | `tests::tls_api::stream_id_max` |
| 1 | `src/tests/tls_api.rs:8027:14` | server connection not accepted | `tests::tls_api::tls_api` |
| 1 | `src/tests/tls_api.rs:8069:9` | client did not derive handshake epoch keys before ACK injection | `tests::tls_api::tls_api_inject_hs_ack` |
| 1 | `src/tests/tls_api.rs:8107:10` | oneway_stream: Generic | `tests::tls_api::tls_api_oneway_stream` |
| 1 | `src/tests/tls_api.rs:8127:6` | q2_and_r2_stream: Generic | `tests::tls_api::tls_api_q2_and_r2_stream` |
| 1 | `src/tests/tls_api.rs:8138:10` | q_and_r_stream: Generic | `tests::tls_api::tls_api_q_and_r_stream` |
| 1 | `src/tests/tls_api.rs:8168:14` | server connection not accepted | `tests::tls_api::tls_api_sni` |
| 1 | `src/tests/tls_api.rs:8186:52` | very_long_congestion ready: Generic | `tests::tls_api::tls_api_very_long_congestion` |
| 1 | `src/tests/tls_api.rs:8226:6` | very_long_max: Generic | `tests::tls_api::tls_api_very_long_max` |
| 1 | `src/tests/tls_api.rs:8246:6` | very_long_stream: Generic | `tests::tls_api::tls_api_very_long_stream` |
| 1 | `src/tests/tls_api.rs:8261:52` | very_long_with_err ready: Generic | `tests::tls_api::tls_api_very_long_with_err` |
| 1 | `src/tests/tls_api.rs:8311:5` | assertion `left == right` failed: wrong_alpn: client did not disconnect | `tests::tls_api::tls_api_wrong_alpn` |
| 1 | `src/tests/tls_api.rs:8355:10` | server connection not accepted | `tests::tls_api::tls_exporter` |
| 1 | `src/tests/tls_api.rs:8397:5` | first client connection is not ready | `tests::tls_api::two_connections` |
| 1 | `src/tests/tls_api.rs:8493:6` | unidir: Generic | `tests::tls_api::unidir` |
| 1 | `src/tests/tls_api.rs:8545:5` | VN response too short: got 19, need at least 393 | `tests::tls_api::version_invariant` |
| 1 | `src/tests/tls_api.rs:8674:5` | assertion `left == right` failed: VN spoof mode 0 has no effect | `tests::tls_api::version_negotiation_spoof` |
| 1 | `src/tests/tls_api.rs:8875:9` | assertion `left == right` failed: iteration 0: QUIC time does not follow simu... | `tests::tls_api::virtual_time` |
| 1 | `src/tests/tls_api.rs:8948:64` | vn_compat V1 -> V2: Generic | `tests::tls_api::vn_compat` |
| 1 | `src/tests/tls_api.rs:9006:48` | zero_rtt: Generic | `tests::tls_api::zero_rtt` |
| 1 | `src/tests/tls_api.rs:9019:6` | zero_rtt_bad_param: Generic | `tests::tls_api::zero_rtt_bad_param` |
| 1 | `src/tests/tls_api.rs:9047:6` | zero_rtt_delay: Generic | `tests::tls_api::zero_rtt_delay` |
| 1 | `src/tests/tls_api.rs:9060:6` | zero_rtt_ech: Generic | `tests::tls_api::zero_rtt_ech` |
| 1 | `src/tests/tls_api.rs:9072:6` | zero_rtt_long: Generic | `tests::tls_api::zero_rtt_long` |
| 1 | `src/tests/tls_api.rs:9086:29` | zero_rtt_loss i=1: Generic | `tests::tls_api::zero_rtt_loss` |
| 1 | `src/tests/tls_api.rs:9109:29` | zero_rtt_many_losses i=0, mask=9024423006603000: Generic | `tests::tls_api::zero_rtt_many_losses` |
| 1 | `src/tests/tls_api.rs:9122:6` | zero_rtt_no_coal: Generic | `tests::tls_api::zero_rtt_no_coal` |
| 1 | `src/tests/tls_api.rs:9135:6` | zero_rtt_retry: Generic | `tests::tls_api::zero_rtt_retry` |
| 1 | `src/tests/tls_api.rs:9147:6` | zero_rtt_spurious: Generic | `tests::tls_api::zero_rtt_spurious` |
| 1 | `src/tests/transport_param.rs:331:10` | transport parameter connection | `tests::transport_param::transport_param` |
| 1 | `src/tests/transport_param.rs:937:43` | assertion `left == right` failed: default_tp[22] grease_quic_bit: max_idle_ti... | `tests::transport_param::transport_param_default` |
| 1 | `src/tests/util.rs:6440:5` | Retry test completes in 120010071 microsec, more than 230000 | `tests::tls_api::retry` |
| 1 | `src/tests/util.rs:6534:9` | assertion `left == right` failed: Second retry did not use the stored token, ... | `tests::tls_api::retry_token` |
| 1 | `src/tests/warptest.rs:21:28` | warptest_param: Generic | `tests::warptest::warptest_param` |
| 1 | `src/tests/warptest.rs:35:28` | warptest_video: Generic | `tests::warptest::warptest_video` |
| 1 | `src/tests/warptest.rs:50:28` | warptest_video_audio: Generic | `tests::warptest::warptest_video_audio` |
| 1 | `src/tests/warptest.rs:66:28` | warptest_video_data_audio: Generic | `tests::warptest::warptest_video_data_audio` |
| 1 | `src/tests/warptest.rs:82:28` | warptest_worst: Generic | `tests::warptest::warptest_worst` |
| 1 | `src/tests/wifitest.rs:106:41` | wifi_bbr: Generic | `tests::wifitest::wifi_bbr` |
| 1 | `src/tests/wifitest.rs:115:41` | wifi_bbr1: Generic | `tests::wifitest::wifi_bbr1` |
| 1 | `src/tests/wifitest.rs:130:47` | wifi_bbr1_hard: Generic | `tests::wifitest::wifi_bbr1_hard` |
| 1 | `src/tests/wifitest.rs:145:47` | wifi_bbr1_long: Generic | `tests::wifitest::wifi_bbr1_long` |
| 1 | `src/tests/wifitest.rs:160:46` | wifi_bbr_hard: Generic | `tests::wifitest::wifi_bbr_hard` |
| 1 | `src/tests/wifitest.rs:175:46` | wifi_bbr_long: Generic | `tests::wifitest::wifi_bbr_long` |
| 1 | `src/tests/wifitest.rs:190:46` | wifi_bbr_many: Generic | `tests::wifitest::wifi_bbr_many` |
| 1 | `src/tests/wifitest.rs:207:48` | wifi_bbr_shadow: Generic | `tests::wifitest::wifi_bbr_shadow` |
| 1 | `src/tests/wifitest.rs:214:43` | wifi_cubic: Generic | `tests::wifitest::wifi_cubic` |
| 1 | `src/tests/wifitest.rs:229:48` | wifi_cubic_hard: Generic | `tests::wifitest::wifi_cubic_hard` |
| 1 | `src/tests/wifitest.rs:244:48` | wifi_cubic_long: Generic | `tests::wifitest::wifi_cubic_long` |
| 1 | `src/tests/wifitest.rs:251:42` | wifi_reno: Generic | `tests::wifitest::wifi_reno` |
| 1 | `src/tests/wifitest.rs:266:47` | wifi_reno_hard: Generic | `tests::wifitest::wifi_reno_hard` |
| 1 | `src/tests/wifitest.rs:281:47` | wifi_reno_long: Generic | `tests::wifitest::wifi_reno_long` |

## Unclassified (no panic line matched)
* `tests::tls_api::integrity_limit`
