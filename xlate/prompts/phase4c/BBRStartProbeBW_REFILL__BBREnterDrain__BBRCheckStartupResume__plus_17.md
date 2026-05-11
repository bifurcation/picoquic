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

## Pair `picoquic/bbr.c:BBRStartProbeBW_REFILL`
C: `picoquic/bbr.c:1823-1835 BBRStartProbeBW_REFILL`
Rust: `rs/fq/src/bbr.rs:1572-1585 start_probe_bw_refill`

### C body
```c
{
    bbr_state->pacing_gain = BBRProbeBwRefillPacingGain;  /* pace at rate */
    bbr_state->cwnd_gain = BBRProbeBwRefillCwndGain;   /* maintain cwnd */
    BBRResetLowerBounds(bbr_state);
    bbr_state->bw_probe_up_rounds = 0;
    bbr_state->bw_probe_up_acks = 0;
    bbr_state->full_bw = bbr_state->max_bw;
    bbr_state->ack_phase = picoquic_bbr_acks_refilling;
    BBRStartRound(bbr_state, path_x);
    bbr_state->state = picoquic_bbr_alg_probe_bw_refill;
    path_x->is_cca_probing_up = 1;
}
```

### Rust body
```rust
    fn start_probe_bw_refill(&mut self, connection: &Connection, path_x: &mut Path) {
        const BBR_PROBE_BW_REFILL_PACING_GAIN: f64 = 1.0;
        const BBR_PROBE_BW_REFILL_CWND_GAIN: f64 = 2.0;
        self.pacing_gain = BBR_PROBE_BW_REFILL_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_REFILL_CWND_GAIN;
        self.reset_lower_bounds();
        self.bw_probe_up_rounds = 0;
        self.bw_probe_up_acks = 0;
        self.full_bw = self.max_bw;
        self.ack_phase = BbrAckPhase::Refilling;
        self.start_round(connection, path_x);
        self.state = BbrAlgState::ProbeBwRefill;
        path_x.is_cca_probing_up = true;
    }
```

## Pair `picoquic/bbr.c:BBREnterDrain`
C: `picoquic/bbr.c:1932-1941 BBREnterDrain`
Rust: `rs/fq/src/bbr.rs:383-390 enter_drain`

### C body
```c
{
    path_x->is_ssthresh_initialized = 1; /* Picoquic specific: notify transport that the startup phase is complete */
    bbr_state->state = picoquic_bbr_alg_drain;
    bbr_state->pacing_gain = 1.0 / BBRStartupCwndGain;  /* pace slowly */
    bbr_state->cwnd_gain = BBRStartupCwndGain;   /* maintain cwnd */

    path_x->is_cca_probing_up = 0;
}
```

### Rust body
```rust
    pub fn enter_drain(&mut self, path_x: &mut Path) {
        // Picoquic-specific: notify transport that the startup phase is complete.
        path_x.is_ssthresh_initialized = true;
        self.state = BbrAlgState::Drain;
        self.pacing_gain = 1.0 / BBR_STARTUP_CWND_GAIN; // pace slowly
        self.cwnd_gain = BBR_STARTUP_CWND_GAIN; // maintain cwnd
        path_x.is_cca_probing_up = false;
    }
```

## Pair `picoquic/bbr.c:BBRCheckStartupResume`
C: `picoquic/bbr.c:1982-2000 BBRCheckStartupResume`
Rust: `rs/fq/src/bbr.rs:1691-1711 check_startup_resume`

### C body
```c
{
    if (bbr_state->state == picoquic_bbr_alg_startup_resume) {
        BBRCheckStartupHighLoss(bbr_state, path_x, rs);
        if (!bbr_state->filled_pipe && (double)bbr_state->max_bw > BBRStartupResumeIncreaseThreshold * bbr_state->bdp_seed) {
            BBREnterStartup(bbr_state, path_x);
        }
        else {
            BBRCheckStartupFullBandwidthGeneric(bbr_state, rs, BBRStartupResumeIncreaseThreshold);
            if (bbr_state->filled_pipe) {
                if (bbr_state->full_bw_count > 0) {
                    bbr_state->probe_probe_bw_quickly = 1;
                    bbr_state->full_bw_count = 0;
                }
                BBREnterDrain(bbr_state, path_x);
            }
        }
    }
}
```

