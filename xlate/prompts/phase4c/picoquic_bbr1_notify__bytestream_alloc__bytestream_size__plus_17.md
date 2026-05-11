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

## Pair `picoquic/bbr1.c:picoquic_bbr1_notify`
C: `picoquic/bbr1.c:1191-1319 picoquic_bbr1_notify`
Rust: `rs/fq/src/bbr1.rs:1203-1336 notify`

### C body
```c
{
    picoquic_bbr1_state_t* bbr1_state = (picoquic_bbr1_state_t*)path_x->congestion_alg_state;
    path_x->is_cc_data_updated = 1;

    if (bbr1_state != NULL) {
        switch (notification) {
        case picoquic_congestion_notification_ecn_ec:
            /* Non standard code to react on ECN_EC */
            if (ack_state->lost_packet_number >= bbr1_state->congestion_sequence) {
                picoquic_bbr1_notify_congestion(bbr1_state, cnx, path_x, current_time, 0);
            }
            break;
        case picoquic_congestion_notification_repeat:
        case picoquic_congestion_notification_timeout:
            /* Non standard code to react to high rate of packet loss, or timeout loss */
            if (ack_state->lost_packet_number >= bbr1_state->congestion_sequence &&
                picoquic_cc_hystart_loss_test(&bbr1_state->rtt_filter, notification, ack_state->lost_packet_number, 0.20)) {
                picoquic_bbr1_notify_congestion(bbr1_state, cnx, path_x, current_time,
                    (notification == picoquic_congestion_notification_timeout) ? 1 : 0);
            }
            break;
        case picoquic_congestion_notification_spurious_repeat:
            if (bbr1_state->is_suspended) {
                picoquic_bbr1_suspension_almost_over(bbr1_state, ack_state->lost_packet_number);
            }
            break;
        case picoquic_congestion_notification_acknowledgement:
            /* sum the amount of data acked per packet */
            if (bbr1_state->is_suspended) {
                picoquic_bbr1_suspension_exit(bbr1_state, path_x);
            }
            bbr1_state->bytes_delivered += ack_state->nb_bytes_acknowledged;

            if (bbr1_state->state == picoquic_bbr1_alg_startup && path_x->rtt_min > BBR1_HYSTART_THRESHOLD_RTT) {
                BBR1EnterStartupLongRTT(bbr1_state, path_x);
            }

            if (bbr1_state->state == picoquic_bbr1_alg_startup_long_rtt) {
                if (picoquic_cc_hystart_test(&bbr1_state->rtt_filter, (cnx->is_time_stamp_enabled) ? ack_state->one_way_delay : ack_state->rtt_measurement,
                    cnx->path[0]->pacing.packet_time_microsec, current_time, cnx->is_time_stamp_enabled)) {
                    BBR1ExitStartupLongRtt(bbr1_state, path_x, current_time);
                }
            }

            /* RTT measurements will happen after the bandwidth is estimated */
            if (bbr1_state->state == picoquic_bbr1_alg_startup_long_rtt) {
                uint64_t max_win;
                uint64_t min_win;

                BBR1UpdateBtlBw(bbr1_state, path_x, current_time);
                if (ack_state->rtt_measurement <= bbr1_state->rt_prop) {
                    bbr1_state->rt_prop = ack_state->rtt_measurement;
                    bbr1_state->rt_prop_stamp = current_time;
                }
                if (path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                    path_x->cwin += picoquic_cc_slow_start_increase(path_x, bbr1_state->bytes_delivered);
                }
                bbr1_state->bytes_delivered = 0;

                max_win = PICOQUIC_BYTES_FROM_RATE(bbr1_state->rt_prop, path_x->peak_bandwidth_estimate);
                min_win = max_win /= 2;

                if (path_x->cwin < min_win) {
                    path_x->cwin = min_win;
                }
                else if (path_x->smoothed_rtt > PICOQUIC_TARGET_RENO_RTT) {
                    path_x->pacing.bandwidth_pause = 1;
                }

                picoquic_update_pacing_data(path_x, 1);
            } else {
                BBR1UpdateOnACK(bbr1_state, path_x,
                    ack_state->rtt_measurement, path_x->bytes_in_transit, 0 /* packets_lost */, bbr1_state->bytes_delivered,
                    current_time);
                /* Remember the number in flight before the next ACK -- TODO: update after send instead. */
                bbr1_state->prior_in_flight = path_x->bytes_in_transit;
                /* Reset the number of bytes delivered */
                bbr1_state->bytes_delivered = 0;

                if (bbr1_state->pacing_rate > 0) {
                    /* Set the pacing rate in picoquic sender */
                    picoquic_update_pacing_rate(path_x, bbr1_state->pacing_rate, bbr1_state->send_quantum);
                }
            }
            break;
        case picoquic_congestion_notification_cwin_blocked:
            break;
        case picoquic_congestion_notification_reset:
            picoquic_bbr1_reset(bbr1_state, path_x, current_time);
            break;
        case picoquic_congestion_notification_seed_cwin:
            if (bbr1_state->state == picoquic_bbr1_alg_startup_long_rtt) {
                BBR1ExitStartupSeedBDP(bbr1_state, path_x, ack_state->nb_bytes_acknowledged, current_time);
                picoquic_update_pacing_data(path_x, 1);
            }
            else if (bbr1_state->state == picoquic_bbr1_alg_startup){
                /* If in initial startup phase, do something */
                double seed_bw_estimate = (double)ack_state->nb_bytes_acknowledged;
                uint64_t bwe;
                seed_bw_estimate /= (double)path_x->smoothed_rtt;
                seed_bw_estimate *= 1000000;
                bwe = (uint64_t)seed_bw_estimate*2; /* Hack -- account for div by two in BBR1UpdateBtlBw */
                if (path_x->bandwidth_estimate_max < bwe) {
                    path_x->bandwidth_estimate_max = bwe;
                    BBR1UpdateBtlBw(bbr1_state, path_x, current_time);
                    BBR1SetPacingRate(bbr1_state);
                    if (bbr1_state->pacing_rate > 0) {
                        /* Set the pacing rate in picoquic sender */
                        picoquic_update_pacing_rate(path_x, bbr1_state->pacing_rate, bbr1_state->send_quantum);
                    }
                }
            }
            break;
        default:
            /* ignore */
            break;
        }
    }
}
```

