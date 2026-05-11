# Phase 4C body-only translation audit

Compare each C/Rust pair using only the function bodies shown
below. Do not infer from dependencies, type definitions, callers,
module context, tests, or external knowledge. This is a cheap
superficial check for obvious inconsistencies.

Return only JSON with this shape:

```json
{"reviews":[{"c_id":"...","status":"ok|suspect|definitely_not_ok","rationale":"body-visible reason"}]}
```

Status meanings:
* `ok`: no obvious body-level concern.
* `suspect`: possible mismatch visible from the bodies.
* `definitely_not_ok`: clear mismatch or placeholder-like code.

## Pair `picoquic/picosocks.c:picoquic_bind_to_port`
C: `picoquic/picosocks.c:25-50 picoquic_bind_to_port`
Rust: `rs/fq/src/socks.rs:105-107 bind_to_port`

### C body
```c
{
    struct sockaddr_storage sa;
    int addr_length = 0;

    memset(&sa, 0, sizeof(sa));

    if (af == AF_INET) {
        struct sockaddr_in* s4 = (struct sockaddr_in*)&sa;
#ifdef _WINDOWS
        s4->sin_family = (ADDRESS_FAMILY)af;
#else
        s4->sin_family = af;
#endif
        s4->sin_port = htons((unsigned short)port);
        addr_length = sizeof(struct sockaddr_in);
    } else {
        struct sockaddr_in6* s6 = (struct sockaddr_in6*)&sa;

        s6->sin6_family = AF_INET6;
        s6->sin6_port = htons((unsigned short)port);
        addr_length = sizeof(struct sockaddr_in6);
    }

    return bind(fd, (struct sockaddr*)&sa, addr_length);
}
```

### Rust body
```rust
    fn bind_to_port(&mut self, _af: i32, _port: i32) -> Result<(), Error> {
        Err(Error::Generic)
    }
```

## Pair `picoquic/picosocks.c:picoquic_socket_set_ecn_options`
C: `picoquic/picosocks.c:225-228 picoquic_socket_set_ecn_options`
Rust: `rs/fq/src/socks.rs:511-513 picoquic_socket_set_ecn_options`

### C body
```c
{
    return picoquic_socket_set_ecn_options_ex(sd, af, recv_set, send_set, PICOQUIC_ECN_ECT_1);
}
```

### Rust body
```rust
pub fn picoquic_socket_set_ecn_options(sd: i32, af: i32) -> Result<(bool, bool), Error> {
    picoquic_socket_set_ecn_options_ex(sd, af, EcnCodepoint::Ect1)
}
```

## Pair `picoquic/picosocks.c:picoquic_close_server_sockets`
C: `picoquic/picosocks.c:327-335 picoquic_close_server_sockets`
Rust: `rs/fq/src/socks.rs:251-255 close`

### C body
```c
{
    for (int i = 0; i < PICOQUIC_NB_SERVER_SOCKETS; i++) {
        if (sockets->s_socket[i] != INVALID_SOCKET) {
            SOCKET_CLOSE(sockets->s_socket[i]);
            sockets->s_socket[i] = INVALID_SOCKET;
        }
    }
}
```

### Rust body
```rust
    pub fn close(&mut self) {
        for slot in &mut self.sockets {
            *slot = None;
        }
    }
```

## Pair `picoquic/picosocks.c:picoquic_recvmsg`
C: `picoquic/picosocks.c:957-1020 picoquic_recvmsg`
Rust: `rs/fq/src/socks.rs:78-101 recv`

