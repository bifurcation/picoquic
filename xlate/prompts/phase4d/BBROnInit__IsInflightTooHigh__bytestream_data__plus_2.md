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

## `picoquic/bbr.c:BBROnInit`
* Phase 4C status: `suspect`
* Phase 4C rationale: C resets the RTT jitter buffer only under an RTTJitterBuffer conditional block, while Rust always calls reset_rtt_jitter_buffer.
* C source: `picoquic/bbr.c:558-596`
* C signature: `void BBROnInit(picoquic_bbr_state_t *, picoquic_path_t *, uint64_t, const char *)`
* Rust source: `rs/fq/src/bbr.rs:1890-1927`
* Rust item: `on_init`

### C body
```c
{
    /* TODO:
    init_windowed_max_filter(filter = BBR.MaxBwFilter, value = 0, time = 0)
    */
    memset(bbr_state, 0, sizeof(picoquic_bbr_state_t));
    bbr_state->option_string = option_string;

    BBRInitRandom(bbr_state, path_x, current_time);
    /* If RTT was already sampled, use it, other wise set min RTT to infinity */
    if (path_x->smoothed_rtt == PICOQUIC_INITIAL_RTT
        && path_x->rtt_variant == 0) {
        bbr_state->min_rtt = UINT64_MAX;
    }
    else {
        bbr_state->min_rtt = path_x->smoothed_rtt;
    }
#ifdef RTTJitterBuffer
    BBRResetRTTJitterBuffer(bbr_state, bbr_state->min_rtt, current_time);
#endif
    bbr_state->probe_rtt_min_stamp = current_time;
    bbr_state->probe_rtt_min_delay = bbr_state->min_rtt;
    bbr_state->min_rtt_stamp = current_time;
    bbr_state->extra_acked_interval_start = current_time;
    bbr_state->extra_acked_delivered = 0;
    /* Support for the experimental options */
    BBRSetOptions(bbr_state);
    if (bbr_state->quantum_ratio == 0) {
        bbr_state->quantum_ratio = 0.001;
    }

    BBRResetCongestionSignals(bbr_state);
    BBRResetLowerBounds(bbr_state);
    BBRInitRoundCounting(bbr_state, path_x);
    BBRInitFullPipe(bbr_state);
    BBRInitPacingRate(bbr_state, path_x);
    BBREnterStartup(bbr_state, path_x);
}
```

### Rust body
```rust
    ) {
        *self = BbrState::default();
        self.option_string = option_string;

        self.init_random(connection, path_x, current_time);
        if path_x.smoothed_rtt == INITIAL_RTT && path_x.rtt_variant.ticks() == 0 {
            self.min_rtt = u64::MAX;
        } else {
            self.min_rtt = path_x.smoothed_rtt.ticks();
        }
        // RTTJitterBuffer is always enabled in this build.
        let min_rtt = self.min_rtt;
        self.reset_rtt_jitter_buffer(min_rtt, current_time);

        self.probe_rtt_min_stamp = current_time;
        self.probe_rtt_min_delay = self.min_rtt;
        self.min_rtt_stamp = current_time;
        self.extra_acked_interval_start = current_time;
        self.extra_acked_delivered = 0;

        self.set_options();
        if self.quantum_ratio == 0.0 {
            self.quantum_ratio = 0.001;
        }

        self.reset_congestion_signals();
        self.reset_lower_bounds();
        self.init_round_counting(connection, path_x);
        self.init_full_pipe();
        self.init_pacing_rate(path_x);
        self.enter_startup(path_x);
    }
```

## `picoquic/bbr.c:IsInflightTooHigh`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust uses saturating_sub for rs_delivered while C uses unsigned subtraction, which would wrap on underflow.
* C source: `picoquic/bbr.c:1149-1172`
* C signature: `int IsInflightTooHigh(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:925-934`
* Rust item: `is_inflight_too_high`

### C body
```c
{
    if (rs->ecn_alpha > BBRExcessiveEcnCE) {
        return 1;
    }
    else {
        uint64_t rs_delivered = path_x->delivered - rs->delivered;
        if (rs_delivered > bbr_state->recovery_delivered &&
            rs->lost > (uint64_t)(((double)rs->tx_in_flight) * BBRLossThresh) &&
            rs->lost > 3 * path_x->send_mtu) {
            return 1;
        }
        else {
            return 0;
        }
    }
}
```

### Rust body
```rust
    pub fn is_inflight_too_high(&self, path_x: &Path, rs: &BbrPerAckState) -> bool {
        if rs.ecn_alpha > BBR_EXCESSIVE_ECN_CE {
            return true;
        }
        // rs_delivered = bytes delivered since the affected packets were sent.
        let rs_delivered = path_x.delivered.saturating_sub(rs.delivered);
        rs_delivered > self.recovery_delivered
            && rs.lost > (rs.tx_in_flight as f64 * BBR_LOSS_THRESH) as u64
            && rs.lost > 3 * path_x.send_mtu as u64
    }
```