### Rust body
```rust
    ) {
        path_x.is_cc_data_updated = true;

        match notification {
            CongestionNotification::EcnEc
                if ack_state.lost_packet_number >= self.congestion_sequence =>
            {
                self.notify_congestion(cnx, path_x, current_time, false);
            }
            CongestionNotification::Repeat | CongestionNotification::Timeout
                if ack_state.lost_packet_number >= self.congestion_sequence
                    && self.rtt_filter.hystart_loss_test(
                        notification,
                        ack_state.lost_packet_number,
                        0.20,
                    ) =>
            {
                let is_timeout = notification == CongestionNotification::Timeout;
                self.notify_congestion(cnx, path_x, current_time, is_timeout);
            }
            CongestionNotification::SpuriousRepeat if self.is_suspended => {
                self.suspension_almost_over(ack_state.lost_packet_number);
            }
            CongestionNotification::Acknowledgement => {
                if self.is_suspended {
                    self.suspension_exit(path_x);
                }
                self.bytes_delivered += ack_state.nb_bytes_acknowledged;

                if self.state == Bbr1AlgState::Startup
                    && path_x.rtt_min.ticks() > BBR1_HYSTART_THRESHOLD_RTT
                {
                    self.enter_startup_long_rtt(path_x);
                }

                if self.state == Bbr1AlgState::StartupLongRtt {
                    let rtt_meas = if cnx.is_time_stamp_enabled {
                        ack_state.one_way_delay
                    } else {
                        ack_state.rtt_measurement
                    };
                    let packet_time =
                        Instant::from_ticks(path_x.pacing.packet_time_microsec.ticks());
                    if self.rtt_filter.hystart_test(
                        rtt_meas,
                        packet_time,
                        Instant::from_ticks(current_time),
                        cnx.is_time_stamp_enabled,
                    ) {
                        self.exit_startup_long_rtt(path_x, current_time);
                    }
                }

                if self.state == Bbr1AlgState::StartupLongRtt {
                    self.update_btl_bw(path_x, current_time);
                    if ack_state.rtt_measurement.ticks() <= self.rt_prop {
                        self.rt_prop = ack_state.rtt_measurement.ticks();
                        self.rt_prop_stamp = current_time;
                    }
                    if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
                        let increase = path_x.slow_start_increase(self.bytes_delivered);
                        path_x.cwin += increase;
                    }
                    self.bytes_delivered = 0;

                    let max_win = bytes_from_rate(self.rt_prop, path_x.peak_bandwidth_estimate);
                    let min_win = max_win / 2;

                    if path_x.cwin < min_win {
                        path_x.cwin = min_win;
                    } else if path_x.smoothed_rtt > TARGET_RENO_RTT {
                        path_x.pacing.bandwidth_pause = 1;
                    }
                    path_x.update_pacing_data(1);
                } else {
                    let rtt_sample = ack_state.rtt_measurement.ticks();
                    let bytes_in_transit = path_x.bytes_in_transit;
                    let bytes_delivered = self.bytes_delivered;
                    self.update_on_ack(
                        path_x,
                        rtt_sample,
                        bytes_in_transit,
                        0,
                        bytes_delivered,
                        current_time,
                    );
                    self.prior_in_flight = path_x.bytes_in_transit;
                    self.bytes_delivered = 0;
                    if self.pacing_rate > 0.0 {
                        path_x.update_pacing_rate(self.pacing_rate, self.send_quantum);
                    }
                }
            }
            CongestionNotification::CwinBlocked => {}
            CongestionNotification::Reset => {
                self.reset(path_x, current_time);
            }
            CongestionNotification::SeedCwin => {
                if self.state == Bbr1AlgState::StartupLongRtt {
                    self.exit_startup_seed_bdp(
                        path_x,
                        ack_state.nb_bytes_acknowledged,
                        current_time,
                    );
                    path_x.update_pacing_data(1);
                } else if self.state == Bbr1AlgState::Startup {
                    // Estimate bandwidth from the seeded BDP; account for the
                    // div-by-two inside BBR1UpdateBtlBw with *2 (C comment: "Hack").
                    let smoothed_rtt_us = path_x.smoothed_rtt.ticks() as f64;
                    if smoothed_rtt_us > 0.0 {
                        let seed_bw =
                            ack_state.nb_bytes_acknowledged as f64 / smoothed_rtt_us * 1_000_000.0;
                        let bwe = (seed_bw as u64) * 2;
                        if path_x.bandwidth_estimate_max < bwe {
                            path_x.bandwidth_estimate_max = bwe;
                            self.update_btl_bw(path_x, current_time);
                            self.set_pacing_rate();
                            if self.pacing_rate > 0.0 {
                                path_x.update_pacing_rate(self.pacing_rate, self.send_quantum);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
```

