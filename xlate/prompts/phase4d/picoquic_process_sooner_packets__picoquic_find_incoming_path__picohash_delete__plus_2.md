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

## `picoquic/packet.c:picoquic_process_sooner_packets`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C retries eligible queued packets and deletes only processed ones; Rust unconditionally clears the queued packets.
* C source: `picoquic/packet.c:2449-2516`
* C signature: `void picoquic_process_sooner_packets(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:14976-14978`
* Rust item: `process_sooner_packets`

### C body
```c
{
    picoquic_stateless_packet_t* packet = cnx->first_sooner;
    picoquic_stateless_packet_t* previous = NULL;

    cnx->recycle_sooner_needed = 0;

    while (packet != NULL) {
        picoquic_stateless_packet_t* next_packet = packet->next_packet;
        int could_try_now = 1;
        picoquic_epoch_enum epoch = 0;
        switch (packet->ptype) {
        case picoquic_packet_handshake:
            epoch = picoquic_epoch_handshake;
            break;
        case picoquic_packet_1rtt_protected:
            epoch = picoquic_epoch_1rtt;
            break;
        default:
            could_try_now = 0;
            break;
        }

        if (could_try_now &&
            (cnx->crypto_context[epoch].aead_decrypt != NULL || cnx->crypto_context[epoch].pn_dec != NULL))
        {
            size_t consumed_index = 0;
            int ret = 0;
            picoquic_connection_id_t previous_destid = picoquic_null_connection_id;
            picoquic_cnx_t* first_cnx = NULL;


            while (consumed_index < packet->length) {
                size_t consumed = 0;

                ret = picoquic_incoming_segment(cnx->quic, packet->bytes + consumed_index,
                    packet->length - consumed_index, packet->length,
                    &consumed, (struct sockaddr*) & packet->addr_to, (struct sockaddr*) & packet->addr_local, packet->if_index_local,
                    packet->received_ecn, current_time, packet->receive_time, &previous_destid, &first_cnx);

                if (ret == 0 && consumed > 0) {
                    consumed_index += consumed;
                }
                else {
                    break;
                }
            }

            if (ret != 0) {
                DBG_PRINTF("Processing sooner packet type %d returns %d (0x%d)", (int)packet->ptype, ret, ret);
            }

            if (previous == NULL) {
                cnx->first_sooner = packet->next_packet;
            }
            else {
                previous->next_packet = packet->next_packet;
            }
            picoquic_delete_stateless_packet(packet);
        }
        else {
            previous = packet;
        }

        packet = next_packet;
    }
}
```

### Rust body
```rust
    pub fn process_sooner_packets(&mut self, _current_time: Instant) {
        self.sooner_stateless.clear();
    }
```

## `picoquic/paths.c:picoquic_find_incoming_path`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust only scans existing paths for exact address/if_index matches, while the C body handles CID-based lookup, path creation, local CID assignment, tuple creation, NAT rebinding, CID migration, and timestamp update.
* C source: `picoquic/paths.c:614-747`
* C signature: `int picoquic_find_incoming_path(picoquic_cnx_t *, picoquic_packet_header *, struct sockaddr *, struct sockaddr *, int, uint64_t, int *)`
* Rust source: `rs/fq/src/internal.rs:4376-4396`
* Rust item: `find_incoming_path`

