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

## `picoquic/quicctx.c:picoquic_get_cnx_start_time`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body returns zero-RTT availability rather than the connection start time.
* C source: `picoquic/quicctx.c:4495-4499`
* C signature: `uint64_t picoquic_get_cnx_start_time(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3075-3082`
* Rust item: `start_time`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->start_time;
}
```

### Rust body
```rust
    pub fn is_0rtt_available(&self) -> bool {
        self.zero_rtt_data_accepted
    }
```

## `picoquic/quicctx.c:picoquic_get_default_connection_id_ttl`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns local_cnxid_ttl; Rust body shown sets mtu_max and returns nothing.
* C source: `picoquic/quicctx.c:4714-4718`
* C signature: `uint64_t picoquic_get_default_connection_id_ttl(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:1859-1866`
* Rust item: `default_connection_id_ttl`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->local_cnxid_ttl;
}
```

### Rust body
```rust
    pub fn set_mtu_max(&mut self, mtu_max: u32) {
        self.mtu_max = mtu_max;
    }
```

## `picoquic/quicctx.c:picoquic_get_local_if_index`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the first tuple if_index; Rust sets the local address.
* C source: `picoquic/quicctx.c:4453-4457`
* C signature: `unsigned long picoquic_get_local_if_index(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2916-2940`
* Rust item: `local_if_index`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->first_tuple->if_index;
}
```

### Rust body
```rust
    pub fn set_local_addr(&mut self, addr: &SocketAddr) -> Result<(), Error> {
        if let Some(path) = self.paths.first_mut()
            && let Some(tuple) = path.tuples.first_mut()
        {
            if !socket_addr_is_unspecified(&tuple.local_addr) {
                return Err(Error::Generic);
            }
            tuple.local_addr = *addr;
            return if socket_addr_is_unspecified(&tuple.local_addr) {
                Err(Error::Generic)
            } else {
                Ok(())
            };
        }
        Err(Error::InvalidArgument)
    }
```

## `picoquic/quicctx.c:picoquic_get_remote_error`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns remote_error; Rust returns application_error.
* C source: `picoquic/quicctx.c:5508-5512`
* C signature: `uint64_t picoquic_get_remote_error(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4477-4484`
* Rust item: `remote_error`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->remote_error;
}
```

### Rust body
```rust
    pub fn application_error(&self) -> u64 {
        self.application_error
    }
```

## `picoquic/quicctx.c:picoquic_insert_cnx_in_list`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C inserts a connection at the head of a list and increments current_number_connections; Rust body deletes a connection, removes table memberships/paths/list entries, decrements half-open count, and removes the token.
* C source: `picoquic/quicctx.c:1436-1448`
* C signature: `void picoquic_insert_cnx_in_list(picoquic_quic_t *, picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:3888-3936`
* Rust item: `insert_cnx_in_list`

### C body
```c
{
    if (quic->cnx_list != NULL) {
        quic->cnx_list->previous_in_table = cnx;
        cnx->next_in_table = quic->cnx_list;
    } else {
        quic->cnx_last = cnx;
        cnx->next_in_table = NULL;
    }
    quic->cnx_list = cnx;
    cnx->previous_in_table = NULL;
    quic->current_number_connections++;
}
```

### Rust body
```rust
    pub fn delete_connection(&mut self, token: ConnectionToken) {
        let Some(cnx) = self.connections.get(token) else {
            return;
        };
        let initial_cid = cnx.initial_connection_id;
        let local_cid_memberships: Vec<_> = cnx
            .local_connection_id_lists
            .iter()
            .flat_map(|list| list.connection_ids.iter().copied())
            .filter_map(|tok| {
                cnx.local_connection_ids
                    .get(tok)
                    .and_then(|l_cid| l_cid.connection_by_id_membership)
            })
            .collect();
        let was_half_open = cnx.is_half_open;

        for membership in local_cid_memberships {
            self.connection_by_id.remove(membership);
        }
        if !initial_cid.is_empty()
            && let Some(ht) = self.connection_by_id.lookup(&initial_cid)
            && self.connection_by_id.get(ht).copied() == Some(token)
        {
            self.connection_by_id.remove(ht);
        }
        while self
            .connections
            .get(token)
            .is_some_and(|cnx| !cnx.paths.is_empty())
        {
            self.clear_path_data(token, 0);
        }
        self.remove_cnx_from_list(token);
        self.remove_cnx_from_wake_list(token);

        // Update accounting.
        if was_half_open {
            self.current_number_half_open = self.current_number_half_open.saturating_sub(1);
        }

        self.connections.remove(token);
    }
```
