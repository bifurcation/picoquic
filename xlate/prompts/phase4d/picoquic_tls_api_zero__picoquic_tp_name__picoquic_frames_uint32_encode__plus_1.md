# Phase 4D deep translation classification

You are classifying Phase 4C non-OK C/Rust function-pair
audit entries.  Phase 4C was intentionally body-only and
shallow; Phase 4D classification is allowed to inspect
broader context.

For each entry:

1. Read the C function and any directly relevant C context:
   types, constants/macros, helper callees, and callers when
   needed to understand observable behavior.
2. Read the Rust function in context, including local types,
   helpers, tests, and nearby translated functions.
3. Decide whether the Phase 4C concern is a false positive.

Do not edit files in this classification pass.  Report:

* `ok` when the Rust behavior is acceptable after deeper
  inspection.
* `needs_fix` when the Rust translation is actually wrong and
  should be repaired in a later 4D repair pass.
* `blocked` only when the analysis cannot be completed without
  a concrete external decision or missing dependency.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short deeper-review conclusion","fix_summary":"empty unless outcome is needs_fix","files_changed":[],"verification":[]}]}
```

Entries:

## `picoquic/tls_api.c:picoquic_tls_api_zero`
* Phase 4C status: `suspect`
* Phase 4C rationale: C clears many callback/function pointers, while Rust visibly clears only cipher/key exchange slots and two provider fields.
* C source: `picoquic/tls_api.c:183-202`
* C signature: `void picoquic_tls_api_zero(void)`
* Rust source: `rs/fq/src/tls_api.rs:2325-2331`
* Rust item: `zero`

### C body
```c
{
    memset(picoquic_cipher_suites, 0, sizeof(picoquic_cipher_suites));
    memset((void*)picoquic_key_exchanges, 0, sizeof(picoquic_key_exchanges));
    memset((void*)picoquic_key_exchange_secp256r1, 0, sizeof(picoquic_key_exchange_secp256r1));

    picoquic_set_private_key_from_file_fn = NULL;
    picoquic_dispose_sign_certificate_fn = NULL;
    picoquic_get_certs_from_file_fn = NULL;
    picoquic_get_public_key_from_private_fn = NULL;

    picoquic_get_certificate_verifier_fn = NULL;
    picoquic_dispose_certificate_verifier_fn = NULL;
    picoquic_set_tls_root_certificates_fn = NULL;

    picoquic_explain_crypto_error_fn = NULL;
    picoquic_clear_crypto_errors_fn = NULL;
 
    picoquic_crypto_random_provider_fn = NULL;
}
```

### Rust body
```rust
    fn zero(&mut self) {
        self.cipher_suites = [CipherSuiteSlot::EMPTY; PICOQUIC_CIPHER_SUITES_NB_MAX];
        self.key_exchanges = [KeyExchangeSlot::EMPTY; PICOQUIC_KEY_EXCHANGES_NB_MAX];
        self.key_exchange_secp256r1 = None;
        self.crypto_random_provider = None;
        self.private_key_provider = None;
    }
