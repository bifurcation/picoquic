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

## `picoquic/quicctx.c:picoquic_probe_new_tuple`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C verifies/creates a tuple, assigns a peer connection ID, sets challenge fields, and returns status; Rust is a placeholder returning Generic error.
* C source: `picoquic/quicctx.c:2437-2466`
* C signature: `int picoquic_probe_new_tuple(picoquic_cnx_t *, picoquic_path_t *, const struct sockaddr *, const struct sockaddr *, int, uint64_t, int)`
* Rust source: `rs/fq/src/lib.rs:2462-2473`
* Rust item: `probe_new_tuple`

### C body
```c
{
    int ret;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    ret = picoquic_verify_proposed_tuple(cnx, &addr_peer, &addr_local, &if_index);

    /* TODO: check whether that tuple already exists */

    /* Verify that a CID is available */
    ret = picoquic_check_cid_for_new_tuple(cnx, path_x->unique_path_id);

    if (ret == 0) {
        picoquic_tuple_t * tuple = picoquic_create_tuple(path_x, addr_local, addr_peer, if_index);
        if (tuple == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
        else {
            ret = picoquic_assign_peer_cnxid_to_tuple(cnx, path_x, tuple);
            if (ret == 0) {
                /* There was no NAT ongoing NAT rebinding, we created one, we need to initiate path challenges. */
                picoquic_set_tuple_challenge(tuple, current_time, cnx->quic->use_constant_challenges);
                tuple->challenge_required = 1;
                tuple->to_preferred_address = to_preferred_address;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves tuple creation within a path.
        Err(Error::Generic)
    }
```

## `picoquic/quicctx.c:picoquic_renew_path_connection_id`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C obtains a remote stashed CID, enforces migration-disabled checks, dereferences the old remote CID, installs the new remote CID, and may register a secret; Rust creates a local connection ID and queues retirement of the old local sequence.
* C source: `picoquic/quicctx.c:3278-3323`
* C signature: `int picoquic_renew_path_connection_id(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:5361-5378`
* Rust item: `renew_path_connection_id`

### C body
```c
{
    int ret = 0;
    picoquic_remote_cnxid_t* stashed = NULL;
    uint64_t cid_path_id = (cnx->is_multipath_enabled) ? path_x->unique_path_id : 0;
    picoquic_remote_cnxid_stash_t* cnxid_stash = picoquic_find_or_create_remote_cnxid_stash(cnx, cid_path_id, 0);

    if (cnxid_stash == NULL) {
        ret = PICOQUIC_ERROR_CNXID_NOT_AVAILABLE;
    }
    else if ((cnx->remote_parameters.migration_disabled != 0 &&
            path_x->first_tuple->p_remote_cnxid != NULL &&
            path_x->first_tuple->p_remote_cnxid->sequence >= cnxid_stash->retire_cnxid_before) ||
            cnx->local_parameters.migration_disabled != 0) {
            /* Do not switch cnx_id if migration is disabled */
            ret = PICOQUIC_ERROR_MIGRATION_DISABLED;
        }
    else {
        stashed = picoquic_obtain_stashed_cnxid(cnx, cid_path_id);

        if (stashed == NULL) {
            ret = PICOQUIC_ERROR_CNXID_NOT_AVAILABLE;
        }
        else if (path_x->first_tuple->p_remote_cnxid != NULL &&
            stashed->sequence == path_x->first_tuple->p_remote_cnxid->sequence) {
            /* If the available cnx_id is same as old one, we do nothing */
            ret = PICOQUIC_ERROR_CNXID_NOT_AVAILABLE;
        }
        else {
            picoquic_dereference_stashed_cnxid(cnx, path_x, 0);

            /* Install the new value */
            path_x->first_tuple->p_remote_cnxid = stashed;
            stashed->nb_path_references++;

            /* If default path, reset the secret pointer */
            if (path_x == cnx->path[0]) {
                ret = picoquic_register_net_secret(cnx);
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn renew_path_connection_id(&mut self, path_x: &mut Path) -> Result<(), crate::Error> {
        let old_sequence = path_x
            .tuples
            .first()
            .and_then(|tuple| tuple.local_connection_id)
            .and_then(|tok| self.local_connection_ids.get(tok))
            .map(|lcid| lcid.sequence);
        let token =
            self.create_local_connection_id(path_x.unique_path_id, None, self.start_time)?;
        if let Some(tuple) = path_x.tuples.first_mut() {
            tuple.local_connection_id = Some(token);
        }
        path_x.path_cid_rotated = true;
        if let Some(sequence) = old_sequence {
            self.queue_retire_connection_id_frame(path_x.unique_path_id, sequence)?;
        }
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_set_app_wake_time`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets app_wake_time and may reinsert by wake time; Rust body shown sets desired_version instead.
* C source: `picoquic/quicctx.c:5191-5199`
* C signature: `void picoquic_set_app_wake_time(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:2421-2428`
* Rust item: `set_app_wake_time`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->app_wake_time = app_wake_time;
    if (cnx->app_wake_time != 0 && cnx->app_wake_time < cnx->next_wake_time) {
        picoquic_reinsert_by_wake_time(cnx->quic, cnx, app_wake_time);
    }
}
```

### Rust body
```rust
    pub fn set_desired_version(&mut self, desired_version: u32) {
        self.desired_version = desired_version;
    }
```

## `picoquic/quicctx.c:picoquic_set_default_crypto_epoch_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets crypto_epoch_length_max with zero mapped to a default; Rust body is a getter returning crypto_epoch_length_max and performs no set or default handling.
* C source: `picoquic/quicctx.c:1004-1009`
* C signature: `void picoquic_set_default_crypto_epoch_length(picoquic_quic_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:1792-1799`
* Rust item: `set_default_crypto_epoch_length`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->crypto_epoch_length_max = (crypto_epoch_length_max == 0) ?
        PICOQUIC_DEFAULT_CRYPTO_EPOCH_LENGTH : crypto_epoch_length_max;
}
```

### Rust body
```rust
    pub fn default_crypto_epoch_length(&self) -> u64 {
        self.crypto_epoch_length_max
    }
```

## `picoquic/quicctx.c:picoquic_set_feedback_loss_notification`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets lost feedback notification requirement; Rust sets forced probe-up requirement.
* C source: `picoquic/quicctx.c:5406-5410`
* C signature: `void picoquic_set_feedback_loss_notification(picoquic_cnx_t *, unsigned int)`
* Rust source: `rs/fq/src/lib.rs:4639-4646`
* Rust item: `set_feedback_loss_notification`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->is_lost_feedback_notification_required = should_notify;
}
```

### Rust body
```rust
    pub fn request_forced_probe_up(&mut self, request_forced_probe_up: bool) {
        self.is_forced_probe_up_required = request_forced_probe_up;
    }
```
