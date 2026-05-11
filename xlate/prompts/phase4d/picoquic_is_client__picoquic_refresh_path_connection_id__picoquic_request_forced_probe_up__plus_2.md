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

## `picoquic/quicctx.c:picoquic_is_client`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns client_mode; Rust body shown returns local_error.
* C source: `picoquic/quicctx.c:5494-5498`
* C signature: `int picoquic_is_client(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4467-4474`
* Rust item: `is_client`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->client_mode;
}
```

### Rust body
```rust
    pub fn local_error(&self) -> u64 {
        self.local_error
    }
```

## `picoquic/quicctx.c:picoquic_refresh_path_connection_id`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust ignores the unique_path_id parameter and returns a generic error with a placeholder comment, while C looks up the path and conditionally renews its connection ID.
* C source: `picoquic/quicctx.c:2769-2777`
* C signature: `int picoquic_refresh_path_connection_id(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:2559-2562`
* Rust item: `refresh_path_connection_id`

### C body
```c
{
    int ret = -1;
    int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        ret = picoquic_renew_path_connection_id(cnx, cnx->path[path_id]);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn refresh_path_connection_id(&mut self, _unique_path_id: u64) -> Result<(), Error> {
        // Complex: involves CID generation and registration.
        Err(Error::Generic)
    }
```

## `picoquic/quicctx.c:picoquic_request_forced_probe_up`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets is_forced_probe_up_required from the request flag; Rust updates pacing thresholds and pacing update request instead.
* C source: `picoquic/quicctx.c:5412-5416`
* C signature: `void picoquic_request_forced_probe_up(picoquic_cnx_t *, unsigned int)`
* Rust source: `rs/fq/src/lib.rs:4644-4658`
* Rust item: `request_forced_probe_up`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->is_forced_probe_up_required = request_forced_probe_up;
}
```

### Rust body
```rust
    ) {
        self.pacing_decrease_threshold = decrease_threshold;
        self.pacing_increase_threshold = increase_threshold;
        self.is_pacing_update_requested = true;
    }
```

## `picoquic/quicctx.c:picoquic_set_cookie_mode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C maps bit 1 to force_check_token, bit 2 to provide_token, then derives check_token from force_check_token or a half-open threshold; Rust maps different bits directly to check_token, force_check_token, and provide_token.
* C source: `picoquic/quicctx.c:1187-1204`
* C signature: `void picoquic_set_cookie_mode(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1549-1553`
* Rust item: `set_cookie_mode`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    if (cookie_mode&1) {
        quic->force_check_token = 1;
    } else {
        quic->force_check_token = 0;
    }

    if (cookie_mode & 2) {
        quic->provide_token = 1;
    }
    else {
        quic->provide_token = 0;
    }

    quic->check_token = (quic->force_check_token || quic->max_half_open_before_retry <= quic->current_number_half_open);
}
```

### Rust body
```rust
    pub fn set_cookie_mode(&mut self, cookie_mode: i32) {
        self.check_token = (cookie_mode & 1) != 0;
        self.force_check_token = (cookie_mode & 2) != 0;
        self.provide_token = (cookie_mode & 4) != 0;
    }
```

## `picoquic/quicctx.c:picoquic_set_default_lossbit_policy`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets default lossbit policy and default_tp enable_loss_bit; Rust body shown sets default_multipath_option instead.
* C source: `picoquic/quicctx.c:935-940`
* C signature: `void picoquic_set_default_lossbit_policy(picoquic_quic_t *, picoquic_lossbit_version_enum)`
* Rust source: `rs/fq/src/lib.rs:1690-1697`
* Rust item: `set_default_lossbit_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_lossbit_policy = default_lossbit_policy;
    quic->default_tp.enable_loss_bit = (int)default_lossbit_policy;
}
```

### Rust body
```rust
    pub fn set_default_multipath_option(&mut self, multipath_option: i32) {
        self.default_multipath_option = multipath_option as u32;
    }
```
