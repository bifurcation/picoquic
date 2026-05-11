# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/mediatest.c:mediatest_video2_probe_test`
* C test-table name: `mediatest_video2_probe`
* C entry function: `mediatest_video2_probe_test`
* Rust test: `mediatest_video2_probe`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1488-1501`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Spec fields match C, but Rust mediatest_one is a generic stream scenario and does not reproduce C's media harness or media latency-stat checks for audio/video/video2; the latency bounds are not enforced equivalently.
* Phase 5A fix note: Port or restore a faithful mediatest harness with media callbacks/statistics, probe-up behavior, and average/max latency checks for audio, video, and video2.
* Phase 5B analysis: Existing Rust #[test] matches the C API-level contract, including BBR, bandwidth 0.1, audio/video/video2, default data_size 0, latency bounds, probe-up, and audio/video/video2 stat checks. Any early media/transport runtime failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: No Rust test changes; appended COMMANDS.log entry.

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.1;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.latency_average = 25000;
    spec.latency_max = 150000;
    spec.do_probe_up = 1;
    ret = mediatest_one(mediatest_video2_probe, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_video2_probe() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 25_000,
        latency_max: 150_000,
        do_probe_up: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Probe, &spec).expect("mediatest_video2_probe");
}
```

## `picoquictest/multipath_test.c:monopath_hole_test`
* C test-table name: `monopath_hole`
* C entry function: `monopath_hole_test`
* Rust test: `monopath_hole`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1456-1458`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test selects the right monopath Hole scenario and checks holes inserted, but the shared Rust scenario verification is placeholder-like: it does not verify stream completion, payload receipt, completion time, or readiness failure the way the C helper path does.
* Phase 5A fix note: Implement Rust TLS scenario verification/completion-time checks and make readiness/wait helpers fail when the required state is not reached.
* Phase 5B analysis: Rust #[test] monopath_hole is present and calls monopath_test_one(MonopathTestId::Hole), matching C monopath_hole_test -> monopath_test_one(monopath_test_hole). The shared Rust helper includes the Hole setup and API-visible holes-inserted assertion; callback/stream-completion failures are Phase 5C runtime implementation issues, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    return monopath_test_one(monopath_test_hole);
}
```

### Current Rust test body
```rust
fn monopath_hole() {
    monopath_test_one(MonopathTestId::Hole);
}
```

## `picoquictest/multipath_test.c:multipath_dg_af_test`
* C test-table name: `multipath_dg_af`
* C entry function: `multipath_dg_af_test`
* Rust test: `multipath_dg_af`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1602-1604`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper uses the correct timeout and DgAf enum, and datagram-affinity checks mostly mirror C, but the shared scenario verifier drops C's stream-completion and completion-time assertions.
* Phase 5A fix note: Repair shared scenario verification and path-readiness checks so the datagram-affinity test also proves the underlying transfer completed within 1,100,000 us.
* Phase 5B analysis: Rust test is present, compiles as a Rust harness test, and matches the C API-level wrapper by calling multipath_test_one(1_100_000, MultipathTestId::DgAf). The shared Rust helper includes the second-path readiness check, stream completion bound, and datagram affinity assertions; any early failure from incomplete multipath negotiation is Phase 5C implementation work.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 1100000;

    return multipath_test_one(max_completion_microsec, multipath_test_dg_af);
}
```

### Current Rust test body
```rust
fn multipath_dg_af() {
    multipath_test_one(1_100_000, MultipathTestId::DgAf);
}
```

