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

## `picoquic/bbr.c:BBRUpdateACKAggregation`
* Phase 4C status: `suspect`
* Phase 4C rationale: Logic is broadly similar, but Rust uses wrapping_sub and saturating arithmetic where C uses plain unsigned subtraction/multiplication.
* C source: `picoquic/bbr.c:1129-1147`
* C signature: `void BBRUpdateACKAggregation(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:1989-2015`
* Rust item: `update_ack_aggregation`

### C body
```c
{
    /* Find excess ACKed beyond expected amount over this interval */
    uint64_t interval = (current_time - bbr_state->extra_acked_interval_start);
    uint64_t expected_delivered = bbr_state->bw * interval;
    /* Reset interval if ACK rate is below expected rate: */
    if (bbr_state->extra_acked_delivered <= expected_delivered) {
        bbr_state->extra_acked_delivered = 0;
        bbr_state->extra_acked_interval_start = current_time;
        expected_delivered = 0;
    }
    bbr_state->extra_acked_delivered += rs->newly_acked;
    uint64_t extra = bbr_state->extra_acked_delivered - expected_delivered;
    if (extra > path_x->cwin) {
        extra = path_x->cwin;
    }
    bbr_state->extra_acked =
        update_windowed_max_filter(bbr_state->ExtraACKedFilter, extra, bbr_state->round_count, BBRExtraAckedFilterLen);
}
```

### Rust body
```rust
    ) {
        let interval = current_time.wrapping_sub(self.extra_acked_interval_start);
        // C: expected_delivered = bw * interval (bw bytes/s, interval µs; unit
        // mismatch is intentional — mirrors the C source verbatim).
        let mut expected_delivered = self.bw.saturating_mul(interval);
        if self.extra_acked_delivered <= expected_delivered {
            self.extra_acked_delivered = 0;
            self.extra_acked_interval_start = current_time;
            expected_delivered = 0;
        }
        self.extra_acked_delivered += rs.newly_acked;
        let extra = self
            .extra_acked_delivered
            .saturating_sub(expected_delivered)
            .min(path_x.cwin);
        self.extra_acked = update_windowed_max_filter(
            &mut self.extra_acked_filter,
            extra,
            self.round_count,
            10, // BBRExtraAckedFilterLen
        );
    }
```

## `picoquic/bytestream.c:byteread_vlen`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both read a varint and check usize/size_t fit, but C writes the cast value before returning a fit error while Rust returns no value on overflow.
* C source: `picoquic/bytestream.c:168-175`
* C signature: `int byteread_vlen(bytestream *, size_t *)`
* Rust source: `rs/fq/src/bytestream.rs:454-461`
* Rust item: `read_vlen`

### C body
```c
{
    uint64_t val_read = 0;
    int ret = byteread_vint(s, &val_read);

    *value = (size_t)val_read;
    return *value != val_read ? -1 : ret;
}
```

### Rust body
```rust
    pub fn read_vlen(&mut self) -> Result<usize, Error> {
        let val = self.read_varint()?;
        let as_usize = val as usize;
        if as_usize as u64 != val {
            return Err(Error::InvalidArgument);
        }
        Ok(as_usize)
    }
```

## `picoquic/c4.c:c4_init`
* Phase 4C status: `suspect`
* Phase 4C rationale: C allocates or reuses path congestion state, sets is_lost_feedback_notification_required, calls c4_reset, and stores the state pointer; Rust shown only resets an existing state and enters initial.
* C source: `picoquic/c4.c:625-641`
* C signature: `void c4_init(picoquic_path_t *, const char *, uint64_t)`
* Rust source: `rs/fq/src/c4.rs:834-873`
* Rust item: `reset`

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

## `picoquic/config.c:config_optval_string`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns the mutable buffer after NUL-terminating copied bytes; Rust returns a UTF-8 str over copied bytes and returns empty string on invalid UTF-8.
* C source: `picoquic/config.c:181-189`
* C signature: `char * config_optval_string(char *, size_t, const char *, size_t)`
* Rust source: `rs/fq/src/config.rs:553-560`
* Rust item: `config_optval_string`

### C body
```c
{
    if (p_length + 1 > buffer_max) {
        p_length = buffer_max - 1;
    }
    memcpy(buffer, p, p_length);
    buffer[p_length] = 0;
    return buffer;
}
```

### Rust body
```rust
fn config_optval_string<'a>(buffer: &'a mut [u8], p: &[u8]) -> &'a str {
    let len = p.len().min(buffer.len().saturating_sub(1));
    buffer[..len].copy_from_slice(&p[..len]);
    if !buffer.is_empty() {
        buffer[len] = 0;
    }
    core::str::from_utf8(&buffer[..len]).unwrap_or("")
}
```

## `picoquic/cubic.c:cubic_W_cubic`
* Phase 4C status: `suspect`
* Phase 4C rationale: Formula matches, but Rust uses wrapping_sub for current_time - start_of_epoch, which is a body-visible arithmetic difference if current_time is earlier.
* C source: `picoquic/cubic.c:121-130`
* C signature: `double cubic_W_cubic(picoquic_cubic_state_t *, uint64_t)`
* Rust source: `rs/fq/src/cubic.rs:112-116`
* Rust item: `w_cubic`

### C body
```c
{
    double delta_t_sec = ((double)(current_time - cubic_state->start_of_epoch) / 1000000.0) - cubic_state->K;
    double W_cubic = (PICOQUIC_CUBIC_C * (delta_t_sec * delta_t_sec * delta_t_sec)) + cubic_state->W_max;

    return W_cubic;
}
```

### Rust body
```rust
    fn w_cubic(&self, current_time: u64) -> f64 {
        let delta_t_sec =
            current_time.wrapping_sub(self.start_of_epoch) as f64 / 1_000_000.0 - self.k;
        CUBIC_C * (delta_t_sec * delta_t_sec * delta_t_sec) + self.w_max
    }
```
