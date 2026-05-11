# Phase 5B repair Rust test mismatches

You are repairing Phase 5A `needs_fix` entries.  The goal
is to make the Rust tests faithfully check the same behavior
as the C tests.

Rules:

* Edit Rust tests, Rust test helpers, and Rust test fixtures
  only: `rs/fq/src/tests/` and `rs/fq/tests/fixtures/`.
* Do not edit C sources.
* Do not weaken assertions, skip important C cases, or replace
  tests with placeholders.
* If the test already matches after closer inspection, report
  `ok` and do not edit source.
* Phase 5B is about test/API correspondence, not test success.
  The Rust test must exist, compile as a test, and be runnable
  by the Rust test harness, but it may fail arbitrarily early
  because the Rust library implementation is incomplete.
* Do not report `blocked` merely because the implementation
  returns the wrong state, fails a handshake, lacks protocol
  behavior, or would fail the test. Those are Phase 5C issues.
* Report `blocked` only when the faithful test cannot be
  written, compiled, or exposed as a runnable Rust test because
  the necessary Rust API/test-harness surface is missing or
  ambiguous.
* Do not run full `cargo test` in this pass. Use source review
  and, if needed, `cargo check --tests` for compile validation.

Owned Rust test file(s): `rs/fq/src/tests/tls_api.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/tls_api_test.c:grease_quic_bit_test`
* C test-table name: `grease_quic_bit`
* C entry function: `grease_quic_bit_test`
* Rust test: `grease_quic_bit`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:2751-2753`
* Phase 5A analysis: The Rust wrapper passes the correct false flag and checks the main GREASE bit flags, but the helper changes the C very_long scenario input and relies on a Rust scenario verifier that only closes instead of verifying stream completion.
* Phase 5A fix note: Use the C-equivalent very_long stream input (q_len 257, r_len 1000000) and ensure scenario completion is asserted before the existing quic_bit_greased/quic_bit_received_0 checks.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Wrapper calls grease_quic_bit_test_one(false), and the helper uses the very-long scenario and GREASE flag checks, but it builds client TransportParameters from Default instead of calling init_transport_parameters as C does before setting do_grease_quic_bit.
* Phase 5C fix note: Initialize client transport parameters with init_transport_parameters, then set do_grease_quic_bit=true before tls_api_one_scenario_init_ex.

### C test body
```c
{
    return  grease_quic_bit_test_one(0);
}
```

### Current Rust test body
```rust
fn grease_quic_bit() {
    grease_quic_bit_test_one(false).expect("grease_quic_bit");
}
```

## `picoquictest/tls_api_test.c:key_rotation_stress_test`
* C test-table name: `key_rotation_stress`
* C entry function: `key_rotation_stress_test`
* Rust test: `key_rotation_stress`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:3488-3490`
* Phase 5A analysis: The wrapper passes 10, but the Rust helper is not the C stress behavior: it uses a different one-stream scenario, starts 10 rotations immediately before data transfer, ignores rotation errors, omits the send-sequence/key-phase gated rotation loop, and omits the server close/deletion wait.
* Phase 5A fix note: Translate the C stress loop using test_scenario_sustained, rotation_sequence gating every nb_packets, max_rotations 100, checked start_key_rotation results, close, and the 4s server-close wait.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The repaired local stress helper matches the C sustained-transfer rotation loop, but tls_api.rs still imports the stale util::key_rotation_stress_test_one while defining a same-named local helper, so the merged test surface is not cleanly runnable/resolved.
* Phase 5C fix note: Remove the stale util import, and preferably retire the stale util helper, so key_rotation_stress resolves to the repaired tls_api.rs helper.

### C test body
```c
{
    return key_rotation_stress_test_one(10);
}
```

### Current Rust test body
```rust
fn key_rotation_stress() {
    key_rotation_stress_test_one(10).expect("key_rotation_stress");
}
```

