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

## `picoquic/picosplay.c:picosplay_new_tree`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C allocates and initializes a new tree; Rust body allocates or reuses a node slot, not a tree.
* C source: `picoquic/picosplay.c:90-97`
* C signature: `picosplay_tree_t * picosplay_new_tree(picosplay_comparator, picosplay_create, picosplay_delete_node, picosplay_node_value)`
* Rust source: `rs/fq/src/splay.rs:161-204`
* Rust item: `new_tree`

### C body
```c
picosplay_tree_t* picosplay_new_tree(picosplay_comparator comp, picosplay_create create, picosplay_delete_node delete_node, picosplay_node_value node_value) {
    picosplay_tree_t *new = malloc(sizeof(picosplay_tree_t));
    if (new != NULL) {
        picosplay_init_tree(new, comp, create, delete_node, node_value);
    }
    return new;
}
```

### Rust body
```rust
    fn alloc_slot(&mut self, key: K, value: V) -> Result<u32, Error> {
        if let Some(free_idx) = self.free {
            let next_free = match &self.slots[free_idx as usize].state {
                SlotState::Free { next_free } => *next_free,
                SlotState::Filled { .. } => unreachable!(),
            };
            let r#gen = self.slots[free_idx as usize].generation;
            self.slots[free_idx as usize] = Slot {
                generation: r#gen,
                state: SlotState::Filled {
                    key,
                    value,
                    parent: None,
                    left: None,
                    right: None,
                },
            };
            self.free = next_free;
            Ok(free_idx)
        } else {
            let idx = self.slots.len();
            if idx > u32::MAX as usize {
                return Err(Error::Memory);
            }
            self.slots.push(Slot {
                generation: 0,
                state: SlotState::Filled {
                    key,
                    value,
                    parent: None,
                    left: None,
                    right: None,
                },
            });
            Ok(idx as u32)
        }
    }
```

