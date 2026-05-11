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

## Pair `picoquic/cc_common.c:picoquic_cc_hystart_loss_volume_test`
C: `picoquic/cc_common.c:138-165 picoquic_cc_hystart_loss_volume_test`
Rust: `rs/fq/src/cc_common.rs:162-187 hystart_loss_volume_test`

### C body
```c
{
    int ret = 0;

    rtt_track->smoothed_bytes_lost_16 -= rtt_track->smoothed_bytes_lost_16 / 16;
    rtt_track->smoothed_bytes_lost_16 += nb_bytes_newly_lost;
    rtt_track->smoothed_bytes_sent_16 -= rtt_track->smoothed_bytes_sent_16 / 16;
    rtt_track->smoothed_bytes_sent_16 += nb_bytes_newly_acked + nb_bytes_newly_lost;

    if (rtt_track->smoothed_bytes_sent_16 > 0) {
        rtt_track->smoothed_drop_rate = ((double)rtt_track->smoothed_bytes_lost_16) / ((double)rtt_track->smoothed_bytes_sent_16);
    }
    else {
        rtt_track->smoothed_drop_rate = 0;
    }

    switch (event) {
    case picoquic_congestion_notification_acknowledgement:
        ret = rtt_track->smoothed_drop_rate > PICOQUIC_SMOOTHED_LOSS_THRESHOLD;
        break;
    case picoquic_congestion_notification_timeout:
        ret = 1;
    default:
        break;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> bool {
        self.smoothed_bytes_lost_16 -= self.smoothed_bytes_lost_16 / 16;
        self.smoothed_bytes_lost_16 += nb_bytes_newly_lost;
        self.smoothed_bytes_sent_16 -= self.smoothed_bytes_sent_16 / 16;
        self.smoothed_bytes_sent_16 += nb_bytes_newly_acked + nb_bytes_newly_lost;

        if self.smoothed_bytes_sent_16 > 0 {
            self.smoothed_drop_rate =
                self.smoothed_bytes_lost_16 as f64 / self.smoothed_bytes_sent_16 as f64;
        } else {
            self.smoothed_drop_rate = 0.0;
        }

        match event {
            CongestionNotification::Acknowledgement => {
                self.smoothed_drop_rate > SMOOTHED_LOSS_THRESHOLD
            }
            CongestionNotification::Timeout => true,
            _ => false,
        }
    }
```

## Pair `picoquic/cc_common.c:picoquic_cc_slow_start_increase_ex2`
C: `picoquic/cc_common.c:235-256 picoquic_cc_slow_start_increase_ex2`
Rust: `rs/fq/src/cc_common.rs:330-416 slow_start_increase_ex2`

### C body
```c
uint64_t picoquic_cc_slow_start_increase_ex2(picoquic_path_t* path_x, uint64_t nb_delivered, int in_css, uint64_t prague_alpha) {
    if (prague_alpha != 0) { /* monitoring of ECN */
        uint64_t delta = nb_delivered;

        /* Calculate delta based on prague_ahpha. */
        if (path_x->smoothed_rtt <= PICOQUIC_TARGET_RENO_RTT) {
            /* smoothed_rtt <= 100ms */
            delta *= (1024 - prague_alpha);
            delta /= 1024;
        } else {
            delta *= path_x->smoothed_rtt;
            delta *= (1024 - prague_alpha);
            delta /= PICOQUIC_TARGET_RENO_RTT;
            delta /= 1024;
        }

        return picoquic_cc_slow_start_increase_ex(path_x, delta, in_css);
    }

    /* Fallback to HyStart++ Consecutive Slow Start. */
    return picoquic_cc_slow_start_increase_ex(path_x, nb_delivered, in_css);
}
```

