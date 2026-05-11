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

## `picoquic/quicctx.c:picoquic_get_path_id_from_unique`
* Phase 4C status: `suspect`
* Phase 4C rationale: C initializes ret to -1 and returns -1 if no path matches; Rust body shows returns inside the loop but no visible fallback return.
* C source: `picoquic/quicctx.c:2525-2538`
* C signature: `int picoquic_get_path_id_from_unique(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:4896-4902`
* Rust item: `get_path_id_from_unique`

### C body
```c
{
    int ret = -1;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    for (int i = 0; i < cnx->nb_paths; i++) {
        if (cnx->path[i]->unique_path_id == unique_path_id) {
            ret = i;
            break;
        }
    }

    return ret;
}
```

### Rust body
```rust
        for (i, p) in self.paths.iter().enumerate() {
            if p.unique_path_id == unique_path_id {
                return i as i32;
            }
        }
```

## `picoquic/quicctx.c:picoquic_load_token_file`
* Phase 4C status: `suspect`
* Phase 4C rationale: C treats no-such-file as success and stores token_file_name on success; Rust body only delegates to load_tokens.
* C source: `picoquic/quicctx.c:777-796`
* C signature: `int picoquic_load_token_file(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/internal.rs:3942-3947`
* Rust item: `load_token_file`

### C body
```c
{
    int ret;
    PICOQUIC_THREAD_CHECK(quic);
    ret = picoquic_load_tokens(quic, token_file_name);

    if (ret == PICOQUIC_ERROR_NO_SUCH_FILE) {
        DBG_PRINTF("Ticket file <%s> not created yet.\n", token_file_name);
        ret = 0;
    }
    else if (ret != 0) {
        DBG_PRINTF("Cannot load tickets from <%s>\n", token_file_name);
    }

    if (ret == 0) {
        quic->token_file_name = token_file_name;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        self.load_tokens(token_file_name)
    }
```

## `picoquic/quicctx.c:picoquic_registered_token_create`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns the address of an embedded registered_token_node; Rust returns registered_token_membership, which may be equivalent but the body-visible fields differ.
* C source: `picoquic/quicctx.c:551-554`
* C signature: `picosplay_node_t * picoquic_registered_token_create(void *)`
* Rust source: `rs/fq/src/internal.rs:1223-1225`
* Rust item: `splay_node`

### C body
```c
{
    return &((picoquic_registered_token_t*)value)->registered_token_node;
}
```

### Rust body
```rust
    pub(crate) fn splay_node(&self) -> Option<SplayToken> {
        self.registered_token_membership
```

## `picoquic/quicctx.c:picoquic_reset_ack_context`
* Phase 4C status: `suspect`
* Phase 4C rationale: C clears the ack context, initializes the sack list, and zeros ECN totals; Rust only calls clear_ack_ctx.
* C source: `picoquic/quicctx.c:4893-4902`
* C signature: `void picoquic_reset_ack_context(picoquic_ack_context_t *)`
* Rust source: `rs/fq/src/internal.rs:13871-13873`
* Rust item: `reset_ack_context`

### C body
```c
{
    picoquic_clear_ack_ctx(ack_ctx);

    picoquic_sack_list_init(&ack_ctx->sack_list);

    ack_ctx->ecn_ect0_total_local = 0;
    ack_ctx->ecn_ect1_total_local = 0;
    ack_ctx->ecn_ce_total_local = 0;
}
```

### Rust body
```rust
    pub fn reset_ack_context(&mut self) {
        self.clear_ack_ctx();
    }
```

## `picoquic/quicctx.c:picoquic_set_first_tuple`
* Phase 4C status: `suspect`
* Phase 4C rationale: C unchains the tuple and always makes it first, linking the old first after it; Rust only moves an existing tuple when index is greater than 0 and within bounds.
* C source: `picoquic/quicctx.c:1766-1772`
* C signature: `void picoquic_set_first_tuple(picoquic_path_t *, picoquic_tuple_t *)`
* Rust source: `rs/fq/src/internal.rs:4137-4141`
* Rust item: `set_first_tuple`

### C body
```c
{
    picoquic_tuple_t* old_first = path_x->first_tuple;
    picoquic_unchain_tuple(path_x, tuple);
    path_x->first_tuple = tuple;
    tuple->next_tuple = old_first;
}
```

### Rust body
```rust
        if index > 0 && index < self.tuples.len() {
            let t = self.tuples.remove(index);
            self.tuples.insert(0, t);
        }
```
