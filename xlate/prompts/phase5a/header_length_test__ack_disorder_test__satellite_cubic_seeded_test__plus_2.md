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

## `picoquictest/parseheadertest.c:header_length_test`
* C test-table name: `header_length`
* C entry function: `header_length_test`
* Rust test: `header_length`
* C source: `picoquictest/parseheadertest.c:1205-1216`
* Rust source: `rs/fq/src/tests/parseheadertest.rs:916-1485`

### C test body
```c
{
    int ret = 0;
    for (size_t i = 0; i < nb_header_length_cases; i++) {
        ret = header_length_test_one(&header_length_case[i]);
        if (ret != 0) {
            DBG_PRINTF("Header length test %zu fails", i);
            break;
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn header_length() {
    const U64MAX: Option<u64> = None;
    let cases: &[HlCase] = &[
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 63,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 64,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 255,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xff_ffff_ffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 255,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xff_ffff_ffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
    ];

    for (i, hlc) in cases.iter().enumerate() {
        header_length_test_one(hlc);
        let _ = i; // used in assertions within header_length_test_one
    }
}
```

## `picoquictest/sacktest.c:ack_disorder_test`
* C test-table name: `ack_disorder`
* C entry function: `ack_disorder_test`
* Rust test: `ack_disorder`
* C source: `picoquictest/sacktest.c:784-788`
* Rust source: `rs/fq/src/tests/sacktest.rs:751-753`

### C test body
```c
{
    int ret = ack_disorder_test_one(ACK_DISORDER_LOG, 0, 133.0);
    return ret;
}
```

### Rust test body
```rust
fn ack_disorder() {
    ack_disorder_one("ack_disorder_test.csv", 0, 133.0);
}
```

## `picoquictest/satellite_test.c:satellite_cubic_seeded_test`
* C test-table name: `satellite_cubic_seeded`
* C entry function: `satellite_cubic_seeded_test`
* Rust test: `satellite_cubic_seeded`
* C source: `picoquictest/satellite_test.c:290-293`
* Rust source: `rs/fq/src/tests/satellite.rs:418-433`

### C test body
```c
{
    return satellite_test_one(picoquic_cubic_algorithm, 100000000, 5000000, 250, 3, 0, 0, 0, 1, 0, 0);
}
```

### Rust test body
```rust
fn satellite_cubic_seeded() {
    let cubic = get_congestion_algorithm("cubic").expect("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        5_000_000,
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

## `picoquictest/satellite_test.c:satellite_preemptive_fc_test`
* C test-table name: `satellite_preemptive_fc`
* C entry function: `satellite_preemptive_fc_test`
* Rust test: `satellite_preemptive_fc`
* C source: `picoquictest/satellite_test.c:332-336`
* Rust source: `rs/fq/src/tests/satellite.rs:494-499`

### C test body
```c
{
    /* Should be less than 10 sec per draft etosat, but cubic is a bit slower */
    return satellite_test_one(picoquic_bbr_algorithm, 10000000, 20000000, 20, 2, 0, 1, 1, 0, 1, 0);
}
```

### Rust test body
```rust
fn satellite_preemptive_fc() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr, 10_000_000, 20_000_000, 20, 2, 0, true, true, false, true, false,
    );
}
```

## `picoquictest/skip_frame_test.c:dataqueue_packet_test`
* C test-table name: `dataqueue_packet`
* C entry function: `dataqueue_packet_test`
* Rust test: `dataqueue_packet`
* C source: `picoquictest/skip_frame_test.c:3240-3319`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1054-1058`

### C test body
```c
{
    picoquic_quic_t* qtest = NULL;
    picoquic_cnx_t* cnx = NULL;
    int ret = 0;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;

    memset(&saddr, 0, sizeof(struct sockaddr_in));

    /* Initialize the connection context */
    if (ret == 0) {
        qtest = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
            NULL, NULL, NULL, NULL, simulated_time,
            &simulated_time, NULL, NULL, 0);
        if (qtest == NULL) {
            DBG_PRINTF("%s", "Cannot create QUIC context\n");
            ret = -1;
        }
        else {
            cnx = picoquic_create_cnx(qtest,
                picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr*) & saddr,
                simulated_time, 0, "test-sni", "test-alpn", 1);

            if (cnx == NULL) {
                ret = -1;
            }
            else {
                /* Create stream 0 so the later tests succeed */
                uint8_t new_bytes[256];
                memset(new_bytes, 0, sizeof(new_bytes));
                if ((ret = picoquic_add_to_stream(cnx, 0, new_bytes, sizeof(new_bytes), 0)) != 0) {
                    DBG_PRINTF("%s", "Cannot initialize stream 0\n");
                    ret = -1;
                }
            }
        }
    }

    if (ret == 0) {
        /* Create a packet and chain it to the data queue */
        picoquic_packet_t* packet = picoquic_create_packet(qtest);
        if (packet == NULL) {
            ret = -1;
        }
        else {
            (void)dataqueue_prepare_packet(packet, 1, 0, 0, 0, 256);
            picoquic_queue_data_repeat_packet(cnx, packet);
        }
    }

    if (ret == 0 &&
        (ret = dataqueue_packet_test_iterate(1, cnx, 2, 0, 1, 1)) == 0 &&
        (ret = dataqueue_packet_test_iterate(2, cnx, 128, 0, 1, 1)) == 0 &&
        (ret = dataqueue_packet_test_iterate(3, cnx, 1024, 1, 0, 0)) == 0) {
        ret = dataqueue_packet_test_iterate(4, cnx, 1024, 0, 0, 1);
    }

    if (ret == 0) {
        /* Create a packet and chain it to the data queue */
        picoquic_packet_t* packet = picoquic_create_packet(qtest);
        if (packet == NULL) {
            ret = -1;
        }
        else {
            (void)dataqueue_prepare_packet(packet, 1, 0, 0, 0, 256);
            picoquic_dequeue_data_repeat_packet(cnx, packet);
        }
    }

    /* Free the connection context */
    if (cnx != NULL) {
        picoquic_delete_cnx(cnx);
    }

    if (qtest != NULL) {
        picoquic_free(qtest);
    }
    return ret;
}
```

### Rust test body
```rust
fn dataqueue_packet() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    dataqueue_packet_test_iterate(&mut quic).expect("dataqueue_packet");
}
```