## Pair `picoquic/bytestream.c:bytestream_alloc`
C: `picoquic/bytestream.c:52-63 bytestream_alloc`
Rust: `rs/fq/src/bytestream.rs:624-632 with_capacity`

### C body
```c
{
    s->data = (uint8_t*)malloc(nb_bytes);
    if (s->data == NULL) {
        free(s);
        return NULL;
    }
    s->size = nb_bytes;
    s->ptr = 0;

    return s;
}
```

### Rust body
```rust
    pub fn with_capacity(nb_bytes: usize) -> Option<Self> {
        let mut v = Vec::new();
        v.try_reserve_exact(nb_bytes).ok()?;
        v.resize(nb_bytes, 0);
        Some(ByteStream {
            data: ByteStreamData::Owned(v),
            ptr: 0,
        })
    }
```

## Pair `picoquic/bytestream.c:bytestream_size`
C: `picoquic/bytestream.c:83-86 bytestream_size`
Rust: `rs/fq/src/internal.rs:8653-8676 size`

### C body
```c
{
    return s->size;
}
```

### Rust body
```rust
impl SackItem {
    /// Inclusive start of this SACK range.  C:
    /// `sack_item_range_start`.
    pub fn range_start(&self) -> u64 {
        self.start_of_sack_range
    }

    /// Exclusive end of this SACK range.  C:
    /// `sack_item_range_end`.
    pub fn range_end(&self) -> u64 {
        self.end_of_sack_range
    }

    /// Number of times this range has been sent in an ACK frame.
    /// C: `sack_item_nb_times_sent`.
    pub fn nb_times_sent(&self, is_opportunistic: i32) -> i32 {
        self.nb_times_sent[is_opportunistic.clamp(0, 1) as usize]
    }
}
```

## Pair `picoquic/bytestream.c:bytestream_clear`
C: `picoquic/bytestream.c:103-107 bytestream_clear`
Rust: `rs/fq/src/arena.rs:208-212 clear`

### C body
```c
{
    s->ptr = 0;
    memset(s->data, 0, s->size);
}
```

### Rust body
```rust
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free = None;
        self.len = 0;
    }
```

## Pair `picoquic/bytestream.c:bytewrite_vint`
C: `picoquic/bytestream.c:136-145 bytewrite_vint`
Rust: `rs/fq/src/bytestream.rs:366-399 write_varint`

### C body
```c
{
    size_t len = picoquic_varint_encode(s->data + s->ptr, s->size - s->ptr, value);
    if (len == 0) {
        return bytestream_error(s);
    } else {
        s->ptr += len;
        return 0;
    }
}
```

### Rust body
```rust
    pub fn write_varint(&mut self, value: u64) -> Result<(), Error> {
        let len = Self::varint_encoded_len(value);
        if self.data_ref().len() - self.ptr < len {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        let d = self.data_mut();
        match len {
            1 => d[ptr] = value as u8,
            2 => {
                d[ptr] = 0x40 | (value >> 8) as u8;
                d[ptr + 1] = value as u8;
            }
            4 => {
                d[ptr] = 0x80 | (value >> 24) as u8;
                d[ptr + 1] = (value >> 16) as u8;
                d[ptr + 2] = (value >> 8) as u8;
                d[ptr + 3] = value as u8;
            }
            _ => {
                d[ptr] = 0xC0 | (value >> 56) as u8;
                d[ptr + 1] = (value >> 48) as u8;
                d[ptr + 2] = (value >> 40) as u8;
                d[ptr + 3] = (value >> 32) as u8;
                d[ptr + 4] = (value >> 24) as u8;
                d[ptr + 5] = (value >> 16) as u8;
                d[ptr + 6] = (value >> 8) as u8;
                d[ptr + 7] = value as u8;
            }
        }
        self.ptr += len;
        Ok(())
    }
```