```

## `picoquic/tp_names.c:picoquic_tp_name`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns the string "unknown" for default; Rust returns None for unknown, and the visible numeric mapping swaps retry_connection_id and handshake_connection_id positions compared with C case order.
* C source: `picoquic/tp_names.c:3-97`
* C signature: `const char * picoquic_tp_name(picoquic_tp_enum)`
* Rust source: `rs/fq/src/tp.rs:59-91`
* Rust item: `name`

### C body
```c
{
    char const* tp_name = "unknown";

    switch (tp_number) {
    case picoquic_tp_original_connection_id:
        tp_name = "original_connection_id";
        break;
    case picoquic_tp_idle_timeout:
        tp_name = "idle_timeout";
        break;
    case picoquic_tp_stateless_reset_token:
        tp_name = "stateless_reset_token";
        break;
    case picoquic_tp_max_packet_size:
        tp_name = "max_packet_size";
        break;
    case picoquic_tp_initial_max_data:
        tp_name = "initial_max_data";
        break;
    case picoquic_tp_initial_max_stream_data_bidi_local:
        tp_name = "initial_max_stream_data_bidi_local";
        break;
    case picoquic_tp_initial_max_stream_data_bidi_remote:
        tp_name = "initial_max_stream_data_bidi_remote";
        break;
    case picoquic_tp_initial_max_stream_data_uni:
        tp_name = "initial_max_stream_data_uni";
        break;
    case picoquic_tp_initial_max_streams_bidi:
        tp_name = "initial_max_streams_bidi";
        break;
    case picoquic_tp_initial_max_streams_uni:
        tp_name = "initial_max_streams_uni";
        break;
    case picoquic_tp_ack_delay_exponent:
        tp_name = "ack_delay_exponent";
        break;
    case picoquic_tp_max_ack_delay:
        tp_name = "max_ack_delay";
        break;
    case picoquic_tp_disable_migration:
        tp_name = "disable_migration";
        break;
    case picoquic_tp_server_preferred_address:
        tp_name = "server_preferred_address";
        break;
    case picoquic_tp_active_connection_id_limit:
        tp_name = "active_connection_id_limit";
        break;
    case picoquic_tp_retry_connection_id:
        tp_name = "retry_connection_id";
        break;
    case picoquic_tp_handshake_connection_id:
        tp_name = "handshake_connection_id";
        break;
    case picoquic_tp_max_datagram_frame_size:
        tp_name = "max_datagram_frame_size";
        break;
    case picoquic_tp_test_large_chello:
        tp_name = "large_chello";
        break;
    case picoquic_tp_enable_loss_bit:
        tp_name = "enable_loss_bit";
        break;
    case picoquic_tp_min_ack_delay:
        tp_name = "min_ack_delay";
        break;
    case picoquic_tp_enable_time_stamp:
        tp_name = "enable_time_stamp";
        break;
    case picoquic_tp_grease_quic_bit:
        tp_name = "grease_quic_bit";
        break;
    case picoquic_tp_version_negotiation:
        tp_name = "version_negotiation";
        break;
    case picoquic_tp_enable_bdp_frame:
        tp_name = "enable_bdp_frame";
        break;
    case picoquic_tp_initial_max_path_id:
        tp_name = "initial_max_path_id";
        break;
    case picoquic_tp_address_discovery:
        tp_name = "address_discovery";
        break;
    case picoquic_tp_reset_stream_at:
        tp_name = "reset_stream_at";
        break;
    default:
        break;
    }

    return tp_name;
}
```

### Rust body
```rust
    pub fn name(tp_number: u64) -> Option<&'static str> {
        match tp_number {
            0 => Some("original_connection_id"),
            1 => Some("idle_timeout"),
            2 => Some("stateless_reset_token"),
            3 => Some("max_packet_size"),
            4 => Some("initial_max_data"),
            5 => Some("initial_max_stream_data_bidi_local"),
            6 => Some("initial_max_stream_data_bidi_remote"),
            7 => Some("initial_max_stream_data_uni"),
            8 => Some("initial_max_streams_bidi"),
            9 => Some("initial_max_streams_uni"),
            10 => Some("ack_delay_exponent"),
            11 => Some("max_ack_delay"),
            12 => Some("disable_migration"),
            13 => Some("server_preferred_address"),
            14 => Some("active_connection_id_limit"),
            15 => Some("handshake_connection_id"),
            16 => Some("retry_connection_id"),
            0x11 => Some("version_negotiation"),
            32 => Some("max_datagram_frame_size"),
            3127 => Some("large_chello"),
            0x1057 => Some("enable_loss_bit"),
            0x7158 => Some("enable_time_stamp"),
            0x2ab2 => Some("grease_quic_bit"),
            0xebd9 => Some("enable_bdp_frame"),
            0x3e => Some("initial_max_path_id"),
            0xff04de1b => Some("min_ack_delay"),
            0x9f81a176 => Some("address_discovery"),
            0x17f7586d2cb571 => Some("reset_stream_at"),
            _ => None,
        }
    }
```

## `picoquic/util.c:picoquic_frames_uint32_encode`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks capacity then writes four big-endian bytes and returns the advanced pointer; Rust body shown only performs the capacity check with no visible writes or return of the advanced slice.
* C source: `picoquic/util.c:1027-1039`
* C signature: `uint8_t * picoquic_frames_uint32_encode(uint8_t *, const uint8_t *, uint32_t)`
* Rust source: `rs/fq/src/utils.rs:890-893`
* Rust item: `frames_uint32_encode`

### C body
```c
{
    if (bytes + sizeof(n) > bytes_max) {
        bytes = NULL;
    }
    else {
        *bytes++ = (uint8_t)(n >> 24);
        *bytes++ = (uint8_t)(n >> 16);
        *bytes++ = (uint8_t)(n >> 8);
        *bytes++ = (uint8_t)n;
    }
    return (bytes);
}
```

### Rust body
```rust
    if bytes.len() < 4 {
        return None;
    }
```

## `picoquic/util.c:picoquic_uint8_to_str`
* Phase 4C status: `suspect`
* Phase 4C rationale: C always writes a terminating NUL at text[rendered]; Rust only writes it if rendered < text_len and returns a slice excluding the NUL.
* C source: `picoquic/util.c:1406-1438`
* C signature: `char * picoquic_uint8_to_str(char *, size_t, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/utils.rs:1038-1063`
* Rust item: `uint8_to_str`

### C body
```c
{
    size_t render_length = data_len;
    size_t rendered;

    if (render_length + 1 > text_len) {
        if (text_len > 4) {
            render_length = text_len - 4;
        }
        else {
            render_length = 0;
        }
    }

    for (rendered = 0; rendered < render_length; rendered++) {
        int c = data[rendered];
        if (c < ' ' || c >= 127) {
            c = '?';
        }
        text[rendered] = (char)c;
    }

    if (rendered < data_len) {
        for (size_t i = 0; i < 3 && rendered + 1 < text_len; i++, rendered++) {
            text[rendered] = '.';
        }
    }
    text[rendered] = 0;

    return text;
}
```

### Rust body
```rust
pub fn uint8_to_str<'a>(text: &'a mut [u8], data: &[u8]) -> &'a [u8] {
    let text_len = text.len();
    let data_len = data.len();
    let render_length = if data_len >= text_len {
        text_len.saturating_sub(4)
    } else {
        data_len
    };
    let mut rendered = 0usize;
    for &c in &data[..render_length] {
        text[rendered] = if (b' '..127).contains(&c) { c } else { b'?' };
        rendered += 1;
    }
    if rendered < data_len {
        let mut i = 0usize;
        while i < 3 && rendered + 1 < text_len {
            text[rendered] = b'.';
            rendered += 1;
            i += 1;
        }
    }
    if rendered < text_len {
        text[rendered] = 0;
    }
    &text[..rendered]
}
```
