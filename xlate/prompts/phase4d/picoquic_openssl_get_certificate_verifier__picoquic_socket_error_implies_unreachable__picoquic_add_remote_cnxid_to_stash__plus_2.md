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

## `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_get_certificate_verifier`
* Phase 4C status: `suspect`
* Phase 4C rationale: C passes is_cert_store_not_empty into verifier creation and explicitly sets the free callback on success; Rust does not visibly pass that flag and returns an Option registration instead.
* C source: `picoquic/picoquic_ptls_openssl.c:277-292`
* C signature: `ptls_verify_certificate_t * picoquic_openssl_get_certificate_verifier(const char *, unsigned int *, picoquic_free_verify_certificate_ctx *)`
* Rust source: `rs/fq/src/sys/openssl.rs:419-427`
* Rust item: `picoquic_openssl_get_certificate_verifier`

### C body
```c
{
    ptls_verify_certificate_t* verify_cert = NULL;
    ptls_openssl_verify_certificate_t* verifier = picoquic_openssl_get_openssl_certificate_verifier(cert_root_file_name,
        is_cert_store_not_empty);

    if (verifier == NULL) {
        free_certificate_verifier_fn = NULL;
    }
    else {
        verify_cert = &verifier->super;
        *free_certificate_verifier_fn = picoquic_openssl_dispose_certificate_verifier;
    }
    return verify_cert;
}
```

### Rust body
```rust
) -> Option<CertificateVerifierRegistration> {
    let verifier = get_openssl_certificate_verifier(cert_root_file_name).ok()?;
    Some(CertificateVerifierRegistration {
        verifier,
        free_certificate_verifier_fn: picoquic_openssl_dispose_certificate_verifier,
    })
}
```

## `picoquic/picosocks.c:picoquic_socket_error_implies_unreachable`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks explicit raw error-code lists including EAFNOSUPPORT; Rust maps to ErrorKind and matches a shorter visible set.
* C source: `picoquic/picosocks.c:1402-1421`
* C signature: `int picoquic_socket_error_implies_unreachable(int)`
* Rust source: `rs/fq/src/socks.rs:46-56`
* Rust item: `is_unreachable`

### C body
```c
{
#ifdef _WINDOWS
    static int unreachable_errors[] = {
        WSAEACCES, WSAEADDRNOTAVAIL, WSAEAFNOSUPPORT, WSAECONNRESET,
        WSAEDESTADDRREQ, WSAEHOSTUNREACH, WSAENETDOWN, WSAENETRESET,
        WSAENETUNREACH, WSAESHUTDOWN, -1 };
#else
    static int unreachable_errors[] = {
        EAFNOSUPPORT, ECONNRESET, EHOSTUNREACH, ENETDOWN, ENETUNREACH, -1 };
#endif
    size_t nb_errors = sizeof(unreachable_errors) / sizeof(int);
    int ret = 0;

    for (size_t i = 0; ret == 0 && i < nb_errors; i++) {
        ret = (sock_err == unreachable_errors[i]);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn is_unreachable(self) -> bool {
        use std::io::ErrorKind;
        let kind = std::io::Error::from_raw_os_error(self.0).kind();
        matches!(
            kind,
            ErrorKind::ConnectionReset
                | ErrorKind::HostUnreachable
                | ErrorKind::NetworkDown
                | ErrorKind::NetworkUnreachable
        )
    }
```

## `picoquic/quicctx.c:picoquic_add_remote_cnxid_to_stash`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust handles parsing, duplicate checks, and insertion, but has no visible null-remote-CID protocol check or active_connection_id_limit enforcement present in C.
* C source: `picoquic/quicctx.c:2944-3036`
* C signature: `uint64_t picoquic_add_remote_cnxid_to_stash(picoquic_cnx_t *, picoquic_remote_cnxid_stash_t *, uint64_t, const uint64_t, const uint8_t, const uint8_t *, const uint8_t *, picoquic_remote_cnxid_t **)`
* Rust source: `rs/fq/src/internal.rs:5022-5121`
* Rust item: `add_remote_connection_id_to_stash`

