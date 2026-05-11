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

## Pair `picoquic/picosocks.c:picoquic_socket_set_ecn_options_ex`
C: `picoquic/picosocks.c:93-223 picoquic_socket_set_ecn_options_ex`
Rust: `rs/fq/src/socks.rs:461-506 picoquic_socket_set_ecn_options_ex`

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

## Pair `picoquic/picosocks.c:picoquic_open_server_sockets`
C: `picoquic/picosocks.c:285-325 picoquic_open_server_sockets`
Rust: `rs/fq/src/socks.rs:241-247 open`

### C body
```c
{
    int ret = 0;

#ifdef _WINDOWS
    WSADATA wsaData = { 0 };
    if (WSA_START(MAKEWORD(2, 2), &wsaData)) {
        ret = -1;
    }
#endif

    const int sock_af[] = { AF_INET6, AF_INET };

    for (int i = 0; i < PICOQUIC_NB_SERVER_SOCKETS; i++) {
        if (ret == 0) {
            sockets->s_socket[i] = socket(sock_af[i], SOCK_DGRAM, IPPROTO_UDP);
        } else {
            sockets->s_socket[i] = INVALID_SOCKET;
        }

        if (sockets->s_socket[i] == INVALID_SOCKET) {
            ret = -1;
        }
        else {
            int recv_set = 0;
            int send_set = 0;
            if (picoquic_socket_set_ecn_options(sockets->s_socket[i], sock_af[i], &recv_set, &send_set) != 0) {
                DBG_PRINTF("Cannot set ECN options (af=%d)\n", sock_af[i]);
            }
            ret = picoquic_socket_set_pkt_info(sockets->s_socket[i], sock_af[i]);
            if (ret == 0) {
                ret = picoquic_bind_to_port(sockets->s_socket[i], sock_af[i], port);
            }
            if (ret == 0) {
                ret = picoquic_socket_set_pmtud_options(sockets->s_socket[i], sock_af[i]);
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn open(port: i32) -> Result<Self, Error> {
        let s6 = S::open_server_v6(port)?;
        let s4 = S::open_server_v4(port)?;
        Ok(Self {
            sockets: [Some(s6), Some(s4)],
        })
    }
```

## Pair `picoquic/picosocks.c:picoquic_socks_cmsg_format`
C: `picoquic/picosocks.c:642-736 picoquic_socks_cmsg_format`
Rust: `rs/fq/src/socks.rs:544-554 format_cmsg`