### C body
```c
{
    GUID WSARecvMsg_GUID = WSAID_WSARECVMSG;
    LPFN_WSARECVMSG WSARecvMsg;
    char cmsg_buffer[1024];
    DWORD NumberOfBytes;
    int nResult;
    WSAMSG msg;
    WSABUF dataBuf;
    int recv_ret = 0;
    int bytes_recv;
    int last_error;

    if (dest_if != NULL) {
        *dest_if = 0;
    }

    if (received_ecn != NULL) {
        *received_ecn = 0;
    }

    nResult = WSAIoctl(fd, SIO_GET_EXTENSION_FUNCTION_POINTER,
        &WSARecvMsg_GUID, sizeof WSARecvMsg_GUID,
        &WSARecvMsg, sizeof WSARecvMsg,
        &NumberOfBytes, NULL, NULL);

    if (nResult == SOCKET_ERROR) {
        last_error = WSAGetLastError();
        DBG_PRINTF("Could not initialize WSARecvMsg) on UDP socket %d= %d!\n",
            (int)fd, last_error);
        bytes_recv = -1;
    } else {
        dataBuf.buf = (char*)buffer;
        dataBuf.len = buffer_max;

        msg.name = (struct sockaddr*)addr_from;
        msg.namelen = sizeof(struct sockaddr_storage);
        msg.lpBuffers = &dataBuf;
        msg.dwBufferCount = 1;
        msg.dwFlags = 0;
        msg.Control.buf = cmsg_buffer;
        msg.Control.len = sizeof(cmsg_buffer);

        recv_ret = WSARecvMsg(fd, &msg, &NumberOfBytes, NULL, NULL);

        if (recv_ret != 0) {
            last_error = WSAGetLastError();
            DBG_PRINTF("Could not receive message (WSARecvMsg) on UDP socket %d = %d!\n",
                (int)fd, last_error);
            bytes_recv = -1;
        } else {
            bytes_recv = NumberOfBytes;
            picoquic_socks_cmsg_parse(&msg, addr_dest, dest_if, received_ecn, NULL);
        }
    }

    return bytes_recv;
}
```

### Rust body
```rust
    {
        Err(Error::Generic)
    }
```

## Pair `picoquic/picosocks.c:picoquic_send_through_socket`
C: `picoquic/picosocks.c:1262-1271 picoquic_send_through_socket`
Rust: `rs/fq/src/socks.rs:194-212 picoquic_send_through_socket`

### C body
```c
{
    int sent = picoquic_sendmsg(fd, addr_dest, addr_from, from_if, bytes, length, 0, sock_err);

    return sent;
}
```

### Rust body
```rust
) -> Result<usize, OsError> {
    match socket.send(addr_dest, addr_from, from_if, bytes, 0) {
        Ok(sent) => {
            *sock_err = None;
            Ok(sent)
        }
        Err(err) => {
            *sock_err = Some(err);
            Err(err)
        }
    }
}
```

## Pair `picoquic/picosocks.c:picoquic_socket_error_implies_unreachable`
C: `picoquic/picosocks.c:1402-1421 picoquic_socket_error_implies_unreachable`
Rust: `rs/fq/src/socks.rs:46-56 is_unreachable`

### C body
```c
{
#ifdef _WINDOWS
    static int unreachable_errors[] = {
        WSAEACCES, WSAEADDRNOTAVAIL, WSAEAFNOSUPPORT, WSAECONNRESET,
        WSAEDESTADDRREQ, WSAEHOSTUNREACH, WSAENETDOWN, WSAENETRESET,
        WSAENETUNREACH, WSAESHUTDOWN, -1 };
#else
    static int unreachable_errors[] = {
        EAFNOSUPPORT, ECONNRESET, EHOSTUNREACH, ENETDOWN, ENETUNREACH, -1 };
#endif
    size_t nb_errors = sizeof(unreachable_errors) / sizeof(int);
    int ret = 0;

    for (size_t i = 0; ret == 0 && i < nb_errors; i++) {
        ret = (sock_err == unreachable_errors[i]);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn is_unreachable(self) -> bool {
        use std::io::ErrorKind;
        let kind = std::io::Error::from_raw_os_error(self.0).kind();
        matches!(
            kind,
            ErrorKind::ConnectionReset
                | ErrorKind::HostUnreachable
                | ErrorKind::NetworkDown
                | ErrorKind::NetworkUnreachable
        )
    }
```

## Pair `picoquic/picosplay.c:zigzag`
C: `picoquic/picosplay.c:72-78 zigzag`
Rust: `rs/fq/src/splay.rs:358-361 zigzag`

### C body
```c
static void zigzag(picosplay_node_t *x) {
    rotate(x);
    rotate(x);
}
```

### Rust body
```rust
    fn zigzag(&mut self, x: u32) {
        self.rotate(x);
        self.rotate(x);
    }
```

## Pair `picoquic/picosplay.c:picosplay_find`
C: `picoquic/picosplay.c:140-162 picosplay_find`
Rust: `rs/fq/src/splay.rs:460-476 find`

