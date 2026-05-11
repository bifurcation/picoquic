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

## `picoquic/bbr.c:BBRBoundBWForModel`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown only applies the bw_hi bound and omits setting bw to max_bw and bounding by bw_lo.
* C source: `picoquic/bbr.c:1094-1104`
* C signature: `void BBRBoundBWForModel(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:362-371`
* Rust item: `bound_bw_for_model`

### C body
```c
static void  BBRBoundBWForModel(picoquic_bbr_state_t* bbr_state) {
    /* set bw = min(max_bw, bw_lo, bw_hi)  */
    bbr_state->bw = bbr_state->max_bw;
    if (bbr_state->bw > bbr_state->bw_lo) {
        bbr_state->bw = bbr_state->bw_lo;
    }
    /* TODO: remove the test bw_hi != 0 once variables properly initialized. */
    if (bbr_state->bw > bbr_state->bw_hi && bbr_state->bw_hi != 0) {
        bbr_state->bw = bbr_state->bw_hi;
    }
}
```

### Rust body
```rust
        if self.bw > self.bw_hi && self.bw_hi != 0 {
            self.bw = self.bw_hi;
        }
```
