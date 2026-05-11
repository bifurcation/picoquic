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

## `picoquictest/tls_api_test.c:tls_api_very_long_congestion_test`
* C test-table name: `tls_api_very_long_congestion`
* C entry function: `tls_api_very_long_congestion_test`
* Rust test: `tls_api_very_long_congestion`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:8114-8141`
* Phase 5A analysis: Rust omits test_scenario_very_long, max_data=128000, queue_delay_max=20000 behavior, and effective completion verification; the 20000 argument lands in an ignored Rust parameter.
* Phase 5A fix note: Run the 1 MB very-long scenario with max_data 128000, queue delay 20000 us, and a real 1000000 us completion assertion.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust body matches the C very-long congestion case: no loss, queue_delay_max 20000, max_data 128000 on both endpoints, very-long 1MB scenario, and 1000000us completion verification. But TEST_SCENARIO_VERY_LONG is defined multiple times at top level in tls_api.rs, so this test is not currently exposed as a runnable Rust test.
* Phase 5C fix note: De-duplicate the top-level TEST_SCENARIO_Q_AND_R and TEST_SCENARIO_VERY_LONG definitions in rs/fq/src/tests/tls_api.rs; no very_long_congestion body mismatch found.

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 128000, 20000, 0, 1000000, NULL, NULL);
}
```

### Current Rust test body
```rust
fn tls_api_q2_and_r2_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_Q2_AND_R2,
        0,
        0,
        0,
        0,
        86_000,
    )
    .expect("q2_and_r2_stream");
}

/// C: `tls_api_q_and_r_stream_test` in `picoquictest/tls_api_test.c`.
///
/// Q-and-R scenario: one send + one receive stream; target 75 ms.
#[test]
fn tls_api_q_and_r_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, TEST_SCENARIO_Q_AND_R, 0, 0, 0, 0, 75_000)
        .expect("q_and_r_stream");
}

/// C: `tls_api_sni_test` in `picoquictest/tls_api_test.c`.
```

## `picoquictest/tls_api_test.c:tls_api_very_long_stream_test`
* C test-table name: `tls_api_very_long_stream`
* C entry function: `tls_api_very_long_stream_test`
* Rust test: `tls_api_very_long_stream`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:8168-8182`
* Phase 5A analysis: C runs test_scenario_very_long with stream 4 query length 257 and response length 1000000, then verifies scenario completion and the 1000000 us target. Rust passes an empty scenario, so it does not exercise the very-long stream transfer.
* Phase 5A fix note: Use the Rust equivalent of test_scenario_very_long and ensure the helper verifies stream byte counts/completion and the max completion threshold.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust test uses the correct very-long scenario and 1,000,000us target, but TEST_SCENARIO_VERY_LONG is duplicated in tls_api.rs, blocking clean test exposure.
* Phase 5C fix note: Deduplicate the repeated scenario constants/helper definitions in rs/fq/src/tests/tls_api.rs; no scenario-intent mismatch found.

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 0, 0, 0, 1000000, NULL, NULL);
}
```

### Current Rust test body
```rust
            .expect("server connection not accepted");
        assert_tls_api_final_negotiation(client, server, Some(TEST_SNI), Some(TEST_ALPN));
    }
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("tls_api_sni close");
}

/// C: `tls_api_very_long_congestion_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream scenario with `queue_delay_max=20000 µs` to stress
/// the congestion window estimator; target 1 s.
#[test]
fn tls_api_very_long_congestion() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    let mut loss_mask = 0u64;
```

## `picoquictest/tls_api_test.c:tls_api_very_long_with_err_test`
* C test-table name: `tls_api_very_long_with_err`
* C entry function: `tls_api_very_long_with_err_test`
* Rust test: `tls_api_very_long_with_err`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:8189-8217`
* Phase 5A analysis: Rust omits the very-long scenario and max_data=128000. It passes an empty scenario and the helper signature currently cannot express the C max_data parameter, so it does not exercise the 1 MB transfer under loss.
* Phase 5A fix note: Run scenario [{stream_id:4, previous_stream_id:0, q_len:257, r_len:1000000}] with loss_mask=0x30000, max_data=128000, queue_delay=0, and completion bound 2210000; extend/use a helper that preserves max_data.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test body is faithful to the C very-long/loss/max_data scenario, but merged tls_api.rs has duplicate top-level scenario constants, so the test file is not cleanly exposed as runnable.
* Phase 5C fix note: Remove/merge duplicate top-level TEST_SCENARIO_Q_AND_R and TEST_SCENARIO_VERY_LONG constants in rs/fq/src/tests/tls_api.rs; keep the current faithful test body.

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0x30000, 128000, 0, 0, 2210000, NULL, NULL);
}
```

### Current Rust test body
```rust
        "very_long_congestion: server connection was not accepted"
    );
    {
        let client = ctx.cnx_client();
        client.maxdata_local = 128_000;
        client.maxdata_remote = 128_000;
    }
    {
        let server = ctx.cnx_server();
        server.maxdata_local = 128_000;
        server.maxdata_remote = 128_000;
    }
    test_api_init_send_recv_scenario(&mut ctx, TEST_SCENARIO_VERY_LONG)
        .expect("very_long_congestion scenario");
    tls_api_data_sending_loop(&mut ctx, &mut loss_mask, &mut t, 0)
        .expect("very_long_congestion data");
    tls_api_one_scenario_body_verify(&mut ctx, &mut t, 1_000_000).expect("very_long_congestion");
}

