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

## `picoquic/quicctx.c:picoquic_set_low_memory_mode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns picoquic_set_cipher_suite(quic, 0), while Rust only sets use_low_memory and always returns Ok(()).
* C source: `picoquic/quicctx.c:1175-1180`
* C signature: `int picoquic_set_low_memory_mode(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1543-1546`
* Rust item: `set_low_memory_mode`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->use_low_memory = (low_memory_mode == 0) ? 0 : 1;
    return picoquic_set_cipher_suite(quic, 0);
}
```

### Rust body
```rust
    pub fn set_low_memory_mode(&mut self, low_memory_mode: bool) -> Result<(), Error> {
        self.use_low_memory = low_memory_mode;
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_set_padding_policy`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets padding defaults; Rust body opens or clears a key log file.
* C source: `picoquic/quicctx.c:4681-4686`
* C signature: `void picoquic_set_padding_policy(picoquic_quic_t *, uint32_t, uint32_t)`
* Rust source: `rs/fq/src/lib.rs:1123-1144`
* Rust item: `set_padding_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->padding_minsize_default = padding_min_size;
    quic->padding_multiple_default = padding_multiple;
}
```

### Rust body
```rust
    pub fn set_key_log_file(&mut self, keylog_filename: Option<&str>) {
        // Open or clear the SSL keylog file.  When a path is given, open it
        // for appending; errors are silently ignored (matching the C behaviour
        // of falling back to no logging rather than crashing).
        use std::fs::OpenOptions;
        match keylog_filename {
            Some(path) => {
                if let Ok(f) = OpenOptions::new().create(true).append(true).open(path) {
                    self.f_log = Some(Box::new(f));
                }
            }
            None => {
                self.f_log = None;
            }
        }
    }
```

## `picoquic/quicctx.c:picoquic_stash_remote_cnxid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C uses unique_path_id and delegates add-to-stash with retire_before_next and pstashed; Rust always uses stash path 0 and directly pushes a constructed CID without those visible behaviors.
* C source: `picoquic/quicctx.c:3038-3053`
* C signature: `uint64_t picoquic_stash_remote_cnxid(picoquic_cnx_t *, uint64_t, const uint64_t, const uint64_t, const uint8_t, const uint8_t *, const uint8_t *, picoquic_remote_cnxid_t **)`
* Rust source: `rs/fq/src/internal.rs:16260-16304`
* Rust item: `stash_remote_cnxid`

### C body
```c
{
    uint64_t transport_error = 0;
    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = picoquic_find_or_create_remote_cnxid_stash(cnx, unique_path_id, 1);

    if (remote_cnxid_stash == NULL) {
        transport_error = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
    }
    else {
        transport_error = picoquic_add_remote_cnxid_to_stash(cnx, remote_cnxid_stash, retire_before_next,
            sequence, cid_length, cnxid_bytes, secret_bytes, pstashed);
    }
    return transport_error;
}
```

### Rust body
```rust
) -> crate::Result<()> {
    use crate::RESET_SECRET_SIZE;
    // Find or create a default stash (path_id 0).
    let stash_idx = cnx
        .find_or_create_remote_connection_id_stash(0, true)
        .ok_or(crate::Error::Memory)?;
    let cid = ConnectionId::clone_from_slice(cid_bytes).ok_or(crate::Error::Memory)?;
    let mut secret = [0u8; RESET_SECRET_SIZE];
    let copy_len = reset_secret.len().min(RESET_SECRET_SIZE);
    secret[..copy_len].copy_from_slice(&reset_secret[..copy_len]);
    let r_cid = RemoteConnectionId {
        sequence,
        connection_id: cid,
        reset_secret: secret,
        nb_path_references: 0,
        needs_removal: false,
        retire_sent: false,
        retire_acked: false,
        pkt_ctx: PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: u64::MAX,
            latest_time_acknowledged: crate::Instant::from_ticks(0),
            highest_acknowledged_time: crate::Instant::from_ticks(0),
            pending: std::collections::BTreeMap::new(),
            retransmitted: std::collections::BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        },
    };
    cnx.remote_connection_id_stashes[stash_idx]
        .connection_ids
        .push(r_cid);
    Ok(())
}
```

## `picoquic/quicctx.c:picoquic_update_peer_addr`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C stores into first_tuple->peer_addr and resets observed_addr_acked and nb_observed_repeat; Rust only conditionally updates registered_peer_addr and has no visible counter resets.
* C source: `picoquic/quicctx.c:2842-2849`
* C signature: `void picoquic_update_peer_addr(picoquic_path_t *, const struct sockaddr *)`
* Rust source: `rs/fq/src/internal.rs:14571-14574`
* Rust item: `update_peer_addr`

### C body
```c
{
    /* Set the addresses */
    picoquic_store_addr(&path_x->first_tuple->peer_addr, peer_addr);
    /* Keep track of the update */
    path_x->observed_addr_acked = 0;
    path_x->first_tuple->nb_observed_repeat = 0;
}
```

### Rust body
```rust
        if let Some(addr) = peer_addr {
            self.registered_peer_addr = *addr;
        }
```

## `picoquic/sacks.c:picoquic_sack_item_record_reset`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C loops over two counters, adjusts range_counts, and resets nb_times_sent; Rust body shown only reads old nb_times_sent and returns on missing item.
* C source: `picoquic/sacks.c:487-496`
* C signature: `void picoquic_sack_item_record_reset(picoquic_sack_list_t *, picoquic_sack_item_t *)`
* Rust source: `rs/fq/src/internal.rs:8703-8707`
* Rust item: `item_record_reset`

### C body
```c
{
    for (int r = 0; r < 2; r++) {
        if (sack_item->nb_times_sent[r] < PICOQUIC_MAX_ACK_RANGE_REPEAT) {
            sack_list->rc[r].range_counts[sack_item->nb_times_sent[r]] -= 1;
        }
        sack_item->nb_times_sent[r] = 0;
        sack_list->rc[r].range_counts[sack_item->nb_times_sent[r]] += 1;
    }
}
```

### Rust body
```rust
        let old = match self.sack_items.get(token) {
            Some(item) => item.nb_times_sent,
            None => return,
        };
```