### C body
```c
{
    int ret = 0;
    picoquic_path_t* path_x = NULL;
    picoquic_tuple_t* tuple = NULL;
    int path_id = (ph->l_cid == NULL) ? 0 : picoquic_find_path_by_unique_id(cnx, ph->l_cid->path_id);

    if (path_id < 0) {
        /* Either this path has not yet been created, or it was already destroyed.
        * The packet decryption was successful, which means that the CID is valid,
        * but on the server side we might have a "probe".
         */
        if (cnx->nb_paths < PICOQUIC_NB_PATH_TARGET &&
            (cnx->quic->is_port_blocking_disabled || !picoquic_check_addr_blocked(addr_from)) &&
            picoquic_create_path(cnx, current_time, addr_to, addr_from, if_index_to, ph->l_cid->path_id) > 0) {
            /* if we do create a new path, it should have the right path_id. We cannot
            * assume that paths will be created in the full order, so that means we may
            * have to create "empty" paths in invalid state. Or, more simply,
            * create a path and override the unique path id, which should be OK
            * as that unique ID does not exist.
            * TODO: modify path creation to force path_id, return error if impossible.
             */
            path_id = cnx->nb_paths - 1;
            path_x = cnx->path[path_id];

            /* when creating the path, we need to copy the dest CID and chose
             * destination CID with the matching path ID.
             */
            path_x->first_tuple->p_local_cnxid = picoquic_find_local_cnxid(cnx, path_x->unique_path_id, &ph->dest_cnx_id);
            picoquic_assign_peer_cnxid_to_tuple(cnx, path_x, path_x->first_tuple);
        }
    }
    else
    {
        path_x = cnx->path[path_id];
        tuple = path_x->first_tuple;

        /* If the local CID is not set, set it */
        if (path_x->first_tuple->p_local_cnxid == NULL) {
            path_x->first_tuple->p_local_cnxid = picoquic_find_local_cnxid(cnx, path_x->unique_path_id, &ph->dest_cnx_id);
            if (!cnx->client_mode && cnx->is_multipath_enabled && path_x->first_tuple->challenge_verified) {
                /* If the peer renewed its connection id, the retire connection ID frame may already
                 * have arrived on a separate path. If the server noticed that, it should also renew
                 * its "remote path" ID */
                (void)picoquic_renew_connection_id(cnx, path_id);
            }
        }

        /* Treat the special case of the unkown local address, which should only happen
         * for clients and for the first tuple. */
        if (path_x->first_tuple->local_addr.ss_family == AF_UNSPEC && addr_to->sa_family != AF_UNSPEC) {
            picoquic_store_addr(&cnx->path[path_id]->first_tuple->local_addr, addr_to);
        }

        /* Look for the best match among existing tuples */
        while (tuple != NULL) {
            /* If the addresses match, we are good. */
            if (picoquic_compare_addr(addr_from, (struct sockaddr*)&tuple->peer_addr) == 0 &&
                picoquic_compare_addr(addr_to, (struct sockaddr*)&tuple->local_addr) == 0) {
                break;
            }
            else
            {
                tuple = tuple->next_tuple;
            }
        }
        if (tuple == NULL) {
            /* If the addresses do not match, we have two possibilities:
            * either the creation of a new tuple, or a NAT rebinding on an existing tuple.
            * In all cases, we need to create a new tuple. In the NAT rebinding cases, we
            * need to be a bit more agressive, i.e., immediately promote the new tuple
            * as the default. In fact, we MUST do that if the CID also changed, otherwise
            * we will stumble on a bug if the packet asks to retire the CID.
            *
            * We thus need to distinguish the NAT rebinding case from the non-multipath
            * path-migration. This is bound to be ambiguous, but we can use a simple heuristic:
            *
            * - if multipath is enabled, the old style path migration is supported but
            *   discouraged. It is mostly there to support "migration to a preferred
            *   address", and there is no much harm to always treat that as a NAT
            *   rebinding. Maybe make an exception if the destination address is
            *   one of the preferred addresses.
            * - if multipath is not enabled, check whether this looks like a challenge
            *   for a new address, i.e., it contains a PATH CHALLENGE frame and only
            *   non-path validating packets. If true, treat it as a path migration challenge.
            *   else, treat it as a NAT rebinding.
            */

            if (picoquic_check_cid_for_new_tuple(cnx, path_x->unique_path_id) == 0 &&
                (tuple = picoquic_create_tuple(path_x, addr_to, addr_from, if_index_to)) != NULL &&
                picoquic_assign_peer_cnxid_to_tuple(cnx, path_x, tuple) == 0) {
                picoquic_set_tuple_challenge(tuple, current_time, cnx->quic->use_constant_challenges);
                if (picoquic_compare_connection_id(&path_x->first_tuple->p_local_cnxid->cnx_id, &ph->dest_cnx_id) != 0 &&
                    cnx->is_multipath_enabled) {
                    /* Treat this as a NAT rebinding. */
                    picoquic_tuple_t* old_tuple = path_x->first_tuple;
                    /* We need to replace the first tuple by this tuple. */
                    picoquic_set_first_tuple(path_x, tuple);
                    tuple->challenge_verified = 1;
                    /* set a challenge on the old tuple to recover from spoofed addresses */
                    picoquic_set_tuple_challenge(old_tuple, current_time, cnx->quic->use_constant_challenges);
                    old_tuple->challenge_required = 1;
                    old_tuple->challenge_verified = 0;
                }
                else {
                    /* Treat this a new tuple challenge. */
                    tuple->challenge_required = 1;
                }
            }
            /* TODO: clean up in case of failure. */
        }
        else {
            /* If the addresses do match, but the CID do not, we have a case of CID migration.
             */
            if (tuple == path_x->first_tuple &&
                picoquic_compare_connection_id(&path_x->first_tuple->p_local_cnxid->cnx_id, &ph->dest_cnx_id) != 0) {
                path_x->first_tuple->p_local_cnxid = picoquic_find_local_cnxid(cnx, path_x->unique_path_id, &ph->dest_cnx_id);
                if (cnx->client_mode == 0) {
                    (void)picoquic_renew_connection_id(cnx, path_id);
                }
            }
        }
    }
    *p_path_id = path_id;
    cnx->path[path_id]->last_packet_received_at = current_time;

    return ret;
}
```

