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

## `picoquic/picosocks.c:cmsg_format_header_return_data_ptr`
* Phase 4C status: `suspect`
* Phase 4C rationale: For subsequent headers C conditionally advances by CMSG_ALIGN(last_cmsg->cmsg_len), while Rust always uses CMSG_NXTHDR.
* C source: `picoquic/picosocks.c:519-543`
* C signature: `void * cmsg_format_header_return_data_ptr(struct msghdr *, struct cmsghdr **, int *, int, int, size_t)`
* Rust source: `rs/fq/src/socks.rs:579-622`
* Rust item: `cmsg_format_header_return_data_ptr`

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

## `picoquic/picosplay.c:picosplay_insert`
* Phase 4C status: `suspect`
* Phase 4C rationale: C always inserts a new node, putting equal keys on the right; Rust first finds an existing key and replaces its value instead of inserting a duplicate.
* C source: `picoquic/picosplay.c:99-138`
* C signature: `picosplay_node_t * picosplay_insert(picosplay_tree_t *, void *)`
* Rust source: `rs/fq/src/splay.rs:415-456`
* Rust item: `insert`

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

## `picoquic/quicctx.c:picoquic_compare_stream_priority`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust body orders streams by priority then stream_id, matching the comparison logic, but it is an enqueue_output_stream_token helper rather than a comparison function returning -1/0/1.
* C source: `picoquic/quicctx.c:3486-3500`
* C signature: `int picoquic_compare_stream_priority(picoquic_stream_head_t *, picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/lib.rs:3791-3827`
* Rust item: `compare_stream_priority`

### C body
```c
int picoquic_compare_stream_priority(picoquic_stream_head_t * stream, picoquic_stream_head_t * other) {
    int ret = 1;
    if (stream->stream_priority < other->stream_priority) {
        ret = -1;
    }
    else if (stream->stream_priority == other->stream_priority) {
        if (stream->stream_id < other->stream_id) {
            ret = -1;
        }
        else if (stream->stream_id == other->stream_id) {
            ret = 0;
        }
    }
    return ret;
}
```

### Rust body
```rust
fn enqueue_output_stream_token(connection: &mut Connection, token: internal::StreamToken) {
    if connection
        .output_streams
        .iter()
        .any(|&existing| existing == token)
    {
        return;
    }

    let pos = connection
        .output_streams
        .iter()
        .position(|&existing| {
            let Some(left) = connection.streams.get(token) else {
                return false;
            };
            let Some(right) = connection.streams.get(existing) else {
                return true;
            };
            left.stream_priority
                .cmp(&right.stream_priority)
                .then_with(|| left.stream_id.cmp(&right.stream_id))
                == core::cmp::Ordering::Less
        })
        .unwrap_or(connection.output_streams.len());
    connection.output_streams.insert(pos, token);
}
```

## `picoquic/quicctx.c:picoquic_create_path`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust initializes a Path and creates a tuple, but body-visible differences include fixed send_mtu initialization, rtt_min set to INITIAL_RTT instead of zero, no path challenge setup, and no insertion into a connection path array.
* C source: `picoquic/quicctx.c:1786-1866`
* C signature: `int picoquic_create_path(picoquic_cnx_t *, uint64_t, const struct sockaddr *, const struct sockaddr *, int, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:4145-4332`
* Rust item: `new`

