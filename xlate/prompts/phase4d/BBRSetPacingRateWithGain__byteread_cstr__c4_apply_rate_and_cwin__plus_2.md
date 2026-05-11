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

## `picoquic/bbr.c:BBRSetPacingRateWithGain`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust adds min_rtt > 0 and min_rtt != u64::MAX guards before using bdp_seed resume rate; C body has no such guard.
* C source: `picoquic/bbr.c:944-960`
* C signature: `void BBRSetPacingRateWithGain(picoquic_bbr_state_t *, double)`
* Rust source: `rs/fq/src/bbr.rs:698-714`
* Rust item: `set_pacing_rate_with_gain`

### C body
```c
{
    double rate = pacing_gain * ((double)(bbr_state->bw * (100 - BBRPacingMarginPercent))) / (double)100;

    if (bbr_state->state == picoquic_bbr_alg_startup_resume &&
        !bbr_state->filled_pipe &&
        bbr_state->bdp_seed > 0) {
        double bdp_rate = (((double)bbr_state->bdp_seed*1000000.0) / (double)bbr_state->min_rtt);
        if (bdp_rate > rate) {
            rate = bdp_rate;
        }
    }

    if (bbr_state->filled_pipe || rate > bbr_state->pacing_rate) {
        bbr_state->pacing_rate = rate;
    }
}
```

### Rust body
```rust
    pub fn set_pacing_rate_with_gain(&mut self, pacing_gain: f64) {
        let rate = pacing_gain * (self.bw * (100 - BBR_PACING_MARGIN_PERCENT)) as f64 / 100.0;
        let rate = if self.state == BbrAlgState::StartupResume
            && !self.filled_pipe
            && self.bdp_seed > 0
            && self.min_rtt > 0
            && self.min_rtt != u64::MAX
        {
            let bdp_rate = self.bdp_seed as f64 * 1_000_000.0 / self.min_rtt as f64;
            rate.max(bdp_rate)
        } else {
            rate
        };
        if self.filled_pipe || rate > self.pacing_rate {
            self.pacing_rate = rate;
        }
    }
```

## `picoquic/bytestream.c:byteread_cstr`
* Phase 4C status: `suspect`
* Phase 4C rationale: C requires space for a trailing NUL and writes it; Rust accepts dst length equal to string length and does not write a NUL terminator.
* C source: `picoquic/bytestream.c:354-369`
* C signature: `int byteread_cstr(bytestream *, char *, size_t)`
* Rust source: `rs/fq/src/bytestream.rs:538-551`
* Rust item: `read_str`

### C body
```c
{
    uint64_t l_read = 0;
    int ret = byteread_vint(s, &l_read);

    size_t l_cstr = (size_t)l_read;

    if (ret != 0 || l_cstr != l_read || l_cstr + 1 > max_len) {
        ret = -1;
    } else {
        ret |= byteread_buffer(s, cstr, l_cstr);
        cstr[l_cstr] = 0;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn read_str(&mut self, dst: &mut [u8]) -> Result<usize, Error> {
        let l_read = self.read_varint()?;
        let l = l_read as usize;
        if (l_read as usize as u64) != l_read {
            self.set_error();
            return Err(Error::InvalidArgument);
        }
        if l > dst.len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        self.read_bytes(&mut dst[..l])?;
        Ok(l)
    }
```

## `picoquic/c4.c:c4_apply_rate_and_cwin`
* Phase 4C status: `suspect`
* Phase 4C rationale: Bodies mostly match, but Rust uses saturating_sub for alpha_1024_current - 1024 where C performs direct unsigned subtraction; visible behavior can differ if alpha is below 1024.
* C source: `picoquic/c4.c:359-424`
* C signature: `void c4_apply_rate_and_cwin(picoquic_path_t *, c4_state_t *)`
* Rust source: `rs/fq/src/c4.rs:490-544`
* Rust item: `apply_rate_and_cwin`

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

## `picoquic/cc_common.c:picoquic_cc_slow_start_increase`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks cnx->cwin_blocked directly; Rust substitutes bytes_in_transit < cwin/otherwise, which is not body-visible equivalent.
* C source: `picoquic/cc_common.c:210-222`
* C signature: `uint64_t picoquic_cc_slow_start_increase(picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/cc_common.rs:317-416`
* Rust item: `slow_start_increase`

### C body
```c
uint64_t picoquic_cc_slow_start_increase(picoquic_path_t * path_x, uint64_t nb_delivered) {
    /* App limited. */
    /* TODO discuss
     * path_x->cwin < path_x->bytes_in_transit returns false in cc code
     * path_x->cnx->cwin_blocked is set to true
     * (path_x->cwin < path_x->bytes_in_transit) != path_x->cnx->cwin_blocked?
     */
    if (!path_x->cnx->cwin_blocked) {
        return 0;
    }

    return nb_delivered;
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

## `picoquic/config.c:picoquic_config_usage_file`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust prints the option table but omits the C body's special extra supported-values line for the CC_ALGO option.
* C source: `picoquic/config.c:580-607`
* C signature: `void picoquic_config_usage_file(FILE *)`
* Rust source: `rs/fq/src/config.rs:1337-1347`
* Rust item: `write_usage`

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
