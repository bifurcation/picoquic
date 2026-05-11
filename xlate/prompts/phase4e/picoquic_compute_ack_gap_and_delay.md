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

Owned Rust file(s): `rs/fq/src/internal.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/frames.c:picoquic_compute_ack_gap_and_delay`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust uses different direct formulas and lacks the C correction block based on smoothed RTT, return data rate, ACK transmission time, and final gap cap condition.
* Phase 4D analysis: Real mismatch: Rust computes packet window/gap with different direct formulas and omits C's high-smoothed-RTT conservative correction plus the final 32-packet cap condition.
* Phase 4D fix note: Rewrite `Connection::compute_ack_gap_and_delay` to use the translated helper behavior for packet window, ACK delay, and ACK gap, then add the smoothed-RTT correction block and final gap cap.
* C source: `picoquic/frames.c:3066-3130`
* C signature: `void picoquic_compute_ack_gap_and_delay(picoquic_cnx_t *, uint64_t, uint64_t, uint64_t, uint64_t *, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:8852-8925`
* Rust item: `compute_ack_gap_and_delay`

### C body
```c
{
    uint64_t nb_packets = picoquic_compute_packets_in_window(cnx, data_rate);

    *ack_delay_max = picoquic_compute_ack_delay_max(cnx, rtt, remote_min_ack_delay);
    *ack_gap = picoquic_compute_ack_gap(cnx, data_rate, nb_packets);

    if (2 * cnx->path[0]->smoothed_rtt > 3 * cnx->path[0]->rtt_min) {
        uint64_t return_data_rate = 0;

        /* This code kicks in when the smoothed RTT is larger than 1.5 times the RTT Min.
         * If that is the case, the default computation of ACK gap and ACK delay may
         * be wrong, and a more conservative computation is required.
         * This code assume that ACK gap and ACK delay are already computed using
         * the default algorithms.
         */
        if (cnx->is_ack_frequency_negotiated) {
            return_data_rate = cnx->path[0]->receive_rate_max;
        }
        else {
            return_data_rate = cnx->path[0]->bandwidth_estimate;
        }

        if (nb_packets < 2) {
            nb_packets = 2;
        }
        if (return_data_rate > 0) {
            /* Estimate of ACK size = L2 + IPv6 + UDP + padded ACK */
            const uint64_t ack_size = 12 + 40 + 8 + 55;
            /* Estimate of ACK transmission time *in microseconds */
            uint64_t ack_transmission_time = (ack_size * 1000000) / return_data_rate;
            /* if ACK transmission time > ack delay, perform correction */
            if (ack_transmission_time > * ack_delay_max) {
                *ack_delay_max = ack_transmission_time;
                if (*ack_delay_max > PICOQUIC_ACK_DELAY_MAX) {
                    *ack_delay_max = PICOQUIC_ACK_DELAY_MAX;
                }
            }
            /* if ack gap smaller than ack time fraction of CWIN, perform correction */
            uint64_t rtt_target = (cnx->path[0]->smoothed_rtt + cnx->path[0]->rtt_min) / 2;

            if (!cnx->path[0]->is_ssthresh_initialized) {
                nb_packets /= 2;
            }

            uint64_t nb_ack_per_rtt = (*ack_gap > 0) ? (nb_packets + *ack_gap - 1) / (*ack_gap):nb_packets;
            if (nb_ack_per_rtt * (*ack_delay_max) > rtt_target) {
                uint64_t nb_acks_max = cnx->path[0]->smoothed_rtt / (*ack_delay_max);
                if (nb_acks_max <= 1) {
                    *ack_gap = nb_packets;
                }
                else {
                    uint64_t ack_gap_min = (nb_packets + nb_acks_max - 1) / nb_acks_max;
                    if (*ack_gap < ack_gap_min) {
                        *ack_gap = ack_gap_min;
                    }
                }
            }
        }
    }
    if (cnx->path[0]->rtt_min < *ack_delay_max * 4 && *ack_gap > 32) {
        *ack_gap = 32;
    }
}
```

### Rust body
```rust
    pub fn compute_ack_gap_and_delay(
        &self,
        rtt: Duration,
        remote_min_ack_delay: u64,
        data_rate: u64,
        ack_gap: &mut u64,
        ack_delay_max: &mut u64,
    ) {
        let first_path = self.paths.first();
        let rtt_ticks = rtt.ticks();
        let bytes_in_window = data_rate
            .saturating_mul(rtt_ticks)
            .saturating_div(1_000_000);
        let mut nb_packets = (bytes_in_window / MAX_PACKET_SIZE as u64).max(2);

        *ack_delay_max = (rtt_ticks / 4).min(ACK_DELAY_MAX.ticks());
        if !self.is_ack_frequency_negotiated
            && first_path
                .map(|p| !p.is_ssthresh_initialized)
                .unwrap_or(true)
        {
            *ack_delay_max /= 2;
        }
        *ack_delay_max = (*ack_delay_max).max(remote_min_ack_delay);

        if self.is_ack_frequency_negotiated
            && first_path
                .map(|p| !p.is_ssthresh_initialized)
                .unwrap_or(false)
        {
            nb_packets /= 2;
        }

        if let Some(path) = first_path
            && path.rtt_min < Duration::from_ticks(4 * ACK_DELAY_MIN.ticks())
        {
            let mult = if path.rtt_min > ACK_DELAY_MIN {
                (4 * ACK_DELAY_MIN.ticks()) / path.rtt_min.ticks().max(1)
            } else {
                4
            };
            nb_packets = nb_packets.saturating_mul(mult);
        }

        let mut gap = nb_packets.div_ceil(4);
        let mut gap_min = 2;
        if data_rate > BANDWIDTH_MEDIUM {
            gap_min = if first_path
                .map(|p| p.rtt_min > TARGET_RENO_RTT)
                .unwrap_or(false)
            {
                10
            } else {
                4
            };
        }
        if gap < gap_min {
            gap = gap_min;
        } else if gap > 32 {
            let cc_number = self
                .congestion_alg
                .map(|cc| cc.congestion_algorithm_number)
                .unwrap_or(CC_ALGO_NUMBER_NEW_RENO);
            if self.is_multipath_enabled
                || cc_number == CC_ALGO_NUMBER_NEW_RENO
                || cc_number == CC_ALGO_NUMBER_FAST
            {
```
