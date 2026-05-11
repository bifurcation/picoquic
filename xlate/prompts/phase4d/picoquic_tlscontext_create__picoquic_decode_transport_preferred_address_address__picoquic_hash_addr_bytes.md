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

## `picoquic/tls_api.c:picoquic_tlscontext_create`
* Phase 4C status: `suspect`
* Phase 4C rationale: C allocates and initializes TLS context buffers and handshake properties with a server-without-cert error check; Rust creates a session object, clears a send buffer, and uses visibly different setup paths.
* C source: `picoquic/tls_api.c:1924-1997`
* C signature: `int picoquic_tlscontext_create(picoquic_quic_t *, picoquic_cnx_t *)`
* Rust source: `rs/fq/src/tls_api.rs:986-1021`
* Rust item: `create_tls_context`

### C body
```c
{
    int ret = 0;
    /* allocate a context structure, but only if checks are correct */
    picoquic_tls_ctx_t* ctx = NULL;

    if (!cnx->client_mode && ((ptls_context_t*)quic->tls_master_ctx)->encrypt_ticket == NULL) {
        /* A server side connection, but no cert/key where given for the master context */
        ret = PICOQUIC_ERROR_TLS_SERVER_CON_WITHOUT_CERT;
    }
    else {
        ctx = (picoquic_tls_ctx_t*)malloc(sizeof(picoquic_tls_ctx_t));
        if (ctx == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
    }

    /* Create the TLS context */
    if (ctx != NULL) {
        memset(ctx, 0, sizeof(picoquic_tls_ctx_t));
        ctx->ext_data_size = PICOQUIC_TRANSPORT_PARAMETERS_MAX_SIZE;
        if (!cnx->client_mode && quic->test_large_server_flight) {
            ctx->ext_data_size += 4096;
        }
        ctx->ext_data = (uint8_t*)malloc(ctx->ext_data_size);
        ctx->alpn_vec = (ptls_iovec_t*)malloc(sizeof(ptls_iovec_t) * PICOQUIC_ALPN_NUMBER_MAX);
        if (ctx->ext_data == NULL || ctx->alpn_vec == NULL) {
            ret = -1;
        }
        else {
            ctx->alpn_vec_size = PICOQUIC_ALPN_NUMBER_MAX;
            ctx->cnx = cnx;

            ctx->handshake_properties.collect_extension = picoquic_tls_collect_extensions_cb;
            ctx->handshake_properties.collected_extensions = picoquic_tls_collected_extensions_cb;
            ctx->client_mode = cnx->client_mode;

            ctx->tls = ptls_new((ptls_context_t*)quic->tls_master_ctx,
                (ctx->client_mode) ? 0 : 1);
            if (ctx->tls == NULL) {
                picoquic_tlscontext_free(ctx, cnx->client_mode);
                ctx = NULL;
                ret = PICOQUIC_ERROR_MEMORY;
            }
            else{
                *ptls_get_data_ptr(ctx->tls) = cnx;
                if (!ctx->client_mode) {
                    /* The server should never attempt a stateless retry */
                    ctx->handshake_properties.server.enforce_retry = 0;
                    ctx->handshake_properties.server.retry_uses_cookie = 0;
                    ctx->handshake_properties.server.cookie.key = NULL;
                    ctx->handshake_properties.server.cookie.additional_data.base = NULL;
                    ctx->handshake_properties.server.cookie.additional_data.len = 0;
                }
                else {
                    ctx->handshake_properties.client.ech.retry_configs = &ctx->retry_configs;
                }
            }
        }
    }

    if (cnx->tls_ctx != NULL) {
        picoquic_tlscontext_free(cnx->tls_ctx, cnx->client_mode);
    }

    cnx->tls_ctx = (void*)ctx;

    return ret;
}
```

### Rust body
```rust
    pub fn create_tls_context(&mut self, quic: &mut Quic) -> Result<(), Error> {
        let version = connection_version(self);
        let transport_params = Vec::new();
        let session: Box<dyn crate::tls::Session> = if self.client_mode {
            if let Some(config) = quic.tls_client_config.as_ref() {
                let sni = self.sni.as_deref().unwrap_or("");
                config
                    .start_session(version as u32, sni, &transport_params)
                    .map_err(|_| Error::Tls)?
            } else {
                Box::new(LocalSession::new(
                    true,
                    version.parameters().tls_prefix_label,
                ))
            }
        } else if let Some(config) = quic.tls_server_config.as_ref() {
            config
                .start_session(version as u32, &transport_params)
                .map_err(|_| Error::Tls)?
        } else {
            if quic.enforce_client_only {
                return Err(Error::Protocol(
                    InternalError::TlsServerConWithoutCert as u64,
                ));
            }
            Box::new(LocalSession::new(
                false,
                version.parameters().tls_prefix_label,
            ))
        };

        self.tls_ctx = Some(session);
        self.tls_sendbuf.clear();
        self.app_secret_len = SHA256_SIZE;
        Ok(())
    }
```