## Pair `picoquic/bytestream.c:bytewrite_int8`
C: `picoquic/bytestream.c:177-186 bytewrite_int8`
Rust: `rs/fq/src/bytestream.rs:246-255 write_u8`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 1) {
        return bytestream_error(s);
    } else {
        s->data[s->ptr++] = value;
        return 0;
    }
}
```

### Rust body
```rust
    pub fn write_u8(&mut self, value: u8) -> Result<(), Error> {
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr] = value;
        self.ptr += 1;
        Ok(())
    }
```

## Pair `picoquic/bytestream.c:byteread_int16`
C: `picoquic/bytestream.c:222-234 byteread_int16`
Rust: `rs/fq/src/bytestream.rs:293-297 read_u16`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 2) {
        return bytestream_error(s);
    }
    else {
        const uint8_t * ptr = s->data + s->ptr;
        *value = (ptr[0] << 8) | ptr[1];
        s->ptr += 2;
        return 0;
    }
}
```

### Rust body
```rust
        if self.data_ref().len() - self.ptr < 2 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
```

## Pair `picoquic/bytestream.c:byteread_int64`
C: `picoquic/bytestream.c:274-289 byteread_int64`
Rust: `rs/fq/src/bytestream.rs:343-347 read_u64`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 8) {
        return bytestream_error(s);
    }
    else {
        uint64_t v = 0;
        for (size_t i = 0; i < 8; i++) {
            v <<= 8;
            v += s->data[s->ptr++];
        }
        *value = v;
        return 0;
    }
}
```

### Rust body
```rust
        if self.data_ref().len() - self.ptr < 8 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
