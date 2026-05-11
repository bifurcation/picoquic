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

## `picoquic/packet.c:picoquic_incoming_stateless_reset`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C marks stateless reset, invokes callback, disconnects, and returns AEAD check error; Rust is a generic incoming-packet loop with no reset handling visible.
* C source: `picoquic/packet.c:1813-1829`
* C signature: `int picoquic_incoming_stateless_reset(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3289-3338`
* Rust item: `incoming_packet_ex`

### C body
```c
{
    /* Stateless reset. The connection should be abandonned */
    if (cnx->cnx_state <= picoquic_state_ready) {
        cnx->remote_error = PICOQUIC_ERROR_STATELESS_RESET;
    }
    if (cnx->callback_fn) {
        (void)(cnx->callback_fn)(cnx, 0, NULL, 0, picoquic_callback_stateless_reset, cnx->callback_ctx, NULL);
    }
    picoquic_connection_disconnect(cnx);

    return PICOQUIC_ERROR_AEAD_CHECK;
}
```

### Rust body
```rust
    ) -> Result<Option<&mut Connection>, Error> {
        let packet_length = bytes.len();
        let mut consumed_index = 0usize;
        let mut previous_dest_id = ConnectionId::default();
        let mut first_cnx = None;

        while consumed_index < packet_length {
            let mut consumed = 0usize;
            let ret = self.incoming_segment(
                &mut bytes[consumed_index..],
                packet_length - consumed_index,
                packet_length,
                &mut consumed,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
                current_time,
                current_time,
                &mut previous_dest_id,
                &mut first_cnx,
            );

            if ret == 0 {
                consumed_index = consumed_index.saturating_add(consumed);
                if consumed == 0 {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(token) = first_cnx
            && let Some(cnx) = self.connections.get_mut(token)
            && packet_length > cnx.max_mtu_received
        {
            cnx.max_mtu_received = packet_length;
        }

        Ok(first_cnx.and_then(|token| self.connections.get_mut(token)))
    }
```

## `picoquic/packet.c:picoquic_queue_stateless_retry`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C constructs and queues a retry stateless packet with header, token, integrity/ODCID handling, addresses, interface, and log id; Rust only creates a stateless packet or returns.
* C source: `picoquic/packet.c:1144-1208`
* C signature: `void picoquic_queue_stateless_retry(picoquic_quic_t *, picoquic_packet_header *, picoquic_connection_id_t *, const struct sockaddr *, const struct sockaddr *, unsigned long, uint8_t *, size_t)`
* Rust source: `rs/fq/src/lib.rs:5976-5987`
* Rust item: `queue_stateless_retry`

### C body
```c
{
    picoquic_stateless_packet_t* sp = picoquic_create_stateless_packet(quic);
    void * integrity_aead = picoquic_find_retry_protection_context(quic, ph->version_index, 1);
    size_t checksum_length = (integrity_aead == NULL) ? 0 : picoquic_aead_get_checksum_length(integrity_aead);

    if (sp != NULL) {
        uint8_t* bytes = sp->bytes;
        size_t byte_index = 0;
        size_t header_length = 0;
        size_t pn_offset;
        size_t pn_length;

        byte_index = header_length = picoquic_create_long_header(
            picoquic_packet_retry,
            &ph->srce_cnx_id,
            s_cid,
            0 /* No grease bit here */,
            ph->vn,
            ph->version_index,
            0, /* Sequence number is not used */
            retry_token_length,
            retry_token,
            bytes,
            &pn_offset,
            &pn_length);

        /* Add the token to the payload. */
        if (byte_index + retry_token_length < PICOQUIC_MAX_PACKET_SIZE) {
            memcpy(bytes + byte_index, retry_token, retry_token_length);
            byte_index += retry_token_length;
        }

        /* In the old drafts, there is no header protection and the sender copies the ODCID
         * in the packet. In the recent draft, the ODCID is not sent but
         * is verified as part of integrity checksum */
        if (integrity_aead == NULL) {
            bytes[byte_index++] = ph->dest_cnx_id.id_len;
            byte_index += picoquic_format_connection_id(bytes + byte_index,
                PICOQUIC_MAX_PACKET_SIZE - byte_index - checksum_length, ph->dest_cnx_id);
        }
        else {
            /* Encode the retry integrity protection if required. */
            byte_index = picoquic_encode_retry_protection(integrity_aead, bytes, PICOQUIC_MAX_PACKET_SIZE, byte_index, &ph->dest_cnx_id);
        }

        sp->length = byte_index;

        sp->ptype = picoquic_packet_retry;

        picoquic_store_addr(&sp->addr_to, addr_from);
        picoquic_store_addr(&sp->addr_local, addr_to);
        sp->if_index_local = if_index_to;
        sp->cnxid_log64 = picoquic_val64_connection_id(ph->dest_cnx_id);

        picoquic_queue_stateless_packet(quic, sp);
    }
}
```

