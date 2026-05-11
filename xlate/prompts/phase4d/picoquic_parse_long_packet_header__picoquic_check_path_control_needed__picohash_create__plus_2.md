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

## `picoquic/packet.c:picoquic_parse_long_packet_header`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C fully parses long headers, packet types, payload lengths, connection lookup, and error cases; Rust only checks length < 5 and returns None.
* C source: `picoquic/packet.c:210-394`
* C signature: `int picoquic_parse_long_packet_header(picoquic_quic_t *, const uint8_t *, size_t, const struct sockaddr *, picoquic_packet_header *, picoquic_cnx_t **)`
* Rust source: `rs/fq/src/internal.rs:6602-6612`
* Rust item: `parse_long_packet_header_inner`

### C body
```c
{
    int ret = 0;

    const uint8_t* bytes_start = bytes;
    const uint8_t* bytes_max = bytes + length;
    uint8_t flags = 0;

    if ((bytes = picoquic_frames_uint8_decode(bytes, bytes_max, &flags)) == NULL ||
        (bytes = picoquic_frames_uint32_decode(bytes, bytes_max, &ph->vn)) == NULL)
    {
        ret = -1;
    }
    else if (ph->vn != 0) {
        ph->version_index = picoquic_get_version_index(ph->vn);
        if (ph->version_index < 0) {
            DBG_PRINTF("Version is not recognized: 0x%08x\n", ph->vn);
            ph->ptype = picoquic_packet_error;
            ph->pc = 0;
            ret = PICOQUIC_ERROR_VERSION_NOT_SUPPORTED;
        }
    }
    
    if (ret == 0 && (
        (bytes = picoquic_frames_cid_decode(bytes, bytes_max, &ph->dest_cnx_id)) == NULL ||
        (bytes = picoquic_frames_cid_decode(bytes, bytes_max, &ph->srce_cnx_id)) == NULL)) {
        ret = -1;
    }

    if (ret == 0) {
        ph->offset = bytes - bytes_start;

        if (ph->vn == 0) {
            /* VN = zero identifies a version negotiation packet */
            ph->ptype = picoquic_packet_version_negotiation;
            ph->pc = picoquic_packet_context_initial;
            ph->payload_length = (uint16_t)((length > ph->offset) ? length - ph->offset : 0);
            ph->pl_val = ph->payload_length; /* saving the value found in the packet */

            if (*pcnx == NULL && quic != NULL) {
                /* The version negotiation should always include the cnx-id sent by the client */
                if (quic->local_cnxid_length == 0) {
                    *pcnx = picoquic_cnx_by_net(quic, addr_from);
                }
                else if (ph->dest_cnx_id.id_len == quic->local_cnxid_length) {
                    *pcnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
                }
            }
        }
        else {
            size_t payload_length = 0;
            /* If the version is supported now, the format field in the version table
            * describes the encoding. */
            ph->spin = 0;
            ph->has_spin_bit = 0;
            ph->quic_bit_is_zero = (flags & 0x40) == 0;

            /* The first byte is defined in RFC 9000 as:
             *     Header Form (1) = 1,
             *     Fixed Bit (1) = 1,
             *     Long Packet Type (2),
             *     Type-Specific Bits (4)
             * The packet type is version dependent. In fact, the whole first byte is version
             * dependent, the invariant draft only specifies the "header form" bit = 1 for long
             * header. In version 1, the packet specific bytes are two reserved bytes +
             * sequence number length. We assume the same for version 2.
             */
            ph->ptype = picoquic_parse_long_packet_type(flags, ph->version_index);
            switch (ph->ptype) {
            case picoquic_packet_initial: /* Initial */
            {
                /* special case of the initial packets. They contain a retry token between the header
                * and the encrypted payload */
                size_t tok_len = 0;
                bytes = picoquic_frames_varlen_decode(bytes, bytes_max, &tok_len);

                size_t bytes_left = bytes_max - bytes;

                ph->epoch = picoquic_epoch_initial;
                if (bytes == NULL || bytes_left < tok_len) {
                    /* packet is malformed */
                    ph->ptype = picoquic_packet_error;
                    ph->pc = 0;
                    ph->offset = length;
                }
                else {
                    ph->pc = picoquic_packet_context_initial;
                    ph->token_length = tok_len;
                    ph->token_bytes = bytes;
                    bytes += tok_len;
                    ph->offset = bytes - bytes_start;
                }

                break;
            }
            case picoquic_packet_0rtt_protected: /* 0-RTT Protected */
                ph->pc = picoquic_packet_context_application;
                ph->epoch = picoquic_epoch_0rtt;
                break;
            case picoquic_packet_handshake: /* Handshake */
                ph->pc = picoquic_packet_context_handshake;
                ph->epoch = picoquic_epoch_handshake;
                break;
            case picoquic_packet_retry: /* Retry */
            default:
                /* No default branch in this statement, because there are only 4 possible types
                 * parsed in picoquic_parse_long_packet_type */
                ph->pc = picoquic_packet_context_initial;
                ph->epoch = picoquic_epoch_initial;
                break;
            }

            if (ph->ptype == picoquic_packet_retry) {
                /* No segment length or sequence number in retry packets */
                if (length > ph->offset) {
                    payload_length = length - ph->offset;
                }
                else {
                    payload_length = 0;
                    ph->ptype = picoquic_packet_error;
                }
            }
            else if (ph->ptype != picoquic_packet_error) {
                bytes = picoquic_frames_varlen_decode(bytes, bytes_max, &payload_length);

                size_t bytes_left = (bytes_max > bytes) ? bytes_max - bytes : 0;
                if (bytes == NULL || bytes_left < payload_length || ph->version_index < 0) {
                    ph->ptype = picoquic_packet_error;
                    ph->payload_length = (uint16_t)((length > ph->offset) ? length - ph->offset : 0);
                    ph->pl_val = ph->payload_length;
                }
            }

            if (ph->ptype != picoquic_packet_error)
            {
                ph->pl_val = (uint16_t)payload_length;
                ph->payload_length = (uint16_t)payload_length;
                ph->offset = bytes - bytes_start;
                ph->pn_offset = ph->offset;

                /* Retrieve the connection context */
                if (*pcnx == NULL) {
                    if (quic->local_cnxid_length == 0) {
                        *pcnx = picoquic_cnx_by_net(quic, addr_from);
                    }
                    else
                    {
                        if (ph->dest_cnx_id.id_len == quic->local_cnxid_length) {
                            *pcnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
                        }

                        if (*pcnx == NULL && (ph->ptype == picoquic_packet_initial || ph->ptype == picoquic_packet_0rtt_protected)) {
                            *pcnx = picoquic_cnx_by_icid(quic, &ph->dest_cnx_id, addr_from);
                        }
                        else if (*pcnx == NULL) {
                            DBG_PRINTF("Dropped packet of type %d, no connection", ph->ptype);
                        }
                    }
                }

                if (ph->quic_bit_is_zero && *pcnx != NULL && !(*pcnx)->local_parameters.do_grease_quic_bit) {
                    ph->ptype = picoquic_packet_error;
                }
            }
            else {
                /* Try to find the connection context, for logging purpose. */
                if (*pcnx == NULL) {
                    if (quic->local_cnxid_length == 0) {
                        *pcnx = picoquic_cnx_by_net(quic, addr_from);
                    }
                    else if (ph->dest_cnx_id.id_len == quic->local_cnxid_length) {
                        *pcnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
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
        if bytes.len() < 5 {
            ph.packet_type = PacketType::Error;
            return None;
        }
```

