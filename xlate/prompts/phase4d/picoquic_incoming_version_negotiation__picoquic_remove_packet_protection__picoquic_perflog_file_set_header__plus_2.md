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

## `picoquic/packet.c:picoquic_incoming_version_negotiation`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C validates a version negotiation packet and may signal/disconnect; Rust body processes incoming packet segments generally and does not show VN-specific checks.
* C source: `picoquic/packet.c:904-986`
* C signature: `int picoquic_incoming_version_negotiation(picoquic_cnx_t *, uint8_t *, size_t, struct sockaddr *, picoquic_packet_header *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:3289-3338`
* Rust item: `incoming_packet_ex`

### C body
```c
{
    int ret = 0;
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(addr_from);
    UNREFERENCED_PARAMETER(current_time);
#endif

    /* Check the connection state */
    if (cnx->cnx_state != picoquic_state_client_init_sent) {
        /* This is an unexpected packet. Log and drop.*/
        DBG_PRINTF("Unexpected VN packet (%d), state %d, type: %d, epoch: %d, pc: %d, pn: %d\n",
            cnx->client_mode, cnx->cnx_state, ph->ptype, ph->epoch, ph->pc, (int)ph->pn);
    } else if (picoquic_compare_connection_id(&ph->dest_cnx_id, &cnx->path[0]->first_tuple->p_local_cnxid->cnx_id) != 0 || ph->vn != 0) {
        /* Packet destination ID does not match local CID, should be logged and ignored */
        DBG_PRINTF("VN packet (%d), does not pass echo test.\n", cnx->client_mode);
        ret = PICOQUIC_ERROR_DETECTED;
    }
    else if (picoquic_compare_connection_id(&ph->srce_cnx_id, &cnx->initial_cnxid) != 0 || ph->vn != 0) {
        /* Packet destination ID does not match initial DCID, should be logged and ignored */
        DBG_PRINTF("VN packet (%d), does not pass echo test.\n", cnx->client_mode);
        ret = PICOQUIC_ERROR_DETECTED;
    } else {
        /* Add DOS resilience */
        const uint8_t * v_bytes = bytes + ph->offset;
        const uint8_t* bytes_max = bytes + length;
        int nb_vn = 0;
        while (v_bytes < bytes_max) {
            uint32_t vn = 0;
            if ((v_bytes = picoquic_frames_uint32_decode(v_bytes, bytes_max, &vn)) == NULL){
                DBG_PRINTF("VN packet (%d), length %zu, coding error after %d version numbers.\n",
                    cnx->client_mode, length, nb_vn);
                ret = PICOQUIC_ERROR_DETECTED;
                break;
            } else if (vn == cnx->proposed_version || vn == 0) {
                DBG_PRINTF("VN packet (%d), proposed_version[%d] = 0x%08x.\n", cnx->client_mode, nb_vn, vn);
                ret = PICOQUIC_ERROR_DETECTED;
                break;
            }
            else if (picoquic_get_version_index(vn) >= 0){
                /* The VN packet proposes a valid version that is locally supported */
                nb_vn++;
            }
        }
        if (ret == 0) {
            if (nb_vn == 0) {
                DBG_PRINTF("VN packet (%d), does not propose any interesting version.\n", cnx->client_mode);
                ret = PICOQUIC_ERROR_DETECTED;
            }
            else {
                /* Signal VN to the application */
                if (cnx->callback_fn && length > ph->offset) {
                    (void)(cnx->callback_fn)(cnx, 0, bytes + ph->offset, length - ph->offset,
                        picoquic_callback_version_negotiation, cnx->callback_ctx, NULL);
                }
                /* TODO: consider rewriting the version negotiation code */
                DBG_PRINTF("%s", "Disconnect upon receiving version negotiation.\n");
                cnx->remote_error = PICOQUIC_ERROR_VERSION_NEGOTIATION;
                picoquic_connection_disconnect(cnx);
                ret = 0;
            }
        }
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

## `picoquic/packet.c:picoquic_remove_packet_protection`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body performs duplicate detection plus AEAD decryption, key rotation, integrity failure accounting, and returns decoded length; the Rust body shown only sets already_received.
* C source: `picoquic/packet.c:633-768`
* C signature: `size_t picoquic_remove_packet_protection(picoquic_cnx_t *, uint8_t *, uint8_t *, picoquic_packet_header *, uint64_t, int *)`
* Rust source: `rs/fq/src/internal.rs:7257-7271`
* Rust item: `remove_packet_protection`

### C body
```c
{
    size_t decoded;
    int ret = 0;

    /* verify that the packet is new */
    if (already_received != NULL) {
        if (picoquic_is_pn_already_received(cnx, ph->pc, ph->l_cid, ph->pn64) != 0) {
            /* Set error type: already received */
            *already_received = 1;
        }
        else {
            *already_received = 0;
        }
    }

    if (ph->epoch == picoquic_epoch_1rtt) {
        int need_integrity_check = 1;
        picoquic_ack_context_t* ack_ctx = picoquic_ack_ctx_from_cnx_context(cnx, picoquic_packet_context_application, ph->l_cid);

        /* Manage key rotation */
        if (ph->key_phase == cnx->key_phase_dec) {
            /* AEAD Decrypt */
            if (cnx->is_multipath_enabled && ph->ptype == picoquic_packet_1rtt_protected) {
                decoded = picoquic_aead_decrypt_mp(decoded_bytes + ph->offset,
                    bytes + ph->offset,
                    ph->payload_length, 
                    ph->l_cid->path_id, ph->pn64, decoded_bytes, ph->offset,
                    cnx->crypto_context[picoquic_epoch_1rtt].aead_decrypt);
            } else {
                decoded = picoquic_aead_decrypt_generic(decoded_bytes + ph->offset,
                    bytes + ph->offset, ph->payload_length, ph->pn64, decoded_bytes, ph->offset, 
                    cnx->crypto_context[picoquic_epoch_1rtt].aead_decrypt);
            }
            if (decoded <= ph->payload_length && ph->pn64 < ack_ctx->crypto_rotation_sequence) {
                ack_ctx->crypto_rotation_sequence = ph->pn64;
            }
        }
        else if ((ack_ctx->crypto_rotation_sequence == UINT64_MAX && current_time <= cnx->crypto_rotation_time_guard) ||
            ph->pn64 < ack_ctx->crypto_rotation_sequence) {
            /* This packet claims to be encoded with the old key */
            if (current_time > cnx->crypto_rotation_time_guard) {
                /* Too late. Ignore the packet. Could be some kind of attack. */
                decoded = ph->payload_length + 1;
                need_integrity_check = 0;
            }
            else if (cnx->crypto_context_old.aead_decrypt != NULL) {
                if (cnx->is_multipath_enabled) {
                    decoded = picoquic_aead_decrypt_mp(decoded_bytes + ph->offset, bytes + ph->offset, ph->payload_length,
                        ph->l_cid->path_id, ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context_old.aead_decrypt);
                }
                else {
                    decoded = picoquic_aead_decrypt_generic(decoded_bytes + ph->offset, bytes + ph->offset, ph->payload_length,
                        ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context_old.aead_decrypt);
                }
            }
            else {
                /* old context is either not yet available, or already removed */
                decoded = ph->payload_length + 1;
                need_integrity_check = 0;
            }
        }
        else {
            /* TODO: check that this is larger than last received with current key */
            /* These could only be a new key */
            if (cnx->crypto_context_new.aead_decrypt == NULL &&
                cnx->crypto_context_new.aead_encrypt == NULL) {
                /* If the new context was already computed, don't do it again */
                ret = picoquic_compute_new_rotated_keys(cnx);
            }
            /* if decoding succeeds, the rotation should be validated */
            if (ret == 0 && cnx->crypto_context_new.aead_decrypt != NULL) {
                if (cnx->is_multipath_enabled) {
                    decoded = picoquic_aead_decrypt_mp(decoded_bytes + ph->offset, bytes + ph->offset, ph->payload_length,
                        ph->l_cid->path_id, ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context_new.aead_decrypt);

                }
                else {
                    decoded = picoquic_aead_decrypt_generic(decoded_bytes + ph->offset,
                        bytes + ph->offset, ph->payload_length, ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context_new.aead_decrypt);
                }
                if (decoded <= ph->payload_length) {
                    /* Rotation only if the packet was correctly decrypted with the new key */
                    cnx->crypto_rotation_time_guard = current_time + cnx->path[0]->retransmit_timer;
                    if (cnx->is_multipath_enabled) {
                        for (int i=0; i < cnx->nb_paths; i++){
                            cnx->path[i]->ack_ctx.crypto_rotation_sequence = UINT64_MAX;
                        }
                    }
                    ack_ctx->crypto_rotation_sequence = ph->pn64;
                    picoquic_apply_rotated_keys(cnx, 0);
                    cnx->nb_crypto_key_rotations++;

                    if (cnx->crypto_context_new.aead_encrypt != NULL) {
                        /* If that move was not already validated, move to the new encryption keys */
                        picoquic_apply_rotated_keys(cnx, 1);
                    }
                }
            }
            else {
                /* new context could not be computed  */
                decoded = ph->payload_length + 1;
                need_integrity_check = 0;
            }
        }

        if (need_integrity_check && decoded > ph->payload_length) {
            cnx->crypto_failure_count++;
            if (cnx->crypto_failure_count > picoquic_aead_integrity_limit(cnx->crypto_context[picoquic_epoch_1rtt].aead_decrypt)) {
                picoquic_log_app_message(cnx, "AEAD Integrity limit reached after 0x%" PRIx64 " failed decryptions.", cnx->crypto_failure_count);
                (void)picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_AEAD_LIMIT_REACHED, 0);
            }
        }
    }
    else {
        /* TODO: get rid of handshake some time after handshake complete */
        /* For all the other epochs, there is a single crypto context and no key rotation */
        if (cnx->crypto_context[ph->epoch].aead_decrypt != NULL) {
            decoded = picoquic_aead_decrypt_generic(decoded_bytes + ph->offset,
                bytes + ph->offset, ph->payload_length, ph->pn64, decoded_bytes, ph->offset, cnx->crypto_context[ph->epoch].aead_decrypt);
        }
        else {
            decoded = ph->payload_length + 1;
        }
    }

    /* by conventions, values larger than input indicate error */
    return decoded;
}
```

### Rust body
```rust
        if let Some(already_received) = already_received {
            *already_received = self.is_pn_already_received(
                ph.packet_context,
                ph.local_connection_id,
                ph.packet_number_full,
            );
        }
