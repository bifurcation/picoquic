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

## `picoquic/cc_common.c:picoquic_cc_slow_start_increase`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks cnx->cwin_blocked directly; Rust substitutes bytes_in_transit < cwin/otherwise, which is not body-visible equivalent.
* Prior Phase 4D analysis: C uses the connection-level cwin_blocked latch, which can differ from current path bytes_in_transit >= cwin because it is set by sender blockage conditions, can include connection-wide cwin_max, and persists until prepare_segment resets it. Rust's path-local approximation is observably different.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Phase 4D was correct: the Rust helper used path-local bytes_in_transit/cwin instead of the connection-level cwin_blocked latch used by C.
* Phase 4E fix summary: Changed slow_start_increase, slow_start_increase_ex, and slow_start_increase_ex2 to take Connection and gate growth on connection.cwin_blocked; updated BBR, BBR1, Cubic, and Prague callers to pass the connection context.
* C source: `picoquic/cc_common.c:210-222`
* C signature: `uint64_t picoquic_cc_slow_start_increase(picoquic_path_t *, uint64_t)`
* Current Rust source: `rs/fq/src/cc_common.rs:318-439`
* Current Rust item: `slow_start_increase`

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

### Current Rust body
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

    fn slow_start_increase(&self, connection: &Connection, nb_delivered: u64) -> u64 {
        if connection.cwin_blocked {
            nb_delivered
        } else {
            0
        }
    }

    fn slow_start_increase_ex(
        &self,
        connection: &Connection,
        nb_delivered: u64,
        in_css: bool,
    ) -> u64 {
        if in_css {
            self.slow_start_increase(connection, nb_delivered / HYSTART_PP_CSS_GROWTH_DIVISOR)
        } else {
            self.slow_start_increase(connection, nb_delivered)
        }
    }

    fn slow_start_increase_ex2(
        &self,
        connection: &Connection,
        nb_delivered: u64,
        in_css: bool,
        prague_alpha: u64,
    ) -> u64 {
        if prague_alpha != 0 {
            let delta = if self.smoothed_rtt <= TARGET_RENO_RTT {
                nb_delivered * (1024 - prague_alpha) / 1024
            } else {
                nb_delivered * self.smoothed_rtt.ticks() * (1024 - prague_alpha)
                    / TARGET_RENO_RTT.ticks()
                    / 1024
            };
            self.slow_start_increase_ex(connection, delta, in_css)
        } else {
            self.slow_start_increase_ex(connection, nb_delivered, in_css)
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
            self.cwin
        }
    }
}
```
