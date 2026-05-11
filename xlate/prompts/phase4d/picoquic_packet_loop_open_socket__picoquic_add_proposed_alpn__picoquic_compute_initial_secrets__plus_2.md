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

## `picoquic/sockloop.c:picoquic_packet_loop_open_socket`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust omits body-visible Windows coalescing setup/error path and treats several setup calls as direct Result propagation rather than C's combined ret=-1 flow; visible socket option ordering also differs around local_address and PMTUD.
* C source: `picoquic/sockloop.c:363-462`
* C signature: `int picoquic_packet_loop_open_socket(int, int, picoquic_socket_ctx_t *, uint8_t)`
* Rust source: `rs/fq/src/packet_loop.rs:624-657`
* Rust item: `packet_loop_open_socket`

### C body
```c
{
    int ret = 0;
    struct sockaddr_storage local_address;
    int recv_set = 0;
    int send_set = 0;
    int opt_val = 1;
#ifdef _WINDOWS
    int recv_coalesced = 0;
    int send_coalesced = 0;

    /* Assess whether coalescing is supported */
    if (!do_not_use_gso) {
        picoquic_sockloop_win_coalescing_test(&recv_coalesced, &send_coalesced);
#if 0
        /* TODO: remove temporary fix, after we figure how to work that out. */
        recv_coalesced = 0;
#endif
    }
    s_ctx->overlap.hEvent = WSA_INVALID_EVENT;
    s_ctx->fd = WSASocket(s_ctx->af, SOCK_DGRAM, IPPROTO_UDP, NULL, 0, WSA_FLAG_OVERLAPPED);
#else
    s_ctx->fd = socket(s_ctx->af, SOCK_DGRAM, IPPROTO_UDP);
#endif


    if (s_ctx->fd == INVALID_SOCKET ||
#ifndef ESP_PLATFORM
        /* TODO: set option IPv6 only */
        picoquic_socket_set_ecn_options_ex(s_ctx->fd, s_ctx->af, &recv_set, &send_set, ecn_value) != 0 ||
#endif
        picoquic_socket_set_pkt_info(s_ctx->fd, s_ctx->af) != 0 ||
        (s_ctx->is_port_shared && setsockopt(s_ctx->fd, SOL_SOCKET, SO_REUSEADDR, (const char*)&opt_val, sizeof(opt_val)) != 0) ||
#if defined(SO_REUSEPORT)
        (s_ctx->is_port_shared && setsockopt(s_ctx->fd, SOL_SOCKET, SO_REUSEPORT, (const char*)&opt_val, sizeof(opt_val)) != 0) ||
#endif
        picoquic_bind_to_port(s_ctx->fd,s_ctx->af, s_ctx->port) != 0 ||
        picoquic_get_local_address(s_ctx->fd, &local_address) != 0 ||
        picoquic_socket_set_pmtud_options(s_ctx->fd, s_ctx->af) != 0)
    {
        DBG_PRINTF("Cannot set socket (af=%d, port = %d)\n", s_ctx->af, s_ctx->port);
        ret = -1;
    }
    else {
        if (local_address.ss_family == AF_INET6) {
            s_ctx->port = ntohs(((struct sockaddr_in6*)&local_address)->sin6_port);
        }
        else if (local_address.ss_family == AF_INET) {
            s_ctx->port = ntohs(((struct sockaddr_in*)&local_address)->sin_port);
        }
        
#ifndef ESP_PLATFORM
        if (socket_buffer_size > 0) {
            socklen_t opt_len;
            int opt_ret;
            int so_sndbuf;
            int so_rcvbuf;
            int last_op = SO_SNDBUF;
            char const* last_op_name = "SO_SNDBUF";

            opt_len = sizeof(int);
            so_sndbuf = socket_buffer_size;
            opt_ret = setsockopt(s_ctx->fd, SOL_SOCKET, SO_SNDBUF, (const char*)&so_sndbuf, opt_len);
            if (opt_ret == 0) {
                last_op = SO_RCVBUF;
                last_op_name = "SO_RECVBUF";
                opt_len = sizeof(int);
                so_rcvbuf = socket_buffer_size;
                opt_ret = setsockopt(s_ctx->fd, SOL_SOCKET, SO_RCVBUF, (const char*)&so_rcvbuf, opt_len);
            }
            if (opt_ret != 0) {
                int so_errbuf = 0;
#ifdef _WINDOWS
                int sock_error = WSAGetLastError();
#else
                int sock_error = errno;
#endif
                opt_ret = getsockopt(s_ctx->fd, SOL_SOCKET, last_op, (char*)&so_errbuf, &opt_len);
                DBG_PRINTF("Cannot set %s to %d, err=%d, so_sndbuf=%d (%d)",
                    last_op_name, socket_buffer_size, sock_error, so_errbuf, opt_ret);
                ret = -1;
            }
        }
#endif

#ifdef _WINDOWS
        if (ret == 0) {
            ret = picoquic_packet_set_windows_socket(send_coalesced, recv_coalesced, s_ctx);
        }
#endif
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    let mut fd = match s_ctx.af {
        AF_INET | AF_INET6 => S::open_udp(s_ctx.af)?,
        _ => return Err(Error::InvalidArgument),
    };

    fd.set_ecn_options_ex(ecn_codepoint(ecn_value))?;
    fd.set_pkt_info()?;
    if s_ctx.is_port_shared {
        fd.set_reuse_addr(true)?;
        fd.set_reuse_port(true)?;
    }
    fd.bind_to_port(s_ctx.af, s_ctx.port as i32)?;
    fd.set_pmtud_options()?;

    let local_address = fd.local_address()?;
    s_ctx.port = local_address.port();
    s_ctx.n_port = s_ctx.port.to_be();
    if socket_buffer_size > 0 {
        let size = usize::try_from(socket_buffer_size).map_err(|_| Error::InvalidArgument)?;
        fd.set_send_buffer_size(size)?;
        fd.set_recv_buffer_size(size)?;
    }
    s_ctx.fd = Some(fd);
    s_ctx.is_started = true;
    s_ctx.supports_udp_send_coalesced = false;
    s_ctx.supports_udp_recv_coalesced = false;
    Ok(())
}
```