### Rust body
```rust
    pub(crate) fn check_startup_resume(&mut self, path_x: &mut Path, rs: &BbrPerAckState) {
        const BBR_STARTUP_RESUME_INCREASE_THRESHOLD: f64 = 1.125;
        if self.state != BbrAlgState::StartupResume {
            return;
        }
        self.check_startup_high_loss(path_x, rs);
        if !self.filled_pipe
            && self.max_bw as f64 > BBR_STARTUP_RESUME_INCREASE_THRESHOLD * self.bdp_seed as f64
        {
            self.enter_startup(path_x);
        } else {
            self.check_startup_full_bandwidth_generic(rs, BBR_STARTUP_RESUME_INCREASE_THRESHOLD);
            if self.filled_pipe {
                if self.full_bw_count > 0 {
                    self.probe_probe_bw_quickly = true;
                    self.full_bw_count = 0;
                }
                self.enter_drain(path_x);
            }
        }
    }
```

## Pair `picoquic/bbr.c:BBREnterStartup`
C: `picoquic/bbr.c:2061-2067 BBREnterStartup`
Rust: `rs/fq/src/bbr.rs:487-519 enter_startup`

### C body
```c
{
    bbr_state->state = picoquic_bbr_alg_startup;
    bbr_state->pacing_gain = BBRStartupPacingGain;
    bbr_state->cwnd_gain = BBRStartupCwndGain;
    path_x->is_cca_probing_up = 1;
}
```

### Rust body
```rust
    pub fn enter_startup_long_rtt(&mut self, path_x: &mut Path) {
        let mut cwnd = CWIN_INITIAL;
        self.state = BbrAlgState::StartupLongRtt;
        if path_x.rtt_min > TARGET_RENO_RTT {
            let rtt_cap = if path_x.rtt_min > TARGET_SATELLITE_RTT {
                TARGET_SATELLITE_RTT
            } else {
                path_x.rtt_min
            };
            cwnd = (cwnd as f64 * rtt_cap.ticks() as f64 / TARGET_RENO_RTT.ticks() as f64) as u64;
        }
        if cwnd < self.bdp_seed {
            cwnd = self.bdp_seed;
        }
        if cwnd > path_x.cwin {
            path_x.cwin = cwnd;
        }
        path_x.is_cca_probing_up = true;
    }
```

## Pair `picoquic/bbr.c:BBRCheckStartupLongRtt`
C: `picoquic/bbr.c:2128-2153 BBRCheckStartupLongRtt`
Rust: `rs/fq/src/bbr.rs:1648-1660 check_startup_long_rtt`

### C body
```c
{
    if ((bbr_state->state == picoquic_bbr_alg_startup ||
        bbr_state->state == picoquic_bbr_alg_startup_resume) &&
        path_x->rtt_min > BBRLongRttThreshold) {
        BBREnterStartupLongRTT(bbr_state, path_x);
    }
    else if (bbr_state->state != picoquic_bbr_alg_startup_long_rtt) {
        return;
    }

    if (picoquic_cc_hystart_test(&bbr_state->rtt_filter, rs->rtt_sample,
        path_x->pacing.packet_time_microsec, current_time, 0)) {
        BBRExitStartupLongRtt(bbr_state, path_x, current_time);
    }
    else if (rs->ecn_alpha > BBRExcessiveEcnCE) {
        BBRExitStartupLongRtt(bbr_state, path_x, current_time);
    }
    else {
        int excessive_loss = picoquic_cc_hystart_loss_volume_test(&bbr_state->rtt_filter, picoquic_congestion_notification_repeat,
            rs->newly_acked, rs->newly_lost);
        if (excessive_loss) {
            BBRExitStartupLongRtt(bbr_state, path_x, current_time);
        }
    }
}
```

