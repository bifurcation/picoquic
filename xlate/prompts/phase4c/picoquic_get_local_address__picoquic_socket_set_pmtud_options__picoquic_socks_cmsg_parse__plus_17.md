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

## Pair `picoquic/picosocks.c:picoquic_get_local_address`
C: `picoquic/picosocks.c:52-56 picoquic_get_local_address`
Rust: `rs/fq/src/socks.rs:72-101 local_address`

### C body
```c
{
    socklen_t name_len = sizeof(struct sockaddr_storage);
    return getsockname(sd, (struct sockaddr *)addr, &name_len);
}
```

### Rust body
```rust
    {
        Err(Error::Generic)
    }
```

## Pair `picoquic/picosocks.c:picoquic_socket_set_pmtud_options`
C: `picoquic/picosocks.c:230-248 picoquic_socket_set_pmtud_options`
Rust: `rs/fq/src/socks.rs:157-159 set_pmtud_options`

### C body
```c
{
    int ret = 0;
#if defined __linux && defined(IP_MTU_DISCOVER) && defined(IPV6_MTU_DISCOVER) && defined(IP_PMTUDISC_PROBE)
    int val = IP_PMTUDISC_PROBE;
    if (af == AF_INET6) {
        ret = setsockopt(sd, IPPROTO_IPV6, IPV6_MTU_DISCOVER, &val, sizeof(int));
    }
    else {
        ret = setsockopt(sd, IPPROTO_IP, IP_MTU_DISCOVER, &val, sizeof(int));
    }
#else
#ifdef UNREFERENCED_PARAMETER
    UNREFERENCED_PARAMETER(af);
    UNREFERENCED_PARAMETER(sd);
#endif
#endif  /* #if defined __linux && ... */
    return ret;
}
```

### Rust body
```rust
    fn set_pmtud_options(&mut self) -> Result<(), Error> {
        Ok(())
    }
```

## Pair `picoquic/picosocks.c:picoquic_socks_cmsg_parse`
C: `picoquic/picosocks.c:422-497 picoquic_socks_cmsg_parse`
Rust: `rs/fq/src/socks.rs:534-554 parse_cmsg`

### C body
```c
{
    /* Assume that msg has been filled by a call to recvmsg */
    /* Get the control information */
    struct msghdr* msg = (struct msghdr*)vmsg;
    struct cmsghdr* cmsg;

    for (cmsg = CMSG_FIRSTHDR(msg); cmsg != NULL; cmsg = CMSG_NXTHDR(msg, cmsg)) {
        if (cmsg->cmsg_level == IPPROTO_IP) {
#ifdef IP_PKTINFO
            if (cmsg->cmsg_type == IP_PKTINFO) {
                if (addr_dest != NULL) {
                    struct in_pktinfo* pPktInfo = (struct in_pktinfo*)CMSG_DATA(cmsg);
                    ((struct sockaddr_in*)addr_dest)->sin_family = AF_INET;
                    ((struct sockaddr_in*)addr_dest)->sin_port = 0;
                    ((struct sockaddr_in*)addr_dest)->sin_addr.s_addr = pPktInfo->ipi_addr.s_addr;

                    if (dest_if != NULL) {
                        *dest_if = (int)pPktInfo->ipi_ifindex;
                    }
                }
            }
#else
            /* The IP_PKTINFO structure is not defined on BSD */
            if (cmsg->cmsg_type == IP_RECVDSTADDR) {
                if (addr_dest != NULL) {
                    struct in_addr* pPktInfo = (struct in_addr*)CMSG_DATA(cmsg);
                    ((struct sockaddr_in*)addr_dest)->sin_family = AF_INET;
                    ((struct sockaddr_in*)addr_dest)->sin_port = 0;
                    ((struct sockaddr_in*)addr_dest)->sin_addr.s_addr = pPktInfo->s_addr;

                    if (dest_if != NULL) {
                        *dest_if = 0;
                    }
                }
            }
#endif
            else if ((cmsg->cmsg_type == IP_TOS
#ifdef IP_RECVTOS
                || cmsg->cmsg_type == IP_RECVTOS
#endif
                ) && cmsg->cmsg_len > 0) {
                if (received_ecn != NULL) {
                    *received_ecn = *((unsigned char*)CMSG_DATA(cmsg));
                }
            }
        }
        else if (cmsg->cmsg_level == IPPROTO_IPV6) {
            if (cmsg->cmsg_type == IPV6_PKTINFO) {
                if (addr_dest != NULL) {
                    struct in6_pktinfo* pPktInfo6 = (struct in6_pktinfo*)CMSG_DATA(cmsg);

                    ((struct sockaddr_in6*)addr_dest)->sin6_family = AF_INET6;
                    ((struct sockaddr_in6*)addr_dest)->sin6_port = 0;
                    memcpy(&((struct sockaddr_in6*)addr_dest)->sin6_addr, &pPktInfo6->ipi6_addr, sizeof(struct in6_addr));

                    if (dest_if != NULL) {
                        *dest_if = (int)pPktInfo6->ipi6_ifindex;
                    }
                }
            }
            else if (cmsg->cmsg_type == IPV6_TCLASS) {
                if (cmsg->cmsg_len > 0 && received_ecn != NULL) {
                    *received_ecn = *((unsigned char*)CMSG_DATA(cmsg));
                }
            }
        }
    }
#endif
}
```

