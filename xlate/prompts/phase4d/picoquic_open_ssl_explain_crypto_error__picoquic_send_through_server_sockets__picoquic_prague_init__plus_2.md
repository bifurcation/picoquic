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

## `picoquic/picoquic_ptls_openssl.c:picoquic_open_ssl_explain_crypto_error`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body retrieves and returns one OpenSSL error while filling file and line out-parameters; Rust body maps error fields into a CryptoError object, which is similar in purpose but not visibly the same return/out-parameter behavior.
* C source: `picoquic/picoquic_ptls_openssl.c:321-332`
* C signature: `int picoquic_open_ssl_explain_crypto_error(const char **, int *)`
* Rust source: `rs/fq/src/sys/openssl.rs:451-461`
* Rust item: `explain_crypto_error`

### C body
```c
{
#if !defined(LIBRESSL_VERSION_NUMBER) && OPENSSL_VERSION_NUMBER >= 0x30000000L
    const char *func = NULL;
    const char *data = NULL;
    int flags=0;
    return (int)ERR_get_error_all(err_file, err_line, &func, &data, &flags);
#else
    return ERR_get_error_line(err_file, err_line);
#endif
}
```

### Rust body
```rust
        .map(|e| CryptoError {
            code: e.code(),
            reason: e.reason().map(str::to_owned),
            library: e.library().map(str::to_owned),
            file: e.file().to_owned(),
            line: e.line(),
        })
```

## `picoquic/picosocks.c:picoquic_send_through_server_sockets`
* Phase 4C status: `suspect`
* Phase 4C rationale: C computes a socket index and immediately returns the result of sending through that socket; Rust snippet only shows index computation.
* C source: `picoquic/picosocks.c:1273-1283`
* C signature: `int picoquic_send_through_server_sockets(picoquic_server_sockets_t *, struct sockaddr *, struct sockaddr *, int, const char *, int, int *)`
* Rust source: `rs/fq/src/socks.rs:260-267`
* Rust item: `send_through`

### C body
```c
{
    /* Both Linux and Windows use separate sockets for V4 and V6 */
    int socket_index = (addr_dest->sa_family == AF_INET) ? 1 : 0;

    return picoquic_send_through_socket(sockets->s_socket[socket_index], addr_dest, addr_from, from_if, bytes, length, sock_err);
}
```

### Rust body
```rust
        let idx = if addr_dest.is_ipv4() { 1 } else { 0 };
```

## `picoquic/prague.c:picoquic_prague_init`
* Phase 4C status: `suspect`
* Phase 4C rationale: C can leave congestion_alg_state NULL on allocation failure; Rust always creates and stores Some(Box::new(state)).
* C source: `picoquic/prague.c:126-142`
* C signature: `void picoquic_prague_init(picoquic_path_t *, const char *, uint64_t)`
* Rust source: `rs/fq/src/prague.rs:198-206`
* Rust item: `picoquic_prague_init`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    picoquic_prague_state_t* pr_state = (picoquic_prague_state_t*)malloc(sizeof(picoquic_prague_state_t));
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(option_string);
#endif

    if (pr_state != NULL) {
        memset(pr_state, 0, sizeof(picoquic_prague_state_t));
        path_x->congestion_alg_state = (void*)pr_state;
        picoquic_prague_init_reno(pr_state, path_x);
    }
    else {
        path_x->congestion_alg_state = NULL;
    }
}
```

### Rust body
```rust
) {
    let mut state = PragueState::default();
    prague_init_reno(&mut state, path_x);
    path_x.congestion_alg_state = Some(Box::new(state));
}
```

## `picoquic/quicctx.c:picoquic_connection_error`
* Phase 4C status: `suspect`
* Phase 4C rationale: C delegates to connection_error_ex with NULL reason; Rust body implements error state updates and accepts an optional reason.
* C source: `picoquic/quicctx.c:5021-5024`
* C signature: `int picoquic_connection_error(picoquic_cnx_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:5722-5743`
* Rust item: `connection_error`

### C body
```c
{
    return picoquic_connection_error_ex(cnx, local_error, frame_type, NULL);
}
```

### Rust body
```rust
    ) -> i32 {
        // C: picoquic_connection_error_ex
        self.local_error = local_error;
        self.offending_frame_type = frame_type;
        self.local_error_reason = local_reason.map(|s| s.to_owned());
        // Move to disconnecting state if not already past that.
        if (self.connection_state as u32) < crate::State::Disconnecting as u32 {
            self.connection_state = crate::State::Disconnecting;
        }
        local_error as i32
    }
```

## `picoquic/quicctx.c:picoquic_create_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: Most flow-control and output eligibility logic matches, but C calls remove_output_stream and delete_stream_if_closed for non-output streams while Rust simply does nothing in that branch.
* C source: `picoquic/quicctx.c:3638-3696`
* C signature: `picoquic_stream_head_t * picoquic_create_stream(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:9366-9485`
* Rust item: `create_stream`