### Rust body
```rust
        {
            self.enter_startup_long_rtt(path_x);
        } else if self.state != BbrAlgState::StartupLongRtt {
```

## Pair `picoquic/bbr.c:BBRAccessEcnPacketContext`
C: `picoquic/bbr.c:2244-2260 BBRAccessEcnPacketContext`
Rust: `rs/fq/src/bbr.rs:2421-2434 access_ecn_packet_context`

### C body
```c
{
    /* TODO: ECN counts should be a function of path, not number space! */
    picoquic_packet_context_t* pkt_ctx = &path_x->cnx->pkt_ctx[picoquic_packet_context_application];

    if (path_x->cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }
    else if (path_x != path_x->cnx->path[0]) {
        /* When doing simple multipath, or when preparing transitions,
         * only consider the default path */
        pkt_ctx = NULL;
    }

    return pkt_ctx;
}
```

### Rust body
```rust
) -> Option<&'a PacketContextState> {
    if connection.is_multipath_enabled {
        Some(&path_x.pkt_ctx)
    } else if path_x.unique_path_id != 0 {
        // Non-default path in single-path mode: ECN counts are not
        // tracked per-path here; the caller should use the default path.
        None
    } else {
        Some(&connection.pkt_ctx[PacketContext::Application as usize])
    }
}
```

## Pair `picoquic/bbr.c:BBRUpdateControlParameters`
C: `picoquic/bbr.c:2328-2333 BBRUpdateControlParameters`
Rust: `rs/fq/src/bbr.rs:2101-2105 update_control_parameters`

### C body
```c
{
    BBRSetPacingRate(bbr_state);
    BBRSetSendQuantum(bbr_state, path_x);
    BBRSetCwnd(bbr_state, path_x, rs);
}
```

### Rust body
```rust
    pub(crate) fn update_control_parameters(&mut self, path_x: &mut Path, rs: &BbrPerAckState) {
        self.set_pacing_rate();
        self.set_send_quantum(path_x);
        self.set_cwnd(path_x, rs);
    }
```

## Pair `picoquic/bbr.c:picoquic_bbr_notify`
C: `picoquic/bbr.c:2400-2464 picoquic_bbr_notify`
Rust: `rs/fq/src/bbr.rs:2350-2406 notify`