## `picoquictest/tls_api_test.c:long_rtt_test`
* C test-table name: `long_rtt`
* C entry function: `long_rtt_test`
* Rust test: `long_rtt`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:3606-3633`
* Phase 5A analysis: Rust sets 300 ms link latency but runs an empty scenario, omits the C very-long 1 MB transfer, omits queue_delay_max=2*latency, and uses a much looser completion target.
* Phase 5A fix note: Run the very-long scenario {stream 4, q_len 257, r_len 1000000}, preserve 300 ms each-way latency, pass queue_delay_max=600000, and verify completion against the C 3600000 usec target.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test uses the right CID, 300 ms link latency, very-long scenario, and 3.6s target, but the called tls_api_one_scenario_body ignores its queue_delay_max argument, so the C 2*latency queue-delay behavior is not actually exercised.
* Phase 5C fix note: Route 2*LATENCY into the connection loop, e.g. by fixing tls_api_one_scenario_body or using the _ex path that passes queue_delay_max through.

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    uint64_t latency = 300000ull; /* assume that each direction is 300 ms, e.g. satellite link */
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x10, 0x10, 30, 0, 0, 0, 0, 0}, 8 };

    ret = tls_api_one_scenario_init_ex(&test_ctx, &simulated_time,
        0, NULL, NULL, &initial_cid);

    if (ret == 0) {
        /* set the delay estimate, then launch the test */
        test_ctx->c_to_s_link->microsec_latency = latency;
        test_ctx->s_to_c_link->microsec_latency = latency;

        picoquic_set_qlog(test_ctx->qserver, ".");

        /* The transmission delay cannot be less than 2.6 sec:
         * 3 handshakes at 1 RTT each = 1.8 sec, plus
         * 1MB over a 10Mbps link = 0.8 sec. We observe
         * 3.31 seconds instead, i.e. 1.51 sec for the
         * data transmission. This is due to the slow start
         * phase of the congestion control, which we accelerated
         * but could not completely fix. */
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 0, 2*latency,
            3600000);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn long_rtt() {
    const LATENCY: u64 = 300_000;
    const TEST_SCENARIO_VERY_LONG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    let mut t = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x10, 0x10, 30, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut ctx = tls_api_init_ctx_ex(&mut t, 0, None, Some(&initial_cid)).expect("ctx");
    ctx.c_to_s_link.microsec_latency = LATENCY;
    ctx.s_to_c_link.microsec_latency = LATENCY;
    ctx.qserver.set_qlog(".").ok();
    tls_api_one_scenario_body(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_VERY_LONG,
        0,
        0,
        0,
        2 * LATENCY,
        3_600_000,
    )
    .expect("long_rtt");
}
```

