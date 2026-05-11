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

## `picoquic/bbr.c:BBRCheckTimeToCruise`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only implements the first headroom check shown; it lacks the C body's second inflight <= BDP check and final return.
* C source: `picoquic/bbr.c:1626-1636`
* C signature: `int BBRCheckTimeToCruise(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:1719-1722`
* Rust item: `check_time_to_cruise`

### C body
```c
{
    if (path_x->bytes_in_transit > BBRInflightWithHeadroom(bbr_state, path_x)) {
        return 0; /* not enough headroom */
    }
    if (path_x->bytes_in_transit <= BBRInflightWithBw(bbr_state, path_x, 1.0, bbr_state->max_bw)) {
        return 1;  /* inflight <= estimated BDP */
    }
    return 0;
}
```

### Rust body
```rust
        if path_x.bytes_in_transit > self.inflight_with_headroom(path_x) {
            return false; // not enough headroom yet
        }
```

## `picoquic/bbr.c:BBRHandleProbeRTT`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown stops after the else-if line and omits the C logic that marks round done and calls the ProbeRTT completion check.
* C source: `picoquic/bbr.c:1456-1485`
* C signature: `void BBRHandleProbeRTT(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:1399-1411`
* Rust item: `handle_probe_rtt`

### C body
```c
{
    /* Ignore low rate samples during ProbeRTT.*/
    /* We do not implement:
    * MarkConnectionAppLimited();
    * because the app_limited status is maintained as part of app logic.
    */
    /* 
    * testing the bytes in flight when the last ACK was sent,
    * as they reflect the size of the queue encountered when
    * measuring the RTT.
    */
    if (bbr_state->probe_rtt_done_stamp == 0 &&
        rs->tx_in_flight <= BBRProbeRTTCwnd(bbr_state, path_x)) {
        /* Wait for at least ProbeRTTDuration to elapse: */
        bbr_state->probe_rtt_done_stamp =
            current_time + BBRProbeRTTDuration;
        /* Wait for at least one round to elapse: */
        bbr_state->probe_rtt_round_done = 0;
        BBRStartRound(bbr_state, path_x);
    }
    else if (bbr_state->probe_rtt_done_stamp != 0) {
        if (bbr_state->round_start) {
            bbr_state->probe_rtt_round_done = 1;
        }
        if (bbr_state->probe_rtt_round_done) {
            BBRCheckProbeRTTDone(bbr_state, path_x, current_time);
        }
    }
}
```

### Rust body
```rust
        if self.probe_rtt_done_stamp == 0 && rs.tx_in_flight <= self.probe_rtt_cwnd(path_x) {
            self.probe_rtt_done_stamp = current_time + BBR_PROBE_RTT_DURATION;
            self.probe_rtt_round_done = false;
            self.start_round(connection, path_x);
        } else if self.probe_rtt_done_stamp != 0 {
```

## `picoquic/bbr.c:BBRRestoreCwnd`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is for setting pacing rate and does not return max(prior_cwnd, path cwin).
* C source: `picoquic/bbr.c:736-744`
* C signature: `uint64_t BBRRestoreCwnd(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:687-714`
* Rust item: `restore_cwnd`

### C body
```c
{
    if (bbr_state->prior_cwnd > path_x->cwin) {
        return bbr_state->prior_cwnd;
    }
    else {
        return path_x->cwin;
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

## `picoquic/bbr.c:IsRTTTooHigh`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns whether nb_rtt_excess exceeds a jitter-buffer length, while Rust body returns a tuple of state and bw.
* C source: `picoquic/bbr.c:1386-1389`
* C signature: `int IsRTTTooHigh(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:941-953`
* Rust item: `is_rtt_too_high`

### C body
```c
{
    return (bbr_state->nb_rtt_excess > BBRRTTJitterBufferLen);
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        (self.state as u64, self.bw)
    }
```

## `picoquic/bbr1.c:BBR1RestoreCwnd`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C only restores cwin to prior_cwnd if smaller, while the shown Rust body reduces cwin on loss and applies packet-conservation logic.
* C source: `picoquic/bbr1.c:918-923`
* C signature: `void BBR1RestoreCwnd(picoquic_bbr1_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr1.rs:306-333`
* Rust item: `restore_cwnd`

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
