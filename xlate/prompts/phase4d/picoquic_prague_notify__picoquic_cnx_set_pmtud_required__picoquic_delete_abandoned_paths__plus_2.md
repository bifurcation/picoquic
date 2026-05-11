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

## `picoquic/prague.c:picoquic_prague_notify`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C handles many notification cases and updates pacing; Rust only takes congestion_alg_state and returns if absent.
* C source: `picoquic/prague.c:320-394`
* C signature: `void picoquic_prague_notify(picoquic_cnx_t *, picoquic_path_t *, picoquic_congestion_notification_t, picoquic_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/prague.rs:296-305`
* Rust item: `picoquic_prague_notify`

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

## `picoquic/quicctx.c:picoquic_cnx_set_pmtud_required`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets PMTUD policy from the input flag; Rust returns whether a PSK cipher suite is set, an unrelated operation.
* C source: `picoquic/quicctx.c:4563-4567`
* C signature: `void picoquic_cnx_set_pmtud_required(picoquic_cnx_t *, int)`
* Rust source: `rs/fq/src/lib.rs:2881-2893`
* Rust item: `set_pmtud_required`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->pmtud_policy = (is_pmtud_required) ? picoquic_pmtud_required : picoquic_pmtud_basic;
}
```

### Rust body
```rust
    pub fn tls_is_psk_handshake(&self) -> bool {
        self.psk_cipher_suite_id != 0
    }
