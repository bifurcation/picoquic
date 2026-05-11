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

## `picoquictest/congestion_test.c:bdp_short_hi_test`
* C test-table name: `bdp_short_hi`
* C entry function: `bdp_short_hi_test`
* Rust test: `bdp_short_hi`
* C source: `picoquictest/congestion_test.c:707-710`
* Rust source: `rs/fq/src/tests/congestion.rs:930-932`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_short_hi);
}
```

### Rust test body
```rust
fn bdp_short_hi() {
    bdp_option_test_one(BdpTestOption::ShortHi);
}
```

## `picoquictest/cpu_limited.c:limited_reno_test`
* C test-table name: `limited_reno`
* C entry function: `limited_reno_test`
* Rust test: `limited_reno`
* C source: `picoquictest/cpu_limited.c:210-218`
* Rust source: `rs/fq/src/tests/cpu_limited.rs:160-165`

### C test body
```c
{
    limited_test_config_t config;
    limited_config_set_default(&config, 1);
    config.ccalgo = picoquic_newreno_algorithm;
    config.max_completion_time = 4600000;

    return limited_client_test_one(&config);
}
```

### Rust test body
```rust
fn limited_reno() {
    let mut config = limited_config_default(1);
    config.ccalgo = get_congestion_algorithm("newreno").expect("newreno algo");
    config.max_completion_time = 4_600_000;
    limited_client_test_one(config);
}
```

## `picoquictest/datagram_tests.c:datagram_too_long_test`
* C test-table name: `datagram_too_long_test`
* C entry function: `datagram_too_long_test`
* Rust test: `datagram_too_long_test`
* C source: `picoquictest/datagram_tests.c:734-743`
* Rust source: `rs/fq/src/tests/datagram.rs:791-799`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 5;
    dg_ctx.dg_target[1] = 5;
    dg_ctx.test_too_long = 1;

    return datagram_test_one(10, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_too_long_test() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [5, 5],
        test_too_long: true,
        ..Default::default()
    };
    datagram_test_one(10, &mut dg_ctx, 0);
}
```

## `picoquictest/edge_cases.c:crypto_hs_offset_test`
* C test-table name: `crypto_hs_offset`
* C entry function: `crypto_hs_offset_test`
* Rust test: `crypto_hs_offset`
* C source: `picoquictest/edge_cases.c:1632-1644`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1070-1078`

### C test body
```c
{
    picoquic_packet_context_enum pc[] = { picoquic_packet_context_initial,
        picoquic_packet_context_handshake, picoquic_packet_context_application };
    size_t nb_pc = sizeof(pc) / sizeof(picoquic_packet_context_enum);
    int ret = 0;

    for (size_t i = 0; i < nb_pc && ret == 0; i++) {
        ret = crypto_hs_offset_test_one(pc[i]);
    }

    return ret;
}
```

### Rust test body
```rust
fn crypto_hs_offset() {
    for pc in [
        PacketContext::Initial,
        PacketContext::Handshake,
        PacketContext::Application,
    ] {
        crypto_hs_offset_one(pc).expect("crypto_hs_offset_one");
    }
}
```

## `picoquictest/edge_cases.c:error_name_test`
* C test-table name: `error_name`
* C entry function: `error_name_test`
* Rust test: `error_name`
* C source: `picoquictest/edge_cases.c:2035-2165`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1531-1647`

