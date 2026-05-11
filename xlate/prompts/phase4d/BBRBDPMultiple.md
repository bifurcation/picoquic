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

## `picoquic/bbr.c:BBRBDPMultiple`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns BBRBDPMultipleWithBw using bw and gain, while Rust body bounds cwin for model and has unrelated state/cap logic.
* C source: `picoquic/bbr.c:878-881`
* C signature: `uint64_t BBRBDPMultiple(picoquic_bbr_state_t *, picoquic_path_t *, double)`
* Rust source: `rs/fq/src/bbr.rs:1084-1116`
* Rust item: `bdp_multiple`

### C body
```c
{
    return BBRBDPMultipleWithBw(bbr_state, path_x, gain, bbr_state->bw);
}
```

### Rust body
```rust
    pub fn bound_cwnd_for_model(&mut self, path_x: &mut Path) {
        const BBR_MIN_PIPE_CWND: u64 = 4;
        let mut cap = u64::MAX;
        if self.is_in_a_probe_bw_state() && self.state != BbrAlgState::ProbeBwCruise {
            if self.inflight_hi > 0 {
                cap = self.inflight_hi;
            }
        } else if self.state == BbrAlgState::ProbeRtt || self.state == BbrAlgState::ProbeBwCruise {
            cap = self.inflight_with_headroom(path_x);
        }
        if cap > self.inflight_lo {
            cap = self.inflight_lo;
        }
        let floor = BBR_MIN_PIPE_CWND * path_x.send_mtu as u64;
        if cap < floor {
            cap = floor;
        }
        if path_x.cwin > cap {
            path_x.cwin = cap;
        }
    }
```