### C body
```c
{
    int ret = 0;
    int is_duplicate = 0;
    size_t nb_cid_received = 0;
    picoquic_connection_id_t cnx_id;
    picoquic_remote_cnxid_t* next_stash = remote_cnxid_stash->cnxid_stash_first;
    picoquic_remote_cnxid_t* last_stash = NULL;
    picoquic_remote_cnxid_t* stashed = NULL;
    int nb_cid_retired_before = 0;

    if (retire_before < remote_cnxid_stash->retire_cnxid_before) {
        retire_before = remote_cnxid_stash->retire_cnxid_before;
    }

    /* verify the format */
    if (picoquic_parse_connection_id(cnxid_bytes, cid_length, &cnx_id) == 0) {
        ret = PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR;
    }

    if (ret == 0 && cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len == 0) {
        /* Protocol error. The peer is using null length cnx_id */
        ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
    }

    while (ret == 0 && is_duplicate == 0 && next_stash != NULL) {
        if (picoquic_compare_connection_id(&cnx_id, &next_stash->cnx_id) == 0)
        {
            if (next_stash->sequence == sequence &&
                cnx_id.id_len == next_stash->cnx_id.id_len &&
                (cnx_id.id_len == 0 || memcmp(cnx_id.id, next_stash->cnx_id.id, cnx_id.id_len) == 0) &&
                memcmp(secret_bytes, next_stash->reset_secret, PICOQUIC_RESET_SECRET_SIZE) == 0) {
                is_duplicate = 1;
            }
            else {
                ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
            }
            break;
        }
        else if (next_stash->sequence == sequence) {
            ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
        }
        else if (memcmp(secret_bytes, next_stash->reset_secret, PICOQUIC_RESET_SECRET_SIZE) == 0) {
            ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
        }
        else {
            if (next_stash->sequence < retire_before || next_stash->retire_sent) {
                nb_cid_retired_before++;
            }
            nb_cid_received++;
        }
        last_stash = next_stash;
        next_stash = next_stash->next;
    }

    if (ret == 0 && is_duplicate == 0) {
        if (nb_cid_received >= cnx->local_parameters.active_connection_id_limit + nb_cid_retired_before ||
            nb_cid_received >= 2*cnx->local_parameters.active_connection_id_limit) {
            ret = PICOQUIC_TRANSPORT_CONNECTION_ID_LIMIT_ERROR;
        }
        else {
            stashed = (picoquic_remote_cnxid_t*)malloc(sizeof(picoquic_remote_cnxid_t));

            if (stashed == NULL) {
                ret = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
            }
            else {
                memset(stashed, 0, sizeof(picoquic_remote_cnxid_t));
                (void)picoquic_parse_connection_id(cnxid_bytes, cid_length, &stashed->cnx_id);
                stashed->sequence = sequence;
                memcpy(stashed->reset_secret, secret_bytes, PICOQUIC_RESET_SECRET_SIZE);
                stashed->next = NULL;

                if (last_stash == NULL) {
                    remote_cnxid_stash->cnxid_stash_first = stashed;
                }
                else {
                    last_stash->next = stashed;
                }
            }
        }
    }

    /* the return argument is only used in tests */

    if (pstashed != NULL) {
        *pstashed = stashed;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> StashResult {
        // C: picoquic_add_remote_cnxid_to_stash
        // C transport error codes: 0x1=INTERNAL, 0x7=FRAME_FORMAT, 0xA=PROTOCOL_VIOLATION
        const INTERNAL_ERROR: u64 = 0x1;
        const FRAME_FORMAT_ERROR: u64 = 0x7;
        const PROTOCOL_VIOLATION: u64 = 0xA;

        let stash = match self.remote_connection_id_stashes.get_mut(stash_index) {
            Some(s) => s,
            None => {
                return StashResult {
                    status: INTERNAL_ERROR,
                    stashed_index: None,
                };
            }
        };

        let cnx_id = match crate::ConnectionId::clone_from_slice(connection_id_bytes) {
            Some(id) => id,
            None => {
                return StashResult {
                    status: FRAME_FORMAT_ERROR,
                    stashed_index: None,
                };
            }
        };

        // Ensure retire_connection_id_before moves forward.
        if retire_before_next > stash.retire_connection_id_before {
            stash.retire_connection_id_before = retire_before_next;
        }

        // Check for duplicates / sequence collision.
        let mut secret_arr = [0u8; RESET_SECRET_SIZE];
        let slen = secret_bytes.len().min(RESET_SECRET_SIZE);
        secret_arr[..slen].copy_from_slice(&secret_bytes[..slen]);

        for (idx, r) in stash.connection_ids.iter().enumerate() {
            if r.connection_id == cnx_id {
                if r.sequence == sequence && r.reset_secret == secret_arr {
                    // Duplicate — not an error, just no-op.
                    return StashResult {
                        status: 0,
                        stashed_index: Some(idx),
                    };
                } else {
                    return StashResult {
                        status: PROTOCOL_VIOLATION,
                        stashed_index: None,
                    };
                }
            } else if r.sequence == sequence || r.reset_secret == secret_arr {
                return StashResult {
                    status: PROTOCOL_VIOLATION,
                    stashed_index: None,
                };
            }
        }

        let zero_pkt_ctx = PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: 0u64.wrapping_sub(1),
            latest_time_acknowledged: self.start_time,
            highest_acknowledged_time: self.start_time,
            pending: std::collections::BTreeMap::new(),
            retransmitted: std::collections::BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        };
        let stash = &mut self.remote_connection_id_stashes[stash_index];
        let new_rcid = RemoteConnectionId {
            sequence,
            connection_id: cnx_id,
            reset_secret: secret_arr,
            nb_path_references: 0,
            needs_removal: false,
            retire_sent: false,
            retire_acked: false,
            pkt_ctx: zero_pkt_ctx,
        };
        stash.connection_ids.push(new_rcid);
        let stashed_index = stash.connection_ids.len() - 1;
        StashResult {
            status: 0,
            stashed_index: Some(stashed_index),
        }
    }
```

