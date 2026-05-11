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

## Pair `picoquic/bbr1.c:picoquic_bbr1_observe`
C: `picoquic/bbr1.c:1323-1328 picoquic_bbr1_observe`
Rust: `rs/fq/src/bbr1.rs:419-421 observe`

### C body
```c
{
    picoquic_bbr1_state_t* bbr1_state = (picoquic_bbr1_state_t*)path_x->congestion_alg_state;
    *cc_state = (uint64_t)bbr1_state->state;
    *cc_param = bbr1_state->btl_bw;
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        (self.state as u64, self.btl_bw)
    }
```

## Pair `picoquic/bytestream.c:bytestream_delete`
C: `picoquic/bytestream.c:65-71 bytestream_delete`
Rust: `rs/fq/src/lib.rs:2416-2423 delete`

### C body
```c
{
    if (s->data != NULL) {
        free(s->data);
        s->data = NULL;
    }
}
```

### Rust body
```rust
    pub fn set_app_wake_time(&mut self, app_wake_time: Instant) {
        self.app_wake_time = app_wake_time;
    }
```

## Pair `picoquic/bytestream.c:bytestream_length`
C: `picoquic/bytestream.c:88-91 bytestream_length`
Rust: `rs/fq/src/bytestream.rs:197-204 len`

### C body
```c
{
    return s->ptr;
}
```

### Rust body
```rust
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
```

## Pair `picoquic/bytestream.c:bytestream_finished`
C: `picoquic/bytestream.c:109-112 bytestream_finished`
Rust: `rs/fq/src/bytestream.rs:227-229 is_finished`

### C body
```c
{
    return s->ptr >= s->size;
}
```

### Rust body
```rust
    pub fn is_finished(&self) -> bool {
        self.ptr >= self.data_ref().len()
    }
```

## Pair `picoquic/bytestream.c:byteread_vint`
C: `picoquic/bytestream.c:147-161 byteread_vint`
Rust: `rs/fq/src/bytestream.rs:402-406 read_varint`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 1) {
        return bytestream_error(s);
    }

    size_t len = picoquic_varint_decode(s->data + s->ptr, s->size - s->ptr, value);
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
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
```

## Pair `picoquic/bytestream.c:byteread_int8`
C: `picoquic/bytestream.c:188-197 byteread_int8`
Rust: `rs/fq/src/bytestream.rs:258-262 read_u8`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 1) {
        return bytestream_error(s);
    } else {
        *value = s->data[s->ptr++];
        return 0;
    }
}
```

### Rust body
```rust
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
```

## Pair `picoquic/bytestream.c:bytewrite_int32`
C: `picoquic/bytestream.c:236-246 bytewrite_int32`
Rust: `rs/fq/src/bytestream.rs:306-315 write_u32`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 4) {
        return bytestream_error(s);
    } else {
        picoformat_32(s->data + s->ptr, value);
        s->ptr += 4;
        return 0;
    }
}
```

### Rust body
```rust
    pub fn write_u32(&mut self, value: u32) -> Result<(), Error> {
        if self.data_ref().len() - self.ptr < 4 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr..ptr + 4].copy_from_slice(&value.to_be_bytes());
        self.ptr += 4;
        Ok(())
    }
```

## Pair `picoquic/bytestream.c:bytewrite_buffer`
C: `picoquic/bytestream.c:291-301 bytewrite_buffer`
Rust: `rs/fq/src/bytestream.rs:466-476 write_bytes`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < length) {
        return bytestream_error(s);
    }

    memcpy(s->data + s->ptr, buffer, length);
    s->ptr += length;
    return 0;
}
```

### Rust body
```rust
    pub fn write_bytes(&mut self, buffer: &[u8]) -> Result<(), Error> {
        let length = buffer.len();
        if self.data_ref().len() - self.ptr < length {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr..ptr + length].copy_from_slice(buffer);
        self.ptr += length;
        Ok(())
    }
```

## Pair `picoquic/bytestream.c:byteskip_cid`
C: `picoquic/bytestream.c:338-344 byteskip_cid`
Rust: `rs/fq/src/bytestream.rs:517-520 skip_cid`