### Rust body
```rust
        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };
```

## `picoquic/performance_log.c:picoquic_perflog`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a test function calling tls_api_test_with_loss; it does not record a perflog item, delete/free context, clear callbacks, or return ret.
* C source: `picoquic/performance_log.c:225-241`
* C signature: `int picoquic_perflog(picoquic_quic_t *, picoquic_cnx_t *, int)`
* Rust source: `rs/fq/src/tests/tls_api.rs:885-887`
* Rust item: `perflog`

### C body
```c
{
    int ret = 0;
    picoquic_performance_log_ctx_t* perflog_ctx = (picoquic_performance_log_ctx_t*)quic->v_perflog_ctx;

    if (cnx != NULL) {
        ret = picoquic_perflog_record(cnx, perflog_ctx);
    }

    if (should_delete) {
        picoquic_perflog_free(perflog_ctx);
        quic->v_perflog_ctx = NULL;
        quic->perflog_fn = NULL;
    }

    return ret;
}
```

### Rust body
```rust
fn perflog() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("perflog");
}
```

## `picoquic/picoquic_ptls_minicrypto.c:picoquic_clear_minicrypto`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body is intentionally empty, while the Rust body shown is an unrelated IP-address parsing helper.
* C source: `picoquic/picoquic_ptls_minicrypto.c:43-46`
* C signature: `void picoquic_clear_minicrypto(void)`
* Rust source: `rs/fq/src/tls_api.rs:2629-2647`
* Rust item: `clear_minicrypto`

### C body
```c
{
    /* Nothing for now */
}
```

### Rust body
```rust
fn ip_addr_from_stored_bytes(bytes: &[u8]) -> Option<core::net::IpAddr> {
    match bytes.len() {
        0 => Some(core::net::IpAddr::V4(core::net::Ipv4Addr::UNSPECIFIED)),
        4 => Some(core::net::IpAddr::V4(core::net::Ipv4Addr::new(
            bytes[0], bytes[1], bytes[2], bytes[3],
        ))),
        16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(bytes);
            Some(core::net::IpAddr::V6(core::net::Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}
```

## `picoquic/picosocks.c:picoquic_sendmsg`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C builds and sends a message, reports bytes sent and socket errors; Rust always returns Err(Error::Generic).
* C source: `picoquic/picosocks.c:1055-1126`
* C signature: `int picoquic_sendmsg(int, struct sockaddr *, struct sockaddr *, int, const char *, int, int, int *)`
* Rust source: `rs/fq/src/socks.rs:85-101`
* Rust item: `send`

### C body
```c
{
    GUID WSASendMsg_GUID = WSAID_WSASENDMSG;
    LPFN_WSASENDMSG WSASendMsg;
    char cmsg_buffer[1024];
    DWORD NumberOfBytes;
    int ret = 0;
    DWORD dwBytesSent = 0;
    WSAMSG msg;
    WSABUF dataBuf;
    int bytes_sent;
    int last_error = 0;

    ret = WSAIoctl(fd, SIO_GET_EXTENSION_FUNCTION_POINTER,
        &WSASendMsg_GUID, sizeof WSASendMsg_GUID,
        &WSASendMsg, sizeof WSASendMsg,
        &NumberOfBytes, NULL, NULL);

    if (ret == SOCKET_ERROR) {
        last_error = WSAGetLastError();
        DBG_PRINTF("Could not initialize WSASendMsg on UDP socket %d= %d!\n",
            (int)fd, last_error);
        bytes_sent = -1;
    }
    else {
        /* Format the control message header */
        memset(&msg, 0, sizeof(msg));
        msg.name = addr_dest;
        msg.namelen = picoquic_addr_length(addr_dest);
        dataBuf.buf = (char*)bytes;
        dataBuf.len = length;
        msg.lpBuffers = &dataBuf;
        msg.dwBufferCount = 1;
        msg.Control.buf = (char*)cmsg_buffer;
        msg.Control.len = sizeof(cmsg_buffer);

        /* Format the control message */
        picoquic_socks_cmsg_format(&msg, length, send_msg_size, addr_from, dest_if);

        /* Send the message */
        ret = WSASendMsg(fd, &msg, 0, &dwBytesSent, NULL, NULL);

        if (ret != 0) {
            bytes_sent = -1;
        } else {
            bytes_sent = (int)dwBytesSent;
        }

        if (bytes_sent <= 0) {
            last_error = WSAGetLastError();

#ifndef DISABLE_DEBUG_PRINTF
            DBG_PRINTF("Could not send packet on UDP socket[AF=%d]= %d!\n",
                addr_dest->sa_family, last_error);
#endif
        }
    }

    if (sock_err != NULL) {
        *sock_err = last_error;
    }


    return bytes_sent;
}
```

### Rust body
```rust
    {
        Err(Error::Generic)
    }
```
