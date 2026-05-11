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

## `picoquic/picoquic_ptls_openssl.c:set_openssl_sign_certificate_from_key`
* Phase 4C status: `suspect`
* Phase 4C rationale: C allocates/inititializes a signer, assigns ctx->sign_certificate, frees pkey, and frees signer on error; Rust only unwraps/moves the key into SignCertificate with no visible ctx assignment or init call.
* C source: `picoquic/picoquic_ptls_openssl.c:110-136`
* C signature: `int set_openssl_sign_certificate_from_key(EVP_PKEY *, ptls_context_t *)`
* Rust source: `rs/fq/src/sys/openssl.rs:532-542`
* Rust item: `set_sign_certificate_from_key`

### C body
```c
{
    int ret = 0;
    ptls_openssl_sign_certificate_t* signer;

    signer = (ptls_openssl_sign_certificate_t*)malloc(sizeof(ptls_openssl_sign_certificate_t));

    if (signer == NULL || pkey == NULL) {
        ret = -1;
    }
    else {
        ret = ptls_openssl_init_sign_certificate(signer, pkey);
        ctx->sign_certificate = &signer->super;
    }

    if (pkey != NULL) {
        EVP_PKEY_free(pkey);
    }

    if (ret != 0 && signer != NULL) {
        free(signer);
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<SignCertificate, crate::Error> {
    let key = key.ok_or(crate::Error::Generic)?;
    // ptls_openssl_init_sign_certificate stores the key pointer in the
    // signer struct and sets `ctx->sign_certificate`.  In Rust we simply
    // move the key into the struct — no additional initialisation is
    // needed because the signing algorithm is chosen dynamically by
    // OpenSSL when a signature is requested.
    Ok(SignCertificate { key })
}
```

## `picoquic/picosplay.c:picosplay_init_tree`
* Phase 4C status: `suspect`
* Phase 4C rationale: C initializes comparator/callback fields, root, and size; Rust only delegates to Self::new(), whose body is not shown.
* C source: `picoquic/picosplay.c:80-88`
* C signature: `void picosplay_init_tree(picosplay_tree_t *, picosplay_comparator, picosplay_create, picosplay_delete_node, picosplay_node_value)`
* Rust source: `rs/fq/src/splay.rs:149-163`
* Rust item: `new`

### C body
```c
void picosplay_init_tree(picosplay_tree_t* tree, picosplay_comparator comp, picosplay_create create, picosplay_delete_node delete_node, picosplay_node_value node_value) {
    tree->comp = comp;
    tree->create = create;
    tree->delete_node = delete_node;
    tree->node_value = node_value;
    tree->root = NULL;
    tree->size = 0;
}
```

### Rust body
```rust
    pub const fn new_tree() -> Self {
        Self::new()
    }
```

## `picoquic/quicctx.c:picoquic_clear_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: C frees queued send data, removes the output stream if needed, empties received data and sack list; Rust resets the entire StreamHead to defaults, clearing many additional fields beyond the C body.
* C source: `picoquic/quicctx.c:3427-3445`
* C signature: `void picoquic_clear_stream(picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/internal.rs:12968-13017`
* Rust item: `clear_stream`

### C body
```c
{
    picoquic_stream_queue_node_t* ready = stream->send_queue;
    picoquic_stream_queue_node_t* next;

    while ((next = ready) != NULL) {
        ready = next->next_stream_data;
        if (next->bytes != NULL) {
            free(next->bytes);
        }
        free(next);
    }
    stream->send_queue = NULL;
    if (stream->is_output_stream) {
        picoquic_remove_output_stream(stream->cnx, stream);
    }
    picosplay_empty_tree(&stream->stream_data_tree);
    picoquic_sack_list_free(&stream->sack_list);
}
```