```

## Pair `picoquic/bytestream.c:byteread_cid`
C: `picoquic/bytestream.c:324-336 byteread_cid`
Rust: `rs/fq/src/bytestream.rs:502-514 read_cid`

### C body
```c
{
    int ret = byteread_int8(s, &cid->id_len);

    if (cid->id_len > PICOQUIC_CONNECTION_ID_MAX_SIZE) {
        ret = -1;
    } else {
        memset(cid->id, 0, sizeof(cid->id));
        ret |= byteread_buffer(s, cid->id, cid->id_len);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn read_cid(&mut self) -> Result<ConnectionId, Error> {
        let id_len = self.read_u8()?;
        if id_len as usize > crate::CONNECTION_ID_MAX_SIZE {
            self.set_error();
            return Err(Error::InvalidArgument);
        }
        let mut cid = ConnectionId {
            id_len,
            ..ConnectionId::default()
        };
        self.read_bytes(&mut cid.id[..id_len as usize])?;
        Ok(cid)
    }
```

## Pair `picoquic/bytestream.c:byteskip_cstr`
C: `picoquic/bytestream.c:371-384 byteskip_cstr`
Rust: `rs/fq/src/bytestream.rs:554-562 skip_str`

### C body
```c
{
    uint64_t l_read = 0;
    int ret = byteread_vint(s, &l_read);

    size_t l_cstr = (size_t)l_read;

    if (ret != 0 || l_cstr != l_read) {
        ret = -1;
    } else {
        ret = bytestream_skip(s, l_cstr);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn skip_str(&mut self) -> Result<(), Error> {
        let l_read = self.read_varint()?;
        let l = l_read as usize;
        if (l_read as usize as u64) != l_read {
            self.set_error();
            return Err(Error::InvalidArgument);
        }
        self.skip(l)
    }
```

## Pair `picoquic/bytestream.c:bytestream_error`
C: `picoquic/bytestream.c:433-437 bytestream_error`
Rust: `rs/fq/src/bytestream.rs:150-153 set_error`

### C body
```c
{
    s->ptr = s->size;
    return -1;
}
```

### Rust body
```rust
    fn set_error(&mut self) {
        let cap = self.data_ref().len();
        self.ptr = cap;
    }
```

## Pair `picoquic/c4.c:c4_ecn_threshold`
C: `picoquic/c4.c:277-288 c4_ecn_threshold`
Rust: `rs/fq/src/c4.rs:288-291 ecn_threshold`

### C body
```c
{
    uint64_t sensitivity = c4_sensitivity_1024(c4_state);

    uint64_t ecn_threshold = 192 - MULT1024(sensitivity, 96);

    return ecn_threshold;
}
```

### Rust body
```rust
    pub fn ecn_threshold(&self) -> u64 {
        let sensitivity = self.sensitivity_1024();
        192 - mult1024(sensitivity, 96)
    }
```

## Pair `picoquic/c4.c:c4_apply_rate_and_cwin`
C: `picoquic/c4.c:359-424 c4_apply_rate_and_cwin`
Rust: `rs/fq/src/c4.rs:490-544 apply_rate_and_cwin`

### C body
```c
{

    uint64_t pacing_rate = MULT1024(c4_state->alpha_1024_current, c4_state->nominal_rate);
    uint64_t quantum;
    uint64_t target_cwin = PICOQUIC_CWIN_INITIAL;
    if (c4_state->nominal_max_rtt != 0 && c4_state->nominal_rate != 0) {
        target_cwin = PICOQUIC_BYTES_FROM_RATE(c4_state->nominal_max_rtt, pacing_rate);
    }

    if (c4_state->alg_state == c4_initial) {
        if (target_cwin < c4_state->initial_cwnd) {
            target_cwin = c4_state->initial_cwnd;
        }
        /* Initial special case: bandwidth discovery */
        if (c4_state->nb_packets_in_startup > 0) {
            if (path_x->peak_bandwidth_estimate > pacing_rate) {
                uint64_t min_win;
                pacing_rate = (pacing_rate + path_x->peak_bandwidth_estimate) / 2;
                min_win = PICOQUIC_BYTES_FROM_RATE(path_x->smoothed_rtt, path_x->peak_bandwidth_estimate) / 2;
                if (min_win > target_cwin) {
                    target_cwin = min_win;
                }
            }
        }
        /* Initial special case: seed cwin */
        if (c4_state->use_seed_cwin && c4_state->seed_cwin > target_cwin) {
            /* Match half the difference between seed and computed CWIN */
            target_cwin = (c4_state->seed_cwin + target_cwin) / 2;
            c4_state->seed_rate = PICOQUIC_RATE_FROM_BYTES(c4_state->seed_cwin, path_x->smoothed_rtt);
            if (c4_state->seed_rate > pacing_rate) {
                pacing_rate = c4_state->seed_rate;
            }
        }
        c4_state->initial_cwnd = target_cwin;
    }
    else {
        uint64_t delta_rtt_target = C4_RTT_MARGIN_DELAY;
        if (c4_state->nominal_max_rtt < 4* C4_RTT_MARGIN_DELAY) {
            delta_rtt_target = c4_state->nominal_max_rtt / 4;
        }
        target_cwin += PICOQUIC_BYTES_FROM_RATE(delta_rtt_target, pacing_rate);

        if (c4_state->alg_state == c4_pushing) {
            uint64_t delta_alpha = c4_state->alpha_1024_current - 1024;
            uint64_t delta_rate = MULT1024(delta_alpha, c4_state->nominal_rate);
            uint64_t delta_cwin = PICOQUIC_BYTES_FROM_RATE(c4_state->nominal_max_rtt, delta_rate);
            if (delta_cwin < path_x->send_mtu) {
                target_cwin += path_x->send_mtu - delta_cwin;
            }
        }
    }

    path_x->cwin = target_cwin;
    /* set the quantum to 4 milliseconds (OK, 4/1.024 ms, for simplicity) */
    quantum = MULT1024(4, pacing_rate);
    if (quantum > 0x10000) {
        quantum = 0x10000;
    }
    else if (quantum < 2 * path_x->send_mtu) {
        quantum = 2 * path_x->send_mtu;
    }
    picoquic_update_pacing_rate(path_x, (double)pacing_rate, quantum);
}
```

### Rust body
```rust
    fn apply_rate_and_cwin(&mut self, path_x: &mut Path) {
        let mut pacing_rate = mult1024(self.alpha_1024_current, self.nominal_rate);
        let mut target_cwin = CWIN_INITIAL;
        if self.nominal_max_rtt != 0 && self.nominal_rate != 0 {
            target_cwin = bytes_from_rate(self.nominal_max_rtt, pacing_rate);
        }

        if self.alg_state == C4AlgState::Initial {
            if target_cwin < self.initial_cwnd {
                target_cwin = self.initial_cwnd;
            }
            if self.nb_packets_in_startup > 0 && path_x.peak_bandwidth_estimate > pacing_rate {
                pacing_rate = (pacing_rate + path_x.peak_bandwidth_estimate) / 2;
                let min_win =
                    bytes_from_rate(path_x.smoothed_rtt.ticks(), path_x.peak_bandwidth_estimate)
                        / 2;
                if min_win > target_cwin {
                    target_cwin = min_win;
                }
            }
            if self.use_seed_cwin && self.seed_cwin > target_cwin {
                target_cwin = (self.seed_cwin + target_cwin) / 2;
                self.seed_rate = rate_from_bytes(self.seed_cwin, path_x.smoothed_rtt.ticks());
                if self.seed_rate > pacing_rate {
                    pacing_rate = self.seed_rate;
                }
            }
            self.initial_cwnd = target_cwin;
        } else {
            let delta_rtt_target = if self.nominal_max_rtt < 4 * C4_RTT_MARGIN_DELAY {
                self.nominal_max_rtt / 4
            } else {
                C4_RTT_MARGIN_DELAY
            };
            target_cwin += bytes_from_rate(delta_rtt_target, pacing_rate);

            if self.alg_state == C4AlgState::Pushing {
                let delta_alpha = self.alpha_1024_current.saturating_sub(1024);
                let delta_rate = mult1024(delta_alpha, self.nominal_rate);
                let delta_cwin = bytes_from_rate(self.nominal_max_rtt, delta_rate);
                if delta_cwin < path_x.send_mtu as u64 {
                    target_cwin += path_x.send_mtu as u64 - delta_cwin;
                }
            }
        }

        path_x.cwin = target_cwin;
        let mut quantum = mult1024(4, pacing_rate);
        if quantum > 0x10000 {
            quantum = 0x10000;
        } else if quantum < 2 * path_x.send_mtu as u64 {
            quantum = 2 * path_x.send_mtu as u64;
        }
        path_x.update_pacing_rate(pacing_rate as f64, quantum);
    }
```

## Pair `picoquic/c4.c:c4_era_reset`
C: `picoquic/c4.c:475-484 c4_era_reset`
Rust: `rs/fq/src/c4.rs:313-319 era_reset`

### C body
```c
{
    c4_state->era_sequence = picoquic_cc_get_sequence_number(path_x->cnx, path_x);
    c4_state->era_max_rtt = 0;
    c4_state->era_min_rtt = UINT64_MAX;
    c4_state->alpha_1024_previous = c4_state->alpha_1024_current;
    c4_update_ecn_alpha(path_x, c4_state);
}
```

### Rust body
```rust
    fn era_reset(&mut self, path_x: &Path, connection: &Connection) {
        self.era_sequence = connection.sequence_number(path_x);
        self.era_max_rtt = 0;
        self.era_min_rtt = u64::MAX;
        self.alpha_1024_previous = self.alpha_1024_current;
        self.update_ecn_alpha(path_x, connection);
    }
```

## Pair `picoquic/c4.c:c4_seed_cwin`
C: `picoquic/c4.c:527-533 c4_seed_cwin`
Rust: `rs/fq/src/c4.rs:183-197 seed_cwin`

### C body
```c
{
    if (c4_state->alg_state == c4_initial) {
        c4_state->use_seed_cwin = 1;
        c4_state->seed_cwin = bytes_in_flight;
    }
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        (self.alg_state as u64, self.nominal_max_rtt)
    }
```

## Pair `picoquic/c4.c:c4_initial_handle_ack`
C: `picoquic/c4.c:576-623 c4_initial_handle_ack`
Rust: `rs/fq/src/c4.rs:455-481 initial_handle_ack`

### C body
```c
{
    c4_state->nb_packets_in_startup += 1;
    /* We implement Reno style slow start, doubling the CWND every RTT, by
    * incrementing CWND by the number of new bytes acknowledged.
    * However, this is too aggressive at the end of the initial phase.
    * The exit test is "3 successive RTT with no data rate increase".
    * If the CWND keeps increasing after noticing the first increase,
    * it grows too much and the queues build up too much. But then, the first
    * notice can also be due to a jitter event, which causes rate measurment to be low,
    * in which case we need  to increase the CWND to eventually catch with a new RTT.
    * The 'shift' in the formula is a compromise, kinda similar to "Hystart++",
    * causing the CWND to increase slower if we may be close to the exit.
    */
    c4_state->initial_cwnd += ack_state->nb_bytes_acknowledged >> (3 * c4_state->nb_eras_no_increase);
    if (c4_state->use_seed_cwin && c4_state->seed_rate > 0 &&
        c4_state->nominal_rate >= c4_state->seed_rate) {
        /* The nominal bandwidth is larger than the seed. The seed has been validated. */
        c4_state->use_seed_cwin = 0;
    }
    if (c4_era_check(path_x, c4_state)) {
        /*
        * We should only consider a lack of increase if the application is
        * not app limited. However, if the application *is* app limited,
        * that strategy leads to staying in "initial" mode forever,
        * which is not good either. If we don't check if the app limited,
        * we lose in the very common case where the server sends almost
        * nothing for several RTT, until the client asks for some data.
        * So we test that we have seen at least some data.
        */
        int is_growing = c4_growth_evaluate(c4_state);
        if (is_growing) {
            c4_state->nb_eras_no_increase = 0;
        }
        else if (c4_state->push_was_not_limited && c4_state->nominal_rate > 0) {
            c4_state->nb_eras_no_increase++;
        }
        
        c4_era_reset(path_x, c4_state);
        if (c4_state->nb_eras_no_increase >= 3) {
            c4_exit_initial(path_x, c4_state);
            return;
        }
        else {
            c4_growth_reset(c4_state);
        }
    }
}
```

### Rust body
```rust
    ) {
        self.nb_packets_in_startup += 1;
        self.initial_cwnd +=
            ack_state.nb_bytes_acknowledged >> (3 * self.nb_eras_no_increase as u32);
        if self.use_seed_cwin && self.seed_rate > 0 && self.nominal_rate >= self.seed_rate {
            self.use_seed_cwin = false;
        }
        if self.era_check(path_x, connection) {
            let is_growing = self.growth_evaluate();
            if is_growing {
                self.nb_eras_no_increase = 0;
            } else if self.push_was_not_limited && self.nominal_rate > 0 {
                self.nb_eras_no_increase += 1;
            }
            self.era_reset(path_x, connection);
            if self.nb_eras_no_increase >= 3 {
                self.exit_initial(path_x, connection);
            } else {
                self.growth_reset();
            }
        }
    }
```

## Pair `picoquic/c4.c:c4_enter_cruise`
C: `picoquic/c4.c:711-742 c4_enter_cruise`
Rust: `rs/fq/src/c4.rs:374-393 enter_cruise`

### C body
```c
{
    c4_era_reset(path_x, c4_state);
    c4_state->use_seed_cwin = 0;

    if (c4_state->probe_level > C4_PROBE_LEVEL_DEFAULT) {
        c4_state->nb_cruise_left_before_push = 0;
    }
    else {
        if (c4_state->nb_cruise_left_before_push == 0) {
            c4_state->nb_cruise_left_before_push = (c4_state->probe_level == 0) ? 1 : C4_NB_CRUISE_BEFORE_PUSH;
        }     
    }
    c4_state->alpha_1024_current = C4_ALPHA_CRUISE_1024;
    if (path_x->smoothed_rtt < C4_MAX_RTT_MIN) {
        /* When operating in a CPU limited environment, pacing is too
         * conservative, and should be loosened. Ideally, we would detect
         * the CPU limited condition by comparing pacing rate and the actual
         * send rate, but that requires some amount of testing. In practice,
         * we have only tested this loosening in loopback tests, so we only
         * apply it if the RTT is below 1ms.
         */
        c4_state->alpha_1024_current += 48;
    }
    c4_state->alg_state = c4_cruising;
}
```

### Rust body
```rust
    fn enter_cruise(&mut self, path_x: &mut Path, connection: &Connection) {
        self.era_reset(path_x, connection);
        self.use_seed_cwin = false;

        if self.probe_level > C4_PROBE_LEVEL_DEFAULT {
            self.nb_cruise_left_before_push = 0;
        } else if self.nb_cruise_left_before_push == 0 {
            self.nb_cruise_left_before_push = if self.probe_level == 0 {
                1
            } else {
                C4_NB_CRUISE_BEFORE_PUSH
            };
        }

        self.alpha_1024_current = C4_ALPHA_CRUISE_1024;
        if path_x.smoothed_rtt.ticks() < C4_MAX_RTT_MIN {
            self.alpha_1024_current += 48;
        }
        self.alg_state = C4AlgState::Cruising;
    }
```

## Pair `picoquic/c4.c:c4_notify_congestion`
C: `picoquic/c4.c:885-968 c4_notify_congestion`
Rust: `rs/fq/src/c4.rs:553-619 notify_congestion`

### C body
```c
{
    uint64_t beta = C4_BETA_LOSS_1024;
    c4_state->congestion_notified = 1;

    if (c_mode == c4_congestion_loss) {
        /* Make amount of slow down function of sensitivity,
        * for better fairness between C4 connections.
        */
        beta = (C4_BETA_LOSS_1024 + MULT1024(c4_sensitivity_1024(c4_state), C4_BETA_LOSS_1024))/2;
    }
    else if (c_mode == c4_congestion_ecn) {
        /* Apply proportional reduction.
         * The threshold value is a function of sensitivity, and thus the reduction
         * incorporates a sensitivity factor. There is no obvious need for
         * additional sensitivity related factors.
         */
        beta = (c4_state->ecn_alpha - c4_state->ecn_threshold) * 1024 / c4_state->ecn_threshold;
        if (beta > C4_BETA_LOSS_1024) {
            /* capping beta to the standard 1/4th. */
            beta = C4_BETA_LOSS_1024;
        }
    }
    
    if (c_mode == c4_congestion_delay) {
        /* TODO: we should really use bytes in flight! */
        beta = c4_state->recent_delay_excess*1024/c4_state->delay_threshold;

        if (beta > C4_BETA_LOSS_1024) {
            /* capping beta to the standard 1/4th. */
            beta = C4_BETA_LOSS_1024;
        }
    }
    else {
        /* Clear the excess delay to avoid spurious delay measurements */
        c4_state->recent_delay_excess = 0;
    }


    if (c4_state->alg_state == c4_recovery) {
        if (c4_state->alpha_1024_current == C4_ALPHA_RECOVER_1024) {
            /* Congestion notification after entering recovery 
             * indicates that queues are building up. It is thus
             * prudent to decrease "alpha_current" and to spend a bit
             * more time in recovery, to reduce these queues. */
            c4_state->alpha_1024_current = C4_ALPHA_RECOVER2_1024;
            c4_state->era_sequence = picoquic_cc_get_sequence_number(path_x->cnx, path_x);
            C4_LOGGER(path_x, 0, c4_state, NULL, beta, c_mode);
        }
        if (c_mode == c4_congestion_ecn) {
            c4_state->excess_ce_after_push = 1;
        }
    }
    else
    {
        if (c4_state->alg_state != c4_pushing) {
            c4_state->nominal_rate -= MULT1024(beta, c4_state->nominal_rate);
            if (c_mode == c4_congestion_loss) {
                c4_state->nominal_max_rtt -= MULT1024(beta, c4_state->nominal_max_rtt);
                if (c4_state->nominal_max_rtt < C4_MAX_RTT_MIN) {
                    c4_state->nominal_max_rtt = C4_MAX_RTT_MIN;
                }
                c4_state->delay_threshold = c4_delay_threshold(c4_state);
            }
            C4_LOGGER(path_x, 0, c4_state, NULL, beta, c_mode);
        }
        c4_enter_recovery(path_x, c4_state, c_mode);
    }

    c4_apply_rate_and_cwin(path_x, c4_state);

    path_x->is_ssthresh_initialized = 1;
}
```

### Rust body
```rust
    ) {
        let mut beta = C4_BETA_LOSS_1024;
        self.congestion_notified = true;

        if c_mode == C4Congestion::Loss {
            beta = (C4_BETA_LOSS_1024 + mult1024(self.sensitivity_1024(), C4_BETA_LOSS_1024)) / 2;
        } else if c_mode == C4Congestion::Ecn {
            if let Some(beta_ecn) = self
                .ecn_alpha
                .saturating_sub(self.ecn_threshold)
                .checked_mul(1024)
                .and_then(|v| v.checked_div(self.ecn_threshold))
            {
                beta = beta_ecn;
            }
            if beta > C4_BETA_LOSS_1024 {
                beta = C4_BETA_LOSS_1024;
            }
        }

        if c_mode == C4Congestion::Delay {
            if let Some(beta_delay) = self
                .recent_delay_excess
                .checked_mul(1024)
                .and_then(|v| v.checked_div(self.delay_threshold))
            {
                beta = beta_delay;
            }
            if beta > C4_BETA_LOSS_1024 {
                beta = C4_BETA_LOSS_1024;
            }
        } else {
            self.recent_delay_excess = 0;
        }

        if self.alg_state == C4AlgState::Recovery {
            if self.alpha_1024_current == C4_ALPHA_RECOVER_1024 {
                self.alpha_1024_current = C4_ALPHA_RECOVER2_1024;
                self.era_sequence = connection.sequence_number(path_x);
                c4_logger(path_x, 0, self, None, beta, c_mode);
            }
            if c_mode == C4Congestion::Ecn {
                self.excess_ce_after_push = true;
            }
        } else {
            if self.alg_state != C4AlgState::Pushing {
                self.nominal_rate -= mult1024(beta, self.nominal_rate);
                if c_mode == C4Congestion::Loss {
                    self.nominal_max_rtt -= mult1024(beta, self.nominal_max_rtt);
                    if self.nominal_max_rtt < C4_MAX_RTT_MIN {
                        self.nominal_max_rtt = C4_MAX_RTT_MIN;
                    }
                    self.delay_threshold = self.delay_threshold();
                }
                c4_logger(path_x, 0, self, None, beta, c_mode);
            }
            self.enter_recovery(path_x, connection, c_mode);
        }

        self.apply_rate_and_cwin(path_x);
        path_x.is_ssthresh_initialized = true;
    }
```

## Pair `picoquic/c4.c:c4_observe`
C: `picoquic/c4.c:1120-1126 c4_observe`
Rust: `rs/fq/src/c4.rs:195-197 observe`

### C body
```c
{
    c4_state_t* c4_state = (c4_state_t*)path_x->congestion_alg_state;
    *cc_state = (uint64_t)c4_state->alg_state;
    *cc_param = c4_state->nominal_max_rtt;
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        (self.alg_state as u64, self.nominal_max_rtt)
    }
```

## Pair `picoquic/cc_common.c:picoquic_cc_get_ack_sent_time`
C: `picoquic/cc_common.c:63-75 picoquic_cc_get_ack_sent_time`
Rust: `rs/fq/src/cc_common.rs:275-285 ack_sent_time`

### C body
```c
{
    uint64_t latest_time_acknowledged;

    if (cnx->is_multipath_enabled) {
        latest_time_acknowledged = path_x->pkt_ctx.latest_time_acknowledged;
    }
    else {
        latest_time_acknowledged = cnx->pkt_ctx[picoquic_packet_context_application].latest_time_acknowledged;
    }

    return latest_time_acknowledged;
}
```

### Rust body
```rust
    fn sequence_number(&self, path_x: &Path) -> u64 {
        if self.is_multipath_enabled {
            path_x.pkt_ctx.send_sequence
        } else {
            self.pkt_ctx[crate::PacketContext::Application as usize].send_sequence
        }
    }
```
