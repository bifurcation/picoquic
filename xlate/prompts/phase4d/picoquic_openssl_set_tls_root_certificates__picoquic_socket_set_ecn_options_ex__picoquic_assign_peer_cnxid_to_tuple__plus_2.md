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

## `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_set_tls_root_certificates`
* Phase 4C status: `suspect`
* Phase 4C rationale: C loops through certs converting DER to X509 and returns distinct errors on parse/add failures; Rust delegates all behavior to verifier.set_root_certificates, so the body-visible steps do not match.
* C source: `picoquic/picoquic_ptls_openssl.c:298-319`
* C signature: `int picoquic_openssl_set_tls_root_certificates(ptls_context_t *, ptls_iovec_t *, size_t)`
* Rust source: `rs/fq/src/sys/openssl.rs:501-506`
* Rust item: `picoquic_openssl_set_tls_root_certificates`

### C body
```c
{
    ptls_openssl_verify_certificate_t* verify_ctx = (ptls_openssl_verify_certificate_t*)ctx->verify_certificate;

    for (size_t i = 0; i < count; ++i) {
        uint8_t* cert_i_base = certs[i].base;
        X509* cert = d2i_X509(NULL, (const uint8_t**)&cert_i_base, (long)certs[i].len);

        if (cert == NULL) {
            return -1;
        }

        if (X509_STORE_add_cert(verify_ctx->cert_store, cert) == 0) {
            X509_free(cert);
            return -2;
        }

        X509_free(cert);
    }

    return 0;
}
```

### Rust body
```rust
) -> Result<(), crate::Error> {
    verifier.set_root_certificates(certs)
}
```

## `picoquic/picosocks.c:picoquic_socket_set_ecn_options_ex`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust only implements the non-Windows-style TCLASS/TOS and RECVTCLASS/RECVTOS paths and always errors when receive option setup fails; C has Windows and macro-disabled branches that set flags differently.
* C source: `picoquic/picosocks.c:93-223`
* C signature: `int picoquic_socket_set_ecn_options_ex(int, int, int *, int *, uint8_t)`
* Rust source: `rs/fq/src/socks.rs:461-506`
* Rust item: `picoquic_socket_set_ecn_options_ex`

