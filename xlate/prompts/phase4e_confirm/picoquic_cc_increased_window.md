# Phase 4E repair confirmation

This is a read-only re-triage after a Phase 4E repair or
repair-level `ok` claim.  Do not edit files.

For each entry, inspect directly relevant C and Rust context
and decide whether the current Rust translation is now
acceptable.

Report:

* `ok` when the current Rust behavior is acceptable.
* `needs_fix` when a real mismatch remains.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short confirmation conclusion","fix_summary":"remaining mismatch if any, or empty","files_changed":[],"verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/cc_common.c:picoquic_cc_increased_window`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is unrelated stream limit frame encoding and returns a byte slice, while C computes and returns a congestion window from previous_window and rtt_min.
* Prior Phase 4D analysis: Confirmed mismatch: Rust Connection::cc_increased_window returns the first path cwin and ignores previous_window and rtt_min; C doubles below the Reno RTT target or scales previous_window by min(rtt_min, satellite target) / Reno target.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Confirmed mismatch repaired: Rust now computes the window increase from previous_window and primary-path rtt_min using the Reno threshold and satellite RTT cap, matching the C formula.
* Phase 4E fix summary: Replaced the path cwin placeholder with the translated C formula in Connection::cc_increased_window.
* C source: `picoquic/cc_common.c:291-304`
* C signature: `uint64_t picoquic_cc_increased_window(picoquic_cnx_t *, uint64_t)`
* Current Rust source: `rs/fq/src/internal.rs:12896-12913`
* Current Rust item: `cc_increased_window`

### C body
```c
{
    uint64_t new_window;
    if (cnx->path[0]->rtt_min <= PICOQUIC_TARGET_RENO_RTT) {
        new_window = previous_window * 2;
    }
    else {
        double w = (double)previous_window;
        w /= (double)PICOQUIC_TARGET_RENO_RTT;
        w *= (cnx->path[0]->rtt_min > PICOQUIC_TARGET_SATELLITE_RTT) ? PICOQUIC_TARGET_SATELLITE_RTT : (double)cnx->path[0]->rtt_min;
        new_window = (uint64_t)w;
    }
    return new_window;
}
```

### Current Rust body
```rust
    pub fn cc_increased_window(&self, previous_window: u64) -> u64 {
        let rtt_min = self
            .paths
            .first()
            .map(|path| path.rtt_min)
            .unwrap_or(INITIAL_RTT);
        if rtt_min <= TARGET_RENO_RTT {
            previous_window.saturating_mul(2)
        } else {
            let rtt_cap = if rtt_min > TARGET_SATELLITE_RTT {
                TARGET_SATELLITE_RTT
            } else {
                rtt_min
            };
            (previous_window as f64 / TARGET_RENO_RTT.ticks() as f64 * rtt_cap.ticks() as f64)
                as u64
        }
    }
```