## `picoquic/tls_api.c:picoquic_add_proposed_alpn`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks a context-specific alpn_vec_size and stores base/len; Rust uses a fixed ALPN_NUMBER_MAX and pushes an owned string, so the visible capacity behavior may differ.
* C source: `picoquic/tls_api.c:2206-2223`
* C signature: `int picoquic_add_proposed_alpn(void *, const char *)`
* Rust source: `rs/fq/src/tls_api.rs:2771-2777`
* Rust item: `add_proposed_alpn`

### C body
```c
{
    int ret = 0;
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)tls_context;
    if (ctx == NULL) {
        ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
    }
    else if (ctx->alpn_count >= ctx->alpn_vec_size) {
        ret = PICOQUIC_ERROR_SEND_BUFFER_TOO_SMALL;
    } else {
        ctx->alpn_vec[ctx->alpn_count].base = (uint8_t*)alpn;
        ctx->alpn_vec[ctx->alpn_count].len = strlen(alpn);
        ctx->alpn_count++;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn add_proposed_alpn(&mut self, alpn: &str) -> Result<(), Error> {
        if self.alpn_proposals.len() >= crate::internal::ALPN_NUMBER_MAX {
            return Err(Error::BufferTooSmall);
        }
        self.alpn_proposals.push(alpn.to_owned());
        Ok(())
```

## `picoquic/tls_api.c:picoquic_compute_initial_secrets`
* Phase 4C status: `suspect`
* Phase 4C rationale: C fails if AES128GCM_SHA256 suite lookup returns NULL; Rust unconditionally selects Aes128GcmSha256 and has no visible suite lookup failure path.
* C source: `picoquic/tls_api.c:1478-1498`
* C signature: `int picoquic_compute_initial_secrets(picoquic_quic_t *, int, picoquic_connection_id_t *, ptls_cipher_suite_t **, uint8_t *, uint8_t *)`
* Rust source: `rs/fq/src/tls_api.rs:685-698`
* Rust item: `picoquic_compute_initial_secrets`

### C body
```c
{
    int ret = 0;
    ptls_iovec_t salt;
    uint8_t master_secret[256]; /* secret_max */
    *cipher = picoquic_get_aes128gcm_sha256(quic->use_low_memory);
    if (*cipher == NULL) {
        ret = -1;
    }
    else {
        picoquic_setup_cleartext_aead_salt(version_index, &salt);

        /* Extract the master key -- key length will be 32 per SHA256 */
        ret = picoquic_setup_initial_master_secret(*cipher, salt, *initial_cnxid, master_secret);
        if (ret == 0) {
            ret = picoquic_setup_initial_secrets(*cipher, master_secret, client_secret, server_secret);
        }
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<(AeadSuiteId, [u8; SHA256_SIZE], [u8; SHA256_SIZE]), Error> {
    let suite = AeadSuiteId::Aes128GcmSha256;
    let salt = setup_cleartext_aead_salt(version_index);
    let mut master = [0u8; SHA256_SIZE];
    let mut client = [0u8; SHA256_SIZE];
    let mut server = [0u8; SHA256_SIZE];
    setup_initial_master_secret(salt, *initial_cnxid, &mut master)?;
    setup_initial_secrets(&master, &mut client, &mut server)?;
    Ok((suite, client, server))
}
```

