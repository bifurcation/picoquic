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

## `picoquic/packet.c:picoquic_queue_immediate_close`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only creates a stateless packet and returns on failure; it does not prepare, queue, or delete the packet as the C body does.
* C source: `picoquic/packet.c:1317-1334`
* C signature: `void picoquic_queue_immediate_close(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:7360-7363`
* Rust item: `queue_immediate_close`

### C body
```c
{
    picoquic_stateless_packet_t* sp = picoquic_create_stateless_packet(cnx->quic);

    if (sp != NULL) {
        int ret = picoquic_prepare_packet_ex(cnx, current_time, sp->bytes, PICOQUIC_MAX_PACKET_SIZE,
            &sp->length, &sp->addr_to, &sp->addr_local, &sp->if_index_local, NULL);
        if (ret == 0 && sp->length > 0) {
            picoquic_queue_stateless_packet(cnx->quic, sp);
        }
        else {
            picoquic_delete_stateless_packet(sp);
        }
    }
}
```

### Rust body
```rust
        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };
```

## `picoquic/paths.c:picoquic_select_next_path_tuple`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C handles control messages, abandon path, retransmit probes, single-path fallback, availability verification, and sorting; Rust only scans paced non-demoted paths and returns the first tuple whose challenge has not failed.
* C source: `picoquic/paths.c:488-556`
* C signature: `void picoquic_select_next_path_tuple(picoquic_cnx_t *, uint64_t, uint64_t *, picoquic_path_t **, picoquic_tuple_t **)`
* Rust source: `rs/fq/src/internal.rs:4627-4650`
* Rust item: `select_next_path_tuple`

### C body
```c
{
    int nb_available = 0;
    uint64_t min_retransmit = 0;

    *next_path = NULL;
    *next_tuple = NULL;

    /* First check whether path contol messages are needed */
    for (int path_index = 0; path_index < cnx->nb_paths; path_index++)
    {
        if (cnx->path[path_index]->path_is_demoted) {
            continue;
        }
        else if (cnx->is_multipath_enabled && cnx->path[path_index]->first_tuple->challenge_failed && !cnx->path[path_index]->path_abandon_sent) {
            (void)picoquic_abandon_path(cnx, cnx->path[path_index]->unique_path_id, PICOQUIC_TRANSPORT_UNSTABLE_INTERFACE, current_time);
        }
        else if ((*next_tuple = picoquic_check_path_control_needed(cnx, cnx->path[path_index], current_time, next_wake_time)) != NULL) {
            *next_path = cnx->path[path_index];
            (*next_path)->challenger++;
            break;
        }
        else if (cnx->nb_paths > 0 && cnx->path[path_index]->first_tuple->challenge_verified && cnx->path[path_index]->nb_retransmit > 0 &&
            cnx->cnx_state == picoquic_state_ready && cnx->path[path_index]->bytes_in_transit == 0) {
            cnx->path[path_index]->is_multipath_probe_needed = 1;
            *next_path = cnx->path[path_index];
            *next_tuple = (*next_path)->first_tuple;
            (*next_path)->challenger++;
            break;
        }
    }
    if (*next_path != NULL) {
        /* we are done */
    }
    else  if (cnx->nb_paths == 1) {
        /* No choice, just use this path -- this is the default if multipath is not selected. */
        *next_path = cnx->path[0];
        *next_tuple = (*next_path)->first_tuple;
    }
    else if ((nb_available = picoquic_verify_path_available(cnx, next_path, &min_retransmit, current_time)) < 2) {
        /* Only 0 or 1 path to chose from. Just select that. */
        if (*next_path == NULL) {
            *next_path = cnx->path[0];
        }
        *next_tuple = (*next_path)->first_tuple;
    }
    else {
        /* Several paths are available. We will chose from that, looking at
        * available path that can send ACK, or paced data, or congestion
        * controlled data.
         */
        picoquic_sort_available_paths(cnx, current_time, next_wake_time, next_path, min_retransmit, next_tuple);
    }
}
```

### Rust body
```rust
    ) -> Option<(PathToken, usize)> {
        for (path_idx, path) in self.paths.iter_mut().enumerate() {
            if path.path_is_demoted || path.path_abandon_received {
                continue;
            }
            if !path
                .pacing
                .is_authorized(current_time, next_wake_time, false, None)
            {
                continue;
            }
            if let Some(tuple_idx) = path.tuples.iter().position(|tuple| !tuple.challenge_failed) {
                return Some((
                    PathToken::synthetic(path_idx as u32, path_idx as u32),
                    tuple_idx,
                ));
            }
        }
        None
    }
```

## `picoquic/picohash.c:picohash_siphash`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body calls siphash and assembles an 8-byte little-endian result; Rust body shown only defines sip_round and contains no top-level hashing or return value.
* C source: `picoquic/picohash.c:204-219`
* C signature: `uint64_t picohash_siphash(const uint8_t *, size_t, const uint8_t *)`
* Rust source: `rs/fq/src/siphash.rs:22-38`
* Rust item: `siphash`

### C body
```c
{
    uint8_t sip_out[8];
    uint64_t hash;
    (void)siphash(bytes, length, hash_seed, sip_out, 8);
    hash =
        (uint64_t)sip_out[0] +
        (((uint64_t)sip_out[1]) << 8) +
        (((uint64_t)sip_out[2]) << 16) +
        (((uint64_t)sip_out[3]) << 24) +
        (((uint64_t)sip_out[4]) << 32) +
        (((uint64_t)sip_out[5]) << 40) +
        (((uint64_t)sip_out[6]) << 48) +
        (((uint64_t)sip_out[7]) << 56);
    return hash;
}
```

### Rust body
```rust
    fn sip_round(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
        *v0 = v0.wrapping_add(*v1);
        *v1 = v1.rotate_left(13);
        *v1 ^= *v0;
        *v0 = v0.rotate_left(32);
        *v2 = v2.wrapping_add(*v3);
        *v3 = v3.rotate_left(16);
        *v3 ^= *v2;
        *v0 = v0.wrapping_add(*v3);
        *v3 = v3.rotate_left(21);
        *v3 ^= *v0;
        *v2 = v2.wrapping_add(*v1);
        *v1 = v1.rotate_left(17);
        *v1 ^= *v2;
        *v2 = v2.rotate_left(32);
    }
```

## `picoquic/picosocks.c:picoquic_select_ex`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C waits with select using delta_t, returns 0 on timeout, updates current_time, and receives only from ready sockets; Rust polls sockets directly once and returns Err if none receive data.
* C source: `picoquic/picosocks.c:1167-1246`
* C signature: `int picoquic_select_ex(int *, int, struct sockaddr_storage *, struct sockaddr_storage *, int *, unsigned char *, uint8_t *, int, int64_t, int *, uint64_t *)`
* Rust source: `rs/fq/src/socks.rs:334-365`
* Rust item: `select`

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

## `picoquic/picosplay.c:picosplay_last`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the rightmost node from the tree root; Rust body is a previous-token traversal rather than a tree-last operation.
* C source: `picoquic/picosplay.c:258-260`
* C signature: `picosplay_node_t * picosplay_last(picosplay_tree_t *)`
* Rust source: `rs/fq/src/splay.rs:514-541`
* Rust item: `last`

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
