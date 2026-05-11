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

## Pair `picoquic/bytestream.c:bytestream_ref_init`
C: `picoquic/bytestream.c:30-37 bytestream_ref_init`
Rust: `rs/fq/src/bytestream.rs:164-175 from_slice`

### C body
```c
{
    s->data = (uint8_t*)bytes;
    s->size = nb_bytes;
    s->ptr = 0;

    return s;
}
```

### Rust body
```rust
    pub fn as_bytes(&self) -> &[u8] {
        &self.data_ref()[..self.ptr]
    }
```

## Pair `picoquic/bytestream.c:bytestream_data`
C: `picoquic/bytestream.c:73-76 bytestream_data`
Rust: `rs/fq/src/bytestream.rs:173-175 as_bytes`

### C body
```c
{
    return s->data;
}
```

### Rust body
```rust
    pub fn as_bytes(&self) -> &[u8] {
        &self.data_ref()[..self.ptr]
    }
```

## Pair `picoquic/bytestream.c:bytestream_remain`
C: `picoquic/bytestream.c:93-96 bytestream_remain`
Rust: `rs/fq/src/bytestream.rs:208-216 remaining`

### C body
```c
{
    return s->size - s->ptr;
}
```

### Rust body
```rust
    pub fn reset(&mut self) {
        self.ptr = 0;
    }
```

## Pair `picoquic/bytestream.c:bytestream_skip`
C: `picoquic/bytestream.c:114-123 bytestream_skip`
Rust: `rs/fq/src/bytestream.rs:236-243 skip`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < nb_bytes) {
        return bytestream_error(s);
    } else {
        s->ptr += nb_bytes;
        return 0;
    }
}
```

### Rust body
```rust
    pub fn skip(&mut self, nb_bytes: usize) -> Result<(), Error> {
        if self.data_ref().len() - self.ptr < nb_bytes {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        self.ptr += nb_bytes;
        Ok(())
    }
```

## Pair `picoquic/bytestream.c:bytestream_vint_len`
C: `picoquic/bytestream.c:163-166 bytestream_vint_len`
Rust: `rs/fq/src/bytestream.rs:656-661 bytestream_vint_len`

### C body
```c
{
    return picoquic_encode_varint_length(value);
}
```

### Rust body
```rust
mod test {}
```

## Pair `picoquic/bytestream.c:byteshow_int8`
C: `picoquic/bytestream.c:199-208 byteshow_int8`
Rust: `rs/fq/src/bytestream.rs:273-276 peek_u8`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 1) {
        return -1;
    } else {
        *value = s->data[s->ptr];
        return 0;
    }
}
```

### Rust body
```rust
        if self.ptr >= self.data_ref().len() {
            return Err(Error::BufferTooSmall);
        }
```

## Pair `picoquic/bytestream.c:byteread_int32`
C: `picoquic/bytestream.c:248-260 byteread_int32`
Rust: `rs/fq/src/bytestream.rs:318-322 read_u32`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 4) {
        return bytestream_error(s);
    }
    else {
        const uint8_t * ptr = s->data + s->ptr;
        *value = (((uint32_t)ptr[0]) << 24) | (((uint32_t)ptr[1]) << 16) | (((uint32_t)ptr[2]) << 8) | ptr[3];
        s->ptr += 4;
        return 0;
    }
}
```

### Rust body
```rust
        if self.data_ref().len() - self.ptr < 4 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
```

## Pair `picoquic/bytestream.c:byteread_buffer`
C: `picoquic/bytestream.c:303-313 byteread_buffer`
Rust: `rs/fq/src/bytestream.rs:480-489 read_bytes`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < length) {
        return bytestream_error(s);
    }

    memcpy(buffer, s->data + s->ptr, length);
    s->ptr += length;
    return 0;
}
```

### Rust body
```rust
    pub fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        let length = buffer.len();
        if self.data_ref().len() - self.ptr < length {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        buffer.copy_from_slice(&self.data_ref()[self.ptr..self.ptr + length]);
        self.ptr += length;
        Ok(())
    }
```

## Pair `picoquic/bytestream.c:bytewrite_cstr`
C: `picoquic/bytestream.c:346-352 bytewrite_cstr`
Rust: `rs/fq/src/bytestream.rs:526-530 write_str`