### Rust body
```rust
impl PathCc for Path {
    fn lowest_not_ack(&self) -> u64 {
        // C reads cnx->pkt_ctx[app] for single-path, path->pkt_ctx for multipath.
        // Path has no back-pointer to Connection, so we always use path->pkt_ctx
        // (exact for multipath; conservative approximation for single-path).
        self.pkt_ctx
            .pending
            .keys()
            .next()
            .copied()
            .unwrap_or(self.pkt_ctx.highest_acknowledged + 1)
    }

    fn slow_start_increase(&self, nb_delivered: u64) -> u64 {
        // C body checks cnx->cwin_blocked.  Path has no back-pointer to
        // Connection, so approximate with bytes_in_transit >= cwin, which is
        // the condition that sets cwin_blocked in the C library.
        if self.bytes_in_transit < self.cwin {
            0
        } else {
            nb_delivered
        }
    }

    fn slow_start_increase_ex(&self, nb_delivered: u64, in_css: bool) -> u64 {
        if in_css {
            self.slow_start_increase(nb_delivered / HYSTART_PP_CSS_GROWTH_DIVISOR)
        } else {
            self.slow_start_increase(nb_delivered)
        }
    }

    fn slow_start_increase_ex2(&self, nb_delivered: u64, in_css: bool, prague_alpha: u64) -> u64 {
        if prague_alpha != 0 {
            let delta = if self.smoothed_rtt <= TARGET_RENO_RTT {
                nb_delivered * (1024 - prague_alpha) / 1024
            } else {
                nb_delivered * self.smoothed_rtt.ticks() * (1024 - prague_alpha)
                    / TARGET_RENO_RTT.ticks()
                    / 1024
            };
            self.slow_start_increase_ex(delta, in_css)
        } else {
            self.slow_start_increase_ex(nb_delivered, in_css)
        }
    }

    fn update_target_cwin_estimation(&self) -> u64 {
        // BYTES_FROM_RATE(smoothed_rtt, peak_bandwidth_estimate) = rtt_us * bps / 1_000_000
        let max_win = self.smoothed_rtt.ticks() * self.peak_bandwidth_estimate / 1_000_000;
        let min_win = max_win / 2;
        if min_win > self.cwin {
            min_win
        } else {
            self.cwin
        }
    }

    fn update_cwin_for_long_rtt(&self) -> u64 {
        let rtt_cap = if self.rtt_min > TARGET_SATELLITE_RTT {
            TARGET_SATELLITE_RTT
        } else {
            self.rtt_min
        };
        let min_cwnd =
            (CWIN_INITIAL as f64 * rtt_cap.ticks() as f64 / TARGET_RENO_RTT.ticks() as f64) as u64;
        if min_cwnd > self.cwin {
            min_cwnd
        } else {
            self.cwin
        }
    }
}
```

## Pair `picoquic/config.c:config_parse_target_version`
C: `picoquic/config.c:116-146 config_parse_target_version`
Rust: `rs/fq/src/config.rs:544-560 parse_hex_version`

### C body
```c
{
    /* Expect the version to be encoded in base 16 */
    uint32_t v = 0;
    char const* x = v_arg;

    while (*x != 0) {
        int c = *x;

        if (c >= '0' && c <= '9') {
            c -= '0';
        }
        else if (c >= 'a' && c <= 'f') {
            c -= 'a';
            c += 10;
        }
        else if (c >= 'A' && c <= 'F') {
            c -= 'A';
            c += 10;
        }
        else {
            v = 0;
            break;
        }
        v *= 16;
        v += c;
        x++;
    }

    return v;
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

## Pair `picoquic/config.c:config_atoi`
C: `picoquic/config.c:202-224 config_atoi`
Rust: `rs/fq/src/config.rs:592-595 config_atoi`

### C body
```c
{
    int v = 0;

    if (params == NULL || x < 0 || x >= nb_param) {
        *ret = -1;
    }
    else {
        for (size_t i = 0; i < params[x].length; i++) {
            int c = params[x].param[i] - '0';
            if (c < 0 || c > 9) {
                v = -1;
                *ret = -1;
                break;
            }
            else {
                v *= 10;
                v += c;
            }
        }
    }
    return v;
}
```

### Rust body
```rust
    if x >= params.len() {
        return Err(Error::InvalidArgument);
    }