### C body
```c
{
    struct msghdr* msg = (struct msghdr*)vmsg;
    int control_length = 0;
    struct cmsghdr* last_cmsg = NULL;
    int is_null = 0;

    if (addr_from != NULL && addr_from->sa_family != 0) {
        if (addr_from->sa_family == AF_INET) {
#ifdef IP_PKTINFO
            struct in_pktinfo* pktinfo = (struct in_pktinfo*)cmsg_format_header_return_data_ptr(msg, &last_cmsg,
                &control_length, IPPROTO_IP, IP_PKTINFO, sizeof(struct in_pktinfo));
            if (pktinfo != NULL) {
                pktinfo->ipi_spec_dst.s_addr = ((struct sockaddr_in*)addr_from)->sin_addr.s_addr;
                pktinfo->ipi_ifindex = (unsigned long)dest_if;
            }
            else {
                is_null = 1;
            }
#else 
            /* The IP_PKTINFO structure is not defined on BSD */
            /* Some versions of freeBSD do not define IP_SENDSRCADDR, use IP_RECVDSTADDR instead. */
            struct in_addr* pktinfo = (struct in_addr*)cmsg_format_header_return_data_ptr(msg, &last_cmsg,
                &control_length, IPPROTO_IP,
#ifdef IP_SENDSRCADDR
                IP_SENDSRCADDR
#else
                IP_RECVDSTADDR
#endif
                , sizeof(struct in_addr));
            if (pktinfo != NULL) {
                pktinfo->s_addr = ((struct sockaddr_in*)addr_from)->sin_addr.s_addr;
            }
            else {
                is_null = 1;
            }
#endif
        }
        else {
            struct in6_pktinfo* pktinfo6 = (struct in6_pktinfo*)cmsg_format_header_return_data_ptr(msg, &last_cmsg,
                &control_length, IPPROTO_IPV6, IPV6_PKTINFO, sizeof(struct in6_pktinfo));
            if (pktinfo6 != NULL) {
                memcpy(&pktinfo6->ipi6_addr, &((struct sockaddr_in6*)addr_from)->sin6_addr, sizeof(struct in6_addr));
                pktinfo6->ipi6_ifindex = (unsigned long)dest_if;
            }
            else {
                is_null = 1;
            }
#ifdef IPV6_DONTFRAG
            if (!is_null) {
                int* pval = (int*)cmsg_format_header_return_data_ptr(msg, &last_cmsg,
                    &control_length, SOL_IPV6, IPV6_DONTFRAG, sizeof(int));
                if (pval != NULL) {
                    *pval = 1;
                }
                else {
                    is_null = 1;
                }
            }
#endif
        }
    }
#if defined(UDP_SEGMENT)
    if (!is_null && send_msg_size > 0 && send_msg_size < message_length) {
        uint16_t* pval = (uint16_t*)cmsg_format_header_return_data_ptr(msg, &last_cmsg,
            &control_length, SOL_UDP, UDP_SEGMENT, sizeof(uint16_t));
        if (pval != NULL) {
            *pval = (uint16_t)send_msg_size;
        }
        else {
            is_null = 1;
        }
    }
#endif

    msg->msg_controllen = control_length;
    if (control_length == 0) {
        msg->msg_control = NULL;
    }
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

## Pair `picoquic/picosocks.c:picoquic_select`
C: `picoquic/picosocks.c:1248-1260 picoquic_select`
Rust: `rs/fq/src/lib.rs:540-553 select`

### C body
```c
    uint64_t* current_time) {
    int socket_rank;
    return picoquic_select_ex(sockets, nb_sockets, addr_from, addr_dest, dest_if,
        received_ecn, buffer, buffer_max, delta_t, &socket_rank, current_time);
}
```

### Rust body
```rust
pub trait ConnectionIdCallback {
    fn produce(
        &mut self,
        quic: &mut Quic,
        connection_id_local: ConnectionId,
        connection_id_remote: ConnectionId,
    ) -> ConnectionId;
}
```

## Pair `picoquic/picosocks.c:picoquic_set_key_log_file_from_env`
C: `picoquic/picosocks.c:1369-1396 picoquic_set_key_log_file_from_env`
Rust: `rs/fq/src/lib.rs:1154-1159 set_key_log_file_from_env`

### C body
```c
{
#ifdef PICOQUIC_WITHOUT_SSLKEYLOG
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(quic);
#endif /* WINDOWS*/
#else
    if (picoquic_is_sslkeylog_enabled(quic)) {
        char* keylog_filename = NULL;

#ifdef _WINDOWS
        size_t len;

        if (_dupenv_s(&keylog_filename, &len, "SSLKEYLOGFILE") != 0 ||
            keylog_filename == NULL) {
            return;
        }
#else
        keylog_filename = getenv("SSLKEYLOGFILE");
        if (keylog_filename == NULL) {
            return;
        }
#endif

        picoquic_set_key_log_file(quic, keylog_filename);
    }
#endif /* PICOQUIC_WITHOUT_SSLKEYLOG */
}
```

### Rust body
```rust
        {
            self.set_key_log_file(Some(&path));
        }
```

## Pair `picoquic/picosplay.c:zigzig`
C: `picoquic/picosplay.c:64-70 zigzig`
Rust: `rs/fq/src/splay.rs:352-355 zigzig`

### C body
```c
static void zigzig(picosplay_node_t *x, picosplay_node_t *p) {
    rotate(p);
    rotate(x);
}
```

### Rust body
```rust
    fn zigzig(&mut self, x: u32, p: u32) {
        self.rotate(p);
        self.rotate(x);
    }
```

## Pair `picoquic/picosplay.c:picosplay_insert`
C: `picoquic/picosplay.c:99-138 picosplay_insert`
Rust: `rs/fq/src/splay.rs:415-456 insert`

### C body
```c
picosplay_node_t* picosplay_insert(picosplay_tree_t *tree, void *value) {
    picosplay_node_t *new = tree->create(value);

    if (new != NULL) {
        new->left = NULL;
        new->right = NULL;
        if (tree->root == NULL) {
            tree->root = new;
            new->parent = NULL;
        }
        else {
            picosplay_node_t *curr = tree->root;
            picosplay_node_t *parent = NULL;
            int left = 0;
            while (curr != NULL) {
                parent = curr;
                if (tree->comp(tree->node_value(new), tree->node_value(curr)) < 0) {
                    left = 1;
                    curr = curr->left;
                }
                else {
                    left = 0;
                    curr = curr->right;
                }
            }
            new->parent = parent;
            if (left)
                parent->left = new;
            else
                parent->right = new;
        }
        splay(tree, new);
        tree->size++;
    }

    return new;
}
```

### Rust body
```rust
    pub fn insert(&mut self, key: K, value: V) -> Result<(SplayToken, Option<V>), Error> {
        if let Some(tok) = self.find(&key) {
            let old = core::mem::replace(self.value_of_mut(tok.idx), value);
            return Ok((tok, Some(old)));
        }

        if self.root.is_none() {
            let idx = self.alloc_slot(key, value)?;
            self.root = Some(idx);
            self.len += 1;
            return Ok((self.token_of(idx), None));
        }

        let mut cur = self.root.unwrap();
        let mut par;
        let mut go_left;
        loop {
            par = cur;
            let cmp = key.cmp(self.key_of(cur));
            go_left = cmp == core::cmp::Ordering::Less;
            let next = if go_left {
                self.left_of(cur)
            } else {
                self.right_of(cur)
            };
            match next {
                None => break,
                Some(n) => cur = n,
            }
        }

        let idx = self.alloc_slot(key, value)?;
        self.set_parent(idx, Some(par));
        if go_left {
            self.set_left(par, Some(idx));
        } else {
            self.set_right(par, Some(idx));
        }
        self.splay(idx);
        self.len += 1;
        Ok((self.token_of(idx), None))
    }
