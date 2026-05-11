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

## Pair `picoquic/picosocks.c:picoquic_socket_set_pkt_info`
C: `picoquic/picosocks.c:58-91 picoquic_socket_set_pkt_info`
Rust: `rs/fq/src/socks.rs:134-136 set_pkt_info`

### C body
```c
{
    int ret;
#ifdef _WINDOWS
    int option_value = 1;
    if (af == AF_INET6) {
        ret = setsockopt(sd, IPPROTO_IPV6, IPV6_PKTINFO, (char*)&option_value, sizeof(int));
    }
    else {
        ret = setsockopt(sd, IPPROTO_IP, IP_PKTINFO, (char*)&option_value, sizeof(int));
    }
#else
    if (af == AF_INET6) {
        int val = 1;
        ret = setsockopt(sd, IPPROTO_IPV6, IPV6_V6ONLY,
            &val, sizeof(val));
        if (ret == 0) {
            val = 1;
            ret = setsockopt(sd, IPPROTO_IPV6, IPV6_RECVPKTINFO, (char*)&val, sizeof(int));
        }
    }
    else {
        int val = 1;
#ifdef IP_PKTINFO
        ret = setsockopt(sd, IPPROTO_IP, IP_PKTINFO, (char*)&val, sizeof(int));
#else
        /* The IP_PKTINFO structure is not defined on BSD */
        ret = setsockopt(sd, IPPROTO_IP, IP_RECVDSTADDR, (char*)&val, sizeof(int));
#endif
    }
#endif

    return ret;
}
```

### Rust body
```rust
    fn set_pkt_info(&mut self) -> Result<(), Error> {
        Ok(())
    }
```

## Pair `picoquic/picosocks.c:picoquic_open_client_socket`
C: `picoquic/picosocks.c:250-283 picoquic_open_client_socket`
Rust: `rs/fq/src/socks_socket2.rs:29-38 open_client`

### C body
```c
{
#ifdef _WINDOWS
    WSADATA wsaData = { 0 };
    (void)WSA_START(MAKEWORD(2, 2), &wsaData);
    SOCKET_TYPE sd = WSASocket(af, SOCK_DGRAM, IPPROTO_UDP, NULL, 0, WSA_FLAG_OVERLAPPED);
#else
    SOCKET_TYPE sd = socket(af, SOCK_DGRAM, IPPROTO_UDP);
#endif

    if (sd != INVALID_SOCKET) {
        int send_set = 0;
        int recv_set = 0;

        if (picoquic_socket_set_pkt_info(sd, af) != 0) {
            DBG_PRINTF("Cannot set PKTINFO option (af=%d)\n", af);
        }
        if (picoquic_socket_set_ecn_options(sd, af, &recv_set, &send_set) != 0) {
            DBG_PRINTF("Cannot set ECN options (af=%d)\n", af);
        }
        if (picoquic_socket_set_pmtud_options(sd, af) != 0) {
            DBG_PRINTF("Cannot set PMTUD options (af=%d)\n", af);
        }
    }
    else {
#ifdef _WINDOWS
        DBG_PRINTF("Cannot open socket(AF=%d), error: %d\n", af, GetLastError());
#else
        DBG_PRINTF("Cannot open socket(AF=%d), error: %d\n", af, errno);
#endif
    }

    return sd;
}
```

### Rust body
```rust
    pub fn open_client(af: i32) -> Result<Self, Error> {
        let domain = socket2::Domain::from(af);
        let sock = socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))
            .map_err(|_| Error::Generic)?;
        let mut udp = Socket2Udp(sock, af);
        let _ = udp.set_pkt_info();
        let _ = udp.set_ecn_options();
        let _ = udp.set_pmtud_options();
        Ok(udp)
    }
```

## Pair `picoquic/picosocks.c:cmsg_format_header_return_data_ptr`
C: `picoquic/picosocks.c:519-543 cmsg_format_header_return_data_ptr`
Rust: `rs/fq/src/socks.rs:579-622 cmsg_format_header_return_data_ptr`

### C body
```c
{
    void* cmsg_data_ptr = NULL;
#ifdef CMSG_ALIGN
    struct cmsghdr* cmsg = (*last_cmsg == NULL) ? CMSG_FIRSTHDR(msg) :
        (struct cmsghdr*)((unsigned char*)(*last_cmsg) + CMSG_ALIGN((*last_cmsg)->cmsg_len));
#else
    struct cmsghdr* cmsg = (*last_cmsg == NULL) ? CMSG_FIRSTHDR(msg) : CMSG_NXTHDR(msg, *last_cmsg);
#endif

    if (cmsg != NULL) {
        size_t cmsg_required_space = CMSG_SPACE(cmsg_data_len);
        *control_length += (int)cmsg_required_space;
        memset(cmsg, 0, cmsg_required_space);
        cmsg->cmsg_level = cmsg_level;
        cmsg->cmsg_type = cmsg_type;
        cmsg->cmsg_len = CMSG_LEN(cmsg_data_len);
        cmsg_data_ptr = (void*)CMSG_DATA(cmsg);
        *last_cmsg = cmsg;
    }

    return cmsg_data_ptr;
}
```

