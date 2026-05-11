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

## `picoquic/quicctx.c:picoquic_get_quic_ctx`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns NULL for a NULL connection and otherwise returns cnx->quic; Rust always asserts/dereferences quic_ptr with no body-visible null-return path.
* C source: `picoquic/quicctx.c:1419-1422`
* C signature: `picoquic_quic_t * picoquic_get_quic_ctx(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2804-2809`
* Rust item: `quic`

### C body
```c
{
    return (cnx == NULL)?NULL:cnx->quic;
}
```

### Rust body
```rust
    pub unsafe fn quic(&mut self) -> &mut Quic {
        debug_assert!(!self.quic_ptr.is_null(), "quic_ptr not initialised");
        // SAFETY: quic_ptr is set to `self as *mut Quic` in
        // create_cnx_internal and remains valid for the connection's lifetime.
        unsafe { &mut *self.quic_ptr }
    }
```

## `picoquic/quicctx.c:picoquic_purge_misc_frames_after_ready`
* Phase 4C status: `suspect`
* Phase 4C rationale: C purges non-application misc frames unconditionally; Rust only does so when connection_state is Ready.
* C source: `picoquic/quicctx.c:4850-4865`
* C signature: `void picoquic_purge_misc_frames_after_ready(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:13833-13838`
* Rust item: `purge_misc_frames_after_ready`

### C body
```c
{
    picoquic_misc_frame_header_t* misc_frame;
    PICOQUIC_THREAD_CHECK(cnx->quic);
        
    misc_frame = cnx->first_misc_frame;

    while (misc_frame != NULL) {
        picoquic_misc_frame_header_t* next_frame = misc_frame->next_misc_frame;

        if (misc_frame->pc != picoquic_packet_context_application) {
            picoquic_delete_misc_or_dg(&cnx->first_misc_frame, &cnx->last_misc_frame, misc_frame);
        }
        misc_frame = next_frame;
    }
}
```

### Rust body
```rust
    pub fn purge_misc_frames_after_ready(&mut self) {
        if self.connection_state == State::Ready {
            self.misc_frames
                .retain(|frame| frame.packet_context == PacketContext::Application);
        }
    }
```

## `picoquic/quicctx.c:picoquic_remember_issued_ticket`
* Phase 4C status: `suspect`
* Phase 4C rationale: C updates an existing ticket if found and trims the table before inserting; Rust always constructs and inserts a new ticket with no visible lookup, update, or trimming.
* C source: `picoquic/quicctx.c:485-524`
* C signature: `int picoquic_remember_issued_ticket(picoquic_quic_t *, uint64_t, uint64_t, uint64_t, const uint8_t *, uint8_t)`
* Rust source: `rs/fq/src/internal.rs:1843-1865`
* Rust item: `remember_issued_ticket`

### C body
```c
{
    int ret = 0;

    picoquic_issued_ticket_t* ticket = picoquic_retrieve_issued_ticket(quic,
        ticket_id);
    if (ticket != NULL) {
        picoquic_update_issued_ticket(ticket, rtt, cwin, ip_addr, ip_addr_length);
    }
    else {
        while (quic->table_issued_tickets_nb > quic->max_number_connections) {
            picoquic_delete_issued_ticket(quic, quic->table_issued_tickets_last);
        }
        ticket = (picoquic_issued_ticket_t*)malloc(sizeof(picoquic_issued_ticket_t));
        if (ticket != NULL) {
            memset(ticket, 0, sizeof(picoquic_issued_ticket_t));
            ticket->ticket_id = ticket_id;
            picoquic_update_issued_ticket(ticket, rtt, cwin, ip_addr, ip_addr_length);
            ticket->next_ticket = quic->table_issued_tickets_first;
            quic->table_issued_tickets_first = ticket;
            if (ticket->next_ticket == NULL) {
                quic->table_issued_tickets_last = ticket;
            }
            else {
                ticket->next_ticket->previous_ticket = ticket;
            }
            picohash_insert(quic->table_issued_tickets, ticket);
        }
        else {
            ret = PICOQUIC_ERROR_MEMORY;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let ticket = IssuedTicket {
            issued_tickets_membership: None,
            ticket_id,
            creation_time: crate::Instant::from_ticks(0),
            rtt,
            cwin,
            ip_addr,
        };
        let tok = self.issued_tickets.insert(ticket)?;
        let (htok, _) = self.issued_tickets_by_id.insert(ticket_id, tok)?;
        // Store the hash token back into the ticket so we can do O(1) removal later.
        if let Some(entry) = self.issued_tickets.get_mut(tok) {
            entry.issued_tickets_membership = Some(htok);
        }
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_set_congestion_algorithm_ex`
* Phase 4C status: `suspect`
* Phase 4C rationale: C deletes old algorithm state and initializes the new algorithm on each path; Rust only stores the algorithm and option string.
* C source: `picoquic/quicctx.c:5372-5393`
* C signature: `void picoquic_set_congestion_algorithm_ex(picoquic_cnx_t *, const picoquic_congestion_algorithm_t *, const char *)`
* Rust source: `rs/fq/src/lib.rs:4622-4629`
* Rust item: `set_congestion_algorithm_ex`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    if (cnx->congestion_alg != NULL) {
        if (cnx->path != NULL) {
            for (int i = 0; i < cnx->nb_paths; i++) {
                cnx->congestion_alg->alg_delete(cnx->path[i]);
            }
        }
    }

    cnx->congestion_alg = alg;
    cnx->congestion_alg_option_string = alg_option_string;

    if (cnx->congestion_alg != NULL) {
        if (cnx->path != NULL) {
            for (int i = 0; i < cnx->nb_paths; i++) {
                cnx->congestion_alg->alg_init(cnx->path[i], alg_option_string, picoquic_get_quic_time(cnx->quic));
            }
        }
    }
}
```

### Rust body
```rust
    ) {
        self.congestion_alg = Some(alg);
        self.congestion_alg_option_string = alg_option_string.map(|s| s.to_owned());
    }
```

## `picoquic/quicctx.c:picoquic_set_path_challenge`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only resets the challenge when required is false or verified is true and may invoke a suspended callback; Rust unconditionally sets challenge_required, resets the challenge, and clears challenge_verified.
* C source: `picoquic/quicctx.c:2136-2151`
* C signature: `void picoquic_set_path_challenge(picoquic_cnx_t *, int, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:4781-4790`
* Rust item: `set_path_challenge`

### C body
```c
{
    if (!cnx->path[path_id]->first_tuple->challenge_required || cnx->path[path_id]->first_tuple->challenge_verified) {
        /* Reset the path challenge */
        cnx->path[path_id]->first_tuple->challenge_required = 1;
        picoquic_set_tuple_challenge(cnx->path[path_id]->first_tuple, current_time, cnx->quic->use_constant_challenges);
        if (cnx->path[path_id]->first_tuple->challenge_verified && cnx->are_path_callbacks_enabled && cnx->callback_fn != NULL) {
            if (cnx->callback_fn(cnx, cnx->path[path_id]->unique_path_id, NULL, 0, picoquic_callback_path_suspended,
                cnx->callback_ctx, cnx->path[path_id]->app_path_ctx) != 0) {
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, picoquic_frame_type_path_challenge);
            }
        }
        cnx->path[path_id]->first_tuple->challenge_verified = 0;
    }
}
```

### Rust body
```rust
        {
            tuple.challenge_required = true;
            set_tuple_challenge(tuple, current_time, 0);
            tuple.challenge_verified = false;
        }
```
