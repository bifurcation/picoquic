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

## `picoquic/frames.c:picoquic_record_ack_packet_data`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C records per-path ACK data and accumulates data_acked; Rust body shown only early-returns when send_path is absent.
* C source: `picoquic/frames.c:3132-3168`
* C signature: `void picoquic_record_ack_packet_data(picoquic_packet_data_t *, picoquic_packet_t *)`
* Rust source: `rs/fq/src/internal.rs:8721-8724`
* Rust item: `record_ack_packet_data`

### C body
```c
{
    picoquic_path_t* old_path = acked_packet->send_path;

    if (old_path != NULL) {
        /* Find the path index in the packet data structure */
        int path_i = 0;
        while (path_i < packet_data->nb_path_ack &&
            packet_data->path_ack[path_i].acked_path != old_path) {
            path_i++;
        }
        if (path_i == packet_data->nb_path_ack) {
            if (path_i > PICOQUIC_NB_PATH_TARGET) {
                /* Too many ACKs in this packet -- do not update path status. */
                return;
            }
            packet_data->nb_path_ack++;
            packet_data->path_ack[path_i].acked_path = old_path;
        }

        if (!packet_data->path_ack[path_i].is_set) {
            packet_data->path_ack[path_i].largest_sent_time = acked_packet->send_time;
            packet_data->path_ack[path_i].delivered_prior = acked_packet->delivered_prior;
            packet_data->path_ack[path_i].delivered_time_prior = acked_packet->delivered_time_prior;
            packet_data->path_ack[path_i].delivered_sent_prior = acked_packet->delivered_sent_prior;
            packet_data->path_ack[path_i].lost_prior = acked_packet->lost_prior;
            packet_data->path_ack[path_i].inflight_prior = acked_packet->inflight_prior;
            packet_data->path_ack[path_i].rs_is_path_limited = acked_packet->delivered_app_limited;
            packet_data->path_ack[path_i].rs_is_cwnd_limited = acked_packet->sent_cwin_limited;
            packet_data->path_ack[path_i].is_set = 1;
        }
        packet_data->path_ack[path_i].data_acked += acked_packet->length;
    }
}
```

### Rust body
```rust
        let Some(send_path) = acked_packet.send_path else {
            return;
        };
```

## `picoquic/logger.c:textlog_cc_dump`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C only calls textlog_congestion_state for the supplied path; Rust iterates all paths, invokes memlog/text/bin/qlog callbacks, checks update flags, and mutates path state.
* C source: `picoquic/logger.c:2377-2380`
* C signature: `void textlog_cc_dump(picoquic_cnx_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/logger.rs:846-883`
* Rust item: `cc_dump`

### C body
```c
{
    textlog_congestion_state(cnx->quic->F_log, cnx, path_x, current_time);
}
```

### Rust body
```rust
    fn cc_dump(&mut self, current_time: Instant) {
        let mut paths = core::mem::take(&mut self.paths);

        if let Some(mut memlog) = self.memlog_call_back.take() {
            if let Some(path0) = paths.first_mut() {
                memlog.callback(self, path0, 0, current_time);
            }
            self.memlog_call_back = Some(memlog);
        }

        if self.is_still_logging() {
            for path_x in &mut paths {
                if !path_x.is_cc_data_updated {
                    continue;
                }

                if let Some(text) = logger_ref(&self.text_log_fns) {
                    text.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.f_binlog.is_some()
                    && let Some(bin) = logger_ref(&self.bin_log_fns)
                {
                    bin.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.qlog_ctx.is_some()
                    && let Some(qlog) = logger_ref(&self.qlog_fns)
                {
                    qlog.borrow_mut().cc_dump(self, path_x, current_time);
                }

                path_x.is_cc_data_updated = false;
            }
        }

        self.paths = paths;
    }
```

## `picoquic/logwriter.c:binlog_get_path_id`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body returns a path id based on multipath/path state, while the Rust body shown is an append_varint helper and does not compute or return a path id.
* C source: `picoquic/logwriter.c:689-698`
* C signature: `uint64_t binlog_get_path_id(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/binlog.rs:215-228`
* Rust item: `get_path_id`

### C body
```c
{
    uint64_t path_id = 0;

    if (cnx->is_multipath_enabled && path_x != NULL) {
        path_id = path_x->unique_path_id;
    }

    return path_id;
}
```

### Rust body
```rust
fn append_varint(out: &mut Vec<u8>, value: u64) {
    let mut buf = [0u8; 8];
    let n = varint_encode(&mut buf, value);
    out.extend_from_slice(&buf[..n]);
}
```

## `picoquic/logwriter.c:picoquic_log_varint_skip`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns NULL for NULL/out-of-range input or skips the varint length; the Rust body shown only handles empty input and lacks the non-empty skip.
* C source: `picoquic/logwriter.c:42-45`
* C signature: `const uint8_t * picoquic_log_varint_skip(const uint8_t *, const uint8_t *)`
* Rust source: `rs/fq/src/internal.rs:6438-6441`
* Rust item: `frames_varint_skip`