### Rust body
```rust
) -> *mut core::ffi::c_void {
    // Rust 2024: unsafe operations inside unsafe fn still need explicit
    // unsafe {} blocks.

    // Locate the next available cmsg slot.  On the first call last_cmsg
    // is null so CMSG_FIRSTHDR returns the start of the control buffer;
    // on subsequent calls CMSG_NXTHDR advances past the previous entry.
    let cmsg: *mut libc::cmsghdr = unsafe {
        if (*last_cmsg).is_null() {
            libc::CMSG_FIRSTHDR(msg as *const libc::msghdr)
        } else {
            // The C source uses CMSG_ALIGN (Linux) when defined and falls
            // back to CMSG_NXTHDR otherwise.  CMSG_NXTHDR is correct on
            // all POSIX platforms we target and is always available.
            libc::CMSG_NXTHDR(
                msg as *const libc::msghdr,
                *last_cmsg as *const libc::cmsghdr,
            )
        }
    };

    if cmsg.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        let cmsg_required_space = libc::CMSG_SPACE(cmsg_data_len as libc::c_uint) as usize;
        *control_length += cmsg_required_space as libc::c_int;
        // Zero-fill the entire padded region (mirrors the C memset call).
        core::ptr::write_bytes(cmsg as *mut u8, 0, cmsg_required_space);
        (*cmsg).cmsg_level = cmsg_level;
        (*cmsg).cmsg_type = cmsg_type;
        (*cmsg).cmsg_len = libc::CMSG_LEN(cmsg_data_len as libc::c_uint) as _;
        *last_cmsg = cmsg;
        libc::CMSG_DATA(cmsg) as *mut core::ffi::c_void
    }
}
```

## Pair `picoquic/picosocks.c:picoquic_select_ex`
C: `picoquic/picosocks.c:1167-1246 picoquic_select_ex`
Rust: `rs/fq/src/socks.rs:334-365 select`

### C body
```c
{
    fd_set readfds;
    struct timeval tv;
    int ret_select = 0;
    int bytes_recv = 0;
    int sockmax = 0;

    if (received_ecn != NULL) {
        *received_ecn = 0;
    }

    FD_ZERO(&readfds);

    for (int i = 0; i < nb_sockets; i++) {
        if (sockmax < (int)sockets[i]) {
            sockmax = (int)sockets[i];
        }
        FD_SET(sockets[i], &readfds);
    }

    if (delta_t <= 0) {
        tv.tv_sec = 0;
        tv.tv_usec = 0;
    } else {
        if (delta_t > 10000000) {
            tv.tv_sec = (long)10;
            tv.tv_usec = 0;
        } else {
            tv.tv_sec = (long)(delta_t / 1000000);
            tv.tv_usec = (long)(delta_t % 1000000);
        }
    }

    ret_select = select(sockmax + 1, &readfds, NULL, NULL, &tv);

    if (ret_select < 0) {
        bytes_recv = -1;
        DBG_PRINTF("Error: select returns %d\n", ret_select);
    } else if (ret_select > 0) {
        for (int i = 0; i < nb_sockets; i++) {
            if (FD_ISSET(sockets[i], &readfds)) {
                *socket_rank = i;
                bytes_recv = picoquic_recvmsg(sockets[i], addr_from,
                    addr_dest, dest_if, received_ecn,
                    buffer, buffer_max);

                if (bytes_recv <= 0) {
#ifdef _WINDOWS
                    int last_error = WSAGetLastError();

                    if (last_error == WSAECONNRESET || last_error == WSAEMSGSIZE) {
                        bytes_recv = 0;
                        continue;
                    }
#endif
                    DBG_PRINTF("Could not receive packet on UDP socket[%d]= %d!\n",
                        i, (int)sockets[i]);

                    break;
                } else {
                    break;
                }
            }
        }
    }

    *current_time = picoquic_current_time();

    return bytes_recv;
}
```

### Rust body
```rust
) -> Result<SelectInfo, Error> {
    // Faithful note: the C body uses select(2) with raw file descriptors.
    // The generic Socket trait does not expose a file descriptor, so this
    // fallback tries each socket once in order and returns the first that
    // delivers data.  Platform-specific implementations can supply a more
    // efficient multiplexed select via the concrete Socket type.
    for (rank, socket) in sockets.iter_mut().enumerate() {
        match socket.recv(buffer) {
            Ok(info) if info.bytes_recv > 0 => {
                let now_us = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_micros() as u64)
                    .unwrap_or(0);
                return Ok(SelectInfo {
                    addr_from: info.addr_from,
                    addr_dest: info.addr_dest,
                    dest_if: info.dest_if,
                    received_ecn: info.received_ecn,
                    bytes_recv: info.bytes_recv,
                    current_time: Instant::from_ticks(now_us),
                    socket_rank: rank,
                });
            }
            _ => {}
        }
    }
    Err(Error::Generic)
}
```

## Pair `picoquic/picosocks.c:picoquic_get_server_address`
C: `picoquic/picosocks.c:1285-1355 picoquic_get_server_address`
Rust: `rs/fq/src/tests/socket.rs:22-25 get_server_address`

### C body
```c
{
    int ret = 0;
    struct sockaddr_in* ipv4_dest = (struct sockaddr_in*)server_address;
    struct sockaddr_in6* ipv6_dest = (struct sockaddr_in6*)server_address;

    /* get the IP address of the server */
    memset(server_address, 0, sizeof(struct sockaddr_storage));
    *is_name = 0;

    if (inet_pton(AF_INET, ip_address_text, &ipv4_dest->sin_addr) == 1) {
        /* Valid IPv4 address */
        ipv4_dest->sin_family = AF_INET;
        ipv4_dest->sin_port = htons((unsigned short)server_port);
    } else if (inet_pton(AF_INET6, ip_address_text, &ipv6_dest->sin6_addr) == 1) {
        /* Valid IPv6 address */
        ipv6_dest->sin6_family = AF_INET6;
        ipv6_dest->sin6_port = htons((unsigned short)server_port);
    } else {
        /* Server is described by name. Do a lookup for the IP address,
        * and then use the name as SNI parameter */
        struct addrinfo* result = NULL;
        struct addrinfo hints;

        memset(&hints, 0, sizeof(hints));
        hints.ai_family = AF_UNSPEC;
        hints.ai_socktype = SOCK_DGRAM;
        hints.ai_protocol = IPPROTO_UDP;

        if ((ret = getaddrinfo(ip_address_text, NULL, &hints, &result)) != 0) {
#ifdef _WINDOWS
            int err = GetLastError();
#else
            int err = ret;
#endif
            fprintf(stderr, "Cannot get IP address for %s, err = %d (0x%x)\n", ip_address_text, err, err);
            ret = -1;
        } else {
            *is_name = 1;

            switch (result->ai_family) {
            case AF_INET:
                ipv4_dest->sin_family = AF_INET;
                ipv4_dest->sin_port = htons((unsigned short)server_port);
#ifdef _WINDOWS
                ipv4_dest->sin_addr.S_un.S_addr = ((struct sockaddr_in*)result->ai_addr)->sin_addr.S_un.S_addr;
#else
                ipv4_dest->sin_addr.s_addr = ((struct sockaddr_in*)result->ai_addr)->sin_addr.s_addr;
#endif
                break;
            case AF_INET6:
                ipv6_dest->sin6_family = AF_INET6;
                ipv6_dest->sin6_port = htons((unsigned short)server_port);
                memcpy(&ipv6_dest->sin6_addr,
                    &((struct sockaddr_in6*)result->ai_addr)->sin6_addr,
                    sizeof(ipv6_dest->sin6_addr));
                break;
            default:
                fprintf(stderr, "Error getting IPv6 address for %s, family = %d\n",
                    ip_address_text, result->ai_family);
                ret = -1;
                break;
            }

            freeaddrinfo(result);
        }
    }

    return ret;
}
```

