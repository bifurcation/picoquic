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

## Pair `picoquic/sender.c:picoquic_prepare_packet_0rtt`
C: `picoquic/sender.c:1649-1743 picoquic_prepare_packet_0rtt`
Rust: `rs/fq/src/internal.rs:17856-17994 prepare_packet_0rtt`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream = NULL;
    picoquic_packet_type_enum packet_type = picoquic_packet_0rtt_protected;
    size_t header_length = 0;
    uint8_t* bytes = packet->bytes;
    size_t length = 0;
    size_t checksum_overhead = picoquic_aead_get_checksum_length(cnx->crypto_context[1].aead_encrypt);
    uint8_t* bytes_max;
    uint8_t* bytes_next;
    int more_data = 0;
    int is_pure_ack = 1;
    int stream_tried_and_failed = 0;

    send_buffer_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;
    if (path_x->bytes_in_transit + send_buffer_max > PICOQUIC_DEFAULT_0RTT_WINDOW) {
        if (path_x->bytes_in_transit > PICOQUIC_DEFAULT_0RTT_WINDOW) {
            send_buffer_max = 0;
        }
        else {
            send_buffer_max = (size_t)PICOQUIC_DEFAULT_0RTT_WINDOW - (size_t)path_x->bytes_in_transit;
        }
    }
    bytes_max = bytes + send_buffer_max - checksum_overhead;

    stream = picoquic_find_ready_stream(cnx);
    length = picoquic_predict_packet_header_length(cnx, packet_type, &cnx->pkt_ctx[picoquic_packet_context_application]);
    packet->ptype = picoquic_packet_0rtt_protected;
    packet->offset = length;
    header_length = length;
    packet->pc = picoquic_packet_context_application;
    packet->sequence_number = cnx->pkt_ctx[picoquic_packet_context_application].send_sequence;
    packet->send_time = current_time;
    packet->send_path = path_x;
    packet->checksum_overhead = checksum_overhead;
    bytes_next = bytes + length;


    
    /* Consider sending 0-RTT */
    if ((stream == NULL && cnx->first_misc_frame == NULL && padding_required == 0) || 
        send_buffer_max < PICOQUIC_MIN_SEGMENT_SIZE) {
        length = 0;
    } else {
        /* If present, send misc frame */
        bytes_next = picoquic_format_misc_frames_in_context(cnx, bytes_next, bytes_max,
            &more_data, &is_pure_ack, picoquic_packet_context_application);

        /* We assume that if BDP data is associated with the zero RTT ticket, it can be sent */
        /* Encode the bdp frame */
        if (cnx->local_parameters.enable_bdp_frame) {
            bytes_next = picoquic_format_bdp_frame(cnx, bytes_next, bytes_max, path_x, &more_data, &is_pure_ack);
        }

        /* Encode the stream frame, or frames */
        bytes_next = picoquic_format_available_stream_frames(cnx, NULL, bytes_next, bytes_max, UINT64_MAX,
            &more_data, &is_pure_ack, &stream_tried_and_failed, &ret);

        length = bytes_next - bytes;

        if (more_data) {
            *next_wake_time = current_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }

        if (stream_tried_and_failed) {
            path_x->last_sender_limited_time = current_time;
        }

        /* Add padding if required */
        if (padding_required) {
            length = picoquic_pad_to_target_length(bytes, length, send_buffer_max - checksum_overhead);
        }
    }

    picoquic_finalize_and_protect_packet(cnx, packet,
        ret, length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_max,
        path_x, current_time);

    if (length > 0) {
        /* Accounting of zero rtt packets sent */
        cnx->nb_zero_rtt_sent++;
    }

    /* the reinsertion by wake up time will happen in the calling function */

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut ret = 0;
        let packet_type = PacketType::ZeroRttProtected;
        let checksum_overhead = self.get_checksum_length(Epoch::ZeroRtt);
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let mut stream_tried_and_failed = 0;

        send_buffer_max = send_buffer_max.min(path_x.send_mtu);
        if path_x
            .bytes_in_transit
            .saturating_add(send_buffer_max as u64)
            > DEFAULT_0RTT_WINDOW as u64
        {
            send_buffer_max = if path_x.bytes_in_transit > DEFAULT_0RTT_WINDOW as u64 {
                0
            } else {
                DEFAULT_0RTT_WINDOW.saturating_sub(path_x.bytes_in_transit as usize)
            };
        }
        let header_length =
            self.predict_packet_header_length_for_pc(packet_type, PacketContext::Application);
        let mut length = header_length;
        packet.packet_type = packet_type;
        packet.offset = header_length;
        packet.packet_context = PacketContext::Application;
        packet.sequence_number = self.pkt_ctx[PacketContext::Application as usize].send_sequence;
        packet.send_time = current_time;
        packet.send_path = Some(Self::path_token_for_path(path_x));
        packet.checksum_overhead = checksum_overhead;

        if (self.find_ready_stream().is_none()
            && self.misc_frames.is_empty()
            && padding_required == 0)
            || send_buffer_max < MIN_SEGMENT_SIZE
            || send_buffer_max <= checksum_overhead
        {
            length = 0;
        } else {
            let bytes_limit = send_buffer_max
                .saturating_sub(checksum_overhead)
                .min(packet.bytes.len());
            if length <= bytes_limit {
                let mut offset = length;
                let tail_len = {
                    let tail = &mut packet.bytes[offset..bytes_limit];
                    match format_misc_frames_in_context(
                        self,
                        tail,
                        &mut more_data,
                        &mut is_pure_ack,
                        PacketContext::Application,
                    ) {
                        Some(next) => next.len(),
                        None => {
                            ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                            tail.len()
                        }
                    }
                };
                offset = bytes_limit.saturating_sub(tail_len);
                if ret == 0 && self.local_parameters.enable_bdp_frame {
                    let tail_len = {
                        let tail = &mut packet.bytes[offset..bytes_limit];
                        match format_bdp_frame(self, tail, path_x, &mut more_data, &mut is_pure_ack)
                        {
                            Some(next) => next.len(),
                            None => {
                                ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                                tail.len()
                            }
                        }
                    };
                    offset = bytes_limit.saturating_sub(tail_len);
                }
                if ret == 0 {
                    let tail_len = {
                        let tail = &mut packet.bytes[offset..bytes_limit];
                        match format_available_stream_frames(
                            self,
                            path_x,
                            tail,
                            u64::MAX,
                            &mut more_data,
                            &mut is_pure_ack,
                            &mut stream_tried_and_failed,
                            &mut ret,
                        ) {
                            Some(next) => next.len(),
                            None => {
                                ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                                tail.len()
                            }
                        }
                    };
                    offset = bytes_limit.saturating_sub(tail_len);
                }
                length = offset;
            }
            self.note_more_data(more_data, next_wake_time, current_time);
            if stream_tried_and_failed != 0 {
                path_x.last_sender_limited_time = current_time;
            }
            if padding_required != 0 && length > 0 {
                length = pad_to_target_length(
                    &mut packet.bytes,
                    length,
                    send_buffer_max.saturating_sub(checksum_overhead),
                );
            }
        }

        self.finalize_and_protect_packet(
            packet,
            ret,
            length,
            header_length,
            checksum_overhead,
            send_length,
            send_buffer,
            send_buffer_max,
            path_x,
            current_time,
        );
        if length > 0 {
            self.nb_zero_rtt_sent = self.nb_zero_rtt_sent.saturating_add(1);
        }
        ret
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_server_address_migration`
C: `picoquic/sender.c:1868-1930 picoquic_prepare_server_address_migration`
Rust: `rs/fq/src/internal.rs:7595-7683 picoquic_prepare_server_address_migration`

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

## Pair `picoquic/sender.c:picoquic_format_new_local_id_as_needed`
C: `picoquic/sender.c:2596-2658 picoquic_format_new_local_id_as_needed`
Rust: `rs/fq/src/internal.rs:18342-18431 format_new_local_id_as_needed`

### C body
```c
{
    int no_space_left = 0;
    picoquic_local_cnxid_list_t* local_cnxid_list = cnx->first_local_cnxid_list;
    if (cnx->is_multipath_enabled) {
        /* If the number of local list is lower than the max number of paths, 
        * update that number and queue a MAX PATH ID frame.
         */
        uint64_t new_max_path_id = cnx->next_path_id_in_lists +
            cnx->local_parameters.initial_max_path_id -
            cnx->nb_local_cnxid_lists;
        if (cnx->max_path_id_local < new_max_path_id) {
            uint8_t * bytes_next = picoquic_format_max_path_id_frame(bytes, bytes_max, new_max_path_id, more_data);
            if (bytes_next == bytes) {
                no_space_left = 1;
            }
            else {
                bytes = bytes_next;
                cnx->max_path_id_local = new_max_path_id;
            }
        }
        /* If the number of local lists is lower than the max number of paths,
         * create more. The code assume that path[0] is created during handshake. */
        while (!no_space_left && cnx->nb_local_cnxid_lists <= cnx->local_parameters.initial_max_path_id &&
            cnx->next_path_id_in_lists <= cnx->max_path_id_remote) {
            (void) picoquic_find_or_create_local_cnxid_list(cnx, cnx->next_path_id_in_lists, 1);
        }
    }

    while (local_cnxid_list != NULL && !no_space_left) {
        /* Check whether time has comed to obsolete local CID */
        picoquic_check_local_cnxid_ttl(cnx, local_cnxid_list, current_time, next_wake_time);

        /* Push new CID if needed */
        while (
            local_cnxid_list->nb_local_cnxid < ((int)(cnx->remote_parameters.active_connection_id_limit) + local_cnxid_list->nb_local_cnxid_expired) &&
            local_cnxid_list->nb_local_cnxid <= (PICOQUIC_NB_PATH_TARGET + local_cnxid_list->nb_local_cnxid_expired)) {
            uint8_t* bytes0 = bytes;
            picoquic_local_cnxid_t* l_cid = picoquic_create_local_cnxid(cnx, local_cnxid_list->unique_path_id, NULL, current_time);

            if (l_cid == NULL) {
                /* OOPS, no memory left */
                no_space_left = 1;
                break;
            }
            else {
                bytes = picoquic_format_new_connection_id_frame(cnx, local_cnxid_list, bytes, bytes_max, more_data, is_pure_ack, l_cid);

                if (bytes == bytes0) {
                    no_space_left = 1;
                    /* Oops. Try again next time. */
                    picoquic_delete_local_cnxid(cnx, l_cid);
                    local_cnxid_list->local_cnxid_sequence_next--;
                    break;
                }
            }
        }
        local_cnxid_list = local_cnxid_list->next_list;
    }
    return bytes;
}
```

### Rust body
```rust
    ) -> Option<&'a mut [u8]> {
        let mut no_space_left = false;

        if self.is_multipath_enabled {
            let new_max_path_id = self
                .next_path_id_in_lists
                .saturating_add(self.local_parameters.initial_max_path_id)
                .saturating_sub(self.local_connection_id_lists.len() as u64);
            if self.max_path_id_local < new_max_path_id {
                let before = bytes.len();
                bytes = format_max_path_id_frame(bytes, new_max_path_id, more_data)?;
                if bytes.len() == before {
                    no_space_left = true;
                } else {
                    self.max_path_id_local = new_max_path_id;
                }
            }
            while !no_space_left
                && (self.local_connection_id_lists.len() as u64)
                    <= self.local_parameters.initial_max_path_id
                && self.next_path_id_in_lists <= self.max_path_id_remote
            {
                let unique_path_id = self.next_path_id_in_lists;
                if self
                    .local_connection_id_lists
                    .iter()
                    .all(|list| list.unique_path_id != unique_path_id)
                {
                    self.local_connection_id_lists.push(LocalConnectionIdList {
                        unique_path_id,
                        local_connection_id_sequence_next: 0,
                        local_connection_id_retire_before: 0,
                        local_connection_id_oldest_created: current_time.ticks(),
                        nb_local_connection_id_expired: 0,
                        is_demoted: false,
                        demotion_time: Instant::from_ticks(u64::MAX),
                        connection_ids: Vec::new(),
                    });
                }
                self.next_path_id_in_lists = self.next_path_id_in_lists.saturating_add(1);
            }
        }

        let mut list_index = 0usize;
        while list_index < self.local_connection_id_lists.len() && !no_space_left {
            let mut list = self.local_connection_id_lists.remove(list_index);
            self.check_local_connection_id_ttl(&mut list, current_time, next_wake_time);

            while (list.connection_ids.len() as i32)
                < self.remote_parameters.active_connection_id_limit as i32
                    + list.nb_local_connection_id_expired
                && (list.connection_ids.len() as i32)
                    <= NB_PATH_TARGET as i32 + list.nb_local_connection_id_expired
            {
                let token = match self.create_local_connection_id_in_list(&mut list, current_time) {
                    Ok(token) => token,
                    Err(_) => {
                        no_space_left = true;
                        break;
                    }
                };
                let before = bytes.len();
                bytes = format_new_connection_id_frame(
                    self,
                    &mut list,
                    bytes,
                    more_data,
                    is_pure_ack,
                    Some(token),
                )?;
                if bytes.len() == before {
                    no_space_left = true;
                    self.delete_local_connection_id(token);
                    list.local_connection_id_sequence_next =
                        list.local_connection_id_sequence_next.saturating_sub(1);
                    break;
                }
            }
            self.local_connection_id_lists.insert(list_index, list);
            list_index += 1;
        }
        Some(bytes)
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_datagram_ready`
C: `picoquic/sender.c:2777-2801 picoquic_prepare_datagram_ready`
Rust: `rs/fq/src/internal.rs:17617-17659 picoquic_prepare_datagram_ready`

### C body
```c
{
    uint8_t* bytes0 = bytes_next;

    if (cnx->first_datagram != NULL) {
        bytes_next = picoquic_format_first_datagram_frame(cnx, bytes_next, bytes_max, is_first_in_packet, more_data, is_pure_ack);
        *more_data |= (cnx->first_datagram != NULL);
    }
    else {
        while (cnx->is_datagram_ready || path_x->is_datagram_ready) {
            uint8_t* dg_start = bytes_next;
            bytes_next = picoquic_format_ready_datagram_frame(cnx, path_x, bytes_next, bytes_max,
                more_data, is_pure_ack, ret);
            if (bytes_next == NULL || bytes_next == dg_start) {
                break;
            }
        }
    }
    *datagram_tried_and_failed = (bytes_next == bytes0);
    *datagram_sent = !*datagram_tried_and_failed;

    return bytes_next;
}
```

### Rust body
```rust
    ) -> Option<&'a mut [u8]> {
        let bytes0_len = bytes_next.len();

        if !self.datagrams.is_empty() {
            bytes_next = format_first_datagram_frame(
                self,
                bytes_next,
                is_first_in_packet,
                more_data,
                is_pure_ack,
            )?;
            *more_data |= i32::from(!self.datagrams.is_empty());
        } else {
            while self.is_datagram_ready || path_x.is_datagram_ready {
                let dg_start_len = bytes_next.len();
                bytes_next = format_ready_datagram_frame(
                    self,
                    path_x,
                    bytes_next,
                    more_data,
                    is_pure_ack,
                    ret,
                )?;
                if bytes_next.len() == dg_start_len {
                    break;
                }
            }
        }

        *datagram_tried_and_failed = i32::from(bytes_next.len() == bytes0_len);
        *datagram_sent = i32::from(*datagram_tried_and_failed == 0);
        Some(bytes_next)
    }