### Rust body
```rust
    pub fn clear_stream(&mut self) {
        let stream_id = self.stream_id;
        *self = StreamHead {
            stream_tree_membership: None,
            stream_id,
            affinity_path: None,
            consumed_offset: 0,
            fin_offset: 0,
            reset_offset: 0,
            maxdata_local: 0,
            maxdata_local_acked: 0,
            maxdata_remote: 0,
            local_error: 0,
            remote_error: 0,
            local_stop_error: 0,
            remote_stop_error: 0,
            last_time_data_sent: crate::Instant::from_ticks(0),
            stream_data_tree: crate::splay::SplayTree::default(),
            stream_data_nodes: crate::arena::Arena::new(),
            sent_offset: 0,
            reliable_size: 0,
            send_queue: std::collections::VecDeque::new(),
            app_stream_ctx: None,
            direct_receive_fn: None,
            direct_receive_ctx: None,
            sack_list: SackList::new(),
            stream_priority: 0,
            is_active: false,
            fin_requested: false,
            fin_sent: false,
            fin_received: false,
            fin_signalled: false,
            reset_requested: false,
            reset_sent: false,
            reset_acked: false,
            reset_received: false,
            reset_signalled: false,
            stop_sending_requested: false,
            stop_sending_sent: false,
            stop_sending_received: false,
            stop_sending_signalled: false,
            max_stream_updated: false,
            stream_data_blocked_sent: false,
            is_output_stream: false,
            is_closed: false,
            is_discarded: false,
            use_app_flow_control: false,
            is_not_coalesced: false,
        };
    }
```

## `picoquic/quicctx.c:picoquic_create_local_cnxid`
* Phase 4C status: `suspect`
* Phase 4C rationale: Bodies mostly align on CID selection, sequence assignment, list append, and registration, but Rust stores the new local CID before connection_by_id.insert; if that insert fails, Rust returns an error after leaving the local CID in the list, while C's register call has no visible failure path and still returns the created entry.
* C source: `picoquic/quicctx.c:3795-3866`
* C signature: `picoquic_local_cnxid_t * picoquic_create_local_cnxid(picoquic_cnx_t *, uint64_t, picoquic_connection_id_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:2226-2329`
* Rust item: `create_local_cnxid`

### C body
```c
{
    picoquic_local_cnxid_list_t* local_cnxid_list = picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 1);
    picoquic_local_cnxid_t* l_cid = NULL;
    int is_unique = 0;

    if (local_cnxid_list != NULL) {
        l_cid = (picoquic_local_cnxid_t*)malloc(sizeof(picoquic_local_cnxid_t));

        if (l_cid != NULL) {
            memset(l_cid, 0, sizeof(picoquic_local_cnxid_t));
            l_cid->create_time = current_time;

            if (cnx->quic->local_cnxid_length == 0) {
                is_unique = 1;
            }
            else {
                for (int i = 0; i < 32; i++) {
                    if (i == 0 && suggested_value != NULL) {
                        l_cid->cnx_id = *suggested_value;
                    }
                    else {
                        picoquic_create_local_cnx_id(cnx->quic, &l_cid->cnx_id, cnx->initial_cnxid);
                    }

                    if (picoquic_cnx_by_id(cnx->quic, l_cid->cnx_id, NULL) == NULL) {
                        is_unique = 1;
                        break;
                    }
                }
            }

            if (is_unique) {
                picoquic_local_cnxid_t* previous = NULL;
                picoquic_local_cnxid_t* next = local_cnxid_list->local_cnxid_first;

                while (next != NULL) {
                    previous = next;
                    next = next->next;
                }

                if (previous == NULL) {
                    local_cnxid_list->local_cnxid_first = l_cid;
                }
                else {
                    previous->next = l_cid;
                }

                l_cid->sequence = local_cnxid_list->local_cnxid_sequence_next++;
                l_cid->path_id = unique_path_id;
                local_cnxid_list->nb_local_cnxid++;

                if (cnx->quic->local_cnxid_length > 0) {
                    picoquic_register_cnx_id(cnx->quic, cnx, l_cid);
                }
                if (l_cid->sequence == 0) {
                    local_cnxid_list->local_cnxid_oldest_created = current_time;
                    if (local_cnxid_list->unique_path_id > cnx->max_path_id_in_cnxid_lists) {
                        cnx->max_path_id_in_cnxid_lists = local_cnxid_list->unique_path_id;
                    }
                }
            }
            else {
                free(l_cid);
                l_cid = NULL;
            }
        }
    }

    return l_cid;
}
```