### Rust body
```rust
fn get_server_address(addr_text: &str, port: u16) -> crate::Result<(SocketAddr, bool)> {
    let sa = crate::socks::ServerAddress::resolve(addr_text, port as i32)?;
    Ok((sa.addr, sa.is_name))
}
```

## Pair `picoquic/picosplay.c:zig`
C: `picoquic/picosplay.c:59-62 zig`
Rust: `rs/fq/src/splay.rs:347-349 zig`

### C body
```c
static void zig(picosplay_node_t *x) {
    rotate(x);
}
```

### Rust body
```rust
    fn zig(&mut self, x: u32) {
        self.rotate(x);
    }
```

## Pair `picoquic/picosplay.c:picosplay_new_tree`
C: `picoquic/picosplay.c:90-97 picosplay_new_tree`
Rust: `rs/fq/src/splay.rs:161-204 new_tree`

### C body
```c
picosplay_tree_t* picosplay_new_tree(picosplay_comparator comp, picosplay_create create, picosplay_delete_node delete_node, picosplay_node_value node_value) {
    picosplay_tree_t *new = malloc(sizeof(picosplay_tree_t));
    if (new != NULL) {
        picosplay_init_tree(new, comp, create, delete_node, node_value);
    }
    return new;
}
```

### Rust body
```rust
    fn alloc_slot(&mut self, key: K, value: V) -> Result<u32, Error> {
        if let Some(free_idx) = self.free {
            let next_free = match &self.slots[free_idx as usize].state {
                SlotState::Free { next_free } => *next_free,
                SlotState::Filled { .. } => unreachable!(),
            };
            let r#gen = self.slots[free_idx as usize].generation;
            self.slots[free_idx as usize] = Slot {
                generation: r#gen,
                state: SlotState::Filled {
                    key,
                    value,
                    parent: None,
                    left: None,
                    right: None,
                },
            };
            self.free = next_free;
            Ok(free_idx)
        } else {
            let idx = self.slots.len();
            if idx > u32::MAX as usize {
                return Err(Error::Memory);
            }
            self.slots.push(Slot {
                generation: 0,
                state: SlotState::Filled {
                    key,
                    value,
                    parent: None,
                    left: None,
                    right: None,
                },
            });
            Ok(idx as u32)
        }
    }
```

## Pair `picoquic/picosplay.c:picosplay_delete`
C: `picoquic/picosplay.c:188-192 picosplay_delete`
Rust: `rs/fq/src/lib.rs:2416-2423 delete`

### C body
```c
void picosplay_delete(picosplay_tree_t *tree, void *value) {
    picosplay_node_t *node = picosplay_find(tree, value);
    picosplay_delete_hint(tree, node);
}
```

### Rust body
```rust
    pub fn set_app_wake_time(&mut self, app_wake_time: Instant) {
        self.app_wake_time = app_wake_time;
    }
```

## Pair `picoquic/picosplay.c:picosplay_previous`
C: `picoquic/picosplay.c:237-243 picosplay_previous`
Rust: `rs/fq/src/splay.rs:520-523 previous`

### C body
```c
picosplay_node_t* picosplay_previous(picosplay_node_t* node) {
    if (node->left != NULL)
        return rightmost(node->left);
    while (node->parent != NULL && node == node->parent->left)
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

## Pair `picoquic/picosplay.c:mark_gp`
C: `picoquic/picosplay.c:307-318 mark_gp`
Rust: `rs/fq/src/splay.rs:310-322 mark_gp`

### C body
```c
static void mark_gp(picosplay_node_t *child) {
    picosplay_node_t *parent = child->parent;
    picosplay_node_t *grand = parent->parent;
    child->parent = grand;
    parent->parent = child;
    if(grand == NULL)
        return;
    if(grand->left == parent)
        grand->left = child;
    else
        grand->right = child;
}
```

### Rust body
```rust
    fn mark_gp(&mut self, child: u32) {
        let parent = self.parent_of(child).expect("mark_gp requires a parent");
        let grand = self.parent_of(parent);
        self.set_parent(child, grand);
        self.set_parent(parent, Some(child));
        if let Some(g) = grand {
            if self.left_of(g) == Some(parent) {
                self.set_left(g, Some(child));
            } else {
                self.set_right(g, Some(child));
            }
        }
    }
