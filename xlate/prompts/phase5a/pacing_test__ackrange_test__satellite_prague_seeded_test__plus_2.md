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

## `picoquictest/pacing_test.c:pacing_test`
* C test-table name: `pacing`
* C entry function: `pacing_test`
* Rust test: `pacing`
* C source: `picoquictest/pacing_test.c:45-132`
* Rust source: `rs/fq/src/tests/pacing.rs:458-534`

### C test body
```c
{
    /* Create a connection so as to instantiate the pacing context */
    int ret = 0;
    uint64_t current_time = 0;
    picoquic_quic_t* quic = NULL;
    picoquic_cnx_t* cnx = NULL;
    struct sockaddr_in saddr;
    const uint64_t test_byte_per_sec = 1250000;
    const uint64_t test_quantum = 0x4000;
    int nb_sent = 0;
    int nb_round = 0;
    const int nb_target = 10000;

    quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, current_time,
        &current_time, NULL, NULL, 0);

    memset(&saddr, 0, sizeof(struct sockaddr_in));
    saddr.sin_family = AF_INET;
    saddr.sin_port = 1000;

    if (quic == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }
    else {
        cnx = picoquic_create_cnx(quic,
            picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr*) & saddr,
            current_time, 0, "test-sni", "test-alpn", 1);

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create connection\n");
            ret = -1;
        }
    }

    if (ret == 0) {
        /* Set pacing parameters to specified value */
        picoquic_update_pacing_rate(cnx->path[0], (double)test_byte_per_sec, test_quantum);
        /* Run a loop of N tests based on next wake time. */
        while (ret == 0 && nb_sent < nb_target) {
            nb_round++;
            if (nb_round > 4 * nb_target) {
                DBG_PRINTF("Pacing needs more that %d rounds for %d packets", nb_round, nb_target);
                ret = -1;
            }
            else {
                uint64_t next_time = current_time + 10000000;
                if (picoquic_is_sending_authorized_by_pacing(cnx, cnx->path[0], current_time, &next_time)) {
                    nb_sent++;
                    picoquic_update_pacing_after_send(cnx->path[0], cnx->path[0]->send_mtu, current_time);
                }
                else {
                    if (current_time < next_time) {
                        current_time = next_time;
                    }
                    else {
                        DBG_PRINTF("Pacing next = %" PRIu64", current = %d" PRIu64, next_time, current_time);
                        ret = -1;
                    }
                }
            }
        }

        /* Verify that the total send time matches expectations */
        if (ret == 0) {
            uint64_t volume_sent = ((uint64_t)nb_target) * cnx->path[0]->send_mtu;
            uint64_t time_max = ((volume_sent * 1000000) / test_byte_per_sec) + 1;
            uint64_t time_min = (((volume_sent - test_quantum) * 1000000) / test_byte_per_sec) + 1;

            if (current_time > time_max) {
                DBG_PRINTF("Pacing used = %" PRIu64", expected max = %d" PRIu64, current_time, time_max);
                ret = -1;
            }
            else if (current_time < time_min) {
                DBG_PRINTF("Pacing used = %" PRIu64", expected min = %d" PRIu64, current_time, time_min);
                ret = -1;
            }
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
fn pacing() {
    const TEST_BYTE_PER_SEC: u64 = 1_250_000;
    const TEST_QUANTUM: u64 = 0x4000;
    const NB_TARGET: i32 = 10_000;

    let mut current_time = Instant::from_ticks(0);

    let mut quic = crate::Quic::new(
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
    .expect("quic");

    let saddr: std::net::SocketAddr = "127.0.0.1:1000".parse().unwrap();

    let cnx = quic
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            Some(&saddr),
            current_time,
            0,
            Some("test-sni"),
            Some("test-alpn"),
            true,
        )
        .expect("cnx");

    cnx.paths[0].update_pacing_rate(TEST_BYTE_PER_SEC as f64, TEST_QUANTUM);

    let mut nb_sent = 0i32;
    let mut nb_round = 0i32;

    while nb_sent < NB_TARGET {
        nb_round += 1;
        assert!(
            nb_round <= 4 * NB_TARGET,
            "pacing needs more than {nb_round} rounds for {NB_TARGET} packets"
        );
        let mut next_time = current_time + Duration::from_ticks(10_000_000);
        if cnx.is_sending_authorized_by_pacing(0, current_time, &mut next_time) {
            nb_sent += 1;
            let send_mtu = cnx.paths[0].send_mtu;
            cnx.paths[0].update_pacing_after_send(send_mtu, current_time);
        } else {
            assert!(
                current_time < next_time,
                "pacing next={next_time:?} <= current={current_time:?}"
            );
            current_time = next_time;
        }
    }

    let send_mtu = cnx.paths[0].send_mtu;
    let volume_sent = NB_TARGET as u64 * send_mtu as u64;
    let time_max = (volume_sent * 1_000_000) / TEST_BYTE_PER_SEC + 1;
    let time_min = (volume_sent.saturating_sub(TEST_QUANTUM) * 1_000_000) / TEST_BYTE_PER_SEC + 1;
    let current_us = current_time.ticks();

    assert!(
        current_us <= time_max,
        "pacing used = {current_us}, expected max = {time_max}"
    );
    assert!(
        current_us >= time_min,
        "pacing used = {current_us}, expected min = {time_min}"
    );
}
```

