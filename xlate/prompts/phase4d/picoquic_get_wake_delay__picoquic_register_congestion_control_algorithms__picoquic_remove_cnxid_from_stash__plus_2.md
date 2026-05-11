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

## `picoquic/quicctx.c:picoquic_get_wake_delay`
* Phase 4C status: `suspect`
* Phase 4C rationale: C computes next_wake_time through picoquic_get_wake_time and clamps using current_time + delay_max; Rust reads next_wake_time directly and casts the delta to i64 before min.
* C source: `picoquic/quicctx.c:1593-1612`
* C signature: `int64_t picoquic_get_wake_delay(picoquic_cnx_t *, uint64_t, int64_t)`
* Rust source: `rs/fq/src/lib.rs:2822-2831`
* Rust item: `wake_delay`

### C body
```c
{
    /* See get_next_wake_delay for reasoning about integer overflow */
    uint64_t next_wake_time = picoquic_get_wake_time(cnx, current_time);
    int64_t wake_delay = 0;

    if (next_wake_time > current_time) {
        uint64_t delta_m = current_time + delay_max;

        if (next_wake_time >= delta_m) {
            wake_delay = delay_max;
        }
        else {
            wake_delay = (int64_t)(next_wake_time - current_time);
        }
    }

    return wake_delay;
}
```

### Rust body
```rust
    pub fn wake_delay(&self, current_time: Instant, delay_max: i64) -> i64 {
        let next = self.next_wake_time.ticks();
        let now = current_time.ticks();
        if next <= now {
            0
        } else {
            let delta = (next - now) as i64;
            delta.min(delay_max)
        }
    }
```

## `picoquic/quicctx.c:picoquic_register_congestion_control_algorithms`
* Phase 4C status: `suspect`
* Phase 4C rationale: C overwrites the global algorithm registry and count; Rust uses OnceLock set semantics, making later registrations no-ops and no visible count is stored.
* C source: `picoquic/quicctx.c:5304-5309`
* C signature: `void picoquic_register_congestion_control_algorithms(const picoquic_congestion_algorithm_t **, size_t)`
* Rust source: `rs/fq/src/lib.rs:4554-4558`
* Rust item: `register_congestion_control_algorithms`

### C body
```c
{
    picoquic_congestion_control_algorithms = alg;
    picoquic_nb_congestion_control_algorithms = nb_algorithms;
}
```

### Rust body
```rust
pub fn register_congestion_control_algorithms(alg: &'static [&'static CongestionAlgorithm]) {
    // Best-effort set: if the registry was already initialised, this is a no-op
    // (OnceLock semantics).
    let _ = CC_ALGORITHM_REGISTRY.set(alg.to_vec());
}
```

## `picoquic/quicctx.c:picoquic_remove_cnxid_from_stash`
* Phase 4C status: `suspect`
* Phase 4C rationale: C validates or searches for the removed element using the previous pointer before unlinking; Rust removes directly by index and does not show equivalent previous validation/search logic.
* C source: `picoquic/quicctx.c:3055-3091`
* C signature: `picoquic_remote_cnxid_t * picoquic_remove_cnxid_from_stash(picoquic_cnx_t *, picoquic_remote_cnxid_stash_t *, picoquic_remote_cnxid_t *, picoquic_remote_cnxid_t *)`
* Rust source: `rs/fq/src/internal.rs:5158-5174`
* Rust item: `remove_connection_id_from_stash`

### C body
```c
{
    picoquic_remote_cnxid_t* stashed = NULL;

    if (cnx != NULL && remote_cnxid_stash != NULL && remote_cnxid_stash->cnxid_stash_first != NULL && removed != NULL) {
        stashed = remote_cnxid_stash->cnxid_stash_first;
        /* Verify the value of the previous pointer */
        if (previous != NULL) {
            if (previous->next == removed) {
                stashed = removed;
            }
            else {
                previous = NULL;
            }
        }
        /* If the previous pointer was NULL or invalid, reset it */
        if (previous == NULL) {
            while (stashed != NULL && removed != stashed) {
                previous = stashed;
                stashed = stashed->next;
            }
        }
        /* Actually remove the element from the stash */
        if (stashed != NULL) {
            stashed = stashed->next;
            if (previous == NULL) {
                remote_cnxid_stash->cnxid_stash_first = stashed;
            }
            else {
                previous->next = stashed;
            }
            free(removed);
        }
    }
    return stashed;
}
```

### Rust body
```rust
    ) -> Option<usize> {
        let stash = self.remote_connection_id_stashes.get_mut(stash_index)?;
        if removed_index >= stash.connection_ids.len() {
            return None;
        }
        stash.connection_ids.remove(removed_index);
        // Return the index of the next live entry (same index since we removed one).
        if removed_index < stash.connection_ids.len() {
            Some(removed_index)
        } else {
            None
        }
    }
```

## `picoquic/quicctx.c:picoquic_set_default_connection_id_length`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only validates active-context restrictions when the requested length differs from the current length; Rust always applies validation and assignment.
* C source: `picoquic/quicctx.c:4688-4706`
* C signature: `int picoquic_set_default_connection_id_length(picoquic_quic_t *, uint8_t)`
* Rust source: `rs/fq/src/lib.rs:1839-1850`
* Rust item: `set_default_connection_id_length`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(quic);

    if (cid_length != quic->local_cnxid_length) {
        if (cid_length > PICOQUIC_CONNECTION_ID_MAX_SIZE) {
            ret = PICOQUIC_ERROR_CNXID_CHECK;
        }
        else if (quic->cnx_list != NULL) {
            ret = PICOQUIC_ERROR_CANNOT_CHANGE_ACTIVE_CONTEXT;
        }
        else {
            quic->local_cnxid_length = cid_length;
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn set_default_connection_id_length(&mut self, cid_length: u8) -> Result<(), Error> {
        if cid_length as usize > CONNECTION_ID_MAX_SIZE {
            return Err(Error::Protocol(InternalError::CnxidCheck as u64));
        }
        if self.current_number_connections > 0 {
            return Err(Error::Protocol(
                InternalError::CannotChangeActiveContext as u64,
            ));
        }
        self.local_connection_id_length = cid_length;
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_set_random_initial`
* Phase 4C status: `suspect`
* Phase 4C rationale: C clamps random_initial to 0, 1, or 2; Rust casts directly to u8, which differs for negative values and values above 2.
* C source: `picoquic/quicctx.c:4666-4671`
* C signature: `void picoquic_set_random_initial(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1112-1114`
* Rust item: `set_random_initial`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    /* If set, triggers randomization of initial PN numbers. */
    quic->random_initial = (random_initial > 1) ? 2 : ((random_initial > 0) ? 1 : 0);
}
```

### Rust body
```rust
    pub fn set_random_initial(&mut self, random_initial: i32) {
        self.random_initial = random_initial as u8;
    }
```
