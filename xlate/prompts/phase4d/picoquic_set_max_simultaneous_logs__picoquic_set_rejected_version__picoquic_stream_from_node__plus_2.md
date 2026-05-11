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

## `picoquic/quicctx.c:picoquic_set_max_simultaneous_logs`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body is a mutating setter with an argument and thread check; Rust body is a getter returning the field with no mutation.
* C source: `picoquic/quicctx.c:1044-1048`
* C signature: `void picoquic_set_max_simultaneous_logs(picoquic_quic_t *, uint32_t)`
* Rust source: `rs/fq/src/lib.rs:1905-1912`
* Rust item: `set_max_simultaneous_logs`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->max_simultaneous_logs = max_simultaneous_logs;
}
```

### Rust body
```rust
    pub fn max_simultaneous_logs(&self) -> u32 {
        self.max_simultaneous_logs
    }
```

## `picoquic/quicctx.c:picoquic_set_rejected_version`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets desired_version and do_version_negotiation; Rust returns a generic error placeholder.
* C source: `picoquic/quicctx.c:5209-5214`
* C signature: `void picoquic_set_rejected_version(picoquic_cnx_t *, uint32_t)`
* Rust source: `rs/fq/src/lib.rs:2431-2444`
* Rust item: `set_rejected_version`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->desired_version = rejected_version;
    cnx->do_version_negotiation = 1;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves path creation and network probing.
        Err(Error::Generic)
    }
```

## `picoquic/quicctx.c:picoquic_stream_from_node`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body implements unrelated close-packet and incoming-client-initial logic rather than converting a stream node to a stream pointer.
* C source: `picoquic/quicctx.c:3459-3466`
* C signature: `picoquic_stream_head_t * picoquic_stream_from_node(picosplay_node_t *)`
* Rust source: `rs/fq/src/lib.rs:7347-7511`
* Rust item: `stream_from_node`

### C body
```c
{
#ifdef TOO_CAUTIOUS
    return(picoquic_stream_head_t *)((node == NULL)?NULL:picoquic_stream_node_value(node));
#else
    return (picoquic_stream_head_t *)node;
#endif
}
```

