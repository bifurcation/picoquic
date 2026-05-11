# Phase 4D deep translation classification

You are classifying Phase 4C non-OK C/Rust function-pair
audit entries.  Phase 4C was intentionally body-only and
shallow; Phase 4D classification is allowed to inspect
broader context.

For each entry:

1. Read the C function and any directly relevant C context:
   types, constants/macros, helper callees, and callers when
   needed to understand observable behavior.
2. Read the Rust function in context, including local types,
   helpers, tests, and nearby translated functions.
3. Decide whether the Phase 4C concern is a false positive.

Do not edit files in this classification pass.  Report:

* `ok` when the Rust behavior is acceptable after deeper
  inspection.
* `needs_fix` when the Rust translation is actually wrong and
  should be repaired in a later 4D repair pass.
* `blocked` only when the analysis cannot be completed without
  a concrete external decision or missing dependency.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short deeper-review conclusion","fix_summary":"empty unless outcome is needs_fix","files_changed":[],"verification":[]}]}
```

Entries:

## `picoquic/bytestream.c:byteread_vint`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C decodes a varint, advances ptr, and returns success or error; Rust shown only performs the empty-buffer error check.
* C source: `picoquic/bytestream.c:147-161`
* C signature: `int byteread_vint(bytestream *, uint64_t *)`
* Rust source: `rs/fq/src/bytestream.rs:402-406`
* Rust item: `read_varint`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 1) {
        return bytestream_error(s);
    }

    size_t len = picoquic_varint_decode(s->data + s->ptr, s->size - s->ptr, value);
    if (len == 0) {
        return bytestream_error(s);
    } else {
        s->ptr += len;
        return 0;
    }
}
```

### Rust body
```rust
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
```

## `picoquic/bytestream.c:bytestream_reset`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C only sets ptr to 0; Rust replaces an entire BBR state object with defaults, clearly unrelated.
* C source: `picoquic/bytestream.c:98-101`
* C signature: `void bytestream_reset(bytestream *)`
* Rust source: `rs/fq/src/bbr1.rs:1089-1139`
* Rust item: `reset`

### C body
```c
{
    s->ptr = 0;
}
```

### Rust body
```rust
        *self = Self {
            state: Bbr1AlgState::Startup,
            btl_bw: 0,
            next_round_delivered: 0,
            btl_bw_filter: [0; BBR1_BTL_BW_FILTER_LENGTH],
            full_bw: 0,
            rt_prop: 0,
            rt_prop_stamp: 0,
            cycle_stamp: 0,
            probe_rtt_done_stamp: 0,
            prior_cwnd: 0,
            prior_in_flight: 0,
            bytes_delivered: 0,
            send_quantum: 0,
            rtt_filter: MinMaxRtt::default(),
            target_cwnd: 0,
            pacing_gain: 0.0,
            cwnd_gain: 0.0,
            pacing_rate: 0.0,
            cycle_index: 0,
            cycle_start: 0,
            round_count: 0,
            full_bw_count: 0,
            lt_rtt_cnt: 0,
            lt_bw: 0,
            lt_last_stamp: 0,
            previous_round_lost: 0,
            previous_sampling_delivered: 0,
            previous_sampling_lost: 0,
            loss_interval_start: 0,
            congestion_sequence: 0,
            cwin_before_suspension: 0,
            option_string: None,
            wifi_shadow_rtt: 0,
            quantum_ratio: 0.0,
            filled_pipe: false,
            round_start: false,
            rt_prop_expired: false,
            probe_rtt_round_done: false,
            idle_restart: false,
            packet_conservation: false,
            btl_bw_increased: false,
            lt_use_bw: false,
            lt_is_sampling: false,
            last_loss_was_timeout: false,
            cycle_on_loss: false,
            is_suspended: false,
            is_suspension_nearly_over: false,
        };
```

## `picoquic/cc_common.c:picoquic_cc_hystart_test`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only implements the initial early return and omits the RTT filtering, state updates, excess counting, and true-return path visible in C.
* C source: `picoquic/cc_common.c:167-208`
* C signature: `int picoquic_cc_hystart_test(picoquic_min_max_rtt_t *, uint64_t, uint64_t, uint64_t, int)`
* Rust source: `rs/fq/src/cc_common.rs:193-202`
* Rust item: `hystart_test`