### C body
```c
{
    int ret = -1;

    if (cnx->nb_paths >= cnx->nb_path_alloc)
    {
        int new_alloc = (cnx->nb_path_alloc == 0) ? 1 : 2 * cnx->nb_path_alloc;
        picoquic_path_t ** new_path = (picoquic_path_t **)malloc(new_alloc * sizeof(picoquic_path_t *));

        if (new_path != NULL)
        {
            if (cnx->path != NULL)
            {
                memset (new_path, 0, new_alloc * sizeof(picoquic_path_t*));
                if (cnx->nb_paths > 0)
                {
                    memcpy(new_path, cnx->path, cnx->nb_paths * sizeof(picoquic_path_t *));
                }
                free(cnx->path);
            }
            cnx->path = new_path;
            cnx->nb_path_alloc = new_alloc;
        }
    }

    if (cnx->nb_paths < cnx->nb_path_alloc)
    {
        uint64_t unique_path_id = picoquic_find_avalaible_unique_path_id(cnx, requested_id);
        picoquic_path_t* path_x = (unique_path_id == UINT64_MAX) ? NULL :
            (picoquic_path_t*)malloc(sizeof(picoquic_path_t));

        if (path_x != NULL)
        {
            memset(path_x, 0, sizeof(picoquic_path_t));
            /* Register the sequence number */
            path_x->unique_path_id = unique_path_id;
            path_x->cnx = cnx;
            picoquic_tuple_t* tuple = picoquic_create_tuple(path_x, local_addr, peer_addr, if_index);

            if (tuple != NULL) {
                /* Initialize per path time measurement */
                path_x->smoothed_rtt = PICOQUIC_INITIAL_RTT;
                path_x->rtt_variant = 0;
                path_x->retransmit_timer = PICOQUIC_INITIAL_RETRANSMIT_TIMER;
                path_x->rtt_min = 0;

                /* Initialize per path congestion control state */
                path_x->cwin = PICOQUIC_CWIN_INITIAL;
                path_x->bytes_in_transit = 0;
                path_x->congestion_alg_state = NULL;

                /* Initialize per path pacing state */
                picoquic_pacing_init(&path_x->pacing, start_time);

                /* Initialize the MTU */
                path_x->send_mtu = (peer_addr == NULL || peer_addr->sa_family == AF_INET) ? PICOQUIC_INITIAL_MTU_IPV4 : PICOQUIC_INITIAL_MTU_IPV6;

                /* initialize the quality reporting thresholds */
                path_x->rtt_update_delta = cnx->rtt_update_delta;
                path_x->pacing_rate_update_delta = cnx->pacing_rate_update_delta;
                picoquic_refresh_path_quality_thresholds(path_x);

                /* In case of unique path_id multipath, initialize the context. We do that systematically,
                 * because path 0 is created before multipath options are negotiated.
                 */
                picoquic_init_ack_ctx(cnx, &path_x->ack_ctx);
                picoquic_init_packet_ctx(cnx, &path_x->pkt_ctx, picoquic_packet_context_application);
                /* Record the path */
                cnx->path[cnx->nb_paths] = path_x;
                ret = cnx->nb_paths++;

                /* Set the challenge used for this path */
                picoquic_set_path_challenge(cnx, cnx->nb_paths - 1, start_time);
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<Self, crate::Error> {
        use core::net::{IpAddr, Ipv4Addr};
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let zero_instant = crate::Instant::from_ticks(0);
        let zero_dur = crate::Duration::from_ticks(0);
        let mut path = Self {
            registered_peer_addr: peer_addr.copied().unwrap_or(default_addr),
            connection_by_net_membership: None,
            unique_path_id,
            app_path_ctx: None,
            ack_ctx: AckContext {
                sack_list: SackList::new(),
                time_stamp_largest_received: zero_instant,
                act: [
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: zero_instant,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: zero_instant,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                ],
                crypto_rotation_sequence: 0,
                ecn_ect0_total_local: 0,
                ecn_ect1_total_local: 0,
                ecn_ce_total_local: 0,
                sending_ecn_ack: false,
            },
            pkt_ctx: PacketContextState {
                send_sequence: 0,
                next_sequence_hole: 0,
                retransmit_sequence: 0,
                highest_acknowledged: 0,
                latest_time_acknowledged: zero_instant,
                highest_acknowledged_time: zero_instant,
                pending: BTreeMap::new(),
                retransmitted: BTreeMap::new(),
                preemptive_repeat_seq: None,
                retransmitted_queue_size: 0,
                ecn_ect0_total_remote: 0,
                ecn_ect1_total_remote: 0,
                ecn_ce_total_remote: 0,
                ack_of_ack_requested: false,
            },
            tuples: Vec::new(),
            observed_address_received: 0,
            observed_sequence_sent: 0,
            observed_addr_acked: false,
            last_non_path_probing_pn: 0,
            demotion_time: zero_instant,
            last_sent_time: zero_instant,
            status_sequence_to_receive_next: 0,
            status_sequence_sent_last: 0,
            mtu_probe_sent: false,
            path_is_published: false,
            path_is_backup: false,
            path_is_demoted: false,
            path_abandon_received: false,
            path_abandon_sent: false,
            current_spin: false,
            last_bw_estimate_path_limited: false,
            path_cid_rotated: false,
            is_nat_challenge: false,
            is_cc_data_updated: false,
            is_multipath_probe_needed: false,
            is_ssthresh_initialized: false,
            is_token_published: false,
            is_ticket_seeded: false,
            is_bdp_sent: false,
            is_nominal_ack_path: false,
            is_ack_lost: false,
            is_ack_expected: false,
            is_datagram_ready: false,
            is_pto_required: false,
            is_probing_nat: false,
            is_lost_feedback_notified: false,
            is_cca_probing_up: false,
            rtt_is_initialized: false,
            sending_path_cid_blocked_frame: false,
            last_packet_received_at: zero_instant,
            last_loss_event_detected: zero_instant,
            nb_retransmit: 0,
            total_bytes_lost: 0,
            nb_losses_found: 0,
            nb_timer_losses: 0,
            nb_spurious: 0,
            nb_losses_reported: 0,
            q_square: 0,
            max_ack_delay: crate::internal::ACK_DELAY_MAX_DEFAULT,
            rtt_sample: zero_dur,
            one_way_delay_sample: zero_dur,
            smoothed_rtt: INITIAL_RTT,
            rtt_variant: zero_dur,
            retransmit_timer: INITIAL_RETRANSMIT_TIMER,
            rtt_min: INITIAL_RTT,
            rtt_max: zero_dur,
            max_spurious_rtt: zero_dur,
            max_reorder_delay: zero_dur,
            max_reorder_gap: 0,
            latest_sent_time: zero_instant,
            rtt_packet_previous_period: zero_dur,
            rtt_time_previous_period: zero_dur,
            nb_rtt_estimate_in_period: 0,
            sum_rtt_estimate_in_period: zero_dur,
            max_rtt_estimate_in_period: zero_dur,
            min_rtt_estimate_in_period: zero_dur,
            send_mtu: ENFORCED_INITIAL_MTU,
            send_mtu_max_tried: 0,
            delivered: 0,
            delivered_last: 0,
            delivered_time_last: zero_instant,
            delivered_sent_last: zero_instant.ticks(),
            delivered_limited_index: 0,
            delivered_last_packet: 0,
            bandwidth_estimate: 0,
            bandwidth_estimate_max: 0,
            max_sample_acked_time: zero_instant,
            max_sample_sent_time: zero_instant,
            max_sample_delivered: 0,
            peak_bandwidth_estimate: 0,
            bytes_sent: 0,
            received: 0,
            receive_rate_epoch: 0,
            received_prior: 0,
            receive_rate_estimate: 0,
            receive_rate_max: 0,
            cwin: CWIN_INITIAL,
            bytes_in_transit: 0,
            last_sender_limited_time: zero_instant,
            last_cwin_blocked_time: zero_instant,
            last_time_acked_data_frame_sent: zero_instant,
            congestion_alg_state: None,
            pacing: Pacing {
                rate: 0,
                evaluation_time: zero_instant,
                bucket_max: 0,
                packet_time_microsec: zero_dur,
                quantum_max: 0,
                rate_max: 0,
                bandwidth_pause: 0,
                bucket_nanosec: 0,
                packet_time_nanosec: 0,
            },
            nb_mtu_losses: 0,
            lost_after_delivered: 0,
            responder: 0,
            challenger: 0,
            polled: 0,
            paced: 0,
            congested: 0,
            selected: 0,
            nb_delay_outliers: 0,
            rtt_update_delta: connection.rtt_update_delta,
            pacing_rate_update_delta: connection.pacing_rate_update_delta,
            rtt_threshold_low: zero_dur,
            rtt_threshold_high: zero_dur,
            pacing_rate_threshold_low: 0,
            pacing_rate_threshold_high: 0,
            receive_rate_threshold_low: 0,
            receive_rate_threshold_high: 0,
            rtt_min_remote: zero_dur,
            cwin_remote: 0,
            ip_client_remote: [0u8; 16],
            ip_client_remote_length: 0,
        };
        // Create the initial tuple.
        path.create_tuple(local_addr, peer_addr, if_index)?;
        let _ = start_time;
        Ok(path)
    }
```