```

## Pair `picoquic/config.c:picoquic_config_usage_file`
C: `picoquic/config.c:580-607 picoquic_config_usage_file`
Rust: `rs/fq/src/config.rs:1337-1347 write_usage`

### C body
```c
{
    fprintf(F, "Picoquic options:\n");
    for (size_t i = 0; i < option_table_size; i++) {
        size_t spacer = strlen(option_table[i].param_sample);
        fprintf(F, "  -%c %s", option_table[i].option_letter, option_table[i].param_sample);
        while (spacer++ < 12) {
            putc(' ', F);
        }
        fprintf(F, " %s\n", option_table[i].option_help);
        if (option_table[i].option_num == picoquic_option_CC_ALGO){
            if (picoquic_congestion_control_algorithms != NULL &&
                picoquic_nb_congestion_control_algorithms > 0) {
                /* Add a line with supported values. */
                for (size_t j = 0; j < 18; j++) {
                    putc(' ', F);
                }
                for (size_t k = 0; k < picoquic_nb_congestion_control_algorithms; k++) {
                    if (k != 0) {
                        fprintf(F, ", ");
                    }
                    fprintf(F, "%s", picoquic_congestion_control_algorithms[k]->congestion_algorithm_id);
                }
                fprintf(F, ".\n");
            }
        }
    }
}
```

### Rust body
```rust
    pub fn write_usage(w: &mut dyn core::fmt::Write) {
        let _ = w.write_str("Picoquic options:\n");
        for e in OPTION_TABLE {
            let _ = write!(w, "  -{} {}", e.letter, e.param_sample);
            let pad = 12usize.saturating_sub(e.param_sample.len());
            for _ in 0..pad {
                let _ = w.write_char(' ');
            }
            let _ = writeln!(w, " {}", e.help);
        }
    }