## `picoquictest/netperf_test.c:netperf_basic_test`
* C test-table name: `netperf_basic`
* C entry function: `netperf_basic_test`
* Rust test: `netperf_basic`
* Expected Rust file: `rs/fq/src/tests/netperf.rs`
* Current Rust span: `rs/fq/src/tests/netperf.rs:246-254`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust preserves the basic scenario inputs, zero loss, deadline argument, and packet-count assertions, but it does not actually use the 10-packet send buffer/coalesced-send path that the C netperf test exercises. The shared Rust verifier also ignores the 1,000,000 us max completion time.
* Phase 5A fix note: Implement the netperf large-send-buffer path in Rust, including coalesced packet preparation/splitting comparable to C, and enforce max_completion_microsec.
* Phase 5B analysis: Rust netperf_basic already matches the C entry's API-level contract: basic scenario, no congestion algorithm, zero loss/max-data/queue-delay/version defaults, 1,000,000 us completion bound, and 10 * MAX_PACKET_SIZE send buffer. Any early failure from large-buffer Initial/Handshake behavior is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = netperf_one_scenario(netperf_scenario_basic, sizeof(netperf_scenario_basic),
        NULL,
        0, 0, 0, 0, 0, 1000000, NULL, NULL, 10 * PICOQUIC_MAX_PACKET_SIZE);

    return ret;
}
```

### Current Rust test body
```rust
fn netperf_basic() {
    netperf_one_scenario(
        NETPERF_SCENARIO_BASIC,
        None,
        0,
        1_000_000,
        10 * MAX_PACKET_SIZE,
    );
}
```

## `picoquictest/sacktest.c:sacktest`
* C test-table name: `ack_sack`
* C entry function: `sacktest`
* Rust test: `ack_sack`
* Expected Rust file: `rs/fq/src/tests/sacktest.rs`
* Current Rust span: `rs/fq/src/tests/sacktest.rs:500-627`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Most inputs and duplicate/timestamp checks match, but the Rust test uses SackList::first_range(), which returns the topmost range, not the C picoquic_sack_list_first_range second-range helper. The C assertion checks that there is no second range; the Rust assertion checks different behavior.
* Phase 5A fix note: Use the Rust free helper matching C picoquic_sack_list_first_range, or otherwise assert absence of a second SACK range, in both pn=0 and final contiguous checks.
* Phase 5B analysis: Current Rust #[test] ack_sack is present, compiles under the Rust test harness, and mirrors the C API-level contract, including local CID setup, pn=0 checks, SACK first/last/no-second-range assertions, TEST_PNS record/query/duplicate loops, and final timestamp/range assertions. The duplicate pn 3 return mismatch is a Phase 5C library behavior failure, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    picoquic_cnx_t *cnx;
    picoquic_quic_t* quic;
    uint64_t current_time = 0;
    uint64_t highest_seen = 0;
    uint64_t highest_seen_time = 0;
    picoquic_packet_context_enum pc = 0;

    if (picoquic_test_set_minimal_cnx(&quic, &cnx) != 0) {
        return -1;
    }

    if (picoquic_create_local_cnxid(cnx, 0, NULL, 0) == NULL) {
        return -1;
    }

    /* Do a basic test with packet zero */
    if (picoquic_is_pn_already_received(cnx, pc,
        cnx->first_local_cnxid_list->local_cnxid_first, 0) != 0) {
        ret = -1;
    }
    else if (picoquic_record_pn_received(cnx, pc, cnx->first_local_cnxid_list->local_cnxid_first,
        0, current_time) != 0) {
        ret = -1;
    }
    else if (picoquic_is_pn_already_received(cnx, pc,
        cnx->first_local_cnxid_list->local_cnxid_first, 0) == 0) {
        ret = -1;
    }
    else if (picoquic_sack_list_first(&cnx->ack_ctx[pc].sack_list) != 0 ||
        picoquic_sack_list_last(&cnx->ack_ctx[pc].sack_list) != 0 ||
        picoquic_sack_list_first_range(&cnx->ack_ctx[pc].sack_list) != NULL) {
        ret = -1;
    }
    else {
        /* reset for the next test */
        picoquic_test_reset_minimal_cnx(quic, &cnx);
    }

    if (ret == 0) {
        ret = check_ack_ranges(&cnx->ack_ctx[pc].sack_list);
    }

    for (size_t i = 0; ret == 0 && i < nb_test_pn64; i++) {
        current_time = ((uint64_t)i) * 100 + 1;

        if (test_pn64[i] > highest_seen) {
            highest_seen = test_pn64[i];
            highest_seen_time = current_time;
        }

        if (picoquic_record_pn_received(cnx, pc, cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[i], current_time) != 0) {
            ret = -1;
        }

        if (ret == 0) {
            ret = check_ack_ranges(&cnx->ack_ctx[pc].sack_list);
        }

        for (size_t j = 0; ret == 0 && j <= i; j++) {
            if (picoquic_is_pn_already_received(cnx, pc,
                cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[j]) == 0) {
                ret = -1;
            }

            if (picoquic_record_pn_received(cnx, pc, cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[j], current_time) != 1) {
                ret = -1;
            }
        }

        for (size_t j = i + 1; ret == 0 && j < nb_test_pn64; j++) {
            if (picoquic_is_pn_already_received(cnx, pc,
                cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[j]) != 0) {
                ret = -1;
            }
        }
    }

    if (ret == 0) {
        if (picoquic_sack_list_last(&cnx->ack_ctx[pc].sack_list) != 21 ||
            picoquic_sack_list_first(&cnx->ack_ctx[pc].sack_list) != 0 ||
            cnx->ack_ctx[pc].time_stamp_largest_received != highest_seen_time ||
            picoquic_sack_list_first_range(&cnx->ack_ctx[pc].sack_list) != NULL) {
            ret = -1;
        }
    }

    /* Free the sack lists*/
    picoquic_test_delete_minimal_cnx(&quic, &cnx);

    return ret;
}
```

