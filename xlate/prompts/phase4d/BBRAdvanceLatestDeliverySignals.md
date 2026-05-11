# Phase 4D deep translation review and repair

You are resolving Phase 4C non-OK C/Rust function-pair audit
entries.  Phase 4C was intentionally body-only and shallow;
Phase 4D is allowed to inspect broader context and edit Rust.

For each entry:

1. Read the C function and any directly relevant C context:
   types, constants/macros, helper callees, and callers when
   needed to understand observable behavior.
2. Read the Rust function in context, including local types,
   helpers, tests, and nearby translated functions.
3. Decide whether the Phase 4C concern is a false positive.
   If the Rust behavior is acceptable, do not edit source and
   report outcome `ok`.
4. If the Rust translation is actually wrong, fix it now in
   `rs/fq/` while preserving safe, idiomatic Rust and the
   repository translation rules.
5. Only report `blocked` if a real mismatch remains impossible
   to fix in this batch, and give a concrete human-actionable
   reason.  Do not use vague deferrals.

Do not edit C sources.  Do not replace working Rust with a stub,
placeholder, fabricated default, or weaker behavior.  Keep public
API shape stable unless the existing shape cannot express the C
behavior safely.

If you fix anything, run the narrowest useful test if one is
obvious.  The driver will run the standard cargo gates after
the batch when needed.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|fixed|blocked","analysis":"short deeper-review conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/bbr.c:BBRAdvanceLatestDeliverySignals`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C updates bw_latest and inflight_latest when loss_round_start is set, while Rust body is bdp_multiple_with_bw and unrelated.
* C source: `picoquic/bbr.c:1006-1012`
* C signature: `void BBRAdvanceLatestDeliverySignals(picoquic_bbr_state_t *, bbr_per_ack_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:331-354`
* Rust item: `advance_latest_delivery_signals`

### C body
```c
static void BBRAdvanceLatestDeliverySignals(picoquic_bbr_state_t* bbr_state, bbr_per_ack_state_t * rs) {
    if (bbr_state->loss_round_start) {
        bbr_state->bw_latest = rs->delivery_rate;
        bbr_state->inflight_latest = rs->delivered;
    }
}
```

### Rust body
```rust
    pub fn bdp_multiple_with_bw(&mut self, path_x: &Path, gain: f64, bw: u64) -> u64 {
        if self.min_rtt == u64::MAX {
            return CWIN_INITIAL * path_x.send_mtu as u64;
        }
        // PICOQUIC_BYTES_FROM_RATE(min_rtt_us, bps) = min_rtt * bps / 1_000_000
        self.bdp = self.min_rtt * bw / 1_000_000;
        (gain * self.bdp as f64) as u64
    }
```
