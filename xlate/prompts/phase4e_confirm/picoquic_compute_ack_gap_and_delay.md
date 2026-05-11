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

## `picoquic/frames.c:picoquic_compute_ack_gap_and_delay`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust uses different direct formulas and lacks the C correction block based on smoothed RTT, return data rate, ACK transmission time, and final gap cap condition.
* Prior Phase 4D analysis: Real mismatch: Rust computes packet window/gap with different direct formulas and omits C's high-smoothed-RTT conservative correction plus the final 32-packet cap condition.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Rust now follows the translated helper sequence and includes the C conservative high-smoothed-RTT correction and final 32-packet cap.
* Phase 4E fix summary: Reworked compute_ack_gap_and_delay to call packet-window, ACK-delay, and ACK-gap helpers, then added return-rate ACK transmission correction, RTT-target gap correction, and final cap.
* C source: `picoquic/frames.c:3066-3130`
* C signature: `void picoquic_compute_ack_gap_and_delay(picoquic_cnx_t *, uint64_t, uint64_t, uint64_t, uint64_t *, uint64_t *)`
* Current Rust source: `rs/fq/src/internal.rs:8860-8925`
* Current Rust item: `compute_ack_gap_and_delay`

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

### Current Rust body
```rust
    ) {
        let mut nb_packets = self.compute_packets_in_window(data_rate);

        *ack_delay_max = self.compute_ack_delay_max(rtt.ticks(), remote_min_ack_delay);
        *ack_gap = self.compute_ack_gap(data_rate, nb_packets);

        let Some(path) = self.paths.first() else {
            return;
        };

        let smoothed_rtt = path.smoothed_rtt.ticks();
        let rtt_min = path.rtt_min.ticks();
        if smoothed_rtt.saturating_mul(2) > rtt_min.saturating_mul(3) {
            let return_data_rate = if self.is_ack_frequency_negotiated {
                path.receive_rate_max
            } else {
                path.bandwidth_estimate
            };

            if nb_packets < 2 {
                nb_packets = 2;
            }
            const ACK_SIZE: u64 = 12 + 40 + 8 + 55;
            if let Some(ack_transmission_time) = ACK_SIZE
                .saturating_mul(1_000_000)
                .checked_div(return_data_rate)
            {
                if ack_transmission_time > *ack_delay_max {
                    *ack_delay_max = ack_transmission_time.min(ACK_DELAY_MAX.ticks());
                }

                let rtt_target = smoothed_rtt.saturating_add(rtt_min) / 2;
                if !path.is_ssthresh_initialized {
                    nb_packets /= 2;
                }

                let nb_ack_per_rtt = if *ack_gap > 0 {
                    nb_packets.div_ceil(*ack_gap)
                } else {
                    nb_packets
                };
                if nb_ack_per_rtt.saturating_mul(*ack_delay_max) > rtt_target {
                    let nb_acks_max = smoothed_rtt / *ack_delay_max;
                    if nb_acks_max <= 1 {
                        *ack_gap = nb_packets;
                    } else {
                        let ack_gap_min = nb_packets.div_ceil(nb_acks_max);
                        if *ack_gap < ack_gap_min {
                            *ack_gap = ack_gap_min;
                        }
                    }
                }
            }
        }

        if rtt_min < (*ack_delay_max).saturating_mul(4) && *ack_gap > 32 {
            *ack_gap = 32;
        }
    }
```