### C body
```c
{
    return bytes == NULL ? NULL : (bytes < bytes_max ? picoquic_log_fixed_skip(bytes, bytes_max, VARINT_LEN_T(bytes, size_t)) : NULL);
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## `picoquic/packet.c:picoquic_incoming_retry`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C validates retry state/header, verifies retry integrity/ODCID, copies token, resets connection, and returns retry error; Rust is a generic incoming-packet loop with none of that visible.
* C source: `picoquic/packet.c:1521-1613`
* C signature: `int picoquic_incoming_retry(picoquic_cnx_t *, uint8_t *, picoquic_packet_header *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:3289-3338`
* Rust item: `incoming_packet_ex`

### C body
```c
{
    int ret = 0;
    size_t token_length = 0;
    uint8_t * token = NULL;

    if ((cnx->cnx_state != picoquic_state_client_init_sent && cnx->cnx_state != picoquic_state_client_init_resent) ||
        cnx->original_cnxid.id_len != 0) {
        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
    } else {
        /* Verify that the header is a proper echo of what was sent */
        if (ph->vn != picoquic_supported_versions[cnx->version_index].version) {
            /* Packet that do not match the "echo" checks should be logged and ignored */
            ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
        } else if (ph->pn64 != 0) {
            /* after draft-12, PN is required to be 0 */
            ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
        }
    }

    if (ret == 0) {
        /* Parse the retry frame */
        void * integrity_aead = picoquic_find_retry_protection_context(cnx->quic, cnx->version_index, 0);
        size_t byte_index = ph->offset;
        size_t data_length = ph->offset + ph->payload_length;

        /* Assume that is aead context is null, this is the old format and the 
         * integrity shall be verifed by checking the ODCID */
        if (integrity_aead == NULL) {
            uint8_t odcil = bytes[byte_index++];

            if (odcil != cnx->initial_cnxid.id_len || (size_t)odcil + 1u > ph->payload_length ||
                memcmp(cnx->initial_cnxid.id, &bytes[byte_index], odcil) != 0) {
                /* malformed ODCIL, or does not match initial cid; ignore */
                ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
                picoquic_log_app_message(cnx, "Retry packet rejected: odcid check failed");
            }
            else {
                byte_index += odcil;
            }
        }
        else {
            ret = picoquic_verify_retry_protection(integrity_aead, bytes, &data_length, byte_index, &cnx->initial_cnxid);

            if (ret != 0) {
                picoquic_log_app_message(cnx, "Retry packet rejected: integrity check failed, ret=0x%x", ret);
            }
        }

        if (ret == 0) {
            token_length = data_length - byte_index;

            if (token_length > 0) {
                token = malloc(token_length);
                if (token == NULL) {
                    ret = PICOQUIC_ERROR_MEMORY;
                }
                else {
                    memcpy(token, &bytes[byte_index], token_length);
                }
            }
        }
    }

    if (ret == 0) {
        /* Close the log, because it is keyed by initial_cnxid */
        picoquic_log_close_connection(cnx);
        /* if this is the first reset, reset the original cid */
        if (cnx->original_cnxid.id_len == 0) {
            cnx->original_cnxid = cnx->initial_cnxid;
        }
        /* reset the initial CNX_ID to the version sent by the server */
        cnx->initial_cnxid = ph->srce_cnx_id;

        /* keep a copy of the retry token */
        if (cnx->retry_token != NULL) {
            free(cnx->retry_token);
        }
        cnx->retry_token = token;
        cnx->retry_token_length = (uint16_t)token_length;

        picoquic_reset_cnx(cnx, current_time);

        /* Mark the packet as not required for ack */
        ret = PICOQUIC_ERROR_RETRY;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<Option<&mut Connection>, Error> {
        let packet_length = bytes.len();
        let mut consumed_index = 0usize;
        let mut previous_dest_id = ConnectionId::default();
        let mut first_cnx = None;

        while consumed_index < packet_length {
            let mut consumed = 0usize;
            let ret = self.incoming_segment(
                &mut bytes[consumed_index..],
                packet_length - consumed_index,
                packet_length,
                &mut consumed,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
                current_time,
                current_time,
                &mut previous_dest_id,
                &mut first_cnx,
            );

            if ret == 0 {
                consumed_index = consumed_index.saturating_add(consumed);
                if consumed == 0 {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(token) = first_cnx
            && let Some(cnx) = self.connections.get_mut(token)
            && packet_length > cnx.max_mtu_received
        {
            cnx.max_mtu_received = packet_length;
        }

        Ok(first_cnx.and_then(|token| self.connections.get_mut(token)))
    }
```
