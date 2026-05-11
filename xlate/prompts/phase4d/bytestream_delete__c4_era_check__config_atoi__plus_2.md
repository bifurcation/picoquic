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

## `picoquic/bytestream.c:bytestream_delete`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C frees and clears stream data, while Rust shown sets app_wake_time and is unrelated.
* C source: `picoquic/bytestream.c:65-71`
* C signature: `void bytestream_delete(bytestream *)`
* Rust source: `rs/fq/src/lib.rs:2416-2423`
* Rust item: `delete`

### C body
```c
{
    if (s->data != NULL) {
        free(s->data);
        s->data = NULL;
    }
}
```

### Rust body
```rust
    pub fn set_app_wake_time(&mut self, app_wake_time: Instant) {
        self.app_wake_time = app_wake_time;
    }
```

## `picoquic/c4.c:c4_era_check`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns a boolean-like check based on connection state and lowest-not-ack; Rust resets era fields and ECN state with no returned check.
* C source: `picoquic/c4.c:459-473`
* C signature: `int c4_era_check(picoquic_path_t *, c4_state_t *)`
* Rust source: `rs/fq/src/c4.rs:299-319`
* Rust item: `era_check`

### C body
```c
{
    if (path_x->cnx->cnx_state < picoquic_state_ready) {
        return 0;
    }
    else {
        return (picoquic_cc_get_lowest_not_ack(path_x) > c4_state->era_sequence);
    }
}
```

### Rust body
```rust
    fn era_reset(&mut self, path_x: &Path, connection: &Connection) {
        self.era_sequence = connection.sequence_number(path_x);
        self.era_max_rtt = 0;
        self.era_min_rtt = u64::MAX;
        self.alpha_1024_previous = self.alpha_1024_current;
        self.update_ecn_alpha(path_x, connection);
    }
```

## `picoquic/config.c:config_atoi`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C parses decimal digits and updates ret on errors; Rust body only checks x against params length and returns an error.
* C source: `picoquic/config.c:202-224`
* C signature: `int config_atoi(const option_param_t *, int, int, int *)`
* Rust source: `rs/fq/src/config.rs:592-595`
* Rust item: `config_atoi`

### C body
```c
{
    int v = 0;

    if (params == NULL || x < 0 || x >= nb_param) {
        *ret = -1;
    }
    else {
        for (size_t i = 0; i < params[x].length; i++) {
            int c = params[x].param[i] - '0';
            if (c < 0 || c > 9) {
                v = -1;
                *ret = -1;
                break;
            }
            else {
                v *= 10;
                v += c;
            }
        }
    }
    return v;
}
```

### Rust body
```rust
    if x >= params.len() {
        return Err(Error::InvalidArgument);
    }
```

## `picoquic/dualq_aqm.c:dualq_has_pending`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns whether either queue has a first packet; Rust body shown is admit_pending and only calls update_it.
* C source: `picoquic/dualq_aqm.c:347-352`
* C signature: `int dualq_has_pending(picoquictest_aqm_t *)`
* Rust source: `rs/fq/src/tests/dualq.rs:246-254`
* Rust item: `has_pending`

### C body
```c
{
    dualq_state_t* dualq = (dualq_state_t*)self;

    return (dualq->lq.queue_first != NULL || dualq->cq.queue_first != NULL);
}
```

### Rust body
```rust
    fn admit_pending(&mut self, link: &mut TestSimLink, current_time: Instant) {
        self.update_it(link, current_time);
    }
```

## `picoquic/ech.c:picoquic_ech_create_config_from_public_key`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C loads a PUBLIC KEY file and creates config from that public key; Rust calls create_config_from_private_key_file using a private_key_file argument.
* C source: `picoquic/ech.c:857-876`
* C signature: `int picoquic_ech_create_config_from_public_key(uint8_t **, size_t *, const char *, const char *)`
* Rust source: `rs/fq/src/lib.rs:4761-4775`
* Rust item: `ech_create_config_from_public_key`

### C body
```c
{
    int ret = 0;
    ptls_iovec_t public_key_asn1 = ptls_iovec_init(NULL, 0);
    size_t pub_key_objects = 0;

    /* Read the public key from a file. */
    ret = ptls_load_pem_objects(public_key_file, "PUBLIC KEY", &public_key_asn1, 1, &pub_key_objects);
    if (ret != 0) {
        DBG_PRINTF("Cannot load pubkey from <%s>, err: %x", public_key_file, ret);
    }
    else
    {
        ret = picoquic_ech_create_config_from_binary(config, config_len, public_key_asn1, public_name);
    }
    if (public_key_asn1.base != NULL) {
        free(public_key_asn1.base);
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<Vec<u8>, Error> {
    crate::ech::ech_create_config_from_private_key_file(private_key_file, public_name)
}
```
