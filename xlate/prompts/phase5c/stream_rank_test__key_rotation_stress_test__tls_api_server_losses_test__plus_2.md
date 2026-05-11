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

## `picoquictest/stream0_frame_test.c:stream_rank_test`
* C test-table name: `stream_rank`
* C entry function: `stream_rank_test`
* Rust test: `stream_rank`
* Expected Rust file: `rs/fq/src/tests/stream0_frame.rs`
* Current Rust span: `rs/fq/src/tests/stream0_frame.rs:755-809`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The tables and formulas match, but the Rust test uses local closures instead of the translated production stream-id helpers, so it would pass even if StreamId::rank or StreamId::from_parts were wrong.
* Phase 5A fix note: Rewrite the Rust assertions to exercise crate::stream::StreamId::rank and StreamId::from_parts with Role and Direction for all four quadrants.
* Phase 5B analysis: Rust test now exercises the translated StreamId rank/from_parts helpers for the same four C stream-id quadrants.
* Phase 5B fix note: Replaced local bit-formula closures with StreamId::rank and StreamId::from_parts using Role and Direction.

### C test body
```c
{
    uint64_t stream_rank[] = { 1, 2, 3, 1000, 10000 };
    uint64_t stream_client_bidir[] = { 0, 4, 8, 3996, 39996 };
    uint64_t stream_client_unidir[] = { 2, 6, 10, 3998, 39998 };
    uint64_t stream_server_bidir[] = { 1, 5, 9, 3997, 39997 };
    uint64_t stream_server_unidir[] = { 3, 7, 11, 3999, 39999 };
    size_t n = sizeof(stream_rank) / sizeof(uint64_t);
    int ret = 0;

    ret |= stream_rank_test_one(n, stream_rank, stream_client_bidir, 0, 1);
    ret |= stream_rank_test_one(n, stream_rank, stream_client_unidir, 1, 1);
    ret |= stream_rank_test_one(n, stream_rank, stream_server_bidir, 0, 0);
    ret |= stream_rank_test_one(n, stream_rank, stream_server_unidir, 1, 0);

    return ret;
}
```

### Current Rust test body
```rust
fn stream_rank() {
    let ranks: [u64; 5] = [1, 2, 3, 1000, 10000];
    let client_bidir: [u64; 5] = [0, 4, 8, 3996, 39996];
    let client_unidir: [u64; 5] = [2, 6, 10, 3998, 39998];
    let server_bidir: [u64; 5] = [1, 5, 9, 3997, 39997];
    let server_unidir: [u64; 5] = [3, 7, 11, 3999, 39999];

    for i in 0..5 {
        let r = ranks[i];

        assert_eq!(
            StreamId(client_bidir[i]).rank(),
            r,
            "rank_from_id client_bidir[{i}]"
        );
        assert_eq!(
            StreamId::from_parts(r, Role::Client, Direction::Bidir).0,
            client_bidir[i],
            "id_from_rank client_bidir[{i}]"
        );

        assert_eq!(
            StreamId(client_unidir[i]).rank(),
            r,
            "rank_from_id client_unidir[{i}]"
        );
        assert_eq!(
            StreamId::from_parts(r, Role::Client, Direction::Unidir).0,
            client_unidir[i],
            "id_from_rank client_unidir[{i}]"
        );

        assert_eq!(
            StreamId(server_bidir[i]).rank(),
            r,
            "rank_from_id server_bidir[{i}]"
        );
        assert_eq!(
            StreamId::from_parts(r, Role::Server, Direction::Bidir).0,
            server_bidir[i],
            "id_from_rank server_bidir[{i}]"
        );

        assert_eq!(
            StreamId(server_unidir[i]).rank(),
            r,
            "rank_from_id server_unidir[{i}]"
        );
        assert_eq!(
            StreamId::from_parts(r, Role::Server, Direction::Unidir).0,
            server_unidir[i],
            "id_from_rank server_unidir[{i}]"
        );
    }
}
```

