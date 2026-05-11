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

## `picoquic/sockloop.c:picoquic_internal_thread_delete`
* Phase 4C status: `suspect`
* Phase 4C rationale: C deletes a thread handle, while Rust joins the thread; body-visible behavior could block rather than just delete/free.
* C source: `picoquic/sockloop.c:1763-1766`
* C signature: `void picoquic_internal_thread_delete(void **)`
* Rust source: `rs/fq/src/packet_loop.rs:1475-1477`
* Rust item: `internal_thread_delete`

### C body
```c
{
    picoquic_delete_thread((picoquic_thread_t *)v_thread_id);
}
```

### Rust body
```rust
pub fn internal_thread_delete(thread: JoinHandle<()>) {
    let _ = thread.join();
}
```

## `picoquic/ticket_store.c:picoquic_get_ticket_and_version`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets was_used to mark_used unconditionally and only writes ticket_version when tp is non-null; Rust only sets was_used when mark_used is true and always returns version/transport parameters.
* C source: `picoquic/ticket_store.c:354-383`
* C signature: `int picoquic_get_ticket_and_version(picoquic_quic_t *, const char *, uint16_t, const char *, uint16_t, uint32_t, uint32_t *, uint8_t **, uint16_t *, picoquic_tp_t *, int)`
* Rust source: `rs/fq/src/internal.rs:1520-1549`
* Rust item: `get_ticket_and_version`

### C body
```c
{
    int ret = 0;
    picoquic_stored_ticket_t* next = picoquic_get_stored_ticket(
        quic, sni, sni_length, alpn, alpn_length, version, mark_used, 0);

    if (next == NULL) {
        *ticket = NULL;
        *ticket_length = 0;
        ret = -1;
    } else {
        if (tp != NULL) {
            tp->initial_max_data = next->tp_0rtt[picoquic_tp_0rtt_max_data];
            tp->initial_max_stream_data_bidi_local = next->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_local];
            tp->initial_max_stream_data_bidi_remote = next->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_remote];
            tp->initial_max_stream_data_uni = next->tp_0rtt[picoquic_tp_0rtt_max_stream_data_uni];
            tp->initial_max_stream_id_bidir = next->tp_0rtt[picoquic_tp_0rtt_max_streams_id_bidir];
            tp->initial_max_stream_id_unidir = next->tp_0rtt[picoquic_tp_0rtt_max_streams_id_unidir];
            *ticket_version = next->version;
        }
        *ticket = next->ticket;
        *ticket_length = next->ticket_length;
        next->was_used = mark_used;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(u32, &[u8], TransportParameters), crate::Error> {
        let idx = self
            .stored_tickets
            .iter()
            .position(|ticket| {
                ticket.time_valid_until.ticks() > 0
                    && ticket.sni.as_deref() == sni
                    && ticket.alpn.as_deref() == alpn
                    && (version == 0 || ticket.version == version)
                    && (!mark_used || !ticket.was_used)
            })
            .ok_or(crate::Error::Generic)?;

        if mark_used {
            self.stored_tickets[idx].was_used = true;
        }

        let ticket = &self.stored_tickets[idx];
        Ok((
            ticket.version,
            ticket.ticket.as_slice(),
            stored_ticket_tp(ticket),
        ))
    }
```

## `picoquic/tls_api.c:picoquic_aes128_ecb_create`
* Phase 4C status: `suspect`
* Phase 4C rationale: C delegates creation by name for AES128-ECB; the shown Rust body only shows the decrypt branch, so the full create behavior is not body-visible.
* C source: `picoquic/tls_api.c:489-491`
* C signature: `void * picoquic_aes128_ecb_create(int, const void *)`
* Rust source: `rs/fq/src/tls_api.rs:2196-2206`
* Rust item: `new`

### C body
```c
void* picoquic_aes128_ecb_create(int is_enc, const void* ecb_key) {
    return picoquic_ecb_create_by_name(is_enc, ecb_key, "AES128-ECB");
}
```

### Rust body
```rust
        } else {
            Aes128EcbContext::Decrypt(
                aes::Aes128Dec::new_from_slice(ecb_key).expect("key is 16 bytes"),
            )
        }
```

## `picoquic/tls_api.c:picoquic_get_certs_from_file`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns NULL when no registered callback exists and otherwise delegates; Rust directly reads the file and returns an empty Vec on read error.
* C source: `picoquic/tls_api.c:631-641`
* C signature: `ptls_iovec_t * picoquic_get_certs_from_file(const char *, size_t *)`
* Rust source: `rs/fq/src/sys/openssl.rs:346-350`
* Rust item: `get_certs_from_file`

### C body
```c
{
    if (picoquic_get_certs_from_file_fn == NULL) {
        return NULL;
    }
    else {
        return picoquic_get_certs_from_file_fn(file_name, count);
    }
}
```

### Rust body
```rust
    let pem = match std::fs::read(file_name) {
        Ok(data) => data,
        Err(_) => return Vec::new(),
    };
```

## `picoquic/tls_api.c:picoquic_register_tls_key_provider_fn`
* Phase 4C status: `suspect`
* Phase 4C rationale: C conditionally registers multiple function pointers including public-key extraction; Rust stores a single provider value.
* C source: `picoquic/tls_api.c:321-337`
* C signature: `void picoquic_register_tls_key_provider_fn(picoquic_set_private_key_from_file_t, picoquic_dispose_sign_certificate_t, picoquic_get_certs_from_file_t, picoquic_get_public_key_from_private_t)`
* Rust source: `rs/fq/src/tls_api.rs:2378-2380`
* Rust item: `register_tls_key_provider`

### C body
```c
{
    DBG_PRINTF("%s", "Loading set key functions.");
    if (set_key_from_key_file_fn != NULL) {
        picoquic_set_private_key_from_file_fn = set_key_from_key_file_fn;
        picoquic_dispose_sign_certificate_fn = dispose_sign_certificate_fn;
        picoquic_get_certs_from_file_fn = get_certs_from_file_fn;
    }

    if (get_public_key_from_private_fn != NULL) {
        picoquic_get_public_key_from_private_fn = get_public_key_from_private_fn;
    }
}
```

### Rust body
```rust
    fn register_tls_key_provider(&mut self, provider: CryptoProvider) {
        self.private_key_provider = Some(provider);
    }
```
