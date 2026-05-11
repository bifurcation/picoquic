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

Owned Rust test file(s): `rs/fq/src/tests/datagram.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/datagram_tests.c:datagram_test`
* C test-table name: `datagram`
* C entry function: `datagram_test`
* Rust test: `datagram`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Rust span: `rs/fq/src/tests/datagram.rs:642-649`
* Phase 5A analysis: The Rust wrapper fields match, but the Rust helper forces negotiated datagram transport parameters to expected values and manually delivers/acks datagrams outside the QUIC DATAGRAM provider/callback path, so it does not check the same behavior as C.
* Phase 5A fix note: Wire the Rust datagram helper through actual QUIC datagram send/receive/ack callbacks, assert negotiated max_datagram_frame_size values instead of overwriting them, and remove direct synthetic delivery.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust wrapper matches the C initializer, but the shared Rust helper still manually sends, receives, and ACKs datagrams outside the QUIC DATAGRAM provider/callback path that the C test exercises.
* Phase 5C fix note: Use actual Rust datagram provider/receive/ack callback surfaces and remove synthetic datagram_app_round delivery while keeping negotiated max_datagram_frame_size assertions.

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 5;
    dg_ctx.dg_target[1] = 5;

    return datagram_test_one(1, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [5, 5],
        ..Default::default()
    };
    datagram_test_one(1, &mut dg_ctx, 0);
}
```

## `picoquictest/datagram_tests.c:datagram_loss_test`
* C test-table name: `datagram_loss`
* C entry function: `datagram_loss_test`
* Rust test: `datagram_loss`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Rust span: `rs/fq/src/tests/datagram.rs:698-707`
* Phase 5A analysis: The wrapper uses the same parameters, but the Rust helper manually simulates datagram sends, losses, receives, and ACK/loss callbacks instead of driving QUIC datagram frames through the simulator as the C helper does.
* Phase 5A fix note: Phase 5B should make datagram_test_one exercise the Rust QUIC datagram transport/callback path under the supplied loss mask, then assert the same ACK/NACK/spurious and delivery accounting as C.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust datagram_loss has the right wrapper parameters, but datagram_test_one still fabricates datagram send/receive/loss/ACK accounting in datagram_app_round instead of driving QUIC DATAGRAM frames and callback events through the simulator like C.
* Phase 5C fix note: Rewrite the Rust datagram helper to use the Rust datagram readiness/prepare/receive/ack callback path over tls_api_one_sim_round under the supplied loss mask, then assert the same ACK/NACK/spurious and delivery accounting.

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 20000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;

    return datagram_test_one(4, &dg_ctx, 0x040080100200400ull);
}
```

### Current Rust test body
```rust
fn datagram_loss() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        ..Default::default()
    };
    datagram_test_one(4, &mut dg_ctx, 0x040080100200400);
}
```

