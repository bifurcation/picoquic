# Phase 5A C/Rust test correspondence audit

Review each C/Rust test pair and decide whether the Rust
`#[test]` checks the same behavior as the C test.  This is
a read-only pass: do not edit files.

You may inspect directly relevant context when needed:
Rust test helpers, fixtures, translated implementation under
`rs/fq/`, C helper functions in `picoquictest/`, constants,
and nearby tests.  Do not require byte-for-byte structure;
idiomatic Rust is fine when it preserves the C test intent,
inputs, expected results, and important edge cases.

Classify each entry as:

* `ok` when the Rust test is an acceptable translation.
* `needs_fix` when the Rust test is missing checks, checks
  materially different behavior, weakens assertions, skips
  cases the C test covers, or has placeholder-like logic.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"reviews":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short conclusion","fix_summary":"what 5B should change, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/parseheadertest.c:parseheadertest`
* C test-table name: `parseheader`
* C entry function: `parseheadertest`
* Rust test: `parseheader`
* C source: `picoquictest/parseheadertest.c:456-587`
* Rust source: `rs/fq/src/tests/parseheadertest.rs:197-496`

### C test body
```c
{
    int ret = 0;
    picoquic_packet_header ph;
    picoquic_quic_t* quic = NULL;
    picoquic_cnx_t* cnx_10 = NULL;
    struct sockaddr_in addr_10;
    picoquic_cnx_t* pcnx;
    uint8_t packet[PICOQUIC_MAX_PACKET_SIZE];

    /* Initialize the quic context and the connection contexts */
    memset(&addr_10, 0, sizeof(struct sockaddr_in));
    addr_10.sin_family = AF_INET;
#ifdef _WINDOWS
    addr_10.sin_addr.S_un.S_addr = 0x0A000002;
#else
    addr_10.sin_addr.s_addr = 0x0A000002;
#endif
    // addr_07.sin_port = 4433;
    addr_10.sin_port = 4434;

    quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0);
    if (quic == NULL) {
        ret = -1;
    } else {
        cnx_10 = picoquic_create_cnx(quic, test_cnxid_ini, test_cnxid_rem, (struct sockaddr*)&addr_10,
            0, PICOQUIC_INTERNAL_TEST_VERSION_1, NULL, NULL, 1);

        if (cnx_10 == NULL || cnx_10->first_local_cnxid_list == NULL) {
            ret = -1;
        }
        else {
            /* Remove old local CID from table and avoid leak. */
            picoquic_local_cnxid_list_t* local_cnxid_list = cnx_10->first_local_cnxid_list;

            picoquic_delete_local_cnxid(cnx_10, cnx_10->path[0]->first_tuple->p_local_cnxid);
            if (local_cnxid_list->nb_local_cnxid != 0) {
                DBG_PRINTF("Expected 0 cnxid left, got %d", local_cnxid_list->nb_local_cnxid);
            }
            else {
                /* Update the local cnx_id so it be predictable in tests */
                picoquic_local_cnxid_t* local_cnxid0 = picoquic_create_local_cnxid(cnx_10, 0, &test_cnxid_local, 0);
                if (local_cnxid0 == NULL) {
                    DBG_PRINTF("%s", "Cannot create the new CNX_ID");
                    ret = -1;
                }
                else {
                    cnx_10->path[0]->first_tuple->p_local_cnxid = local_cnxid0;
                }
            }
        }
    }

    for (size_t i = 0; ret == 0 && i < nb_test_entries; i++) {
        pcnx = (i < 3) ? NULL : cnx_10;
        quic->local_cnxid_length = test_entries[i].local_cid_length;
        memset(packet, 0xcc, sizeof(packet));
        memcpy(packet, test_entries[i].packet, (uint32_t)test_entries[i].length);

        if (picoquic_parse_packet_header(quic, packet, sizeof(packet),
                (struct sockaddr*)&addr_10, &ph, &pcnx, 1)
            != 0) {
            ret = -1;
        } else if (picoquic_compare_connection_id(&ph.dest_cnx_id, &test_entries[i].ph->dest_cnx_id) != 0) {
            ret = -1;
        } else if (picoquic_compare_connection_id(&ph.srce_cnx_id, &test_entries[i].ph->srce_cnx_id) != 0) {
            ret = -1;
        } else if (ph.vn != test_entries[i].ph->vn) {
            ret = -1;
        } else if (ph.offset != test_entries[i].ph->offset) {
            ret = -1;
        } else if (ph.pn_offset != test_entries[i].ph->pn_offset) {
            ret = -1;
        } else if (ph.payload_length != test_entries[i].ph->payload_length) {
            ret = -1;
        } else if (ph.ptype != test_entries[i].ph->ptype) {
            ret = -1;
        } else if (ph.spin != test_entries[i].ph->spin) {
            ret = -1;
        } else if (ph.epoch != test_entries[i].ph->epoch) {
            ret = -1;
        } else if (ph.pc != test_entries[i].ph->pc) {
            ret = -1;
        } else if (ph.key_phase != test_entries[i].ph->key_phase) {
            ret = -1;
        }
    }

    if (ret == 0) {
        quic->local_cnxid_length = 8;
    }

    for (size_t i = 0; ret == 0 && i < nb_test_entries; i++) {
        size_t header_length;
        size_t pn_offset;
        size_t pn_length;

        if (test_entries[i].decode_test_only) {
            continue;
        }

        pcnx = (i < 3) ? NULL : cnx_10;
        memset(packet, 0xcc, sizeof(packet));
        /* Prepare the header inside the packet */
        if (i < 2) {
            cnx_10->path[0]->first_tuple->p_remote_cnxid->cnx_id = picoquic_null_connection_id;
        }
        else {
            cnx_10->path[0]->first_tuple->p_remote_cnxid->cnx_id = test_cnxid_r10;
        }
        header_length = picoquic_create_packet_header(cnx_10, test_entries[i].ph->ptype,
            test_entries[i].ph->pn, cnx_10->path[0], cnx_10->path[0]->first_tuple, 0, packet, &pn_offset, &pn_length);
        picoquic_update_payload_length(packet, pn_offset, pn_offset, pn_offset +
            test_entries[i].ph->payload_length);
        
        if ( pn_offset != test_entries[i].ph->pn_offset) {
           ret = -1;
        }
        
        if (memcmp(packet, test_entries[i].packet, header_length) != 0)
        {
            ret = -1;
        }
    }

    if (quic != NULL) {
        picoquic_free(quic);
    }

    return ret;
}
```

### Rust test body
```rust
fn parseheader() {
    let current_time = Instant::from_ticks(0);
    let addr: SocketAddr = "10.0.0.2:4434".parse().unwrap();

    let mut quic = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("create quic");

    // Create connection with predictable CIDs.
    // C: picoquic_create_cnx(..., is_client=1) — client mode.
    quic.create_connection(
        ini_id(),
        rem_id(),
        Some(&addr),
        current_time,
        Version::InternalTest1 as u32,
        None,
        None,
        true,
    )
    .expect("create connection");

    // Replace default local CID with a predictable one
    {
        let cnx = quic.connection_ref_by_id(ini_id()).expect("lookup cnx");
        let orig = cnx
            .create_local_connection_id(0, None, current_time)
            .expect("create orig cid");
        cnx.delete_local_connection_id(orig);
        let new_tok = cnx
            .create_local_connection_id(0, Some(&local_id()), current_time)
            .expect("create local cid");
        cnx.set_path_tuple_local_cid(0, 0, new_tok);
    }

    struct Entry {
        packet: &'static [u8],
        dest: fn() -> ConnectionId,
        src: fn() -> ConnectionId,
        version: u32,
        offset: usize,
        pn_offset: usize,
        payload_length: usize,
        ptype: PacketType,
        spin: bool,
        epoch: Epoch,
        pc: PacketContext,
        key_phase: bool,
        decode_only: bool,
        local_cid_len: u8,
    }
    let null = ConnectionId::default;
    let entries: &[Entry] = &[
        Entry {
            packet: PINITIAL10,
            dest: ini_id,
            src: rem_id,
            version: 0x50435130,
            offset: 22,
            pn_offset: 22,
            payload_length: 0x400,
            ptype: PacketType::Initial,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PINITIAL10_L,
            dest: ini_id,
            src: local_id,
            version: 0x50435130,
            offset: 26,
            pn_offset: 26,
            payload_length: 0x400,
            ptype: PacketType::Initial,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: false,
            local_cid_len: 8,
        },
        Entry {
            packet: PVNEGO10,
            dest: r10_id,
            src: rem_id,
            version: 0,
            offset: 19,
            pn_offset: 0,
            payload_length: crate::MAX_PACKET_SIZE - 19,
            ptype: PacketType::VersionNegotiation,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PVNEGOBIS10,
            dest: r10_id,
            src: rem_id,
            version: 0,
            offset: 19,
            pn_offset: 0,
            payload_length: crate::MAX_PACKET_SIZE - 19,
            ptype: PacketType::VersionNegotiation,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PHANDSHAKE,
            dest: local_id,
            src: rem_id,
            version: 0x50435130,
            offset: 21,
            pn_offset: 21,
            payload_length: 0x400,
            ptype: PacketType::Handshake,
            spin: false,
            epoch: Epoch::Handshake,
            pc: PacketContext::Handshake,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PACKET_SHORT_PHI0_C_32,
            dest: r10_id,
            src: null,
            version: 0,
            offset: 9,
            pn_offset: 9,
            payload_length: crate::MAX_PACKET_SIZE - 9,
            ptype: PacketType::OneRttProtected,
            spin: false,
            epoch: Epoch::OneRtt,
            pc: PacketContext::Application,
            key_phase: false,
            decode_only: false,
            local_cid_len: 8,
        },
        Entry {
            packet: PACKET_SHORT_PHI0_C_32_SPIN,
            dest: r10_id,
            src: null,
            version: 0,
            offset: 9,
            pn_offset: 9,
            payload_length: crate::MAX_PACKET_SIZE - 9,
            ptype: PacketType::OneRttProtected,
            spin: true,
            epoch: Epoch::OneRtt,
            pc: PacketContext::Application,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PACKET_SHORT_PHI1_NOC_32,
            dest: null,
            src: null,
            version: 0,
            offset: 1,
            pn_offset: 1,
            payload_length: crate::MAX_PACKET_SIZE - 1,
            ptype: PacketType::OneRttProtected,
            spin: false,
            epoch: Epoch::OneRtt,
            pc: PacketContext::Application,
            key_phase: true,
            decode_only: true,
            local_cid_len: 0,
        },
        Entry {
            packet: PACKET_INTEL_BUG,
            dest: || {
                ConnectionId::clone_from_slice(&[0xbb, 0xba, 0xda, 0x0e, 0xf9, 0x26, 0x00, 0xc8])
                    .unwrap()
            },
            src: null,
            version: 1,
            offset: 18,
            pn_offset: 18,
            payload_length: 1182,
            ptype: PacketType::Initial,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
    ];

    let mut packet = [0u8; crate::MAX_PACKET_SIZE];

    // First loop: decode test
    for (i, e) in entries.iter().enumerate() {
        quic.local_connection_id_length = e.local_cid_len;
        packet.fill(0xcc);
        packet[..e.packet.len()].copy_from_slice(e.packet);
        let mut ph = PacketHeader::default();
        quic.parse_packet_header(&packet, Some(&addr), &mut ph, true)
            .unwrap_or_else(|_| panic!("parse_packet_header failed at entry {i}"));
        assert_eq!(
            ph.dest_connection_id,
            (e.dest)(),
            "dest CID mismatch at {i}"
        );
        assert_eq!(ph.src_connection_id, (e.src)(), "src CID mismatch at {i}");
        assert_eq!(ph.version, e.version, "version mismatch at {i}");
        assert_eq!(ph.offset, e.offset, "offset mismatch at {i}");
        assert_eq!(
            ph.packet_number_offset, e.pn_offset,
            "pn_offset mismatch at {i}"
        );
        assert_eq!(
            ph.payload_length, e.payload_length,
            "payload_length mismatch at {i}"
        );
        assert_eq!(ph.packet_type, e.ptype, "ptype mismatch at {i}");
        assert_eq!(ph.spin, e.spin, "spin mismatch at {i}");
        assert_eq!(ph.epoch, e.epoch, "epoch mismatch at {i}");
        assert_eq!(ph.packet_context, e.pc, "pc mismatch at {i}");
        assert_eq!(ph.key_phase, e.key_phase, "key_phase mismatch at {i}");
    }

    quic.local_connection_id_length = 8;

    // Second loop: encode + verify (decode_test_only == false entries only)
    for (i, e) in entries.iter().enumerate() {
        if e.decode_only {
            continue;
        }

        {
            let cnx = quic
                .connection_ref_by_id(ini_id())
                .expect("lookup cnx 2nd loop");
            if i < 2 {
                cnx.set_path_tuple_remote_cid(0, 0, ConnectionId::default());
            } else {
                cnx.set_path_tuple_remote_cid(0, 0, r10_id());
            }
        }

        packet.fill(0xcc);
        let mut pn_offset = 0usize;
        let mut pn_length = 0usize;

        let header_length = {
            let cnx = quic
                .connection_ref_by_id(ini_id())
                .expect("lookup cnx create_hdr");
            cnx.create_packet_header_at(
                e.ptype,
                0xDEADBEEF,
                0,
                0,
                0,
                &mut packet,
                &mut pn_offset,
                &mut pn_length,
            )
        };

        update_payload_length(
            &mut packet,
            pn_offset,
            pn_offset,
            pn_offset + e.payload_length,
        );

        assert_eq!(pn_offset, e.pn_offset, "create: pn_offset mismatch at {i}");
        assert_eq!(
            &packet[..header_length],
            &e.packet[..header_length],
            "header bytes mismatch at {i}"
        );
    }
}
```

## `picoquictest/sacktest.c:sendacktest`
* C test-table name: `ack_send`
* C entry function: `sendacktest`
* Rust test: `ack_send`
* C source: `picoquictest/sacktest.c:365-419`
* Rust source: `rs/fq/src/tests/sacktest.rs:632-695`

### C test body
```c
{
    int ret = 0;
    picoquic_quic_t* quic;
    picoquic_cnx_t * cnx;
    uint64_t current_time;
    uint64_t received_mask = 0;
    uint64_t previous_mask = 0;
    uint8_t bytes[256];
    picoquic_packet_context_enum pc = 0;

    if (picoquic_test_set_minimal_cnx(&quic, &cnx) != 0) {
        return -1;
    }
    cnx->sending_ecn_ack = 0; /* don't write an ack_ecn frame */
    
    if (check_ack_ranges(&cnx->ack_ctx[pc].sack_list) != 0) {
        ret = -1;
    }

    for (size_t i = 0; ret == 0 && i < nb_test_pn64; i++) {
        current_time = ((uint64_t)i) * 100;

        if (picoquic_record_pn_received(cnx, pc, cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[i], current_time) != 0) {
            ret = -1;
        }

        if (check_ack_ranges(&cnx->ack_ctx[pc].sack_list) != 0) {
            ret = -1;
        }

        if (ret == 0) {
            int more_data = 0;
            uint8_t* bytes_next = picoquic_format_ack_frame(cnx, bytes, bytes + sizeof(bytes), &more_data, 0, pc, 0);

            received_mask |= 1ull << (test_pn64[i] & 63);

            if (check_ack_ranges(&cnx->ack_ctx[pc].sack_list) != 0) {
                ret = -1;
            }

            if (ret == 0) {
                ret = basic_ack_parse(bytes, bytes_next - bytes, &expected_ack[i], &previous_mask, received_mask);
            }

            if (ret != 0) {
                ret = -1; /* useless code, but helps with checkpointing */
            }
        }
    }

    picoquic_test_delete_minimal_cnx(&quic, &cnx);

    return ret;
}
```

### Rust test body
```rust
fn ack_send() {
    let pc = PacketContext::Application;
    let t0 = Instant::from_ticks(0);
    let mut quic = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("quic");
    let cnx = quic
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            None,
            t0,
            0,
            Some(util::TEST_SNI),
            Some("minimal"),
            true,
        )
        .expect("cnx");

    cnx.sending_ecn_ack = false;
    let l_cid = cnx.create_local_connection_id(0, None, t0).expect("l_cid");

    util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

    let mut received_mask = 0u64;
    let mut previous_mask = 0u64;
    let mut bytes = [0u8; 256];

    for (i, &pn) in TEST_PNS.iter().enumerate() {
        let current_time = Instant::from_ticks(i as u64 * 100);

        assert_eq!(
            cnx.record_pn_received(pc, Some(l_cid), pn, current_time),
            0,
            "record pn {pn} at step {i}"
        );
        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        let mut more_data = 0i32;
        let written = util::format_ack_frame_written(cnx, &mut bytes, &mut more_data, t0, pc, 0)
            .expect("format_ack_frame at step {i}");

        received_mask |= 1u64 << (pn & 63);
        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        basic_ack_parse(
            &bytes[..written],
            &EXPECTED_ACKS[i],
            &mut previous_mask,
            received_mask,
        );
    }
}
```

## `picoquictest/satellite_test.c:satellite_prague_seeded_test`
* C test-table name: `satellite_prague_seeded`
* C entry function: `satellite_prague_seeded_test`
* Rust test: `satellite_prague_seeded`
* C source: `picoquictest/satellite_test.c:307-311`
* Rust source: `rs/fq/src/tests/satellite.rs:475-490`

### C test body
```c
{
    /* TODO check max_completion_time */
    return satellite_test_one(picoquic_prague_algorithm, 100000000, 5300000, 250, 3, 0, 0, 0, 1, 0, 0);
}
```

### Rust test body
```rust
fn satellite_prague_seeded() {
    let prague = get_congestion_algorithm("prague").expect("prague");
    satellite_test_one(
        prague,
        100_000_000,
        5_300_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}
```

## `picoquictest/skip_frame_test.c:dataqueue_copy_test`
* C test-table name: `dataqueue_copy`
* C entry function: `dataqueue_copy_test`
* Rust test: `dataqueue_copy`
* C source: `picoquictest/skip_frame_test.c:3138-3185`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1042-1050`

### C test body
```c
{
    int ret = 0;
    picoquic_packet_t packet;
    uint8_t data[1536];
    uint8_t output[1536];
    size_t length_max = 1536;

    for (int case_opt = 0; ret == 0 && case_opt < 5; case_opt++) {
        int has_length = (case_opt & 1) == 0;
        int has_fin = (case_opt & 2) == 2;

        for (int basic_case = 1; ret == 0 && basic_case <= 6; basic_case++) {
            size_t next_frame = 0;
            size_t next_index = 0;
            size_t buffer_size = 0;
            size_t frame_length = 0;

            ret = dataqueue_prepare_test(basic_case, has_length, has_fin, &packet,
                &next_frame, &next_index, &buffer_size, &frame_length, data, length_max);

            if (ret != 0) {
                DBG_PRINTF("Prepare test fails for case %d, option &x", basic_case, case_opt);
                ret = -1;
            }
            else {
                uint8_t* next_byte = picoquic_copy_stream_frame_for_retransmit(
                    NULL, &packet, output, output + buffer_size);
                if (next_byte == NULL) {
                    DBG_PRINTF("Copy stream frame fails for case %d, option &x", basic_case, case_opt);
                    ret = -1;
                }
                else
                {
                    size_t output_length = next_byte - output;
                    if (dataqueue_verify_test(&packet, next_frame, next_index, frame_length, data, output, output_length) != 0) {
                        if (frame_length >= PICOQUIC_MIN_STREAM_DATA_FRAGMENT || output_length != 0) {
                            DBG_PRINTF("Verify data fails for case %d, option &x", basic_case, case_opt);
                            ret = -1;
                        }
                    }
                }
            }
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn dataqueue_copy() {
    for case_opt in 0..5i32 {
        let has_length = (case_opt & 1) == 0;
        let has_fin = (case_opt & 2) == 2;
        for basic_case in 1..=6i32 {
            dataqueue_copy_test_one(basic_case, has_length, has_fin).expect("dataqueue_copy");
        }
    }
}
```

## `picoquictest/skip_frame_test.c:cnxid_stash_test`
* C test-table name: `new_cnxid_stash`
* C entry function: `cnxid_stash_test`
* Rust test: `new_cnxid_stash`
* C source: `picoquictest/skip_frame_test.c:2358-2440`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1001-1012`

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;
    picoquic_quic_t * qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);


    memset(&saddr, 0, sizeof(struct sockaddr_in));
    if (qclient == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }

    /* First test: enqueue and dequeue immediately */
    /* Second test: enqueue all and then dequeue - verify order */
    /* Third test: enqueue all and then delete the connection */
    for (int test_mode = 0; ret == 0 && test_mode < 3; test_mode++) {
        picoquic_cnx_t * cnx = picoquic_create_cnx(qclient,
            picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr *) &saddr,
            simulated_time, 0, "test-sni", "test-alpn", 1);

        picoquic_remote_cnxid_t * stashed = NULL;

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create QUIC CNX context\n");
            ret = -1;
        } else {
            /* init the various connection id to a length compatible with test */
            cnx->path[0]->first_tuple->p_local_cnxid->cnx_id = stash_test_init_local;
            cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id = stash_test_init_remote;
        }

        for (size_t i = 0; ret == 0 && i < nb_stash_test_case; i++) {
            uint64_t transport_error = picoquic_stash_remote_cnxid(cnx, 0, 0,
                stash_test_case[i].sequence, stash_test_case[i].cnx_id.id_len,
                stash_test_case[i].cnx_id.id, stash_test_case[i].reset_secret, &stashed);
            if (transport_error != 0) {
                DBG_PRINTF("Test %d, cannot stash cnxid %d, err 0x%" PRIx64 ".\n", test_mode, i, transport_error);
                ret = -1;
            } else {
                if (stashed == NULL) {
                    DBG_PRINTF("Test %d, cannot stash cnxid %d (duplicate).\n", test_mode, i);
                    ret = -1;
                }
                else if (test_mode == 0) {
                    stashed = picoquic_obtain_stashed_cnxid(cnx, 0);
                    stashed->nb_path_references++;
                    ret = cnxid_stash_compare(test_mode, stashed, i);
                }
            }
        }

        /* Dequeue all in mode 1, verify order */
        if (test_mode == 1) {
            for (size_t i = 0; ret == 0 && i < nb_stash_test_case; i++) {
                stashed = picoquic_obtain_stashed_cnxid(cnx, 0);
                stashed->nb_path_references++;
                ret = cnxid_stash_compare(test_mode, stashed, i);
            }
        }

        /* Verify nothing left in queue in mode 0, 1 */
        if (test_mode < 2) {
            stashed = picoquic_obtain_stashed_cnxid(cnx, 0);
            if (stashed != NULL) {
                DBG_PRINTF("Test %d, unexpected cnxid left, #%d.\n", test_mode, (int)stashed->sequence);
                ret = -1;
            }
        }

        /* Delete the connecton and free the stash */
        picoquic_delete_cnx(cnx);
    }

    if (qclient != NULL) {
        picoquic_free(qclient);
    }

    return ret;
}
```

### Rust test body
```rust
fn new_cnxid_stash() {
    use crate::internal::{obtain_stashed_cnxid, stash_remote_cnxid};

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut cnx = quic.create_test_cnx(&mut simulated_time).expect("cnx");

    // Enqueue and dequeue one CID immediately.
    stash_remote_cnxid(&mut cnx, 1, &[0xaau8; 8], &[0u8; 16]).expect("stash");
    let stashed = obtain_stashed_cnxid(&mut cnx).expect("obtain");
    assert!(stashed, "stashed CID not retrieved");
}
```