### Current Rust test body
```rust
fn ack_sack() {
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

    // Phase 1: basic pn=0 case (mirrors C before the reset).
    {
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

        let l_cid = cnx.create_local_connection_id(0, None, t0).expect("l_cid");

        assert!(
            !cnx.is_pn_already_received(pc, Some(l_cid), 0),
            "pn 0 should not be received yet"
        );
        assert_eq!(
            cnx.record_pn_received(pc, Some(l_cid), 0, t0),
            0,
            "first record of pn 0 should return 0"
        );
        assert!(
            cnx.is_pn_already_received(pc, Some(l_cid), 0),
            "pn 0 should now be received"
        );
        // C: picoquic_sack_list_first == 0 (min PN), == Rust last()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.last(), 0);
        // C: picoquic_sack_list_last == 0 (max PN), == Rust first()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.first(), 0);
        // C: no second range after the first ascending SACK range.
        assert!(picoquic_sack_list_first_range(&cnx.ack_ctx[pc as usize].sack_list).is_none());
    }

    // Phase 2: fresh connection (mirrors picoquic_test_reset_minimal_cnx).
    {
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
            .expect("cnx after reset");

        let l_cid = cnx
            .create_local_connection_id(0, None, t0)
            .expect("l_cid after reset");

        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        let mut highest_seen = 0u64;
        let mut highest_seen_time = t0;

        for (i, &pn) in TEST_PNS.iter().enumerate() {
            let current_time = Instant::from_ticks(i as u64 * 100 + 1);

            if pn > highest_seen {
                highest_seen = pn;
                highest_seen_time = current_time;
            }

            assert_eq!(
                cnx.record_pn_received(pc, Some(l_cid), pn, current_time),
                0,
                "record pn {pn} at step {i}"
            );
            util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

            for &pn_j in &TEST_PNS[..=i] {
                assert!(
                    cnx.is_pn_already_received(pc, Some(l_cid), pn_j),
                    "pn {pn_j} should be received at step {i}"
                );
                assert_eq!(
                    cnx.record_pn_received(pc, Some(l_cid), pn_j, current_time),
                    1,
                    "duplicate record of pn {pn_j} should return 1 at step {i}"
                );
            }

            for &pn_j in &TEST_PNS[i + 1..] {
                assert!(
                    !cnx.is_pn_already_received(pc, Some(l_cid), pn_j),
                    "pn {pn_j} should not yet be received at step {i}"
                );
            }
        }

        // Final state: [0..21] all received, contiguous.
        // C: picoquic_sack_list_last == 21 (max) → Rust first()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.first(), 21);
        // C: picoquic_sack_list_first == 0 (min) → Rust last()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.last(), 0);
        assert_eq!(
            cnx.ack_ctx[pc as usize].time_stamp_largest_received,
            highest_seen_time
        );
        // C: no second range after the first ascending SACK range.
        assert!(picoquic_sack_list_first_range(&cnx.ack_ctx[pc as usize].sack_list).is_none());
    }
}
```