## `picoquic/quicctx.c:picoquic_create`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust mirrors much initialization, but visibly omits reset_seed randomization when no reset seed is supplied and ignores load_tickets errors instead of handling specific return codes.
* C source: `picoquic/quicctx.c:633-775`
* C signature: `picoquic_quic_t * picoquic_create(uint32_t, const char *, const char *, const char *, const char *, picoquic_stream_data_cb_fn, void *, picoquic_connection_id_cb_fn, void *, uint8_t[16], uint64_t, uint64_t *, const char *, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/lib.rs:1334-1535`
* Rust item: `new`

### C body
```c
{
    picoquic_quic_t* quic = (picoquic_quic_t*)malloc(sizeof(picoquic_quic_t));
    int ret = 0;

    if (quic != NULL) {
        /* TODO: winsock init */
        /* TODO: open UDP sockets - maybe */
        memset(quic, 0, sizeof(picoquic_quic_t));

        quic->default_callback_fn = default_callback_fn;
        quic->default_callback_ctx = default_callback_ctx;
        quic->default_congestion_alg = PICOQUIC_DEFAULT_CONGESTION_ALGORITHM;
        quic->default_alpn = picoquic_string_duplicate(default_alpn);
        quic->cnx_id_callback_fn = cnx_id_callback;
        quic->cnx_id_callback_ctx = cnx_id_callback_ctx;
        quic->p_simulated_time = p_simulated_time;
        quic->local_cnxid_length = 8; /* TODO: should be lower on clients-only implementation */
        quic->padding_multiple_default = 0; /* TODO: consider default = 128 */
        quic->padding_minsize_default = PICOQUIC_RESET_PACKET_MIN_SIZE;
        quic->crypto_epoch_length_max = 0;
        quic->max_simultaneous_logs = PICOQUIC_DEFAULT_SIMULTANEOUS_LOGS;
        quic->max_half_open_before_retry = PICOQUIC_DEFAULT_HALF_OPEN_RETRY_THRESHOLD;
        quic->default_lossbit_policy = 0; /* For compatibility with old behavior. Consider 0 */
        quic->local_cnxid_ttl = UINT64_MAX;
        quic->stateless_reset_next_time = current_time;
        quic->stateless_reset_min_interval = PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT;
        quic->default_stream_priority = PICOQUIC_DEFAULT_STREAM_PRIORITY;
        quic->default_datagram_priority = PICOQUIC_DEFAULT_STREAM_PRIORITY;
        quic->cwin_max = UINT64_MAX;
        quic->sequence_hole_pseudo_period = PICOQUIC_DEFAULT_HOLE_PERIOD;

        picoquic_init_transport_parameters(&quic->default_tp);

        quic->random_initial = 1;
        picoquic_wake_list_init(quic);

        if (cnx_id_callback != NULL) {
            quic->unconditional_cnx_id = 1;
        }
        if (ticket_file_name != NULL) {
            quic->ticket_file_name = ticket_file_name;
        }

        if (ret == 0) {
            size_t max_cnx4 = 0;
            if (max_nb_connections == 0) {
                max_nb_connections = 1;
            }

            quic->tentative_max_number_connections = max_nb_connections;
            quic->max_number_connections = max_nb_connections;
            max_cnx4 = 4 * (size_t)max_nb_connections;



            if (max_cnx4 < (size_t)max_nb_connections ||
                (quic->table_cnx_by_id = picohash_create_ex((size_t)max_nb_connections * 4,
                picoquic_local_cnxid_hash, picoquic_local_cnxid_compare, picoquic_local_cnxid_to_item, quic->hash_seed)) == NULL ||
                (quic->table_cnx_by_net = picohash_create_ex((size_t)max_nb_connections * 4,
                    picoquic_net_id_hash, picoquic_net_id_compare, picoquic_local_netid_to_item, quic->hash_seed)) == NULL ||
                (quic->table_cnx_by_icid = picohash_create_ex((size_t)max_nb_connections,
                    picoquic_net_icid_hash, picoquic_net_icid_compare, picoquic_net_icid_to_item, quic->hash_seed)) == NULL ||
                (quic->table_cnx_by_secret = picohash_create_ex((size_t)max_nb_connections * 4,
                    picoquic_net_secret_hash, picoquic_net_secret_compare, picoquic_net_secret_to_item, quic->hash_seed)) == NULL ||
                (quic->table_issued_tickets = picohash_create_ex((size_t)max_nb_connections,
                    picoquic_issued_ticket_hash, picoquic_issued_ticket_compare, picoquic_issued_ticket_key_to_item, quic->hash_seed)) == NULL) {
                ret = -1;
                DBG_PRINTF("%s", "Cannot initialize hash tables\n");
            }
            else {
                picosplay_init_tree(&quic->token_reuse_tree, picoquic_registered_token_compare,
                    picoquic_registered_token_create, picoquic_registered_token_delete, picoquic_registered_token_value);
                if (picoquic_master_tlscontext(quic, cert_file_name, key_file_name, cert_root_file_name, ticket_encryption_key, ticket_encryption_key_length) != 0) {
                    ret = -1;
                    DBG_PRINTF("%s", "Cannot create TLS context \n");
                }
                else {
                    /* In the absence of certificate or key, we assume that this is a client only context */
                    quic->enforce_client_only = (cert_file_name == NULL || key_file_name == NULL);
                    /* the random generator was initialized as part of the TLS context.
                     * Use it to create the seed for generating the per context stateless
                     * resets and the retry tokens */

                    if (!reset_seed)
                        picoquic_crypto_random(quic, quic->reset_seed, sizeof(quic->reset_seed));
                    else
                        memcpy(quic->reset_seed, reset_seed, sizeof(quic->reset_seed));

                    picoquic_crypto_random(quic, quic->retry_seed, sizeof(quic->retry_seed));
                    picoquic_crypto_random(quic, quic->hash_seed, sizeof(quic->hash_seed));

                    /* If there is no root certificate context specified, use a null certifier. */
                    /* Load tickets */
                    if (quic->ticket_file_name != NULL) {
                        ret = picoquic_load_tickets(quic, ticket_file_name);

                        if (ret == PICOQUIC_ERROR_NO_SUCH_FILE) {
                            DBG_PRINTF("Ticket file <%s> not created yet.\n", ticket_file_name);
                            ret = 0;
                        }
                        else if (ret != 0) {
                            DBG_PRINTF("Cannot load tickets from <%s>\n", ticket_file_name);
                            ret = 0;
                        }
                    }
                }
            }
        }
#ifdef BBRExperiment
        if (ret == 0) {
            quic->bbr_exp_flags.do_early_exit = 1;
            quic->bbr_exp_flags.do_rapid_start = 1;
            quic->bbr_exp_flags.do_handle_suspension = 1;
            quic->bbr_exp_flags.do_control_lost = 1;
            quic->bbr_exp_flags.do_exit_probeBW_up_on_delay = 1;
            quic->bbr_exp_flags.do_enter_probeBW_after_limited = 1;
        }
#endif

        if (ret != 0) {
            picoquic_free(quic);
            quic = NULL;
        }
    }

    return quic;
}
```

