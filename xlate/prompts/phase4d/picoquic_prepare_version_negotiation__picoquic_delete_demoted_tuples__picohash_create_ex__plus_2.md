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

## `picoquic/packet.c:picoquic_prepare_version_negotiation`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C builds and queues a complete version negotiation packet; Rust body only checks original_bytes.len() < 7 and returns.
* C source: `picoquic/packet.c:994-1075`
* C signature: `void picoquic_prepare_version_negotiation(picoquic_quic_t *, struct sockaddr *, struct sockaddr *, unsigned long, picoquic_packet_header *, uint8_t *)`
* Rust source: `rs/fq/src/internal.rs:643-653`
* Rust item: `prepare_version_negotiation`

### C body
```c
{
    picoquic_cnx_t* cnx = NULL;
    uint8_t dcid_length = original_bytes[5];
    uint8_t * dcid = original_bytes + 6;
    uint8_t scid_length = original_bytes[6 + dcid_length];
    uint8_t* scid = original_bytes + 6 + dcid_length + 1;

    /* Verify that this is not a spurious error by checking whether a connection context
     * already exists */
    if (dcid_length <= PICOQUIC_CONNECTION_ID_MAX_SIZE) {
        (void) picoquic_parse_connection_id(dcid, dcid_length, &ph->dest_cnx_id);
        if (ph->dest_cnx_id.id_len == quic->local_cnxid_length) {
            if (quic->local_cnxid_length == 0) {
                cnx = picoquic_cnx_by_net(quic, addr_from);
            }
            else {
                cnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
            }
        }
        if (cnx == NULL) {
            cnx = picoquic_cnx_by_icid(quic, &ph->dest_cnx_id, addr_from);
        }
    }

    /* If no connection context exists, send back a version negotiation */
    if (cnx == NULL) {
        picoquic_stateless_packet_t* sp = picoquic_create_stateless_packet(quic);

        if (sp != NULL) {
            uint8_t* bytes = sp->bytes;
            size_t byte_index = 0;
            uint32_t rand_vn;

            /* Packet type set to random value for version negotiation */
            picoquic_public_random(bytes + byte_index, 1);
            bytes[byte_index++] |= 0x80;
            /* Set the version number to zero */
            picoformat_32(bytes + byte_index, 0);
            byte_index += 4;

            /* Copy the connection identifiers */
            bytes[byte_index++] = scid_length;
            memcpy(bytes + byte_index, scid, scid_length);
            byte_index += scid_length;
            bytes[byte_index++] = dcid_length;
            memcpy(bytes + byte_index, dcid, dcid_length);
            byte_index += dcid_length;

            /* Set the payload to the list of versions */
            for (size_t i = 0; i < picoquic_nb_supported_versions; i++) {
                picoformat_32(bytes + byte_index, picoquic_supported_versions[i].version);
                byte_index += 4;
            }
            /* Add random reserved value as grease, but be careful to not match proposed version */
            do {
                rand_vn = (((uint32_t)picoquic_public_random_64()) & 0xF0F0F0F0) | 0x0A0A0A0A;
            } while (rand_vn == ph->vn);
            picoformat_32(bytes + byte_index, rand_vn);
            byte_index += 4;

            /* Set length and addresses, and queue. */
            sp->length = byte_index;
            picoquic_store_addr(&sp->addr_to, addr_from);
            picoquic_store_addr(&sp->addr_local, addr_to);
            sp->if_index_local = if_index_to;
            sp->initial_cid = ph->dest_cnx_id;
            sp->cnxid_log64 = picoquic_val64_connection_id(sp->initial_cid);
            sp->ptype = picoquic_packet_version_negotiation;

            picoquic_log_quic_pdu(quic, 1, picoquic_get_quic_time(quic), 0, addr_to, addr_from, sp->length);

            picoquic_queue_stateless_packet(quic, sp);
        }
    }
}
```

### Rust body
```rust
        if original_bytes.len() < 7 {
            return;
        }
```

## `picoquic/paths.c:picoquic_delete_demoted_tuples`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C iterates non-demoted paths and deletes failed tuples after the first tuple; Rust instead drains paths, checks path_is_demoted and path.demotion_time, and clears all tuples on expired demoted paths.
* C source: `picoquic/paths.c:259-286`
* C signature: `void picoquic_delete_demoted_tuples(picoquic_cnx_t *, uint64_t, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:4349-4362`
* Rust item: `delete_demoted_tuples`