```

## Pair `picoquic/port_blocking.c:picoquic_check_addr_blocked`
C: `picoquic/port_blocking.c:146-160 picoquic_check_addr_blocked`
Rust: `rs/fq/src/lib.rs:1239-1245 check_addr_blocked`

### C body
```c
{
    /* The sockaddr is always in network order. We must translate to
     * host order before performaing the check */
    uint16_t port = UINT16_MAX;

    if (addr_from->sa_family == AF_INET) {
        port = ntohs(((struct sockaddr_in*)addr_from)->sin_port);
    }
    else if (addr_from->sa_family == AF_INET6) {
        /* configure an IPv6 sockaddr */
        port = ntohs(((struct sockaddr_in6*)addr_from)->sin6_port);
    }
    return picoquic_check_port_blocked(port);
}
```

### Rust body
```rust
pub(crate) fn unspecified_socket_addr() -> SocketAddr {
    SocketAddr::new(core::net::IpAddr::V4(core::net::Ipv4Addr::UNSPECIFIED), 0)
}
```

## Pair `picoquic/prague.c:picoquic_prague_get_pkt_ctx`
C: `picoquic/prague.c:144-154 picoquic_prague_get_pkt_ctx`
Rust: `rs/fq/src/prague.rs:71-80 prague_get_pkt_ctx`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = &cnx->pkt_ctx[picoquic_packet_context_application];

    /* Reset the L3S measurement context to the current value */
    if (cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }

    return pkt_ctx;
}
```

### Rust body
```rust
) -> &'a PacketContextState {
    if cnx.is_multipath_enabled {
        &path_x.pkt_ctx
    } else {
        &cnx.pkt_ctx[PacketContext::Application as usize]
    }
}
```

## Pair `picoquic/prague.c:picoquic_prague_enter_recovery`
C: `picoquic/prague.c:186-208 picoquic_prague_enter_recovery`
Rust: `rs/fq/src/prague.rs:132-142 prague_enter_recovery`

### C body
```c
{
    pr_state->ssthresh = path_x->cwin / 2;
    if (pr_state->ssthresh < PICOQUIC_CWIN_MINIMUM) {
        pr_state->ssthresh = PICOQUIC_CWIN_MINIMUM;
    }
    
    path_x->cwin = pr_state->ssthresh;
    pr_state->alg_state = picoquic_prague_alg_congestion_avoidance;

    picoquic_prague_initialize_era(cnx, path_x, pr_state, current_time);
}
```

### Rust body
```rust
) {
    pr_state.ssthresh = (path_x.cwin / 2).max(CWIN_MINIMUM);
    path_x.cwin = pr_state.ssthresh;
    pr_state.alg_state = PragueAlgState::CongestionAvoidance;
    prague_initialize_era(cnx, path_x, pr_state, current_time);
}
```

## Pair `picoquic/prague.c:picoquic_prague_notify`
C: `picoquic/prague.c:320-394 picoquic_prague_notify`
Rust: `rs/fq/src/prague.rs:296-305 picoquic_prague_notify`

### C body
```c
{
    picoquic_prague_state_t* pr_state = (picoquic_prague_state_t*)path_x->congestion_alg_state;

    if (pr_state != NULL) {
        switch (notification) {
        /* RTT measurements will happen before acknowledgement is signalled */
        case picoquic_congestion_notification_acknowledgement: {
            /* Increase or reduce the congestion window based on alpha */
            switch (pr_state->alg_state) {
            case picoquic_prague_alg_slow_start:
                picoquic_prague_process_start_ack(cnx, path_x, pr_state, ack_state, current_time);
                break;
            case picoquic_prague_alg_congestion_avoidance:
            default:
                picoquic_prague_process_ack(cnx, path_x, pr_state, ack_state, current_time);
                break;
            }
            break;
        }
        case picoquic_congestion_notification_ecn_ec:
            /* already managed as part of ACK */
            break;
        case picoquic_congestion_notification_repeat:
            /* enter recovery on loss. We should do nothing on timeout */
            if (picoquic_cc_hystart_loss_test(&pr_state->rtt_filter, notification, ack_state->lost_packet_number,
                PICOQUIC_SMOOTHED_LOSS_THRESHOLD) && current_time - pr_state->recovery_stamp > path_x->smoothed_rtt) {
                picoquic_prague_enter_recovery(cnx, path_x, pr_state, current_time);
            }
            break;
        case picoquic_congestion_notification_timeout:
            /* We should not react on PTO */
            break;
        case picoquic_congestion_notification_spurious_repeat:
            /* we should do nothing, since we do not react on PTO */
            break;
        case picoquic_congestion_notification_rtt_measurement:
            if (pr_state->alg_state == picoquic_prague_alg_slow_start &&
                pr_state->ssthresh == UINT64_MAX) {

                if (path_x->rtt_min > PICOQUIC_TARGET_RENO_RTT) {
                    path_x->cwin = picoquic_cc_update_cwin_for_long_rtt(path_x);
                }

                /* HyStart. */
                /* Using RTT increases as signal to get out of initial slow start */
                if (picoquic_cc_hystart_test(&pr_state->rtt_filter, (cnx->is_time_stamp_enabled) ? ack_state->one_way_delay : ack_state->rtt_measurement,
                    cnx->path[0]->pacing.packet_time_microsec, current_time,
                    cnx->is_time_stamp_enabled)) {
                    /* RTT increased too much, get out of slow start! */
                    pr_state->ssthresh = path_x->cwin;
                    pr_state->alg_state = picoquic_prague_alg_congestion_avoidance;
                    path_x->is_ssthresh_initialized = 1;
                }
            }
            break;
        case picoquic_congestion_notification_reset:
            picoquic_prague_reset(cnx, pr_state, path_x);
            break;
        default:
            /* ignore */
            break;
        }
        /* Compute pacing data */
        picoquic_update_pacing_data(path_x, pr_state->alg_state == picoquic_prague_alg_slow_start &&
            pr_state->ssthresh == UINT64_MAX);
    }
}
```

### Rust body
```rust
    let Some(boxed_state) = path_x.congestion_alg_state.take() else {
        return;
    };
```