### Rust body
```rust
) {
    // Control-message formatting requires CMSG_ macros not available in
    // safe Rust through socket2 0.5.  The kernel will pick the source
    // address; GSO segmentation and ECN marking are not applied.
}
```

## Pair `picoquic/picosocks.c:picoquic_sendmsg`
C: `picoquic/picosocks.c:1055-1126 picoquic_sendmsg`
Rust: `rs/fq/src/socks.rs:85-101 send`

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

## Pair `picoquic/picosocks.c:picoquic_send_through_server_sockets`
C: `picoquic/picosocks.c:1273-1283 picoquic_send_through_server_sockets`
Rust: `rs/fq/src/socks.rs:260-267 send_through`

### C body
```c
{
    /* Both Linux and Windows use separate sockets for V4 and V6 */
    int socket_index = (addr_dest->sa_family == AF_INET) ? 1 : 0;

    return picoquic_send_through_socket(sockets->s_socket[socket_index], addr_dest, addr_from, from_if, bytes, length, sock_err);
}
```

### Rust body
```rust
        let idx = if addr_dest.is_ipv4() { 1 } else { 0 };
```

## Pair `picoquic/picosplay.c:splay`
C: `picoquic/picosplay.c:40-57 splay`
Rust: `rs/fq/src/splay.rs:363-371 splay`

### C body
```c
static void splay(picosplay_tree_t *tree, picosplay_node_t *x) {
    while(1) {
        picosplay_node_t *p = x->parent;
        if(p == NULL) {
            tree->root = x;
            return;
        }
        picosplay_node_t *g = p->parent;
        if(p->parent == NULL)
            zig(x);
        else
            if((x == p->left && p == g->left) ||
                    (x == p->right && p == g->right))
                zigzig(x, p);
            else
                zigzag(x);
    }
}
```

### Rust body
```rust
            let p = match self.parent_of(idx) {
                None => {
                    self.root = Some(idx);
                    return;
                }
                Some(p) => p,
            };
```

## Pair `picoquic/picosplay.c:picosplay_init_tree`
C: `picoquic/picosplay.c:80-88 picosplay_init_tree`
Rust: `rs/fq/src/splay.rs:149-163 new`

### C body
```c
void picosplay_init_tree(picosplay_tree_t* tree, picosplay_comparator comp, picosplay_create create, picosplay_delete_node delete_node, picosplay_node_value node_value) {
    tree->comp = comp;
    tree->create = create;
    tree->delete_node = delete_node;
    tree->node_value = node_value;
    tree->root = NULL;
    tree->size = 0;
}
```