## `picoquic/tls_api.c:picoquic_master_tlscontext_free`
* Phase 4C status: `suspect`
* Phase 4C rationale: C frees many TLS context subfields and callbacks while Rust only clears client and server config fields.
* C source: `picoquic/tls_api.c:1874-1913`
* C signature: `void picoquic_master_tlscontext_free(picoquic_quic_t *)`
* Rust source: `rs/fq/src/tls_api.rs:968-971`
* Rust item: `free_master_tls_context`

### C body
```c
{
    if (quic->tls_master_ctx != NULL) {
        ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;

        if (quic->p_simulated_time != NULL && ctx->get_time != NULL) {
            free(ctx->get_time);
            ctx->get_time = NULL;
        }

        free_certificates_list(ctx->certificates.list, ctx->certificates.count);

        picoquic_dispose_sign_certificate(ctx);

        picoquic_dispose_verify_certificate_callback(quic);

        if (ctx->on_client_hello != NULL) {
            free(ctx->on_client_hello);
        }

        if (ctx->encrypt_ticket != NULL) {
            free(ctx->encrypt_ticket);
        }

        if (ctx->update_traffic_key != NULL) {
            free(ctx->update_traffic_key);
        }

        /* Need to be tested */
        if (ctx->save_ticket != NULL) {
            free(ctx->save_ticket);
        }

        if (ctx->cipher_suites != NULL) {
            free((void*)ctx->cipher_suites);
        }

        picoquic_free_log_event(quic);
    }
}
```

### Rust body
```rust
    pub fn free_master_tls_context(&mut self) {
        self.tls_client_config = None;
        self.tls_server_config = None;
    }
```

## `picoquic/tls_api.c:picoquic_server_encrypt_retry_token`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns ret and sets *token_length to 0 on insufficient space; Rust returns BufferTooSmall. Otherwise the visible random prefix, new-token bit, sequence, address AAD, encryption, and length computation broadly match.
* C source: `picoquic/tls_api.c:2849-2886`
* C signature: `int picoquic_server_encrypt_retry_token(picoquic_quic_t *, const struct sockaddr *, int, uint8_t *, size_t *, size_t, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/tls_api.rs:1743-1775`
* Rust item: `picoquic_server_encrypt_retry_token`

### C body
```c
{
    int ret = 0;
    uint64_t sequence;
    uint8_t* auth_data;
    size_t auth_data_length;

    if (text_length + 1u + 16u > token_max) {
        ret = -1;
        *token_length = 0;
    }
    else {

        if (addr_peer->sa_family == AF_INET) {
            auth_data = (uint8_t*)&((struct sockaddr_in*)addr_peer)->sin_addr;
            auth_data_length = 4;
        }
        else {
            auth_data = (uint8_t*)&((struct sockaddr_in6*)addr_peer)->sin6_addr;
            auth_data_length = 16;
        }
        picoquic_crypto_random(quic, token, 8);
        if (is_new_token) {
            token[0] |= 0x80;
        }
        else {
            token[0] &= 0x7F;
        }
        sequence = PICOPARSE_64(token);

        *token_length = (size_t)8u + picoquic_aead_encrypt_generic(token + 8, text, text_length,
            sequence, auth_data, auth_data_length, quic->aead_encrypt_ticket_ctx);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<usize, Error> {
        ensure_ticket_aead_contexts(self)?;
        let required = 8usize
            .checked_add(text.len())
            .and_then(|n| n.checked_add(QUIC_AEAD_TAG_LEN))
            .ok_or(Error::BufferTooSmall)?;
        if token.len() < required {
            return Err(Error::BufferTooSmall);
        }

        self.crypto_random(&mut token[..8]);
        if is_new_token {
            token[0] |= 0x80;
        } else {
            token[0] &= 0x7f;
        }
        let sequence = u64::from_be_bytes(token[..8].try_into().unwrap());
        let aad = ip_auth_data(addr_peer);
        let mut payload = text.to_vec();
        let aead = self
            .aead_encrypt_ticket_ctx
            .as_ref()
            .ok_or(Error::InvalidState)?;
        aead.encrypt(sequence, &aad, &mut payload);
        token[8..8 + payload.len()].copy_from_slice(&payload);
        Ok(8 + payload.len())
    }
```
