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

## Pair `picoquic/quicctx.c:picoquic_create_local_cnxid`
C: `picoquic/quicctx.c:3795-3866 picoquic_create_local_cnxid`
Rust: `rs/fq/src/lib.rs:2226-2329 create_local_cnxid`

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

## Pair `picoquic/quicctx.c:picoquic_get_local_cnxid`
C: `picoquic/quicctx.c:4459-4463 picoquic_get_local_cnxid`
Rust: `rs/fq/src/lib.rs:2953-2971 local_cnxid`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->first_tuple->p_local_cnxid->cnx_id;
}
```

### Rust body
```rust
    ) -> Option<crate::internal::LocalConnectionIdToken> {
        self.find_local_connection_id(unique_path_id, connection_id)
    }
```