### C body
```c
{
    size_t l_cstr = strlen(cstr);
    int ret = bytewrite_vint(s, l_cstr);
    ret |= bytewrite_buffer(s, cstr, l_cstr);
    return ret;
}
```

### Rust body
```rust
    pub fn write_str(&mut self, s: &str) -> Result<(), Error> {
        let bytes = s.as_bytes();
        self.write_varint(bytes.len() as u64)?;
        self.write_bytes(bytes)
    }
```

## Pair `picoquic/bytestream.c:byteread_addr`
C: `picoquic/bytestream.c:401-418 byteread_addr`
Rust: `rs/fq/src/bytestream.rs:587-600 read_addr`

### C body
```c
{
    uint64_t family = 0;
    int ret = byteread_vint(s, &family);

    if (ret == 0 && family == AF_INET) {
        struct sockaddr_in* s4 = (struct sockaddr_in*)addr;
        s4->sin_family = AF_INET;
        ret |= byteread_buffer(s, &s4->sin_addr, 4);
        ret |= byteread_int16(s, &s4->sin_port);
    } else {
        struct sockaddr_in6* s6 = (struct sockaddr_in6*)addr;
        s6->sin6_family = AF_INET6;
        ret |= byteread_buffer(s, &s6->sin6_addr, 16);
        ret |= byteread_int16(s, &s6->sin6_port);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn read_addr(&mut self) -> Result<SocketAddr, Error> {
        let family = self.read_varint()?;
        if family == WIRE_AF_INET {
            let mut octets = [0u8; 4];
            self.read_bytes(&mut octets)?;
            let port = self.read_u16()?;
            Ok(SocketAddr::from((octets, port)))
        } else {
            let mut octets = [0u8; 16];
            self.read_bytes(&mut octets)?;
            let port = self.read_u16()?;
            Ok(SocketAddr::from((octets, port)))
        }
    }
```

## Pair `picoquic/c4.c:c4_sensitivity_1024`
C: `picoquic/c4.c:244-260 c4_sensitivity_1024`
Rust: `rs/fq/src/c4.rs:131-140 sensitivity_1024`

### C body
```c
{
    uint64_t sensitivity = 1024;
    if (c4_state->nominal_rate < 50000) {
        sensitivity = 0;
    }
    else if (c4_state->nominal_rate > 10000000) {
        sensitivity = 1024;
    }
    else if (c4_state->nominal_rate < 1000000) {
        sensitivity = (c4_state->nominal_rate - 50000) * 963 / 950000;
    }
    else {
        sensitivity = 963 + ((c4_state->nominal_rate - 1000000) * 61 / 9000000);
    }
    return sensitivity;
}
```

### Rust body
```rust
        } else {
            963 + ((self.nominal_rate - 1_000_000) * 61 / 9_000_000)
        }
```

## Pair `picoquic/c4.c:c4_update_loss_rate`
C: `picoquic/c4.c:301-319 c4_update_loss_rate`
Rust: `rs/fq/src/c4.rs:206-222 update_loss_rate`

### C body
```c
{
    uint64_t next_number = c4_state->last_lost_packet_number;

    if (lost_packet_number > next_number) {
        if (next_number + PICOQUIC_SMOOTHED_LOSS_SCOPE < lost_packet_number) {
            next_number = lost_packet_number - PICOQUIC_SMOOTHED_LOSS_SCOPE;
        }

        while (next_number < lost_packet_number) {
            c4_state->smoothed_drop_rate *= (1.0 - PICOQUIC_SMOOTHED_LOSS_FACTOR);
            next_number++;
        }

        c4_state->smoothed_drop_rate += (1.0 - c4_state->smoothed_drop_rate) * PICOQUIC_SMOOTHED_LOSS_FACTOR;
        c4_state->last_lost_packet_number = lost_packet_number;
    }
}
```

### Rust body
```rust
    pub fn update_loss_rate(&mut self, lost_packet_number: u64) {
        let mut next_number = self.last_lost_packet_number;

        if lost_packet_number > next_number {
            if next_number + SMOOTHED_LOSS_SCOPE < lost_packet_number {
                next_number = lost_packet_number - SMOOTHED_LOSS_SCOPE;
            }

            while next_number < lost_packet_number {
                self.smoothed_drop_rate *= 1.0 - SMOOTHED_LOSS_FACTOR;
                next_number += 1;
            }

            self.smoothed_drop_rate += (1.0 - self.smoothed_drop_rate) * SMOOTHED_LOSS_FACTOR;
            self.last_lost_packet_number = lost_packet_number;
        }
    }
```

