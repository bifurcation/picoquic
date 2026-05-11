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

## `picoquic/sockloop.c:picoquic_internal_thread_create`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns the result of thread creation; Rust unconditionally wraps std::thread::spawn in Ok with no visible error path.
* C source: `picoquic/sockloop.c:1727-1731`
* C signature: `int picoquic_internal_thread_create(void **, picoquic_thread_fn, void *)`
* Rust source: `rs/fq/src/packet_loop.rs:1464-1468`
* Rust item: `internal_thread_create`

### C body
```c
{
    int ret = picoquic_create_thread((picoquic_thread_t*)thread_id, thread_fn, thread_arg);
    return ret;
}
```

### Rust body
```rust
) -> Result<JoinHandle<()>, OsError> {
    Ok(std::thread::spawn(thread_fn))
}
```

## `picoquic/ticket_store.c:picoquic_format_ticket`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both populate ticket fields and 0-RTT transport parameters, but C truncates IP address lengths to PICOQUIC_STORED_IP_MAX and stores explicit lengths; Rust body directly stores ip_addr and ip_addr_client with no visible truncation or length handling.
* C source: `picoquic/ticket_store.c:29-99`
* C signature: `picoquic_stored_ticket_t * picoquic_format_ticket(uint64_t, const char *, uint16_t, const char *, uint16_t, uint32_t, const uint8_t *, uint8_t, const uint8_t *, uint8_t, uint8_t *, uint16_t, const picoquic_tp_t *)`
* Rust source: `rs/fq/src/tls_api.rs:2702-2736`
* Rust item: `format_ticket`

### C body
```c
{
    size_t ticket_size = sizeof(picoquic_stored_ticket_t) + sni_length + 1 + alpn_length + 1 + ticket_length
        + 1 + 2*PICOQUIC_STORED_IP_MAX;
    picoquic_stored_ticket_t* stored = (picoquic_stored_ticket_t*)malloc(ticket_size);
    
    if (stored != NULL) {
        char* next_p = ((char*)stored) + sizeof(picoquic_stored_ticket_t);

        memset(stored, 0, ticket_size);
        stored->time_valid_until = time_valid_until;
        stored->sni = next_p;
        stored->sni_length = sni_length;
        memcpy(next_p, sni, sni_length);
        next_p += sni_length;
        *next_p++ = 0;

        stored->alpn = next_p;
        stored->alpn_length = alpn_length;
        memcpy(next_p, alpn, alpn_length);
        next_p += alpn_length;
        *next_p++ = 0;

        stored->version = version;

        stored->ip_addr = (uint8_t *)next_p;
        if (ip_addr == NULL || ip_addr_length == 0) {
            stored->ip_addr_length = 0;
        }
        else {
            if (ip_addr_length > PICOQUIC_STORED_IP_MAX) {
                ip_addr_length = PICOQUIC_STORED_IP_MAX;
            }
            stored->ip_addr_length = ip_addr_length;
            memcpy(next_p, ip_addr, ip_addr_length);
        }
        next_p += PICOQUIC_STORED_IP_MAX;

        stored->ip_addr_client = (uint8_t*)next_p;
        if (ip_addr_client == NULL || ip_addr_client_length == 0) {
            stored->ip_addr_length = 0;
        }
        else {
            if (ip_addr_client_length > PICOQUIC_STORED_IP_MAX) {
                ip_addr_client_length = PICOQUIC_STORED_IP_MAX;
            }
            stored->ip_addr_client_length = ip_addr_client_length;
            memcpy(next_p, ip_addr_client, ip_addr_client_length);
        }
        next_p += PICOQUIC_STORED_IP_MAX;

        if (tp != NULL) {
            stored->tp_0rtt[picoquic_tp_0rtt_max_data] = tp->initial_max_data;
            stored->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_local] = tp->initial_max_stream_data_bidi_local;
            stored->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_remote] = tp->initial_max_stream_data_bidi_remote;
            stored->tp_0rtt[picoquic_tp_0rtt_max_stream_data_uni] = tp->initial_max_stream_data_uni;
            stored->tp_0rtt[picoquic_tp_0rtt_max_streams_id_bidir] = tp->initial_max_stream_id_bidir;
            stored->tp_0rtt[picoquic_tp_0rtt_max_streams_id_unidir] = tp->initial_max_stream_id_unidir;
        }

        stored->ticket = (uint8_t*)next_p;
        stored->ticket_length = ticket_length;
        memcpy(next_p, ticket, ticket_length);
    }

    return stored;
}
```

