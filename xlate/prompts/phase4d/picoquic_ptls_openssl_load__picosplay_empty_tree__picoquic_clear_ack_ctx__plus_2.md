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

## `picoquic/picoquic_ptls_openssl.c:picoquic_ptls_openssl_load`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust mirrors unload handling, initialization, and version logging, but returns a provider registration instead of visibly performing the C body's cipher suite, key exchange, verifier, error, random, and keyex registration calls.
* C source: `picoquic/picoquic_ptls_openssl.c:406-454`
* C signature: `void picoquic_ptls_openssl_load(int)`
* Rust source: `rs/fq/src/sys/openssl.rs:772-789`
* Rust item: `picoquic_ptls_openssl_load`

### C body
```c
{
    if (unload) {
        if (unload == 1) {
            picoquic_clear_openssl();
        }
    }
    else {
        picoquic_init_openssl();
#ifdef OPENSSL_VERSION_NUMBER
        DBG_PRINTF("Open ssl include version: %x", OPENSSL_VERSION_NUMBER);
#endif
#ifdef LIBRESSL_VERSION_NUMBER
        DBG_PRINTF("LIBRE SSL include version: %x", LIBRESSL_VERSION_NUMBER);
#endif
        DBG_PRINTF("OpenSSL_version_num(): %x", OpenSSL_version_num());

        picoquic_register_ciphersuite(&ptls_openssl_aes128gcmsha256, 1);
        picoquic_register_ciphersuite(&ptls_openssl_aes256gcmsha384, 1);
        picoquic_register_key_exchange_algorithm(&ptls_openssl_secp256r1);
        picoquic_register_hpke_cipher_suite(&picoquic_openssl_hpke_aes128gcmsha256);
        picoquic_register_hpke_cipher_suite(&picoquic_openssl_hpke_aes128gcmsha512);
        picoquic_register_hpke_cipher_suite(&picoquic_openssl_hpke_aes256gcmsha384);
        picoquic_register_hpke_kem(&picoquic_openssl_hpke_kem_p256sha256);
        picoquic_register_hpke_kem(&picoquic_openssl_hpke_kem_p384sha384);

#ifdef PTLS_OPENSSL_HAVE_CHACHA20_POLY1305
        picoquic_register_ciphersuite(&ptls_openssl_chacha20poly1305sha256, 1);
        picoquic_register_key_exchange_algorithm(&ptls_openssl_x25519);
        picoquic_register_hpke_cipher_suite(&picoquic_openssl_hpke_chacha20poly1305sha256);
        picoquic_register_hpke_kem(&picoquic_openssl_hpke_kem_x25519sha256);
#endif
        picoquic_register_tls_key_provider_fn(
            set_openssl_private_key_from_key_file,
            picoquic_openssl_dispose_sign_certificate,
            picoquic_openssl_get_certs_from_file,
            picoquic_openssl_get_public_key_from_key_file);
        picoquic_register_verify_certificate_fn(picoquic_openssl_get_certificate_verifier,
            picoquic_openssl_dispose_certificate_verifier,
            picoquic_openssl_set_tls_root_certificates);
        picoquic_register_explain_crypto_error_fn(picoquic_open_ssl_explain_crypto_error,
            picoquic_openssl_clear_crypto_errors);
        picoquic_register_crypto_random_provider_fn(ptls_openssl_random_bytes);
        picoquic_register_keyex_from_key_file_fn(openssl_keyex_from_key_file, openssl_keyex_dispose);

    }
}
```

### Rust body
```rust
pub fn picoquic_ptls_openssl_load(unload: i32) -> Option<&'static OpenSslProviderRegistration> {
    if unload != 0 {
        if unload == 1 {
            clear_openssl();
        }
        None
    } else {
        init_openssl();
        if let Some(version) = source_version_number() {
            log::debug!("Open ssl include version: {:x}", version);
        }
        if let Some(version) = libressl_source_version_number() {
            log::debug!("LIBRE SSL include version: {:x}", version);
        }
        log::debug!("OpenSSL_version_num(): {:x}", openssl::version::number());
        Some(&OPENSSL_PROVIDER_REGISTRATION)
    }
}
```

## `picoquic/picosplay.c:picosplay_empty_tree`
* Phase 4C status: `suspect`
* Phase 4C rationale: C repeatedly deletes root nodes through the delete helper; Rust clears storage fields directly, bypassing per-node delete behavior visible in C.
* C source: `picoquic/picosplay.c:224-231`
* C signature: `void picosplay_empty_tree(picosplay_tree_t *)`
* Rust source: `rs/fq/src/splay.rs:660-665`
* Rust item: `clear`

### C body
```c
{
    if (tree != NULL) {
        while (tree->root != NULL) {
            picosplay_delete_hint(tree, tree->root);
        }
    }
}
```

### Rust body
```rust
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free = None;
        self.root = None;
        self.len = 0;
    }
```

## `picoquic/quicctx.c:picoquic_clear_ack_ctx`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only frees the sack_list; Rust frees sack_list and resets many additional ACK/ECN fields.
* C source: `picoquic/quicctx.c:4886-4890`
* C signature: `void picoquic_clear_ack_ctx(picoquic_ack_context_t *)`
* Rust source: `rs/fq/src/internal.rs:13849-13866`
* Rust item: `clear_ack_ctx`

### C body
```c
{
    picoquic_sack_list_free(&ack_ctx->sack_list);

}
```

### Rust body
```rust
    pub fn clear_ack_ctx(&mut self) {
        self.sack_list.free();
        self.time_stamp_largest_received = crate::Instant::from_ticks(0);
        self.crypto_rotation_sequence = 0;
        self.ecn_ect0_total_local = 0;
        self.ecn_ect1_total_local = 0;
        self.ecn_ce_total_local = 0;
        self.sending_ecn_ack = false;
        for act in &mut self.act {
            act.highest_ack_sent = 0;
            act.highest_ack_sent_time = crate::Instant::from_ticks(0);
            act.time_oldest_unack_packet_received = crate::Instant::from_ticks(0);
            act.ack_needed = false;
            act.ack_after_fin = false;
            act.out_of_order_received = false;
            act.is_immediate_ack_required = false;
        }
    }
```