### Rust body
```rust
    ) -> Option<Box<Quic>> {
        // C: if max_nb_connections == 0, clamp to 1.
        if max_nb_connections == 0 {
            max_nb_connections = 1;
        }

        // C: enforce_client_only = (cert_file_name == NULL || key_file_name == NULL)
        let enforce_client_only = cert_file_name.is_none() || key_file_name.is_none();

        let unconditional_cnx_id = cnx_id_callback.is_some();

        // The C body allocates hash tables before it refreshes
        // quic->hash_seed, so the tables are created with the zeroed
        // initial seed and keep their own copy of it.
        let table_seed = [0u8; 16];
        let nb_bin = (max_nb_connections as usize).saturating_mul(4);
        let nb_bin_small = max_nb_connections as usize;
        let table_cnx_by_id = crate::hash::HashTable::with_seed(nb_bin, &table_seed).ok()?;
        let table_cnx_by_net = crate::hash::HashTable::with_seed(nb_bin, &table_seed).ok()?;
        let table_cnx_by_icid =
            crate::hash::HashTable::with_seed(nb_bin_small, &table_seed).ok()?;
        let table_cnx_by_secret = crate::hash::HashTable::with_seed(nb_bin, &table_seed).ok()?;
        let table_issued_tickets =
            crate::hash::HashTable::with_seed(nb_bin_small, &table_seed).ok()?;

        struct SystemRandom;
        impl rand_core::RngCore for SystemRandom {
            fn next_u32(&mut self) -> u32 {
                let mut bytes = [0u8; 4];
                self.fill_bytes(&mut bytes);
                u32::from_le_bytes(bytes)
            }
            fn next_u64(&mut self) -> u64 {
                let mut bytes = [0u8; 8];
                self.fill_bytes(&mut bytes);
                u64::from_le_bytes(bytes)
            }
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                use std::io::Read;

                std::fs::File::open("/dev/urandom")
                    .and_then(|mut file| file.read_exact(dest))
                    .expect("failed to read random bytes from /dev/urandom");
            }
        }
        impl rand_core::CryptoRng for SystemRandom {}

        let mut rng = SystemRandom;
        let mut retry_seed = [0u8; crate::internal::RETRY_SECRET_SIZE];
        rand_core::RngCore::fill_bytes(&mut rng, &mut retry_seed);
        let mut hash_seed = [0u8; 16];
        rand_core::RngCore::fill_bytes(&mut rng, &mut hash_seed);
        let mut default_tp = crate::tp::TransportParameters::default();
        crate::internal::init_transport_parameters(&mut default_tp);

        let mut quic = Box::new(internal::Quic {
            tls_client_config: None,
            tls_server_config: None,
            tls_callbacks: None,
            default_callback_fn: default_callback,
            default_callback_ctx: None,
            mask_ctx: None,
            mask_fns: None,
            default_alpn: default_alpn.map(|s| s.to_owned()),
            alpn_select_fn: None,
            reset_seed,
            retry_seed,
            rng: Box::new(rng),
            hash_seed,
            ticket_file_name: ticket_file_name.map(std::path::PathBuf::from),
            token_file_name: None,
            stored_tickets: Vec::new(),
            stored_tokens: Vec::new(),
            token_reuse_tree: crate::splay::SplayTree::default(),
            registered_tokens: crate::arena::Arena::new(),
            local_connection_id_length: 8,
            default_stream_priority: DEFAULT_STREAM_PRIORITY,
            default_datagram_priority: DEFAULT_STREAM_PRIORITY,
            local_connection_id_ttl: u64::MAX,
            mtu_max: 0,
            padding_multiple_default: 0,
            padding_minsize_default: RESET_PACKET_MIN_SIZE as u32,
            sequence_hole_pseudo_period: crate::internal::DEFAULT_HOLE_PERIOD as u32,
            default_pmtud_policy: PmtudPolicy::default(),
            default_spin_policy: SpinbitVersion::default(),
            default_lossbit_policy: LossbitVersion::default(),
            default_multipath_option: 0,
            default_handshake_timeout: crate::Duration::from_ticks(0),
            crypto_epoch_length_max: 0,
            max_simultaneous_logs: crate::internal::DEFAULT_SIMULTANEOUS_LOGS,
            current_number_of_open_logs: 0,
            max_half_open_before_retry: crate::internal::DEFAULT_HALF_OPEN_RETRY_THRESHOLD,
            current_number_half_open: 0,
            current_number_connections: 0,
            tentative_max_number_connections: max_nb_connections,
            max_number_connections: max_nb_connections,
            stateless_reset_next_time: current_time,
            stateless_reset_min_interval:
                crate::internal::MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT,
            cwin_max: u64::MAX,
            check_token: false,
            force_check_token: false,
            provide_token: false,
            unconditional_cnx_id,
            client_zero_share: false,
            server_busy: false,
            is_cert_store_not_empty: false,
            use_long_log: false,
            should_close_log: false,
            enable_sslkeylog: false,
            use_unique_log_names: false,
            dont_coalesce_init: false,
            one_way_grease_quic_bit: false,
            random_initial: 1,
            packet_train_mode: false,
            use_constant_challenges: false,
            use_low_memory: false,
            is_preemptive_repeat_enabled: false,
            default_send_receive_bdp_frame: false,
            enforce_client_only,
            test_large_server_flight: false,
            is_port_blocking_disabled: false,
            are_path_callbacks_enabled: false,
            use_predictable_random: false,
            client_authentication: false,
            use_exporter: false,
            ech_opener: None,
            ech_server_retry_config: None,
            ech_client_enabled: false,
            pending_stateless_packets: std::collections::VecDeque::new(),
            default_congestion_alg: Some(&NEWRENO_ALGORITHM),
            default_congestion_alg_option_string: None,
            connections: crate::arena::Arena::new(),
            connection_wake_tree: crate::splay::SplayTree::default(),
            connection_in_progress: None,
            connection_by_id: table_cnx_by_id,
            connection_by_net: table_cnx_by_net,
            connection_by_icid: table_cnx_by_icid,
            connection_by_secret: table_cnx_by_secret,
            issued_tickets_by_id: table_issued_tickets,
            issued_tickets: crate::arena::Arena::new(),
            nb_packets_allocated: 0,
            nb_packets_allocated_max: 0,
            nb_data_nodes_allocated: 0,
            nb_data_nodes_allocated_max: 0,
            connection_id_callback_fn: cnx_id_callback,
            connection_id_callback_ctx: None,
            aead_encrypt_ticket_ctx: None,
            aead_decrypt_ticket_ctx: None,
            retry_integrity_sign_ctx: Vec::new(),
            retry_integrity_verify_ctx: Vec::new(),
            default_tp,
            fuzz_fn: None,
            fuzz_ctx: None,
            wake_file: 0,
            wake_line: 0,
            max_data_limit: 0,
            rtt_update_delta: crate::Duration::from_ticks(0),
            pacing_rate_update_delta: 0,
            f_log: None,
            binlog_dir: None,
            qlog_dir: None,
            autoqlog_fn: None,
            text_log_fns: None,
            bin_log_fns: None,
            qlog_fns: None,
            perflog_fn: None,
            v_perflog_ctx: None,
            v_thread_ctx: None,
        });
        quic.wake_list_init();

        if quic
            .init_master_tls_context(
                cert_file_name,
                key_file_name,
                _cert_root_file_name,
                ticket_encryption_key,
            )
            .is_err()
        {
            return None;
        }

        if let Some(ticket_file_name) = ticket_file_name {
            let _ = quic.load_tickets(ticket_file_name);
        }

        Some(quic)
    }
```

## `picoquic/quicctx.c:picoquic_delete_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: C deletes the stream from stream_tree only; Rust removes it from the output queue and streams arena, with no visible stream_tree deletion.
* C source: `picoquic/quicctx.c:3698-3701`
* C signature: `void picoquic_delete_stream(picoquic_cnx_t *, picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/internal.rs:13023-13037`
* Rust item: `delete_stream`

### C body
```c
{
    picosplay_delete(&cnx->stream_tree, stream);
}
```

### Rust body
```rust
        {
            // Remove from output queue.
            if stream.is_output_stream {
                if let Some(pos) = self.output_streams.iter().position(|&t| t == stream_tok.1) {
                    self.output_streams.remove(pos);
                }
                stream.is_output_stream = false;
            }
            // Remove from the streams arena.
            self.streams.remove(stream_tok.1);
        }
```