### C body
```c
{
    uint8_t id_len = 0;
    int ret = byteread_int8(s, &id_len);
    ret |= bytestream_skip(s, id_len);
    return ret;
}
```

### Rust body
```rust
    pub fn skip_cid(&mut self) -> Result<(), Error> {
        let id_len = self.read_u8()?;
        self.skip(id_len as usize)
    }
```

## Pair `picoquic/bytestream.c:bytewrite_addr`
C: `picoquic/bytestream.c:386-399 bytewrite_addr`
Rust: `rs/fq/src/bytestream.rs:568-582 write_addr`

### C body
```c
{
    int ret = bytewrite_vint(s, addr->sa_family);
    if (addr->sa_family == AF_INET) {
        struct sockaddr_in* s4 = (struct sockaddr_in*)addr;
        ret |= bytewrite_buffer(s, &s4->sin_addr, 4);
        ret |= bytewrite_int16(s, s4->sin_port);
    } else {
        struct sockaddr_in6* s6 = (struct sockaddr_in6*)addr;
        ret |= bytewrite_buffer(s, &s6->sin6_addr, 16);
        ret |= bytewrite_int16(s, s6->sin6_port);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn write_addr(&mut self, addr: &SocketAddr) -> Result<(), Error> {
        match addr {
            SocketAddr::V4(a) => {
                self.write_varint(WIRE_AF_INET)?;
                self.write_bytes(&a.ip().octets())?;
                self.write_u16(a.port())?;
            }
            SocketAddr::V6(a) => {
                self.write_varint(WIRE_AF_INET6)?;
                self.write_bytes(&a.ip().octets())?;
                self.write_u16(a.port())?;
            }
        }
        Ok(())
    }
```

## Pair `picoquic/c4.c:c4_logger`
C: `picoquic/c4.c:199-216 c4_logger`
Rust: `rs/fq/src/c4.rs:968-994 c4_logger`

### C body
```c
{
    picoquic_log_app_message(path_x->cnx,
        "C4_rate, %" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%d ,%" PRIu64 ", %d, %d, %d, %d, %d, %d, %d",
        rate_measurement, c4_state->nominal_rate,
        (ack_state == NULL) ? 0 : ack_state->nb_bytes_delivered_since_packet_sent,
        (ack_state == NULL) ? 0 : ack_state->rtt_measurement,
        (ack_state == NULL) ? 0 : ack_state->send_delay,
        c4_state->nominal_max_rtt, (int)c4_state->alg_state, path_x->bandwidth_estimate,
        (int)(path_x->bytes_in_transit),
        beta, congestion_mode, (int)c4_state->ecn_alpha,
        (int)(path_x->smoothed_rtt),
        (int)(path_x->rtt_variant),
        (int)c4_state->alpha_1024_previous);
}
```

### Rust body
```rust
) {
    log::debug!(
        "C4_rate, {},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        rate_measurement,
        c4_state.nominal_rate,
        ack_state.map_or(0, |a| a.nb_bytes_delivered_since_packet_sent),
        ack_state.map_or(0, |a| a.rtt_measurement.ticks()),
        ack_state.map_or(0, |a| a.send_delay.ticks()),
        c4_state.nominal_max_rtt,
        c4_state.alg_state as u64,
        path_x.bandwidth_estimate,
        path_x.bytes_in_transit,
        beta,
        congestion_mode as u64,
        c4_state.ecn_alpha,
        path_x.smoothed_rtt.ticks(),
        path_x.rtt_variant.ticks(),
        c4_state.alpha_1024_previous,
    );
}
```

## Pair `picoquic/c4.c:c4_loss_threshold`
C: `picoquic/c4.c:290-299 c4_loss_threshold`
Rust: `rs/fq/src/c4.rs:362-366 loss_threshold`

### C body
```c
{
    uint64_t sensitivity = c4_sensitivity_1024(c4_state);
    double fraction = ((double)sensitivity) / 1024.0;
    double loss_threshold = 0.02 + 0.50 * (1-fraction);

    return loss_threshold;
}
```

### Rust body
```rust
    pub fn loss_threshold(&self) -> f64 {
        let sensitivity = self.sensitivity_1024();
        let fraction = sensitivity as f64 / 1024.0;
        0.02 + 0.50 * (1.0 - fraction)
    }
```