### Rust body
```rust
    ) -> Result<crate::internal::LocalConnectionIdToken, Error> {
        let initial_connection_id = self
            .connections
            .get(connection)
            .map(|cnx| cnx.initial_connection_id)
            .ok_or(Error::InvalidArgument)?;

        let connection_id = if self.local_connection_id_length == 0 {
            ConnectionId::default()
        } else {
            let mut selected = None;
            for attempt in 0..32 {
                let candidate = if attempt == 0 {
                    if let Some(suggested) = suggested_value {
                        suggested
                    } else {
                        let mut generated = ConnectionId::default();
                        self.create_local_cnx_id(&mut generated, initial_connection_id);
                        generated
                    }
                } else {
                    let mut generated = ConnectionId::default();
                    self.create_local_cnx_id(&mut generated, initial_connection_id);
                    generated
                };
                if self.connection_by_id.lookup(&candidate).is_none() {
                    selected = Some(candidate);
                    break;
                }
            }
            selected.ok_or(Error::Generic)?
        };

        let token = {
            let cnx = self
                .connections
                .get_mut(connection)
                .ok_or(Error::InvalidArgument)?;
            let list_idx = match cnx
                .local_connection_id_lists
                .iter()
                .position(|list| list.unique_path_id == unique_path_id)
            {
                Some(idx) => idx,
                None => {
                    cnx.local_connection_id_lists
                        .push(crate::internal::LocalConnectionIdList {
                            unique_path_id,
                            local_connection_id_sequence_next: 0,
                            local_connection_id_retire_before: 0,
                            local_connection_id_oldest_created: current_time.ticks(),
                            nb_local_connection_id_expired: 0,
                            is_demoted: false,
                            demotion_time: Instant::from_ticks(u64::MAX),
                            connection_ids: Vec::new(),
                        });
                    cnx.local_connection_id_lists.len() - 1
                }
            };
            let sequence =
                cnx.local_connection_id_lists[list_idx].local_connection_id_sequence_next;
            let local_cid = crate::internal::LocalConnectionId {
                connection_by_id_membership: None,
                path_id: unique_path_id,
                sequence,
                create_time: current_time,
                connection_id,
                is_acked: false,
            };
            let token = cnx
                .local_connection_ids
                .insert(local_cid)
                .map_err(|_| Error::Memory)?;
            cnx.local_connection_id_lists[list_idx].local_connection_id_sequence_next += 1;
            cnx.local_connection_id_lists[list_idx]
                .connection_ids
                .push(token);
            if sequence == 0 {
                cnx.local_connection_id_lists[list_idx].local_connection_id_oldest_created =
                    current_time.ticks();
                if unique_path_id > cnx.max_path_id_in_connection_id_lists {
                    cnx.max_path_id_in_connection_id_lists = unique_path_id;
                }
            }
            token
        };

        if self.local_connection_id_length > 0 {
            let (membership, _) = self.connection_by_id.insert(connection_id, connection)?;
            if let Some(cnx) = self.connections.get_mut(connection)
                && let Some(local_cid) = cnx.local_connection_ids.get_mut(token)
            {
                local_cid.connection_by_id_membership = Some(membership);
            }
        }

        Ok(token)
    }
```

## `picoquic/quicctx.c:picoquic_get_next_cnx`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns cnx->next_in_table directly; Rust computes slot_idx()+1 and asks a connection collection for next_after_idx.
* C source: `picoquic/quicctx.c:1430-1434`
* C signature: `picoquic_cnx_t * picoquic_get_next_cnx(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3187-3190`
* Rust item: `next_cnx`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->next_in_table;
}
```

### Rust body
```rust
    pub fn next_cnx(&mut self, current_token: ConnectionToken) -> Option<&mut Connection> {
        let next_idx = current_token.slot_idx() + 1;
        self.connections.next_after_idx(next_idx)
    }
```