## Pair `picoquic/c4.c:c4_growth_reset`
C: `picoquic/c4.c:449-456 c4_growth_reset`
Rust: `rs/fq/src/c4.rs:160-167 growth_reset`

### C body
```c
{
    c4_state->congestion_notified = 0;
    c4_state->push_was_not_limited = 0;
    c4_state->push_rate_old = c4_state->nominal_rate;
    /* Push alpha will have to be reset to the correct value when entering push */
    c4_state->push_alpha = c4_state->alpha_1024_current;
}
```

### Rust body
```rust
    fn growth_reset(&mut self) {
        self.congestion_notified = false;
        self.push_was_not_limited = false;
        self.push_rate_old = self.nominal_rate;
        // push_alpha is reset to current alpha here; caller sets it to correct
        // value when entering push state.
        self.push_alpha = self.alpha_1024_current;
    }
```

## Pair `picoquic/c4.c:c4_set_options`
C: `picoquic/c4.c:499-515 c4_set_options`
Rust: `rs/fq/src/c4.rs:173-179 set_options`

### C body
```c
{
    if (c4_state->option_string != NULL) {
        char const* x = c4_state->option_string;
        char c;
        int ended = 0;

        while ((c = *x) != 0 && !ended) {
            x++;
            switch (c) {
            default:
                ended = 1;
                break;
            }
        }
    }
}
```

### Rust body
```rust
        if let Some(s) = self.option_string.as_deref() {
            // No recognized option characters are currently defined.  The C switch
            // hits `default: ended = 1` for every character, so the loop exits
            // immediately after consuming the first character.
            let _ = s.chars().next();
        }
```

## Pair `picoquic/c4.c:c4_initial_handle_rtt_excess`
C: `picoquic/c4.c:551-566 c4_initial_handle_rtt_excess`
Rust: `rs/fq/src/c4.rs:670-677 initial_handle_rtt_excess`

### C body
```c
{
    /* HyStart. */
    /* Using RTT increases as congestion signal. This is used
     * for getting out of slow start, but also for ending a cycle
     * during congestion avoidance */
    /* we do not directly use "hystart test", because we want to separate the
    * "update_rtt" functions from the actual tests.
     */
    if (c4_state->recent_delay_excess > 0
        && c4_state->nb_eras_no_increase > 1
        && c4_state->push_rate_old >= c4_state->nominal_rate){

        c4_exit_initial(path_x, c4_state);
    }
}
```

### Rust body
```rust
    fn initial_handle_rtt_excess(&mut self, path_x: &mut Path, connection: &Connection) {
        if self.recent_delay_excess > 0
            && self.nb_eras_no_increase > 1
            && self.push_rate_old >= self.nominal_rate
        {
            self.exit_initial(path_x, connection);
        }
    }
```

## Pair `picoquic/c4.c:c4_enter_recovery`
C: `picoquic/c4.c:643-666 c4_enter_recovery`
Rust: `rs/fq/src/c4.rs:345-355 enter_recovery`

### C body
```c
{
    if (c4_state->alg_state == c4_initial) {
        c4_growth_reset(c4_state);
    }
    /* There may be multiple congestion signals coming in, but we 
    * will not reinitialize the state if C4 is already in recovery.
     */
    if (c4_state->alg_state != c4_recovery) {
        c4_state->excess_ce_after_push = (c_mode != c4_congestion_ecn) ? 0 : 1;
        c4_state->alg_state = c4_recovery;
        c4_era_reset(path_x, c4_state);
        c4_state->alpha_1024_current = C4_ALPHA_RECOVER_1024;
    }
}
```

### Rust body
```rust
    fn enter_recovery(&mut self, path_x: &Path, connection: &Connection, c_mode: C4Congestion) {
        if self.alg_state == C4AlgState::Initial {
            self.growth_reset();
        }
        if self.alg_state != C4AlgState::Recovery {
            self.excess_ce_after_push = c_mode == C4Congestion::Ecn;
            self.alg_state = C4AlgState::Recovery;
            self.era_reset(path_x, connection);
            self.alpha_1024_current = C4_ALPHA_RECOVER_1024;
        }
    }
```