## Pair `picoquic/c4.c:c4_growth_evaluate`
C: `picoquic/c4.c:426-447 c4_growth_evaluate`
Rust: `rs/fq/src/c4.rs:149-157 growth_evaluate`

### C body
```c
{
    int is_growing = 0;
    if (c4_state->push_alpha > C4_ALPHA_PUSH_LOW_1024) {
        /* If the value of "push_alpha" was large enough, we can reasonably
         * measure growth. */
        uint64_t target_rate = (3*c4_state->push_rate_old +
            MULT1024(c4_state->push_alpha, c4_state->push_rate_old)) / 4;
        is_growing = (c4_state->nominal_rate > target_rate);
    }
    else {
        /* If the value was not big enough, we have to make decision
         * based on congestion signals.
         */
        is_growing = (c4_state->nominal_rate > c4_state->push_rate_old &&
            !c4_state->congestion_notified);
    }
    return is_growing;
}
```

### Rust body
```rust
    fn growth_evaluate(&self) -> bool {
        if self.push_alpha > C4_ALPHA_PUSH_LOW_1024 {
            let target_rate =
                (3 * self.push_rate_old + mult1024(self.push_alpha, self.push_rate_old)) / 4;
            self.nominal_rate > target_rate
        } else {
            self.nominal_rate > self.push_rate_old && !self.congestion_notified
        }
    }
```

## Pair `picoquic/c4.c:c4_enter_initial`
C: `picoquic/c4.c:486-497 c4_enter_initial`
Rust: `rs/fq/src/c4.rs:327-337 enter_initial`

### C body
```c
{
    c4_state->alg_state = c4_initial;
    c4_state->initial_cwnd = path_x->cwin;
    c4_state->probe_level = C4_PROBE_LEVEL_DEFAULT;
    c4_state->alpha_1024_current = C4_ALPHA_INITIAL;
    c4_state->nb_packets_in_startup = 0;
    c4_era_reset(path_x, c4_state);
    c4_state->nb_eras_no_increase = 0;
    c4_state->ecn_alpha = 0;
    c4_growth_reset(c4_state);
}
```

### Rust body
```rust
    fn enter_initial(&mut self, path_x: &Path, connection: &Connection) {
        self.alg_state = C4AlgState::Initial;
        self.initial_cwnd = path_x.cwin;
        self.probe_level = C4_PROBE_LEVEL_DEFAULT;
        self.alpha_1024_current = C4_ALPHA_INITIAL;
        self.nb_packets_in_startup = 0;
        self.era_reset(path_x, connection);
        self.nb_eras_no_increase = 0;
        self.ecn_alpha = 0;
        self.growth_reset();
    }
```

## Pair `picoquic/c4.c:c4_exit_initial`
C: `picoquic/c4.c:535-549 c4_exit_initial`
Rust: `rs/fq/src/c4.rs:401-412 exit_initial`

### C body
```c
{
    if (c4_state->nominal_rate > 0) {
        /* We assume that any required correction is done prior to calling this */
        uint64_t ssthresh = c4_state->initial_cwnd / 2;
        c4_state->nominal_max_rtt = ssthresh * 1000000 / c4_state->nominal_rate;
        if (c4_state->nominal_max_rtt < C4_MAX_RTT_MIN) {
            c4_state->nominal_max_rtt = C4_MAX_RTT_MIN;
        }
        c4_state->delay_threshold = c4_delay_threshold(c4_state);
        c4_state->nb_eras_no_increase = 0;
        c4_state->probe_level = C4_PROBE_LEVEL_DEFAULT;
        c4_enter_recovery(path_x, c4_state, c4_congestion_none);
    }
}
```

### Rust body
```rust
        if let Some(nominal_max_rtt) = (ssthresh * 1_000_000).checked_div(self.nominal_rate) {
            self.nominal_max_rtt = nominal_max_rtt;
            if self.nominal_max_rtt < C4_MAX_RTT_MIN {
                self.nominal_max_rtt = C4_MAX_RTT_MIN;
            }
            self.delay_threshold = self.delay_threshold();
            self.nb_eras_no_increase = 0;
            self.probe_level = C4_PROBE_LEVEL_DEFAULT;
            self.enter_recovery(path_x, connection, C4Congestion::None);
        }
```