### Rust body
```rust
    pub const fn new_tree() -> Self {
        Self::new()
    }
```

## Pair `picoquic/picosplay.c:picosplay_find_previous`
C: `picoquic/picosplay.c:164-186 picosplay_find_previous`
Rust: `rs/fq/src/splay.rs:483-504 find_previous`

### C body
```c
{
    picosplay_node_t* curr = tree->root;
    picosplay_node_t* previous = NULL;
    int found = 0;
    while (curr != NULL && !found) {
        int64_t relation = tree->comp(value, tree->node_value(curr));
        if (relation == 0) {
            found = 1;
            previous = curr;
        }
        else if (relation < 0) {
            curr = curr->left;
        }
        else {
            previous = curr;
            curr = curr->right;
        }
    }

    return previous;
}
```

### Rust body
```rust
    pub fn find_previous(&self, key: &K) -> Option<SplayToken> {
        let mut cur = self.root?;
        let mut prev: Option<u32> = None;
        loop {
            let cmp = key.cmp(self.key_of(cur));
            if cmp == core::cmp::Ordering::Equal {
                return Some(self.token_of(cur));
            } else if cmp == core::cmp::Ordering::Less {
                match self.left_of(cur) {
                    None => break,
                    Some(l) => cur = l,
                }
            } else {
                prev = Some(cur);
                match self.right_of(cur) {
                    None => break,
                    Some(r) => cur = r,
                }
            }
        }
        prev.map(|p| self.token_of(p))
    }
```

## Pair `picoquic/picosplay.c:picosplay_first`
C: `picoquic/picosplay.c:233-235 picosplay_first`
Rust: `rs/fq/src/splay.rs:508-516 first`

### C body
```c
picosplay_node_t* picosplay_first(picosplay_tree_t *tree) {
    return leftmost(tree->root);
}
```

### Rust body
```rust
    pub fn last(&self) -> Option<SplayToken> {
        self.rightmost(self.root).map(|i| self.token_of(i))
    }
```

## Pair `picoquic/picosplay.c:rotate`
C: `picoquic/picosplay.c:288-305 rotate`
Rust: `rs/fq/src/splay.rs:324-344 rotate`

### C body
```c
static void rotate(picosplay_node_t *child) {
    picosplay_node_t *parent = child->parent;
    assert(parent != NULL);
    if(parent->left == child) { /* A left child given. */
        mark_gp(child);
        parent->left = child->right;
        if(child->right != NULL)
            child->right->parent = parent;
        child->right = parent;
    } else { /* A right child given. */ 
        mark_gp(child);
        parent->right = child->left;
        if(child->left != NULL)
            child->left->parent = parent;
        child->left = parent;
    }
}
```

### Rust body
```rust
    fn rotate(&mut self, child: u32) {
        let parent = self.parent_of(child).expect("rotate requires a parent");
        let is_left = self.left_of(parent) == Some(child);
        self.mark_gp(child);

        if is_left {
            let cr = self.right_of(child);
            self.set_left(parent, cr);
            if let Some(cr) = cr {
                self.set_parent(cr, Some(parent));
            }
            self.set_right(child, Some(parent));
        } else {
            let cl = self.left_of(child);
            self.set_right(parent, cl);
            if let Some(cl) = cl {
                self.set_parent(cl, Some(parent));
            }
            self.set_left(child, Some(parent));
        }
    }
```

## Pair `picoquic/port_blocking.c:picoquic_check_port_blocked`
C: `picoquic/port_blocking.c:132-144 picoquic_check_port_blocked`
Rust: `rs/fq/src/lib.rs:1224-1235 check_port_blocked`

### C body
```c
{
    int ret = 0;

    for (size_t i = 0; i < nb_picoquic_blocked_port_list && port <= picoquic_blocked_port_list[i]; i++) {
        if (port == picoquic_blocked_port_list[i]){
            ret = 1;
            break;
        }
    }

    return ret;
}
```

