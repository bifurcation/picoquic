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

## `picoquic/quicctx.c:picoquic_init_cnxid_stash`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust creates an initial stash entry and reference count like C, but C fills reset_secret with random bytes while Rust sets reset_secret to all zeros.
* C source: `picoquic/quicctx.c:2917-2942`
* C signature: `int picoquic_init_cnxid_stash(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:4965-5009`
* Rust item: `init_connection_id_stash`

### C body
```c
{
    int ret = 0;
    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = picoquic_find_or_create_remote_cnxid_stash(cnx, 0, 1);
    if (remote_cnxid_stash == NULL || remote_cnxid_stash->cnxid_stash_first != NULL) {
        ret = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
    }
    else {
        remote_cnxid_stash->cnxid_stash_first = (picoquic_remote_cnxid_t*)malloc(sizeof(picoquic_remote_cnxid_t));
        cnx->path[0]->first_tuple->p_remote_cnxid = remote_cnxid_stash->cnxid_stash_first;
        if (remote_cnxid_stash->cnxid_stash_first == NULL) {
            ret = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
        }
        else {
            memset(remote_cnxid_stash->cnxid_stash_first, 0, sizeof(picoquic_remote_cnxid_t));
            remote_cnxid_stash->cnxid_stash_first->nb_path_references++;

            /* Initialize the reset secret to a random value. This
            * will prevent spurious matches to an all zero value, for example.
            * The real value will be set when receiving the transport parameters.
            */
            picoquic_public_random(remote_cnxid_stash->cnxid_stash_first->reset_secret, PICOQUIC_RESET_SECRET_SIZE);
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn init_connection_id_stash(&mut self) -> Result<(), crate::Error> {
        // C: picoquic_init_cnxid_stash
        // Ensure the path-0 stash exists and has an initial (empty) entry that
        // `paths[0].tuples[0]` can reference.
        let stash_idx = self
            .find_or_create_remote_connection_id_stash(0, true)
            .ok_or(crate::Error::Memory)?;
        // If there's already a CID in the stash, leave it.
        if !self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .is_empty()
        {
            return Err(crate::Error::Generic); // transport internal error
        }
        let zero_pkt_ctx = PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: 0u64.wrapping_sub(1),
            latest_time_acknowledged: self.start_time,
            highest_acknowledged_time: self.start_time,
            pending: std::collections::BTreeMap::new(),
            retransmitted: std::collections::BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        };
        let initial_rcid = RemoteConnectionId {
            sequence: 0,
            connection_id: crate::ConnectionId::default(),
            reset_secret: [0u8; RESET_SECRET_SIZE],
            nb_path_references: 1,
            needs_removal: false,
            retire_sent: false,
            retire_acked: false,
            pkt_ctx: zero_pkt_ctx,
        };
        self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .push(initial_rcid);
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_register_net_id`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both unregister, use first tuple peer address, check for existing registration, and insert, but Rust returns Ok for unspecified peer address while C proceeds with retrieval/insertion.
* C source: `picoquic/quicctx.c:1292-1310`
* C signature: `int picoquic_register_net_id(picoquic_quic_t *, picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/lib.rs:2037-2065`
* Rust item: `register_net_id`

### C body
```c
{
    int ret = 0;
    picohash_item* item;

    /* If registration was present, remove it */
    picoquic_unregister_net_id(cnx, path_x);
    /* Try registering the new address */
    picoquic_store_addr(&path_x->registered_peer_addr, (struct sockaddr *)&path_x->first_tuple->peer_addr);
    item = picohash_retrieve(quic->table_cnx_by_net, path_x);

    if (item != NULL) {
        ret = -1;
    } else {
        ret = picohash_insert(quic->table_cnx_by_net, path_x);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        self.unregister_net_id(connection, path_index);

        let peer_addr = self
            .connections
            .get(connection)
            .and_then(|cnx| cnx.paths.get(path_index))
            .and_then(|path| path.tuples.first())
            .map(|tuple| tuple.peer_addr)
            .ok_or(Error::InvalidArgument)?;
        if socket_addr_is_unspecified(&peer_addr) {
            return Ok(());
        }
        if self.connection_by_net.lookup(&peer_addr).is_some() {
            return Err(Error::Generic);
        }
        let (membership, _) = self.connection_by_net.insert(peer_addr, connection)?;
        if let Some(cnx) = self.connections.get_mut(connection)
            && let Some(path) = cnx.paths.get_mut(path_index)
        {
            path.registered_peer_addr = peer_addr;
            path.connection_by_net_membership = Some(membership);
        }
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_remove_not_before_cid`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both find a matching stash and remove entries not before the threshold, but Rust rewrites unique_path_id to 0 when multipath is disabled, which is not visible in the C body.
* C source: `picoquic/quicctx.c:3236-3246`
* C signature: `uint64_t picoquic_remove_not_before_cid(picoquic_cnx_t *, uint64_t, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:5322-5355`
* Rust item: `remove_not_before_cid`

### C body
```c
{
    uint64_t transport_error = 0;
    picoquic_remote_cnxid_stash_t* cnxid_stash = picoquic_find_or_create_remote_cnxid_stash(cnx, unique_path_id, 0);

    if (cnxid_stash != NULL) {
        transport_error = picoquic_remove_not_before_from_stash(cnx, cnxid_stash, not_before, current_time);
    }

    return transport_error;
}
```

### Rust body
```rust
    ) -> u64 {
        // C: picoquic_remove_not_before_cid
        let eff_path_id = if self.is_multipath_enabled {
            unique_path_id
        } else {
            0
        };
        if let Some(stash_idx) = self
            .remote_connection_id_stashes
            .iter()
            .position(|s| s.unique_path_id == eff_path_id)
        {
            // Temporarily extract the stash to avoid borrow conflicts.
            let mut stash = std::mem::replace(
                &mut self.remote_connection_id_stashes[stash_idx],
                RemoteConnectionIdStash {
                    unique_path_id: eff_path_id,
                    retire_connection_id_before: 0,
                    connection_ids: Vec::new(),
                    is_in_use: false,
                },
            );
            let removed = self.remove_not_before_from_stash(&mut stash, not_before, current_time);
            self.remote_connection_id_stashes[stash_idx] = stash;
            removed
        } else {
            0
        }
    }
```

## `picoquic/quicctx.c:picoquic_set_default_multipath_option`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets the multipath option and conditionally sets initial_max_path_id to 2; Rust only stores the multipath option.
* C source: `picoquic/quicctx.c:942-950`
* C signature: `void picoquic_set_default_multipath_option(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1695-1697`
* Rust item: `set_default_multipath_option`

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

## `picoquic/quicctx.c:picoquic_set_tp_value_by_type`
* Phase 4C status: `suspect`
* Phase 4C rationale: C assigns grease_quic_bit and address_discovery cases into max_idle_timeout, while Rust assigns do_grease_quic_bit and address_discovery_mode.
* C source: `picoquic/quicctx.c:819-891`
* C signature: `int picoquic_set_tp_value_by_type(picoquic_tp_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:1267-1297`
* Rust item: `set_tp_value_by_type`

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