```

## `picoquic/performance_log.c:picoquic_perflog_file_set_header`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is only an else-return fragment and does not open the file or write the CSV header.
* C source: `picoquic/performance_log.c:298-318`
* C signature: `void picoquic_perflog_file_set_header(const char *)`
* Rust source: `rs/fq/src/performance_log.rs:314-322`
* Rust item: `file_set_header`

### C body
```c
{
    FILE* F = picoquic_file_open(perflog_file_name, "w");

    if (F != NULL) {
        fprintf(F, "Log_v, PQ_v, Duration, Sent, Received, Mpbs_S, Mbps_R");
        fprintf(F, ", QUIC_v, ALPN, CNX_ID, T64");
        /* Print the additional values */
        for (size_t i = 0; i < PICOQUIC_PERF_LOG_MAX_ITEMS; i++) {
            char buf[16];
            char const* s = picoquic_perflog_param_name((picoquic_perflog_column_enum)i);
            if (s == NULL) {
                (void)picoquic_sprintf(buf, sizeof(buf), NULL, "v%zu", i);
                s = buf;
            }
            fprintf(F, ", %s", s);
        }
        fprintf(F, "\n");
        fclose(F);
    }
}
```

### Rust body
```rust
    else {
        return;
    };
```

## `picoquic/picoquic_ptls_minicrypto.c:picoquic_init_minicrypto`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body does nothing; Rust body reads and validates a private key file, which is unrelated visible behavior.
* C source: `picoquic/picoquic_ptls_minicrypto.c:48-51`
* C signature: `void picoquic_init_minicrypto(void)`
* Rust source: `rs/fq/src/tls_api.rs:2608-2623`
* Rust item: `init_minicrypto`

### C body
```c
{
    /* Nothing for now */
}
```

### Rust body
```rust
pub fn set_minicrypto_private_key_from_key_file(keypem: &str) -> Result<(), Error> {
    let contents = std::fs::read_to_string(keypem).map_err(|_| Error::NoSuchFile)?;
    if contents.contains("-----BEGIN ") && contents.contains("PRIVATE KEY-----") {
        Ok(())
    } else {
        Err(Error::InvalidFile)
    }
}
```

## `picoquic/picosocks.c:picoquic_socket_set_pkt_info`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets multiple socket options and returns the setsockopt result; Rust immediately returns Ok(()) with no body action.
* C source: `picoquic/picosocks.c:58-91`
* C signature: `int picoquic_socket_set_pkt_info(int, int)`
* Rust source: `rs/fq/src/socks.rs:134-136`
* Rust item: `set_pkt_info`

### C body
```c
{
    int ret;
#ifdef _WINDOWS
    int option_value = 1;
    if (af == AF_INET6) {
        ret = setsockopt(sd, IPPROTO_IPV6, IPV6_PKTINFO, (char*)&option_value, sizeof(int));
    }
    else {
        ret = setsockopt(sd, IPPROTO_IP, IP_PKTINFO, (char*)&option_value, sizeof(int));
    }
#else
    if (af == AF_INET6) {
        int val = 1;
        ret = setsockopt(sd, IPPROTO_IPV6, IPV6_V6ONLY,
            &val, sizeof(val));
        if (ret == 0) {
            val = 1;
            ret = setsockopt(sd, IPPROTO_IPV6, IPV6_RECVPKTINFO, (char*)&val, sizeof(int));
        }
    }
    else {
        int val = 1;
#ifdef IP_PKTINFO
        ret = setsockopt(sd, IPPROTO_IP, IP_PKTINFO, (char*)&val, sizeof(int));
#else
        /* The IP_PKTINFO structure is not defined on BSD */
        ret = setsockopt(sd, IPPROTO_IP, IP_RECVDSTADDR, (char*)&val, sizeof(int));
#endif
    }
#endif

    return ret;
}
```

### Rust body
```rust
    fn set_pkt_info(&mut self) -> Result<(), Error> {
        Ok(())
    }
```