## Pair `picoquic/c4.c:c4_update_min_max_rtt`
C: `picoquic/c4.c:756-800 c4_update_min_max_rtt`
Rust: `rs/fq/src/c4.rs:696-732 update_min_max_rtt`

### C body
```c
{
    /* Include the last sample, to deal with order of arrivals between ACK and RTT */
    if (path_x->rtt_sample > c4_state->era_max_rtt) {
        c4_state->era_max_rtt = path_x->rtt_sample;
    }
    if (path_x->rtt_sample < c4_state->era_min_rtt) {
        c4_state->era_min_rtt = path_x->rtt_sample;
    }
    if (c4_state->alpha_1024_previous <= C4_ALPHA_NEUTRAL_1024) {
        /* Update the running min RTT, as the max RTT computation depends on it. */
        if (c4_state->era_min_rtt < c4_state->running_min_rtt) {
            c4_state->running_min_rtt = c4_state->era_min_rtt;
        }
        else {
            c4_state->running_min_rtt = (7 * c4_state->running_min_rtt + c4_state->era_min_rtt) / 8;
        }

        /* We want to increase the max RTT, but we want to limit the jitter
         * measurement to avoid aberrant behavior.
         */
        uint64_t corrected_max = (c4_state->era_max_rtt < c4_state->running_min_rtt + C4_MAX_JITTER) ?
            c4_state->era_max_rtt : c4_state->running_min_rtt + C4_MAX_JITTER;

        if (corrected_max > c4_state->nominal_max_rtt) {
            c4_state->nominal_max_rtt = corrected_max;
        }
        else {
            /* If not growing, slowly diminish the max rtt */
            c4_state->nominal_max_rtt = (7 * c4_state->nominal_max_rtt + corrected_max) / 8;
        }
        /* Recompute the delay threshold as the max RTT was updated. */
        c4_state->delay_threshold = c4_delay_threshold(c4_state);
    }
    else if (c4_state->nominal_max_rtt == 0) {
        /* Initialize the max RTT and the delay threshold. */
        c4_state->nominal_max_rtt = c4_state->era_max_rtt;
        c4_state->delay_threshold = c4_delay_threshold(c4_state);
    }

    if (c4_state->nominal_max_rtt < C4_MAX_RTT_MIN) {
        c4_state->nominal_max_rtt = C4_MAX_RTT_MIN;
        c4_state->delay_threshold = c4_delay_threshold(c4_state);
    }
}
```

### Rust body
```rust
    fn update_min_max_rtt(&mut self, path_x: &Path) {
        let rtt_sample = path_x.rtt_sample.ticks();
        if rtt_sample > self.era_max_rtt {
            self.era_max_rtt = rtt_sample;
        }
        if rtt_sample < self.era_min_rtt {
            self.era_min_rtt = rtt_sample;
        }
        if self.alpha_1024_previous <= C4_ALPHA_NEUTRAL_1024 {
            if self.era_min_rtt < self.running_min_rtt {
                self.running_min_rtt = self.era_min_rtt;
            } else {
                self.running_min_rtt = (7 * self.running_min_rtt + self.era_min_rtt) / 8;
            }

            let corrected_max = if self.era_max_rtt < self.running_min_rtt + C4_MAX_JITTER {
                self.era_max_rtt
            } else {
                self.running_min_rtt + C4_MAX_JITTER
            };

            if corrected_max > self.nominal_max_rtt {
                self.nominal_max_rtt = corrected_max;
            } else {
                self.nominal_max_rtt = (7 * self.nominal_max_rtt + corrected_max) / 8;
            }
            self.delay_threshold = self.delay_threshold();
        } else if self.nominal_max_rtt == 0 {
            self.nominal_max_rtt = self.era_max_rtt;
            self.delay_threshold = self.delay_threshold();
        }

        if self.nominal_max_rtt < C4_MAX_RTT_MIN {
            self.nominal_max_rtt = C4_MAX_RTT_MIN;
            self.delay_threshold = self.delay_threshold();
        }
    }
```

## Pair `picoquic/c4.c:c4_handle_rtt_excess`
C: `picoquic/c4.c:1008-1017 c4_handle_rtt_excess`
Rust: `rs/fq/src/c4.rs:683-687 handle_rtt_excess`