## `picoquic/bytestream.c:bytestream_data`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns the raw data pointer without using ptr; Rust returns a slice limited to ..ptr.
* C source: `picoquic/bytestream.c:73-76`
* C signature: `const uint8_t * bytestream_data(bytestream *)`
* Rust source: `rs/fq/src/bytestream.rs:173-175`
* Rust item: `as_bytes`

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

## `picoquic/c4.c:c4_update_ecn_alpha`
* Phase 4C status: `suspect`
* Phase 4C rationale: Core update is similar, but Rust adds a sum > 0 guard and uses 0 when the C expression would divide by delta_ce + delta_ect1 directly.
* C source: `picoquic/c4.c:321-350`
* C signature: `void c4_update_ecn_alpha(picoquic_path_t *, c4_state_t *)`
* Rust source: `rs/fq/src/c4.rs:228-251`
* Rust item: `ecn_alpha_update_counts`

### C body
```c
{
    uint64_t frac = 0;
    picoquic_packet_context_t* pkt_ctx = (path_x->cnx->is_multipath_enabled)?
        &path_x->pkt_ctx : &path_x->cnx->pkt_ctx[picoquic_packet_context_application];
    int64_t delta_ect1 = pkt_ctx->ecn_ect1_total_remote - c4_state->ecn_ect1;
    int64_t delta_ce = pkt_ctx->ecn_ce_total_remote - c4_state->ecn_ce;

    c4_state->ecn_ect1 = pkt_ctx->ecn_ect1_total_remote;
    c4_state->ecn_ce = pkt_ctx->ecn_ce_total_remote;

    if (delta_ce > 0 || delta_ect1 > 0) {
        frac = (delta_ce * 1024) / (delta_ce + delta_ect1);

        if (frac > c4_state->ecn_alpha && frac >= 512) {
            c4_state->ecn_alpha = frac;
        }
        else
        {
            uint64_t alpha_shifted = c4_state->ecn_alpha << C4_ECN_SHIFT_G;
            alpha_shifted -= c4_state->ecn_alpha;
            alpha_shifted += frac;
            c4_state->ecn_alpha = alpha_shifted >> C4_ECN_SHIFT_G;
        }
    }
}
```

### Rust body
```rust
    fn ecn_alpha_update_counts(&mut self, ecn_ect1_remote: u64, ecn_ce_remote: u64) {
        let delta_ect1 = ecn_ect1_remote as i64 - self.ecn_ect1 as i64;
        let delta_ce = ecn_ce_remote as i64 - self.ecn_ce as i64;

        self.ecn_ect1 = ecn_ect1_remote;
        self.ecn_ce = ecn_ce_remote;

        if delta_ce > 0 || delta_ect1 > 0 {
            let sum = delta_ce + delta_ect1;
            let frac: u64 = if sum > 0 {
                ((delta_ce * 1024) / sum) as u64
            } else {
                0
            };

            if frac > self.ecn_alpha && frac >= 512 {
                self.ecn_alpha = frac;
            } else {
                // EWMA: alpha = alpha*(1 - 1/2^shift) + frac*(1/2^shift)
                let alpha_shifted = (self.ecn_alpha << C4_ECN_SHIFT_G) - self.ecn_alpha + frac;
                self.ecn_alpha = alpha_shifted >> C4_ECN_SHIFT_G;
            }
        }
    }
```

## `picoquic/config.c:picoquic_config_command_line`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both look up an option by letter and apply/parse it, but C constructs an opt_string and reports unknown options to stderr while Rust returns InvalidArgument directly.
* C source: `picoquic/config.c:722-740`
* C signature: `int picoquic_config_command_line(int, int *, int, const char **, const char *, picoquic_quic_config_t *)`
* Rust source: `rs/fq/src/config.rs:1281-1291`
* Rust item: `command_line`

### C body
```c
{
    int ret = 0;
    int option_index = -1;
    char opt_string[3] = { '-', 0, 0 };

    opt_string[1] = (char)opt;
    option_index = picoquic_config_get_option_char_index(opt);

    if (option_index == -1) {
        fprintf(stderr, "Unknown option: -%c\n", opt);
        ret = -1;
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
        let (_, entry) = option_entry_by_letter(opt).ok_or(Error::InvalidArgument)?;
        let params = collect_params(entry, p_optind, argv, optarg)?;
        apply_option(self, entry, &params)
    }
```
