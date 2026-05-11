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

## `picoquic/quicctx.c:picoquic_wake_list_init`
* Phase 4C status: `suspect`
* Phase 4C rationale: C initializes the wake tree; Rust also clears each connection's wake membership, which is extra body-visible behavior.
* C source: `picoquic/quicctx.c:1499-1503`
* C signature: `void picoquic_wake_list_init(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:3368-3373`
* Rust item: `wake_list_init`

### C body
```c
{
    picosplay_init_tree(&quic->cnx_wake_tree, picoquic_wake_list_compare,
        picoquic_wake_list_create_node, picoquic_wake_list_delete_node, picoquic_wake_list_node_value);
}
```

### Rust body
```rust
    fn wake_list_init(&mut self) {
        self.connection_wake_tree = crate::splay::SplayTree::new();
        for cnx in self.connections.iter_mut() {
            cnx.connection_wake_membership = None;
        }
    }
```

## `picoquic/sender.c:picoquic_create_packet_header`
* Phase 4C status: `suspect`
* Phase 4C rationale: C directly constructs short/long headers from tuple/path fields; Rust delegates to create_packet_header_at after deriving a path index and passes 0 for one argument, so key visible header-selection logic is absent.
* C source: `picoquic/sender.c:711-790`
* C signature: `size_t picoquic_create_packet_header(picoquic_cnx_t *, picoquic_packet_type_enum, uint64_t, picoquic_path_t *, picoquic_tuple_t *, size_t, uint8_t *, size_t *, size_t *)`
* Rust source: `rs/fq/src/internal.rs:6897-6926`
* Rust item: `create_packet_header`

### C body
```c
{
    size_t length = 0;

    /* Prepare the packet header */
    if (packet_type == picoquic_packet_1rtt_protected) {
        /* Create a short packet -- using 32 bit sequence numbers for now */
        uint8_t K = (cnx->key_phase_enc) ? 0x04 : 0;
        uint8_t C = 0x40; /* set the QUIC bit */
        size_t pn_l = 4;  /* default packet length to 4 bytes */

        if (cnx->do_grease_quic_bit) {
            /* we grease the quic bit if both local and remote agreed to do so */
            C &= (uint8_t)picoquic_public_random_64();
            cnx->quic_bit_greased |= (C == 0);
        }

        length = 0;
        bytes[length++] = (K | C | picoquic_spin_function_table[cnx->spin_policy].spinbit_outgoing(cnx));
        length += picoquic_format_connection_id(&bytes[length], PICOQUIC_MAX_PACKET_SIZE - length, tuple->p_remote_cnxid->cnx_id);

        *pn_offset = length;
        if (header_length > length && header_length < length + 4) {
            pn_l = header_length - length;
        }
        *pn_length = pn_l;
        bytes[0] |= (pn_l - 1);
        switch (pn_l) {
        case 1:
            bytes[length] = (uint8_t)sequence_number;
            break;
        case 2:
            picoformat_16(&bytes[length], (uint16_t)sequence_number);
            break;
        case 3:
            picoformat_24(&bytes[length], (uint32_t)sequence_number);
            break;
        default:
            picoformat_32(&bytes[length], (uint32_t)sequence_number);
            break;
        }
        length += pn_l;
    }
    else {
        /* Create a long packet */
        picoquic_connection_id_t * dest_cnx_id =
            (cnx->client_mode && (packet_type == picoquic_packet_initial ||
                packet_type == picoquic_packet_0rtt_protected)
                && picoquic_is_connection_id_null(&path_x->first_tuple->p_remote_cnxid->cnx_id)) ?
            &cnx->initial_cnxid : &path_x->first_tuple->p_remote_cnxid->cnx_id;
        picoquic_connection_id_t* srce_cnx_id = &path_x->first_tuple->p_local_cnxid->cnx_id;
        uint32_t version = ((cnx->cnx_state == picoquic_state_client_init || cnx->cnx_state == picoquic_state_client_init_sent) && packet_type == picoquic_packet_initial) ?
            cnx->proposed_version : picoquic_supported_versions[cnx->version_index].version;

        length = picoquic_create_long_header(
            packet_type,
            dest_cnx_id,
            srce_cnx_id,
            cnx->do_grease_quic_bit,
            version,
            cnx->version_index,
            sequence_number,
            cnx->retry_token_length,
            cnx->retry_token,
            bytes,
            pn_offset,
            pn_length);
    }

    return length;
}
```

