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

## `picoquic/sockloop.c:picoquic_open_network_wake_up`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust always creates an mpsc channel and marks wake_up_defined true without setting ret on failure; C creates an OS event or pipe and records errno/GetLastError on failure.
* C source: `picoquic/sockloop.c:1705-1725`
* C signature: `void picoquic_open_network_wake_up(picoquic_network_thread_ctx_t *, int *)`
* Rust source: `rs/fq/src/packet_loop.rs:710-716`
* Rust item: `open_network_wake_up`

### C body
```c
{
    thread_ctx->wake_up_defined = 0;
#ifdef _WINDOWS
    thread_ctx->wake_up_event = CreateEvent(NULL, TRUE, FALSE, NULL);
    if (thread_ctx->wake_up_event == NULL) {
        *ret = GetLastError();
    }
    else {
        thread_ctx->wake_up_defined = 1;
    }
#else
    if (pipe(thread_ctx->wake_up_pipe_fd) != 0) {
        *ret = errno;
    }
    else
    {
        thread_ctx->wake_up_defined = 1;
    }
#endif
}
```

### Rust body
```rust
fn open_network_wake_up(thread_ctx: &mut NetworkThreadCtx, _ret: &mut i32) {
    thread_ctx.wake_up_defined = false;
    let (sender, receiver) = std::sync::mpsc::channel();
    thread_ctx.wake_up_sender = Some(sender);
    thread_ctx.wake_up_receiver = Some(receiver);
    thread_ctx.wake_up_defined = true;
}
```

## `picoquic/ticket_store.c:picoquic_store_ticket`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust computes time_valid_until and stores the ticket but has no body-visible current_time expiration rejection corresponding to the C check.
* C source: `picoquic/ticket_store.c:259-326`
* C signature: `int picoquic_store_ticket(picoquic_quic_t *, const char *, uint16_t, const char *, uint16_t, uint32_t, const uint8_t *, uint8_t, const uint8_t *, uint8_t, uint8_t *, uint16_t, const picoquic_tp_t *)`
* Rust source: `rs/fq/src/internal.rs:1439-1479`
* Rust item: `store_ticket`

### C body
```c
{
    uint64_t current_time = picoquic_get_tls_time(quic);
    picoquic_stored_ticket_t** pp_first_ticket = &quic->p_first_ticket;
    int ret = 0;

    if (ticket_length < 17) {
        ret = PICOQUIC_ERROR_INVALID_TICKET;
    } else {
        uint64_t ticket_issued_time;
        uint64_t ttl_seconds;
        uint64_t time_valid_until;

        ticket_issued_time = PICOPARSE_64(ticket);
        ttl_seconds = PICOPARSE_32(ticket + 13);

        if (ttl_seconds > (7 * 24 * 3600)) {
            ttl_seconds = (7 * 24 * 3600);
        }

        time_valid_until = (ticket_issued_time * 1000) + (ttl_seconds * 1000000);

        if (current_time != 0 && time_valid_until < current_time) {
            ret = PICOQUIC_ERROR_INVALID_TICKET;
        } else {
            picoquic_stored_ticket_t* stored = picoquic_format_ticket(time_valid_until, sni, sni_length,
                alpn, alpn_length, version, ip_addr, ip_addr_length,
                ip_addr_client, ip_addr_client_length,
                ticket, ticket_length, tp);
            if (stored == NULL) {
                ret = PICOQUIC_ERROR_MEMORY;
            }
            else {
                picoquic_stored_ticket_t* next;
                picoquic_stored_ticket_t** pprevious;

                stored->next_ticket = next = *pp_first_ticket;
                *pp_first_ticket = stored;
                pprevious = &stored->next_ticket;

                /* Now remove the old tickets for that SNI & ALPN & version */
                while (next != NULL) {
                    if (next->time_valid_until <= stored->time_valid_until &&
                        next->sni_length == sni_length &&
                        next->alpn_length == alpn_length &&
                        memcmp(next->sni, sni, sni_length) == 0 &&
                        memcmp(next->alpn, alpn, alpn_length) == 0 &&
                        next->version == version) {
                        picoquic_stored_ticket_t* deleted = next;
                        next = next->next_ticket;
                        *pprevious = next;
                        memset(&deleted->ticket, 0, deleted->ticket_length);
                        free(deleted);
                    } else {
                        pprevious = &next->next_ticket;
                        next = next->next_ticket;
                    }
                }
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let time_valid_until = ticket_valid_until(ticket)?;
        let stored = StoredTicket {
            sni: sni.map(str::to_owned),
            alpn: alpn.map(str::to_owned),
            ip_addr,
            ip_addr_client,
            tp_0rtt: {
                use crate::tp::TransportParameter0RttKind::*;
                let mut arr = [0u64; crate::tp::NB_TP_0RTT];
                arr[MaxData as usize] = tp.initial_max_data;
                arr[MaxStreamDataBidiLocal as usize] = tp.initial_max_stream_data_bidi_local;
                arr[MaxStreamDataBidiRemote as usize] = tp.initial_max_stream_data_bidi_remote;
                arr[MaxStreamDataUni as usize] = tp.initial_max_stream_data_uni;
                arr[MaxStreamsIdBidir as usize] = tp.initial_max_stream_id_bidir;
                arr[MaxStreamsIdUnidir as usize] = tp.initial_max_stream_id_unidir;
                arr
            },
            ticket: ticket.to_vec(),
            time_valid_until,
            version,
            was_used: false,
        };
        self.stored_tickets.retain(|old| {
            old.sni.as_deref() != sni
                || old.alpn.as_deref() != alpn
                || old.version != version
                || old.time_valid_until > time_valid_until
        });
        self.stored_tickets.insert(0, stored);
        Ok(())
    }
```