## `picoquictest/tls_api_test.c:tls_api_many_losses`
* C test-table name: `many_losses`
* C entry function: `tls_api_many_losses`
* Rust test: `many_losses`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:3987-4016`
* Phase 5A analysis: Rust covers only eight fixed masks and its tls_api_loss_test ignores the supplied mask; it omits the C preprogrammed mask matrix and the 50 deterministic 30% random-loss scenario runs.
* Phase 5A fix note: Recreate the C mask loops, deterministic random mask generation from 0x1055ca45c001baba, and ensure Rust loss/scenario helpers actually apply each loss mask including the q_and_r max_data=128000 random-loss scenario.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust mirrors the C loss-mask matrix and deterministic random q_and_r scenario, but the preprogrammed-mask helper weakens the C tls_api_loss_test checks by omitting the final TLS API assertions for transport extension, SNI, ALPN, and negotiated version.
* Phase 5C fix note: Make many_losses_loss_test use or mirror the Rust tls_api_test_with_loss/tls_api_test_with_loss_final surface so each preprogrammed handshake-loss case also verifies the C final TLS negotiation checks before closing.

### C test body
```c
{
    uint64_t loss_mask = 0;
    int ret = 0;
    uint64_t random_context = 0x1055ca45c001babaull;

    /* We first test with a set of preprogrammed masks, checking consecutive drops */
    for (int i = 0; ret == 0 && i < 6; i++) {
        for (int j = 0; ret == 0 && j < 4; j++) {
            uint64_t j_mask = ~(UINT64_MAX << j);
            loss_mask = j_mask << i;
            ret = tls_api_loss_test(loss_mask);
            if (ret != 0) {
                DBG_PRINTF("Handshake fails for mask %d-%d = %llx", i, j, (unsigned long long)loss_mask);
            }
        }
        for (uint64_t j = 8; ret == 0 && j < 11; j++) {
            loss_mask = (j | (j << 4) | (j << 8))<<i;
            ret = tls_api_loss_test(loss_mask);
            if (ret != 0) {
                DBG_PRINTF("Handshake fails for mask %d, %" PRIu64" = %llx", i, j,  (unsigned long long)loss_mask);
            }
        }
    }

    /* Then we verify that we can establish 50 connections with packet drop rate=30% */
    for (int i = 0; ret == 0 &&  i < 50; i++)
    {
        uint64_t loss_mask = 0;
        for (int j = 0; j < 64; j++)
        {
            loss_mask <<= 1;

            if (picoquic_test_uniform_random(&random_context, 1000) < 300) {
                loss_mask |= 1;
            }
        }

        ret = tls_api_one_scenario_test(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, loss_mask, 128000, 0, 0, 0, NULL, NULL);
        if (ret != 0) {
            DBG_PRINTF("Handshake fails for random mask %d, mask = %llx", i, (unsigned long long)loss_mask);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn many_losses() {
    for i in 0..6u32 {
        for j in 0..4u32 {
            let j_mask = if j == 0 { 0 } else { (1u64 << j) - 1 };
            let loss_mask = j_mask << i;
            many_losses_loss_test(loss_mask)
                .unwrap_or_else(|e| panic!("many_losses mask {i}-{j}={loss_mask:#x}: {e:?}"));
        }

        for j in 8u64..11 {
            let loss_mask = (j | (j << 4) | (j << 8)) << i;
            many_losses_loss_test(loss_mask)
                .unwrap_or_else(|e| panic!("many_losses mask {i},{j}={loss_mask:#x}: {e:?}"));
        }
    }

    let mut random_context = 0x1055_ca45_c001_babau64;
    for i in 0..50 {
        let mut loss_mask = 0u64;
        for _ in 0..64 {
            loss_mask <<= 1;
            if test_uniform_random(&mut random_context, 1000) < 300 {
                loss_mask |= 1;
            }
        }

        many_losses_q_and_r_scenario(loss_mask)
            .unwrap_or_else(|e| panic!("many_losses random mask {i}={loss_mask:#x}: {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:migration_test_loss`
* C test-table name: `migration_with_loss`
* C entry function: `migration_test_loss`
* Rust test: `migration_with_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:4256-4258`
* Phase 5A analysis: Rust passes loss mask 0x09, but uses an empty/default scenario instead of C's q_and_r 257/2000 case and omits the C migration checks for challenge renewal/verification and connection ID changes.
* Phase 5A fix note: Pass the q_and_r scenario and implement the C helper's migration assertions: propagate probe errors, apply loss during transfer, verify scenario completion, wait for path challenge validation, and check remote/local CID changes.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust wrapper/helper match C q_and_r, loss mask 0x09, cid_zero=false, and migration CID/challenge assertions, but the final test tree is not compile-exposed due merged tls_api.rs/util.rs harness damage.
* Phase 5C fix note: Repair duplicate tls_api.rs TEST_SCENARIO_* constants and duplicate/misplaced util.rs TestTlsApiCtx harness items; no pair-specific semantic mismatch found.

### C test body
```c
{
    uint64_t loss_mask = 0x09;

    return migration_test_scenario(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), loss_mask, 0);
}
```

### Current Rust test body
```rust
fn migration_with_loss() {
    migration_test_scenario(TEST_SCENARIO_Q_AND_R, 0x09, false).expect("migration_with_loss");
}
```