## Pair `picoquic/quicctx.c:picoquic_retrieve_issued_ticket`
C: `picoquic/quicctx.c:426-442 picoquic_retrieve_issued_ticket`
Rust: `rs/fq/src/internal.rs:1869-1873 retrieve_issued_ticket`

### C body
```c
{
    picoquic_issued_ticket_t* ret = NULL;
    picohash_item* item;
    picoquic_issued_ticket_t key;

    memset(&key, 0, sizeof(key));
    key.ticket_id = ticket_id;

    item = picohash_retrieve(quic->table_issued_tickets, &key);

    if (item != NULL) {
        ret = (picoquic_issued_ticket_t*)item->key;
    }
    return ret;
}
```

### Rust body
```rust
    pub fn retrieve_issued_ticket(&mut self, ticket_id: u64) -> Option<&mut IssuedTicket> {
        let htok = self.issued_tickets_by_id.lookup(&ticket_id)?;
        let tok = *self.issued_tickets_by_id.get(htok)?;
        self.issued_tickets.get_mut(tok)
    }
```

## Pair `picoquic/quicctx.c:picoquic_registered_token_check_reuse`
C: `picoquic/quicctx.c:568-595 picoquic_registered_token_check_reuse`
Rust: `rs/fq/src/internal.rs:2177-2212 registered_token_check_reuse`

### C body
```c
{
    int ret = -1;
    if (token_length >= 8) {
        picoquic_registered_token_t* rt = (picoquic_registered_token_t*)malloc(sizeof(picoquic_registered_token_t));
        if (rt != NULL) {
            picosplay_node_t* rt_n = NULL;
            memset(rt, 0, sizeof(picoquic_registered_token_t));
            rt->token_time = expiry_time;
            rt->token_hash = PICOPARSE_64(token + token_length - 8);
            rt->count = 1;
            rt_n = picosplay_find(&quic->token_reuse_tree, rt);
            if (rt_n != NULL) {
                free(rt);
                rt = (picoquic_registered_token_t*)picoquic_registered_token_value(rt_n);
                rt->count++;
                DBG_PRINTF("Token reuse detected, count=%d", rt->count);
            }
            else {
                (void)picosplay_insert(&quic->token_reuse_tree, rt);
                ret = 0;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        if token_length < 8 {
            return Err(crate::Error::InvalidArgument);
        }
        // Last 8 bytes of token form the hash key (C: PICOPARSE_64(token + length - 8)).
        let token_hash = parse_64(&token[token_length - 8..token_length]);
        if let Some(rt_splay_tok) = self.token_reuse_tree.find(&token_hash) {
            // Already registered — increment count.
            if let Some(arena_tok) = self.token_reuse_tree.get(rt_splay_tok) {
                let arena_tok = *arena_tok;
                if let Some(rt) = self.registered_tokens.get_mut(arena_tok) {
                    rt.count += 1;
                }
            }
            Err(crate::Error::Generic) // token reuse detected
        } else {
            let rt = RegisteredToken {
                registered_token_membership: None,
                token_time: crate::Instant::from_ticks(expiry_time),
                token_hash,
                count: 1,
            };
            let rt_tok = self.registered_tokens.insert(rt)?;
            let (st, _) = self.token_reuse_tree.insert(token_hash, rt_tok)?;
            // Store the splay token back for O(1) removal.
            if let Some(entry) = self.registered_tokens.get_mut(rt_tok) {
                entry.registered_token_membership = Some(st);
            }
            Ok(())
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_create`
C: `picoquic/quicctx.c:633-775 picoquic_create`
Rust: `rs/fq/src/lib.rs:1334-1535 new`