## `picoquictest/sacktest.c:ackrange_test`
* C test-table name: `ack_range`
* C entry function: `ackrange_test`
* Rust test: `ack_range`
* C source: `picoquictest/sacktest.c:572-621`
* Rust source: `rs/fq/src/tests/sacktest.rs:715-744`

### C test body
```c
{
    int ret = 0;
    picoquic_sack_list_t sack0;

    picoquic_sack_list_init(&sack0);

    for (size_t i = 0; ret == 0 && i < nb_ack_range; i++) {
        ret = picoquic_check_sack_list(&sack0,
            ack_range[i].range_min, ack_range[i].range_max);

        if (ret == 0) {
            ret = picoquic_update_sack_list(&sack0,
                ack_range[i].range_min, ack_range[i].range_max, 0);
        }

        if (ret == 0) {
            ret = check_ack_ranges(&sack0);
        }

        for (size_t j = 0; j < i; j++) {
            if (picoquic_check_sack_list(&sack0,
                    ack_range[j].range_min, ack_range[j].range_max)
                == 0) {
                ret = -1;
                break;
            }
        }

        if (ret != 0) {
            break;
        }
    }

    if (ret == 0 && picoquic_sack_list_first(&sack0) != 0) {
        ret = -1;
    }

    if (ret == 0 && picoquic_sack_list_last(&sack0)!= 7500) {
        ret = -1;
    }

    if (ret == 0 && picoquic_sack_list_first_range(&sack0) != NULL) {
        ret = -1;
    }

    picoquic_sack_list_free(&sack0);

    return ret;
}
```

### Rust test body
```rust
fn ack_range() {
    let t0 = Instant::from_ticks(0);
    let mut sack0 = SackList::new();

    for (i, &(rmin, rmax)) in ACK_RANGES.iter().enumerate() {
        // C: picoquic_check_sack_list == 0 means not yet in sack.
        assert!(
            !sack0.check(rmin, rmax),
            "range ({rmin},{rmax}) already in sack at step {i}"
        );
        sack0.update(rmin, rmax, t0).expect("update sack");
        util::check_ack_ranges(&mut sack0);

        for &(jmin, jmax) in ACK_RANGES.iter().take(i) {
            assert!(
                sack0.check(jmin, jmax),
                "range ({jmin},{jmax}) not in sack at step {i}"
            );
        }
    }

    // Final state: [0..7500] fully covered (no gaps).
    // C: picoquic_sack_list_first == 0 (min) → Rust last()
    assert_eq!(sack0.last(), 0);
    // C: picoquic_sack_list_last == 7500 (max) → Rust first()
    assert_eq!(sack0.first(), 7500);
    assert!(sack0.first_range().is_none());

    sack0.free();
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

## `picoquictest/skip_frame_test.c:frames_ackack_error_test`
* C test-table name: `frames_ackack_error`
* C entry function: `frames_ackack_error_test`
* Rust test: `frames_ackack_error`
* C source: `picoquictest/skip_frame_test.c:1482-1513`
* Rust source: `rs/fq/src/tests/skip_frame.rs:972-979`

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    picoquic_quic_t* qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    struct sockaddr_in saddr = { 0 };
    picoquic_packet_t p;
    int nb_trials = 0;
    int nb_disconnected = 0;

    if (qclient == NULL) {
        ret = -1;
    }
    else {
        for (size_t i = 0; ret == 0 && i < nb_test_skip_list; i++) {
            for (int v = 1; v <= test_skip_list[i].nb_varints; v++) {
                int disconnected = 0;
                
                frame_ackack_error_packet(qclient, (struct sockaddr*)&saddr, simulated_time, &p, i, v,
                        test_skip_list[i].epoch, test_skip_list[i].mpath, &disconnected);
                nb_trials++;
                nb_disconnected += disconnected;
            }
        }
        picoquic_free(qclient);
    }
    DBG_PRINTF("%d ackack trials, %d disconnections", nb_trials, nb_disconnected);

    return ret;
}
```

### Rust test body
```rust
fn frames_ackack_error() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    // Exercise one representative frame through the ack-ack error path.
    let sample = &[0x01u8]; // PING
    let _disconnected = frame_ackack_error_packet(&mut quic, sample, 3, false, 1);
}
```