### C body
```c
{
    if (c4_state->recent_delay_excess > 0 &&
        c4_state->alpha_1024_previous > 1024) {
        /* May well be congested */
        c4_notify_congestion(path_x, c4_state, c4_congestion_delay);
    }
}
```

### Rust body
```rust
    fn handle_rtt_excess(&mut self, path_x: &mut Path, connection: &Connection) {
        if self.recent_delay_excess > 0 && self.alpha_1024_previous > 1024 {
            self.notify_congestion(path_x, connection, C4Congestion::Delay);
        }
    }
```

## Pair `picoquic/cc_common.c:picoquic_cc_get_ack_number`
C: `picoquic/cc_common.c:41-53 picoquic_cc_get_ack_number`
Rust: `rs/fq/src/cc_common.rs:271-302 ack_number`

### C body
```c
{
    uint64_t highest_acknowledged;

    if (cnx->is_multipath_enabled) {
        highest_acknowledged = path_x->pkt_ctx.highest_acknowledged;
    }
    else {
        highest_acknowledged = cnx->pkt_ctx[picoquic_packet_context_application].highest_acknowledged;
    }

    return highest_acknowledged;
}
```

### Rust body
```rust
impl ConnectionCc for Connection {
    fn sequence_number(&self, path_x: &Path) -> u64 {
        if self.is_multipath_enabled {
            path_x.pkt_ctx.send_sequence
        } else {
            self.pkt_ctx[crate::PacketContext::Application as usize].send_sequence
        }
    }

    fn ack_number(&self, path_x: &Path) -> u64 {
        if self.is_multipath_enabled {
            path_x.pkt_ctx.highest_acknowledged
        } else {
            self.pkt_ctx[crate::PacketContext::Application as usize].highest_acknowledged
        }
    }

    fn ack_sent_time(&self, path_x: &Path) -> Instant {
        if self.is_multipath_enabled {
            path_x.pkt_ctx.latest_time_acknowledged
        } else {
            self.pkt_ctx[crate::PacketContext::Application as usize].latest_time_acknowledged
        }
    }
}
```

## Pair `picoquic/cc_common.c:picoquic_cc_hystart_loss_test`
C: `picoquic/cc_common.c:105-136 picoquic_cc_hystart_loss_test`
Rust: `rs/fq/src/cc_common.rs:126-157 hystart_loss_test`

### C body
```c
{
    int ret = 0;
    uint64_t next_number = rtt_track->last_lost_packet_number;

    if (lost_packet_number > next_number) {
        if (next_number + PICOQUIC_SMOOTHED_LOSS_SCOPE < lost_packet_number) {
            next_number = lost_packet_number - PICOQUIC_SMOOTHED_LOSS_SCOPE;
        }

        while (next_number < lost_packet_number) {
            rtt_track->smoothed_drop_rate *= (1.0 - PICOQUIC_SMOOTHED_LOSS_FACTOR);
            next_number++;
        }

        rtt_track->smoothed_drop_rate += (1.0 - rtt_track->smoothed_drop_rate) * PICOQUIC_SMOOTHED_LOSS_FACTOR;
        rtt_track->last_lost_packet_number = lost_packet_number;

        switch (event) {
        case picoquic_congestion_notification_repeat:
            ret = rtt_track->smoothed_drop_rate > error_rate_max;
            break;
        case picoquic_congestion_notification_timeout:
            ret = 1;
        default:
            break;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> bool {
        let mut ret = false;
        let mut next_number = self.last_lost_packet_number;

        if lost_packet_number > next_number {
            if next_number + SMOOTHED_LOSS_SCOPE < lost_packet_number {
                next_number = lost_packet_number - SMOOTHED_LOSS_SCOPE;
            }
            while next_number < lost_packet_number {
                self.smoothed_drop_rate *= 1.0 - SMOOTHED_LOSS_FACTOR;
                next_number += 1;
            }
            self.smoothed_drop_rate += (1.0 - self.smoothed_drop_rate) * SMOOTHED_LOSS_FACTOR;
            self.last_lost_packet_number = lost_packet_number;

            match event {
                CongestionNotification::Repeat => {
                    ret = self.smoothed_drop_rate > error_rate_max;
                }
                CongestionNotification::Timeout => {
                    ret = true;
                }
                _ => {}
            }
        }
        ret
    }
```