```

## Pair `picoquic/sender.c:picoquic_check_idle_timer`
C: `picoquic/sender.c:3719-3759 picoquic_check_idle_timer`
Rust: `rs/fq/src/internal.rs:16950-17003 check_idle_timer`

### C body
```c
{
    int ret = 0;
    uint64_t idle_timer = 0;

    if (cnx->cnx_state >= picoquic_state_ready) {
        uint64_t rto = picoquic_current_retransmit_timer(cnx, cnx->path[0]);
        idle_timer = cnx->idle_timeout;
        if (idle_timer < 3 * rto) {
            idle_timer = 3 * rto;
        }
        idle_timer += cnx->latest_receive_time;

        if (idle_timer < cnx->idle_timeout) {
            idle_timer = UINT64_MAX;
        }
    }
    else if (cnx->quic->default_handshake_timeout > 0) {
        idle_timer = cnx->start_time + cnx->quic->default_handshake_timeout;
    }
    else if (cnx->local_parameters.max_idle_timeout > 0) {
        idle_timer = cnx->start_time + cnx->local_parameters.max_idle_timeout*1000ull;
    }
    else {
        idle_timer = cnx->start_time + PICOQUIC_MICROSEC_HANDSHAKE_MAX;
    }

    if (current_time >= idle_timer) {
        /* Too long silence, break it. */
        if (cnx->cnx_state != picoquic_state_draining) {
            cnx->local_error = PICOQUIC_ERROR_IDLE_TIMEOUT;
        }
        ret = PICOQUIC_ERROR_DISCONNECTED;
        picoquic_connection_disconnect(cnx);
    } else if (idle_timer < *next_wake_time) {
        *next_wake_time = idle_timer;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn check_idle_timer(&mut self, next_wake_time: &mut Instant, current_time: Instant) -> i32 {
        let idle_timer = if self.connection_state >= State::Ready {
            let rto = self
                .paths
                .first()
                .map(|path| self.current_retransmit_timer_ticks_for_path(path))
                .unwrap_or(INITIAL_RETRANSMIT_TIMER.ticks());
            let mut idle = self.idle_timeout.ticks();
            if idle < 3u64.saturating_mul(rto) {
                idle = 3u64.saturating_mul(rto);
            }
            let candidate = self.latest_receive_time.ticks().saturating_add(idle);
            if candidate < self.idle_timeout.ticks() {
                u64::MAX
            } else {
                candidate
            }
        } else if self
            .quic_ref()
            .map(|q| q.default_handshake_timeout.ticks() > 0)
            .unwrap_or(false)
        {
            self.start_time.ticks().saturating_add(
                self.quic_ref()
                    .map(|q| q.default_handshake_timeout.ticks())
                    .unwrap_or(0),
            )
        } else if self.local_parameters.max_idle_timeout.ticks() > 0 {
            self.start_time.ticks().saturating_add(
                self.local_parameters
                    .max_idle_timeout
                    .ticks()
                    .saturating_mul(1000),
            )
        } else {
            self.start_time
                .ticks()
                .saturating_add(MICROSEC_HANDSHAKE_MAX.ticks())
        };

        if current_time.ticks() >= idle_timer {
            if self.connection_state != State::Draining {
                self.local_error = crate::errors::InternalError::IdleTimeout as u64;
            }
            self.connection_disconnect();
            crate::errors::InternalError::Disconnected as i32
        } else {
            let idle = Instant::from_ticks(idle_timer);
            if idle < *next_wake_time {
                *next_wake_time = idle;
            }
            0
        }
    }
```

## Pair `picoquic/sender.c:picoquic_handle_app_wake_time`
C: `picoquic/sender.c:3881-3892 picoquic_handle_app_wake_time`
Rust: `rs/fq/src/internal.rs:16768-16783 handle_app_wake_time`

### C body
```c
{
    int ret = 0;
    while (cnx->app_wake_time != 0 && cnx->app_wake_time <= current_time){
        cnx->app_wake_time = 0;
        if (cnx->callback_fn != NULL) {
            ret = cnx->callback_fn(cnx, current_time, NULL, 0, picoquic_callback_app_wakeup,
                cnx->callback_ctx, NULL);
        }
    }
    return ret;
}
```

### Rust body
```rust
        while self.app_wake_time.ticks() != 0 && self.app_wake_time <= current_time {
            self.app_wake_time = Instant::from_ticks(0);
            if let Some(mut cb) = self.callback_fn.take() {
                // C passes `current_time` in the `stream_id` slot for AppWakeup.
                ret = cb.callback(
                    self,
                    current_time.ticks(),
                    &[],
                    CallbackEvent::AppWakeup,
                    None,
                );
                self.callback_fn = Some(cb);
            }
        }
```

## Pair `picoquic/sender.c:picoquic_handle_send_train_statistics`
C: `picoquic/sender.c:3959-3979 picoquic_handle_send_train_statistics`
Rust: `rs/fq/src/internal.rs:19534-19555 handle_send_train_statistics`

### C body
```c
{
    cnx->nb_trains_sent++;
    if (send_msg_size != NULL) {
        if (coalesced_packet_size == 0 && *send_length < 8 * (*send_msg_size)) {
            if (path_x->cwin <= path_x->bytes_in_transit) {
                cnx->nb_trains_blocked_cwin++;
            }
            else if (picoquic_is_pacing_blocked(&path_x->pacing)) {
                cnx->nb_trains_blocked_pacing++;
            }
            else {
                cnx->nb_trains_blocked_others++;
            }
        }
        else {
            cnx->nb_trains_short++;
        }
    }
}
```

### Rust body
```rust
    ) {
        self.nb_trains_sent = self.nb_trains_sent.saturating_add(1);
        if let Some(msg_size) = send_msg_size {
            if coalesced_packet_size == 0 && send_length < 8usize.saturating_mul(msg_size) {
                if path_x.cwin <= path_x.bytes_in_transit {
                    self.nb_trains_blocked_cwin = self.nb_trains_blocked_cwin.saturating_add(1);
                } else if path_x.pacing.is_blocked() {
                    self.nb_trains_blocked_pacing = self.nb_trains_blocked_pacing.saturating_add(1);
                } else {
                    self.nb_trains_blocked_others = self.nb_trains_blocked_others.saturating_add(1);
                }
            } else {
                self.nb_trains_short = self.nb_trains_short.saturating_add(1);
            }
        }
    }
```

## Pair `picoquic/sender.c:picoquic_close_ex`
C: `picoquic/sender.c:4201-4222 picoquic_close_ex`
Rust: `rs/fq/src/internal.rs:5748-5804 picoquic_close_ex`

### C body
```c
{
    int ret = 0;
    uint64_t current_time = picoquic_get_quic_time(cnx->quic);

    if (cnx->cnx_state == picoquic_state_ready ||
        cnx->cnx_state == picoquic_state_server_false_start || cnx->cnx_state == picoquic_state_client_ready_start) {
        cnx->cnx_state = picoquic_state_disconnecting;
        cnx->application_error = application_reason_code;
    } else if (cnx->cnx_state < picoquic_state_client_ready_start) {
        cnx->cnx_state = picoquic_state_handshake_failure;
        cnx->application_error = 0;
        cnx->local_error = PICOQUIC_TRANSPORT_APPLICATION_ERROR;
    } else {
        ret = -1;
    }
    cnx->local_error_reason = error_reason;
    cnx->offending_frame_type = 0;
    picoquic_reinsert_by_wake_time(cnx->quic, cnx, current_time);

    return ret;
}
```

### Rust body
```rust
    ) -> crate::Result<()> {
        let current_time = Instant::from_ticks(
            self.quic_ref()
                .map(|quic| quic.time())
                .unwrap_or_else(crate::current_time),
        );
        let ret = if matches!(
            self.connection_state,
            State::Ready | State::ServerFalseStart | State::ClientReadyStart
        ) {
            self.connection_state = State::Disconnecting;
            self.application_error = application_reason_code;
            Ok(())
        } else if self.connection_state < State::ClientReadyStart {
            self.connection_state = State::HandshakeFailure;
            self.application_error = 0;
            self.local_error = crate::TransportError::ApplicationError as u64;
            Ok(())
        } else {
            Err(crate::Error::InvalidState)
        };

        self.local_error_reason = error_reason.map(str::to_owned);
        self.offending_frame_type = 0;
        self.next_wake_time = current_time;

        let own_token = self.own_token;
        let old_membership = self.connection_wake_membership.take();
        let mut new_membership = None;
        if let Some(token) = own_token
            && let Some(quic) = self.quic_mut()
            && quic.connections.contains(token)
        {
            if let Some(old_membership) = old_membership {
                quic.connection_wake_tree.remove(old_membership);
            }
            if let Ok((tree_token, old_token)) = quic
                .connection_wake_tree
                .insert(current_time.ticks(), token)
            {
                new_membership = Some(tree_token);
                if let Some(old_token) = old_token
                    && old_token != token
                    && let Some(old_connection) = quic.connections.get_mut(old_token)
                {
                    old_connection.connection_wake_membership = None;
                }
            }
        }
        self.connection_wake_membership = new_membership;

        ret
    }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_create`
C: `picoquic/sim_link.c:38-62 picoquictest_sim_link_create`
Rust: `rs/fq/src/tests/util.rs:335-373 create`

### C body
```c
{
    picoquictest_sim_link_t* link = (picoquictest_sim_link_t*)malloc(sizeof(picoquictest_sim_link_t));
    if (link != NULL) {
        double pico_d = (data_rate_in_gps <= 0) ? 0 : (8000.0 / data_rate_in_gps);
        memset(link, 0, sizeof(picoquictest_sim_link_t));
        pico_d *= (1.024 * 1.024); /* account for binary units */
        link->next_send_time = current_time;
        link->queue_time = current_time;
        link->queue_delay_max = queue_delay_max;
        link->picosec_per_byte = (uint64_t)pico_d; 
        link->microsec_latency = microsec_latency;
        link->packets_dropped = 0;
        link->packets_sent = 0;
        link->first_packet = NULL;
        link->last_packet = NULL;
        link->loss_mask = loss_mask;
        link->jitter_seed = 0xDEADBEEFBABAC001ull;
        link->jitter = 0;
        link->path_mtu = PICOQUIC_MAX_PACKET_SIZE;
    }

    return link;
}
```

### Rust body
```rust
    ) -> Result<Self, crate::Error> {
        let pico_d = if data_rate_in_gbps <= 0.0 {
            0.0
        } else {
            8000.0 / data_rate_in_gbps
        };
        let pico_d = pico_d * 1.024 * 1.024;
        Ok(Self {
            next_send_time: current_time,
            queue_time: current_time,
            resume_time: Instant::from_ticks(0),
            queue_delay_max,
            picosec_per_byte: pico_d as u64,
            microsec_latency,
            packets_dropped: 0,
            packets_sent: 0,
            jitter: 0,
            jitter_mode: JitterMode::Gauss,
            jitter_seed: 0xDEAD_BEEF_BABA_C001u64,
            path_mtu: MAX_PACKET_SIZE,
            packets: std::collections::VecDeque::new(),
            loss_mask,
            nb_loss_in_burst: 0,
            packets_between_losses: 0,
            packets_sent_next_burst: 0,
            nb_losses_this_burst: 0,
            end_of_burst_time: Instant::from_ticks(0),
            aqm_state: None,
            is_switched_off: false,
            is_unreachable: false,
            is_suspended: false,
        })
    }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_admit_pending`
C: `picoquic/sim_link.c:118-123 picoquictest_sim_link_admit_pending`
Rust: `rs/fq/src/tests/util.rs:395-399 admit_pending`

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

## Pair `picoquic/sim_link.c:picoquictest_sim_link_wifi_jitter`
C: `picoquic/sim_link.c:201-228 picoquictest_sim_link_wifi_jitter`
Rust: `rs/fq/src/tests/harness.rs:60-85 picoquictest_sim_link_wifi_jitter`

### C body
```c
{
    const uint64_t exp_minus_1_x40000000 = 395007542; /* exp(-1) time 2^30 */
    const uint64_t primary_jitter = 1000;
    uint64_t N1 = picoquic_test_poisson_random(&link->jitter_seed, exp_minus_1_x40000000);
    uint64_t jitter = N1 * primary_jitter;
    if (N1 > 0) {
        /* smoothing variable */
        jitter  -= picoquic_test_uniform_random(&link->jitter_seed, primary_jitter);
    }

    if (link->jitter > 1000) {
        uint64_t r = picoquic_test_random(&link->jitter_seed);
        r ^= r >> 30;
        r &= 0x3fffffff;
        r *= 84000;
        if (r < ((link->jitter - 1000) << 30)) {
            const uint64_t exp_minus_12_x40000000 = 6597; /* exp(-12) time 2^30 */
            const uint64_t secondary_jitter = 7500;
            uint64_t N2 = picoquic_test_poisson_random(&link->jitter_seed, exp_minus_12_x40000000);
            jitter += N2 * secondary_jitter;
            if (N2 > 1) {
                jitter -= picoquic_test_uniform_random(&link->jitter_seed, secondary_jitter);
            }
        }
    }
    return jitter;
}
```

### Rust body
```rust
pub fn picoquictest_sim_link_wifi_jitter(link: &mut TestSimLink) -> u64 {
    const EXP_MINUS_1_X40000000: u64 = 395_007_542;
    const PRIMARY_JITTER: u64 = 1000;
    let n1 = test_poisson_random(&mut link.jitter_seed, EXP_MINUS_1_X40000000);
    let mut jitter = n1 * PRIMARY_JITTER;
    if n1 > 0 {
        jitter -= test_uniform_random(&mut link.jitter_seed, PRIMARY_JITTER);
    }

    if link.jitter > 1000 {
        let mut r = test_random(&mut link.jitter_seed);
        r ^= r >> 30;
        r &= 0x3fff_ffff;
        r = r.wrapping_mul(84_000);
        if r < ((link.jitter - 1000) << 30) {
            const EXP_MINUS_12_X40000000: u64 = 6597;
            const SECONDARY_JITTER: u64 = 7500;
            let n2 = test_poisson_random(&mut link.jitter_seed, EXP_MINUS_12_X40000000);
            jitter += n2 * SECONDARY_JITTER;
            if n2 > 1 {
                jitter -= test_uniform_random(&mut link.jitter_seed, SECONDARY_JITTER);
            }
        }
    }
    jitter
}
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_queue_delay`
C: `picoquic/sim_link.c:301-304 picoquictest_sim_link_queue_delay`
Rust: `rs/fq/src/tests/util.rs:503-507 queue_delay`

### C body
```c
{
    return (current_time > link->queue_time) ? 0 : link->queue_time - current_time;
}
```

### Rust body
```rust
    pub fn queue_delay(&mut self, current_time: Instant) -> u64 {
        let qt = self.queue_time.ticks();
        let ct = current_time.ticks();
        qt.saturating_sub(ct)
    }