### C body
```c
{
    picoquic_quic_t* quic = (picoquic_quic_t*)malloc(sizeof(picoquic_quic_t));
    int ret = 0;

    if (quic != NULL) {
        /* TODO: winsock init */
        /* TODO: open UDP sockets - maybe */
        memset(quic, 0, sizeof(picoquic_quic_t));

        quic->default_callback_fn = default_callback_fn;
        quic->default_callback_ctx = default_callback_ctx;
        quic->default_congestion_alg = PICOQUIC_DEFAULT_CONGESTION_ALGORITHM;
        quic->default_alpn = picoquic_string_duplicate(default_alpn);
        quic->cnx_id_callback_fn = cnx_id_callback;
        quic->cnx_id_callback_ctx = cnx_id_callback_ctx;
        quic->p_simulated_time = p_simulated_time;
        quic->local_cnxid_length = 8; /* TODO: should be lower on clients-only implementation */
        quic->padding_multiple_default = 0; /* TODO: consider default = 128 */
        quic->padding_minsize_default = PICOQUIC_RESET_PACKET_MIN_SIZE;
        quic->crypto_epoch_length_max = 0;
        quic->max_simultaneous_logs = PICOQUIC_DEFAULT_SIMULTANEOUS_LOGS;
        quic->max_half_open_before_retry = PICOQUIC_DEFAULT_HALF_OPEN_RETRY_THRESHOLD;
        quic->default_lossbit_policy = 0; /* For compatibility with old behavior. Consider 0 */
        quic->local_cnxid_ttl = UINT64_MAX;
        quic->stateless_reset_next_time = current_time;
        quic->stateless_reset_min_interval = PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT;
        quic->default_stream_priority = PICOQUIC_DEFAULT_STREAM_PRIORITY;
        quic->default_datagram_priority = PICOQUIC_DEFAULT_STREAM_PRIORITY;
        quic->cwin_max = UINT64_MAX;
        quic->sequence_hole_pseudo_period = PICOQUIC_DEFAULT_HOLE_PERIOD;

        picoquic_init_transport_parameters(&quic->default_tp);

        quic->random_initial = 1;
        picoquic_wake_list_init(quic);

        if (cnx_id_callback != NULL) {
            quic->unconditional_cnx_id = 1;
        }
        if (ticket_file_name != NULL) {
            quic->ticket_file_name = ticket_file_name;
        }

        if (ret == 0) {
            size_t max_cnx4 = 0;
            if (max_nb_connections == 0) {
                max_nb_connections = 1;
            }

            quic->tentative_max_number_connections = max_nb_connections;
            quic->max_number_connections = max_nb_connections;
            max_cnx4 = 4 * (size_t)max_nb_connections;



            if (max_cnx4 < (size_t)max_nb_connections ||
                (quic->table_cnx_by_id = picohash_create_ex((size_t)max_nb_connections * 4,
                picoquic_local_cnxid_hash, picoquic_local_cnxid_compare, picoquic_local_cnxid_to_item, quic->hash_seed)) == NULL ||
                (quic->table_cnx_by_net = picohash_create_ex((size_t)max_nb_connections * 4,
                    picoquic_net_id_hash, picoquic_net_id_compare, picoquic_local_netid_to_item, quic->hash_seed)) == NULL ||
                (quic->table_cnx_by_icid = picohash_create_ex((size_t)max_nb_connections,
                    picoquic_net_icid_hash, picoquic_net_icid_compare, picoquic_net_icid_to_item, quic->hash_seed)) == NULL ||
                (quic->table_cnx_by_secret = picohash_create_ex((size_t)max_nb_connections * 4,
                    picoquic_net_secret_hash, picoquic_net_secret_compare, picoquic_net_secret_to_item, quic->hash_seed)) == NULL ||
                (quic->table_issued_tickets = picohash_create_ex((size_t)max_nb_connections,
                    picoquic_issued_ticket_hash, picoquic_issued_ticket_compare, picoquic_issued_ticket_key_to_item, quic->hash_seed)) == NULL) {
                ret = -1;
                DBG_PRINTF("%s", "Cannot initialize hash tables\n");
            }
            else {
                picosplay_init_tree(&quic->token_reuse_tree, picoquic_registered_token_compare,
                    picoquic_registered_token_create, picoquic_registered_token_delete, picoquic_registered_token_value);
                if (picoquic_master_tlscontext(quic, cert_file_name, key_file_name, cert_root_file_name, ticket_encryption_key, ticket_encryption_key_length) != 0) {
                    ret = -1;
                    DBG_PRINTF("%s", "Cannot create TLS context \n");
                }
                else {
                    /* In the absence of certificate or key, we assume that this is a client only context */
                    quic->enforce_client_only = (cert_file_name == NULL || key_file_name == NULL);
                    /* the random generator was initialized as part of the TLS context.
                     * Use it to create the seed for generating the per context stateless
                     * resets and the retry tokens */

                    if (!reset_seed)
                        picoquic_crypto_random(quic, quic->reset_seed, sizeof(quic->reset_seed));
                    else
                        memcpy(quic->reset_seed, reset_seed, sizeof(quic->reset_seed));

                    picoquic_crypto_random(quic, quic->retry_seed, sizeof(quic->retry_seed));
                    picoquic_crypto_random(quic, quic->hash_seed, sizeof(quic->hash_seed));

                    /* If there is no root certificate context specified, use a null certifier. */
                    /* Load tickets */
                    if (quic->ticket_file_name != NULL) {
                        ret = picoquic_load_tickets(quic, ticket_file_name);

                        if (ret == PICOQUIC_ERROR_NO_SUCH_FILE) {
                            DBG_PRINTF("Ticket file <%s> not created yet.\n", ticket_file_name);
                            ret = 0;
                        }
                        else if (ret != 0) {
                            DBG_PRINTF("Cannot load tickets from <%s>\n", ticket_file_name);
                            ret = 0;
                        }
                    }
                }
            }
        }
#ifdef BBRExperiment
        if (ret == 0) {
            quic->bbr_exp_flags.do_early_exit = 1;
            quic->bbr_exp_flags.do_rapid_start = 1;
            quic->bbr_exp_flags.do_handle_suspension = 1;
            quic->bbr_exp_flags.do_control_lost = 1;
            quic->bbr_exp_flags.do_exit_probeBW_up_on_delay = 1;
            quic->bbr_exp_flags.do_enter_probeBW_after_limited = 1;
        }
#endif

        if (ret != 0) {
            picoquic_free(quic);
            quic = NULL;
        }
    }

    return quic;
}
```