### C body
```c
{
    picoquic_bbr_state_t* bbr_state = (picoquic_bbr_state_t*)path_x->congestion_alg_state;
    path_x->is_cc_data_updated = 1;

    if (bbr_state != NULL) {
        switch (notification) {
        case picoquic_congestion_notification_ecn_ec:
            /* TODO */
            break;
        case picoquic_congestion_notification_repeat:
            BBRUpdateRecoveryOnLoss(bbr_state, path_x, ack_state->nb_bytes_newly_lost);
            break;
        case picoquic_congestion_notification_timeout:
            BBRExitLostFeedback(bbr_state, path_x);
            /* if loss is PTO, we should start the OnPto processing */
            BBROnEnterRTO(bbr_state, path_x, ack_state->lost_packet_number);
            break;
        case picoquic_congestion_notification_spurious_repeat:
            /* handling of suspension */
            BBROnSpuriousLoss(bbr_state, path_x, ack_state->lost_packet_number, current_time);
            break;
        case picoquic_congestion_notification_lost_feedback:
            /* Feedback has been lost. It will be restored at the next notification. */
            BBRExpGate(bbr_state, do_control_lost, break);
            BBREnterLostFeedback(bbr_state, path_x);
            break;
        case picoquic_congestion_notification_rtt_measurement:
            /* TODO: this call is subsumed by the acknowledgement notification.
             * Consider removing it from the API once other CC algorithms are updated.  */
            break;
        case picoquic_congestion_notification_acknowledgement:
            BBRExitLostFeedback(bbr_state, path_x);
            picoquic_bbr_notify_ack(bbr_state, path_x, ack_state, current_time);
            if (bbr_state->state == picoquic_bbr_alg_startup_long_rtt) {
                picoquic_update_pacing_data(path_x, 1);
            }
            else if (bbr_state->pacing_rate > 0) {
                /* Set the pacing rate in picoquic sender */
                picoquic_update_pacing_rate(path_x, bbr_state->pacing_rate, bbr_state->send_quantum);
            }
            break;
        case picoquic_congestion_notification_cwin_blocked:
            break;
        case picoquic_congestion_notification_reset:
            picoquic_bbr_reset(bbr_state, path_x, current_time);
            break;
        case picoquic_congestion_notification_seed_cwin:
            BBRSetBdpSeed(bbr_state, ack_state->nb_bytes_acknowledged);
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
            CongestionNotification::EcnEc => {
                // C leaves ECN-EC handling as an explicit no-op.
            }
            CongestionNotification::Repeat => {
                self.update_recovery_on_loss(path_x, ack_state.nb_bytes_newly_lost);
            }
            CongestionNotification::Timeout => {
                self.exit_lost_feedback(path_x);
                self.on_enter_rto(path_x, ack_state.lost_packet_number);
            }
            CongestionNotification::SpuriousRepeat => {
                self.on_spurious_loss(
                    connection,
                    path_x,
                    ack_state.lost_packet_number,
                    current_time,
                );
            }
            CongestionNotification::LostFeedback => {
                // BBRExpGate: only enter if do_control_lost experiment is enabled.
                if self.exp_flags.do_control_lost {
                    self.enter_lost_feedback(connection, path_x);
                }
            }
            CongestionNotification::RttMeasurement => {
                // Subsumed by the Acknowledgement notification; no-op.
            }
            CongestionNotification::Acknowledgement => {
                self.exit_lost_feedback(path_x);
                self.notify_ack(connection, path_x, ack_state, current_time);
                if self.state == BbrAlgState::StartupLongRtt {
                    path_x.update_pacing_data(1);
                } else if self.pacing_rate > 0.0 {
                    path_x.update_pacing_rate(self.pacing_rate, self.send_quantum);
                }
            }
            CongestionNotification::CwinBlocked => {}
            CongestionNotification::Reset => {
                let opt = self.option_string.take();
                self.on_init(connection, path_x, current_time, opt);
            }
            CongestionNotification::SeedCwin => {
                self.set_bdp_seed(ack_state.nb_bytes_acknowledged);
            }
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1EnterStartup`
C: `picoquic/bbr1.c:327-332 BBR1EnterStartup`
Rust: `rs/fq/src/bbr1.rs:179-203 enter_startup`

### C body
```c
{
    bbr1_state->state = picoquic_bbr1_alg_startup;
    bbr1_state->pacing_gain = BBR1_HIGH_GAIN;
    bbr1_state->cwnd_gain = BBR1_HIGH_GAIN;
}
```

### Rust body
```rust
    pub fn check_full_pipe(&mut self, rs_is_app_limited: bool) {
        if !self.filled_pipe && self.round_start && !rs_is_app_limited {
            if self.btl_bw as f64 >= self.full_bw as f64 * 1.25 {
                self.full_bw = self.btl_bw;
                self.full_bw_count = 0;
            } else {
                self.full_bw_count += 1;
                if self.full_bw_count >= 3 {
                    self.filled_pipe = true;
                }
            }
        }
    }
```

## Pair `picoquic/bbr1.c:picoquic_bbr1_set_options`
C: `picoquic/bbr1.c:373-447 picoquic_bbr1_set_options`
Rust: `rs/fq/src/bbr1.rs:430-485 set_options`