```

## Pair `picoquic/config.c:picoquic_config_get_option_name_index`
C: `picoquic/config.c:654-665 picoquic_config_get_option_name_index`
Rust: `rs/fq/src/config.rs:690-697 picoquic_config_get_option_name_index`

### C body
```c
{
    int option_index = -1;

    for (size_t i = 0; i < option_table_size; i++) {
        if (strncmp(s, option_table[i].option_name, l) == 0) {
            option_index = (int)i;
            break;
        }
    }
    return option_index;
}
```

### Rust body
```rust
pub fn picoquic_config_get_option_name_index(s: &str, l: usize) -> i32 {
    let l = l.min(s.len());
    let prefix = &s[..l];
    OPTION_TABLE
        .iter()
        .position(|e| e.name.len() >= l && &e.name[..l] == prefix)
        .map_or(-1, |i| i as i32)
}
```

## Pair `picoquic/config.c:picoquic_config_command_line_ex`
C: `picoquic/config.c:742-757 picoquic_config_command_line_ex`
Rust: `rs/fq/src/config.rs:1297-1307 command_line_ex`

### C body
```c
{
    int ret = 0;
    int option_index = -1;

    option_index = picoquic_config_get_command_line_option_index(opt_string);

    if (option_index == -1) {
        fprintf(stderr, "Unknown option: %s\n", opt_string);
    }
    else {
        ret = picoquic_get_command_line_option_value(option_index, opt_string, p_optind,
            argv, argc, optarg, config);
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let (_, entry) = parse_option_string(opt_string).ok_or(Error::InvalidArgument)?;
        let params = collect_params(entry, p_optind, argv, optarg)?;
        apply_option(self, entry, &params)
    }
```

## Pair `picoquic/cubic.c:cubic_reset`
C: `picoquic/cubic.c:53-68 cubic_reset`
Rust: `rs/fq/src/cubic.rs:83-101 reset`

### C body
```c
static void cubic_reset(picoquic_cubic_state_t* cubic_state, picoquic_path_t* path_x, uint64_t current_time) {
    memset(&cubic_state->rtt_filter, 0, sizeof(picoquic_min_max_rtt_t));
    memset(cubic_state, 0, sizeof(picoquic_cubic_state_t));
    path_x->cwin = PICOQUIC_CWIN_INITIAL;
    cubic_state->alg_state = picoquic_cubic_alg_slow_start;
    cubic_state->ssthresh = UINT64_MAX;
    cubic_state->W_last_max = (double)cubic_state->ssthresh / (double)path_x->send_mtu;
    cubic_state->W_max = cubic_state->W_last_max;
    cubic_state->start_of_epoch = current_time;
    cubic_state->previous_start_of_epoch = current_time;
    cubic_state->previous_alg_state = cubic_state->alg_state;
    cubic_state->previous_cwin = path_x->cwin;
    cubic_state->previous_ssthresh = UINT64_MAX;
    cubic_state->W_reno = PICOQUIC_CWIN_INITIAL;
    cubic_state->recovery_sequence = 0;
}
```

### Rust body
```rust
    pub fn reset(&mut self, path_x: &mut Path, current_time: u64) {
        let w_last_max = u64::MAX as f64 / path_x.send_mtu as f64;
        path_x.cwin = CWIN_INITIAL;
        *self = Self {
            alg_state: CubicAlgState::SlowStart,
            recovery_sequence: 0,
            start_of_epoch: current_time,
            previous_start_of_epoch: current_time,
            previous_alg_state: CubicAlgState::SlowStart as u64,
            previous_ssthresh: u64::MAX,
            previous_cwin: CWIN_INITIAL,
            k: 0.0,
            w_max: w_last_max,
            w_last_max,
            w_reno: CWIN_INITIAL as f64,
            ssthresh: u64::MAX,
            rtt_filter: MinMaxRtt::default(),
        };
    }
```

## Pair `picoquic/cubic.c:cubic_enter_avoidance`
C: `picoquic/cubic.c:132-142 cubic_enter_avoidance`
Rust: `rs/fq/src/cubic.rs:122-127 enter_avoidance`

### C body
```c
{
    cubic_state->K = cubic_root(cubic_state->W_max*(1.0 - PICOQUIC_CUBIC_BETA_ECN) / PICOQUIC_CUBIC_C);
    cubic_state->alg_state = picoquic_cubic_alg_congestion_avoidance;
    cubic_state->start_of_epoch = current_time;
    cubic_state->previous_start_of_epoch = cubic_state->start_of_epoch;
}
```

### Rust body
```rust
    pub fn enter_avoidance(&mut self, current_time: u64) {
        self.k = cubic_root(self.w_max * (1.0 - CUBIC_BETA_ECN) / CUBIC_C);
        self.alg_state = CubicAlgState::CongestionAvoidance;
        self.start_of_epoch = current_time;
        self.previous_start_of_epoch = self.start_of_epoch;
    }
```

## Pair `picoquic/cubic.c:dcubic_exit_slow_start`
C: `picoquic/cubic.c:424-456 dcubic_exit_slow_start`
Rust: `rs/fq/src/cubic.rs:431-458 dcubic_exit_slow_start`

### C body
```c
{
    if (cubic_state->ssthresh == UINT64_MAX) {
        path_x->is_ssthresh_initialized = 1;
        cubic_state->ssthresh = path_x->cwin;
        cubic_state->W_max = (double)path_x->cwin / (double)path_x->send_mtu;
        cubic_state->W_last_max = cubic_state->W_max;
        cubic_state->W_reno = ((double)path_x->cwin);
        cubic_enter_avoidance(cubic_state, current_time);
        /* apply a correction to enter the test phase immediately */
        uint64_t K_micro = (uint64_t)(cubic_state->K * 1000000.0);
        if (K_micro > current_time) {
            cubic_state->K = ((double)current_time) / 1000000.0;
            cubic_state->start_of_epoch = 0;
        }
        else {
            cubic_state->start_of_epoch = current_time - K_micro;
        }
    }
    else {
        if (current_time - cubic_state->start_of_epoch > path_x->smoothed_rtt ||
            cubic_state->recovery_sequence <= picoquic_cc_get_ack_number(cnx, path_x)) {
            /* re-enter recovery if this is a new event */
            cubic_enter_recovery(cnx, path_x, notification, cubic_state, current_time);
        }
    }
}
```

### Rust body
```rust
    ) {
        if self.ssthresh == u64::MAX {
            path_x.is_ssthresh_initialized = true;
            self.ssthresh = path_x.cwin;
            self.w_max = path_x.cwin as f64 / path_x.send_mtu as f64;
            self.w_last_max = self.w_max;
            self.w_reno = path_x.cwin as f64;
            self.enter_avoidance(current_time);
            // Apply a correction to enter the test phase immediately.
            let k_micro = (self.k * 1_000_000.0) as u64;
            if k_micro > current_time {
                self.k = current_time as f64 / 1_000_000.0;
                self.start_of_epoch = 0;
            } else {
                self.start_of_epoch = current_time - k_micro;
            }
        } else if current_time.wrapping_sub(self.start_of_epoch) > path_x.smoothed_rtt.ticks()
            || self.recovery_sequence <= connection.ack_number(path_x)
        {
            self.enter_recovery(connection, path_x, notification, current_time);
        }
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_dequeue_queue`
C: `picoquic/dualq_aqm.c:114-139 dualq_dequeue_queue`
Rust: `rs/fq/src/tests/dualq.rs:89-99 dequeue`

### C body
```c
{
    picoquictest_sim_packet_t* packet = xq->queue_first;

    if (packet != NULL) {
        xq->queue_first = packet->next_packet;
        packet->next_packet = NULL;
        if (xq->queue_first == NULL) {
            xq->queue_last = NULL;
            xq->queue_bytes = 0;
            xq->count = 0;
        }
        else
        {
            if (packet->length < xq->queue_bytes) {
                xq->queue_bytes -= packet->length;
            }
            else {
                /* error case. do not use 0, as that would stop dequeuing */
                xq->queue_bytes = 1;
            }
            xq->count -= 1;
        }
    }
    return packet;
}
```

### Rust body
```rust
    fn dequeue(&mut self) -> Option<TestSimPacket> {
        let packet = self.packets.pop_front()?;
        if self.packets.is_empty() {
            self.queue_bytes = 0;
        } else if (packet.length as u64) < self.queue_bytes {
            self.queue_bytes -= packet.length as u64;
        } else {
            self.queue_bytes = 1;
        }
        Some(packet)
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_laqm`
C: `picoquic/dualq_aqm.c:210-227 dualq_laqm`
Rust: `rs/fq/src/tests/dualq.rs:332-340 laqm`

### C body
```c
{
    double pprime = 0;
    uint64_t lq_time = 0;
    /* Returns Native L4S AQM probability */
    if (dualq->lq.count > 1) {
        if (dualq->lq.queue_first->arrival_time < current_time) {
            lq_time = current_time - dualq->lq.queue_first->arrival_time;
        }
        if (lq_time >= dualq->maxTh) {
            pprime = 1.0;
        }
        else if (lq_time > dualq->minTh) {
            pprime = ((double)(lq_time - dualq->minTh)) / dualq->range;
        }
    }
    return pprime;
}
```

### Rust body
```rust
        {
            lq_time = current_time.ticks() - packet.arrival_time.ticks();
        }
```

## Pair `picoquic/dualq_aqm.c:dualq_has_pending`
C: `picoquic/dualq_aqm.c:347-352 dualq_has_pending`
Rust: `rs/fq/src/tests/dualq.rs:246-254 has_pending`

### C body
```c
{
    dualq_state_t* dualq = (dualq_state_t*)self;

    return (dualq->lq.queue_first != NULL || dualq->cq.queue_first != NULL);
}
```

### Rust body
```rust
    fn admit_pending(&mut self, link: &mut TestSimLink, current_time: Instant) {
        self.update_it(link, current_time);
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_reset`
C: `picoquic/dualq_aqm.c:390-394 dualq_reset`
Rust: `rs/fq/src/tests/dualq.rs:227-229 reset`

### C body
```c
{
    dualq_state_t* dualq = (dualq_state_t*)self;
    dualq_update_it(dualq, link, current_time);
}
```

### Rust body
```rust
    fn reset(&mut self, link: &mut TestSimLink, current_time: Instant) {
        self.update_it(link, current_time);
    }
```

## Pair `picoquic/ech.c:picoquic_base64_encode`
C: `picoquic/ech.c:76-93 picoquic_base64_encode`
Rust: `rs/fq/src/ech.rs:103-139 base64_encode`

### C body
```c
{

    int ret = 0;
    size_t len = ptls_base64_howlong(v_len);
    *b64_len = len;
    if (len + 1 > b64_size) {
        ret = -1;
    }
    else {
        (void)ptls_base64_encode(v, v_len, b64);
    }
    return ret;
}
```

### Rust body
```rust
pub fn base64_encode(input: &[u8], out: &mut [u8]) -> Result<usize, Error> {
    let len = base64_encoded_len(input.len());
    if len + 1 > out.len() {
        return Err(Error::Generic);
    }
    let mut pos = 0usize;
    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        let b = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out[pos] = BASE64_ALPHABET[((b >> 18) & 0x3f) as usize];
        out[pos + 1] = BASE64_ALPHABET[((b >> 12) & 0x3f) as usize];
        out[pos + 2] = BASE64_ALPHABET[((b >> 6) & 0x3f) as usize];
        out[pos + 3] = BASE64_ALPHABET[(b & 0x3f) as usize];
        pos += 4;
    }
    match chunks.remainder() {
        [a] => {
            let b = (*a as u32) << 16;
            out[pos] = BASE64_ALPHABET[((b >> 18) & 0x3f) as usize];
            out[pos + 1] = BASE64_ALPHABET[((b >> 12) & 0x3f) as usize];
            out[pos + 2] = b'=';
            out[pos + 3] = b'=';
            pos += 4;
        }
        [a, b] => {
            let v = ((*a as u32) << 16) | ((*b as u32) << 8);
            out[pos] = BASE64_ALPHABET[((v >> 18) & 0x3f) as usize];
            out[pos + 1] = BASE64_ALPHABET[((v >> 12) & 0x3f) as usize];
            out[pos + 2] = BASE64_ALPHABET[((v >> 6) & 0x3f) as usize];
            out[pos + 3] = b'=';
            pos += 4;
        }
        _ => {}
    }
    out[pos] = 0; // NUL terminator (mirrors ptls_base64_encode convention)
    Ok(len)
}
```

## Pair `picoquic/ech.c:ech_init_opener_callback`
C: `picoquic/ech.c:280-331 ech_init_opener_callback`
Rust: `rs/fq/src/ech.rs:880-923 ech_init_opener`

### C body
```c
{
    int ret = 0;
    /* Allocate an opener callback */
    ech_opener_callback_t* ech_cb = (ech_opener_callback_t*)malloc(sizeof(ech_opener_callback_t));
    if (ech_cb == NULL) {
        DBG_PRINTF("Cannot allocate callback memory (%zu bytes)", sizeof(ech_opener_callback_t));
        ret = PICOQUIC_ERROR_MEMORY;
    }
    else {
        memset(ech_cb, 0, sizeof(ech_opener_callback_t));
        /* set the callback */
        ech_cb->super.cb = ech_opener_callback;
        ptls_buffer_init(&ech_cb->config, "", 0);
        /* Read the config bytes into the ech_cb->config buffer */
        ret = picoquic_ech_read_config(&ech_cb->config, config_file_name);
        if (ret != 0) {
            DBG_PRINTF("Cannot read ech configuration from %s", config_file_name);
        } else {
            uint16_t kem_id;
            /* Get kem-id from config, then get kem from kem_id */
            kem_id = (((uint16_t)ech_cb->config.base[7]) << 8) + ech_cb->config.base[8];
            for (int i = 0; i < 4 && picoquic_hpke_kems[i] !=  NULL; i++) {
                if (picoquic_hpke_kems[i]->id == kem_id) {
                    ech_cb->kem = picoquic_hpke_kems[i];
                    break;
                }
            }
            if (ech_cb->kem == NULL){
                DBG_PRINTF("Cannot find hpke kwm for code 0x%04x", kem_id);
                ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
            }
            else if (picoquic_keyex_from_key_file_fn == NULL) {
                DBG_PRINTF("%s", "Cannot find picoquic_keyex_from_key_file_fn");
                ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
            }
            else {
                ret = picoquic_keyex_from_key_file_fn(&ech_cb->keyex, private_key_file);
                if (ret != 0) {
                    DBG_PRINTF("picoquic_keyex_from_key_file_fn fails, ret= %d(0x%x)", ret, ret);
                }
            }
        }
        if (ret != 0) {
            ech_dispose_opener_callback(ech_cb);
            ech_cb = NULL;
        }
    }
    *p_ech_cb = ech_cb;
    return ret;
}
```

### Rust body
```rust
) -> Result<EchOpenerState, Error> {
    // Read and decode the ECHConfigList from the base64-encoded config file.
    let config = ech_read_config_file(config_file_name)?;

    // Extract kem_id from ECHConfigList bytes[7:9]:
    //   bytes[0:2] = ECHConfigList outer length
    //   bytes[2:4] = ECHConfig version (0xFE0D)
    //   bytes[4:6] = ECHConfig contents length
    //   bytes[6]   = config_id
    //   bytes[7:9] = kem_id
    if config.len() < 9 {
        return Err(Error::Generic);
    }
    let kem_id = (config[7] as u16) << 8 | config[8] as u16;

    // Validate the kem_id against the supported set.
    if !matches!(
        kem_id,
        kem_id::P256_SHA256 | kem_id::P384_SHA384 | kem_id::X25519_SHA256
    ) {
        return Err(Error::Generic);
    }

    // Read and parse the PEM private key file.
    let pem_text = std::fs::read_to_string(private_key_file).map_err(|_| Error::Generic)?;
    // Determine the key format from the PEM label.
    let label_start = pem_text.find("-----BEGIN ").ok_or(Error::Generic)? + "-----BEGIN ".len();
    let label_end = pem_text[label_start..]
        .find("-----")
        .ok_or(Error::Generic)?;
    let label = &pem_text[label_start..label_start + label_end];

    let der = pem_base64_decode(&pem_text)?;
    let private_key = extract_private_key_der(&der, label)?;

    Ok(EchOpenerState {
        kem_id,
        private_key,
        config,
    })
}
```

## Pair `picoquic/ech.c:picoquic_is_ech_handshake`
C: `picoquic/ech.c:415-422 picoquic_is_ech_handshake`
Rust: `rs/fq/src/lib.rs:4722-4733 is_ech_handshake`

### C body
```c
{
    picoquic_tls_ctx_t* tls_ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return ptls_is_ech_handshake(tls_ctx->tls, NULL, NULL, NULL);
}
```

### Rust body
```rust
    pub fn ech_retry_config(&self) -> &[u8] {
        self.tls_ctx.as_ref().map_or(&[], |s| s.retry_configs())
    }
```

## Pair `picoquic/ech.c:picoquic_ech_get_ciphers_from_kem`
C: `picoquic/ech.c:672-736 picoquic_ech_get_ciphers_from_kem`
Rust: `rs/fq/src/ech.rs:156-162 ech_get_ciphers_from_kem`

### C body
```c
{
    int ret = 0;
    size_t nb_ciphers = 0;
    uint16_t target_kdf_id = PTLS_HPKE_HKDF_SHA256;
    uint16_t target_aead_id = PTLS_HPKE_AEAD_AES_128_GCM;
    ptls_hpke_cipher_suite_t* target_cipher = NULL;
    ptls_hpke_cipher_suite_t* default_cipher = NULL;

    if (cipher_vec_nb_max < 2) {
        return -1;
    }

    switch (kem_id) {
    case PTLS_HPKE_KEM_P256_SHA256:
        target_kdf_id = PTLS_HPKE_HKDF_SHA256;
        target_aead_id = PTLS_HPKE_AEAD_AES_128_GCM;
        break;
    case PTLS_HPKE_KEM_P384_SHA384:
        target_kdf_id = PTLS_HPKE_HKDF_SHA384;
        target_aead_id = PTLS_HPKE_AEAD_AES_256_GCM;
        break;
    case PTLS_HPKE_KEM_X25519_SHA256:
        target_kdf_id = PTLS_HPKE_HKDF_SHA256;
        target_aead_id = PTLS_HPKE_AEAD_CHACHA20POLY1305;
        break;
    default:
        break;
    }

    for (size_t i = 0; i < PICOQUIC_HPKE_CIPHER_SUITE_NB_MAX; i++) {
        if (picoquic_hpke_cipher_suites[i] == NULL) {
            break;
        }
        else if (picoquic_hpke_cipher_suites[i]->id.aead == target_aead_id &&
            picoquic_hpke_cipher_suites[i]->id.kdf == target_kdf_id) {
            target_cipher = picoquic_hpke_cipher_suites[i];
            break;
        }
        else if (picoquic_hpke_cipher_suites[i]->id.aead == PTLS_HPKE_AEAD_AES_128_GCM &&
            picoquic_hpke_cipher_suites[i]->id.kdf == PTLS_HPKE_HKDF_SHA256) {
            default_cipher = picoquic_hpke_cipher_suites[i];
        }
    }
    if (target_cipher == NULL) {
        if (default_cipher != NULL) {
            cipher_vec[0] = default_cipher;
            nb_ciphers = 1;
        }
        else {
            ret = -1;
        }
    } else {
        cipher_vec[0] = target_cipher;
        nb_ciphers = 1;
        if (default_cipher != NULL && default_cipher != target_cipher && cipher_vec_nb_max > 2) {
            cipher_vec[1] = default_cipher;
            nb_ciphers++;
        }
    }
    for (size_t i = nb_ciphers; i < cipher_vec_nb_max; i++) {
        cipher_vec[i] = NULL;
    }
    return ret;
}
```

### Rust body
```rust
    if out.len() < 2 {
        return Err(Error::Generic);
    }
```

## Pair `picoquic/ech.c:picoquic_ech_create_rr_from_binary`
C: `picoquic/ech.c:827-842 picoquic_ech_create_rr_from_binary`
Rust: `rs/fq/src/ech.rs:1019-1025 ech_create_rr_from_binary`

### C body
```c
{
    int ret = 0;
    ptls_iovec_t public_key_bits = ptls_iovec_init(NULL, 0);
    uint16_t group_id = 0;

    /* Parse the ASN1 public key to extract the key type and the key bytes */
    if ((ret = picoquic_ech_parse_public_key(public_key_asn1, &group_id, &public_key_bits)) != 0) {
        DBG_PRINTF("Cannot get group and pubkey from ASN1, err: %x", ret);
    }
    /* Build config from group id and key bits */
    else {
        ret = picoquic_ech_create_config_from_pk(config_buf, group_id, public_key_bits, public_name);
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<Vec<u8>, Error> {
    let (group_id, public_key_bits) = ech_parse_public_key(public_key_asn1)?;
    ech_create_config_from_pk(group_id, public_key_bits, public_name)
}
```

## Pair `picoquic/ech.c:picoquic_ech_create_config_file`
C: `picoquic/ech.c:927-940 picoquic_ech_create_config_file`
Rust: `rs/fq/src/lib.rs:4737-4745 ech_create_config_file`

### C body
```c
{
    uint8_t* config = NULL;
    size_t config_len = 0;
    int ret = picoquic_ech_create_config_from_private_key(&config, &config_len, private_key_file, public_name);

    if (ret == 0) {
        ret = picoquic_ech_save_config(config, config_len, ech_config_file);
    }
    if (config != NULL) {
        free(config);
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    let config =
        crate::ech::ech_create_config_from_private_key_file(_private_key_file, _public_name)?;
    crate::ech::ech_save_config_file(&config, _ech_config_file)
}
```