```

## `picoquic/quicctx.c:picoquic_delete_abandoned_paths`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C demotes paths, schedules demotion wake time, compacts/deletes paths, updates path_demotion_needed, and handles backup promotion; Rust only deletes demoted tuples and retains paths by abandon flag plus empty tuples.
* C source: `picoquic/quicctx.c:1967-2050`
* C signature: `void picoquic_delete_abandoned_paths(picoquic_cnx_t *, uint64_t, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:4756-4760`
* Rust item: `delete_abandoned_paths`

### C body
```c
{
    int path_index_good = 1;
    int path_index_current = 1;
    unsigned int is_demotion_in_progress = 0;

    if (cnx->is_multipath_enabled && cnx->nb_paths > 1) {
        path_index_good = 0;
        path_index_current = 0;
    }

    while (path_index_current < cnx->nb_paths) {
        /* Demote the path if marked for demotion */
        if (!cnx->path[path_index_current]->path_is_demoted){
            if (cnx->path[path_index_current]->first_tuple->challenge_failed ||
                (path_index_current > 0 && cnx->path[path_index_current]->first_tuple->challenge_verified &&
                    current_time - cnx->path[path_index_current]->latest_sent_time >= cnx->idle_timeout)) {
                picoquic_demote_path(cnx, path_index_current, current_time, 0);
            }
        }
        if (cnx->path[path_index_current]->path_is_demoted &&
            current_time >= cnx->path[path_index_current]->demotion_time) {
            /* Waited enough,should now delete this path. */
            path_index_current++;
            is_demotion_in_progress |= 1;
        } else {
            /* Need to keep this path a bit longer */
            /* First set the wake up timer so we don't miss the coming demotion */
            if (cnx->path[path_index_current]->path_is_demoted &&
                current_time < cnx->path[path_index_current]->demotion_time){
                is_demotion_in_progress |= 1;
                if (*next_wake_time > cnx->path[path_index_current]->demotion_time) {
                    *next_wake_time = cnx->path[path_index_current]->demotion_time;
                    SET_LAST_WAKE(cnx->quic, PICOQUIC_QUICCTX);
                }
            }
            /* Then pack the list of paths */
            if (path_index_current > path_index_good) {
                /* swap the path indexed good with current */
                picoquic_path_t * path_x = cnx->path[path_index_current];
                cnx->path[path_index_current] = cnx->path[path_index_good];
                cnx->path[path_index_good] = path_x;
            }
            /* increment both indices */
            path_index_current++;
            path_index_good++;
        }
    }

    if (cnx->nb_paths > path_index_good) {
        do {
            int d_path = cnx->nb_paths - 1;
            picoquic_dereference_stashed_cnxid(cnx, cnx->path[d_path], 0);
            picoquic_delete_path(cnx, d_path);
        }  while (cnx->nb_paths > path_index_good);
        /* If paths have been deleted, it may become possible to create new ones. */
        picoquic_test_and_signal_new_path_allowed(cnx);
    }

    /* TODO: what if there are no paths left? */
    cnx->path_demotion_needed = is_demotion_in_progress;
    int path_left = -1;
    int path_backup = -1;
    if (is_demotion_in_progress && cnx->is_multipath_enabled) {
        /* Verify that if one path is demoted, the other
         * becomes available */
        for (int i = 0; i < cnx->nb_paths; i++) {
            if (cnx->path[i]->path_is_demoted) {
                continue;
            }
            if (cnx->path[i]->path_is_backup && path_backup < 0) {
                path_backup = i;
            }
            else {
                path_left = i;
                break;
            }
        }
        if (path_left < 0 && path_backup >= 0) {
            cnx->path[path_backup]->path_is_backup = 0;
            (void)picoquic_queue_path_available_or_backup_frame(cnx, cnx->path[path_backup], picoquic_path_status_available);
        }
    }
}
```

### Rust body
```rust
    pub fn delete_abandoned_paths(&mut self, current_time: Instant, next_wake_time: &mut Instant) {
        self.delete_demoted_tuples(current_time, next_wake_time);
        self.paths
            .retain(|path| !(path.path_abandon_received && path.tuples.is_empty()));
    }
```

## `picoquic/quicctx.c:picoquic_dequeue_stateless_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C removes and returns the pending stateless packet; Rust parses bytes, builds a version negotiation packet, and queues it.
* C source: `picoquic/quicctx.c:1243-1255`
* C signature: `picoquic_stateless_packet_t * picoquic_dequeue_stateless_packet(picoquic_quic_t *)`
* Rust source: `rs/fq/src/internal.rs:634-763`
* Rust item: `dequeue_stateless_packet`

### C body
```c
{
    picoquic_stateless_packet_t* sp = quic->pending_stateless_packet;

    if (sp != NULL) {
        quic->pending_stateless_packet = sp->next_packet;
        sp->next_packet = NULL;
        picoquic_log_quic_pdu(quic, 0, picoquic_get_quic_time(quic), sp->cnxid_log64,
            (struct sockaddr*) & sp->addr_to, (struct sockaddr*) & sp->addr_local, sp->length);
    }

    return sp;
}
```

### Rust body
```rust
    ) {
        if original_bytes.len() < 7 {
            return;
        }

        let dcid_length = original_bytes[5] as usize;
        let dcid_start = 6usize;
        let dcid_end = dcid_start.saturating_add(dcid_length);
        if dcid_end >= original_bytes.len() {
            return;
        }
        let scid_length = original_bytes[dcid_end] as usize;
        let scid_start = dcid_end + 1;
        let scid_end = scid_start.saturating_add(scid_length);
        if scid_end > original_bytes.len() {
            return;
        }

        let dcid = &original_bytes[dcid_start..dcid_end];
        let scid = &original_bytes[scid_start..scid_end];

        let mut cnx = None;
        if dcid_length <= crate::CONNECTION_ID_MAX_SIZE {
            ph.dest_connection_id = ConnectionId::clone_from_slice(dcid).unwrap_or_default();
            if ph.dest_connection_id.len() == self.local_connection_id_length as usize {
                if self.local_connection_id_length == 0 {
                    cnx = self.connection_by_net(Some(addr_from));
                } else if let Some((tok, local_cid)) = self.connection_by_id(ph.dest_connection_id)
                {
                    ph.local_connection_id = Some(local_cid);
                    cnx = Some(tok);
                }
            }
            if cnx.is_none() {
                cnx = self.connection_by_icid(&ph.dest_connection_id, Some(addr_from));
            }
        }

        if cnx.is_some() {
            return;
        }

        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };

        let mut byte_index = 0usize;
        if byte_index + 1 > sp.bytes.len() {
            return;
        }
        public_random(&mut sp.bytes[byte_index..byte_index + 1]);
        sp.bytes[byte_index] |= 0x80;
        byte_index += 1;

        if byte_index + 4 > sp.bytes.len() {
            return;
        }
        format_32(&mut sp.bytes[byte_index..byte_index + 4], 0);
        byte_index += 4;

        if scid_length > u8::MAX as usize
            || dcid_length > u8::MAX as usize
            || byte_index + 1 + scid_length + 1 + dcid_length > sp.bytes.len()
        {
            return;
        }
        sp.bytes[byte_index] = scid_length as u8;
        byte_index += 1;
        sp.bytes[byte_index..byte_index + scid_length].copy_from_slice(scid);
        byte_index += scid_length;
        sp.bytes[byte_index] = dcid_length as u8;
        byte_index += 1;
        sp.bytes[byte_index..byte_index + dcid_length].copy_from_slice(dcid);
        byte_index += dcid_length;

        for version in SUPPORTED_VERSIONS {
            if byte_index + 4 > sp.bytes.len() {
                return;
            }
            format_32(&mut sp.bytes[byte_index..byte_index + 4], version as u32);
            byte_index += 4;
        }

        let rand_vn = loop {
            let candidate = ((crate::public_random_64() as u32) & 0xF0F0_F0F0) | 0x0A0A_0A0A;
            if candidate != ph.version {
                break candidate;
            }
        };
        if byte_index + 4 > sp.bytes.len() {
            return;
        }
        format_32(&mut sp.bytes[byte_index..byte_index + 4], rand_vn);
        byte_index += 4;

        sp.length = byte_index;
        sp.addr_to = *addr_from;
        sp.addr_local = *addr_to;
        sp.if_index_local = if_index_to;
        sp.initial_connection_id = ph.dest_connection_id;
        sp.connection_id_log64 = sp.initial_connection_id.val64();
        sp.packet_type = PacketType::VersionNegotiation;

        self.log_pdu(
            true,
            Instant::from_ticks(self.time()),
            0,
            addr_to,
            addr_from,
            sp.length,
        );

        self.queue_stateless_packet(sp);
    }
```

## `picoquic/quicctx.c:picoquic_find_or_create_local_cnxid_list`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C finds or optionally creates a local CID list; Rust body shown implements helpers and create_local_connection_id, creating an ID rather than just finding/creating the list.
* C source: `picoquic/quicctx.c:3766-3793`
* C signature: `picoquic_local_cnxid_list_t * picoquic_find_or_create_local_cnxid_list(picoquic_cnx_t *, uint64_t, int)`
* Rust source: `rs/fq/src/internal.rs:13044-13154`
* Rust item: `find_or_create_local_connection_id_list`

### C body
```c
{
    picoquic_local_cnxid_list_t* local_cnxid_list = cnx->first_local_cnxid_list;
    picoquic_local_cnxid_list_t** p_previous = &cnx->first_local_cnxid_list;

    while (local_cnxid_list != NULL) {
        if (local_cnxid_list->unique_path_id == unique_path_id) {
            break;
        }
        p_previous = &local_cnxid_list->next_list;
        local_cnxid_list = local_cnxid_list->next_list;
    }

    if (local_cnxid_list == NULL && do_create) {
        local_cnxid_list = (picoquic_local_cnxid_list_t*)malloc(sizeof(picoquic_local_cnxid_list_t));
        if (local_cnxid_list != NULL) {
            memset(local_cnxid_list, 0, sizeof(picoquic_local_cnxid_list_t));
            local_cnxid_list->unique_path_id = unique_path_id;
            *p_previous = local_cnxid_list;
            cnx->nb_local_cnxid_lists++;
            if (unique_path_id >= cnx->next_path_id_in_lists) {
                cnx->next_path_id_in_lists = unique_path_id + 1;
            }
        }
    }

    return local_cnxid_list;
}
```

### Rust body
```rust
impl Connection {
    fn has_local_connection_id_value(&self, connection_id: &ConnectionId) -> bool {
        self.local_connection_id_lists.iter().any(|list| {
            list.connection_ids.iter().any(|&tok| {
                self.local_connection_ids
                    .get(tok)
                    .map(|l| &l.connection_id == connection_id)
                    .unwrap_or(false)
            })
        })
    }

    fn random_local_connection_id(&self) -> crate::Result<ConnectionId> {
        let copy_len = self.local_cid_length as usize;
        let mut bytes = [0u8; crate::CONNECTION_ID_MAX_SIZE];
        fill_system_random(&mut bytes[..copy_len])?;
        ConnectionId::clone_from_slice(&bytes[..copy_len]).ok_or(crate::Error::Generic)
    }

    pub fn create_local_connection_id(
        &mut self,
        unique_path_id: u64,
        suggested_value: Option<&ConnectionId>,
        current_time: Instant,
    ) -> Result<LocalConnectionIdToken, crate::Error> {
        // Find or create the per-path list.
        let list_idx = match self
            .local_connection_id_lists
            .iter()
            .position(|l| l.unique_path_id == unique_path_id)
        {
            Some(i) => i,
            None => {
                self.local_connection_id_lists.push(LocalConnectionIdList {
                    unique_path_id,
                    local_connection_id_sequence_next: 0,
                    local_connection_id_retire_before: 0,
                    local_connection_id_oldest_created: current_time.ticks(),
                    nb_local_connection_id_expired: 0,
                    is_demoted: false,
                    demotion_time: crate::Instant::from_ticks(u64::MAX),
                    connection_ids: Vec::new(),
                });
                self.local_connection_id_lists.len() - 1
            }
        };

        let connection_id = if self.local_cid_length == 0 {
            ConnectionId::default()
        } else {
            let mut selected = None;
            for attempt in 0..32 {
                let candidate = if attempt == 0 {
                    match suggested_value {
                        Some(suggested) => *suggested,
                        None => self.random_local_connection_id()?,
                    }
                } else {
                    self.random_local_connection_id()?
                };
                if !self.has_local_connection_id_value(&candidate) {
                    selected = Some(candidate);
                    break;
                }
            }
            selected.ok_or(crate::Error::Generic)?
        };

        let seq = self.local_connection_id_lists[list_idx].local_connection_id_sequence_next;

        let l_cid = LocalConnectionId {
            connection_by_id_membership: None,
            path_id: unique_path_id,
            sequence: seq,
            create_time: current_time,
            connection_id,
            is_acked: false,
        };

        let token = self
            .local_connection_ids
            .insert(l_cid)
            .map_err(|_| crate::Error::Memory)?;

        self.local_connection_id_lists[list_idx].local_connection_id_sequence_next += 1;
        self.local_connection_id_lists[list_idx]
            .connection_ids
            .push(token);

        if seq == 0 {
            self.local_connection_id_lists[list_idx].local_connection_id_oldest_created =
                current_time.ticks();
            if unique_path_id > self.max_path_id_in_connection_id_lists {
                self.max_path_id_in_connection_id_lists = unique_path_id;
            }
        }

        Ok(token)
    }
}
```