### Rust body
```rust
pub fn check_port_blocked(port: u16) -> bool {
    // List is sorted descending; iterate while port <= list[i].
    for &blocked in BLOCKED_PORTS {
        if port > blocked {
            break;
        }
        if port == blocked {
            return true;
        }
    }
    false
}
```

## Pair `picoquic/prague.c:picoquic_prague_init`
C: `picoquic/prague.c:126-142 picoquic_prague_init`
Rust: `rs/fq/src/prague.rs:198-206 picoquic_prague_init`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    picoquic_prague_state_t* pr_state = (picoquic_prague_state_t*)malloc(sizeof(picoquic_prague_state_t));
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(option_string);
#endif

    if (pr_state != NULL) {
        memset(pr_state, 0, sizeof(picoquic_prague_state_t));
        path_x->congestion_alg_state = (void*)pr_state;
        picoquic_prague_init_reno(pr_state, path_x);
    }
    else {
        path_x->congestion_alg_state = NULL;
    }
}
```

### Rust body
```rust
) {
    let mut state = PragueState::default();
    prague_init_reno(&mut state, path_x);
    path_x.congestion_alg_state = Some(Box::new(state));
}
```

## Pair `picoquic/prague.c:picoquic_prague_initialize_era`
C: `picoquic/prague.c:172-184 picoquic_prague_initialize_era`
Rust: `rs/fq/src/prague.rs:116-127 prague_initialize_era`

### C body
```c
{
    /* Initialize the era */
    picoquic_packet_context_t* pkt_ctx = picoquic_prague_get_pkt_ctx(cnx, path_x);
    pr_state->l4s_epoch_ect1 = pkt_ctx->ecn_ect1_total_remote;
    pr_state->l4s_epoch_ce = pkt_ctx->ecn_ce_total_remote;
    pr_state->recovery_stamp = current_time;
    pr_state->recovery_sequence = picoquic_cc_get_sequence_number(path_x->cnx, path_x);
}
```

### Rust body
```rust
) {
    let pkt_ctx = prague_get_pkt_ctx(cnx, path_x);
    pr_state.l4s_epoch_ect1 = pkt_ctx.ecn_ect1_total_remote;
    pr_state.l4s_epoch_ce = pkt_ctx.ecn_ce_total_remote;
    pr_state.recovery_stamp = current_time.ticks();
    pr_state.recovery_sequence = cnx.sequence_number(path_x);
}
```

## Pair `picoquic/prague.c:picoquic_prague_process_start_ack`
C: `picoquic/prague.c:294-317 picoquic_prague_process_start_ack`
Rust: `rs/fq/src/prague.rs:264-291 picoquic_prague_process_start_ack`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = picoquic_prague_get_pkt_ctx(cnx, path_x);
    if (pr_state->ssthresh == UINT64_MAX) {
        /* Increase cwin based on bandwidth estimation. */
        path_x->cwin = picoquic_cc_update_target_cwin_estimation(path_x);
    }
    if (pkt_ctx->ecn_ce_total_remote > pr_state->l4s_epoch_ce) {
        /* CE mark received in intitial state:
         * exit and enter recovery.
         */
        picoquic_prague_enter_recovery(cnx, path_x, pr_state, current_time);
    }
    else {
        path_x->cwin += picoquic_cc_slow_start_increase_ex2(path_x, ack_state->nb_bytes_acknowledged, 0, pr_state->alpha);

        /* if cnx->cwin exceeds SSTHRESH, exit and go to CA */
        if (path_x->cwin >= pr_state->ssthresh) {
            pr_state->alg_state = picoquic_prague_alg_congestion_avoidance;
            picoquic_prague_initialize_era(cnx, path_x, pr_state, current_time);
        }
    }
}
```