### C body
```c
{
    const char* x = bbr1_state->option_string;

    if (x != NULL) {
        char c;
        while ((c = *x) != 0) {
            x++;
            switch (c) {
            case 'T': {
                /* Reading digits into an uint64_t  */
                uint64_t u = 0;
                while ((c = *x) != 0) {
                    if (c >= '0' && c <= '9') {
                        u *= 10;
                        u += c - '0';
                        x++;
                    }
                    else {
                        break;
                    }
                }
                bbr1_state->wifi_shadow_rtt = u;
                break;
            }
            case 'Q': {
                /* Reading digits ad one dot into a double  */
                while ((c = *x) != 0) {
                    double d = 0;
                    double div = 1.0;
                    int dotted = 0;
                    while ((c = *x) != 0) {
                        if (c >= '0' && c <= '9') {
                            if (!dotted) {
                                d *= 10;
                                d += c - '0';
                            }
                            else {
                                div /= 10.0;
                                d += div * (c - '0');
                            }
                            x++;
                        }
                        else if (c == '.') {
                            if (dotted) {
                                break;
                            }
                            else {
                                dotted = 1;
                                x++;
                            }
                        }
                        else {
                            break;
                        }
                    }
                    bbr1_state->quantum_ratio = d;
                    break;
                }
            }
            case ':':
                /* Ignore */
                break;
            default:
                break;
            }
        }
    }
}
```

### Rust body
```rust
    fn set_options(&mut self) {
        let opt = match &self.option_string {
            Some(s) => s.clone(),
            None => return,
        };
        let mut chars = opt.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                'T' => {
                    let mut u: u64 = 0;
                    while let Some(&d) = chars.peek() {
                        if d.is_ascii_digit() {
                            u = u * 10 + (d as u64 - '0' as u64);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    self.wifi_shadow_rtt = u;
                }
                'Q' => {
                    // The C outer `while` is an `if` — executes at most once.
                    if chars.peek().is_none() {
                        continue;
                    }
                    let mut d: f64 = 0.0;
                    let mut div: f64 = 1.0;
                    let mut dotted = false;
                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_digit() {
                            let digit = ch as u64 - '0' as u64;
                            if !dotted {
                                d = d * 10.0 + digit as f64;
                            } else {
                                div /= 10.0;
                                d += div * digit as f64;
                            }
                            chars.next();
                        } else if ch == '.' {
                            if dotted {
                                break;
                            } else {
                                dotted = true;
                                chars.next();
                            }
                        } else {
                            break;
                        }
                    }
                    self.quantum_ratio = d;
                    // C falls through to ':' (ignore) here; no extra action needed.
                }
                _ => { /* ignore separator or unknown key */ }
            }
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1ltbwResetSampling`
C: `picoquic/bbr1.c:503-509 BBR1ltbwResetSampling`
Rust: `rs/fq/src/bbr1.rs:550-555 ltbw_reset_sampling`

### C body
```c
{
    bbr1_state->lt_bw = 0;
    bbr1_state->lt_use_bw = 0;
    bbr1_state->lt_is_sampling = 0;
    BBR1ltbwResetInterval(bbr1_state, path_x, current_time);
}
```

### Rust body
```rust
    fn ltbw_reset_sampling(&mut self, path_x: &Path, current_time: u64) {
        self.lt_bw = 0;
        self.lt_use_bw = false;
        self.lt_is_sampling = false;
        self.ltbw_reset_interval(path_x, current_time);
    }
```

## Pair `picoquic/bbr1.c:BBR1UpdateRTprop`
C: `picoquic/bbr1.c:679-696 BBR1UpdateRTprop`
Rust: `rs/fq/src/bbr1.rs:367-379 update_rt_prop`

### C body
```c
{
    bbr1_state->rt_prop_expired =
        current_time > bbr1_state->rt_prop_stamp + BBR1_PROBE_RTT_INTERVAL &&
        current_time > bbr1_state->rt_prop_stamp + 20 * bbr1_state->rt_prop;
    if (rtt_sample <= bbr1_state->rt_prop || bbr1_state->rt_prop_expired) {
        bbr1_state->rt_prop = rtt_sample;
        bbr1_state->rt_prop_stamp = current_time;
    }
    else {
        uint64_t delta = rtt_sample - bbr1_state->rt_prop;
        if (20 * delta < bbr1_state->rt_prop) {
            bbr1_state->rt_prop_stamp = current_time;
        }
    }
}
```