## `picoquic/paths.c:picoquic_check_path_control_needed`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns NULL when no tuple is selected and also records a sender wake side effect when next_wake_time changes; Rust shows no final None/default return and no corresponding wake side effect.
* C source: `picoquic/paths.c:288-321`
* C signature: `picoquic_tuple_t * picoquic_check_path_control_needed(picoquic_cnx_t *, picoquic_path_t *, uint64_t, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:4599-4622`
* Rust item: `check_path_control_needed`

### C body
```c
{
    /* examine each tuple record */
    picoquic_tuple_t* tuple = path_x->first_tuple;

    while (tuple != NULL) {
        if (tuple->challenge_failed) {
            if (tuple != path_x->first_tuple && current_time > tuple->demotion_time) {
                cnx->tuple_demotion_needed = 1;
            }
            /* go to next tuple */
        }
        else if (tuple->response_required) {
            /* selected */
            break;
        }
        else if (tuple->challenge_required && !tuple->challenge_verified) {
            uint64_t next_challenge_time = picoquic_tuple_challenge_time(path_x, tuple, current_time);
            if (current_time >= next_challenge_time) {
                break;
            }
            else if (next_challenge_time < *next_wake_time) {
                *next_wake_time = next_challenge_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
            }
        }
        tuple = tuple->next_tuple;
    }
    return tuple;
}
```

### Rust body
```rust
        for tuple_index in 0..path_x.tuples.len() {
            let tuple = &path_x.tuples[tuple_index];
            if tuple.challenge_failed {
                if tuple_index != 0 && current_time > tuple.demotion_time {
                    self.tuple_demotion_needed = true;
                }
            } else if tuple.response_required {
                return Some(tuple_index);
            } else if tuple.challenge_required && !tuple.challenge_verified {
                let next_challenge_time = tuple_challenge_time(path_x, tuple, current_time);
                if current_time >= next_challenge_time {
                    return Some(tuple_index);
                }
                if next_challenge_time < *next_wake_time {
                    *next_wake_time = next_challenge_time;
                }
            }
        }
```

## `picoquic/picohash.c:picohash_create`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns a created hash table via picohash_create_ex; Rust body is an unrelated trait definition for thread naming.
* C source: `picoquic/picohash.c:60-65`
* C signature: `picohash_table * picohash_create(size_t, uint64_t (*)(const void *, const uint8_t *), int (*)(const void *, const void *))`
* Rust source: `rs/fq/src/packet_loop.rs:395-411`
* Rust item: `create`

### C body
```c
{
    return picohash_create_ex(nb_bin, picohash_hash, picohash_compare, NULL, NULL);
}
```

### Rust body
```rust
pub trait CustomThreadSetnameFn {
    /// Apply `thread_name` to the *current* thread.  Called from
    /// inside the thread after it starts, per the C convention.
    fn set_name(&mut self, thread_name: &str);
}
```

## `picoquic/picosocks.c:picoquic_bind_to_port`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C builds a sockaddr and calls bind; Rust ignores both arguments and always returns a generic error.
* C source: `picoquic/picosocks.c:25-50`
* C signature: `int picoquic_bind_to_port(int, int, int)`
* Rust source: `rs/fq/src/socks.rs:105-107`
* Rust item: `bind_to_port`

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

## `picoquic/picosocks.c:picoquic_socks_cmsg_format`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C builds control messages for source address, interface, dont-frag, and UDP segmentation; Rust body is an explicit no-op placeholder comment.
* C source: `picoquic/picosocks.c:642-736`
* C signature: `void picoquic_socks_cmsg_format(void *, size_t, size_t, struct sockaddr *, int)`
* Rust source: `rs/fq/src/socks.rs:544-554`
* Rust item: `format_cmsg`

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