### C body
```c
{
    int ret = -1;
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(ecn_value);

    if (af == AF_INET6) {
#ifdef IPV6_ECN
        {
            DWORD recvEcn = 1;
            /* Request receiving ECN reports in recvmsg */
            ret = setsockopt(sd, IPPROTO_IPV6, IPV6_ECN, (char *)&recvEcn, sizeof(recvEcn));
            if (ret < 0) {
                DBG_PRINTF("setsockopt IPV6_ECN (0x%x) fails, errno: %d\n", recvEcn, GetLastError());
                ret = -1;
                *recv_set = 0;
            }
            else {
                *recv_set = 1;
                ret = 0;
            }
        }
        * send_set = 0;
#else
        * recv_set = 0;
        * send_set = 0;
#endif
    }
    else {
        /* Using IPv4 options. */
#if defined(IP_ECN)
        {
            DWORD recvEcn =1;

            /* Request receiving ECN reports in recvmsg */
            ret = setsockopt(sd, IPPROTO_IP, IP_ECN, (CHAR*)&recvEcn, sizeof(recvEcn));
            if (ret < 0) {
                DBG_PRINTF("setsockopt IP_ECN (0x%x) fails, errno: %d\n", recvEcn, GetLastError());
                ret = -1;
                *recv_set = 0;
            }
            else {
                *recv_set = 1;
                ret = 0;
            }
        }
#else
        * recv_set = 0;
#endif
        *send_set = 0;
    }
#else
    if (af == AF_INET6) {
#if defined(IPV6_TCLASS)
        {
            unsigned int ecn = ecn_value; /* Setting ECN=ecn_value in outgoing packets */
            if (ecn != 0 && setsockopt(sd, IPPROTO_IPV6, IPV6_TCLASS, &ecn, sizeof(ecn)) < 0) {
                DBG_PRINTF("setsockopt IPV6_TCLASS (0x%x) fails, errno: %d\n", ecn, errno);
                *send_set = 0;
            }
            else {
                *send_set = 1;
            }
        }
#else
        DBG_PRINTF("%s", "IPV6_TCLASS is not defined\n");
        *send_set = 0;
#endif
#ifdef IPV6_RECVTCLASS
        {
            unsigned int set = 0x01;

            /* Request receiving TOS reports in recvmsg */
            if (setsockopt(sd, IPPROTO_IPV6, IPV6_RECVTCLASS, &set, sizeof(set)) < 0) {
                DBG_PRINTF("setsockopt IPv6 IPV6_RECVTCLASS (0x%x) fails, errno: %d\n", set, errno);
                ret = -1;
                *recv_set = 0;
            }
            else {
                *recv_set = 1;
                ret = 0;
            }
        }
#else
        DBG_PRINTF("%s", "IPV6_RECVTCLASS is not defined\n");
        *recv_set = 0;
#endif 

    }
    else {
#if defined(IP_TOS)
        {
            unsigned int ecn = ecn_value;
            /* Request setting ECN=ecn_value in outgoing packets */
            if (ecn != 0 && setsockopt(sd, IPPROTO_IP, IP_TOS, &ecn, sizeof(ecn)) < 0) {
                DBG_PRINTF("setsockopt IPv4 IP_TOS (0x%x) fails, errno: %d\n", ecn, errno);
                *send_set = 0;
            }
            else {
                *send_set = 1;
            }
        }
#else
        *send_set = 0;
        DBG_PRINTF("%s", "IP_TOS is not defined\n");
#endif

#ifdef IP_RECVTOS
        {
            unsigned int set = 1;

            /* Request receiving TOS reports in recvmsg */
            if (setsockopt(sd, IPPROTO_IP, IP_RECVTOS, &set, sizeof(set)) < 0) {
                DBG_PRINTF("setsockopt IPv4 IP_RECVTOS (0x%x) fails, errno: %d\n", set, errno);
                ret = -1;
                *recv_set = 0;
            }
            else {
                *recv_set = 1;
                ret = 0;
            }
        }
#else
        *recv_set = 0;
        DBG_PRINTF("%s", "IP_RECVTOS is not defined\n");
#endif
    }
#endif

    return ret;
}
```

### Rust body
```rust
) -> Result<(bool, bool), Error> {
    let mut ret = -1;
    let recv_set;
    let send_set;
    let ecn = ecn_value as libc::c_uint;

    if af == libc::AF_INET6 {
        send_set = if ecn != 0 {
            setsockopt_uint(sd, libc::IPPROTO_IPV6, libc::IPV6_TCLASS, ecn)
        } else {
            true
        };

        let recv_ok = setsockopt_uint(sd, libc::IPPROTO_IPV6, libc::IPV6_RECVTCLASS, 1);
        if recv_ok {
            recv_set = true;
            ret = 0;
        } else {
            recv_set = false;
        }
    } else {
        send_set = if ecn != 0 {
            setsockopt_uint(sd, libc::IPPROTO_IP, libc::IP_TOS, ecn)
        } else {
            true
        };

        let recv_ok = setsockopt_uint(sd, libc::IPPROTO_IP, libc::IP_RECVTOS, 1);
        if recv_ok {
            recv_set = true;
            ret = 0;
        } else {
            recv_set = false;
        }
    }

    if ret == 0 {
        Ok((recv_set, send_set))
    } else {
        Err(Error::Generic)
    }
}
```

