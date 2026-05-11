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

## `picoquic/quicctx.c:picoquic_find_path_by_unique_id`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns -1 when no path matches; Rust body only returns on match and shows no not-found return.
* C source: `picoquic/quicctx.c:2203-2215`
* C signature: `int picoquic_find_path_by_unique_id(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:4833-4838`
* Rust item: `find_path_by_unique_id`

### C body
```c
{
    int path_index = -1;
    
    for (int i = 0; i < cnx->nb_paths; i++) {
        if (cnx->path[i]->unique_path_id == unique_path_id) {
            path_index = i;
            break;
        }
    }

    return path_index;
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

## `picoquic/quicctx.c:picoquic_get_congestion_algorithm`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C looks up and returns an algorithm by name with a reno-to-newreno fallback; Rust body shown contains default-algorithm setter methods, not the lookup body.
* C source: `picoquic/quicctx.c:5311-5327`
* C signature: `const picoquic_congestion_algorithm_t * picoquic_get_congestion_algorithm(const char *)`
* Rust source: `rs/fq/src/lib.rs:4568-4610`
* Rust item: `get_congestion_algorithm`

### C body
```c
{
    picoquic_congestion_algorithm_t const* alg = NULL;

    if (alg_name != NULL && picoquic_congestion_control_algorithms != NULL) {
        for (size_t i = 0; i < picoquic_nb_congestion_control_algorithms; i++) {
            if (strcmp(alg_name, picoquic_congestion_control_algorithms[i]->congestion_algorithm_id) == 0) {
                alg = picoquic_congestion_control_algorithms[i];
                break;
            }
        }
        if (alg == NULL && strcmp(alg_name, "reno") == 0) {
            alg = picoquic_get_congestion_algorithm("newreno");
        }
    }
    return alg;
}
```

### Rust body
```rust
impl Quic {
    /// Set the default congestion-control algorithm applied to new
    /// connections on this context.
    pub fn set_default_congestion_algorithm(&mut self, algo: &'static CongestionAlgorithm) {
        self.default_congestion_alg = Some(algo);
        self.default_congestion_alg_option_string = None;
    }

    /// Same as [`Self::set_default_congestion_algorithm`] but
    /// passes through a per-algorithm options string.
    pub fn set_default_congestion_algorithm_ex(
        &mut self,
        alg: &'static CongestionAlgorithm,
        alg_option_string: Option<&str>,
    ) {
        self.default_congestion_alg = Some(alg);
        self.default_congestion_alg_option_string = alg_option_string.map(|s| s.to_owned());
    }

    /// Convenience: select the default algorithm by name (looking up
    /// in the registry).  Returns `Err` when no registered algorithm
    /// matches `alg_name`.
    pub fn set_default_congestion_algorithm_by_name(
        &mut self,
        alg_name: &str,
    ) -> Result<(), Error> {
        match get_congestion_algorithm(alg_name) {
            Some(alg) => {
                self.default_congestion_alg = Some(alg);
                self.default_congestion_alg_option_string = None;
                Ok(())
            }
            None => Err(Error::InvalidArgument),
        }
    }
}
```

## `picoquic/quicctx.c:picoquic_get_default_tp`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns default_tp; Rust body shown returns max_number_connections.
* C source: `picoquic/quicctx.c:813-817`
* C signature: `const picoquic_tp_t * picoquic_get_default_tp(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:1579-1587`
* Rust item: `default_tp`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return &quic->default_tp;
}
```

### Rust body
```rust
    pub fn max_nb_connections(&self) -> u32 {
        self.max_number_connections
    }
```

## `picoquic/quicctx.c:picoquic_get_max_half_open_retry_threshold`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns max_half_open_before_retry; Rust body is a setter for is_port_blocking_disabled and returns nothing.
* C source: `picoquic/quicctx.c:1212-1216`
* C signature: `uint32_t picoquic_get_max_half_open_retry_threshold(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:1184-1192`
* Rust item: `max_half_open_retry_threshold`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->max_half_open_before_retry;
}
```

### Rust body
```rust
    pub fn set_port_blocking_disabled(&mut self, is_port_blocking_disabled: bool) {
        self.is_port_blocking_disabled = is_port_blocking_disabled;
    }
```

## `picoquic/quicctx.c:picoquic_get_sequence_number`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns send_sequence from path or packet context; Rust body is get_ack_number and returns first ACK value.
* C source: `picoquic/quicctx.c:1686-1692`
* C signature: `uint64_t picoquic_get_sequence_number(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_context_enum)`
* Rust source: `rs/fq/src/internal.rs:7836-7846`
* Rust item: `get_sequence_number`

### C body
```c
{
    return (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) ? path_x->pkt_ctx.send_sequence:
        cnx->pkt_ctx[pc].send_sequence;
}
```

### Rust body
```rust
    pub fn get_ack_number(&self, _path_x: &mut Path, pc: PacketContext) -> u64 {
        // C: picoquic_get_ack_number
        self.ack_ctx[pc as usize].sack_list.first()
    }
```
