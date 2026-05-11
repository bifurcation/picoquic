# Phase 4C body-only translation audit

Compare each C/Rust pair using only the function bodies shown
below. Do not infer from dependencies, type definitions, callers,
module context, tests, or external knowledge. This is a cheap
superficial check for obvious inconsistencies.

Return only JSON with this shape:

```json
{"reviews":[{"c_id":"...","status":"ok|suspect|definitely_not_ok","rationale":"body-visible reason"}]}
```

Status meanings:
* `ok`: no obvious body-level concern.
* `suspect`: possible mismatch visible from the bodies.
* `definitely_not_ok`: clear mismatch or placeholder-like code.

## Pair `picoquic/quicctx.c:picoquic_set_low_memory_mode`
C: `picoquic/quicctx.c:1175-1180 picoquic_set_low_memory_mode`
Rust: `rs/fq/src/lib.rs:1543-1546 set_low_memory_mode`

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

## Pair `picoquic/quicctx.c:picoquic_init_transport_parameters`
C: `picoquic/quicctx.c:1395-1414 picoquic_init_transport_parameters`
Rust: `rs/fq/src/internal.rs:3950-3968 init_transport_parameters`

### C body
```c
{
    memset(tp, 0, sizeof(picoquic_tp_t));
    tp->initial_max_stream_data_bidi_local = 0x200000;
    tp->initial_max_stream_data_bidi_remote = 65635;
    tp->initial_max_stream_data_uni = 65535;
    tp->initial_max_data = PICOQUIC_INITIAL_FLOW_CONTROL_MAX;
    tp->initial_max_stream_id_bidir = 512;
    tp->initial_max_stream_id_unidir = 512;
    tp->max_idle_timeout = PICOQUIC_MICROSEC_HANDSHAKE_MAX/1000;
    tp->max_packet_size = PICOQUIC_PRACTICAL_MAX_MTU;
    tp->max_datagram_frame_size = 0;
    tp->ack_delay_exponent = 3;
    tp->active_connection_id_limit = PICOQUIC_NB_PATH_TARGET;
    tp->max_ack_delay = PICOQUIC_ACK_DELAY_MAX;
    tp->enable_loss_bit = 2;
    tp->min_ack_delay = PICOQUIC_ACK_DELAY_MIN;
    tp->enable_time_stamp = 0;
    tp->enable_bdp_frame = 0;
}
```

### Rust body
```rust
pub fn init_transport_parameters(tp: &mut TransportParameters) {
    *tp = TransportParameters::default();
    tp.initial_max_stream_data_bidi_local = 0x20_0000;
    tp.initial_max_stream_data_bidi_remote = 65_635;
    tp.initial_max_stream_data_uni = 65_535;
    tp.initial_max_data = INITIAL_FLOW_CONTROL_MAX;
    tp.initial_max_stream_id_bidir = 512;
    tp.initial_max_stream_id_unidir = 512;
    tp.max_idle_timeout = Duration::from_ticks(MICROSEC_HANDSHAKE_MAX.ticks() / 1000);
    tp.max_packet_size = PRACTICAL_MAX_MTU as u32;
    tp.max_datagram_frame_size = 0;
    tp.ack_delay_exponent = 3;
    tp.active_connection_id_limit = NB_PATH_TARGET as u32;
    tp.max_ack_delay = ACK_DELAY_MAX.ticks() as u32;
    tp.enable_loss_bit = 2;
    tp.min_ack_delay = ACK_DELAY_MIN;
    tp.enable_time_stamp = 0;
    tp.enable_bdp_frame = false;
}
```

## Pair `picoquic/quicctx.c:picoquic_create_random_cnx_id`
C: `picoquic/quicctx.c:1630-1639 picoquic_create_random_cnx_id`
Rust: `rs/fq/src/lib.rs:1300-1307 create_random_cnx_id`

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

## Pair `picoquic/quicctx.c:picoquic_demote_path`
C: `picoquic/quicctx.c:2052-2116 picoquic_demote_path`
Rust: `rs/fq/src/internal.rs:4726-4731 demote_path`

### C body
```c
{
    if (!cnx->path[path_index]->path_is_demoted) {
        uint64_t demote_timer = cnx->path[path_index]->retransmit_timer;

        if (demote_timer < PICOQUIC_INITIAL_MAX_RETRANSMIT_TIMER &&
            !cnx->is_multipath_enabled) {
            demote_timer = PICOQUIC_INITIAL_MAX_RETRANSMIT_TIMER;
        }

        cnx->path[path_index]->path_is_demoted = 1;
        cnx->path[path_index]->demotion_time = current_time + 3* demote_timer;
        cnx->path_demotion_needed = 1;

        /* TODO: add suspended callback */
        if (cnx->is_multipath_enabled) {
             /* Special case for path 0: we want to reorder the paths so the path[0]
             * is always a valid path.
             */
            if (path_index == 0) {
                int alt_path0 = 0;
                for (int i = 1; i < cnx->nb_paths; i++) {
                    if (cnx->path[i]->first_tuple->p_remote_cnxid != NULL) {
                        alt_path0 = i;
                        break;
                    }
                }
                if (alt_path0 != 0) {
                    picoquic_path_t* path_x = cnx->path[0];
                    cnx->path[0] = cnx->path[alt_path0];
                    cnx->path[alt_path0] = path_x;
                    path_index = alt_path0;
                }
            }
            if (path_index == 0) {
                picoquic_log_app_message(cnx, "Cannot demote path index 0, unique_id %" PRIu64", was reason % " PRIu64,
                    cnx->path[path_index]->unique_path_id, reason);
            }
            else if (!cnx->path[path_index]->path_abandon_sent) {
                uint64_t path_id = cnx->path[path_index]->unique_path_id;
                if (picoquic_queue_path_abandon_frame(cnx, path_id, reason) == 0){
                    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = 
                        picoquic_find_or_create_remote_cnxid_stash(cnx, 
                            cnx->path[path_index]->unique_path_id, 0);
                    if (remote_cnxid_stash != NULL && path_index != 0) {
                        cnx->path[path_index]->first_tuple->p_remote_cnxid = NULL;
                        picoquic_delete_remote_cnxid_stash(cnx, remote_cnxid_stash);
                    }
                    else {
                        DBG_PRINTF("Cannot abandon path[%d]", cnx->path[path_index]->unique_path_id);
                    }
                    picoquic_log_app_message(cnx, "Abandon path, unique_id %" PRIu64", reason % " PRIu64,
                        cnx->path[path_index]->unique_path_id, reason);
                    cnx->path[path_index]->path_abandon_sent = 1;
                } else {
                    picoquic_log_app_message(cnx, "Cannot queue abandon path [%" PRIu64 "]",
                        cnx->path[path_index]->unique_path_id);
                }
            }
        }
    }
}
```

### Rust body
```rust
        if let Some(p) = self.paths.get_mut(idx) {
            p.path_is_demoted = true;
            p.demotion_time = current_time;
        }
```

## Pair `picoquic/quicctx.c:picoquic_enable_path_callbacks_default`
C: `picoquic/quicctx.c:2519-2523 picoquic_enable_path_callbacks_default`
Rust: `rs/fq/src/lib.rs:1712-1714 enable_path_callbacks_default`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->are_path_callbacks_enabled = are_enabled;
}
```

### Rust body
```rust
    pub fn enable_path_callbacks_default(&mut self, enabled: bool) {
        self.are_path_callbacks_enabled = enabled;
    }
```