### C body
```c
{
    picosplay_node_t *curr = tree->root;
    int found = 0;
    while(curr != NULL && !found) {
        int64_t relation = tree->comp(value, tree->node_value(curr));
        if(relation == 0) {
            found = 1;
        } else if(relation < 0) {
            curr = curr->left;
        } else {
            curr = curr->right;
        }
    }

    /* TODO: there may or may not be a need to perform a splay on a find operation.
     * The Wikipedia example omits it, but this code keeps it. We should
     * perform measurements with and without it and keep the best alternative. */
    if(curr != NULL)
        splay(tree, curr);
    return curr;
}
```

### Rust body
```rust
            match key.cmp(self.key_of(cur)) {
                core::cmp::Ordering::Equal => {
                    self.splay(cur);
                    return Some(self.token_of(cur));
                }
                core::cmp::Ordering::Less => match self.left_of(cur) {
                    None => return None,
                    Some(l) => cur = l,
                },
                core::cmp::Ordering::Greater => match self.right_of(cur) {
                    None => return None,
                    Some(r) => cur = r,
                },
            }
```

## Pair `picoquic/picosplay.c:picosplay_empty_tree`
C: `picoquic/picosplay.c:224-231 picosplay_empty_tree`
Rust: `rs/fq/src/splay.rs:660-665 clear`

### C body
```c
{
    if (tree != NULL) {
        while (tree->root != NULL) {
            picosplay_delete_hint(tree, tree->root);
        }
    }
}
```

### Rust body
```rust
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free = None;
        self.root = None;
        self.len = 0;
    }
```

## Pair `picoquic/picosplay.c:picosplay_last`
C: `picoquic/picosplay.c:258-260 picosplay_last`
Rust: `rs/fq/src/splay.rs:514-541 last`

### C body
```c
picosplay_node_t* picosplay_last(picosplay_tree_t *tree) {
    return rightmost(tree->root);
}
```

### Rust body
```rust
    pub fn previous(&self, token: SplayToken) -> Option<SplayToken> {
        if !self.is_valid(token) {
            return None;
        }
        let idx = token.idx;
        if let Some(l) = self.left_of(idx) {
            return self.rightmost(Some(l)).map(|p| self.token_of(p));
        }
        let mut node = idx;
        loop {
            match self.parent_of(node) {
                None => return None,
                Some(p) => {
                    if self.left_of(p) == Some(node) {
                        node = p;
                    } else {
                        return Some(self.token_of(p));
                    }
                }
            }
        }
    }
```

## Pair `picoquic/picosplay.c:rightmost`
C: `picoquic/picosplay.c:329-336 rightmost`
Rust: `rs/fq/src/splay.rs:397-403 rightmost`

### C body
```c
static picosplay_node_t* rightmost(picosplay_node_t *node) {
    picosplay_node_t *parent = NULL;
    while(node != NULL) {
        parent = node;
        node = node->right;
    }
    return parent;
}
```

### Rust body
```rust
            match self.right_of(cur) {
                None => return Some(cur),
                Some(r) => cur = r,
            }
```

## Pair `picoquic/prague.c:picoquic_prague_init_reno`
C: `picoquic/prague.c:118-124 picoquic_prague_init_reno`
Rust: `rs/fq/src/prague.rs:87-92 prague_init_reno`

### C body
```c
{
    pr_state->alg_state = picoquic_prague_alg_slow_start;
    pr_state->ssthresh = UINT64_MAX;
    pr_state->alpha = 0;
    path_x->cwin = PICOQUIC_CWIN_INITIAL;
}
```

### Rust body
```rust
pub(crate) fn prague_init_reno(pr_state: &mut PragueState, path_x: &mut Path) {
    pr_state.alg_state = PragueAlgState::SlowStart;
    pr_state.ssthresh = u64::MAX;
    pr_state.alpha = 0;
    path_x.cwin = CWIN_INITIAL;
}
```

## Pair `picoquic/prague.c:picoquic_prague_reset`
C: `picoquic/prague.c:166-170 picoquic_prague_reset`
Rust: `rs/fq/src/prague.rs:108-111 prague_reset`

