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

## `picoquic/quicctx.c:picoquic_obtain_stashed_cnxid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns a stashed connection ID pointer for the requested unique_path_id; Rust removes the first CID from the first stash and returns a boolean.
* C source: `picoquic/quicctx.c:3115-3121`
* C signature: `picoquic_remote_cnxid_t * picoquic_obtain_stashed_cnxid(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:16309-16316`
* Rust item: `obtain_stashed_cnxid`

### C body
```c
{
    picoquic_remote_cnxid_stash_t* stash = picoquic_find_or_create_remote_cnxid_stash(cnx, unique_path_id, 0);
    picoquic_remote_cnxid_t* stashed = picoquic_get_cnxid_from_stash(stash);
    
    return stashed;
}
```

### Rust body
```rust
    if let Some(stash) = cnx.remote_connection_id_stashes.first_mut() {
        if stash.connection_ids.is_empty() {
            return Ok(false);
        }
        stash.connection_ids.remove(0);
        return Ok(true);
    }
```

## `picoquic/quicctx.c:picoquic_register_path`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body conditionally calls picoquic_register_net_id only when the peer family is nonzero and local_cnxid_length is zero; Rust body unconditionally copies the first tuple peer address into registered_peer_addr when present.
* C source: `picoquic/quicctx.c:1868-1877`
* C signature: `void picoquic_register_path(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:4365-4369`
* Rust item: `register_path`

### C body
```c
{
    if (path_x->first_tuple->peer_addr.ss_family != 0 && cnx->quic->local_cnxid_length == 0) {
        (void)picoquic_register_net_id(cnx->quic, cnx, path_x);
    }
}
```

### Rust body
```rust
        if let Some(tuple) = path_x.tuples.first() {
            path_x.registered_peer_addr = tuple.peer_addr;
        }
```

## `picoquic/quicctx.c:picoquic_seed_bandwidth`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C unconditionally stores seed_rtt_min, seed_cwin, and seed_ip_addr fields; Rust updates first path rtt/cwin/bandwidth with extra zero checks and stores into path.ip_client_remote.
* C source: `picoquic/quicctx.c:4539-4549`
* C signature: `void picoquic_seed_bandwidth(picoquic_cnx_t *, uint64_t, uint64_t, const uint8_t *, uint8_t)`
* Rust source: `rs/fq/src/internal.rs:9087-9106`
* Rust item: `seed_bandwidth`

### C body
```c
{
    cnx->seed_rtt_min = rtt_min;
    cnx->seed_cwin = cwin;
    if (ip_addr_length > PICOQUIC_STORED_IP_MAX) {
        ip_addr_length = PICOQUIC_STORED_IP_MAX;
    }
    memcpy(cnx->seed_ip_addr, ip_addr, ip_addr_length);
    cnx->seed_ip_addr_length = ip_addr_length;
}
```

### Rust body
```rust
        if let Some(path) = self.paths.first_mut() {
            path.rtt_min = if rtt_min.ticks() == 0 {
                path.rtt_min
            } else {
                rtt_min
            };
            if cwin > 0 {
                path.cwin = cwin;
                path.bandwidth_estimate = cwin
                    .saturating_mul(1_000_000)
                    .saturating_div(path.rtt_min.ticks().max(1));
                path.bandwidth_estimate_max =
                    path.bandwidth_estimate_max.max(path.bandwidth_estimate);
            }
            let ip_bytes = stored_ip_bytes(ip_addr);
            let len = ip_bytes.len().min(path.ip_client_remote.len());
            path.ip_client_remote[..len].copy_from_slice(&ip_bytes[..len]);
            path.ip_client_remote_length = len as u8;
        }
```

## `picoquic/quicctx.c:picoquic_set_default_bdp_frame_option`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C assigns the BDP frame option field; Rust validates and sets the default connection ID length instead.
* C source: `picoquic/quicctx.c:1056-1060`
* C signature: `void picoquic_set_default_bdp_frame_option(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1832-1850`
* Rust item: `set_default_bdp_frame_option`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_send_receive_bdp_frame = bdp_option;
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

## `picoquic/quicctx.c:picoquic_set_default_spinbit_policy`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C accepts policies <= picoquic_spinbit_on and rejects larger values; Rust specifically rejects SpinbitVersion::On and accepts all other shown enum values.
* C source: `picoquic/quicctx.c:906-918`
* C signature: `int picoquic_set_default_spinbit_policy(picoquic_quic_t *, picoquic_spinbit_version_enum)`
* Rust source: `rs/fq/src/lib.rs:1677-1687`
* Rust item: `set_default_spinbit_policy`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(quic);

    if (default_spinbit_policy <= picoquic_spinbit_on) {
        quic->default_spin_policy = default_spinbit_policy;
    }
    else {
        ret = -1;
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // SpinbitVersion::On is server-only and must not be set as a default.
        if default_spinbit_policy == SpinbitVersion::On {
            return Err(Error::InvalidArgument);
        }
        self.default_spin_policy = default_spinbit_policy;
        Ok(())
    }
```
