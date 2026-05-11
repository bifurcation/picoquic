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

## `picoquic/bbr.c:BBREnterProbeRTT`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C enters ProbeRTT with pacing gain 1.0 and ProbeRTT cwnd gain, while Rust body enters Drain and uses startup gain constants.
* C source: `picoquic/bbr.c:1487-1493`
* C signature: `void BBREnterProbeRTT(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:375-390`
* Rust item: `enter_probe_rtt`

### C body
```c
{
    bbr_state->state = picoquic_bbr_alg_probe_rtt;
    bbr_state->pacing_gain = 1.0;
    bbr_state->cwnd_gain = BBRProbeRTTCwndGain;  /* 0.5 */
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

## `picoquic/bbr.c:BBRInitLowerBounds`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C initializes bw_lo and inflight_lo when they are UINT64_MAX, while Rust body initializes pacing_rate.
* C source: `picoquic/bbr.c:1024-1033`
* C signature: `void BBRInitLowerBounds(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:535-557`
* Rust item: `init_lower_bounds`

### C body
```c
{
    if (bbr_state->bw_lo == UINT64_MAX) {
        bbr_state->bw_lo = bbr_state->max_bw;
    }
    if (bbr_state->inflight_lo == UINT64_MAX) {
        bbr_state->inflight_lo = path_x->cwin;
    }
}
```

### Rust body
```rust
    pub fn init_pacing_rate(&mut self, path_x: &Path) {
        let initial_rtt = if path_x.smoothed_rtt != INITIAL_RTT || path_x.rtt_variant.ticks() != 0 {
            path_x.smoothed_rtt.ticks()
        } else {
            INITIAL_RTT.ticks()
        };
        let nominal_bandwidth = (1_000_000u64 * CWIN_INITIAL) as f64 / initial_rtt as f64;
        self.pacing_rate = BBR_STARTUP_PACING_GAIN * nominal_bandwidth;
    }
```

## `picoquic/bbr.c:BBRTargetInflight`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns min(bdp, cwin), while Rust body updates latest delivery/loss-round signal fields and returns nothing.
* C source: `picoquic/bbr.c:1691-1696`
* C signature: `uint64_t BBRTargetInflight(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:745-767`
* Rust item: `target_inflight`

### C body
```c
{
    return (bbr_state->bdp < path_x->cwin) ? bbr_state->bdp : path_x->cwin;
}
```

### Rust body
```rust
    pub fn update_latest_delivery_signals(&mut self, path_x: &Path, rs: &BbrPerAckState) {
        self.loss_round_start = false;
        if self.bw_latest < rs.delivery_rate {
            self.bw_latest = rs.delivery_rate;
        }
        if self.inflight_latest < rs.delivered {
            self.inflight_latest = rs.delivered;
        }
        let prior_delivered = path_x.delivered.wrapping_sub(rs.delivered);
        if prior_delivered >= self.loss_round_delivered {
            self.loss_round_delivered = path_x.delivered;
            self.loss_round_start = true;
        }
    }
```

## `picoquic/bbr1.c:BBR1EnterStartup`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C enters startup and sets pacing/cwnd gains, while the shown Rust body checks full-pipe bandwidth and does not enter startup.
* C source: `picoquic/bbr1.c:327-332`
* C signature: `void BBR1EnterStartup(picoquic_bbr1_state_t *)`
* Rust source: `rs/fq/src/bbr1.rs:179-203`
* Rust item: `enter_startup`

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

## `picoquic/bbr1.c:picoquic_bbr1_reset`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C reset also sets path_x->cwin, rt_prop to UINT64_MAX, options/quantum_ratio default, timestamps to current_time, enters startup, sets send quantum, and updates target cwnd. Rust body only assigns a zero/default struct and does not show those follow-up actions.
* C source: `picoquic/bbr1.c:449-467`
* C signature: `void picoquic_bbr1_reset(picoquic_bbr1_state_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr1.rs:1089-1139`
* Rust item: `reset`

### C body
```c
{
    memset(bbr1_state, 0, sizeof(picoquic_bbr1_state_t));
    path_x->cwin = PICOQUIC_CWIN_INITIAL;
    bbr1_state->rt_prop = UINT64_MAX; 
    picoquic_bbr1_set_options(bbr1_state);
    if (bbr1_state->quantum_ratio == 0) {
        bbr1_state->quantum_ratio = 0.001;
    }

    bbr1_state->rt_prop_stamp = current_time;
    bbr1_state->cycle_stamp = current_time;
    bbr1_state->cycle_index = 0;
    bbr1_state->cycle_start = 0;

    BBR1EnterStartup(bbr1_state);
    BBR1SetSendQuantum(bbr1_state, path_x);
    BBR1UpdateTargetCwnd(bbr1_state);
}
```

### Rust body
```rust
        *self = Self {
            state: Bbr1AlgState::Startup,
            btl_bw: 0,
            next_round_delivered: 0,
            btl_bw_filter: [0; BBR1_BTL_BW_FILTER_LENGTH],
            full_bw: 0,
            rt_prop: 0,
            rt_prop_stamp: 0,
            cycle_stamp: 0,
            probe_rtt_done_stamp: 0,
            prior_cwnd: 0,
            prior_in_flight: 0,
            bytes_delivered: 0,
            send_quantum: 0,
            rtt_filter: MinMaxRtt::default(),
            target_cwnd: 0,
            pacing_gain: 0.0,
            cwnd_gain: 0.0,
            pacing_rate: 0.0,
            cycle_index: 0,
            cycle_start: 0,
            round_count: 0,
            full_bw_count: 0,
            lt_rtt_cnt: 0,
            lt_bw: 0,
            lt_last_stamp: 0,
            previous_round_lost: 0,
            previous_sampling_delivered: 0,
            previous_sampling_lost: 0,
            loss_interval_start: 0,
            congestion_sequence: 0,
            cwin_before_suspension: 0,
            option_string: None,
            wifi_shadow_rtt: 0,
            quantum_ratio: 0.0,
            filled_pipe: false,
            round_start: false,
            rt_prop_expired: false,
            probe_rtt_round_done: false,
            idle_restart: false,
            packet_conservation: false,
            btl_bw_increased: false,
            lt_use_bw: false,
            lt_is_sampling: false,
            last_loss_was_timeout: false,
            cycle_on_loss: false,
            is_suspended: false,
            is_suspension_nearly_over: false,
        };
```