### Rust body
```rust
        for (path_id, path) in self.paths.iter_mut().enumerate() {
            if path.tuples.iter().any(|tuple| {
                tuple.peer_addr == *addr_from
                    && tuple.local_addr == *addr_to
                    && tuple.if_index == if_index_to as core::ffi::c_ulong
            }) {
                path.last_packet_received_at = current_time;
                return Ok(IncomingPathLookup {
                    path_id,
                    created: false,
                });
            }
        }
```

## `picoquic/picohash.c:picohash_delete`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C frees hash table bins/items/keys; Rust body only sets an app wake time field.
* C source: `picoquic/picohash.c:155-178`
* C signature: `void picohash_delete(picohash_table *, int)`
* Rust source: `rs/fq/src/lib.rs:2416-2423`
* Rust item: `delete`

### C body
```c
{
    if (hash_table->count > 0) {
        for (uint32_t i = 0; i < hash_table->nb_bin; i++) {
            picohash_item* item = hash_table->hash_bin[i];
            while (item != NULL) {
                picohash_item* tmp = item;
                const void* key_to_delete = tmp->key;

                item = item->next_in_bin;

                if (hash_table->picohash_key_to_item == NULL) {
                    free(tmp);
                }
                if (delete_key_too) {
                    free((void*)key_to_delete);
                }
            }
        }
    }

    free(hash_table->hash_bin);
    free(hash_table);
}
```

### Rust body
```rust
    pub fn set_app_wake_time(&mut self, app_wake_time: Instant) {
        self.app_wake_time = app_wake_time;
    }
```

## `picoquic/picosocks.c:picoquic_recvmsg`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C performs receive setup, calls WSARecvMsg, parses control data, and returns byte count or -1; Rust always returns a generic error.
* C source: `picoquic/picosocks.c:957-1020`
* C signature: `int picoquic_recvmsg(int, struct sockaddr_storage *, struct sockaddr_storage *, int *, unsigned char *, uint8_t *, int)`
* Rust source: `rs/fq/src/socks.rs:78-101`
* Rust item: `recv`

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

## `picoquic/picosplay.c:picosplay_delete`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C finds and deletes a splay node by value; Rust sets app_wake_time and does no delete.
* C source: `picoquic/picosplay.c:188-192`
* C signature: `void picosplay_delete(picosplay_tree_t *, void *)`
* Rust source: `rs/fq/src/lib.rs:2416-2423`
* Rust item: `delete`

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