## `picoquic/tls_api.c:picoquic_aes128_ecb_encrypt`
* Phase 4C status: `suspect`
* Phase 4C rationale: C encrypts an arbitrary input length into a separate output buffer; Rust processes exactly one 16-byte block in place.
* C source: `picoquic/tls_api.c:1337-1340`
* C signature: `void picoquic_aes128_ecb_encrypt(void *, uint8_t *, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/tls_api.rs:2213-2221`
* Rust item: `process`

### C body
```c
{
    ptls_cipher_encrypt((ptls_cipher_context_t*)v_aesecb, output, input, len);
}
```

### Rust body
```rust
    pub fn process(&self, block: &mut [u8; 16]) {
        use cipher::{BlockDecrypt, BlockEncrypt};
        let mut b = cipher::generic_array::GenericArray::clone_from_slice(block);
        match self {
            Aes128EcbContext::Encrypt(enc) => enc.encrypt_block(&mut b),
            Aes128EcbContext::Decrypt(dec) => dec.decrypt_block(&mut b),
        }
        block.copy_from_slice(&b);
    }
```

## `picoquic/tls_api.c:picoquic_get_tls_time`
* Phase 4C status: `suspect`
* Phase 4C rationale: C calls the TLS context get_time callback and multiplies by 1000; Rust just returns self.time().
* C source: `picoquic/tls_api.c:1915-1922`
* C signature: `uint64_t picoquic_get_tls_time(picoquic_quic_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1169-1171`
* Rust item: `tls_time`

### C body
```c
{
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;
    uint64_t now = ctx->get_time->cb(ctx->get_time)*1000;

    return now;
}
```

### Rust body
```rust
    pub fn tls_time(&self) -> u64 {
        self.time()
    }
```

## `picoquic/tls_api.c:picoquic_rotate_app_secret`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust uses secret.len and TLS13_LABEL_PREFIX, while C uses cipher hash digest_size and PICOQUIC_LABEL_QUIC_BASE.
* C source: `picoquic/tls_api.c:1563-1589`
* C signature: `int picoquic_rotate_app_secret(ptls_cipher_suite_t *, uint8_t *, const char *)`
* Rust source: `rs/fq/src/tls_api.rs:1569-1598`
* Rust item: `rotate_app_secret`

### C body
```c
{
    int ret = 0;
    uint8_t new_secret[PTLS_MAX_DIGEST_SIZE];

    ret = ptls_hkdf_expand_label(cipher->hash, new_secret,
        cipher->hash->digest_size, ptls_iovec_init(secret, cipher->hash->digest_size), traffic_update_label,
        ptls_iovec_init(NULL, 0), PICOQUIC_LABEL_QUIC_BASE);
    if (ret == 0) {
        memcpy(secret, new_secret, cipher->hash->digest_size);
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    let mut new_secret = vec![0u8; secret.len()];
    match hash.output_size() {
        32 => hkdf_expand_label_sha256(
            traffic_update_label,
            TLS13_LABEL_PREFIX,
            secret,
            &mut new_secret,
        )?,
        48 => hkdf_expand_label_sha384(
            traffic_update_label,
            TLS13_LABEL_PREFIX,
            secret,
            &mut new_secret,
        )?,
        64 => hkdf_expand_label_sha512(
            traffic_update_label,
            TLS13_LABEL_PREFIX,
            secret,
            &mut new_secret,
        )?,
        _ => return Err(Error::InvalidArgument),
    }
    secret.copy_from_slice(&new_secret);
    Ok(())
}
```