## `picoquic/quicctx.c:picoquic_abandon_path`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is placeholder-like and always returns Err(Error::Generic), while C performs conditional path demotion and may return success.
* C source: `picoquic/quicctx.c:2583-2626`
* C signature: `int picoquic_abandon_path(picoquic_cnx_t *, uint64_t, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:2548-2556`
* Rust item: `abandon_path`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (!cnx->is_multipath_enabled) {
        ret = -1;
    }
    else if (unique_path_id > cnx->max_path_id_remote ||
        unique_path_id > cnx->max_path_id_local) {
        /* that path has not been created yet */
        ret = -1;
    }
    else {
        /* Check whether there is a path by that ID */
        int path_index = picoquic_get_path_id_from_unique(cnx, unique_path_id);

        if (path_index >= 0) {
            /* Check whether this is the last path */
            if (cnx->nb_paths <= 1) {
                /* That would mean deleting the last path. Don't do that */
                ret = -1;
            }
            else if (!cnx->path[path_index]->path_is_demoted) {
                /* if demotion is not already in progress, demote the path,
                * and if the path can be properly identified, post a path abandon frame.
                */
                picoquic_demote_path(cnx, path_index, current_time, reason);
            }
        }
        else {
            /* The path ID is not in use yet, but local cid have been allocated.
             * We need to send an abandon if not sent yet, mark the local CID
             * as demoted, and delete the stash. The stash has to remain deleted
             * even if we receive new CID for that path.
             */
            ret = picoquic_demote_local_cnxid_list(cnx, unique_path_id,
                reason);
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves path teardown signalling.
        Err(Error::Generic)
    }
```

## `picoquic/quicctx.c:picoquic_create_misc_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C allocates and initializes a misc frame; Rust processes a version upgrade, an unrelated operation.
* C source: `picoquic/quicctx.c:4797-4816`
* C signature: `picoquic_misc_frame_header_t * picoquic_create_misc_frame(const uint8_t *, size_t, int, picoquic_packet_context_enum)`
* Rust source: `rs/fq/src/internal.rs:15656-15701`
* Rust item: `create_misc_frame`

### C body
```c
{
    size_t l_alloc = sizeof(picoquic_misc_frame_header_t) + length;

    if (l_alloc < sizeof(picoquic_misc_frame_header_t)) {
        return NULL;
    }
    else {
        picoquic_misc_frame_header_t* head = (picoquic_misc_frame_header_t*)malloc(l_alloc);
        if (head != NULL) {
            memset(head, 0, sizeof(picoquic_misc_frame_header_t));
            head->length = length;
            head->is_pure_ack = is_pure_ack;
            head->pc = pc;
            memcpy(((uint8_t *)head) + sizeof(picoquic_misc_frame_header_t), bytes, length);
        }
        return head;
    }
}
```

### Rust body
```rust
impl Connection {
    pub fn process_version_upgrade(
        &mut self,
        old_version_index: i32,
        new_version_index: i32,
    ) -> i32 {
        self.rejected_version = if old_version_index >= 0 {
            self.proposed_version
        } else {
            self.rejected_version
        };
        self.version_index = new_version_index;
        let version = match new_version_index {
            0 => Version::V1,
            1 => Version::V2,
            2 => Version::V2Draft,
            3 => Version::PostIesg,
            4 => Version::TwentyFirstInterop,
            5 => Version::TwentiethInterop,
            6 => Version::TwentiethPreInterop,
            7 => Version::NineteenthInterop,
            8 => Version::NineteenthBisInterop,
            9 => Version::EighteenthInterop,
            10 => Version::SeventeenthInterop,
            11 => Version::InternalTest2,
            12 => Version::InternalTest1,
            _ => Version::V1,
        };
        self.proposed_version = version as u32;
        self.desired_version = self.proposed_version;
        self.local_parameters.version_negotiation.current = self.proposed_version;
        0
    }
}
```

## `picoquic/quicctx.c:picoquic_delete_local_cnxid_list`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C deletes all listed local CIDs, unlinks the list, frees it, and decrements the count; Rust only checks bounds and returns.
* C source: `picoquic/quicctx.c:3934-3956`
* C signature: `void picoquic_delete_local_cnxid_list(picoquic_cnx_t *, picoquic_local_cnxid_list_t *)`
* Rust source: `rs/fq/src/internal.rs:13191-13194`
* Rust item: `delete_local_connection_id_list`

### C body
```c
{
    while (local_cnxid_list->local_cnxid_first != NULL) {
        picoquic_delete_local_cnxid_listed(cnx, local_cnxid_list, local_cnxid_list->local_cnxid_first);
    }

    if (local_cnxid_list == cnx->first_local_cnxid_list) {
        cnx->first_local_cnxid_list = local_cnxid_list->next_list;
    }
    else {
        picoquic_local_cnxid_list_t* previous = cnx->first_local_cnxid_list;

        while (previous != NULL) {
            if (previous->next_list == local_cnxid_list) {
                previous->next_list = local_cnxid_list->next_list;
            }
            previous = previous->next_list;
        }
    }

    free(local_cnxid_list);
    cnx->nb_local_cnxid_lists--;
}
```

### Rust body
```rust
        if list_index >= self.local_connection_id_lists.len() {
            return;
        }
```

## `picoquic/quicctx.c:picoquic_enable_path_callbacks`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets are_path_callbacks_enabled; Rust body is set_first_if_index and modifies the first tuple if_index.
* C source: `picoquic/quicctx.c:2513-2517`
* C signature: `void picoquic_enable_path_callbacks(picoquic_cnx_t *, int)`
* Rust source: `rs/fq/src/lib.rs:2656-2669`
* Rust item: `enable_path_callbacks`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->are_path_callbacks_enabled = are_enabled;
}
```

### Rust body
```rust
    pub fn set_first_if_index(&mut self, if_index: u32) -> Result<(), Error> {
        if let Some(path) = self.paths.first_mut()
            && let Some(tuple) = path.tuples.first_mut()
        {
            tuple.if_index = if_index as core::ffi::c_ulong;
            return Ok(());
        }
        Err(Error::InvalidArgument)
    }
```
