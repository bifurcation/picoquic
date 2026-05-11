# Phase 4E repair confirmed translation mismatches

You are repairing Phase 4D `needs_fix` entries.  Phase 4D
already performed deeper classification and concluded that
these Rust translations need repair.

Rules:

* Edit Rust only.  Do not edit C sources.
* Keep edits limited to the owned Rust file(s) for this batch
  unless a directly related helper in `rs/fq/` must change.
* Preserve safe, idiomatic Rust and existing public API shape
  unless the current shape cannot express the C behavior.
* Do not replace code with stubs, placeholders, fabricated
  defaults, or weaker behavior.
* If deeper repair inspection proves Phase 4D was mistaken,
  report outcome `ok` and do not edit source.
* The driver will run a separate read-only re-triage before
  recording any `fixed` or `ok` result as resolved.
* Report `blocked` only with a concrete human-actionable
  reason.

Owned Rust file(s): `rs/fq/src/cc_common.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/cc_common.c:picoquic_cc_slow_start_increase`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks cnx->cwin_blocked directly; Rust substitutes bytes_in_transit < cwin/otherwise, which is not body-visible equivalent.
* Phase 4D analysis: C uses the connection-level cwin_blocked latch, which can differ from current path bytes_in_transit >= cwin because it is set by sender blockage conditions, can include connection-wide cwin_max, and persists until prepare_segment resets it. Rust's path-local approximation is observably different.
* Phase 4D fix note: Repair slow-start growth to read the connection-level cwin_blocked state, either by passing connection context into the helper/callers or by maintaining an equivalent path-visible latch set from the same sender conditions.
* C source: `picoquic/cc_common.c:210-222`
* C signature: `uint64_t picoquic_cc_slow_start_increase(picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/cc_common.rs:317-416`
* Rust item: `slow_start_increase`

### C body
```c
uint64_t picoquic_cc_slow_start_increase(picoquic_path_t * path_x, uint64_t nb_delivered) {
    /* App limited. */
    /* TODO discuss
     * path_x->cwin < path_x->bytes_in_transit returns false in cc code
     * path_x->cnx->cwin_blocked is set to true
     * (path_x->cwin < path_x->bytes_in_transit) != path_x->cnx->cwin_blocked?
     */
    if (!path_x->cnx->cwin_blocked) {
        return 0;
    }

    return nb_delivered;
}
```

### Rust body
```rust

impl PathCc for Path {
    fn lowest_not_ack(&self, connection: &Connection) -> u64 {
        let pkt_ctx = if connection.is_multipath_enabled {
            &self.pkt_ctx
        } else {
            &connection.pkt_ctx[crate::PacketContext::Application as usize]
        };

        pkt_ctx
            .pending
            .keys()
            .next()
            .copied()
            .unwrap_or(pkt_ctx.highest_acknowledged.wrapping_add(1))
    }

    fn slow_start_increase(&self, nb_delivered: u64) -> u64 {
        // C body checks cnx->cwin_blocked.  Path has no back-pointer to
        // Connection, so approximate with bytes_in_transit >= cwin, which is
        // the condition that sets cwin_blocked in the C library.
        if self.bytes_in_transit < self.cwin {
            0
        } else {
            nb_delivered
        }
    }

    fn slow_start_increase_ex(&self, nb_delivered: u64, in_css: bool) -> u64 {
        if in_css {
            self.slow_start_increase(nb_delivered / HYSTART_PP_CSS_GROWTH_DIVISOR)
        } else {
            self.slow_start_increase(nb_delivered)
        }
    }

    fn slow_start_increase_ex2(&self, nb_delivered: u64, in_css: bool, prague_alpha: u64) -> u64 {
        if prague_alpha != 0 {
            let delta = if self.smoothed_rtt <= TARGET_RENO_RTT {
                nb_delivered * (1024 - prague_alpha) / 1024
            } else {
                nb_delivered * self.smoothed_rtt.ticks() * (1024 - prague_alpha)
                    / TARGET_RENO_RTT.ticks()
                    / 1024
            };
            self.slow_start_increase_ex(delta, in_css)
        } else {
            self.slow_start_increase_ex(nb_delivered, in_css)
        }
    }

    fn update_target_cwin_estimation(&self) -> u64 {
        // BYTES_FROM_RATE(smoothed_rtt, peak_bandwidth_estimate) = rtt_us * bps / 1_000_000
        let max_win = self.smoothed_rtt.ticks() * self.peak_bandwidth_estimate / 1_000_000;
        let min_win = max_win / 2;
        if min_win > self.cwin {
            min_win
        } else {
            self.cwin
        }
    }

    fn update_cwin_for_long_rtt(&self) -> u64 {
        let rtt_cap = if self.rtt_min > TARGET_SATELLITE_RTT {
            TARGET_SATELLITE_RTT
        } else {
            self.rtt_min
        };
        let min_cwnd =
            (CWIN_INITIAL as f64 * rtt_cap.ticks() as f64 / TARGET_RENO_RTT.ticks() as f64) as u64;
        if min_cwnd > self.cwin {
            min_cwnd
        } else {
```