```

## Pair `picoquic/picosplay.c:picosplay_delete_hint`
C: `picoquic/picosplay.c:194-222 picosplay_delete_hint`
Rust: `rs/fq/src/splay.rs:595-639 remove`

### C body
```c
void picosplay_delete_hint(picosplay_tree_t *tree, picosplay_node_t *node) {
    if(node == NULL)
        return;
    splay(tree, node); /* Now node is tree's root. */
    if(node->left == NULL) {
        tree->root = node->right;
        if(tree->root != NULL)
            tree->root->parent = NULL;
    } else if(node->right == NULL) {
        tree->root = node->left;
        tree->root->parent = NULL;
    } else {
        picosplay_node_t *x = leftmost(node->right);
        if(x->parent != node) {
            x->parent->left = x->right;
            if(x->right != NULL)
                x->right->parent = x->parent;
            x->right = node->right;
            x->right->parent = x;
        }
        tree->root = x;
        x->parent = NULL;
        x->left = node->left;
        x->left->parent = x;
    }
    tree->delete_node(tree, node);
    tree->size--;
}
```

### Rust body
```rust
    pub fn remove(&mut self, token: SplayToken) -> Option<(K, V)> {
        if !self.is_valid(token) {
            return None;
        }
        let node = token.idx;
        self.splay(node);

        let left = self.left_of(node);
        let right = self.right_of(node);

        match (left, right) {
            (None, _) => {
                self.root = right;
                if let Some(r) = right {
                    self.set_parent(r, None);
                }
            }
            (left, None) => {
                self.root = left;
                if let Some(l) = left {
                    self.set_parent(l, None);
                }
            }
            (Some(l), Some(r)) => {
                let x = self.leftmost(Some(r)).unwrap();
                if self.parent_of(x) != Some(node) {
                    let xr = self.right_of(x);
                    let xp = self.parent_of(x).unwrap();
                    self.set_left(xp, xr);
                    if let Some(xr) = xr {
                        self.set_parent(xr, Some(xp));
                    }
                    self.set_right(x, Some(r));
                    self.set_parent(r, Some(x));
                }
                self.set_left(x, Some(l));
                self.set_parent(l, Some(x));
                self.root = Some(x);
                self.set_parent(x, None);
            }
        }

        self.len -= 1;
        Some(self.release_slot(node))
    }
```

## Pair `picoquic/picosplay.c:picosplay_next`
C: `picoquic/picosplay.c:245-256 picosplay_next`
Rust: `rs/fq/src/splay.rs:545-548 next`

### C body
```c
picosplay_node_t* picosplay_next(picosplay_node_t *node) {
    if(node->right != NULL)
        return leftmost(node->right);
    while(node->parent != NULL && node == node->parent->right)
        node = node->parent;
    return node->parent;
}
```

### Rust body
```rust
        if !self.is_valid(token) {
            return None;
        }
```

## Pair `picoquic/picosplay.c:leftmost`
C: `picoquic/picosplay.c:320-327 leftmost`
Rust: `rs/fq/src/splay.rs:387-393 leftmost`

### C body
```c
static picosplay_node_t* leftmost(picosplay_node_t *node) {
    picosplay_node_t *parent = NULL;
    while(node != NULL) {
        parent = node;
        node = node->left;
    }
    return parent;
}
```

### Rust body
```rust
            match self.left_of(cur) {
                None => return Some(cur),
                Some(l) => cur = l,
            }