### Rust body
```rust
    pub fn update_rt_prop(&mut self, rtt_sample: u64, current_time: u64) {
        self.rt_prop_expired = current_time > self.rt_prop_stamp + BBR1_PROBE_RTT_INTERVAL
            && current_time > self.rt_prop_stamp + 20 * self.rt_prop;
        if rtt_sample <= self.rt_prop || self.rt_prop_expired {
            self.rt_prop = rtt_sample;
            self.rt_prop_stamp = current_time;
        } else {
            let delta = rtt_sample - self.rt_prop;
            if 20 * delta < self.rt_prop {
                self.rt_prop_stamp = current_time;
            }
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1CheckCyclePhase`
C: `picoquic/bbr1.c:756-762 BBR1CheckCyclePhase`
Rust: `rs/fq/src/bbr1.rs:889-894 check_cycle_phase`

### C body
```c
{
    if (bbr1_state->state == picoquic_bbr1_alg_probe_bw &&
        BBR1IsNextCyclePhase(bbr1_state, bbr1_state->prior_in_flight, packets_lost, current_time)) {
        BBR1AdvanceCyclePhase(bbr1_state, current_time);
    }
}
```

### Rust body
```rust
            if self.is_next_cycle_phase(prior, packets_lost, current_time) {
                self.advance_cycle_phase(current_time);
            }
```

## Pair `picoquic/bbr1.c:BBR1EnterDrain`
C: `picoquic/bbr1.c:814-822 BBR1EnterDrain`
Rust: `rs/fq/src/bbr1.rs:770-776 enter_drain`

### C body
```c
{
    path_x->is_ssthresh_initialized = 1;
    bbr1_state->state = picoquic_bbr1_alg_drain;
    bbr1_state->pacing_gain = 1.0 / BBR1_HIGH_GAIN;  /* pace slowly */
    bbr1_state->cwnd_gain = BBR1_HIGH_GAIN;   /* maintain cwnd */
    /* Start sampling */
    BBR1ltbwSampling(bbr1_state, path_x, current_time);
}
```

### Rust body
```rust
    fn enter_drain(&mut self, path_x: &mut Path, current_time: u64) {
        path_x.is_ssthresh_initialized = true;
        self.state = Bbr1AlgState::Drain;
        self.pacing_gain = 1.0 / BBR1_HIGH_GAIN;
        self.cwnd_gain = BBR1_HIGH_GAIN;
        self.ltbw_sampling(path_x, current_time);
    }
```

## Pair `picoquic/bbr1.c:BBR1EnterProbeRTT`
C: `picoquic/bbr1.c:885-890 BBR1EnterProbeRTT`
Rust: `rs/fq/src/bbr1.rs:209-221 enter_probe_rtt`

### C body
```c
{
    bbr1_state->state = picoquic_bbr1_alg_probe_rtt;
    bbr1_state->pacing_gain = 1.0;
    bbr1_state->cwnd_gain = 1.0;
}
```

### Rust body
```rust
    pub fn after_one_roundtrip_in_fast_recovery(&mut self) {
        self.packet_conservation = false;
    }
```

## Pair `picoquic/bbr1.c:BBR1RestoreCwnd`
C: `picoquic/bbr1.c:918-923 BBR1RestoreCwnd`
Rust: `rs/fq/src/bbr1.rs:306-333 restore_cwnd`

### C body
```c
{
    if (path_x->cwin < bbr1_state->prior_cwnd) {
        path_x->cwin = bbr1_state->prior_cwnd;
    }
}
```

### Rust body
```rust
    ) {
        if bytes_lost > 0 {
            if path_x.cwin > bytes_lost {
                path_x.cwin -= bytes_lost;
            } else {
                path_x.cwin = path_x.send_mtu as u64;
            }
        }
        if self.packet_conservation && path_x.cwin < bytes_in_transit + bytes_delivered {
            path_x.cwin = bytes_in_transit + bytes_delivered;
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1SetPacingRateWithGain`
C: `picoquic/bbr1.c:982-989 BBR1SetPacingRateWithGain`
Rust: `rs/fq/src/bbr1.rs:846-851 set_pacing_rate_with_gain`