## `picoquictest/tls_api_test.c:key_rotation_stress_test`
* C test-table name: `key_rotation_stress`
* C entry function: `key_rotation_stress_test`
* Rust test: `key_rotation_stress`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3536-3538`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper passes 10, but the Rust helper is not the C stress behavior: it uses a different one-stream scenario, starts 10 rotations immediately before data transfer, ignores rotation errors, omits the send-sequence/key-phase gated rotation loop, and omits the server close/deletion wait.
* Phase 5A fix note: Translate the C stress loop using test_scenario_sustained, rotation_sequence gating every nb_packets, max_rotations 100, checked start_key_rotation results, close, and the 4s server-close wait.
* Phase 5B analysis: Rust test is present, compiles, and is runnable. Removed a Rust-only zero-rotation failure; any remaining send_sequence/key-rotation behavior failures are Phase 5C, not Phase 5B blockers.
* Phase 5B fix note: Removed the extra nb_rotation == 0 assertion so the Rust stress helper matches the C API-level contract: attempt rotations when the C gate opens, cap at max_rotations, propagate start_key_rotation/close/server-close errors.

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

## `picoquictest/tls_api_test.c:tls_api_server_losses_test`
* C test-table name: `server_losses`
* C entry function: `tls_api_server_losses_test`
* Rust test: `server_losses`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7328-7330`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls tls_api_loss_test(6), but the Rust helper ignores its loss_mask argument and runs with zero loss, so it does not test recovery from the server-loss mask used by C.
* Phase 5A fix note: Thread the loss mask through Rust tls_api_loss_test/tls_api_test_with_loss so server_losses actually runs with mask 6.
* Phase 5B analysis: Rust server_losses now exercises the C server-loss mask path instead of running with zero loss.
* Phase 5B fix note: Threaded the loss mask through tls_api_loss_test into tls_api_test_with_loss, and initialized the handshake loss loop from that mask.

### C test body
```c
{
    return tls_api_loss_test(6ull);
}
```

### Current Rust test body
```rust
fn server_losses() {
    tls_api_loss_test(6).expect("server_losses");
}
```

## `picoquictest/util_test.c:util_memcmp_test`
* C test-table name: `util_memcmp`
* C entry function: `util_memcmp_test`
* Rust test: `util_memcmp`
* Expected Rust file: `rs/fq/src/tests/util_test.rs`
* Current Rust span: `rs/fq/src/tests/util_test.rs:101-160`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust preserves the 16-byte equality/difference loops, but it shrinks the working set and omits C's full-buffer comparisons with a single differing byte at 16 positions. The timing printouts can stay omitted, but the full-length mismatch checks are still asserted in C via carry.
* Phase 5A fix note: Keep timing assertions out, but add the C-style full-buffer comparisons over the intended working set with one flipped byte at each of the 16 offsets/intervals.
* Phase 5B analysis: Rust test now covers the C-sized 1 MB working set and the C full-buffer single-byte mismatch cases while still omitting timing-only checks.
* Phase 5B fix note: Updated util_memcmp to use test_random, restored 1 MB buffer size, and added 16 full-length constant_time_memcmp mismatch assertions at the C interval positions.