```

## Pair `picoquic/port_blocking.c:picoquic_disable_port_blocking`
C: `picoquic/port_blocking.c:162-166 picoquic_disable_port_blocking`
Rust: `rs/fq/src/lib.rs:1190-1192 set_port_blocking_disabled`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->is_port_blocking_disabled = is_port_blocking_disabled;
}
```

### Rust body
```rust
    pub fn set_port_blocking_disabled(&mut self, is_port_blocking_disabled: bool) {
        self.is_port_blocking_disabled = is_port_blocking_disabled;
    }
```

## Pair `picoquic/prague.c:picoquic_prague_reset_l3s`
C: `picoquic/prague.c:156-163 picoquic_prague_reset_l3s`
Rust: `rs/fq/src/prague.rs:97-103 prague_reset_l3s`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = picoquic_prague_get_pkt_ctx(cnx, path_x);
    pr_state->l4s_epoch_send = pkt_ctx->send_sequence;
    pr_state->l4s_epoch_ect1 = pkt_ctx->ecn_ect1_total_remote;
    pr_state->l4s_epoch_ce = pkt_ctx->ecn_ce_total_remote;
    pr_state->alpha = 0;
}
```

### Rust body
```rust
fn prague_reset_l3s(cnx: &Connection, pr_state: &mut PragueState, path_x: &Path) {
    let pkt_ctx = prague_get_pkt_ctx(cnx, path_x);
    pr_state.l4s_epoch_send = pkt_ctx.send_sequence;
    pr_state.l4s_epoch_ect1 = pkt_ctx.ecn_ect1_total_remote;
    pr_state.l4s_epoch_ce = pkt_ctx.ecn_ce_total_remote;
    pr_state.alpha = 0;
}
```

## Pair `picoquic/prague.c:picoquic_prague_update_alpha`
C: `picoquic/prague.c:210-249 picoquic_prague_update_alpha`
Rust: `rs/fq/src/prague.rs:147-159 prague_update_alpha`

### C body
```c
{
    uint64_t frac = 0;
    int is_suspect = 0;

    if (delta_ce > 0) {
        frac = (delta_ce * 1024) / (delta_ce + delta_ect1);
    }
    else {
        frac = 0;
    }

    if (pr_state->l4s_update_sent != 0 && frac >= 512 && pr_state->alpha < 128 &&
        current_time - pr_state->recovery_stamp > path_x->smoothed_rtt) {
        /*
         * the epoch lasted more than the RTT. This is most
         * probably due to period of inactivity, then effects of imprecise
         * tuning of pacing's leaky bucket algorithm. Limiting the
         * fraction frac to about 1/8th to avoid too much bad effects. */
        is_suspect = 1;
        frac = 128;
    }

    if (delta_ce > 0 || delta_ect1 > 0) {
        if (frac > pr_state->alpha && (frac >= 512 || is_suspect)) {
            pr_state->alpha = frac;
        }
        else
        {
            uint64_t alpha_shifted = pr_state->alpha << PRAGUE_SHIFT_G;
            alpha_shifted -= pr_state->alpha;
            alpha_shifted += frac;
            pr_state->alpha = alpha_shifted >> PRAGUE_SHIFT_G;
        }
    }
    picoquic_log_app_message(path_x->cnx,
        "Prague: %" PRIu64 ",%d,%d,%d,%" PRIu64 ",%" PRIu64,
        current_time, (int)delta_ect1, (int)delta_ce, (int)pr_state->alpha, path_x->cwin, path_x->rtt_sample);
}
```

### Rust body
```rust
    } else {
        0
    };
```

## Pair `picoquic/prague.c:picoquic_prague_delete`
C: `picoquic/prague.c:396-403 picoquic_prague_delete`
Rust: `rs/fq/src/prague.rs:425-435 alg_delete`

### C body
```c
{
    if (path_x->congestion_alg_state != NULL) {
        free(path_x->congestion_alg_state);
        path_x->congestion_alg_state = NULL;
    }
}
```

### Rust body
```rust
    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|state| state.downcast_ref::<PragueState>())
            .map(PragueState::observe)
    }
