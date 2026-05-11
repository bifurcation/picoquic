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

## `picoquic/bbr.c:BBREnterStartup`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C enters normal startup and sets pacing_gain/cwnd_gain, while the shown Rust body enters StartupLongRtt, adjusts cwin, and does not set pacing_gain or cwnd_gain.
* C source: `picoquic/bbr.c:2061-2067`
* C signature: `void BBREnterStartup(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:487-519`
* Rust item: `enter_startup`

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

## `picoquic/bbr.c:BBRIsProbingBW`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns whether the state is considered probing bandwidth, while Rust body is loss_lower_bounds and updates bw_lo/inflight_lo instead.
* C source: `picoquic/bbr.c:1532-1540`
* C signature: `int BBRIsProbingBW(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:597-615`
* Rust item: `is_probing_bw`

### C body
```c
{
    picoquic_bbr_alg_state_t state = bbr_state->state;

    return (state == picoquic_bbr_alg_probe_bw_down ||
        state == picoquic_bbr_alg_probe_bw_cruise ||
        state == picoquic_bbr_alg_drain ||
        state == picoquic_bbr_alg_probe_rtt) ? 0 : 1;
}
```

### Rust body
```rust
    pub fn loss_lower_bounds(&mut self) {
        self.bw_lo = ((BBR_BETA * self.bw_lo as f64) as u64).max(self.bw_latest);
        self.inflight_lo = ((BBR_BETA * self.inflight_lo as f64) as u64).max(self.inflight_latest);
    }
```

## `picoquic/bbr.c:BBRUpdateOffloadBudget`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets offload_budget to 3 * send_quantum, while Rust body updates the RTT jitter buffer.
* C source: `picoquic/bbr.c:883-886`
* C signature: `void BBRUpdateOffloadBudget(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:796-825`
* Rust item: `update_offload_budget`

### C body
```c
{
    bbr_state->offload_budget = 3 * bbr_state->send_quantum;
}
```

### Rust body
```rust
    pub fn update_rtt_jitter_buffer(&mut self, rs: &BbrPerAckState, current_time: u64) {
        if current_time > self.last_rtt_sample_stamp + 1000 {
            let idx = (self.rtt_jitter_cycle as usize) % BBR_RTT_JITTER_BUFFER_LEN;
            self.rtt_jitter_buffer[idx] = rs.rtt_sample;
            self.rtt_jitter_cycle += 1;
            self.last_rtt_sample_stamp = current_time;
            self.rtt_short_term_min = u64::MAX;
            self.rtt_short_term_max = 0;
            let valid = (self.rtt_jitter_cycle as usize).min(BBR_RTT_JITTER_BUFFER_LEN);
            for i in 0..valid {
                let sample = self.rtt_jitter_buffer[i];
                if sample > self.rtt_short_term_max {
                    self.rtt_short_term_max = sample;
                }
                if sample < self.rtt_short_term_min {
                    self.rtt_short_term_min = sample;
                }
            }
        }
    }
```

## `picoquic/bbr1.c:BBR1ExitStartupLongRtt`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Shown Rust body contains only the rt_prop adjustment fragment and omits the visible C round-filter reset, filled_pipe setup, drain entry, and possible ProbeBW entry.
* C source: `picoquic/bbr1.c:835-858`
* C signature: `void BBR1ExitStartupLongRtt(picoquic_bbr1_state_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr1.rs:1027-1040`
* Rust item: `exit_startup_long_rtt`

### C body
```c
{
    /* Reset the round filter so it will start at current time */
    bbr1_state->next_round_delivered = path_x->delivered;
    bbr1_state->round_count++;
    bbr1_state->round_start = 1;
    /* Set the filled pipe indicator */
    bbr1_state->full_bw = bbr1_state->btl_bw;
    bbr1_state->full_bw_count = 3;
    bbr1_state->filled_pipe = 1;
    /* Check the RTT measurement for pathological cases */
    if ((bbr1_state->rtt_filter.is_init || bbr1_state->rtt_filter.sample_current > 0) &&
        bbr1_state->rt_prop > 30000000 &&
        bbr1_state->rtt_filter.sample_max < bbr1_state->rt_prop) {
        bbr1_state->rt_prop = bbr1_state->rtt_filter.sample_max;
        bbr1_state->rt_prop_stamp = current_time;
    }
    /* Enter drain */
    BBR1EnterDrain(bbr1_state, path_x, current_time);
    /* If there were just few bytes in transit, enter probe */
    if (path_x->bytes_in_transit <= BBR1Inflight(bbr1_state, 1.0)) {
        BBR1EnterProbeBW(bbr1_state, path_x, current_time);
    }
}
```

### Rust body
```rust
        {
            self.rt_prop = self.rtt_filter.sample_max.ticks();
            self.rt_prop_stamp = current_time;
        }
```

## `picoquic/bytestream.c:byteread_int16`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C reads two bytes, composes a big-endian 16-bit value, stores it, and advances ptr; the shown Rust body only performs the capacity check/error path.
* C source: `picoquic/bytestream.c:222-234`
* C signature: `int byteread_int16(bytestream *, uint16_t *)`
* Rust source: `rs/fq/src/bytestream.rs:293-297`
* Rust item: `read_u16`

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