### C test body
```c
{
    int ret = 0;
    size_t nb8 = (1 << 20) / sizeof(uint64_t);
    size_t l_total = nb8 * sizeof(uint64_t);
    uint64_t* x8 = (uint64_t*)malloc(l_total);
    uint8_t* x = (uint8_t*)x8;
    uint8_t* y = (uint8_t*)malloc(l_total);
    uint64_t random_seed = 0xbabac001;
    uint64_t time_start;
    uint64_t const_compare_time[16];
#if 0
    uint64_t memcmp_time[2];
#endif
    uint64_t carry;
    uint64_t nb_round = 2;

    if (x == NULL || y == NULL) {
        ret = -1;
    }
    else {
        x8[0] = 0;
        x8[1] = 0;
        x8[2] = 0xffffffffffffffffull;
        x8[3] = 0xffffffffffffffffull;

        for (size_t i = 4; i < nb8; i++) {
            x8[i] = picoquic_test_random(&random_seed);
        }
        /* test for correct detection of equality */
        memcpy(y, x, l_total);
        for (size_t j = 0; j < l_total; j += 16) {
            if (picoquic_constant_time_memcmp(x + j, y + j, 16) != 0) {
                DBG_PRINTF("Unexpected mismatch, rank %d\n", (int)j);
                ret = -1;
                break;
            }
        }

        for (size_t i = 0; ret == 0 && i < 16; i++) {
            /* prepare the y string */
            memcpy(y, x, l_total);
            for (size_t j = i; j < l_total; j += 16) {
                y[j] ^= (uint8_t)0xff;
            }

            /* test for correct detection of differences */
            for (size_t j = 0; j < l_total; j += 16) {
                if (picoquic_constant_time_memcmp(x + j, y + j, 16) == 0) {
                    DBG_PRINTF("Unexpected match, step %d, rank %d\n", (int)i, (int)j);
                    ret = -1;
                    break;
                }
            }
        }


        /* Time measurement: Compare a long string, at 16 different intervals. */
        while (ret == 0) {
            int zero_found = 0;
            for (size_t i = 0; ret == 0 && i < 16; i++) {
                /* prepare the y string */
                memcpy(y, x, l_total);
                y[1 + ((i * l_total) / 16)] ^= 0xff;

                carry = 1;
                time_start = picoquic_current_time();
                for (uint64_t j = 0; j < nb_round; j++) {
                    x[j] ^= 1;
                    y[j] ^= 1;
                    carry &= (picoquic_constant_time_memcmp(x, y, l_total) != 0);
                }
                const_compare_time[i] = picoquic_current_time() - time_start;
                if (carry != 1) {
                    DBG_PRINTF("Unexpected match, step %d\n", (int)i);
                    ret = -1;
                    break;
                }
                if (i == 0 && const_compare_time[i] < 2000) {
                    zero_found = 1;
                    break;
                }
            }

            if (zero_found) {
                nb_round *= 2;
            }
            else {
                break;
            }
        }
    }

    if (ret == 0) {
        DBG_PRINTF("%s", "Delta at, const memcmp (ns)\n");
        for (size_t i = 0; ret == 0 && i < 16; i++) {
            double d = 1000.0*(double)(const_compare_time[i]) / (double)(nb_round * l_total / 16);
            DBG_PRINTF("%d, %f\n", 1 + ((i * l_total) / 16), d);
        }
    }

    for (size_t i = 0; ret == 0 && i < 16; i++) {
        /* The time tests are information only, because measuring time is to susceptible to random noise */
        if (i > 0 && const_compare_time[0] >= 1000 && const_compare_time[i] >= 1000 && ((const_compare_time[i] > 2 * const_compare_time[0]) || (const_compare_time[0] > 2 * const_compare_time[i]))) {
            DBG_PRINTF("Step %d, const cmp time different from step 0, %d vs %d\n", (int)i, (int)const_compare_time[i], (int)const_compare_time[0]);
        }
    }

    #if 0
    while (ret == 0) {
        for (size_t i = 0; ret == 0 && i < 2; i++) {
            /* prepare the y string */
            memcpy(y, x, l_total);

            for (size_t j = 15*i; j < l_total; j += 16) {
                y[j] ^= (uint8_t)0xff;
            }


            carry = 1;
            time_start = picoquic_current_time();

            for (int r = 0; r < nb_round; r++) {
                /* measure compare time */
                for (size_t j = 0; j < l_total; j += 16) {
                    carry &= (memcmp(x, y, 16) != 0);
                }

                if (carry != 1) {
                    DBG_PRINTF("Unexpected memcmp match, step %d\n", (int)i);
                    ret = -1;
                    break;
                }
            }

            memcmp_time[i] = picoquic_current_time() - time_start;
        }

        if (memcmp_time[0] > 2000 && memcmp_time[1] > 2000) {
            break;
        }
        nb_round *= 2;
    } 

    if (ret == 0){
        if (memcmp_time[1] > 2 * memcmp_time[0]) {
            DBG_PRINTF("Memcmp not constant time on 16 bytes: t[0] = %d, t[15] = %d\n", (int)memcmp_time[0], (int)memcmp_time[1]);
            DBG_PRINTF("%s", "Need to compile with -DPICOQUIC_USE_CONSTANT_TIME_MEMCMP");
            ret = -1;
        }
        else {
            DBG_PRINTF("Memcmp constant time on 16 bytes: t[0] = %d, t[15] = %d\n", (int)memcmp_time[0], (int)memcmp_time[1]);
        }
    }
#endif

    if (x != NULL) {
        free(x);
    }

    if (y != NULL) {
        free(y);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn util_memcmp() {
    use core::cmp::Ordering;

    use super::util::test_random;
    use crate::utils::constant_time_memcmp;

    let nb_words = (1 << 20) / 8;
    let l_total = nb_words * 8;
    let mut x = vec![0u8; l_total];
    let mut seed = 0xbabac001u64;
    for chunk in x[32..].chunks_mut(8) {
        chunk.copy_from_slice(&test_random(&mut seed).to_le_bytes());
    }
    x[16..32].copy_from_slice(&[0xff; 16]);

    let y = x.clone();

    // Equality detection.
    let mut j = 0;
    while j < l_total {
        assert_eq!(
            constant_time_memcmp(&x[j..j + 16], &y[j..j + 16]),
            Ordering::Equal,
            "unexpected mismatch at byte {j}",
        );
        j += 16;
    }

    // Difference detection: flip one byte per 16-byte group.
    for offset in 0..16 {
        let mut y = x.clone();
        let mut j = offset;
        while j < l_total {
            y[j] ^= 0xff;
            j += 16;
        }
        let mut j = 0;
        while j < l_total {
            assert_ne!(
                constant_time_memcmp(&x[j..j + 16], &y[j..j + 16]),
                Ordering::Equal,
                "unexpected match at offset={offset}, byte={j}",
            );
            j += 16;
        }
    }

    // Full-buffer difference detection: one changed byte at each of
    // the 16 interval positions used by the C timing loop.
    for offset in 0..16 {
        let mut y = x.clone();
        let changed = 1 + ((offset * l_total) / 16);
        y[changed] ^= 0xff;
        assert_ne!(
            constant_time_memcmp(&x, &y),
            Ordering::Equal,
            "unexpected full-buffer match at offset={offset}, byte={changed}",
        );
    }
}
```

## `picoquictest/congestion_test.c:bbr_jitter_test`
* C test-table name: `bbr_jitter`
* C entry function: `bbr_jitter_test`
* Rust test: `bbr_jitter`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:788-791`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Entry parameters match, but the shared Rust scenario verifier ignores completion and max-time checks, so the 3,600,000us deadline from the C test is not enforced.
* Phase 5A fix note: Implement faithful `tls_api_one_scenario_body_verify` checks for scenario completion and max completion time, or add equivalent assertions in the Rust congestion helper.
* Phase 5B analysis: Reclassified ok: Rust #[test] is present, compiles under the test harness, selects bbr, and calls congestion_control_test with the same API-level inputs as C. Any Error::Generic/runtime scenario failure is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    return congestion_control_test(picoquic_bbr_algorithm, 3600000, 5000, 5);
}
```

### Current Rust test body
```rust
fn bbr_jitter() {
    let ccalgo = cc_algo("bbr");
    congestion_control_test(ccalgo, 3_600_000, 5_000, 5);
}
```