```

## Pair `picoquic/quicctx.c:picoquic_update_issued_ticket`
C: `picoquic/quicctx.c:444-459 picoquic_update_issued_ticket`
Rust: `rs/fq/src/lib.rs:7539-7547 update`

### C body
```c
{
    /* Update in place */
    if (ip_addr_length > PICOQUIC_STORED_IP_MAX) {
        ip_addr_length = PICOQUIC_STORED_IP_MAX;
    }
    ticket->ip_addr_length = ip_addr_length;
    memcpy(ticket->ip_addr, ip_addr, ip_addr_length);
    ticket->rtt = rtt;
    ticket->cwin = cwin;
}
```

### Rust body
```rust
mod test {}
```

## Pair `picoquic/quicctx.c:picoquic_registered_token_clear`
C: `picoquic/quicctx.c:597-610 picoquic_registered_token_clear`
Rust: `rs/fq/src/internal.rs:2216-2241 registered_token_clear`

### C body
```c
{
    int end_reached = 0;
    do {
        picoquic_registered_token_t* rt_first = (picoquic_registered_token_t*)
            picoquic_registered_token_value(picosplay_first(&quic->token_reuse_tree));
        if (rt_first == NULL || rt_first->token_time >= expiry_time_max) {
            end_reached = 1;
        }
        else {
            picosplay_delete_hint(&quic->token_reuse_tree, &rt_first->registered_token_node);
        }
    } while (!end_reached);
}
```

### Rust body
```rust
    pub fn registered_token_clear(&mut self, expiry_time_max: Instant) {
        let mut expired = Vec::new();
        let mut current = self.token_reuse_tree.first();
        while let Some(st) = current {
            current = self.token_reuse_tree.next(st);
            if let Some(&arena_tok) = self.token_reuse_tree.get(st) {
                let is_expired = self
                    .registered_tokens
                    .get(arena_tok)
                    .map(|rt| rt.token_time < expiry_time_max)
                    .unwrap_or(true);
                if is_expired {
                    expired.push((st, Some(arena_tok)));
                }
            } else {
                expired.push((st, None));
            }
        }

        for (st, arena_tok) in expired {
            self.token_reuse_tree.remove(st);
            if let Some(arena_tok) = arena_tok {
                self.registered_tokens.remove(arena_tok);
            }
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_load_token_file`
C: `picoquic/quicctx.c:777-796 picoquic_load_token_file`
Rust: `rs/fq/src/internal.rs:3942-3947 load_token_file`

### C body
```c
{
    int ret;
    PICOQUIC_THREAD_CHECK(quic);
    ret = picoquic_load_tokens(quic, token_file_name);

    if (ret == PICOQUIC_ERROR_NO_SUCH_FILE) {
        DBG_PRINTF("Ticket file <%s> not created yet.\n", token_file_name);
        ret = 0;
    }
    else if (ret != 0) {
        DBG_PRINTF("Cannot load tickets from <%s>\n", token_file_name);
    }

    if (ret == 0) {
        quic->token_file_name = token_file_name;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        self.load_tokens(token_file_name)
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_tp_value`
C: `picoquic/quicctx.c:893-897 picoquic_set_default_tp_value`
Rust: `rs/fq/src/lib.rs:1620-1622 set_default_tp_value`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return picoquic_set_tp_value_by_type(&quic->default_tp, tp_type, tp_value);
}
```

### Rust body
```rust
    pub fn set_default_tp_value(&mut self, tp_type: u64, tp_value: u64) -> Result<(), Error> {
        set_tp_value_by_type(&mut self.default_tp, tp_type, tp_value)
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_lossbit_policy`
C: `picoquic/quicctx.c:935-940 picoquic_set_default_lossbit_policy`
Rust: `rs/fq/src/lib.rs:1690-1697 set_default_lossbit_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_lossbit_policy = default_lossbit_policy;
    quic->default_tp.enable_loss_bit = (int)default_lossbit_policy;
}
```

### Rust body
```rust
    pub fn set_default_multipath_option(&mut self, multipath_option: i32) {
        self.default_multipath_option = multipath_option as u32;
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_max_data_control`
C: `picoquic/quicctx.c:970-990 picoquic_set_max_data_control`
Rust: `rs/fq/src/lib.rs:1738-1746 set_max_data_control`

### C body
```c
{
    picoquic_cnx_t* cnx;
    PICOQUIC_THREAD_CHECK(quic);
    cnx = quic->cnx_list;
    quic->max_data_limit = max_data;

    quic->default_tp.initial_max_data = max_data;

    while (cnx != NULL) {
        /* If the connection is not yet initialized, reset the maxdata parameter */
        if (cnx->client_mode &&
            cnx->cnx_state == picoquic_state_client_init &&
            cnx->tls_stream[0].sent_offset == 0 &&
            cnx->tls_stream[0].send_queue == NULL){
            cnx->local_parameters.initial_max_data = max_data;
            cnx->maxdata_local = max_data;
        }
        cnx = cnx->next_in_table;
    }
}
```

### Rust body
```rust
    pub fn set_default_idle_timeout(&mut self, idle_timeout: Duration) {
        self.default_tp.max_idle_timeout = idle_timeout;
    }
```