### Rust body
```rust
impl Quic {
    /// Queue an immediate close packet for `connection` if packet preparation
    /// produces bytes to send.
    ///
    /// C: `picoquic_queue_immediate_close` (picoquic/packet.c:1317-1334).
    fn queue_immediate_close(&mut self, connection: ConnectionToken, current_time: Instant) {
        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };
        let prepared = self
            .connections
            .get_mut(connection)
            .and_then(|cnx| cnx.prepare_packet_ex(current_time, &mut sp.bytes).ok());

        if let Some(prepared) = prepared
            && prepared.send_length > 0
        {
            sp.length = prepared.send_length;
            sp.addr_to = prepared.addr_to;
            sp.addr_local = prepared.addr_from;
            sp.if_index_local = prepared.if_index;
            self.queue_stateless_packet(sp);
        }
    }

    /// Process an incoming client Initial packet on a server connection.
    /// Returns the C-style status code and the still-live connection token.
    ///
    /// C: `picoquic_incoming_client_initial` (picoquic/packet.c:1394-1505).
    pub fn incoming_client_initial(
        &mut self,
        connection: ConnectionToken,
        bytes: &[u8],
        packet_length: usize,
        received_data: &mut crate::internal::StreamDataNode,
        addr_from: Option<&SocketAddr>,
        addr_to: Option<&SocketAddr>,
        if_index_to: u64,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
        new_context_created: bool,
    ) -> (i32, Option<ConnectionToken>) {
        let server_busy = self.server_busy;
        let over_connection_limit =
            self.current_number_connections > self.tentative_max_number_connections;
        let mut ret = 0;
        let mut queue_close = false;
        let mut delete_created_connection = false;

        {
            let Some(cnx) = self.connections.get_mut(connection) else {
                return (InternalError::UnexpectedPacket as i32, None);
            };

            if cnx
                .path_local_connection_id(0)
                .is_some_and(|cid| !cid.is_empty() && cid == ph.dest_connection_id)
            {
                cnx.initial_validated = true;
            }

            if !cnx.initial_validated
                && !cnx.pkt_ctx[PacketContext::Initial as usize]
                    .pending
                    .is_empty()
                && packet_length >= crate::internal::ENFORCED_INITIAL_MTU
            {
                cnx.initial_repeat_needed = true;
            }

            if cnx.connection_state == State::ServerInit && (server_busy || over_connection_limit) {
                cnx.local_error = TransportError::ServerBusy as u64;
                cnx.connection_state = State::HandshakeFailure;
            } else if cnx.connection_state == State::ServerInit
                && cnx.initial_connection_id.len()
                    < crate::internal::ENFORCED_INITIAL_CID_LENGTH as usize
            {
                cnx.local_error = TransportError::ProtocolViolation as u64;
                cnx.connection_state = State::HandshakeFailure;
            } else if cnx.connection_state < State::ServerAlmostReady {
                if let Some(path) = cnx.paths.get_mut(0)
                    && let Some(tuple) = path.tuples.first_mut()
                {
                    if Connection::socket_addr_is_unspecified(&tuple.local_addr)
                        && let Some(addr) = addr_to
                    {
                        tuple.local_addr = *addr;
                    }
                    if Connection::socket_addr_is_unspecified(&tuple.peer_addr)
                        && let Some(addr) = addr_from
                    {
                        tuple.peer_addr = *addr;
                    }
                    tuple.if_index = if_index_to as core::ffi::c_ulong;
                }

                let highest_ack_before =
                    cnx.pkt_ctx[PacketContext::Initial as usize].highest_acknowledged;
                let payload = Connection::packet_payload(bytes, ph);
                ret = cnx.decode_frames_on_path(
                    0,
                    payload,
                    received_data,
                    ph.epoch,
                    addr_from,
                    addr_to,
                    ph.packet_number_full,
                    0,
                    current_time,
                );
                if cnx.pkt_ctx[PacketContext::Initial as usize].highest_acknowledged
                    > highest_ack_before
                    && cnx.random_initial > 1
                {
                    cnx.initial_validated = true;
                }

                if ret == 0 {
                    let (tls_ret, data_consumed) = cnx.process_tls_stream_status(current_time);
                    ret = tls_ret;
                    if data_consumed > 0 {
                        cnx.initial_repeat_needed = false;
                    }
                }
            } else if cnx.connection_state < State::Ready {
                cnx.ignore_incoming_handshake(bytes, ph, current_time);
            } else {
                ret = InternalError::UnexpectedPacket as i32;
            }

            if ret == InternalError::InvalidToken as i32
                && cnx.connection_state == State::HandshakeFailure
            {
                ret = 0;
            }

            if ret == 0 && cnx.connection_state == State::HandshakeFailure && new_context_created {
                queue_close = true;
            }

            if ret != 0 || cnx.connection_state == State::Disconnected {
                delete_created_connection = new_context_created;
            }
        }

        if queue_close {
            self.queue_immediate_close(connection, current_time);
        }

        if delete_created_connection {
            self.delete_connection(connection);
            (InternalError::ConnectionDeleted as i32, None)
        } else {
            (ret, Some(connection))
        }
    }
}
```

## `picoquic/sacks.c:picoquic_sack_find_range_below_number`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C searches for the previous range below pn64; Rust body is item_above taking a token and calling sack_previous_item, with no pn64 search.
* C source: `picoquic/sacks.c:156-168`
* C signature: `picoquic_sack_item_t * picoquic_sack_find_range_below_number(picoquic_sack_list_t *, picoquic_sack_item_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:8103-8115`
* Rust item: `find_range_below_number`

### C body
```c
{
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(previous);
#endif
    picoquic_sack_item_t v = { 0 };
    v.start_of_sack_range = pn64;
    v.end_of_sack_range = pn64;
    return(picoquic_sack_item_value(picosplay_find_previous(&sack_list->ack_tree, &v)));
}
```

### Rust body
```rust
    fn item_above(&mut self, token: SackItemToken) -> Option<SackItemToken> {
        self.sack_previous_item(token)
    }
```

## `picoquic/sacks.c:picoquic_sack_list_init`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C zeros the whole sack list and initializes an ack_tree; shown Rust body only constructs SackRangeCount with zeroed range_counts.
* C source: `picoquic/sacks.c:430-437`
* C signature: `void picoquic_sack_list_init(picoquic_sack_list_t *)`
* Rust source: `rs/fq/src/internal.rs:8605-8614`
* Rust item: `new`

### C body
```c
{
    memset(sack_list, 0, sizeof(picoquic_sack_list_t));
    picosplay_init_tree(&sack_list->ack_tree, picoquic_sack_item_compare,
        picoquic_sack_node_create, picoquic_sack_node_delete, picoquic_sack_node_value);
}
```

### Rust body
```rust
                SackRangeCount {
                    range_counts: [0; MAX_ACK_RANGE_REPEAT],
                },
```
