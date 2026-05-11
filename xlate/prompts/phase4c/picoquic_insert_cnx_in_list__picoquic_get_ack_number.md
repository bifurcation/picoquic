# Phase 4C body-only translation audit

Compare each C/Rust pair using only the function bodies shown
below. Do not infer from dependencies, type definitions, callers,
module context, tests, or external knowledge. This is a cheap
superficial check for obvious inconsistencies.

Return only JSON with this shape:

```json
{"reviews":[{"c_id":"...","status":"ok|suspect|definitely_not_ok","rationale":"body-visible reason"}]}
```

Status meanings:
* `ok`: no obvious body-level concern.
* `suspect`: possible mismatch visible from the bodies.
* `definitely_not_ok`: clear mismatch or placeholder-like code.

## Pair `picoquic/quicctx.c:picoquic_insert_cnx_in_list`
C: `picoquic/quicctx.c:1436-1448 picoquic_insert_cnx_in_list`
Rust: `rs/fq/src/internal.rs:3888-3936 insert_cnx_in_list`

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

## Pair `picoquic/quicctx.c:picoquic_get_ack_number`
C: `picoquic/quicctx.c:1694-1698 picoquic_get_ack_number`
Rust: `rs/fq/src/internal.rs:7843-7858 get_ack_number`

### C body
```c
{
    return (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) ? path_x->pkt_ctx.highest_acknowledged :
        cnx->pkt_ctx[pc].highest_acknowledged;
}
```

### Rust body
```rust
impl Connection {
    pub fn get_last_packet(&self, _path_x: &mut Path, pc: PacketContext) -> Option<PacketToken> {
        // C: picoquic_get_last_packet — last (highest seq) in pending queue
        self.pkt_ctx[pc as usize]
            .pending
            .values()
            .next_back()
            .copied()
    }
}
```