## `picoquictest/skip_frame_test.c:queue_network_input_test`
* C test-table name: `queue_network_input`
* C entry function: `queue_network_input_test`
* Rust test: `queue_network_input`
* C source: `picoquictest/skip_frame_test.c:3473-3580`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1098-1126`

### C test body
```c
{
    int ret = 0;

    uint64_t simulated_time = 0;
    picoquic_quic_t* quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    const size_t expected_length[3] = { 4, 2, 4 };
    const uint8_t expected[3][4] = {
        { 0, 1, 2, 3 },
        { 4, 5 },
        { 6, 7, 8, 9 }
    };

    const uint8_t data[10] = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 };
    int new_data_available = 0;

    picosplay_tree_t* tree = picosplay_new_tree(
        picoquic_stream_data_node_compare,
        picoquic_stream_data_node_create,
        picoquic_stream_data_node_delete,
        picoquic_stream_data_node_value);

    if (quic == NULL || tree == NULL) {
        ret = -1;
    }

    /* Fill 0..3 */
    if (ret == 0) {
        new_data_available = 0;
        if ((ret = picoquic_queue_network_input(quic, tree, 0, 0, data, 4, 1, NULL,
            &new_data_available)) != 0) {
            DBG_PRINTF("picoquic_queue_network_input(0, 0, 4) failed (%d)", ret);
        }
        else if (new_data_available == 0) {
            DBG_PRINTF("new_data_available doesn't signal new data (%d)", new_data_available);
            ret = 1;
        }
    }

    /* Fill 6..9 */
    if (ret == 0) {
        new_data_available = 0;
        if ((ret = picoquic_queue_network_input(quic, tree, 0, 6, data + 6, 4, 1, NULL, &new_data_available)) != 0) {
            DBG_PRINTF("picoquic_queue_network_input(0, 6, 4) failed (%d)", ret);
        } else if (new_data_available == 0) {
            DBG_PRINTF("new_data_available doesn't signal new data (%d)", new_data_available);
            ret = 1;
        }
    }

    /* Fill the gap from 4..5 with a chunk from 2..7 */
    if (ret == 0) {
        new_data_available = 0;
        if ((ret = picoquic_queue_network_input(quic, tree, 0, 2, data + 2, 6, 1, NULL, &new_data_available)) != 0) {
            DBG_PRINTF("picoquic_queue_network_input(0, 2, 6) failed (%d)", ret);
        } else if (new_data_available == 0) {
            DBG_PRINTF("new_data_available signals new data (%d)", new_data_available);
            ret = 1;
        }
    }

    /* No new data delivered by chunk 2..7 */
    if (ret == 0) {
        new_data_available = 0;
        if ((ret = picoquic_queue_network_input(quic, tree, 0, 2, data, 6, 1, NULL, &new_data_available)) != 0) {
            DBG_PRINTF("picoquic_queue_network_input(0, 2, 6) failed (%d)", ret);
        }

        if (new_data_available != 0) {
            DBG_PRINTF("new_data_available signals new data (%d)", new_data_available);
            ret = 1;
        }
    }

    if (ret == 0) {
        picoquic_stream_data_node_t* next = (picoquic_stream_data_node_t*)picosplay_first(tree);
        for (int i = 0; i < 3; ++i) {
            if (next == NULL) {
                DBG_PRINTF("tree does not contain enough data (%d chunks vs 3 exptected)", i);
                ret = 1;
                break;
            }
            else {
                if (expected_length[i] != next->length
                    || memcmp(next->bytes, expected[i], next->length) != 0) {
                    DBG_PRINTF("tree does not contain correct data (length: %zu vs %zu expected)", next->length, expected_length[i]);
                    ret = 1;
                    break;
                }
            }
            next = (picoquic_stream_data_node_t*)picosplay_next(&next->stream_data_node);
        }
    }

    if (tree != NULL) {
        picosplay_empty_tree(tree);
        free(tree);
    }

    if (quic != NULL) {
        picoquic_free(quic);
    }

    return ret;
}
```

### Rust test body
```rust
fn queue_network_input() {
    use crate::internal::{StreamDataSplay, queue_network_input};

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut tree = StreamDataSplay::new();

    let data: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    // Fill 0..3.
    let mut new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 0, &data[..4], true, &mut new_data)
        .expect("queue 0..3");
    assert!(new_data, "expected new data after 0..3");

    // Fill 6..9.
    new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 6, &data[6..], true, &mut new_data)
        .expect("queue 6..9");
    assert!(new_data, "expected new data after 6..9");

    // Fill 2..7 (fills the gap at 4..5 → delivers 0..9 contiguously).
    new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 2, &data[2..8], true, &mut new_data)
        .expect("queue 2..7");

    // Verify the tree contains the expected three contiguous segments.
    assert_eq!(tree.len(), 3, "expected 3 segments in tree");
}
```