## `picoquic/transport.c:picoquic_decode_transport_preferred_address_address`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets preferred_address->is_defined = 1 on success; Rust fills address fields and returns length but has no visible equivalent flag assignment.
* C source: `picoquic/transport.c:130-162`
* C signature: `size_t picoquic_decode_transport_preferred_address_address(uint8_t *, size_t, picoquic_tp_preferred_address_t *)`
* Rust source: `rs/fq/src/internal.rs:15160-15215`
* Rust item: `picoquic_decode_transport_preferred_address_address`

### C body
```c
{
    /* first compute the minimal length */
    size_t byte_index = 0;
    uint8_t cnx_id_length = 0;
    size_t minimal_length = 4u + 2u + 16u + 2u + 1u /* + preferred_address->connection_id.id_len */ + 16u;
    size_t ret = 0;

    if (bytes_max >= minimal_length) {
        memcpy(preferred_address->ipv4Address, bytes + byte_index, 4);
        byte_index += 4;
        preferred_address->ipv4Port = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        memcpy(preferred_address->ipv6Address, bytes + byte_index, 16);
        byte_index += 16;
        preferred_address->ipv6Port = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        cnx_id_length = bytes[byte_index++];
        if (cnx_id_length > 0 && cnx_id_length <= PICOQUIC_CONNECTION_ID_MAX_SIZE &&
            byte_index + (size_t)cnx_id_length + 16u <= bytes_max &&
            cnx_id_length == picoquic_parse_connection_id(bytes + byte_index, cnx_id_length,
                &preferred_address->connection_id)){
            byte_index += cnx_id_length;
            memcpy(preferred_address->statelessResetToken, bytes + byte_index, 16);
            byte_index += 16;
            ret = byte_index;
            preferred_address->is_defined = 1;
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> usize {
    let bytes_max = bytes_max.min(bytes.len());
    let minimal_length = 4 + 2 + 16 + 2 + 1 + 16;
    if bytes_max < minimal_length {
        return 0;
    }

    let mut byte_index = 0;
    let ipv4 = core::net::Ipv4Addr::new(
        bytes[byte_index],
        bytes[byte_index + 1],
        bytes[byte_index + 2],
        bytes[byte_index + 3],
    );
    byte_index += 4;
    let ipv4_port = parse_16(&bytes[byte_index..byte_index + 2]);
    byte_index += 2;

    let mut ipv6_octets = [0u8; 16];
    ipv6_octets.copy_from_slice(&bytes[byte_index..byte_index + 16]);
    byte_index += 16;
    let ipv6_port = parse_16(&bytes[byte_index..byte_index + 2]);
    byte_index += 2;

    let cnx_id_length = bytes[byte_index] as usize;
    byte_index += 1;
    if cnx_id_length == 0
        || cnx_id_length > crate::CONNECTION_ID_MAX_SIZE
        || byte_index + cnx_id_length + 16 > bytes_max
    {
        return 0;
    }
    let Some(connection_id) =
        ConnectionId::clone_from_slice(&bytes[byte_index..byte_index + cnx_id_length])
    else {
        return 0;
    };
    byte_index += cnx_id_length;

    let mut stateless_reset_token = [0u8; 16];
    stateless_reset_token.copy_from_slice(&bytes[byte_index..byte_index + 16]);
    byte_index += 16;

    preferred_address.v4 = Some(SocketAddr::from((ipv4, ipv4_port)));
    preferred_address.v6 = Some(SocketAddr::from((
        core::net::Ipv6Addr::from(ipv6_octets),
        ipv6_port,
    )));
    preferred_address.connection_id = connection_id;
    preferred_address.stateless_reset_token = stateless_reset_token;
    byte_index
}
```

## `picoquic/util.c:picoquic_hash_addr_bytes`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body includes AF_INET and AF_INET6 cases and returns 0 for other families; Rust body shown only handles SocketAddr::V4, with no visible IPv6 arm in the provided body.
* C source: `picoquic/util.c:413-429`
* C signature: `size_t picoquic_hash_addr_bytes(const struct sockaddr *, uint8_t *)`
* Rust source: `rs/fq/src/utils.rs:376-386`
* Rust item: `hash_addr_bytes`

### C body
```c
{
    size_t l = 0;
    if (addr->sa_family == AF_INET) {
        memcpy(bytes, &((struct sockaddr_in*)addr)->sin_addr, 4);
        l += 4;
        memcpy(bytes + l, &((struct sockaddr_in*)addr)->sin_port, 2);
        l += 2;
    }
    else if (addr->sa_family == AF_INET6) {
        memcpy(bytes, &((struct sockaddr_in6*)addr)->sin6_addr, 16);
        l += 16;
        memcpy(bytes + l, &((struct sockaddr_in6*)addr)->sin6_port, 2);
        l += 2;
    }
    return l;
}
```

### Rust body
```rust
        SocketAddr::V4(a) => {
            let ip = a.ip().octets();
            bytes[l..l + 4].copy_from_slice(&ip);
            l += 4;
            let port = addr.port().to_ne_bytes();
            bytes[l..l + 2].copy_from_slice(&port);
            l += 2;
        }
```