### C body
```c
{
    for (int path_index = 0; path_index < cnx->nb_paths; path_index++) {
        picoquic_path_t* path_x = cnx->path[path_index];
        if (!path_x->path_is_demoted) {
            /* examine each tuple record */
            picoquic_tuple_t* tuple = path_x->first_tuple;
            picoquic_tuple_t* next_tuple;

            while (tuple != NULL && (next_tuple = tuple->next_tuple) != NULL) {
                if (next_tuple->challenge_failed) {
                    if (current_time > next_tuple->demotion_time) {
                        picoquic_delete_tuple(path_x, next_tuple, 0);
                        continue;
                    }
                    else if (*next_wake_time > next_tuple->demotion_time) {
                        *next_wake_time = next_tuple->demotion_time;
                        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                    }
                }
                tuple = next_tuple;
            }
        }
    }
    cnx->tuple_demotion_needed = 0;
}
```

### Rust body
```rust
    pub fn delete_demoted_tuples(&mut self, current_time: Instant, next_wake_time: &mut Instant) {
        let mut retained = Vec::with_capacity(self.paths.len());
        for mut path in self.paths.drain(..) {
            if path.path_is_demoted && path.demotion_time <= current_time {
                path.tuples.clear();
            } else {
                if path.path_is_demoted && path.demotion_time < *next_wake_time {
                    *next_wake_time = path.demotion_time;
                }
                retained.push(path);
            }
        }
        self.paths = retained;
    }
```

## `picoquic/picohash.c:picohash_create_ex`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C allocates and initializes a hash table, while the Rust body shown hashes a single key and is not a constructor.
* C source: `picoquic/picohash.c:30-58`
* C signature: `picohash_table * picohash_create_ex(size_t, uint64_t (*)(const void *, const uint8_t *), int (*)(const void *, const void *), picohash_item *(*)(const void *), const uint8_t *)`
* Rust source: `rs/fq/src/hash.rs:193-211`
* Rust item: `with_seed`

### C body
```c
{
    static const uint8_t null_seed[16] = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 };
    picohash_table* t = (picohash_table*)malloc(sizeof(picohash_table));
    size_t items_length = sizeof(picohash_item*) * nb_bin;
    t->hash_bin = NULL;
    if (t != NULL && (items_length / sizeof(picohash_item*)) == nb_bin) {
        t->hash_bin = (picohash_item**)malloc(sizeof(picohash_item*) * nb_bin);
    }

    if (t->hash_bin == NULL) {
        free(t);
        t = NULL;
    } else {
        (void)memset(t->hash_bin, 0, sizeof(picohash_item*) * nb_bin);
        t->nb_bin = nb_bin;
        t->count = 0;
        t->picohash_hash = picohash_hash;
        t->picohash_compare = picohash_compare;
        t->picohash_key_to_item = picohash_key_to_item;
        t->hash_seed = (hash_seed == NULL)? null_seed: hash_seed;
    }

    return t;
}
```

### Rust body
```rust
    fn hash_key(&self, key: &K) -> u64 {
        let mut h = SeedHasher::new(&self.seed);
        key.hash(&mut h);
        h.finish()
    }
```

## `picoquic/picosocks.c:picoquic_get_local_address`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C calls getsockname and returns its result; Rust always returns Err(Error::Generic).
* C source: `picoquic/picosocks.c:52-56`
* C signature: `int picoquic_get_local_address(int, struct sockaddr_storage *)`
* Rust source: `rs/fq/src/socks.rs:72-101`
* Rust item: `local_address`

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

## `picoquic/picosocks.c:picoquic_socks_cmsg_parse`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C parses control messages and writes destination address/interface/ECN outputs; Rust body is an explicit no-op comment.
* C source: `picoquic/picosocks.c:422-497`
* C signature: `void picoquic_socks_cmsg_parse(void *, struct sockaddr_storage *, int *, unsigned char *, size_t *)`
* Rust source: `rs/fq/src/socks.rs:534-554`
* Rust item: `parse_cmsg`

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
