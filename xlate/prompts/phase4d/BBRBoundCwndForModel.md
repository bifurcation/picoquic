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

## `picoquic/bbr.c:BBRBoundCwndForModel`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is visibly incomplete/truncated and lacks the cap lower-bound application and cwin assignment present in C.
* C source: `picoquic/bbr.c:647-671`
* C signature: `void BBRBoundCwndForModel(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:1096-1103`
* Rust item: `bound_cwnd_for_model`

### C body
```c
{
    uint64_t cap = UINT64_MAX;
    if (IsInAProbeBWState(bbr_state) &&
        bbr_state->state != picoquic_bbr_alg_probe_bw_cruise) {
        if (bbr_state->inflight_hi > 0) {
            cap = bbr_state->inflight_hi;
        }
    }
    else if (bbr_state->state == picoquic_bbr_alg_probe_rtt ||
        bbr_state->state == picoquic_bbr_alg_probe_bw_cruise) {
        cap = BBRInflightWithHeadroom(bbr_state, path_x);
    }

    /* apply inflight_lo (possibly infinite): */
    if (cap > bbr_state->inflight_lo) {
        cap = bbr_state->inflight_lo;
    }
    if (cap < BBRMinPipeCwnd * path_x->send_mtu) {
        cap = BBRMinPipeCwnd * path_x->send_mtu;
    }
    if (path_x->cwin > cap) {
        path_x->cwin = cap;
    }
}
```

### Rust body
```rust
        if self.is_in_a_probe_bw_state() && self.state != BbrAlgState::ProbeBwCruise {
            if self.inflight_hi > 0 {
                cap = self.inflight_hi;
            }
        } else if self.state == BbrAlgState::ProbeRtt || self.state == BbrAlgState::ProbeBwCruise {
```
