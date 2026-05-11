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

## `picoquic/bbr.c:BBRCheckStartupLongRtt`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The shown Rust body is only a tiny fragment and lacks the visible RTT hystart, ECN, and loss-volume exit checks present in C.
* C source: `picoquic/bbr.c:2128-2153`
* C signature: `void BBRCheckStartupLongRtt(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:1648-1660`
* Rust item: `check_startup_long_rtt`

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

## `picoquic/bbr.c:BBRExitLostFeedback`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is for has_elapsed_in_phase and does not restore cwin or clear is_handling_lost_feedback.
* C source: `picoquic/bbr.c:782-788`
* C signature: `void BBRExitLostFeedback(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:454-468`
* Rust item: `exit_lost_feedback`

### C body
```c
{
    if (bbr_state->is_handling_lost_feedback) {
        path_x->cwin = bbr_state->cwin_before_lost_feedback;
        bbr_state->is_handling_lost_feedback = 0;
    }
}
```

### Rust body
```rust
    pub(crate) fn has_elapsed_in_phase(&self, interval: u64, current_time: u64) -> bool {
        current_time > self.cycle_stamp + interval
    }
```

## `picoquic/bbr.c:BBRResetLowerBounds`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets bw_lo and inflight_lo to UINT64_MAX, while Rust body resets RTT jitter buffer fields.
* C source: `picoquic/bbr.c:1088-1092`
* C signature: `void BBRResetLowerBounds(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:662-680`
* Rust item: `reset_lower_bounds`

### C body
```c
{
    bbr_state->bw_lo = UINT64_MAX;
    bbr_state->inflight_lo = UINT64_MAX;
}
```

### Rust body
```rust
    pub fn reset_rtt_jitter_buffer(&mut self, rtt_init_value: u64, current_time: u64) {
        self.rtt_jitter_cycle = 0;
        self.last_rtt_sample_stamp = current_time;
        self.rtt_short_term_min = rtt_init_value;
        self.rtt_short_term_max = rtt_init_value;
        self.probe_rtt_min_delay = rtt_init_value;
        self.nb_rtt_excess = 0;
    }
```

## `picoquic/bbr.c:IsInAProbeBWState`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body updates offload_budget and does not test whether state is one of the ProbeBW states.
* C source: `picoquic/bbr.c:1522-1530`
* C signature: `int IsInAProbeBWState(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:781-798`
* Rust item: `is_in_a_probe_bw_state`

### C body
```c
{
    picoquic_bbr_alg_state_t state = bbr_state->state;

    return (state == picoquic_bbr_alg_probe_bw_down ||
        state == picoquic_bbr_alg_probe_bw_cruise ||
        state == picoquic_bbr_alg_probe_bw_refill ||
        state == picoquic_bbr_alg_probe_bw_up);
}
```

### Rust body
```rust
    pub fn update_offload_budget(&mut self) {
        self.offload_budget = 3 * self.send_quantum;
    }
```

## `picoquic/bbr1.c:BBR1ModulateCwndForProbeRTT`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C caps cwin in ProbeRTT; Rust body shown sets send_quantum based on pacing_rate and does not reference ProbeRTT or cwin.
* C source: `picoquic/bbr1.c:1015-1023`
* C signature: `void BBR1ModulateCwndForProbeRTT(picoquic_bbr1_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr1.rs:339-362`
* Rust item: `modulate_cwnd_for_probe_rtt`

### C body
```c
{
    if (bbr1_state->state == picoquic_bbr1_alg_probe_rtt)
    {
        if (path_x->cwin > BBR1_MIN_PIPE_CWND((uint64_t)path_x->send_mtu)) {
            path_x->cwin = BBR1_MIN_PIPE_CWND((uint64_t)path_x->send_mtu);
        }
    }
}
```

### Rust body
```rust
    pub fn set_send_quantum(&mut self, path_x: &Path) {
        if self.pacing_rate < BBR1_PACING_RATE_LOW {
            self.send_quantum = path_x.send_mtu as u64;
        } else if self.pacing_rate < BBR1_PACING_RATE_MEDIUM {
            self.send_quantum = 2 * path_x.send_mtu as u64;
        } else {
            self.send_quantum = (self.pacing_rate * self.quantum_ratio) as u64;
            if self.send_quantum > 0x10000 {
                self.send_quantum = 0x10000;
            }
        }
    }
```