### Rust body
```rust
) -> StoredTicket {
    use crate::tp::TransportParameter0RttKind::*;
    let tp_0rtt = if let Some(tp) = tp {
        let mut arr = [0u64; NB_TP_0RTT];
        arr[MaxData as usize] = tp.initial_max_data;
        arr[MaxStreamDataBidiLocal as usize] = tp.initial_max_stream_data_bidi_local;
        arr[MaxStreamDataBidiRemote as usize] = tp.initial_max_stream_data_bidi_remote;
        arr[MaxStreamDataUni as usize] = tp.initial_max_stream_data_uni;
        arr[MaxStreamsIdBidir as usize] = tp.initial_max_stream_id_bidir;
        arr[MaxStreamsIdUnidir as usize] = tp.initial_max_stream_id_unidir;
        arr
    } else {
        [0u64; NB_TP_0RTT]
    };
    StoredTicket {
        sni: sni.map(str::to_owned),
        alpn: alpn.map(str::to_owned),
        ip_addr,
        ip_addr_client,
        tp_0rtt,
        ticket: ticket.to_vec(),
        time_valid_until,
        version,
        was_used: false,
    }
}
```

## `picoquic/tls_api.c:picoquic_aead_encrypt_generic`
* Phase 4C status: `suspect`
* Phase 4C rationale: C writes directly via the AEAD encrypt call; Rust encrypts into a temporary Vec and silently truncates the copy to output.len() while still returning the full encrypted length.
* C source: `picoquic/tls_api.c:2473-2483`
* C signature: `size_t picoquic_aead_encrypt_generic(uint8_t *, const uint8_t *, size_t, uint64_t, const uint8_t *, size_t, void *)`
* Rust source: `rs/fq/src/tls_api.rs:1292-1305`
* Rust item: `aead_encrypt_generic`

### C body
```c
{
    size_t encrypted = 0;

    encrypted = ptls_aead_encrypt((ptls_aead_context_t*)aead_context,
        (void*)output, (const void*)input, input_length, seq_num,
        (void*)auth_data, auth_data_length);

    return encrypted;
}
```

### Rust body
```rust
) -> usize {
    let mut buf = input.to_vec();
    aead_ctx.encrypt(seq_num, auth_data, &mut buf);
    let len = buf.len();
    let copy_len = len.min(output.len());
    output[..copy_len].copy_from_slice(&buf[..copy_len]);
    len
}
```

## `picoquic/tls_api.c:picoquic_get_app_secret`
* Phase 4C status: `suspect`
* Phase 4C rationale: C simply returns the selected stored secret pointer; Rust may modify app_secret_len and returns a length-limited mutable slice.
* C source: `picoquic/tls_api.c:1592-1597`
* C signature: `uint8_t * picoquic_get_app_secret(picoquic_cnx_t *, int)`
* Rust source: `rs/fq/src/tls_api.rs:1488-1497`
* Rust item: `app_secret`

### C body
```c
{
    picoquic_tls_ctx_t * tls_ctx = (picoquic_tls_ctx_t *)cnx->tls_ctx;

    return (is_enc) ?tls_ctx->app_secret_enc:tls_ctx->app_secret_dec;
}
```

### Rust body
```rust
    pub fn app_secret(&mut self, is_enc: bool) -> &mut [u8] {
        if self.app_secret_len == 0 || self.app_secret_len > HASH_SIZE_MAX {
            self.app_secret_len = SHA256_SIZE;
        }
        if is_enc {
            &mut self.app_secret_enc[..self.app_secret_len]
        } else {
            &mut self.app_secret_dec[..self.app_secret_len]
        }
    }
```

## `picoquic/tls_api.c:picoquic_register_key_exchange_algorithm`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust shows the slot replacement loop, but the C body also updates the special secp256r1 entry when the id matches; that special-case update is not visible in Rust.
* C source: `picoquic/tls_api.c:276-292`
* C signature: `void picoquic_register_key_exchange_algorithm(ptls_key_exchange_algorithm_t *)`
* Rust source: `rs/fq/src/tls_api.rs:2356-2362`
* Rust item: `register_key_exchange_algorithm`

### C body
```c
{
    for (int i = 0; i < PICOQUIC_KEY_EXCHANGES_NB_MAX; i++) {
        if (picoquic_key_exchanges[i] == NULL ||
            picoquic_key_exchanges[i]->id == key_exchange->id) {
            /* Replace the lower priority provider if present! */
            picoquic_key_exchanges[i] = key_exchange;
            break;
        }
    }

    if (key_exchange->id == PICOQUIC_GROUP_SECP256R1) {
        /* Replace the lower priority provider if present! */
        picoquic_key_exchange_secp256r1[0] = key_exchange;
    }
}
```

### Rust body
```rust
            if slot.provider.is_none() || slot.id == key_exchange_id {
                slot.id = key_exchange_id;
                slot.provider = Some(provider);
                break;
            }
```