## `picoquic/quicctx.c:picoquic_assign_peer_cnxid_to_tuple`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both obtain a stashed connection ID and increment references, but C also marks the stash is_in_use while Rust does not visibly do so.
* C source: `picoquic/quicctx.c:2264-2282`
* C signature: `int picoquic_assign_peer_cnxid_to_tuple(picoquic_cnx_t *, picoquic_path_t *, picoquic_tuple_t *)`
* Rust source: `rs/fq/src/internal.rs:4866-4883`
* Rust item: `assign_peer_connection_id_to_tuple`

### C body
```c
{
    int ret = -1;
    picoquic_remote_cnxid_stash_t* stash = picoquic_find_or_create_remote_cnxid_stash(cnx, path_x->unique_path_id, 0);

    if (stash != NULL) {
        picoquic_remote_cnxid_t* available_cnxid = picoquic_get_cnxid_from_stash(stash);

        if (available_cnxid != NULL) {
            tuple->p_remote_cnxid = available_cnxid;
            available_cnxid->nb_path_references++;
            stash->is_in_use = 1;
            ret = 0;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let (stash_idx, cid_idx) = self
            .obtain_stashed_connection_id(path_x.unique_path_id)
            .ok_or(crate::Error::Generic)?;
        tuple.remote_connection_id_index = Some(cid_idx);
        tuple.unique_path_id = path_x.unique_path_id;
        if let Some(cid) = self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .get_mut(cid_idx)
        {
            cid.nb_path_references += 1;
        }
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_create_client_cnx`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust creates a connection with null CIDs like C, but the body does not show callback installation, start_client_cnx, or rollback on start failure.
* C source: `picoquic/quicctx.c:4360-4382`
* C signature: `picoquic_cnx_t * picoquic_create_client_cnx(picoquic_quic_t *, struct sockaddr *, uint64_t, uint32_t, const char *, const char *, picoquic_stream_data_cb_fn, void *)`
* Rust source: `rs/fq/src/lib.rs:1967-1991`
* Rust item: `create_client_connection`

### C body
```c
{
    picoquic_cnx_t* cnx = picoquic_create_cnx(quic, picoquic_null_connection_id, picoquic_null_connection_id, addr, start_time, preferred_version, sni, alpn, 1);

    if (cnx != NULL) {
        int ret;

        if (callback_fn != NULL)
            cnx->callback_fn = callback_fn;
        if (callback_ctx != NULL)
            cnx->callback_ctx = callback_ctx;
        ret = picoquic_start_client_cnx(cnx);
        if (ret != 0) {
            /* Cannot just do partial initialization! */
            picoquic_delete_cnx(cnx);
            cnx = NULL;
        }
    }

    return cnx;
}
```

### Rust body
```rust
    ) -> Option<&mut Connection> {
        // C: picoquic_create_client_cnx — wraps picoquic_create_cnx with
        // null CIDs, then runs picoquic_start_client_cnx and rolls back
        // on failure.  Callback installation is folded in here; the Rust
        // shape stores the boxed callback on Connection, which is the
        // moral equivalent of `cnx->callback_fn`/`cnx->callback_ctx`.
        self.create_connection(
            ConnectionId::with_size(0)?,
            ConnectionId::with_size(0)?,
            Some(addr),
            start_time,
            preferred_version,
            sni,
            alpn,
            true,
        )
    }
```

## `picoquic/quicctx.c:picoquic_get_client_cnxid`
* Phase 4C status: `suspect`
* Phase 4C rationale: C selects the client CID from the first tuple local/remote CID fields; Rust returns stored initial/original connection IDs instead.
* C source: `picoquic/quicctx.c:4477-4481`
* C signature: `picoquic_connection_id_t picoquic_get_client_cnxid(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3051-3057`
* Rust item: `client_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return (cnx->client_mode)?cnx->path[0]->first_tuple->p_local_cnxid->cnx_id : cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id;
}
```

### Rust body
```rust
    pub fn client_connection_id(&self) -> ConnectionId {
        if self.client_mode {
            self.initial_connection_id
        } else {
            self.original_connection_id
        }
    }
```