### C body
```c
{
    double rate = pacing_gain * (double)BBR1GetBtlBW(bbr1_state);

    if (bbr1_state->filled_pipe || rate > bbr1_state->pacing_rate){
        bbr1_state->pacing_rate = rate;
    }
}
```

### Rust body
```rust
    pub fn set_pacing_rate_with_gain(&mut self, pacing_gain: f64) {
        let rate = pacing_gain * self.get_btl_bw() as f64;
        if self.filled_pipe || rate > self.pacing_rate {
            self.pacing_rate = rate;
        }
    }
```

## Pair `picoquic/bbr1.c:BBR1SetCwnd`
C: `picoquic/bbr1.c:1025-1047 BBR1SetCwnd`
Rust: `rs/fq/src/bbr1.rs:806-830 set_cwnd`

### C body
```c
{
    BBR1UpdateTargetCwnd(bbr1_state);
    BBR1ModulateCwndForRecovery(bbr1_state, path_x, bytes_in_transit, packets_lost, bytes_delivered);
    if (!bbr1_state->packet_conservation) {
        if (bbr1_state->filled_pipe) {
            path_x->cwin += bytes_delivered;
            if (path_x->cwin > bbr1_state->target_cwnd) {
                path_x->cwin = bbr1_state->target_cwnd;
            }
        }
        else if (path_x->cwin < bbr1_state->target_cwnd || path_x->delivered < PICOQUIC_CWIN_INITIAL)
        {
            path_x->cwin += bytes_delivered;
        }
        if (path_x->cwin < BBR1_MIN_PIPE_CWND((uint64_t)path_x->send_mtu))
        {
            path_x->cwin = BBR1_MIN_PIPE_CWND((uint64_t)path_x->send_mtu);
        }
    }

    BBR1ModulateCwndForProbeRTT(bbr1_state, path_x);
}
```

### Rust body
```rust
    ) {
        self.update_target_cwnd();
        self.modulate_cwnd_for_recovery(path_x, bytes_in_transit, bytes_lost, bytes_delivered);
        if !self.packet_conservation {
            if self.filled_pipe {
                path_x.cwin += bytes_delivered;
                if path_x.cwin > self.target_cwnd {
                    path_x.cwin = self.target_cwnd;
                }
            } else if path_x.cwin < self.target_cwnd || path_x.delivered < CWIN_INITIAL {
                path_x.cwin += bytes_delivered;
            }
            let min_pipe = path_x.send_mtu as u64 * 4;
            if path_x.cwin < min_pipe {
                path_x.cwin = min_pipe;
            }
        }
        self.modulate_cwnd_for_probe_rtt(path_x);
    }
```

## Pair `picoquic/bbr1.c:BBR1OnTransmit`
C: `picoquic/bbr1.c:1083-1086 BBR1OnTransmit`
Rust: `rs/fq/src/bbr1.rs:240-242 on_transmit`

### C body
```c
{
    BBR1HandleRestartFromIdle(bbr1_state, bytes_in_transit, is_app_limited);
}
```

### Rust body
```rust
    pub fn on_transmit(&mut self, bytes_in_transit: u64, is_app_limited: bool) {
        self.handle_restart_from_idle(bytes_in_transit, is_app_limited);
    }
```

## Pair `picoquic/bbr1.c:BBR1ExitFastRecovery`
C: `picoquic/bbr1.c:1112-1116 BBR1ExitFastRecovery`
Rust: `rs/fq/src/bbr1.rs:836-839 exit_fast_recovery`

### C body
```c
{
    bbr1_state->packet_conservation = 0;
    BBR1RestoreCwnd(bbr1_state, path_x);
}
```

### Rust body
```rust
    pub fn exit_fast_recovery(&mut self, path_x: &mut Path) {
        self.packet_conservation = false;
        self.restore_cwnd(path_x);
    }
```