## `picoquic/quicctx.c:picoquic_create_cnx_internal`
* Phase 4C status: `suspect`
* Phase 4C rationale: Bodies broadly construct and initialize a connection, but visible differences include callback fields set to None instead of defaults and local registration conditions differing from C.
* C source: `picoquic/quicctx.c:4039-4348`
* C signature: `picoquic_cnx_t * picoquic_create_cnx_internal(picoquic_quic_t *, picoquic_connection_id_t, picoquic_connection_id_t, const struct sockaddr *, uint64_t, uint32_t, const char *, const char *, char, void *, void *)`
* Rust source: `rs/fq/src/internal.rs:3053-3878`
* Rust item: `create_cnx_internal`

### C body
```c
{
    picoquic_cnx_t* cnx = (picoquic_cnx_t*)malloc(sizeof(picoquic_cnx_t));

    if (cnx != NULL) {
        int ret;
        picoquic_local_cnxid_t* cnxid0;

        memset(cnx, 0, sizeof(picoquic_cnx_t));
        cnx->start_time = start_time;
        cnx->phase_delay = INT64_MAX;
        cnx->client_mode = client_mode;
        if (client_mode) {
            if (picoquic_is_connection_id_null(&initial_cnx_id)) {
                picoquic_create_random_cnx_id(quic, &initial_cnx_id, 8);
            }
        }
        cnx->initial_cnxid = initial_cnx_id;
        cnx->quic = quic;
        cnx->pmtud_policy = quic->default_pmtud_policy;
        /* Create the connection ID number 0 */
        cnxid0 = picoquic_create_local_cnxid(cnx, 0, NULL, start_time);

        /* Initialize path updates and quality updates before creating the first path */
        cnx->are_path_callbacks_enabled = quic->are_path_callbacks_enabled;
        cnx->rtt_update_delta = quic->rtt_update_delta;
        cnx->pacing_rate_update_delta = quic->pacing_rate_update_delta;

        /* Initialize the stream data repeat queue */
        picoquic_queue_data_repeat_init(cnx);

        /* Initialize the connection ID stash */
        ret = picoquic_create_path(cnx, start_time, NULL, addr_to, 0, 0);
        if (ret == 0) {
            /* Should return 0, since this is the first path */
            ret = picoquic_init_cnxid_stash(cnx);
        }

        if (ret != 0 || cnxid0 == NULL) {
            picoquic_delete_cnx(cnx);
            /* free(cnx); */
            cnx = NULL;
        } else {
            cnx->next_wake_time = start_time;
            SET_LAST_WAKE(quic, PICOQUIC_QUICCTX);
            picoquic_insert_cnx_in_list(quic, cnx);
            picoquic_insert_cnx_by_wake_time(quic, cnx);
            /* Do not require verification for default path */
            cnx->path[0]->first_tuple->p_local_cnxid = cnxid0;
            cnx->path[0]->first_tuple->challenge_verified = 1;

            cnx->datagram_priority = cnx->quic->default_datagram_priority;
            cnx->high_priority_stream_id = UINT64_MAX;
            for (int i = 0; i < 4; i++) {
                cnx->next_stream_id[i] = i;
            }
            picoquic_register_path(cnx, cnx->path[0]);
        }
    }

    if (cnx != NULL) {
        memcpy(&cnx->local_parameters, &quic->default_tp, sizeof(picoquic_tp_t));
        /* If the default parameters include preferred address, document it */
        if (cnx->local_parameters.preferred_address.is_defined) {
            /* Create an additional CID -- always for path 0, even if multipath */
            picoquic_local_cnxid_t* cnxid1 = picoquic_create_local_cnxid(cnx, 0, NULL, start_time);
            if (cnxid1 != NULL){
                /* copy the connection ID into the local parameter */
                cnx->local_parameters.preferred_address.connection_id = cnxid1->cnx_id;
                /* Create the reset secret */
                (void)picoquic_create_cnxid_reset_secret(cnx->quic, &cnxid1->cnx_id,
                    cnx->local_parameters.preferred_address.statelessResetToken);
            }
        }

        /* Apply the defined MTU MAX if specified and not set in defaults. */
        if (cnx->local_parameters.max_packet_size == 0 && cnx->quic->mtu_max > 0)
        {
            cnx->local_parameters.max_packet_size = cnx->quic->mtu_max -
                PICOQUIC_MTU_OVERHEAD(addr_to);
        }

        /* If local connection ID size is null, don't allow migration */
        if (!cnx->client_mode && quic->local_cnxid_length == 0) {
            cnx->local_parameters.migration_disabled = 1;
        }

        /* Initialize BDP transport parameter */
        if (quic->default_send_receive_bdp_frame) {
           /* Accept and send BDP extension frame */
            cnx->local_parameters.enable_bdp_frame = 1;
        }
 
        /* Initialize local flow control variables to advertised values */
        cnx->maxdata_local = ((uint64_t)cnx->local_parameters.initial_max_data);
        cnx->max_stream_id_bidir_local = STREAM_ID_FROM_RANK(
            cnx->local_parameters.initial_max_stream_id_bidir, cnx->client_mode, 0);
        cnx->max_stream_id_bidir_local_computed = STREAM_TYPE_FROM_ID(cnx->max_stream_id_bidir_local);
        cnx->max_stream_id_unidir_local = STREAM_ID_FROM_RANK(
            cnx->local_parameters.initial_max_stream_id_unidir, cnx->client_mode, 1);
        cnx->max_stream_id_unidir_local_computed = STREAM_TYPE_FROM_ID(cnx->max_stream_id_unidir_local);
       
        /* Initialize padding policy to default for context */
        cnx->padding_multiple = quic->padding_multiple_default;
        cnx->padding_minsize = quic->padding_minsize_default;

        /* Initialize spin policy, ensure that at least 1/8th of connections do not spin */
        cnx->spin_policy = quic->default_spin_policy;
        if (cnx->spin_policy == picoquic_spinbit_basic) {
            uint8_t rand256 = (uint8_t)picoquic_public_random_64();
            if (rand256 < PICOQUIC_SPIN_RESERVE_MOD_256) {
                cnx->spin_policy = picoquic_spinbit_null;
            }
        }
        else if (cnx->spin_policy == picoquic_spinbit_on) {
            /* Option used in test to avoid randomizing spin bit on/off */
            cnx->spin_policy = picoquic_spinbit_basic;
        }

        if (sni != NULL) {
            cnx->sni = picoquic_string_duplicate(sni);
        }

        if (alpn != NULL) {
            cnx->alpn = picoquic_string_duplicate(alpn);
        }

        cnx->callback_fn = quic->default_callback_fn;
        cnx->callback_ctx = quic->default_callback_ctx;
        cnx->congestion_alg = quic->default_congestion_alg;
        cnx->is_preemptive_repeat_enabled = quic->is_preemptive_repeat_enabled;

        /* Initialize key rotation interval to default value */
        cnx->crypto_epoch_length_max = quic->crypto_epoch_length_max;

        for (int epoch = 0; epoch < PICOQUIC_NUMBER_OF_EPOCHS; epoch++) {
            cnx->tls_stream[epoch].send_queue = NULL;
        }

        /* Perform different initializations for clients and servers */
        if (cnx->client_mode) {
            if (preferred_version == 0) {
                cnx->proposed_version = picoquic_supported_versions[0].version;
                cnx->version_index = 0;
            } else {
                cnx->version_index = picoquic_get_version_index(preferred_version);
                if (cnx->version_index < 0) {
                    cnx->version_index = PICOQUIC_INTEROP_VERSION_INDEX;
                    if ((preferred_version & 0x0A0A0A0A) == 0x0A0A0A0A) {
                        /* This is a hack, to allow greasing the cnx ID */
                        cnx->proposed_version = preferred_version;

                    } else {
                        cnx->proposed_version = picoquic_supported_versions[PICOQUIC_INTEROP_VERSION_INDEX].version;
                    }
                } else {
                    cnx->proposed_version = preferred_version;
                }
            }

            cnx->cnx_state = picoquic_state_client_init;

            if (!quic->is_cert_store_not_empty) {
                /* The open SSL certifier always fails if no certificate is stored, so we just use a NULL verifier */
                picoquic_log_app_message(cnx, "No root crt list specified -- certificate will not be verified.\n");

                picoquic_set_null_verifier(quic);
            }
        } else {
            cnx->is_half_open = 1;
            cnx->quic->current_number_half_open += 1;
            if (cnx->quic->current_number_half_open > cnx->quic->max_half_open_before_retry) {
                cnx->quic->check_token = 1;
            }
            cnx->cnx_state = picoquic_state_server_init;
            cnx->initial_cnxid = initial_cnx_id;
            cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id = remote_cnx_id;

            cnx->version_index = picoquic_get_version_index(preferred_version);
            if (cnx->version_index < 0) {
                /* TODO: this is an internal error condition, should not happen */
                cnx->version_index = 0;
                cnx->proposed_version = picoquic_supported_versions[0].version;
            } else {
                cnx->proposed_version = preferred_version;
            }
        }

        for (picoquic_packet_context_enum pc = 0;
            pc < picoquic_nb_packet_context; pc++) {
            picoquic_init_ack_ctx(cnx, &cnx->ack_ctx[pc]);
            picoquic_init_packet_ctx(cnx, &cnx->pkt_ctx[pc], pc);
        }
        /* Initialize the ACK behavior. By default, picoquic abides with the recommendation to send
         * ACK immediately if packets are received out of order (ack_ignore_order_remote = 0),
         * but this behavior creates too many ACKS on high speed links, so picoquic will request
         * the peer to not do that if the "delayed ACK" extension is available (ack_ignore_order_local = 1)
         */
        cnx->ack_ignore_order_local = 1;
        cnx->ack_ignore_order_remote = 0;

        cnx->latest_progress_time = start_time;
        cnx->latest_receive_time = start_time;

        for (int epoch = 0; epoch < PICOQUIC_NUMBER_OF_EPOCHS; epoch++) {
            cnx->tls_stream[epoch].stream_id = 0;
            cnx->tls_stream[epoch].consumed_offset = 0;
            cnx->tls_stream[epoch].fin_offset = 0;
            cnx->tls_stream[epoch].stream_node.left = NULL;
            cnx->tls_stream[epoch].stream_node.parent = NULL;
            cnx->tls_stream[epoch].stream_node.right = NULL;
            cnx->tls_stream[epoch].sent_offset = 0;
            cnx->tls_stream[epoch].local_error = 0;
            cnx->tls_stream[epoch].remote_error = 0;
            cnx->tls_stream[epoch].maxdata_local = UINT64_MAX;
            cnx->tls_stream[epoch].maxdata_remote = UINT64_MAX;

            picosplay_init_tree(&cnx->tls_stream[epoch].stream_data_tree, picoquic_stream_data_node_compare, picoquic_stream_data_node_create, picoquic_stream_data_node_delete, picoquic_stream_data_node_value);
            picoquic_sack_list_init(&cnx->tls_stream[epoch].sack_list);
            /* No need to reset the state flags, as they are not used for the crypto stream */
        }
        
        cnx->ack_frequency_sequence_local = UINT64_MAX;
        cnx->ack_gap_local = 2;
        cnx->ack_frequency_delay_local = PICOQUIC_ACK_DELAY_MAX_DEFAULT;
        cnx->ack_frequency_sequence_remote = UINT64_MAX;
        cnx->ack_gap_remote = 2;
        cnx->ack_delay_remote = PICOQUIC_ACK_DELAY_MIN;
        cnx->max_ack_delay_remote = cnx->ack_delay_remote;
        cnx->max_ack_gap_remote = cnx->ack_gap_remote;
        cnx->max_ack_delay_local = cnx->ack_frequency_delay_local;
        cnx->max_ack_gap_local = cnx->ack_gap_local;
        cnx->min_ack_delay_remote = cnx->ack_delay_remote;
        cnx->min_ack_delay_local = cnx->ack_frequency_delay_local;


        picosplay_init_tree(&cnx->stream_tree, picoquic_stream_node_compare, picoquic_stream_node_create, picoquic_stream_node_delete, picoquic_stream_node_value);

        cnx->congestion_alg = cnx->quic->default_congestion_alg;
        cnx->congestion_alg_option_string = cnx->quic->default_congestion_alg_option_string;
        if (cnx->congestion_alg != NULL) {
            cnx->congestion_alg->alg_init(cnx->path[0], cnx->congestion_alg_option_string, start_time);
        }
    }

    /* Only initialize TLS after all parameters have been set */
    if (cnx != NULL && picoquic_tlscontext_create(quic, cnx) != 0) {
        /* Cannot just do partial creation! */
        picoquic_delete_cnx(cnx);
        cnx = NULL;
    }

    if (cnx != NULL) {
        if (initial_aead_dec != NULL && initial_pn_dec != NULL) {
            cnx->crypto_context[picoquic_epoch_initial].aead_decrypt = initial_aead_dec;
            cnx->crypto_context[picoquic_epoch_initial].pn_dec = initial_pn_dec;
            if (picoquic_get_initial_aead_context(quic, cnx->version_index, &cnx->initial_cnxid,
                cnx->client_mode, 1 /* encoding mode */,
                &cnx->crypto_context[picoquic_epoch_initial].aead_encrypt,
                &cnx->crypto_context[picoquic_epoch_initial].pn_enc) != 0) {
                /* Cannot initialize aead encrypt for initial packets */
                /* Make sure that we do not delete the already allocated
                * initial_aead_dec and initial_pn_dec when clearing the
                * connection, so as not to mess the application management
                * of memory.
                 */
                cnx->crypto_context[picoquic_epoch_initial].aead_decrypt = NULL;
                cnx->crypto_context[picoquic_epoch_initial].pn_dec = NULL;
                picoquic_delete_cnx(cnx);
                cnx = NULL;
            }
        }
        else if (picoquic_setup_initial_traffic_keys(cnx)) {
            /* Cannot initialize aead for initial packets */
            picoquic_delete_cnx(cnx);
            cnx = NULL;
        }
    }

    if (cnx != NULL && !client_mode && quic->local_cnxid_length > 0) {
        if (picoquic_register_net_icid(cnx) != 0) {
            DBG_PRINTF("%s", "Could not register the ICID in table.\n"); 
            if (initial_aead_dec != NULL && initial_pn_dec != NULL) {
                /* Make sure that we do not delete the already allocated
                * initial_aead_dec and initial_pn_dec when clearing the
                * connection, so as not to mess the application management
                * of memory.
                 */
                cnx->crypto_context[picoquic_epoch_initial].aead_decrypt = NULL;
                cnx->crypto_context[picoquic_epoch_initial].pn_dec = NULL;
            }
            picoquic_delete_cnx(cnx);
            cnx = NULL;
        }
    }

    if (quic->use_unique_log_names) {
        picoquic_crypto_random(quic, &cnx->log_unique, sizeof(cnx->log_unique));
    }

    if (cnx != NULL && !cnx->client_mode) {
        picoquic_log_new_connection(cnx);
    }

    return cnx;
}
```