### Rust body
```rust
    ) -> usize {
        // Delegate to create_packet_header_at using tuple's path context.
        // Find tuple's path index by matching tuple's unique_path_id.
        let path_idx = self
            .paths
            .iter()
            .position(|p| p.unique_path_id == tuple.unique_path_id)
            .unwrap_or(0);
        let _ = tuple;
        self.create_packet_header_at(
            packet_type,
            sequence_number,
            path_idx,
            0,
            header_length,
            bytes,
            pn_offset,
            pn_length,
        )
    }
```

## `picoquic/sender.c:picoquic_preemptive_retransmit_in_context`
* Phase 4C status: `suspect`
* Phase 4C rationale: C implements the retransmit loop directly; Rust delegates to preemptive_retransmit_in_selection, so equivalence is not visible from the body alone.
* C source: `picoquic/sender.c:1421-1492`
* C signature: `int picoquic_preemptive_retransmit_in_context(picoquic_cnx_t *, picoquic_packet_context_t *, uint64_t, uint64_t, uint64_t *, uint8_t *, size_t, size_t *, int *, int *, int)`
* Rust source: `rs/fq/src/internal.rs:17518-17543`
* Rust item: `picoquic_preemptive_retransmit_in_context`

### C body
```c
{
    /* If there is a single packet context for application frames,
     * the code just has to track the preemptive_repeat_ptr for
     * that context. If there are multiple paths, we need to consider
     * packets from every plausible path.
     */
    int ret = 0;

    /* Check that the connection is still active before adding more preemptive repeats */
    if (cnx->latest_progress_time + rtt < current_time ||
        cnx->latest_receive_time + 2*rtt < current_time) {
        return 0;
    }

    /* Find the first packet that might be repeated */
    if (pkt_ctx->preemptive_repeat_ptr == NULL) {
        pkt_ctx->preemptive_repeat_ptr = pkt_ctx->pending_first;
    }
    /* Skip all packets that are too old to be repeated */
    while (pkt_ctx->preemptive_repeat_ptr != NULL) {
        if (pkt_ctx->preemptive_repeat_ptr->send_time + rtt / 2 >= current_time) {
            break;
        }
        pkt_ctx->preemptive_repeat_ptr = pkt_ctx->preemptive_repeat_ptr->packet_next;
    }
    /* Try to format the repeated packet */
    while (pkt_ctx->preemptive_repeat_ptr != NULL) {
        uint64_t early_delay = (rtt > 8 * PICOQUIC_ACK_DELAY_MAX) ? rtt / 8 : PICOQUIC_ACK_DELAY_MAX;
        uint64_t early_time = pkt_ctx->preemptive_repeat_ptr->send_time + early_delay;

        if (!pkt_ctx->preemptive_repeat_ptr->was_preemptively_repeated) {
            if (early_time > current_time) {
                /* Wait until the next repeat */
                if (*next_wake_time > early_time) {
                    *next_wake_time = early_time;
                    SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                }
                break;
            }
            if (test_only) {
                *more_data = 1;
                break;
            }
            ret = picoquic_preemptive_retransmit_packet(pkt_ctx->preemptive_repeat_ptr, cnx,
                new_bytes, send_buffer_max_minus_checksum, length, has_data);
            if (ret != 0) {
                break;
            }
        }
        pkt_ctx->preemptive_repeat_ptr = pkt_ctx->preemptive_repeat_ptr->packet_next;
        if (*has_data) {
            cnx->nb_preemptive_repeat++;
            if (pkt_ctx->preemptive_repeat_ptr != NULL) {
                *more_data = 1;
            }
            break;
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        self.preemptive_retransmit_in_selection(
            PacketContextSelection::Connection(pc),
            rtt,
            current_time,
            next_wake_time,
            new_bytes,
            send_buffer_max_minus_checksum,
            length,
            has_data,
            more_data,
            test_only,
        )
    }
```