/// C: `tls_api_very_long_max_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream with `max_data=128000`; target 1 s.
#[test]
fn tls_api_very_long_max() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body_ex(
        &mut ctx,
        &mut t,
```

## `picoquictest/tls_api_test.c:virtual_time_test`
* C test-table name: `virtual_time`
* C entry function: `virtual_time_test`
* Rust test: `virtual_time`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:8788-8858`
* Phase 5A analysis: Rust test is only the generic TLS handshake/close smoke path. It does not create simulated vs direct QUIC contexts or assert quic_time/tls_time behavior, so it misses the C test's core intent.
* Phase 5A fix note: Replace with a dedicated virtual-time test: create a simulated-time context and assert repeated simulated_time increments are reflected by Quic::time/tls_time; create a direct wall-clock context and assert Quic::time/tls_time track current_time deltas within the C tolerance. This may also expose missing simulated-time support because Quic::new currently documents the simulated-time pointer as dropped and Quic::time returns current_time().
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test faithfully checks simulated and direct QUIC/TLS time through the intended APIs, but the same merged tls_api.rs duplicate top-level constants prevent clean harness compilation/exposure.
* Phase 5C fix note: Remove/merge duplicate top-level scenario constants in rs/fq/src/tests/tls_api.rs; keep the current virtual-time assertions.

### C test body
```c
{
    int ret = 0;
    uint64_t test_time = 0;
    uint64_t simulated_time = 0;
    uint64_t current_time = picoquic_current_time();
    uint64_t ptls_time = 0;
    uint8_t callback_ctx[256];
    char test_server_cert_store_file[512];
    picoquic_quic_t * qsimul = NULL;
    picoquic_quic_t * qdirect = NULL;

    ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }
    else {
        qsimul = picoquic_create(8, NULL, NULL, test_server_cert_store_file,
            NULL, test_api_callback,
            (void*)callback_ctx, NULL, NULL, NULL, simulated_time,
            &simulated_time, ticket_file_name, NULL, 0);
        qdirect = picoquic_create(8, NULL, NULL, PICOQUIC_TEST_FILE_CERT_STORE,
            NULL, test_api_callback,
            (void*)callback_ctx, NULL, NULL, NULL, current_time,
            NULL, ticket_file_name, NULL, 0);

        if (qsimul == NULL || qdirect == NULL)
        {
            ret = -1;
        }
        else
        {
            /* Check that the simulated time follows the simulation */
            for (int i = 0; ret == 0 && i < 5; i++) {
                simulated_time += 12345678000;
                test_time = picoquic_get_quic_time(qsimul);
                ptls_time = picoquic_get_tls_time(qsimul);
                if (test_time != simulated_time) {
                    DBG_PRINTF("Test time: %llu != Simulated: %llu",
                        (unsigned long long)test_time,
                        (unsigned long long)simulated_time);
                    ret = -1;
                }
                else if (ptls_time < test_time || ptls_time > test_time + 1000) {
                    DBG_PRINTF("Test time: %llu does match ptls time: %llu",
                        (unsigned long long)test_time,
                        (unsigned long long)ptls_time);
                    ret = -1;
                }
            }
        }

        if (ret == 0) {
            int64_t delta, delta_low, delta_high;
            uint64_t current_previous = picoquic_current_time();
            uint64_t test_previous = picoquic_current_time();
            uint64_t ptls_previous = picoquic_get_tls_time(qdirect);

            /* Check that the non simulated time follows the current time */
            for (int i = 0; ret == 0 && i < 5; i++) {
#ifdef _WINDOWS
                Sleep(1);
#else
                usleep(1000);
#endif
                current_time = picoquic_current_time();
                test_time = picoquic_get_quic_time(qdirect);
                ptls_time = picoquic_get_tls_time(qdirect);

                delta = current_time - current_previous;
                delta_low = delta - 1000;
                delta_high = delta + 1000;
                if (test_time < test_previous + delta_low || test_time > test_previous + delta_high ) {
                    DBG_PRINTF("Test time: %" PRIu64 " does not match previous test time : %" PRIu64 " + delta : %" PRId64,
                        test_time, test_previous, delta);
                    ret = -1;
                }
                else if (ptls_time < ptls_previous + delta_low || ptls_time > ptls_previous + delta_high) {
                    DBG_PRINTF("Test time: %" PRIu64 " does not match previous test time : %" PRIu64 " + delta : %" PRId64,
                        ptls_time, ptls_previous, delta);
                    ret = -1;
                }
            }
        }
    }

    if (qsimul != NULL)
    {
        picoquic_free(qsimul);
        qsimul = NULL;
    }

    if (qdirect != NULL)
    {
        picoquic_free(qdirect);
        qdirect = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust

        if spoof_mode == 7 {
            append_byte(packet, &mut packet_index, 0xaf)?;
        }
    }

    Ok(packet_index)
}

fn version_negotiation_client_local_cid(cnx: &Connection) -> crate::Result<ConnectionId> {
    let token = cnx
        .paths
        .first()
        .and_then(|path| path.tuples.first())
        .and_then(|tuple| tuple.local_connection_id)
        .ok_or(crate::Error::Generic)?;

    cnx.local_connection_ids
        .get(token)
        .map(|lcid| lcid.connection_id)
        .ok_or(crate::Error::Generic)
}

fn append_cid(packet: &mut [u8], packet_index: &mut usize, cid: ConnectionId) -> crate::Result<()> {
    append_byte(packet, packet_index, cid.len() as u8)?;
    let cid_end = packet_index
        .checked_add(cid.len())
        .ok_or(crate::Error::BufferTooSmall)?;
    if cid_end > packet.len() {
        return Err(crate::Error::BufferTooSmall);
    }
    packet[*packet_index..cid_end].copy_from_slice(cid.as_bytes());
    *packet_index = cid_end;
    Ok(())
}

fn append_u32(packet: &mut [u8], packet_index: &mut usize, value: u32) -> crate::Result<()> {
    let value_end = packet_index
        .checked_add(core::mem::size_of::<u32>())
        .ok_or(crate::Error::BufferTooSmall)?;
    if value_end > packet.len() {
        return Err(crate::Error::BufferTooSmall);
    }
    packet[*packet_index..value_end].copy_from_slice(&value.to_be_bytes());
    *packet_index = value_end;
    Ok(())
}

fn append_byte(packet: &mut [u8], packet_index: &mut usize, value: u8) -> crate::Result<()> {
    let value_end = packet_index
        .checked_add(1)
        .ok_or(crate::Error::BufferTooSmall)?;
    if value_end > packet.len() {
        return Err(crate::Error::BufferTooSmall);
    }
    packet[*packet_index] = value;
    *packet_index = value_end;
    Ok(())
}

/// C: `virtual_time_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that simulated time and wall-clock time are tracked separately
/// and that the library never reads the system clock internally.
#[test]
fn virtual_time() {
    const SIMULATED_STEP: u64 = 12_345_678_000;
    const TLS_TIME_TOLERANCE: u64 = 1000;

    let mut simulated_time = 0u64;
    let mut simulated_config = crate::config::Config {
```

## `picoquictest/tls_api_test.c:vn_compat_test`
* C test-table name: `vn_compat`
* C entry function: `vn_compat_test`
* Rust test: `vn_compat`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:8882-8890`
* Phase 5A analysis: Rust only runs a generic InternalTest1 handshake. C runs three compatibility cases: V1 to V2 must negotiate V2, V1 to V2 draft must negotiate V2 draft, and V1 to InternalTest1 must fail.
* Phase 5A fix note: Implement vn_compat_test_one-style Rust coverage: create a V1 connection, set desired_version to V2 and V2Draft and assert both endpoints negotiate the target, then assert desired InternalTest1 is rejected.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust covers the three C cases and checks endpoint versions, but vn_compat_test_one uses tls_api_init_ctx, which starts the client before set_desired_version; C sets desired_version before picoquic_start_client_cnx.
* Phase 5C fix note: Use a delayed/non-starting context, set desired_version, then start_client before the connection loop.

### C test body
```c
{
    int ret = 0;

    if (vn_compat_test_one(PICOQUIC_V1_VERSION, PICOQUIC_V2_VERSION) != 0) {
        ret = -1;
    }
    else if (vn_compat_test_one(PICOQUIC_V1_VERSION, PICOQUIC_V2_VERSION_DRAFT) != 0) {
        ret = -1;
    }
    else if (vn_compat_test_one(PICOQUIC_V1_VERSION, PICOQUIC_INTERNAL_TEST_VERSION_1) == 0) {
        ret = -1;
    }

    return ret;
}
```

### Current Rust test body
```rust
        );
    }

    let direct_start = crate::current_time();
    let qdirect = Quic::new(
        8,
        None,
        None,
        Some(TEST_FILE_CERT_STORE),
```
