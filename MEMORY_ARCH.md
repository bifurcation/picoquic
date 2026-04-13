# MEMORY_ARCH.md

This document analyzes the memory ownership patterns in picoquic to guide the C-to-Rust translation.

## Ownership Hierarchy

```
picoquic_quic_t (QUIC Context)
├── owns: cnx_list (linked list of connections)
├── owns: hash tables (cnx_by_id, cnx_by_net, cnx_by_icid, cnx_by_secret)
├── owns: packet pool (p_first_packet)
├── owns: data node pool (p_first_data_node)
├── owns: stored tickets (p_first_ticket)
├── owns: stored tokens (p_first_token)
├── owns: issued tickets table
├── owns: TLS master context (tls_master_ctx)
├── owns: AEAD contexts for ticket encryption
└── owns: pending stateless packets

picoquic_cnx_t (Connection)
├── borrows: quic (back-pointer to context)
├── owns: path[] (array of path pointers)
├── owns: stream_tree (splay tree of streams)
├── owns: tls_stream[4] (crypto streams per epoch)
├── owns: crypto_context[4] (encryption contexts per epoch)
├── owns: first_local_cnxid_list (local CID stash)
├── owns: first_remote_cnxid_stash (remote CID stash)
├── owns: first_misc_frame / last_misc_frame (frame queue)
├── owns: first_datagram / last_datagram (datagram queue)
├── owns: first_sooner / last_sooner (early packet copies)
├── owns: tls_ctx (connection TLS context)
├── owns: retry_token
├── owns: remote_error_reason (heap string)
├── borrows: sni, alpn (string references)
└── owns: binlog_file_name, qlog_ctx

picoquic_path_t (Path)
├── borrows: cnx (back-pointer to connection)
├── owns: first_tuple (linked list of tuples)
├── owns: congestion_alg_state (CC algorithm state)
├── owns: ack_ctx (if multipath)
├── owns: pkt_ctx (packet context)
└── contains: pacing (inline struct)

picoquic_stream_head_t (Stream)
├── borrows: cnx (back-pointer to connection)
├── borrows: affinity_path (optional path reference)
├── owns: stream_data_tree (received data segments)
├── owns: send_queue (outgoing data segments)
└── borrows: app_stream_ctx (application context)

picoquic_tuple_t (Address Tuple)
├── borrows: p_remote_cnxid (reference to stashed CID)
├── borrows: p_local_cnxid (reference to local CID)
└── contains: addresses (inline sockaddr_storage)
```

## Allocation Patterns

### Pool Allocation
Packets and data nodes are allocated from pools managed by the QUIC context:
- `picoquic_packet_t` from `quic->p_first_packet`
- `picoquic_stream_data_node_t` from `quic->p_first_data_node`

**Rust translation**: Use typed arenas or object pools with index-based references.

### Linked Lists
Most collections use intrusive linked lists with `next`/`previous` pointers:
- Connections: `next_in_table`, `previous_in_table`
- Streams: `next_output_stream`, `previous_output_stream`
- Tickets/Tokens: `next_ticket`, `next_token`
- Misc frames: `next_misc_frame`, `previous_misc_frame`
- Local CIDs: `next` pointer
- Remote CIDs: `next` pointer
- Tuples: `next_tuple`

**Rust translation**: Use `Vec<T>` or typed indices into arena storage. Consider `slotmap` crate.

### Splay Trees
Ordered data structures use picosplay:
- `cnx_wake_tree`: connections ordered by wake time
- `stream_tree`: streams ordered by ID
- `stream_data_tree`: received data segments
- `ack_tree`: SACK ranges
- `token_reuse_tree`: token reuse detection
- `queue_data_repeat_tree`: retransmit queue

**Rust translation**: Use `BTreeMap` or `BTreeSet` with appropriate key types.

### Hash Tables
Fast lookups use picohash with embedded `picohash_item`:
- `table_cnx_by_id`: lookup by connection ID
- `table_cnx_by_net`: lookup by network address
- `table_cnx_by_icid`: lookup by initial CID
- `table_cnx_by_secret`: lookup by reset secret
- `table_issued_tickets`: ticket lookup

**Rust translation**: Use `HashMap` with appropriate key types.

## Lifetime Relationships

### Long-lived References (entire connection lifetime)
- `cnx->quic`: connection to context
- `stream->cnx`: stream to connection
- `path->cnx`: path to connection

### Medium-lived References (path lifetime)
- `tuple->p_remote_cnxid`: tuple to stashed CID
- `tuple->p_local_cnxid`: tuple to local CID
- `stream->affinity_path`: optional path affinity

### Short-lived References (function scope)
- Packet processing callbacks
- Frame encoding/decoding

## Callback Patterns

All callbacks follow the pattern: function pointer + void* context.

```c
typedef int (*picoquic_stream_data_cb_fn)(picoquic_cnx_t* cnx,
    uint64_t stream_id, uint8_t* bytes, size_t length,
    picoquic_call_back_event_t fin_or_event, void* callback_ctx,
    void* stream_ctx);
```

**Rust translation**: Use trait objects or generic type parameters:
```rust
trait StreamCallback {
    fn on_stream_data(&mut self, cnx: &mut Connection, stream_id: u64, 
                      data: &[u8], event: CallbackEvent) -> Result<()>;
}
```

## Thread Safety

picoquic is single-threaded by design:
- No internal synchronization primitives
- All operations assume single-threaded access
- `PICOQUIC_WITH_THREAD_CHECK` debug flag validates this

**Rust translation**: Types should be `!Sync` and `!Send` by default. Multi-threaded usage requires explicit wrapper with mutex.

## Memory Management Functions

Key allocation/deallocation pairs:
- `picoquic_create` / `picoquic_free`
- `picoquic_create_cnx` / `picoquic_delete_cnx`
- `picoquic_create_path` / `picoquic_delete_path`
- `picoquic_create_stream` / `picoquic_delete_stream`
- `picoquic_create_tuple` / `picoquic_delete_tuple`

## Unsafe Boundaries

Areas requiring careful unsafe handling:
1. **TLS integration**: void* pointers to picotls contexts
2. **Crypto contexts**: void* AEAD/PN encryption handles
3. **Pool allocation**: raw pointer manipulation
4. **Hash table items**: embedded hash_item structs
5. **Splay nodes**: embedded node structs
6. **Network addresses**: sockaddr_storage unions
7. **Callback contexts**: void* user data

## Translation Strategy

### Phase 1: Safe Core
Translate data structures using safe Rust idioms:
- Replace linked lists with `Vec<T>` or indices
- Replace splay trees with `BTreeMap`
- Replace hash tables with `HashMap`
- Use `Rc<RefCell<T>>` for shared mutable state initially

### Phase 2: Performance Optimization
After correctness is established:
- Replace `Rc<RefCell<T>>` with arena indices where beneficial
- Consider unsafe for hot paths if profiling indicates need
- Optimize memory layout for cache efficiency

### Phase 3: Unsafe Encapsulation
Create safe abstractions over remaining unsafe code:
- TLS integration wrapper
- Crypto context wrapper
- Network address handling
