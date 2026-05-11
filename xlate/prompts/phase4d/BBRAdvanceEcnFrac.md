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

## `picoquic/bbr.c:BBRAdvanceEcnFrac`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C only runs the ECN context update when round_start is true, but the shown Rust body lacks that outer round_start guard.
* C source: `picoquic/bbr.c:2288-2305`
* C signature: `void BBRAdvanceEcnFrac(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:1125-1144`
* Rust item: `advance_ecn_frac`

### C body
```c
{
    if (bbr_state->round_start) {
        picoquic_packet_context_t* pkt_ctx = BBRAccessEcnPacketContext(path_x);

        if (pkt_ctx != NULL) {
            if (pkt_ctx->ecn_ect1_total_remote < bbr_state->ecn_ect1_last_round ||
                pkt_ctx->ecn_ce_total_remote < bbr_state->ecn_ce_last_round) {
                bbr_state->ecn_alpha = 0;
            }
            else {
                bbr_state->ecn_alpha = (rs->ecn_frac + 15.0 * bbr_state->ecn_alpha) / 16;
            }
            bbr_state->ecn_ect1_last_round = pkt_ctx->ecn_ect1_total_remote;
            bbr_state->ecn_ce_last_round = pkt_ctx->ecn_ce_total_remote;
        }
    }
}
```

### Rust body
```rust
        if let Some(pkt_ctx) = access_ecn_packet_context(connection, path_x) {
            if pkt_ctx.ecn_ect1_total_remote < self.ecn_ect1_last_round
                || pkt_ctx.ecn_ce_total_remote < self.ecn_ce_last_round
            {
                self.ecn_alpha = 0.0;
            } else {
                self.ecn_alpha = (rs.ecn_frac + 15.0 * self.ecn_alpha) / 16.0;
            }
            self.ecn_ect1_last_round = pkt_ctx.ecn_ect1_total_remote;
            self.ecn_ce_last_round = pkt_ctx.ecn_ce_total_remote;
        }
```