## `picoquictest/datagram_tests.c:datagram_rt_skip_test`
* C test-table name: `datagram_rt_skip`
* C entry function: `datagram_rt_skip_test`
* Rust test: `datagram_rt_skip`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Rust span: `rs/fq/src/tests/datagram.rs:667-678`
* Phase 5A analysis: The Rust wrapper copies the C context values, but the shared Rust datagram helper does not check the same behavior: it manually simulates datagram send/receive/ack instead of exercising the callback/provider datagram path, and it patches negotiated datagram sizes rather than failing like the C helper.
* Phase 5A fix note: Update rs/fq/src/tests/datagram.rs shared datagram_test_one path to exercise the actual datagram callback/provider or queued datagram machinery and preserve the C negotiation, latency, ack/nack/spurious, and skip checks without short-circuiting delivery.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Wrapper fields match C, but the merged helper still bypasses the real DATAGRAM callback/provider path: datagram_app_round manually invokes send/recv/ack counters instead of wiring the C-equivalent datagram send, receive, and ack callbacks into the QUIC simulation.
* Phase 5C fix note: Replace the manual datagram_app_round delivery with the actual Rust callback/provider or queued-datagram harness surface while preserving the C skip, negotiation, latency, ack/nack, and completion checks.

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 10;
    dg_ctx.send_delay = 20000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;
    dg_ctx.dg_latency_target[0] = 13000;
    dg_ctx.dg_latency_target[1] = 20000;
    dg_ctx.do_skip_test[0] = 1;
    dg_ctx.do_skip_test[1] = 1;

    return datagram_test_one(3, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_rt_skip() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 10],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [13_000, 20_000],
        do_skip_test: [true, true],
        ..Default::default()
    };
    datagram_test_one(3, &mut dg_ctx, 0);
}
```

## `picoquictest/datagram_tests.c:datagram_small_test`
* C test-table name: `datagram_small`
* C entry function: `datagram_small_test`
* Rust test: `datagram_small`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Rust span: `rs/fq/src/tests/datagram.rs:723-735`
* Phase 5A analysis: The Rust wrapper copies the C datagram_small field values, but the helper does not check the same behavior: it normalizes negotiated datagram parameters and synthesizes datagram delivery/packet counting instead of exercising the live callback-driven QUIC datagram path and cnx_client->nb_packets_received assertion.
* Phase 5A fix note: Make datagram_test_one assert negotiated max_datagram_frame_size, drive real datagram send/recv/ack callbacks through the simulator, and check the real client packet count against max_packets_received.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust wrapper fields match C, but datagram_test_one still synthesizes datagram send/receive/ack delivery and packet counts instead of using the live DATAGRAM callback/provider path and actual client nb_packets_received.
* Phase 5C fix note: Drive real DATAGRAM callbacks through the simulator and assert max_packets_received against the real client packet counter.

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_small_size = 64;
    dg_ctx.batch_size[0] = 4;
    dg_ctx.batch_size[1] = 4;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 5000;
    dg_ctx.next_gen_time[0] = 50000;
    dg_ctx.next_gen_time[1] = 50000;
    dg_ctx.max_packets_received = 55;

    return datagram_test_one(6, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_small() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        ..Default::default()
    };
    datagram_test_one(6, &mut dg_ctx, 0);
}
```

## `picoquictest/datagram_tests.c:datagram_small_new_test`
* C test-table name: `datagram_small_new`
* C entry function: `datagram_small_new_test`
* Rust test: `datagram_small_new`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Rust span: `rs/fq/src/tests/datagram.rs:739-752`
* Phase 5A analysis: The entry field values match, but the Rust shared datagram helper weakens the C test by normalizing negotiated datagram parameters and synthesizing datagram delivery/packet counts instead of verifying the real negotiated datagram path and connection packet count as the C helper does.
* Phase 5A fix note: Make the Rust datagram helper fail on bad max_datagram_frame_size negotiation and exercise the real datagram send/receive/ack/provider path, including the client packet-count assertion for max_packets_received=55, rather than patching parameters or using synthetic delivery counters.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust entry fields match C, but the helper still synthesizes datagram send/receive/ack behavior and packet counts instead of exercising the real datagram callback/provider path and actual client packet count.
* Phase 5C fix note: Use the real datagram callback/provider/ack harness and check actual cnx_client packet count for max_packets_received=55; remove synthetic delivery counters.

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_small_size = 64;
    dg_ctx.batch_size[0] = 4;
    dg_ctx.batch_size[1] = 4;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 5000;
    dg_ctx.next_gen_time[0] = 50000;
    dg_ctx.next_gen_time[1] = 50000;
    dg_ctx.max_packets_received = 55;
    dg_ctx.use_extended_provider_api = 1;

    return datagram_test_one(7, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_small_new() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        use_extended_provider_api: true,
        ..Default::default()
    };
    datagram_test_one(7, &mut dg_ctx, 0);
}
```