### C body
```c
{
    picoquic_prague_init_reno(pr_state, path_x);
    picoquic_prague_reset_l3s(cnx, pr_state, path_x);
}
```

### Rust body
```rust
fn prague_reset(cnx: &Connection, pr_state: &mut PragueState, path_x: &mut Path) {
    prague_init_reno(pr_state, path_x);
    prague_reset_l3s(cnx, pr_state, path_x);
}
```

## Pair `picoquic/prague.c:picoquic_prague_process_ack`
C: `picoquic/prague.c:251-292 picoquic_prague_process_ack`
Rust: `rs/fq/src/prague.rs:211-221 picoquic_prague_process_ack`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = picoquic_prague_get_pkt_ctx(cnx, path_x);
    uint64_t next_sequence = picoquic_cc_get_ack_number(path_x->cnx, path_x);

    if (next_sequence > pr_state->recovery_sequence) {
        /* new period. Update alpha, etc. */
        int64_t delta_ect1 = pkt_ctx->ecn_ect1_total_remote - pr_state->l4s_epoch_ect1;
        int64_t delta_ce = pkt_ctx->ecn_ce_total_remote - pr_state->l4s_epoch_ce;

        if (delta_ect1 >= 0 && delta_ce >= 0 && delta_ce + delta_ect1 > 0 ) {
            /* We are receiving ECN signals, so update alpha and do CWND reduction */
            uint64_t delta_cwin;
            picoquic_prague_update_alpha(path_x, pr_state, delta_ect1, delta_ce, current_time);

            /* Update the ssthresh and the CWIN */
            delta_cwin = (path_x->cwin * pr_state->alpha) / 2048;
            path_x->cwin -= delta_cwin;
            if (path_x->cwin < PICOQUIC_CWIN_MINIMUM) {
                path_x->cwin = PICOQUIC_CWIN_MINIMUM;
            }
            pr_state->ssthresh = path_x->cwin;
        }
        /* reset the era limits */
        picoquic_prague_initialize_era(cnx, path_x, pr_state, current_time);
    }
    /* Increment CWND whether in recovery or not */
    if (pkt_ctx->ecn_ect1_total_remote >= pr_state->l4s_packet_ect1 &&
        pkt_ctx->ecn_ce_total_remote >= pr_state->l4s_packet_ce) {
        uint64_t delta_ect1_ack = pkt_ctx->ecn_ect1_total_remote - pr_state->l4s_packet_ect1;
        uint64_t delta_ce_ack = pkt_ctx->ecn_ce_total_remote - pr_state->l4s_packet_ce;
        uint64_t ack_bytes = ack_state->nb_bytes_acknowledged;
        double frac_not_ce = 1.0;

        if (delta_ce_ack + delta_ect1_ack > 0) {
            frac_not_ce = ((double)delta_ect1_ack) / (double)(delta_ce_ack + delta_ect1_ack);
            ack_bytes = (uint64_t)(frac_not_ce * (double)ack_bytes);
        }
        path_x->cwin += path_x->send_mtu * ack_bytes / path_x->cwin;
    }
}
```

### Rust body
```rust
    let (ecn_ect1_total_remote, ecn_ce_total_remote) = {
        let pkt_ctx = prague_get_pkt_ctx(cnx, path_x);
        (pkt_ctx.ecn_ect1_total_remote, pkt_ctx.ecn_ce_total_remote)
    };
```

## Pair `picoquic/prague.c:picoquic_prague_observe`
C: `picoquic/prague.c:407-412 picoquic_prague_observe`
Rust: `rs/fq/src/prague.rs:395-403 observe`

### C body
```c
{
    picoquic_prague_state_t* pr_state = (picoquic_prague_state_t*)path_x->congestion_alg_state;
    *cc_state = (uint64_t)pr_state->alg_state;
    *cc_param = (pr_state->ssthresh == UINT64_MAX) ? 0 : pr_state->ssthresh;
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        let cc_state = self.alg_state as u64;
        let cc_param = if self.ssthresh == u64::MAX {
            0
        } else {
            self.ssthresh
        };
        (cc_state, cc_param)
    }