### Rust body
```rust
    ) -> Option<Box<Quic>> {
        // C: if max_nb_connections == 0, clamp to 1.
        if max_nb_connections == 0 {
            max_nb_connections = 1;
        }

        // C: enforce_client_only = (cert_file_name == NULL || key_file_name == NULL)
        let enforce_client_only = cert_file_name.is_none() || key_file_name.is_none();

        let unconditional_cnx_id = cnx_id_callback.is_some();

        // The C body allocates hash tables before it refreshes
        // quic->hash_seed, so the tables are created with the zeroed
        // initial seed and keep their own copy of it.
        let table_seed = [0u8; 16];
        let nb_bin = (max_nb_connections as usize).saturating_mul(4);
        let nb_bin_small = max_nb_connections as usize;
        let table_cnx_by_id = crate::hash::HashTable::with_seed(nb_bin, &table_seed).ok()?;
        let table_cnx_by_net = crate::hash::HashTable::with_seed(nb_bin, &table_seed).ok()?;
        let table_cnx_by_icid =
            crate::hash::HashTable::with_seed(nb_bin_small, &table_seed).ok()?;
        let table_cnx_by_secret = crate::hash::HashTable::with_seed(nb_bin, &table_seed).ok()?;
        let table_issued_tickets =
            crate::hash::HashTable::with_seed(nb_bin_small, &table_seed).ok()?;

        struct SystemRandom;
        impl rand_core::RngCore for SystemRandom {
            fn next_u32(&mut self) -> u32 {
                let mut bytes = [0u8; 4];
                self.fill_bytes(&mut bytes);
                u32::from_le_bytes(bytes)
            }
            fn next_u64(&mut self) -> u64 {
                let mut bytes = [0u8; 8];
                self.fill_bytes(&mut bytes);
                u64::from_le_bytes(bytes)
            }
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                use std::io::Read;

                std::fs::File::open("/dev/urandom")
                    .and_then(|mut file| file.read_exact(dest))
                    .expect("failed to read random bytes from /dev/urandom");
            }
        }
        impl rand_core::CryptoRng for SystemRandom {}

        let mut rng = SystemRandom;
        let mut retry_seed = [0u8; crate::internal::RETRY_SECRET_SIZE];
        rand_core::RngCore::fill_bytes(&mut rng, &mut retry_seed);
        let mut hash_seed = [0u8; 16];
        rand_core::RngCore::fill_bytes(&mut rng, &mut hash_seed);
        let mut default_tp = crate::tp::TransportParameters::default();
        crate::internal::init_transport_parameters(&mut default_tp);

        let mut quic = Box::new(internal::Quic {
            tls_client_config: None,
            tls_server_config: None,
            tls_callbacks: None,
            default_callback_fn: default_callback,
            default_callback_ctx: None,
            mask_ctx: None,
            mask_fns: None,
            default_alpn: default_alpn.map(|s| s.to_owned()),
            alpn_select_fn: None,
            reset_seed,
            retry_seed,
            rng: Box::new(rng),
            hash_seed,
            ticket_file_name: ticket_file_name.map(std::path::PathBuf::from),
            token_file_name: None,
            stored_tickets: Vec::new(),
            stored_tokens: Vec::new(),
            token_reuse_tree: crate::splay::SplayTree::default(),
            registered_tokens: crate::arena::Arena::new(),
            local_connection_id_length: 8,
            default_stream_priority: DEFAULT_STREAM_PRIORITY,
            default_datagram_priority: DEFAULT_STREAM_PRIORITY,
            local_connection_id_ttl: u64::MAX,
            mtu_max: 0,
            padding_multiple_default: 0,
            padding_minsize_default: RESET_PACKET_MIN_SIZE as u32,
            sequence_hole_pseudo_period: crate::internal::DEFAULT_HOLE_PERIOD as u32,
            default_pmtud_policy: PmtudPolicy::default(),
            default_spin_policy: SpinbitVersion::default(),
            default_lossbit_policy: LossbitVersion::default(),
            default_multipath_option: 0,
            default_handshake_timeout: crate::Duration::from_ticks(0),
            crypto_epoch_length_max: 0,
            max_simultaneous_logs: crate::internal::DEFAULT_SIMULTANEOUS_LOGS,
            current_number_of_open_logs: 0,
            max_half_open_before_retry: crate::internal::DEFAULT_HALF_OPEN_RETRY_THRESHOLD,
            current_number_half_open: 0,
            current_number_connections: 0,
            tentative_max_number_connections: max_nb_connections,
            max_number_connections: max_nb_connections,
            stateless_reset_next_time: current_time,
            stateless_reset_min_interval:
                crate::internal::MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT,
            cwin_max: u64::MAX,
            check_token: false,
            force_check_token: false,
            provide_token: false,
            unconditional_cnx_id,
            client_zero_share: false,
            server_busy: false,
            is_cert_store_not_empty: false,
            use_long_log: false,
            should_close_log: false,
            enable_sslkeylog: false,
            use_unique_log_names: false,
            dont_coalesce_init: false,
            one_way_grease_quic_bit: false,
            random_initial: 1,
            packet_train_mode: false,
            use_constant_challenges: false,
            use_low_memory: false,
            is_preemptive_repeat_enabled: false,
            default_send_receive_bdp_frame: false,
            enforce_client_only,
            test_large_server_flight: false,
            is_port_blocking_disabled: false,
            are_path_callbacks_enabled: false,
            use_predictable_random: false,
            client_authentication: false,
            use_exporter: false,
            ech_opener: None,
            ech_server_retry_config: None,
            ech_client_enabled: false,
            pending_stateless_packets: std::collections::VecDeque::new(),
            default_congestion_alg: Some(&NEWRENO_ALGORITHM),
            default_congestion_alg_option_string: None,
            connections: crate::arena::Arena::new(),
            connection_wake_tree: crate::splay::SplayTree::default(),
            connection_in_progress: None,
            connection_by_id: table_cnx_by_id,
            connection_by_net: table_cnx_by_net,
            connection_by_icid: table_cnx_by_icid,
            connection_by_secret: table_cnx_by_secret,
            issued_tickets_by_id: table_issued_tickets,
            issued_tickets: crate::arena::Arena::new(),
            nb_packets_allocated: 0,
            nb_packets_allocated_max: 0,
            nb_data_nodes_allocated: 0,
            nb_data_nodes_allocated_max: 0,
            connection_id_callback_fn: cnx_id_callback,
            connection_id_callback_ctx: None,
            aead_encrypt_ticket_ctx: None,
            aead_decrypt_ticket_ctx: None,
            retry_integrity_sign_ctx: Vec::new(),
            retry_integrity_verify_ctx: Vec::new(),
            default_tp,
            fuzz_fn: None,
            fuzz_ctx: None,
            wake_file: 0,
            wake_line: 0,
            max_data_limit: 0,
            rtt_update_delta: crate::Duration::from_ticks(0),
            pacing_rate_update_delta: 0,
            f_log: None,
            binlog_dir: None,
            qlog_dir: None,
            autoqlog_fn: None,
            text_log_fns: None,
            bin_log_fns: None,
            qlog_fns: None,
            perflog_fn: None,
            v_perflog_ctx: None,
            v_thread_ctx: None,
        });
        quic.wake_list_init();

        if quic
            .init_master_tls_context(
                cert_file_name,
                key_file_name,
                _cert_root_file_name,
                ticket_encryption_key,
            )
            .is_err()
        {
            return None;
        }

        if let Some(ticket_file_name) = ticket_file_name {
            let _ = quic.load_tickets(ticket_file_name);
        }

        Some(quic)
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_tp_value_by_type`
C: `picoquic/quicctx.c:819-891 picoquic_set_tp_value_by_type`
Rust: `rs/fq/src/lib.rs:1267-1297 set_tp_value_by_type`

