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

## `picoquic/quicctx.c:picoquic_set_null_verifier`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C disposes the verify certificate callback, while Rust is only a placeholder comment with no behavior.
* C source: `picoquic/quicctx.c:1182-1185`
* C signature: `void picoquic_set_null_verifier(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:1641-1643`
* Rust item: `set_null_verifier`

### C body
```c
void picoquic_set_null_verifier(picoquic_quic_t* quic) {
    PICOQUIC_THREAD_CHECK(quic);
    picoquic_dispose_verify_certificate_callback(quic);
}
```

### Rust body
```rust
    pub fn set_null_verifier(&mut self) {
        // Delegates to TLS backend; complex.
    }
```

## `picoquic/quicctx.c:picoquic_set_transport_parameters`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C clones transport parameters then updates max packet size and local flow-control stream/data limits; Rust only clones the parameters.
* C source: `picoquic/quicctx.c:4415-4433`
* C signature: `void picoquic_set_transport_parameters(picoquic_cnx_t *, const picoquic_tp_t *)`
* Rust source: `rs/fq/src/lib.rs:1752-1754`
* Rust item: `set_transport_parameters`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->local_parameters = *tp;

    if (cnx->quic->mtu_max > 0 && cnx->local_parameters.max_packet_size == 0)
    {
        cnx->local_parameters.max_packet_size = cnx->quic->mtu_max - 
            PICOQUIC_MTU_OVERHEAD((struct sockaddr*)&(cnx->path[0])->first_tuple->peer_addr);
    }

    /* Initialize local flow control variables to advertised values */

    cnx->maxdata_local = ((uint64_t)cnx->local_parameters.initial_max_data);
    cnx->max_stream_id_bidir_local = STREAM_ID_FROM_RANK(
        cnx->local_parameters.initial_max_stream_id_bidir, cnx->client_mode, 0);
    cnx->max_stream_id_unidir_local = STREAM_ID_FROM_RANK(
        cnx->local_parameters.initial_max_stream_id_unidir, cnx->client_mode, 1);
}
```

### Rust body
```rust
    pub fn set_transport_parameters(&mut self, tp: &TransportParameters) {
        self.local_parameters = tp.clone();
    }
```

## `picoquic/quicctx.c:picoquic_test_and_signal_new_path_allowed`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C checks subscription/notified state, calls check_new_path_allowed, sets notified, and invokes callback; Rust shown body only sets is_notified_that_path_is_allowed.
* C source: `picoquic/quicctx.c:2370-2383`
* C signature: `void picoquic_test_and_signal_new_path_allowed(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:14933-14939`
* Rust item: `test_and_signal_new_path_allowed`

### C body
```c
{
    if (cnx->is_subscribed_to_path_allowed &&
        !cnx->is_notified_that_path_is_allowed)
    {
        if (picoquic_check_new_path_allowed(cnx, 0) == 0) {
            cnx->is_notified_that_path_is_allowed = 1;
            if (cnx->callback_fn != NULL) {
                (void)cnx->callback_fn(cnx, 0, NULL, 0, picoquic_callback_next_path_allowed, cnx->callback_ctx, NULL);
            }
        }
    }
}
```

### Rust body
```rust
        {
            self.is_notified_that_path_is_allowed = true;
        }
```

## `picoquic/sacks.c:picoquic_sack_item_nb_times_sent`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns sack_item->nb_times_sent[is_opportunistic]; shown Rust body defines record/reset mutators and no nb_times_sent return.
* C source: `picoquic/sacks.c:471-474`
* C signature: `int picoquic_sack_item_nb_times_sent(picoquic_sack_item_t *, int)`
* Rust source: `rs/fq/src/internal.rs:8673-8718`
* Rust item: `nb_times_sent`

### C body
```c
{
    return sack_item->nb_times_sent[is_opportunistic];
}
```

### Rust body
```rust
impl SackList {
    /// Bump the per-range send counter for the item at `token` and keep
    /// the per-bucket histogram in `rc` consistent.
    /// C: `picoquic_sack_item_record_sent`
    pub fn item_record_sent(&mut self, token: SackItemToken, is_opportunistic: i32) {
        let idx = is_opportunistic.clamp(0, 1) as usize;
        let old = match self.sack_items.get(token) {
            Some(item) => item.nb_times_sent[idx],
            None => return,
        };
        if (old as usize) < MAX_ACK_RANGE_REPEAT {
            self.rc[idx].range_counts[old as usize] -= 1;
        }
        let new = old + 1;
        if let Some(item) = self.sack_items.get_mut(token) {
            item.nb_times_sent[idx] = new;
        }
        if (new as usize) < MAX_ACK_RANGE_REPEAT {
            self.rc[idx].range_counts[new as usize] += 1;
        }
    }

    /// Reset the per-range send counters for the item at `token` to zero
    /// and keep the per-bucket histogram in `rc` consistent.
    /// C: `picoquic_sack_item_record_reset`
    pub fn item_record_reset(&mut self, token: SackItemToken) {
        let old = match self.sack_items.get(token) {
            Some(item) => item.nb_times_sent,
            None => return,
        };
        for (r, &old_count) in old.iter().enumerate() {
            if (old_count as usize) < MAX_ACK_RANGE_REPEAT {
                self.rc[r].range_counts[old_count as usize] -= 1;
            }
            self.rc[r].range_counts[0] += 1;
        }
        if let Some(item) = self.sack_items.get_mut(token) {
            item.nb_times_sent = [0; 2];
        }
    }
}
```

## `picoquic/sacks.c:picoquic_sack_list_last`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the end value of the last SACK item or 0; Rust resets the SACK list range and returns Result<(), Error>.
* C source: `picoquic/sacks.c:414-420`
* C signature: `uint64_t picoquic_sack_list_last(picoquic_sack_list_t *)`
* Rust source: `rs/fq/src/internal.rs:8548-8565`
* Rust item: `picoquic_sack_list_last`

### C body
```c
{
    picoquic_sack_item_t* last = picoquic_sack_last_item(sack_list);
    return (last == NULL) ? 0 : last->end_of_sack_range;
}
```

### Rust body
```rust
) -> Result<(), crate::Error> {
    sack_list.reset(range_min, range_max, current_time)
}
```