### Rust body
```rust
) {
    let ecn_ce_total_remote = prague_get_pkt_ctx(cnx, path_x).ecn_ce_total_remote;

    if pr_state.ssthresh == u64::MAX {
        path_x.cwin = path_x.update_target_cwin_estimation();
    }

    if ecn_ce_total_remote > pr_state.l4s_epoch_ce {
        prague_enter_recovery(cnx, path_x, pr_state, current_time);
    } else {
        path_x.cwin = path_x.cwin.saturating_add(path_x.slow_start_increase_ex2(
            ack_state.nb_bytes_acknowledged,
            false,
            pr_state.alpha,
        ));

        if path_x.cwin >= pr_state.ssthresh {
            pr_state.alg_state = PragueAlgState::CongestionAvoidance;
            prague_initialize_era(cnx, path_x, pr_state, current_time);
        }
    }
}
```

## Pair `picoquic/quicctx.c:picoquic_context_from_epoch`
C: `picoquic/quicctx.c:382-392 picoquic_context_from_epoch`
Rust: `rs/fq/src/internal.rs:2160-2169 context_from_epoch`

### C body
```c
{
    static picoquic_packet_context_enum const pc[4] = {
        picoquic_packet_context_initial,
        picoquic_packet_context_application,
        picoquic_packet_context_handshake,
        picoquic_packet_context_application
    };

    return (epoch >= 0 && epoch < 4) ? pc[epoch] : 0;
}
```

### Rust body
```rust
    if (0..4).contains(&epoch) {
        PC[epoch as usize]
    } else {
```

## Pair `picoquic/quicctx.c:picoquic_registered_token_create`
C: `picoquic/quicctx.c:551-554 picoquic_registered_token_create`
Rust: `rs/fq/src/internal.rs:1223-1225 splay_node`

### C body
```c
{
    return &((picoquic_registered_token_t*)value)->registered_token_node;
}
```

### Rust body
```rust
    pub(crate) fn splay_node(&self) -> Option<SplayToken> {
        self.registered_token_membership
```

## Pair `picoquic/quicctx.c:picoquic_current_number_connections`
C: `picoquic/quicctx.c:624-628 picoquic_current_number_connections`
Rust: `rs/fq/src/lib.rs:1173-1181 current_number_connections`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->current_number_connections;
}
```

### Rust body
```rust
    pub fn set_max_half_open_retry_threshold(&mut self, max_half_open_before_retry: u32) {
        self.max_half_open_before_retry = max_half_open_before_retry;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_default_tp`
C: `picoquic/quicctx.c:813-817 picoquic_get_default_tp`
Rust: `rs/fq/src/lib.rs:1579-1587 default_tp`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return &quic->default_tp;
}
```

### Rust body
```rust
    pub fn max_nb_connections(&self) -> u32 {
        self.max_number_connections
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_spinbit_policy`
C: `picoquic/quicctx.c:906-918 picoquic_set_default_spinbit_policy`
Rust: `rs/fq/src/lib.rs:1677-1687 set_default_spinbit_policy`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(quic);

    if (default_spinbit_policy <= picoquic_spinbit_on) {
        quic->default_spin_policy = default_spinbit_policy;
    }
    else {
        ret = -1;
    }
    return ret;
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

## Pair `picoquic/quicctx.c:picoquic_set_default_address_discovery_mode`
C: `picoquic/quicctx.c:952-962 picoquic_set_default_address_discovery_mode`
Rust: `rs/fq/src/lib.rs:1702-1708 set_default_address_discovery_mode`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    if (mode > 0 && mode <= 3) {
        quic->default_tp.address_discovery_mode = mode;
    }
    else
    {
        quic->default_tp.address_discovery_mode = 0;
    }
}
```

### Rust body
```rust
    pub fn set_default_address_discovery_mode(&mut self, mode: i32) {
        if mode > 0 && mode <= 3 {
            self.default_tp.address_discovery_mode = mode;
        } else {
            self.default_tp.address_discovery_mode = 0;
        }
    }
```