### C body
```c
{
    int ret = 0;
    switch (tp_type) {
    case picoquic_tp_idle_timeout:
        tp->max_idle_timeout = tp_value;
        break;
    case picoquic_tp_max_packet_size:
        tp->max_packet_size = (uint32_t)tp_value;
        break;
    case picoquic_tp_initial_max_data:
        tp->initial_max_data = tp_value;
        break;
    case picoquic_tp_initial_max_stream_data_bidi_local:
        tp->initial_max_stream_data_bidi_local = tp_value;
        break;
    case picoquic_tp_initial_max_stream_data_bidi_remote:
        tp->initial_max_stream_data_bidi_remote = tp_value;
        break;
    case picoquic_tp_initial_max_stream_data_uni:
        tp->initial_max_stream_data_uni = tp_value;
        break;
    case picoquic_tp_initial_max_streams_bidi:
        tp->initial_max_stream_id_bidir = tp_value;
        break;
    case picoquic_tp_initial_max_streams_uni:
        tp->initial_max_stream_id_unidir = tp_value;
        break;
    case picoquic_tp_ack_delay_exponent:
        tp->ack_delay_exponent = (uint8_t)tp_value;
        break;
    case picoquic_tp_max_ack_delay:
        tp->max_ack_delay = (uint32_t)tp_value;
        break;
    case picoquic_tp_disable_migration:
        tp->migration_disabled = (tp_value != 0);
        break;
    case picoquic_tp_active_connection_id_limit:
        tp->active_connection_id_limit = (uint32_t)tp_value;
        break;
    case picoquic_tp_max_datagram_frame_size:
        tp->max_datagram_frame_size = (uint32_t)tp_value;
        break;
    case picoquic_tp_enable_loss_bit:
        tp->enable_loss_bit = (tp_value != 0);
        break;
    case picoquic_tp_min_ack_delay:
        tp->min_ack_delay = tp_value;
        break;
    case picoquic_tp_enable_time_stamp:
        tp->enable_time_stamp = (int)tp_value;
        break;
    case picoquic_tp_grease_quic_bit:
        tp->max_idle_timeout = (tp_value != 0);
        break;
    case picoquic_tp_enable_bdp_frame:
        tp->enable_bdp_frame = (tp_value != 0);
        break;
    case picoquic_tp_initial_max_path_id:
        tp->initial_max_path_id = tp_value;
        break;
    case picoquic_tp_address_discovery:
        tp->max_idle_timeout = (int)tp_value;
        break;
    case picoquic_tp_reset_stream_at:
        tp->is_reset_stream_at_enabled = (tp_value != 0);
        break;
    default:
        ret = -1;
        break;
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    match tp_type {
        1 => tp.max_idle_timeout = Duration::from_ticks(tp_value),
        3 => tp.max_packet_size = tp_value as u32,
        4 => tp.initial_max_data = tp_value,
        5 => tp.initial_max_stream_data_bidi_local = tp_value,
        6 => tp.initial_max_stream_data_bidi_remote = tp_value,
        7 => tp.initial_max_stream_data_uni = tp_value,
        8 => tp.initial_max_stream_id_bidir = tp_value,
        9 => tp.initial_max_stream_id_unidir = tp_value,
        10 => tp.ack_delay_exponent = tp_value as u8,
        11 => tp.max_ack_delay = tp_value as u32,
        12 => tp.migration_disabled = tp_value != 0,
        14 => tp.active_connection_id_limit = tp_value as u32,
        32 => tp.max_datagram_frame_size = tp_value as u32,
        0x1057 => tp.enable_loss_bit = (tp_value != 0) as i32,
        0xff04de1b => tp.min_ack_delay = Duration::from_ticks(tp_value),
        0x7158 => tp.enable_time_stamp = tp_value as i32,
        0x2ab2 => tp.do_grease_quic_bit = tp_value != 0,
        0xebd9 => tp.enable_bdp_frame = tp_value != 0,
        0x3e => tp.initial_max_path_id = tp_value,
        0x9f81a176 => tp.address_discovery_mode = tp_value as i32,
        0x17f7586d2cb571 => tp.is_reset_stream_at_enabled = tp_value != 0,
        _ => return Err(Error::InvalidArgument),
    }
    Ok(())
}
```

## Pair `picoquic/quicctx.c:picoquic_set_spinbit_policy`
C: `picoquic/quicctx.c:920-933 picoquic_set_spinbit_policy`
Rust: `rs/fq/src/lib.rs:1774-1781 set_spinbit_policy`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (spinbit_policy < picoquic_spinbit_on) {
        cnx->spin_policy = spinbit_policy;
    }
    else
    {
        ret = -1;
    }
    return ret;
}
```

### Rust body
```rust
    pub fn set_spinbit_policy(&mut self, spinbit_policy: SpinbitVersion) -> Result<(), Error> {
        // SpinbitVersion::On is server-only.
        if spinbit_policy == SpinbitVersion::On {
            return Err(Error::InvalidArgument);
        }
        self.spin_policy = spinbit_policy;
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_cwin_max`
C: `picoquic/quicctx.c:964-968 picoquic_set_cwin_max`
Rust: `rs/fq/src/lib.rs:1726-1734 set_cwin_max`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->cwin_max = (cwin_max == 0) ? UINT64_MAX : cwin_max;
}
```

### Rust body
```rust
    pub fn cwin_max(&self) -> u64 {
        self.cwin_max
    }
```
