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

## `picoquic/bbr.c:BBRRandomIntBetween`
* Phase 4C status: `suspect`
* Phase 4C rationale: C delegates directly to picoquic_test_uniform_random over the inclusive range, while Rust implements its own rejection/modulo random selection.
* C source: `picoquic/bbr.c:1639-1647`
* C signature: `uint64_t BBRRandomIntBetween(picoquic_bbr_state_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:988-998`
* Rust item: `random_int_between`

### C body
```c
{
    return (low + picoquic_test_uniform_random(&bbr_state->random_context, (high - low) + 1));
}
```

### Rust body
```rust
    fn random_int_between(&mut self, low: u64, high: u64) -> u64 {
        let range = high - low + 1;
        let rnd_min = u64::MAX % range;
        let rnd = loop {
            let v = self.bbr_random();
            if v >= rnd_min {
                break v;
            }
        };
        low + rnd % range
    }
```

## `picoquic/bbr1.c:BBR1CheckCyclePhase`
* Phase 4C status: `suspect`
* Phase 4C rationale: C visibly guards the cycle advance with state == probe_bw, but the shown Rust fragment only shows the is_next_cycle_phase check and advance.
* C source: `picoquic/bbr1.c:756-762`
* C signature: `void BBR1CheckCyclePhase(picoquic_bbr1_state_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/bbr1.rs:889-894`
* Rust item: `check_cycle_phase`

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

## `picoquic/bytestream.c:bytewrite_addr`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both bodies branch on IPv4 versus IPv6 and write family, address bytes, and port, but C writes addr->sa_family directly while Rust writes WIRE_AF_INET/WIRE_AF_INET6 constants.
* C source: `picoquic/bytestream.c:386-399`
* C signature: `int bytewrite_addr(bytestream *, const struct sockaddr *)`
* Rust source: `rs/fq/src/bytestream.rs:568-582`
* Rust item: `write_addr`

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

## `picoquic/cc_common.c:picoquic_cc_get_lowest_not_ack`
* Phase 4C status: `suspect`
* Phase 4C rationale: The visible lowest_not_ack method matches the pending-or-highest+1 shape, but the Rust body shown is a larger trait impl with additional unrelated methods rather than only the target function.
* C source: `picoquic/cc_common.c:55-61`
* C signature: `uint64_t picoquic_cc_get_lowest_not_ack(picoquic_path_t *)`
* Rust source: `rs/fq/src/cc_common.rs:311-416`
* Rust item: `lowest_not_ack`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = (path_x->cnx->is_multipath_enabled) ? &path_x->pkt_ctx : &path_x->cnx->pkt_ctx[picoquic_packet_context_application];
    uint64_t lowest_not_ack = (pkt_ctx->pending_first != NULL) ? pkt_ctx->pending_first->sequence_number : pkt_ctx->highest_acknowledged + 1;

    return lowest_not_ack;
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

## `picoquic/config.c:picoquic_config_command_line_ex`
* Phase 4C status: `suspect`
* Phase 4C rationale: C prints an unknown-option message and returns ret defaulting to 0; Rust maps parse failure to Error::InvalidArgument and has no visible stderr behavior.
* C source: `picoquic/config.c:742-757`
* C signature: `int picoquic_config_command_line_ex(const char *, int *, int, const char **, const char *, picoquic_quic_config_t *)`
* Rust source: `rs/fq/src/config.rs:1297-1307`
* Rust item: `command_line_ex`

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