```

## Pair `picoquic/quicctx.c:picoquic_remember_issued_ticket`
C: `picoquic/quicctx.c:485-524 picoquic_remember_issued_ticket`
Rust: `rs/fq/src/internal.rs:1843-1865 remember_issued_ticket`

### C body
```c
{
    int ret = 0;

    picoquic_issued_ticket_t* ticket = picoquic_retrieve_issued_ticket(quic,
        ticket_id);
    if (ticket != NULL) {
        picoquic_update_issued_ticket(ticket, rtt, cwin, ip_addr, ip_addr_length);
    }
    else {
        while (quic->table_issued_tickets_nb > quic->max_number_connections) {
            picoquic_delete_issued_ticket(quic, quic->table_issued_tickets_last);
        }
        ticket = (picoquic_issued_ticket_t*)malloc(sizeof(picoquic_issued_ticket_t));
        if (ticket != NULL) {
            memset(ticket, 0, sizeof(picoquic_issued_ticket_t));
            ticket->ticket_id = ticket_id;
            picoquic_update_issued_ticket(ticket, rtt, cwin, ip_addr, ip_addr_length);
            ticket->next_ticket = quic->table_issued_tickets_first;
            quic->table_issued_tickets_first = ticket;
            if (ticket->next_ticket == NULL) {
                quic->table_issued_tickets_last = ticket;
            }
            else {
                ticket->next_ticket->previous_ticket = ticket;
            }
            picohash_insert(quic->table_issued_tickets, ticket);
        }
        else {
            ret = PICOQUIC_ERROR_MEMORY;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let ticket = IssuedTicket {
            issued_tickets_membership: None,
            ticket_id,
            creation_time: crate::Instant::from_ticks(0),
            rtt,
            cwin,
            ip_addr,
        };
        let tok = self.issued_tickets.insert(ticket)?;
        let (htok, _) = self.issued_tickets_by_id.insert(ticket_id, tok)?;
        // Store the hash token back into the ticket so we can do O(1) removal later.
        if let Some(entry) = self.issued_tickets.get_mut(tok) {
            entry.issued_tickets_membership = Some(htok);
        }
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_adjust_max_connections`
C: `picoquic/quicctx.c:612-622 picoquic_adjust_max_connections`
Rust: `rs/fq/src/lib.rs:1164-1170 adjust_max_connections`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);

    if (max_nb_connections <= quic->max_number_connections) {
        quic->tentative_max_number_connections = max_nb_connections;
        return 0;
    }

    return -1;
}
```

### Rust body
```rust
    pub fn adjust_max_connections(&mut self, max_nb_connections: u32) -> Result<(), Error> {
        if max_nb_connections > self.max_number_connections {
            return Err(Error::InvalidArgument);
        }
        self.tentative_max_number_connections = max_nb_connections;
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_tp`
C: `picoquic/quicctx.c:798-811 picoquic_set_default_tp`
Rust: `rs/fq/src/lib.rs:1573-1576 set_default_tp`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(quic);

    if (tp == NULL) {
        picoquic_init_transport_parameters(&quic->default_tp);
    }
    else {
        memcpy(&quic->default_tp, tp, sizeof(picoquic_tp_t));
    }

    return ret;
}
```

### Rust body
```rust
    pub fn set_default_tp(&mut self, tp: &TransportParameters) -> Result<(), Error> {
        self.default_tp = tp.clone();
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_padding`
C: `picoquic/quicctx.c:899-904 picoquic_set_default_padding`
Rust: `rs/fq/src/lib.rs:1671-1687 set_default_padding`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->padding_minsize_default = padding_minsize;
    quic->padding_multiple_default = padding_multiple;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // SpinbitVersion::On is server-only and must not be set as a default.
        if default_spinbit_policy == SpinbitVersion::On {
            return Err(Error::InvalidArgument);
        }
        self.default_spin_policy = default_spinbit_policy;
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_multipath_option`
C: `picoquic/quicctx.c:942-950 picoquic_set_default_multipath_option`
Rust: `rs/fq/src/lib.rs:1695-1697 set_default_multipath_option`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_multipath_option = multipath_option;

    if (multipath_option & 1) {
        quic->default_tp.initial_max_path_id = 2;
    }
}
```

### Rust body
```rust
    pub fn set_default_multipath_option(&mut self, multipath_option: i32) {
        self.default_multipath_option = multipath_option as u32;
    }
```
