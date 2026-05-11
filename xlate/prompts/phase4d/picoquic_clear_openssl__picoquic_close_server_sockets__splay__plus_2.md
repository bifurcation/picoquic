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

## `picoquic/picoquic_ptls_openssl.c:picoquic_clear_openssl`
* Phase 4C status: `suspect`
* Phase 4C rationale: C conditionally unloads providers/engines/errors and runs cleanup before clearing the init flag; Rust only clears the init flag.
* C source: `picoquic/picoquic_ptls_openssl.c:90-108`
* C signature: `void picoquic_clear_openssl(void)`
* Rust source: `rs/fq/src/sys/openssl.rs:295-297`
* Rust item: `clear_openssl`

### C body
```c
{
    if (openssl_is_init) {
#if !defined(LIBRESSL_VERSION_NUMBER) && OPENSSL_VERSION_NUMBER >= 0x30000000L
        if (openssl_default_provider != NULL) {
            (void)OSSL_PROVIDER_unload(openssl_default_provider);
            openssl_default_provider = NULL;
        }
#else
#if !defined(OPENSSL_NO_ENGINE)
        /* Free allocations from engines ENGINEs */
        ENGINE_cleanup();
#endif
        ERR_free_strings();
#endif
        EVP_cleanup();
        openssl_is_init = 0;
    }
}
```

### Rust body
```rust
pub fn clear_openssl() {
    OPENSSL_IS_INIT.store(false, Ordering::SeqCst);
}
```

## `picoquic/picosocks.c:picoquic_close_server_sockets`
* Phase 4C status: `suspect`
* Phase 4C rationale: C explicitly closes each valid socket before invalidating it; Rust only assigns each slot to None, with no close operation visible in the body.
* C source: `picoquic/picosocks.c:327-335`
* C signature: `void picoquic_close_server_sockets(picoquic_server_sockets_t *)`
* Rust source: `rs/fq/src/socks.rs:251-255`
* Rust item: `close`

### C body
```c
{
    for (int i = 0; i < PICOQUIC_NB_SERVER_SOCKETS; i++) {
        if (sockets->s_socket[i] != INVALID_SOCKET) {
            SOCKET_CLOSE(sockets->s_socket[i]);
            sockets->s_socket[i] = INVALID_SOCKET;
        }
    }
}
```

### Rust body
```rust
    pub fn close(&mut self) {
        for slot in &mut self.sockets {
            *slot = None;
        }
    }
```

## `picoquic/picosplay.c:splay`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body shows the full loop including zig/zigzig/zigzag rotations; Rust snippet only shows parent lookup and root assignment.
* C source: `picoquic/picosplay.c:40-57`
* C signature: `void splay(picosplay_tree_t *, picosplay_node_t *)`
* Rust source: `rs/fq/src/splay.rs:363-371`
* Rust item: `splay`

### C body
```c
static void splay(picosplay_tree_t *tree, picosplay_node_t *x) {
    while(1) {
        picosplay_node_t *p = x->parent;
        if(p == NULL) {
            tree->root = x;
            return;
        }
        picosplay_node_t *g = p->parent;
        if(p->parent == NULL)
            zig(x);
        else
            if((x == p->left && p == g->left) ||
                    (x == p->right && p == g->right))
                zigzig(x, p);
            else
                zigzag(x);
    }
}
```

### Rust body
```rust
            let p = match self.parent_of(idx) {
                None => {
                    self.root = Some(idx);
                    return;
                }
                Some(p) => p,
            };
```

## `picoquic/quicctx.c:picoquic_connection_disconnect`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only changes state if not already disconnected and invokes close callback; Rust only sets disconnected state.
* C source: `picoquic/quicctx.c:5026-5034`
* C signature: `void picoquic_connection_disconnect(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:5808-5811`
* Rust item: `connection_disconnect`

### C body
```c
{
    if (cnx->cnx_state != picoquic_state_disconnected) {
        cnx->cnx_state = picoquic_state_disconnected;
        if (cnx->callback_fn) {
            (void)(cnx->callback_fn)(cnx, 0, NULL, 0, picoquic_callback_close, cnx->callback_ctx, NULL);
        }
    }
}
```

### Rust body
```rust
    pub fn connection_disconnect(&mut self) {
        // C: picoquic_connection_disconnect
        self.connection_state = crate::State::Disconnected;
    }
```

## `picoquic/quicctx.c:picoquic_create_random_cnx_id`
* Phase 4C status: `suspect`
* Phase 4C rationale: C uses id_length directly for random fill and stored id_len, while Rust clamps id_length to CONNECTION_ID_MAX_SIZE before allocation/fill and returns that length.
* C source: `picoquic/quicctx.c:1630-1639`
* C signature: `void picoquic_create_random_cnx_id(picoquic_quic_t *, picoquic_connection_id_t *, uint8_t)`
* Rust source: `rs/fq/src/lib.rs:1300-1307`
* Rust item: `create_random_cnx_id`

### C body
```c
{
    if (id_length > 0) {
        picoquic_crypto_random(quic, cnx_id->id, id_length);
    }
    if (id_length < sizeof(cnx_id->id)) {
        memset(cnx_id->id + id_length, 0, sizeof(cnx_id->id) - id_length);
    }
    cnx_id->id_len = id_length;
}
```

### Rust body
```rust
pub(crate) fn create_random_cnx_id(quic: &mut Quic, id_length: u8) -> ConnectionId {
    let len = (id_length as usize).min(CONNECTION_ID_MAX_SIZE);
    let mut cnx_id = ConnectionId::with_size(len).unwrap_or_default();
    if len > 0 {
        rand_core::RngCore::fill_bytes(&mut *quic.rng, cnx_id.as_bytes_mut());
    }
    cnx_id
}
```