## Pair `picoquic/c4.c:c4_init`
C: `picoquic/c4.c:625-641 c4_init`
Rust: `rs/fq/src/c4.rs:834-873 reset`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    c4_state_t* c4_state = path_x->congestion_alg_state;
    
    if (c4_state == NULL) {
        c4_state = (c4_state_t*)malloc(sizeof(c4_state_t));
    }
    
    if (c4_state != NULL){
        path_x->cnx->is_lost_feedback_notification_required = 1;
        
        c4_reset(c4_state, path_x, option_string);
    }

    path_x->congestion_alg_state = (void*)c4_state;
}
```

### Rust body
```rust
    pub fn reset(&mut self, path_x: &mut Path, connection: &Connection) {
        let option_string = self.option_string.take();
        // Zero all fields.
        *self = C4State {
            alg_state: C4AlgState::default(),
            nominal_rate: 0,
            nominal_max_rtt: 0,
            initial_cwnd: 0,
            running_min_rtt: u64::MAX,
            alpha_1024_current: C4_ALPHA_INITIAL,
            alpha_1024_previous: 0,
            nb_packets_in_startup: 0,
            era_sequence: 0,
            nb_cruise_left_before_push: 0,
            seed_cwin: 0,
            seed_rate: 0,
            probe_level: 0,
            nb_eras_no_increase: 0,
            push_rate_old: 0,
            push_alpha: 0,
            era_max_rtt: 0,
            era_min_rtt: 0,
            delay_threshold: 0,
            recent_delay_excess: 0,
            last_lost_packet_number: 0,
            smoothed_drop_rate: 0.0,
            ecn_alpha: 0,
            ecn_ect1: 0,
            ecn_ce: 0,
            ecn_threshold: 0,
            congestion_notified: false,
            push_was_not_limited: false,
            use_seed_cwin: false,
            initial_after_jitter: false,
            excess_ce_after_push: false,
            option_string,
        };
        self.set_options();
        self.enter_initial(path_x, connection);
    }
```

## Pair `picoquic/c4.c:c4_enter_push`
C: `picoquic/c4.c:744-754 c4_enter_push`
Rust: `rs/fq/src/c4.rs:738-744 enter_push`

### C body
```c
{
    c4_state->alpha_1024_current = c4_push_rate_by_probe_level[c4_state->probe_level];
    c4_state->push_alpha = c4_state->alpha_1024_current;
    c4_era_reset(path_x, c4_state);
    c4_state->alg_state = c4_pushing;
}
```

### Rust body
```rust
    fn enter_push(&mut self, path_x: &mut Path, connection: &Connection) {
        let level = self.probe_level.clamp(0, C4_PROBE_LEVEL_MAX) as usize;
        self.alpha_1024_current = C4_PUSH_RATE_BY_PROBE_LEVEL[level];
        self.push_alpha = self.alpha_1024_current;
        self.era_reset(path_x, connection);
        self.alg_state = C4AlgState::Pushing;
    }
