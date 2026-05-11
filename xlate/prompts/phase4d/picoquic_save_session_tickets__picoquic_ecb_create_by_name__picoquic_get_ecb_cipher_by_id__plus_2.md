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

## `picoquic/ticket_store.c:picoquic_save_session_tickets`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust ignores the filename and contains a placeholder comment while C calls ticket serialization/storage logic and returns its result.
* C source: `picoquic/ticket_store.c:511-515`
* C signature: `int picoquic_save_session_tickets(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/lib.rs:1819-1822`
* Rust item: `save_session_tickets`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return picoquic_save_tickets(quic->p_first_ticket, picoquic_get_tls_time(quic), ticket_store_filename);
}
```

### Rust body
```rust
    pub fn save_session_tickets(&mut self, _ticket_store_filename: &str) -> Result<(), Error> {
        // Complex: involves file I/O and ticket serialization.
        Ok(())
    }
```

## `picoquic/tls_api.c:picoquic_ecb_create_by_name`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust returns an AES128 context for any recognized name and the shown else is incomplete; C creates the selected cipher or null.
* C source: `picoquic/tls_api.c:478-488`
* C signature: `void * picoquic_ecb_create_by_name(int, const void *, const char *)`
* Rust source: `rs/fq/src/tls_api.rs:2237-2244`
* Rust item: `ecb_create_by_name`

### C body
```c
{
    void* created = NULL;
    ptls_cipher_algorithm_t* ecb_cipher = picoquic_get_ecb_cipher_by_id(alg_name);

    if (ecb_cipher != NULL) {
        created = (void*)ptls_cipher_new(ecb_cipher, is_enc, ecb_key);
    }
    
    return created;
}
```

### Rust body
```rust
    if get_ecb_cipher_by_name(alg_name) {
        Some(Aes128EcbContext::new(is_enc, ecb_key))
    } else {
```

## `picoquic/tls_api.c:picoquic_get_ecb_cipher_by_id`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C searches cipher suites and returns a matching ECB cipher pointer; Rust only returns whether the name equals a single literal string.
* C source: `picoquic/tls_api.c:451-469`
* C signature: `ptls_cipher_algorithm_t * picoquic_get_ecb_cipher_by_id(const char *)`
* Rust source: `rs/fq/src/tls_api.rs:2229-2231`
* Rust item: `get_ecb_cipher_by_name`

### C body
```c
{
    ptls_cipher_algorithm_t* ecb_cipher = NULL;

    for (int j = 0; j < 2 && ecb_cipher == NULL; j++) {
        for (int i = 0; i < PICOQUIC_CIPHER_SUITES_NB_MAX && ecb_cipher == NULL; i++) {
            ptls_cipher_suite_t* suite = (j == 0) ?
                picoquic_cipher_suites[i].high_memory_suite :
                picoquic_cipher_suites[i].low_memory_suite;

            if (suite != NULL && suite->aead != NULL && suite->aead->ecb_cipher != NULL &&
                strcmp(suite->aead->ecb_cipher->name, ecb_cipher_name) == 0){
                ecb_cipher = suite->aead->ecb_cipher;
                break;
            }
        }
    }
    return ecb_cipher;
}
```

### Rust body
```rust
fn get_ecb_cipher_by_name(ecb_cipher_name: &str) -> bool {
    ecb_cipher_name == "AES128-ECB"
}
```

## `picoquic/tls_api.c:picoquic_pn_iv_size`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the iv_size from the provided cipher context; Rust ignores the argument and always returns 16.
* C source: `picoquic/tls_api.c:2369-2372`
* C signature: `size_t picoquic_pn_iv_size(void *)`
* Rust source: `rs/fq/src/tls_api.rs:2144-2146`
* Rust item: `pn_iv_size`

### C body
```c
{
    return ((ptls_cipher_context_t *)pn_enc)->algo->iv_size;
}
```

### Rust body
```rust
pub fn pn_iv_size(_pn_enc: &dyn crate::tls::HeaderKey) -> usize {
    16
}
```

## `picoquic/tls_api.c:picoquic_tls_client_authentication_activated`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns require_client_authentication; Rust sets use_exporter and returns nothing.
* C source: `picoquic/tls_api.c:2819-2821`
* C signature: `int picoquic_tls_client_authentication_activated(picoquic_quic_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1704-1712`
* Rust item: `tls_client_authentication_activated`

### C body
```c
int picoquic_tls_client_authentication_activated(picoquic_quic_t* quic) {
    return ((ptls_context_t*)quic->tls_master_ctx)->require_client_authentication;
}
```

### Rust body
```rust
    pub fn tls_set_use_exporter(&mut self, use_exporter: bool) {
        self.use_exporter = use_exporter;
    }
```