### C body
```c
{
    int ret = 0;

    /* Add silly instruction to bypass "argument not use" warning without changing the signature */
    if (is_one_way_delay_enabled && rtt_measurement == 0) {
        return 0;
    }

    if(current_time > rtt_track->last_rtt_sample_time + 1000) {
        picoquic_cc_filter_rtt_min_max(rtt_track, rtt_measurement);
        rtt_track->last_rtt_sample_time = current_time;

        if (rtt_track->is_init) {
            uint64_t delta_max;

            if (rtt_track->rtt_filtered_min == 0 ||
                rtt_track->rtt_filtered_min > rtt_track->sample_max) {
                rtt_track->rtt_filtered_min = rtt_track->sample_max;
            }
            delta_max = rtt_track->rtt_filtered_min / 4;
            if (delta_max < packet_time) {
                delta_max = packet_time;
            }

            if (rtt_track->sample_min > rtt_track->rtt_filtered_min) {
                if (rtt_track->sample_min > rtt_track->rtt_filtered_min + delta_max) {
                    rtt_track->nb_rtt_excess++;
                    if (rtt_track->nb_rtt_excess >= PICOQUIC_MIN_MAX_RTT_SCOPE) {
                        /* RTT increased too much, get out of slow start! */
                        ret = 1;
                    }
                }
            }
            else {
                rtt_track->nb_rtt_excess = 0;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
        if is_one_way_delay_enabled && rtt_measurement == Duration::from_ticks(0) {
            return false;
        }
```

## `picoquic/config.c:picoquic_config_option_letters`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a test asserting a fixed string, not an implementation that fills option_string, sets string_length, and returns ret.
* C source: `picoquic/config.c:555-578`
* C signature: `int picoquic_config_option_letters(char *, size_t, size_t *)`
* Rust source: `rs/fq/src/tests/config.rs:438-441`
* Rust item: `config_option_letters`

### C body
```c
{
    size_t l = 0;
    int ret = 0;

    for (size_t i = 0; l + 1 < string_max && i < option_table_size; i++) {
        option_string[l++] = option_table[i].option_letter;
        if (option_table[i].nb_params_required > 0) {
            if (l + 1 < string_max) {
                option_string[l++] = ':';
            }
            else {
                l--;
                ret = -1;
                break;
            }
        }
    }
    option_string[l] = 0;
    if (string_length != NULL) {
        *string_length = l;
    }
    return ret;
}
```

### Rust body
```rust
fn config_option_letters() {
    let expected = "c:k:p:v:o:w:x:rR:s:XS:G:H:P:O:Me:C:i:l:Lb:q:m:n:a:t:zI:d:DQT:N:B:F:VU:0j:W:8J:E:y:K:Z:4:6:h";
    assert_eq!(Config::option_letters(), expected);
}
```

## `picoquic/dualq_aqm.c:dualq_update_it`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust implements the dequeue/enqueue loop but omits the C update_next check, dualq_pi2_update call, and update_next advancement.
* C source: `picoquic/dualq_aqm.c:323-345`
* C signature: `void dualq_update_it(dualq_state_t *, picoquictest_sim_link_t *, uint64_t)`
* Rust source: `rs/fq/src/tests/dualq.rs:381-388`
* Rust item: `update_it`

### C body
```c
{
    picoquictest_sim_packet_t* packet;
    int should_drop;
    
    while (link->queue_time <= current_time) {
        if ((packet = dualq_dequeue_one(dualq, current_time, &should_drop)) != NULL) {
            picoquictest_sim_link_enqueue(link, packet, current_time, should_drop);
        }
        else {
            break;
        }
    }

    if (current_time >= dualq->update_next) {
        dualq_pi2_update(dualq, current_time);
        dualq->update_next = current_time + dualq->Tupdate;
    }
}
```

### Rust body
```rust
        while link.queue_time.ticks() <= current_time.ticks() {
            if let Some((packet, should_drop)) = self.dequeue_one(current_time) {
                link.enqueue(packet, current_time, should_drop);
            } else {
                break;
            }
        }
```