```

## Pair `picoquic/c4.c:c4_update_rtt`
C: `picoquic/c4.c:976-1006 c4_update_rtt`
Rust: `rs/fq/src/c4.rs:638-663 update_rtt`

### C body
```c
{
    if (rtt_measurement > c4_state->era_max_rtt) {
        c4_state->era_max_rtt = rtt_measurement;
    }
    if (rtt_measurement < c4_state->era_min_rtt) {
        c4_state->era_min_rtt = rtt_measurement;
    }
    if (rtt_measurement < c4_state->running_min_rtt) {
        c4_state->running_min_rtt = rtt_measurement;
    }
    if (c4_state->nominal_max_rtt == 0) {
        c4_state->nominal_max_rtt = rtt_measurement;
        if (c4_state->nominal_max_rtt < C4_MAX_RTT_MIN) {
            c4_state->nominal_max_rtt = C4_MAX_RTT_MIN;
        }
        c4_state->delay_threshold = c4_delay_threshold(c4_state);
        c4_state->recent_delay_excess = 0;
    }
    else {
        uint64_t target_rtt = c4_state->nominal_max_rtt + c4_state->delay_threshold;
        if (rtt_measurement > target_rtt) {
            c4_state->recent_delay_excess = rtt_measurement - target_rtt;
        }
        else {
            c4_state->recent_delay_excess = 0;
        }
    }
}
```

### Rust body
```rust
    fn update_rtt(&mut self, rtt_measurement: u64) {
        if rtt_measurement > self.era_max_rtt {
            self.era_max_rtt = rtt_measurement;
        }
        if rtt_measurement < self.era_min_rtt {
            self.era_min_rtt = rtt_measurement;
        }
        if rtt_measurement < self.running_min_rtt {
            self.running_min_rtt = rtt_measurement;
        }
        if self.nominal_max_rtt == 0 {
            self.nominal_max_rtt = rtt_measurement;
            if self.nominal_max_rtt < C4_MAX_RTT_MIN {
                self.nominal_max_rtt = C4_MAX_RTT_MIN;
            }
            self.delay_threshold = self.delay_threshold();
            self.recent_delay_excess = 0;
        } else {
            let target_rtt = self.nominal_max_rtt + self.delay_threshold;
            if rtt_measurement > target_rtt {
                self.recent_delay_excess = rtt_measurement - target_rtt;
            } else {
                self.recent_delay_excess = 0;
            }
        }
    }
```

## Pair `picoquic/cc_common.c:picoquic_cc_get_sequence_number`
C: `picoquic/cc_common.c:27-39 picoquic_cc_get_sequence_number`
Rust: `rs/fq/src/bbr.rs:1041-1048 start_round`

### C body
```c
{
    uint64_t sequence_number;

    if (cnx->is_multipath_enabled) {
            sequence_number = path_x->pkt_ctx.send_sequence;
        }
    else {
       sequence_number = cnx->pkt_ctx[picoquic_packet_context_application].send_sequence;
    }

    return sequence_number;
}
```

### Rust body
```rust
    pub(crate) fn start_round(&mut self, connection: &Connection, path_x: &Path) {
        self.round_start_pn = if connection.is_multipath_enabled {
            path_x.pkt_ctx.send_sequence
        } else {
            connection.pkt_ctx[PacketContext::Application as usize].send_sequence
        };
        self.next_round_delivered = path_x.delivered;
    }
```

## Pair `picoquic/cc_common.c:picoquic_cc_filter_rtt_min_max`
C: `picoquic/cc_common.c:78-103 picoquic_cc_filter_rtt_min_max`
Rust: `rs/fq/src/cc_common.rs:97-119 filter_rtt_min_max`

### C body
```c
{
    int x = rtt_track->sample_current;
    int x_max;

    rtt_track->samples[x] = rtt;

    rtt_track->sample_current = x + 1;
    if (rtt_track->sample_current >= PICOQUIC_MIN_MAX_RTT_SCOPE) {
        rtt_track->is_init = 1;
        rtt_track->sample_current = 0;
    }
    
    x_max = (rtt_track->is_init) ? PICOQUIC_MIN_MAX_RTT_SCOPE : x + 1;

    rtt_track->sample_min = rtt_track->samples[0];
    rtt_track->sample_max = rtt_track->samples[0];

    for (int i = 1; i < x_max; i++) {
        if (rtt_track->samples[i] < rtt_track->sample_min) {
            rtt_track->sample_min = rtt_track->samples[i];
        } else if (rtt_track->samples[i] > rtt_track->sample_max) {
            rtt_track->sample_max = rtt_track->samples[i];
        }
    }
}
```

### Rust body
```rust
    pub fn filter_rtt_min_max(&mut self, rtt: Duration) {
        let x = self.sample_current;
        self.samples[x] = rtt;
        self.sample_current = x + 1;
        if self.sample_current >= MIN_MAX_RTT_SCOPE {
            self.is_init = true;
            self.sample_current = 0;
        }
        let x_max = if self.is_init {
            MIN_MAX_RTT_SCOPE
        } else {
            x + 1
        };
        self.sample_min = self.samples[0];
        self.sample_max = self.samples[0];
        for i in 1..x_max {
            if self.samples[i] < self.sample_min {
                self.sample_min = self.samples[i];
            } else if self.samples[i] > self.sample_max {
                self.sample_max = self.samples[i];
            }
        }
    }
```