### Rust body
```rust
    ) -> Result<ConnectionToken, crate::Error> {
        use core::net::{IpAddr, Ipv4Addr};

        let zero_instant = crate::Instant::from_ticks(0);
        let zero_dur = crate::Duration::from_ticks(0);

        if client_mode && initial_cnx_id.is_empty() {
            initial_cnx_id = crate::create_random_cnx_id(self, 8);
        }

        let supported_version_index = |version: u32| -> Option<i32> {
            SUPPORTED_VERSIONS
                .iter()
                .position(|v| *v as u32 == version)
                .map(|i| i as i32)
        };
        let interop_index = SUPPORTED_VERSIONS
            .iter()
            .position(|v| *v == INTEROP_VERSION_LATEST)
            .unwrap_or(0) as i32;
        let (version_index, proposed_version) = if client_mode {
            if preferred_version == 0 {
                (0, SUPPORTED_VERSIONS[0] as u32)
            } else if let Some(idx) = supported_version_index(preferred_version) {
                (idx, preferred_version)
            } else if (preferred_version & 0x0A0A0A0A) == 0x0A0A0A0A {
                (interop_index, preferred_version)
            } else {
                (interop_index, INTEROP_VERSION_LATEST as u32)
            }
        } else if let Some(idx) = supported_version_index(preferred_version) {
            (idx, preferred_version)
        } else {
            (0, SUPPORTED_VERSIONS[0] as u32)
        };

        // Determine connection state.
        let connection_state = if client_mode {
            crate::State::ClientInit
        } else {
            crate::State::ServerInit
        };

        // Build a null path/tuple for the initial path.
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let peer_addr = addr_to.copied().unwrap_or(default_addr);
        let mut local_connection_ids = Arena::new();
        let initial_lcid_token = local_connection_ids.insert(LocalConnectionId {
            connection_by_id_membership: None,
            path_id: 0,
            sequence: 0,
            create_time: start_time,
            connection_id: initial_cnx_id,
            is_acked: false,
        })?;
        let initial_tuple = Tuple {
            unique_path_id: 0,
            peer_addr,
            local_addr: default_addr,
            if_index: 0,
            observed_addr: default_addr,
            remote_connection_id_index: None,
            local_connection_id: Some(initial_lcid_token),
            nb_observed_repeat: 0,
            observed_time: zero_instant,
            challenge_response: 0,
            challenge: [0u64; CHALLENGE_REPEAT_MAX],
            challenge_time: zero_instant,
            demotion_time: zero_instant,
            challenge_time_first: zero_instant,
            is_nat_rebinding: 0,
            challenge_repeat_count: 0,
            is_backup: 0,
            challenge_required: false,
            challenge_verified: true, // C: path[0] first_tuple verified
            challenge_failed: false,
            response_required: false,
            to_preferred_address: false,
        };
        let initial_path = Path {
            registered_peer_addr: peer_addr,
            connection_by_net_membership: None,
            unique_path_id: 0,
            app_path_ctx: None,
            ack_ctx: AckContext {
                sack_list: SackList::new(),
                time_stamp_largest_received: crate::Instant::from_ticks(u64::MAX),
                act: [
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start_time,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start_time,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                ],
                crypto_rotation_sequence: 0,
                ecn_ect0_total_local: 0,
                ecn_ect1_total_local: 0,
                ecn_ce_total_local: 0,
                sending_ecn_ack: false,
            },
            pkt_ctx: PacketContextState {
                send_sequence: 0,
                next_sequence_hole: 0,
                retransmit_sequence: 0,
                highest_acknowledged: 0u64.wrapping_sub(1),
                latest_time_acknowledged: start_time,
                highest_acknowledged_time: start_time,
                pending: BTreeMap::new(),
                retransmitted: BTreeMap::new(),
                preemptive_repeat_seq: None,
                retransmitted_queue_size: 0,
                ecn_ect0_total_remote: 0,
                ecn_ect1_total_remote: 0,
                ecn_ce_total_remote: 0,
                ack_of_ack_requested: false,
            },
            tuples: vec![initial_tuple],
            observed_address_received: 0,
            observed_sequence_sent: 0,
            observed_addr_acked: false,
            last_non_path_probing_pn: 0,
            demotion_time: zero_instant,
            last_sent_time: zero_instant,
            status_sequence_to_receive_next: 0,
            status_sequence_sent_last: 0,
            mtu_probe_sent: false,
            path_is_published: false,
            path_is_backup: false,
            path_is_demoted: false,
            path_abandon_received: false,
            path_abandon_sent: false,
            current_spin: false,
            last_bw_estimate_path_limited: false,
            path_cid_rotated: false,
            is_nat_challenge: false,
            is_cc_data_updated: false,
            is_multipath_probe_needed: false,
            is_ssthresh_initialized: false,
            is_token_published: false,
            is_ticket_seeded: false,
            is_bdp_sent: false,
            is_nominal_ack_path: false,
            is_ack_lost: false,
            is_ack_expected: false,
            is_datagram_ready: false,
            is_pto_required: false,
            is_probing_nat: false,
            is_lost_feedback_notified: false,
            is_cca_probing_up: false,
            rtt_is_initialized: false,
            sending_path_cid_blocked_frame: false,
            last_packet_received_at: zero_instant,
            last_loss_event_detected: zero_instant,
            nb_retransmit: 0,
            total_bytes_lost: 0,
            nb_losses_found: 0,
            nb_timer_losses: 0,
            nb_spurious: 0,
            nb_losses_reported: 0,
            q_square: 0,
            max_ack_delay: ACK_DELAY_MAX_DEFAULT,
            rtt_sample: zero_dur,
            one_way_delay_sample: zero_dur,
            smoothed_rtt: INITIAL_RTT,
            rtt_variant: zero_dur,
            retransmit_timer: INITIAL_RETRANSMIT_TIMER,
            rtt_min: INITIAL_RTT,
            rtt_max: zero_dur,
            max_spurious_rtt: zero_dur,
            max_reorder_delay: zero_dur,
            max_reorder_gap: 0,
            latest_sent_time: zero_instant,
            rtt_packet_previous_period: zero_dur,
            rtt_time_previous_period: zero_dur,
            nb_rtt_estimate_in_period: 0,
            sum_rtt_estimate_in_period: zero_dur,
            max_rtt_estimate_in_period: zero_dur,
            min_rtt_estimate_in_period: zero_dur,
            send_mtu: ENFORCED_INITIAL_MTU,
            send_mtu_max_tried: 0,
            delivered: 0,
            delivered_last: 0,
            delivered_time_last: zero_instant,
            delivered_sent_last: zero_instant.ticks(),
            delivered_limited_index: 0,
            delivered_last_packet: 0,
            bandwidth_estimate: 0,
            bandwidth_estimate_max: 0,
            max_sample_acked_time: zero_instant,
            max_sample_sent_time: zero_instant,
            max_sample_delivered: 0,
            peak_bandwidth_estimate: 0,
            bytes_sent: 0,
            received: 0,
            receive_rate_epoch: 0,
            received_prior: 0,
            receive_rate_estimate: 0,
            receive_rate_max: 0,
            cwin: CWIN_INITIAL,
            bytes_in_transit: 0,
            last_sender_limited_time: zero_instant,
            last_cwin_blocked_time: zero_instant,
            last_time_acked_data_frame_sent: zero_instant,
            congestion_alg_state: None,
            pacing: Pacing {
                rate: 0,
                evaluation_time: zero_instant,
                bucket_max: 0,
                packet_time_microsec: zero_dur,
                quantum_max: 0,
                rate_max: 0,
                bandwidth_pause: 0,
                bucket_nanosec: 0,
                packet_time_nanosec: 0,
            },
            nb_mtu_losses: 0,
            lost_after_delivered: 0,
            responder: 0,
            challenger: 0,
            polled: 0,
            paced: 0,
            congested: 0,
            selected: 0,
            nb_delay_outliers: 0,
            rtt_update_delta: self.rtt_update_delta,
            pacing_rate_update_delta: self.pacing_rate_update_delta,
            rtt_threshold_low: zero_dur,
            rtt_threshold_high: zero_dur,
            pacing_rate_threshold_low: 0,
            pacing_rate_threshold_high: 0,
            receive_rate_threshold_low: 0,
            receive_rate_threshold_high: 0,
            rtt_min_remote: zero_dur,
            cwin_remote: 0,
            ip_client_remote: [0u8; 16],
            ip_client_remote_length: 0,
        };

        // Build the initial CID list for path 0.
        let initial_cid_list = LocalConnectionIdList {
            unique_path_id: 0,
            local_connection_id_sequence_next: 1,
            local_connection_id_retire_before: 0,
            local_connection_id_oldest_created: start_time.ticks(),
            nb_local_connection_id_expired: 0,
            is_demoted: false,
            demotion_time: zero_instant,
            connection_ids: vec![initial_lcid_token],
        };

        // Build the initial remote CID stash for path 0.
        let zero_pkt_ctx = PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: 0u64.wrapping_sub(1),
            latest_time_acknowledged: start_time,
            highest_acknowledged_time: start_time,
            pending: BTreeMap::new(),
            retransmitted: BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        };
        let initial_remote_cid = RemoteConnectionId {
            sequence: 0,
            connection_id: remote_cnx_id,
            reset_secret: [0u8; RESET_SECRET_SIZE],
            nb_path_references: 1,
            needs_removal: false,
            retire_sent: false,
            retire_acked: false,
            pkt_ctx: zero_pkt_ctx,
        };
        let initial_stash = RemoteConnectionIdStash {
            unique_path_id: 0,
            retire_connection_id_before: 0,
            connection_ids: vec![initial_remote_cid],
            is_in_use: true,
        };

        // Build the null AckContext for connection-level use.
        fn make_ack_ctx(start: crate::Instant) -> AckContext {
            let zi = crate::Instant::from_ticks(0);
            AckContext {
                sack_list: SackList::new(),
                time_stamp_largest_received: crate::Instant::from_ticks(u64::MAX),
                act: [
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start,
                        time_oldest_unack_packet_received: zi,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start,
                        time_oldest_unack_packet_received: zi,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                ],
                crypto_rotation_sequence: 0,
                ecn_ect0_total_local: 0,
                ecn_ect1_total_local: 0,
                ecn_ce_total_local: 0,
                sending_ecn_ack: false,
            }
        }
        fn make_pkt_ctx(start: crate::Instant) -> PacketContextState {
            PacketContextState {
                send_sequence: 0,
                next_sequence_hole: 0,
                retransmit_sequence: 0,
                highest_acknowledged: 0u64.wrapping_sub(1),
                latest_time_acknowledged: start,
                highest_acknowledged_time: start,
                pending: BTreeMap::new(),
                retransmitted: BTreeMap::new(),
                preemptive_repeat_seq: None,
                retransmitted_queue_size: 0,
                ecn_ect0_total_remote: 0,
                ecn_ect1_total_remote: 0,
                ecn_ce_total_remote: 0,
                ack_of_ack_requested: false,
            }
        }
        fn make_tls_stream(_start: crate::Instant) -> StreamHead {
            StreamHead {
                stream_tree_membership: None,
                stream_id: 0,
                affinity_path: None,
                consumed_offset: 0,
                fin_offset: 0,
                reset_offset: 0,
                maxdata_local: u64::MAX,
                maxdata_local_acked: 0,
                maxdata_remote: u64::MAX,
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
                stream_priority: 0,
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
            }
        }
        let _ = start_time; // used in make_*
        let tls_streams = core::array::from_fn::<StreamHead, NUMBER_OF_EPOCHS, _>(|_| {
            make_tls_stream(start_time)
        });
        let crypto_contexts =
            core::array::from_fn::<CryptoContext, NUMBER_OF_EPOCHS, _>(|_| CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            });

        let mut local_params = self.default_tp.clone();
        if local_params.max_packet_size == 0 && self.mtu_max > 0 {
            local_params.max_packet_size =
                self.mtu_max.saturating_sub(crate::mtu_overhead(&peer_addr));
        }
        if !client_mode && self.local_connection_id_length == 0 {
            local_params.migration_disabled = true;
        }
        if self.default_send_receive_bdp_frame {
            local_params.enable_bdp_frame = true;
        }
        let maxdata_local = local_params.initial_max_data;
        // C: STREAM_ID_FROM_RANK(rank, client_mode, unidir)
        //  = 4*rank + (client_mode ? 0 : 1) + (unidir ? 2 : 0)
        let role_bit = if client_mode { 0u64 } else { 1u64 };
        let max_stream_id_bidir_local = 4 * local_params.initial_max_stream_id_bidir + role_bit;
        let max_stream_id_unidir_local =
            4 * local_params.initial_max_stream_id_unidir + role_bit + 2;

        let mut spin_policy = self.default_spin_policy;
        if spin_policy == SpinbitVersion::Basic {
            let rand256 = crate::public_random_64() as u8;
            if rand256 < SPIN_RESERVE_MOD_256 {
                spin_policy = SpinbitVersion::Null;
            }
        } else if spin_policy == SpinbitVersion::On {
            spin_policy = SpinbitVersion::Basic;
        }

        let mut cnx = Connection {
            proposed_version,
            rejected_version: 0,
            desired_version: 0,
            version_index,

            is_0rtt_accepted: false,
            remote_parameters_received: false,
            client_mode,
            key_phase_enc: false,
            key_phase_dec: false,
            zero_rtt_data_accepted: false,
            sending_ecn_ack: false,
            sent_blocked_frame: false,
            stream_blocked_bidir_sent: false,
            stream_blocked_unidir_sent: false,
            max_stream_data_needed: false,
            path_demotion_needed: false,
            tuple_demotion_needed: false,
            alt_path_challenge_needed: false,
            is_handshake_finished: false,
            is_handshake_done_acked: false,
            is_new_token_acked: false,
            is_1rtt_received: false,
            is_1rtt_acked: false,
            has_successful_probe: false,
            grease_transport_parameters: false,
            test_large_chello: false,
            initial_validated: false,
            initial_repeat_needed: false,
            is_loss_bit_enabled_incoming: false,
            is_loss_bit_enabled_outgoing: false,
            is_ack_frequency_negotiated: false,
            is_ack_frequency_updated: false,
            recycle_sooner_needed: false,
            is_time_stamp_enabled: false,
            is_time_stamp_sent: false,
            is_pacing_update_requested: false,
            is_path_quality_update_requested: false,
            is_hcid_verified: false,
            do_grease_quic_bit: false,
            quic_bit_greased: false,
            quic_bit_received_0: false,
            is_half_open: !client_mode,
            did_receive_short_initial: false,
            ack_ignore_order_local: true,
            ack_ignore_order_remote: false,
            are_path_callbacks_enabled: self.are_path_callbacks_enabled,
            is_sending_large_buffer: false,
            is_preemptive_repeat_enabled: self.is_preemptive_repeat_enabled,
            do_version_negotiation: false,
            send_receive_bdp_frame: false,
            cwin_notified_from_seed: false,
            is_datagram_ready: false,
            is_immediate_ack_required: false,
            is_multipath_enabled: false,
            is_lost_feedback_notification_required: false,
            is_forced_probe_up_required: false,
            is_address_discovery_provider: false,
            is_address_discovery_receiver: false,
            is_subscribed_to_path_allowed: false,
            is_notified_that_path_is_allowed: false,
            is_reset_stream_at_enabled: false,

            pmtud_policy: self.default_pmtud_policy,
            spin_policy,
            idle_timeout: crate::Duration::from_ticks(0),
            local_parameters: local_params,
            remote_parameters: crate::tp::TransportParameters::default(),
            padding_multiple: self.padding_multiple_default,
            padding_minsize: self.padding_minsize_default,
            seed_ip_addr: None,
            seed_rtt_min: zero_dur,
            seed_cwin: 0,

            issued_ticket_id: 0,
            resumed_ticket_id: 0,

            sni: sni.map(|s| s.to_owned()),
            alpn: alpn.map(|s| s.to_owned()),
            alpn_proposals: Vec::new(),
            max_early_data_size: 0,

            callback_fn: None, // C: = quic->default_callback_fn (not clonable)
            callback_ctx: None,

            connection_state,
            initial_connection_id: initial_cnx_id,
            original_connection_id: initial_cnx_id,
            registered_icid_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            connection_by_icid_membership: None,
            registered_secret_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            registered_reset_secret: [0u8; RESET_SECRET_SIZE],
            connection_by_secret_membership: None,

            local_cid_length: self.local_connection_id_length,
            local_connection_id_ttl: self.local_connection_id_ttl,
            random_initial: self.random_initial,

            start_time,
            phase_delay: i64::MAX,
            application_error: 0,
            local_error: 0,
            local_error_reason: None,
            remote_application_error: 0,
            remote_error: 0,
            offending_frame_type: 0,
            remote_error_reason: None,
            retry_token: Vec::new(),

            next_wake_time: start_time,
            connection_wake_membership: None,
            app_wake_time: crate::Instant::from_ticks(u64::MAX),

            tls_ctx: None,
            crypto_epoch_length_max: self.crypto_epoch_length_max,
            crypto_epoch_sequence: 0,
            crypto_rotation_time_guard: zero_instant,
            tls_sendbuf: Vec::new(),
            psk_cipher_suite_id: 0,
            ech_client_config: None,

            tls_stream: tls_streams,
            crypto_context: crypto_contexts,
            crypto_context_old: CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            },
            crypto_context_new: CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            },
            app_secret_enc: [0u8; 64],
            app_secret_dec: [0u8; 64],
            app_secret_len: 32,
            crypto_failure_count: 0,

            latest_progress_time: start_time,
            latest_receive_time: start_time,
            last_close_sent: zero_instant,
            pkt_ctx: core::array::from_fn(|_| make_pkt_ctx(start_time)),
            ack_ctx: core::array::from_fn(|_| make_ack_ctx(start_time)),
            observed_number: 0,

            nb_bytes_queued: 0,
            nb_zero_rtt_sent: 0,
            nb_zero_rtt_acked: 0,
            nb_zero_rtt_received: 0,
            max_mtu_sent: 0,
            max_mtu_received: 0,
            nb_packets_received: 0,
            nb_trains_sent: 0,
            nb_trains_short: 0,
            nb_trains_blocked_cwin: 0,
            nb_trains_blocked_pacing: 0,
            nb_trains_blocked_others: 0,
            nb_packets_sent: 0,
            nb_packets_logged: 0,
            use_long_log: self.use_long_log,
            nb_retransmission_total: 0,
            nb_preemptive_repeat: 0,
            nb_spurious: 0,
            nb_crypto_key_rotations: 0,
            nb_packet_holes_inserted: 0,
            max_ack_delay_remote: ACK_DELAY_MAX,
            max_ack_gap_remote: 2,
            max_ack_delay_local: ACK_DELAY_MAX_DEFAULT,
            max_ack_gap_local: 2,
            min_ack_delay_remote: ACK_DELAY_MAX,
            min_ack_delay_local: ACK_DELAY_MAX_DEFAULT,
            cwin_blocked: false,
            flow_blocked: false,
            stream_blocked: false,

            congestion_alg: self.default_congestion_alg,
            congestion_alg_option_string: None,

            rtt_update_delta: self.rtt_update_delta,
            pacing_rate_update_delta: self.pacing_rate_update_delta,
            pacing_rate_signalled: 0,
            pacing_increase_threshold: 0,
            pacing_decrease_threshold: 0,
            pacing_change_threshold: 0,

            initial_data_received: 0,
            initial_data_sent: 0,

            data_sent: 0,
            data_received: 0,
            offset_received: 0,
            maxdata_local,
            maxdata_local_acked: 0,
            maxdata_remote: 0,
            max_stream_data_local: 0,
            max_stream_data_remote: 0,
            max_stream_id_bidir_local,
            max_stream_id_bidir_rank_acked: 0,
            max_stream_id_bidir_local_computed: 0,
            max_stream_id_bidir_remote: 0,
            max_stream_id_unidir_local,
            max_stream_id_unidir_rank_acked: 0,
            max_stream_id_unidir_local_computed: 0,
            max_stream_id_unidir_remote: 0,

            misc_frames: std::collections::VecDeque::new(),

            stream_tree: crate::splay::SplayTree::default(),
            streams: crate::arena::Arena::new(),
            output_streams: std::collections::VecDeque::new(),
            high_priority_stream_id: u64::MAX,
            next_stream_id: [0, 1, 2, 3],
            priority_limit_for_bypass: 0,

            queue_data_repeat_tree: crate::splay::SplayTree::default(),
            queued_packets: crate::arena::Arena::new(),

            datagrams: std::collections::VecDeque::new(),
            datagram_priority: self.default_datagram_priority as u64,
            datagram_conflicts_count: 0,
            datagram_conflicts_max: 0,

            keep_alive_interval: zero_dur,

            paths: vec![initial_path],
            last_path_polled: 0,
            unique_path_id_next: 1,
            nominal_path_for_ack: None,
            status_sequence_to_send_next: 0,
            max_path_id_local: 0,
            max_path_id_acknowledged: 0,
            max_path_id_remote: 0,
            paths_blocked_acknowledged: 0,

            remote_connection_id_stashes: vec![initial_stash],

            next_path_id_in_lists: 1,
            max_path_id_in_connection_id_lists: 0,
            local_connection_id_lists: vec![initial_cid_list],
            local_connection_ids,

            ack_frequency_sequence_local: u64::MAX,
            ack_gap_local: 2,
            ack_frequency_delay_local: ACK_DELAY_MAX_DEFAULT,
            ack_frequency_sequence_remote: u64::MAX,
            ack_gap_remote: 2,
            ack_delay_remote: ACK_DELAY_MAX,
            ack_reordering_threshold_remote: 0,

            sooner_stateless: std::collections::VecDeque::new(),

            log_unique: 0,
            f_binlog: None,
            binlog_file_name: None,
            text_log_fns: self.text_log_fns.clone(),
            bin_log_fns: self.bin_log_fns.clone(),
            qlog_fns: self.qlog_fns.clone(),
            memlog_call_back: None,
            memlog_ctx: None,
            qlog_ctx: None,
            own_token: None,
            quic_ptr: std::ptr::null_mut(),
        };
        cnx.create_tls_context(self)?;

        // Insert into the connection arena.
        let token = self
            .connections
            .insert(cnx)
            .map_err(|_| crate::Error::Memory)?;
        let quic_ptr = self as *mut Quic;
        if let Some(cnx) = self.connections.get_mut(token) {
            cnx.own_token = Some(token);
            cnx.quic_ptr = quic_ptr;
            cnx.setup_initial_traffic_keys()?;
            if let Some(alg) = cnx.congestion_alg {
                let option = cnx.congestion_alg_option_string.as_deref();
                alg.algorithm
                    .alg_init(&mut cnx.paths[0], option, start_time);
            }
        }

        // Update half-open count for server connections.
        if !client_mode {
            self.current_number_half_open += 1;
            if self.current_number_half_open > self.max_half_open_before_retry {
                self.check_token = true;
            }
        }
        self.insert_cnx_in_list(token);

        {
            let cid = self
                .connections
                .get(token)
                .map(|c| c.initial_connection_id)
                .unwrap_or(initial_cnx_id);
            if !cid.is_empty() {
                if self.connection_by_id.lookup(&cid).is_some() {
                    self.delete_connection(token);
                    return Err(crate::Error::Generic);
                }
                let (membership, _) = self.connection_by_id.insert(cid, token)?;
                if let Some(cnx) = self.connections.get_mut(token)
                    && let Some(l_cid) = cnx.local_connection_ids.get_mut(initial_lcid_token)
                {
                    l_cid.connection_by_id_membership = Some(membership);
                }
            }
        }

        let has_preferred_address = self
            .connections
            .get(token)
            .map(|cnx| {
                cnx.local_parameters.preferred_address.v4.is_some()
                    || cnx.local_parameters.preferred_address.v6.is_some()
            })
            .unwrap_or(false);
        if has_preferred_address {
            let cid_token = self.create_local_cnxid(token, 0, None, start_time)?;
            let preferred_cid = self
                .connections
                .get(token)
                .and_then(|cnx| cnx.local_connection_ids.get(cid_token))
                .map(|local_cid| local_cid.connection_id)
                .ok_or(crate::Error::Generic)?;
            let mut reset_token = [0u8; crate::RESET_SECRET_SIZE];
            self.create_connection_id_reset_secret(&preferred_cid, &mut reset_token)?;
            if let Some(cnx) = self.connections.get_mut(token) {
                cnx.local_parameters.preferred_address.connection_id = preferred_cid;
                cnx.local_parameters.preferred_address.stateless_reset_token = reset_token;
            }
        }

        if let Some(cnx) = self.connections.get_mut(token)
            && let Some(path) = cnx.paths.first_mut()
        {
            path.path_is_published = true;
        }

        if self.local_connection_id_length == 0 {
            let should_register = self
                .connections
                .get(token)
                .and_then(|cnx| cnx.paths.first())
                .and_then(|path| path.tuples.first())
                .map(|tuple| !crate::socket_addr_is_unspecified(&tuple.peer_addr))
                .unwrap_or(false);
            if should_register && self.register_net_id(token, 0).is_err() {
                self.delete_connection(token);
                return Err(crate::Error::Generic);
            }
        }

        if !client_mode
            && self.local_connection_id_length > 0
            && self.register_net_icid(token).is_err()
        {
            self.delete_connection(token);
            return Err(crate::Error::Generic);
        }

        if self.use_unique_log_names
            && let Some(cnx) = self.connections.get_mut(token)
        {
            let mut bytes = [0u8; 2];
            rand_core::RngCore::fill_bytes(&mut *self.rng, &mut bytes);
            cnx.log_unique = u16::from_le_bytes(bytes);
        }

        Ok(token)
    }
```

## `picoquic/quicctx.c:picoquic_get_earliest_cnx_to_wake`
* Phase 4C status: `suspect`
* Phase 4C rationale: C takes the first wake-tree entry and only applies the max threshold when max_wake_time is nonzero; Rust filters by the supplied wake time unconditionally and scans all connections.
* C source: `picoquic/quicctx.c:1522-1531`
* C signature: `picoquic_cnx_t * picoquic_get_earliest_cnx_to_wake(picoquic_quic_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:3222-3228`
* Rust item: `earliest_cnx_to_wake`

### C body
```c
{
    picoquic_cnx_t* cnx = (picoquic_cnx_t *)picoquic_wake_list_node_value(picosplay_first(&quic->cnx_wake_tree));
    if (cnx != NULL && max_wake_time != 0 && cnx->next_wake_time > max_wake_time)
    {
        cnx = NULL;
    }

    return cnx;
}
```

### Rust body
```rust
    pub fn earliest_cnx_to_wake(&mut self, wake_time: Instant) -> Option<&mut Connection> {
        let threshold = wake_time.ticks();
        self.connections
            .iter_mut()
            .filter(|c| c.next_wake_time.ticks() <= threshold)
            .min_by_key(|c| c.next_wake_time.ticks())
    }
```