### C test body
```c
{
    int ret = 0;
    int failures = 0;

    // Table of error codes and expected names
    static const error_name_case_t cases[] = {
        // Protocol errors (all switch cases)
        { PICOQUIC_TRANSPORT_INTERNAL_ERROR, "internal" },
        { PICOQUIC_TRANSPORT_SERVER_BUSY, "server busy" },
        { PICOQUIC_TRANSPORT_FLOW_CONTROL_ERROR, "flow control" },
        { PICOQUIC_TRANSPORT_STREAM_LIMIT_ERROR, "stream limit" },
        { PICOQUIC_TRANSPORT_STREAM_STATE_ERROR, "stream state" },
        { PICOQUIC_TRANSPORT_FINAL_OFFSET_ERROR, "final offset" },
        { PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR, "frame format" },
        { PICOQUIC_TRANSPORT_PARAMETER_ERROR, "parameter" },
        { PICOQUIC_TRANSPORT_CONNECTION_ID_LIMIT_ERROR, "connection_id limit" },
        { PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, "protocol violation" },
        { PICOQUIC_TRANSPORT_INVALID_TOKEN, "invalid token" },
        { PICOQUIC_TRANSPORT_APPLICATION_ERROR, "application" },
        { PICOQUIC_TRANSPORT_CRYPTO_BUFFER_EXCEEDED, "crypto buffer exceeded" },
        { PICOQUIC_TRANSPORT_KEY_UPDATE_ERROR, "key update" },
        { PICOQUIC_TRANSPORT_AEAD_LIMIT_REACHED, "aead limit" },
        { PICOQUIC_TLS_ALERT_WRONG_ALPN, "wrong alpn" },
        { PICOQUIC_TLS_HANDSHAKE_FAILED, "tls handshake failed" },
        { PICOQUIC_TRANSPORT_VERSION_NEGOTIATION_ERROR, "version negotiation" },
        { PICOQUIC_TRANSPORT_APPLICATION_ABANDON, "application abandon" },
        { PICOQUIC_TRANSPORT_RESOURCE_LIMIT_REACHED, "resource limit reached" },
        { PICOQUIC_TRANSPORT_UNSTABLE_INTERFACE, "unstable interface" },
        { PICOQUIC_TRANSPORT_NO_CID_AVAILABLE, "no CID available" },

        // Picoquic local error codes (all switch cases)
        { PICOQUIC_ERROR_DUPLICATE, "duplicate" },
        { PICOQUIC_ERROR_AEAD_CHECK, "payload_decrypt_error" },
        { PICOQUIC_ERROR_UNEXPECTED_PACKET, "unexpected packet" },
        { PICOQUIC_ERROR_MEMORY, "memory" },
        { PICOQUIC_ERROR_CNXID_CHECK, "connection ID check" },
        { PICOQUIC_ERROR_INITIAL_TOO_SHORT, "" },
        { PICOQUIC_ERROR_VERSION_NEGOTIATION_SPOOFED, "version negotation spoofed" },
        { PICOQUIC_ERROR_MALFORMED_TRANSPORT_EXTENSION, "malformed transport extension" },
        { PICOQUIC_ERROR_EXTENSION_BUFFER_TOO_SMALL, "extension buffer too small" },
        { PICOQUIC_ERROR_ILLEGAL_TRANSPORT_EXTENSION, "illegal transport extension" },
        { PICOQUIC_ERROR_CANNOT_RESET_STREAM_ZERO, "cannot reset the crypto stream" },
        { PICOQUIC_ERROR_INVALID_STREAM_ID, "invalid stream id" },
        { PICOQUIC_ERROR_STREAM_ALREADY_CLOSED, "stream already closed" },
        { PICOQUIC_ERROR_FRAME_BUFFER_TOO_SMALL, "frame buffer too small" },
        { PICOQUIC_ERROR_INVALID_FRAME, "invalid frame" },
        { PICOQUIC_ERROR_CANNOT_CONTROL_STREAM_ZERO, "cannot control the crypto stream" },
        { PICOQUIC_ERROR_RETRY, "retry" },
        { PICOQUIC_ERROR_DISCONNECTED, "disconnected" },
        { PICOQUIC_ERROR_DETECTED, "error detected" },
        { PICOQUIC_ERROR_INVALID_TICKET, "invalid ticket" },
        { PICOQUIC_ERROR_INVALID_FILE, "invalid file" },
        { PICOQUIC_ERROR_SEND_BUFFER_TOO_SMALL, "send buffer too small" },
        { PICOQUIC_ERROR_UNEXPECTED_STATE, "unexpected state" },
        { PICOQUIC_ERROR_UNEXPECTED_ERROR, "unexpected error" },
        { PICOQUIC_ERROR_TLS_SERVER_CON_WITHOUT_CERT, "server configuration without cert" },
        { PICOQUIC_ERROR_NO_SUCH_FILE, "no such file" },
        { PICOQUIC_ERROR_STATELESS_RESET, "stateless reset" },
        { PICOQUIC_ERROR_CONNECTION_DELETED, "connection deleted" },
        { PICOQUIC_ERROR_CNXID_SEGMENT, "connection ID segment error" },
        { PICOQUIC_ERROR_CNXID_NOT_AVAILABLE, "connection ID not available" },
        { PICOQUIC_ERROR_MIGRATION_DISABLED, "migration disabled" },
        { PICOQUIC_ERROR_CANNOT_COMPUTE_KEY, "cannot compute key" },
        { PICOQUIC_ERROR_CANNOT_SET_ACTIVE_STREAM, "cannot set active stream" },
        { PICOQUIC_ERROR_CANNOT_CHANGE_ACTIVE_CONTEXT, "cannot change active context" },
        { PICOQUIC_ERROR_INVALID_TOKEN, "invalid token" },
        { PICOQUIC_ERROR_INITIAL_CID_TOO_SHORT, "initial CID too short" },
        { PICOQUIC_ERROR_KEY_ROTATION_NOT_READY, "key rotation not ready" },
        { PICOQUIC_ERROR_AEAD_NOT_READY, "aead not ready" },
        { PICOQUIC_ERROR_NO_ALPN_PROVIDED, "no ALPN provided" },
        { PICOQUIC_ERROR_NO_CALLBACK_PROVIDED, "no callback provided" },
        { PICOQUIC_STREAM_RECEIVE_COMPLETE, "stream receive complete" },
        { PICOQUIC_ERROR_PACKET_HEADER_PARSING, "packet header parsing" },
        { PICOQUIC_ERROR_QUIC_BIT_MISSING, "QUIC bit missing" },
        { PICOQUIC_NO_ERROR_TERMINATE_PACKET_LOOP, "terminate packet loop (not an error)" },
        { PICOQUIC_NO_ERROR_SIMULATE_NAT, "simulate NAT (not an error)" },
        { PICOQUIC_NO_ERROR_SIMULATE_MIGRATION, "simulate migration (not an error)" },
        { PICOQUIC_ERROR_VERSION_NOT_SUPPORTED, "version not supported" },
        { PICOQUIC_ERROR_IDLE_TIMEOUT, "idle timeout" },
        { PICOQUIC_ERROR_REPEAT_TIMEOUT, "repeat timeout" },
        { PICOQUIC_ERROR_HANDSHAKE_TIMEOUT, "handshake timeout" },
        { PICOQUIC_ERROR_SOCKET_ERROR, "socket" },
        { PICOQUIC_ERROR_VERSION_NEGOTIATION, "version negotiation" },
        { PICOQUIC_ERROR_PACKET_TOO_LONG, "packet too long" },
        { PICOQUIC_ERROR_PACKET_WRONG_VERSION, "wrong version" },
        { PICOQUIC_ERROR_PORT_BLOCKED, "port blocked" },
        { PICOQUIC_ERROR_DATAGRAM_TOO_LONG, "datagram too long" },
        { PICOQUIC_ERROR_PATH_ID_INVALID, "invalid path ID" },
        { PICOQUIC_ERROR_RETRY_NEEDED, "retry needed" },
        { PICOQUIC_ERROR_SERVER_BUSY, "server busy" },
        { PICOQUIC_ERROR_PATH_DUPLICATE, "duplicate path" },
        { PICOQUIC_ERROR_PATH_ID_BLOCKED, "blocked by lack of path ID" },
        { PICOQUIC_ERROR_PATH_CID_BLOCKED, "blocked by lack of CID" },
        { PICOQUIC_ERROR_PATH_ADDRESS_FAMILY, "path address family" },
        { PICOQUIC_ERROR_PATH_NOT_READY, "path not ready" },
        { PICOQUIC_ERROR_PATH_LIMIT_EXCEEDED, "path limit exceeded" },
        { PICOQUIC_ERROR_REDIRECTED, "redirected to proxy (not an error)" },
        { PICOQUIC_ERROR_PADDING_PACKET, "padding_packet" },

        // Crypto error alert range (default branch)
        { 0x101, "crypto error alert" },
        { 0x150, "crypto error alert" },
        { 0x1FF, "crypto error alert" },

        // Unknown picoquic error range (default branch)
        { 0x450, "unknown picoquic error" },
        { 0x4FF, "unknown picoquic error" },

        // Unknown error (default branch)
        { 0x0, "unknown" },
        { 0x200, "unknown" },
        { 0x500, "unknown" },
        { 0xFFFFFFFFFFFFFFFFull, "unknown" }
    };

    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        const char* got = picoquic_error_name(cases[i].code);
        if (strcmp(got, cases[i].expected) != 0) {
            fprintf(stderr, "FAIL: error_code=0x%llx got=\"%s\" expected=\"%s\"\n",
                (unsigned long long)cases[i].code, got, cases[i].expected);
            ret = -1;
            failures++;
        }
    }

    if (failures == 0) {
        printf("All error_name_test cases passed.\n");
    }
    return ret;
}
```