## `picoquic/sender.c:picoquic_prepare_server_address_migration`
* Phase 4C status: `suspect`
* Phase 4C rationale: C gates all work on preferred_address.is_defined, while Rust proceeds whenever v4 or v6 is present; Rust also manually obtains/stamps a stashed connection ID and creates a tuple instead of the single probe_new_tuple call visible in C.
* C source: `picoquic/sender.c:1868-1930`
* C signature: `int picoquic_prepare_server_address_migration(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:7595-7683`
* Rust item: `picoquic_prepare_server_address_migration`

### C body
```c
{
    int ret = 0;
    uint64_t transport_error = 0;

    if (cnx->remote_parameters.preferred_address.is_defined) {
        uint64_t unique_path_id = (cnx->is_multipath_enabled) ? 1 : 0;
        int ipv4_received = cnx->remote_parameters.preferred_address.ipv4Port != 0;
        int ipv6_received = cnx->remote_parameters.preferred_address.ipv6Port != 0;

        /* Add the connection ID to the local stash */
        transport_error = picoquic_stash_remote_cnxid(cnx, 0, unique_path_id, 1,
            cnx->remote_parameters.preferred_address.connection_id.id_len,
            cnx->remote_parameters.preferred_address.connection_id.id,
            cnx->remote_parameters.preferred_address.statelessResetToken,
            NULL);
        if (transport_error != 0) {
            ret = picoquic_connection_error(cnx, transport_error, picoquic_frame_type_new_connection_id);
        }
        else if(ipv4_received || ipv6_received) {
            struct sockaddr_storage dest_addr;

            memset(&dest_addr, 0, sizeof(struct sockaddr_storage));

            /* program a migration. */
            if (ipv4_received && cnx->path[0]->first_tuple->peer_addr.ss_family == AF_INET) {
                /* select IPv4 */
                ipv6_received = 0;
            }

            if (ipv6_received) {
                /* configure an IPv6 sockaddr */
                struct sockaddr_in6 * d6 = (struct sockaddr_in6 *)&dest_addr;
                d6->sin6_family = AF_INET6;
                d6->sin6_port = htons(cnx->remote_parameters.preferred_address.ipv6Port);
                memcpy(&d6->sin6_addr, cnx->remote_parameters.preferred_address.ipv6Address, 16);
            }
            else {
                /* configure an IPv4 sockaddr */
                struct sockaddr_in * d4 = (struct sockaddr_in *)&dest_addr;
                d4->sin_family = AF_INET;
                d4->sin_port = htons(cnx->remote_parameters.preferred_address.ipv4Port);
                memcpy(&d4->sin_addr, cnx->remote_parameters.preferred_address.ipv4Address, 4);
            }

            /* Only send a probe if not already using that address
             * and the target address is not using a protected port number
             */
            if (picoquic_compare_addr((struct sockaddr *)&dest_addr, (struct sockaddr *)&cnx->path[0]->first_tuple->peer_addr) != 0 &&
                (cnx->quic->is_port_blocking_disabled || !picoquic_check_addr_blocked((struct sockaddr *)&dest_addr))) {
                struct sockaddr* local_addr = NULL;
                if (cnx->path[0]->first_tuple->local_addr.ss_family != 0 && cnx->path[0]->first_tuple->local_addr.ss_family == dest_addr.ss_family) {
                    local_addr = (struct sockaddr*) & cnx->path[0]->first_tuple->local_addr;
                }
                picoquic_probe_new_tuple(cnx, cnx->path[0], (struct sockaddr*)&dest_addr, local_addr, 0, picoquic_get_quic_time(cnx->quic),1);

            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn picoquic_prepare_server_address_migration(&mut self) -> i32 {
        let preferred = self.remote_parameters.preferred_address;
        let ipv4_received = preferred.v4.is_some();
        let ipv6_received = preferred.v6.is_some();
        if !ipv4_received && !ipv6_received {
            return 0;
        }

        let unique_path_id = if self.is_multipath_enabled { 1 } else { 0 };
        let transport_error = self
            .stash_remote_connection_id(
                0,
                unique_path_id,
                1,
                preferred.connection_id.as_bytes(),
                &preferred.stateless_reset_token,
            )
            .status;
        if transport_error != 0 {
            return self.connection_error(
                transport_error,
                crate::frames::FrameType::NewConnectionId as u64,
            );
        }

        let Some(first_tuple) = self.paths.first().and_then(|path| path.tuples.first()) else {
            return 0;
        };
        let use_v6 = ipv6_received && !(ipv4_received && first_tuple.peer_addr.is_ipv4());
        let dest_addr = if use_v6 { preferred.v6 } else { preferred.v4 };
        let Some(dest_addr) = dest_addr else {
            return 0;
        };

        if dest_addr == first_tuple.peer_addr
            || (!self
                .quic_ref()
                .map(|quic| quic.is_port_blocking_disabled)
                .unwrap_or(false)
                && crate::check_addr_blocked(&dest_addr))
        {
            return 0;
        }

        let local_addr = if !crate::socket_addr_is_unspecified(&first_tuple.local_addr)
            && first_tuple.local_addr.is_ipv4() == dest_addr.is_ipv4()
        {
            Some(first_tuple.local_addr)
        } else {
            None
        };
        let current_time = Instant::from_ticks(
            self.quic_ref()
                .map(|quic| quic.time())
                .unwrap_or_else(crate::current_time),
        );
        let use_constant_challenges = self
            .quic_ref()
            .map(|quic| i32::from(quic.use_constant_challenges))
            .unwrap_or(0);
        let path_unique_id = self
            .paths
            .first()
            .map(|path| path.unique_path_id)
            .unwrap_or(0);
        let Some((stash_idx, cid_idx)) = self.obtain_stashed_connection_id(path_unique_id) else {
            return 0;
        };
        if let Some(cid) = self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .get_mut(cid_idx)
        {
            cid.nb_path_references += 1;
        }
        if let Some(path) = self.paths.first_mut()
            && let Ok(tuple_idx) = path.create_tuple(local_addr.as_ref(), Some(&dest_addr), 0)
        {
            let path_unique_id = path.unique_path_id;
            if let Some(tuple) = path.tuples.get_mut(tuple_idx) {
                tuple.remote_connection_id_index = Some(cid_idx);
                tuple.unique_path_id = path_unique_id;
                set_tuple_challenge(tuple, current_time, use_constant_challenges);
                tuple.challenge_required = true;
                tuple.to_preferred_address = true;
            }
        }

        0
    }
```

## `picoquic/sim_link.c:picoquictest_sim_link_admit_pending`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only calls admit_pending when both aqm_state and its admit_pending function pointer are non-NULL; Rust calls admit_pending for any Some aqm_state with no visible per-callback null check.
* C source: `picoquic/sim_link.c:118-123`
* C signature: `void picoquictest_sim_link_admit_pending(picoquictest_sim_link_t *, uint64_t)`
* Rust source: `rs/fq/src/tests/util.rs:395-399`
* Rust item: `admit_pending`

### C body
```c
{
    if (link->aqm_state != NULL && link->aqm_state->admit_pending != NULL) {
        link->aqm_state->admit_pending(link->aqm_state, link, current_time);
    }
}
```

### Rust body
```rust
        if let Some(mut aqm) = self.aqm_state.take() {
            aqm.admit_pending(self, current_time);
            self.aqm_state = Some(aqm);
        }
```