### C body
```c
{
    picoquic_stream_head_t* stream = (picoquic_stream_head_t*)malloc(sizeof(picoquic_stream_head_t));
    if (stream != NULL) {
        memset(stream, 0, sizeof(picoquic_stream_head_t));
        picoquic_sack_list_init(&stream->sack_list);
    }

    if (stream != NULL){
        int is_output_stream = 0;
        stream->stream_id = stream_id;
        stream->cnx = cnx;

        if (IS_LOCAL_STREAM_ID(stream_id, cnx->client_mode)) {
            if (IS_BIDIR_STREAM_ID(stream_id)) {
                stream->maxdata_local = cnx->local_parameters.initial_max_stream_data_bidi_local;
                stream->maxdata_remote = cnx->remote_parameters.initial_max_stream_data_bidi_remote;
                is_output_stream = stream->stream_id <= cnx->max_stream_id_bidir_remote;

            }
            else {
                stream->maxdata_local = 0;
                stream->maxdata_remote = cnx->remote_parameters.initial_max_stream_data_uni;
                is_output_stream = stream->stream_id <= cnx->max_stream_id_unidir_remote;
            }
        }
        else {
            if (IS_BIDIR_STREAM_ID(stream_id)) {
                stream->maxdata_local = cnx->local_parameters.initial_max_stream_data_bidi_remote;
                stream->maxdata_remote = cnx->remote_parameters.initial_max_stream_data_bidi_local;
                is_output_stream = 1;
            }
            else {
                stream->maxdata_local = cnx->local_parameters.initial_max_stream_data_uni;
                stream->maxdata_remote = 0;
                is_output_stream = 0;
            }
        }

        stream->stream_priority = cnx->quic->default_stream_priority;

        picosplay_init_tree(&stream->stream_data_tree, picoquic_stream_data_node_compare, picoquic_stream_data_node_create, picoquic_stream_data_node_delete, picoquic_stream_data_node_value);

        picosplay_insert(&cnx->stream_tree, stream);
        if (is_output_stream) {
            picoquic_insert_output_stream(cnx, stream);
        }
        else {
            picoquic_remove_output_stream(cnx, stream);
            picoquic_delete_stream_if_closed(cnx, stream);
        }

        if (stream_id >= cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]) {
            cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)] = NEXT_STREAM_ID_FOR_TYPE(stream_id);
        }
    }

    return stream;
}
```

### Rust body
```rust
    pub fn create_stream(&mut self, stream_id: u64) -> Result<StreamToken, crate::Error> {
        use crate::stream::{Role, StreamId};

        let sid = StreamId(stream_id);
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };

        // Determine flow-control limits and output eligibility.
        let (maxdata_local, maxdata_remote, is_output_stream) = if sid.is_local(local_role) {
            if sid.is_bidir() {
                (
                    self.local_parameters.initial_max_stream_data_bidi_local,
                    self.remote_parameters.initial_max_stream_data_bidi_remote,
                    stream_id <= self.max_stream_id_bidir_remote,
                )
            } else {
                (
                    0u64,
                    self.remote_parameters.initial_max_stream_data_uni,
                    stream_id <= self.max_stream_id_unidir_remote,
                )
            }
        } else if sid.is_bidir() {
            (
                self.local_parameters.initial_max_stream_data_bidi_remote,
                self.remote_parameters.initial_max_stream_data_bidi_local,
                true,
            )
        } else {
            (
                self.local_parameters.initial_max_stream_data_uni,
                0u64,
                false,
            )
        };

        let stream = StreamHead {
            stream_tree_membership: None,
            stream_id,
            affinity_path: None,
            consumed_offset: 0,
            fin_offset: 0,
            reset_offset: 0,
            maxdata_local,
            maxdata_local_acked: 0,
            maxdata_remote,
            local_error: 0,
            remote_error: 0,
            local_stop_error: 0,
            remote_stop_error: 0,
            last_time_data_sent: crate::Instant::from_ticks(0),
            stream_data_tree: crate::splay::SplayTree::default(),
            stream_data_nodes: crate::arena::Arena::new(),
            sent_offset: 0,
            reliable_size: 0,
            send_queue: std::collections::VecDeque::new(),
            app_stream_ctx: None,
            direct_receive_fn: None,
            direct_receive_ctx: None,
            sack_list: SackList::new(),
            stream_priority: crate::DEFAULT_STREAM_PRIORITY,
            is_active: false,
            fin_requested: false,
            fin_sent: false,
            fin_received: false,
            fin_signalled: false,
            reset_requested: false,
            reset_sent: false,
            reset_acked: false,
            reset_received: false,
            reset_signalled: false,
            stop_sending_requested: false,
            stop_sending_sent: false,
            stop_sending_received: false,
            stop_sending_signalled: false,
            max_stream_updated: false,
            stream_data_blocked_sent: false,
            is_output_stream: false,
            is_closed: false,
            is_discarded: false,
            use_app_flow_control: false,
            is_not_coalesced: false,
        };

        let tok = self
            .streams
            .insert(stream)
            .map_err(|_| crate::Error::Memory)?;

        // Insert into the splay tree keyed by stream_id.
        let (splay_tok, _) = self
            .stream_tree
            .insert(stream_id, tok)
            .map_err(|_| crate::Error::Memory)?;
        if let Some(s) = self.streams.get_mut(tok) {
            s.stream_tree_membership = Some(splay_tok);
            s.is_output_stream = false; // handled below
        }

        // Advance next_stream_id if needed.
        // C: STREAM_TYPE_FROM_ID = (stream_id & 3)
        let type_idx = (stream_id & 3) as usize;
        if stream_id >= self.next_stream_id[type_idx] {
            self.next_stream_id[type_idx] = stream_id + 4;
        }

        // Insert into output queue if applicable.
        if is_output_stream {
            // Borrow the stream and mark it, then enqueue its token.
            if let Some(s) = self.streams.get_mut(tok) {
                s.is_output_stream = true;
            }
            self.enqueue_output_stream_token(tok);
        }

        Ok(tok)
    }
```