### Rust test body
```rust
fn error_name() {
    use crate::errors::InternalError;

    let cases: &[(u64, &str)] = &[
        // Protocol (transport / TLS) errors.
        (0x1, "internal"),
        (0x2, "server busy"),
        (0x3, "flow control"),
        (0x4, "stream limit"),
        (0x5, "stream state"),
        (0x6, "final offset"),
        (0x7, "frame format"),
        (0x8, "parameter"),
        (0x9, "connection_id limit"),
        (0xA, "protocol violation"),
        (0xB, "invalid token"),
        (0xC, "application"),
        (0xD, "crypto buffer exceeded"),
        (0xE, "key update"),
        (0xF, "aead limit"),
        (0x178, "wrong alpn"),
        (0x201, "tls handshake failed"),
        (0x11, "version negotiation"),
        (0x3e, "application abandon"),
        (0x3e75, "resource limit reached"),
        (0x3e76, "unstable interface"),
        (0x3e77, "no CID available"),
        // Picoquic-internal codes (0x400+ range).
        (0x401, "duplicate"),
        (0x403, "payload_decrypt_error"),
        (0x404, "unexpected packet"),
        (0x405, "memory"),
        (0x407, "connection ID check"),
        (0x408, ""),
        (0x409, "version negotation spoofed"),
        (0x40A, "malformed transport extension"),
        (0x40B, "extension buffer too small"),
        (0x40C, "illegal transport extension"),
        (0x40D, "cannot reset the crypto stream"),
        (0x40E, "invalid stream id"),
        (0x40F, "stream already closed"),
        (0x410, "frame buffer too small"),
        (0x411, "invalid frame"),
        (0x412, "cannot control the crypto stream"),
        (0x413, "retry"),
        (0x414, "disconnected"),
        (0x415, "error detected"),
        (0x417, "invalid ticket"),
        (0x418, "invalid file"),
        (0x419, "send buffer too small"),
        (0x41A, "unexpected state"),
        (0x41B, "unexpected error"),
        (0x41C, "server configuration without cert"),
        (0x41D, "no such file"),
        (0x41E, "stateless reset"),
        (0x41F, "connection deleted"),
        (0x420, "connection ID segment error"),
        (0x421, "connection ID not available"),
        (0x422, "migration disabled"),
        (0x423, "cannot compute key"),
        (0x424, "cannot set active stream"),
        (0x425, "cannot change active context"),
        (0x426, "invalid token"),
        (0x427, "initial CID too short"),
        (0x428, "key rotation not ready"),
        (0x429, "aead not ready"),
        (0x42A, "no ALPN provided"),
        (0x42B, "no callback provided"),
        (0x42C, "stream receive complete"),
        (0x42D, "packet header parsing"),
        (0x42E, "QUIC bit missing"),
        (0x42F, "terminate packet loop (not an error)"),
        (0x430, "simulate NAT (not an error)"),
        (0x431, "simulate migration (not an error)"),
        (0x432, "version not supported"),
        (0x433, "idle timeout"),
        (0x434, "repeat timeout"),
        (0x435, "handshake timeout"),
        (0x436, "socket"),
        (0x437, "version negotiation"),
        (0x438, "packet too long"),
        (0x439, "wrong version"),
        (0x43A, "port blocked"),
        (0x43B, "datagram too long"),
        (0x43C, "invalid path ID"),
        (0x43D, "retry needed"),
        (0x43E, "server busy"),
        (0x43F, "duplicate path"),
        (0x440, "blocked by lack of path ID"),
        (0x441, "blocked by lack of CID"),
        (0x442, "path address family"),
        (0x443, "path not ready"),
        (0x444, "path limit exceeded"),
        (0x445, "redirected to proxy (not an error)"),
        (0x446, "padding_packet"),
        // CRYPTO_ERROR alert range (default branch).
        (0x101, "crypto error alert"),
        (0x150, "crypto error alert"),
        (0x1FF, "crypto error alert"),
        // Unknown picoquic error range (default branch).
        (0x450, "unknown picoquic error"),
        (0x4FF, "unknown picoquic error"),
        // Truly unknown.
        (0x0, "unknown"),
        (0x200, "unknown"),
        (0x500, "unknown"),
        (0xFFFF_FFFF_FFFF_FFFF, "unknown"),
    ];

    for &(code, expected) in cases {
        let got = InternalError::name(code).unwrap_or("<none>");
        assert_eq!(
            got, expected,
            "error_code=0x{code:x} got={got:?} expected={expected:?}",
        );
    }
}
```