```

## Pair `picoquic/sim_link.c:sim_link_test`
C: `picoquic/sim_link.c:431-449 sim_link_test`
Rust: `rs/fq/src/tests/harness.rs:148-152 sim_link`

### C body
```c
{
    int ret = 0;
    uint64_t loss_mask = 0;
    
    ret = sim_link_one_test(&loss_mask, 0, 0);

    if (ret == 0) {
        loss_mask = 8;
        ret = sim_link_one_test(&loss_mask, 0, 1);
    }

    if (ret == 0) {
        loss_mask = 0x18;
        ret = sim_link_one_test(&loss_mask, 0, 2);
    }

    return ret;
}
```

### Rust body
```rust
fn sim_link() {
    sim_link_one_test(Some(0), 0, 0);
    sim_link_one_test(Some(8), 0, 1);
    sim_link_one_test(Some(0x18), 0, 2);
}
```

## Pair `picoquic/sockloop.c:picoquic_packet_loop_open_socket`
C: `picoquic/sockloop.c:363-462 picoquic_packet_loop_open_socket`
Rust: `rs/fq/src/packet_loop.rs:624-657 packet_loop_open_socket`

### C body
```c
{
    int ret = 0;
    struct sockaddr_storage local_address;
    int recv_set = 0;
    int send_set = 0;
    int opt_val = 1;
#ifdef _WINDOWS
    int recv_coalesced = 0;
    int send_coalesced = 0;

    /* Assess whether coalescing is supported */
    if (!do_not_use_gso) {
        picoquic_sockloop_win_coalescing_test(&recv_coalesced, &send_coalesced);
#if 0
        /* TODO: remove temporary fix, after we figure how to work that out. */
        recv_coalesced = 0;
#endif
    }
    s_ctx->overlap.hEvent = WSA_INVALID_EVENT;
    s_ctx->fd = WSASocket(s_ctx->af, SOCK_DGRAM, IPPROTO_UDP, NULL, 0, WSA_FLAG_OVERLAPPED);
#else
    s_ctx->fd = socket(s_ctx->af, SOCK_DGRAM, IPPROTO_UDP);
#endif


    if (s_ctx->fd == INVALID_SOCKET ||
#ifndef ESP_PLATFORM
        /* TODO: set option IPv6 only */
        picoquic_socket_set_ecn_options_ex(s_ctx->fd, s_ctx->af, &recv_set, &send_set, ecn_value) != 0 ||
#endif
        picoquic_socket_set_pkt_info(s_ctx->fd, s_ctx->af) != 0 ||
        (s_ctx->is_port_shared && setsockopt(s_ctx->fd, SOL_SOCKET, SO_REUSEADDR, (const char*)&opt_val, sizeof(opt_val)) != 0) ||
#if defined(SO_REUSEPORT)
        (s_ctx->is_port_shared && setsockopt(s_ctx->fd, SOL_SOCKET, SO_REUSEPORT, (const char*)&opt_val, sizeof(opt_val)) != 0) ||
#endif
        picoquic_bind_to_port(s_ctx->fd,s_ctx->af, s_ctx->port) != 0 ||
        picoquic_get_local_address(s_ctx->fd, &local_address) != 0 ||
        picoquic_socket_set_pmtud_options(s_ctx->fd, s_ctx->af) != 0)
    {
        DBG_PRINTF("Cannot set socket (af=%d, port = %d)\n", s_ctx->af, s_ctx->port);
        ret = -1;
    }
    else {
        if (local_address.ss_family == AF_INET6) {
            s_ctx->port = ntohs(((struct sockaddr_in6*)&local_address)->sin6_port);
        }
        else if (local_address.ss_family == AF_INET) {
            s_ctx->port = ntohs(((struct sockaddr_in*)&local_address)->sin_port);
        }
        
#ifndef ESP_PLATFORM
        if (socket_buffer_size > 0) {
            socklen_t opt_len;
            int opt_ret;
            int so_sndbuf;
            int so_rcvbuf;
            int last_op = SO_SNDBUF;
            char const* last_op_name = "SO_SNDBUF";

            opt_len = sizeof(int);
            so_sndbuf = socket_buffer_size;
            opt_ret = setsockopt(s_ctx->fd, SOL_SOCKET, SO_SNDBUF, (const char*)&so_sndbuf, opt_len);
            if (opt_ret == 0) {
                last_op = SO_RCVBUF;
                last_op_name = "SO_RECVBUF";
                opt_len = sizeof(int);
                so_rcvbuf = socket_buffer_size;
                opt_ret = setsockopt(s_ctx->fd, SOL_SOCKET, SO_RCVBUF, (const char*)&so_rcvbuf, opt_len);
            }
            if (opt_ret != 0) {
                int so_errbuf = 0;
#ifdef _WINDOWS
                int sock_error = WSAGetLastError();
#else
                int sock_error = errno;
#endif
                opt_ret = getsockopt(s_ctx->fd, SOL_SOCKET, last_op, (char*)&so_errbuf, &opt_len);
                DBG_PRINTF("Cannot set %s to %d, err=%d, so_sndbuf=%d (%d)",
                    last_op_name, socket_buffer_size, sock_error, so_errbuf, opt_ret);
                ret = -1;
            }
        }
#endif

#ifdef _WINDOWS
        if (ret == 0) {
            ret = picoquic_packet_set_windows_socket(send_coalesced, recv_coalesced, s_ctx);
        }
#endif
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    let mut fd = match s_ctx.af {
        AF_INET | AF_INET6 => S::open_udp(s_ctx.af)?,
        _ => return Err(Error::InvalidArgument),
    };

    fd.set_ecn_options_ex(ecn_codepoint(ecn_value))?;
    fd.set_pkt_info()?;
    if s_ctx.is_port_shared {
        fd.set_reuse_addr(true)?;
        fd.set_reuse_port(true)?;
    }
    fd.bind_to_port(s_ctx.af, s_ctx.port as i32)?;
    fd.set_pmtud_options()?;

    let local_address = fd.local_address()?;
    s_ctx.port = local_address.port();
    s_ctx.n_port = s_ctx.port.to_be();
    if socket_buffer_size > 0 {
        let size = usize::try_from(socket_buffer_size).map_err(|_| Error::InvalidArgument)?;
        fd.set_send_buffer_size(size)?;
        fd.set_recv_buffer_size(size)?;
    }
    s_ctx.fd = Some(fd);
    s_ctx.is_started = true;
    s_ctx.supports_udp_send_coalesced = false;
    s_ctx.supports_udp_recv_coalesced = false;
    Ok(())
}
```

## Pair `picoquic/sockloop.c:monitor_system_call_duration`
C: `picoquic/sockloop.c:1102-1128 monitor_system_call_duration`
Rust: `rs/fq/src/packet_loop.rs:592-619 monitor_system_call_duration`

### C body
```c
{
    uint64_t duration = current_time - previous_time;
    int64_t dev = sc_duration->scd_smoothed - duration;
    int shall_notify = 0;

    if (duration > sc_duration->scd_max) {
        shall_notify = 1;
        sc_duration->scd_max = duration;
    }
    else if (duration != sc_duration->scd_last) {
        int64_t delta_d = sc_duration->scd_last - duration;

        if (delta_d > 1000 || delta_d < -1000 || delta_d < (int64_t)sc_duration->scd_last) {
            shall_notify = 1;
        }
        sc_duration->scd_last = duration;
    }

    sc_duration->scd_smoothed = (duration + 15 * sc_duration->scd_smoothed) / 16;
    if (dev < 0) {
        dev = -dev;
    }
    sc_duration->scd_dev = (7 * sc_duration->scd_dev + dev) / 8;

    return shall_notify;
}
```

### Rust body
```rust
) -> bool {
    let duration = current_time.ticks().saturating_sub(previous_time.ticks());
    let mut dev = sc_duration.scd_smoothed as i64 - duration as i64;
    let mut shall_notify = false;

    if duration > sc_duration.scd_max {
        shall_notify = true;
        sc_duration.scd_max = duration;
    } else if duration != sc_duration.scd_last {
        let delta_d = sc_duration.scd_last as i64 - duration as i64;
        if !(-1000..=1000).contains(&delta_d) || delta_d < sc_duration.scd_last as i64 {
            shall_notify = true;
        }
        sc_duration.scd_last = duration;
    }

    sc_duration.scd_smoothed = (duration + 15 * sc_duration.scd_smoothed) / 16;
    if dev < 0 {
        dev = -dev;
    }
    sc_duration.scd_dev = (7 * sc_duration.scd_dev + dev as u64) / 8;

    shall_notify
}
```

## Pair `picoquic/sockloop.c:picoquic_close_network_wake_up`
C: `picoquic/sockloop.c:1691-1703 picoquic_close_network_wake_up`
Rust: `rs/fq/src/packet_loop.rs:693-699 close_network_wake_up`

### C body
```c
{
    if (thread_ctx->wake_up_defined) {
#ifdef _WINDOWS
        CloseHandle(thread_ctx->wake_up_event);
#else
        for (int i = 0; i < 2; i++) {
            (void)close(thread_ctx->wake_up_pipe_fd[i]);
        }
#endif
        thread_ctx->wake_up_defined = 0;
    }
}
```

### Rust body
```rust
fn close_network_wake_up(thread_ctx: &mut NetworkThreadCtx) {
    if thread_ctx.wake_up_defined {
        thread_ctx.wake_up_sender = None;
        thread_ctx.wake_up_receiver = None;
        thread_ctx.wake_up_defined = false;
    }
}
```

## Pair `picoquic/sockloop.c:picoquic_internal_thread_delete`
C: `picoquic/sockloop.c:1763-1766 picoquic_internal_thread_delete`
Rust: `rs/fq/src/packet_loop.rs:1475-1477 internal_thread_delete`

### C body
```c
{
    picoquic_delete_thread((picoquic_thread_t *)v_thread_id);
}
```

### Rust body
```rust
pub fn internal_thread_delete(thread: JoinHandle<()>) {
    let _ = thread.join();
}
```

## Pair `picoquic/sockloop.c:picoquic_server_set_context`
C: `picoquic/sockloop.c:1882-1933 picoquic_server_set_context`
Rust: `rs/fq/src/packet_loop.rs:1508-1536 create_server`

### C body
```c
{
    int ret = 0;
    /* Create QUIC context */

    if (ret == 0) {
        *qserver = picoquic_create_and_configure(config, default_callback_fn, default_callback_ctx, current_time, NULL);
        if (*qserver == NULL) {
            ret = -1;
        }
        else {
            picoquic_set_key_log_file_from_env(*qserver);

            picoquic_set_alpn_select_fn_v2(*qserver, alpn_select_fn);

            picoquic_use_unique_log_names(*qserver, 1);

            if (config->qlog_dir != NULL)
            {
                picoquic_set_qlog(*qserver, config->qlog_dir);
            }
            if (config->performance_log != NULL)
            {
                ret = picoquic_perflog_setup(*qserver, config->performance_log);
            }
            if (ret == 0 && config->cnx_id_cbdata != NULL) {
                picoquic_load_balancer_config_t lb_config;
                ret = picoquic_lb_compat_cid_config_parse(&lb_config, config->cnx_id_cbdata, strlen(config->cnx_id_cbdata));
                if (ret != 0) {
                    fprintf(stdout, "Cannot parse the CNX_ID config policy: %s.\n", config->cnx_id_cbdata);
                }
                else {
                    ret = picoquic_lb_compat_cid_config(*qserver, &lb_config);
                    if (ret != 0) {
                        fprintf(stdout, "Cannot set the CNX_ID config policy: %s.\n", config->cnx_id_cbdata);
                    }
                }
            }
            if (ret == 0) {
                fprintf(stdout, "Accept enable multipath: %d.\n", (*qserver)->default_multipath_option);
            }
            (*qserver)->default_tp.is_reset_stream_at_enabled = 1;
            (*qserver)->default_tp.max_datagram_frame_size = PICOQUIC_MAX_PACKET_SIZE;
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<Box<Quic>, Error> {
        let mut qserver = config
            .create_and_configure(default_callback, current_time, None)
            .ok_or(Error::Generic)?;

        qserver.set_key_log_file_from_env();
        qserver.set_alpn_select_fn(alpn_select_fn);
        qserver.set_use_unique_log_names(true);

        if let Some(qlog_dir) = config.qlog_dir.as_deref() {
            qserver.set_qlog(qlog_dir)?;
        }
        if let Some(performance_log) = config.performance_log.as_deref() {
            qserver.perflog_setup(performance_log)?;
        }
        if let Some(cnx_id_cbdata) = config.connection_id_cbdata.as_deref() {
            let lb_config = crate::lb::Config::parse(cnx_id_cbdata)?;
            qserver.set_lb_cid_config(&lb_config)?;
        }

        qserver.default_tp.is_reset_stream_at_enabled = true;
        qserver.default_tp.max_datagram_frame_size = crate::MAX_PACKET_SIZE as u32;
        Ok(qserver)
    }
```

## Pair `picoquic/spinbit.c:picoquic_spinbit_basic_outgoing`
C: `picoquic/spinbit.c:37-42 picoquic_spinbit_basic_outgoing`
Rust: `rs/fq/src/spinbit.rs:30-37 outgoing`

### C body
```c
{
    uint8_t spin_bit = (uint8_t)((cnx->path[0]->current_spin) << 5);

    return spin_bit;
}
```

### Rust body
```rust
    fn outgoing(&self, connection: &mut Connection) -> u8 {
        let spin = connection
            .paths
            .first()
            .map(|p| p.current_spin)
            .unwrap_or(false);
        (spin as u8) << 5
    }
```

## Pair `picoquic/spinbit.c:picoquic_spinbit_random_outgoing`
C: `picoquic/spinbit.c:72-76 picoquic_spinbit_random_outgoing`
Rust: `rs/fq/src/spinbit.rs:81-83 picoquic_spinbit_random_outgoing`

### C body
```c
{
    UNREFERENCED_PARAMETER(cnx);
    return (uint8_t) (picoquic_public_random_64()&0x20);
}
```

### Rust body
```rust
pub fn picoquic_spinbit_random_outgoing(connection: &mut Connection) -> u8 {
    RandomSpinBit.outgoing(connection)
}
```