## `picoquic/quicctx.c:picoquic_get_next_wake_delay`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body computes next_wake_time via picoquic_get_next_wake_time and returns 0 when it is not after current_time; Rust body directly takes the minimum connection wake time and returns delay_max when there are no connections.
* C source: `picoquic/quicctx.c:1553-1577`
* C signature: `int64_t picoquic_get_next_wake_delay(picoquic_quic_t *, uint64_t, int64_t)`
* Rust source: `rs/fq/src/lib.rs:3194-3207`
* Rust item: `next_wake_delay`

### C body
```c
{
    /* We assume that "current time" is no more than 100,000 years in the
     * future, which implies the time in microseconds is less than 2^62.
     * The delay MAX is lower than INT64_MAX, i.e., 2^63.
     * The next wake time is often set to UINT64_MAX, and might sometime
     * be just under that value, so we make sure to avoid integer
     * overflow in the computation.
     */
    uint64_t next_wake_time = picoquic_get_next_wake_time(quic, current_time);
    int64_t wake_delay = 0;

    if (next_wake_time > current_time) {
        uint64_t delta_m = current_time + delay_max;

        if (next_wake_time >= delta_m) {
            wake_delay = delay_max;
        }
        else {
            wake_delay = (int64_t)(next_wake_time - current_time);
        }
    }
    return wake_delay;
}
```

### Rust body
```rust
    pub fn next_wake_delay(&self, current_time: Instant, delay_max: i64) -> i64 {
        let now = current_time.ticks();
        // Find the minimum next_wake_time across all connections.
        let earliest = self
            .connections
            .iter()
            .map(|c| c.next_wake_time.ticks())
            .min();
        match earliest {
            None => delay_max,
            Some(t) if t <= now => 0,
            Some(t) => ((t - now) as i64).min(delay_max),
        }
    }
```
