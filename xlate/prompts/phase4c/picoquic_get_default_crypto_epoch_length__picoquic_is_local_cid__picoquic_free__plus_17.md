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

## Pair `picoquic/quicctx.c:picoquic_get_default_crypto_epoch_length`
C: `picoquic/quicctx.c:1011-1015 picoquic_get_default_crypto_epoch_length`
Rust: `rs/fq/src/lib.rs:1797-1799 default_crypto_epoch_length`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->crypto_epoch_length_max;
}
```

### Rust body
```rust
    pub fn default_crypto_epoch_length(&self) -> u64 {
        self.crypto_epoch_length_max
    }
```

## Pair `picoquic/quicctx.c:picoquic_is_local_cid`
C: `picoquic/quicctx.c:1037-1042 picoquic_is_local_cid`
Rust: `rs/fq/src/lib.rs:1808-1816 is_local_cid`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return (cid->id_len == quic->local_cnxid_length &&
        picoquic_cnx_by_id(quic, *cid, NULL) != NULL);
}
```

### Rust body
```rust
    pub fn load_retry_tokens(&mut self, _token_store_filename: &str) -> Result<(), Error> {
        // Complex: involves file I/O and token deserialization.
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_free`
C: `picoquic/quicctx.c:1062-1173 picoquic_free`
Rust: `rs/fq/src/internal.rs:8647-8649 free`

### C body
```c
{
    if (quic != NULL) {
        PICOQUIC_THREAD_DISABLE_CHECK(quic);

        /* delete all the connection contexts -- do this before any other
         * action, as deleting connections may add packets to queues or
         * change connection lists */
        while (quic->cnx_list != NULL) {
            picoquic_delete_cnx(quic->cnx_list);
        }

        /* Delete ECH context if it was created */
        picoquic_release_quic_ech_ctx(quic);

        /* Delete TLS and AEAD cntexts */
        picoquic_delete_retry_protection_contexts(quic);

        if (quic->aead_encrypt_ticket_ctx != NULL) {
            picoquic_aead_free(quic->aead_encrypt_ticket_ctx);
            quic->aead_encrypt_ticket_ctx = NULL;
        }

        if (quic->aead_decrypt_ticket_ctx != NULL) {
            picoquic_aead_free(quic->aead_decrypt_ticket_ctx);
            quic->aead_decrypt_ticket_ctx = NULL;
        }

        if (quic->default_alpn != NULL) {
            free((void*)quic->default_alpn);
            quic->default_alpn = NULL;
        }

        /* delete the stored tickets */
        picoquic_free_tickets(&quic->p_first_ticket);

        /* Delete the stored tokens */
        picoquic_free_tokens(&quic->p_first_token);

        /* Deelete the reused tokens tree */
        picosplay_empty_tree(&quic->token_reuse_tree);

        /* delete packets in pool */
        while (quic->p_first_packet != NULL) {
            picoquic_packet_t * p = quic->p_first_packet->packet_previous;
            free(quic->p_first_packet);
            quic->p_first_packet = p;
            quic->nb_packets_allocated--;
            quic->nb_packets_in_pool--;
        }

        /* delete data nodes in pool */
        while (quic->p_first_data_node != NULL) {
            picoquic_stream_data_node_t* p = quic->p_first_data_node->next_stream_data;
            free(quic->p_first_data_node);
            quic->p_first_data_node = p;
            quic->nb_data_nodes_allocated--;
            quic->nb_data_nodes_in_pool--;
        }

        /* delete all pending stateless packets */
        while (quic->pending_stateless_packet != NULL) {
            picoquic_stateless_packet_t* to_delete = quic->pending_stateless_packet;
            quic->pending_stateless_packet = to_delete->next_packet;
            free(to_delete);
        }

        if (quic->table_cnx_by_id != NULL) {
            picohash_delete(quic->table_cnx_by_id, 0);
        }

        if (quic->table_cnx_by_net != NULL) {
            picohash_delete(quic->table_cnx_by_net, 0);
        }

        if (quic->table_cnx_by_icid != NULL) {
            picohash_delete(quic->table_cnx_by_icid, 0);
        }

        if (quic->table_issued_tickets != NULL) {
            picohash_delete(quic->table_issued_tickets, 1);
        }

        if (quic->table_cnx_by_secret != NULL) {
            picohash_delete(quic->table_cnx_by_secret, 0);
        }

        if (quic->verify_certificate_callback != NULL) {
            picoquic_dispose_verify_certificate_callback(quic);
        }

        /* Delete the picotls context */
        if (quic->tls_master_ctx != NULL) {
            picoquic_master_tlscontext_free(quic);

            free(quic->tls_master_ctx);
            quic->tls_master_ctx = NULL;
        }

        /* Close the logs */
        picoquic_log_close_logs(quic);

        quic->binlog_dir = picoquic_string_free(quic->binlog_dir);
        quic->qlog_dir = picoquic_string_free(quic->qlog_dir);

        if (quic->perflog_fn != NULL) {
            (void)(quic->perflog_fn)(quic, NULL, 1);
        }

        free(quic);
    }
}
```

### Rust body
```rust
    pub fn free(&mut self) {
        self.init();
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_max_half_open_retry_threshold`
C: `picoquic/quicctx.c:1206-1210 picoquic_set_max_half_open_retry_threshold`
Rust: `rs/fq/src/lib.rs:1179-1186 set_max_half_open_retry_threshold`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->max_half_open_before_retry = max_half_open_before_retry;
}
```

### Rust body
```rust
    pub fn max_half_open_retry_threshold(&self) -> u32 {
        self.max_half_open_before_retry
    }
```

## Pair `picoquic/quicctx.c:picoquic_dequeue_stateless_packet`
C: `picoquic/quicctx.c:1243-1255 picoquic_dequeue_stateless_packet`
Rust: `rs/fq/src/internal.rs:634-763 dequeue_stateless_packet`

### C body
```c
{
    picoquic_stateless_packet_t* sp = quic->pending_stateless_packet;

    if (sp != NULL) {
        quic->pending_stateless_packet = sp->next_packet;
        sp->next_packet = NULL;
        picoquic_log_quic_pdu(quic, 0, picoquic_get_quic_time(quic), sp->cnxid_log64,
            (struct sockaddr*) & sp->addr_to, (struct sockaddr*) & sp->addr_local, sp->length);
    }

    return sp;
}
```

### Rust body
```rust
    ) {
        if original_bytes.len() < 7 {
            return;
        }

        let dcid_length = original_bytes[5] as usize;
        let dcid_start = 6usize;
        let dcid_end = dcid_start.saturating_add(dcid_length);
        if dcid_end >= original_bytes.len() {
            return;
        }
        let scid_length = original_bytes[dcid_end] as usize;
        let scid_start = dcid_end + 1;
        let scid_end = scid_start.saturating_add(scid_length);
        if scid_end > original_bytes.len() {
            return;
        }

        let dcid = &original_bytes[dcid_start..dcid_end];
        let scid = &original_bytes[scid_start..scid_end];

        let mut cnx = None;
        if dcid_length <= crate::CONNECTION_ID_MAX_SIZE {
            ph.dest_connection_id = ConnectionId::clone_from_slice(dcid).unwrap_or_default();
            if ph.dest_connection_id.len() == self.local_connection_id_length as usize {
                if self.local_connection_id_length == 0 {
                    cnx = self.connection_by_net(Some(addr_from));
                } else if let Some((tok, local_cid)) = self.connection_by_id(ph.dest_connection_id)
                {
                    ph.local_connection_id = Some(local_cid);
                    cnx = Some(tok);
                }
            }
            if cnx.is_none() {
                cnx = self.connection_by_icid(&ph.dest_connection_id, Some(addr_from));
            }
        }

        if cnx.is_some() {
            return;
        }

        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };

        let mut byte_index = 0usize;
        if byte_index + 1 > sp.bytes.len() {
            return;
        }
        public_random(&mut sp.bytes[byte_index..byte_index + 1]);
        sp.bytes[byte_index] |= 0x80;
        byte_index += 1;

        if byte_index + 4 > sp.bytes.len() {
            return;
        }
        format_32(&mut sp.bytes[byte_index..byte_index + 4], 0);
        byte_index += 4;

        if scid_length > u8::MAX as usize
            || dcid_length > u8::MAX as usize
            || byte_index + 1 + scid_length + 1 + dcid_length > sp.bytes.len()
        {
            return;
        }
        sp.bytes[byte_index] = scid_length as u8;
        byte_index += 1;
        sp.bytes[byte_index..byte_index + scid_length].copy_from_slice(scid);
        byte_index += scid_length;
        sp.bytes[byte_index] = dcid_length as u8;
        byte_index += 1;
        sp.bytes[byte_index..byte_index + dcid_length].copy_from_slice(dcid);
        byte_index += dcid_length;

        for version in SUPPORTED_VERSIONS {
            if byte_index + 4 > sp.bytes.len() {
                return;
            }
            format_32(&mut sp.bytes[byte_index..byte_index + 4], version as u32);
            byte_index += 4;
        }

        let rand_vn = loop {
            let candidate = ((crate::public_random_64() as u32) & 0xF0F0_F0F0) | 0x0A0A_0A0A;
            if candidate != ph.version {
                break candidate;
            }
        };
        if byte_index + 4 > sp.bytes.len() {
            return;
        }
        format_32(&mut sp.bytes[byte_index..byte_index + 4], rand_vn);
        byte_index += 4;

        sp.length = byte_index;
        sp.addr_to = *addr_from;
        sp.addr_local = *addr_to;
        sp.if_index_local = if_index_to;
        sp.initial_connection_id = ph.dest_connection_id;
        sp.connection_id_log64 = sp.initial_connection_id.val64();
        sp.packet_type = PacketType::VersionNegotiation;

        self.log_pdu(
            true,
            Instant::from_ticks(self.time()),
            0,
            addr_to,
            addr_from,
            sp.length,
        );

        self.queue_stateless_packet(sp);
    }
```

## Pair `picoquic/quicctx.c:picoquic_register_net_id`
C: `picoquic/quicctx.c:1292-1310 picoquic_register_net_id`
Rust: `rs/fq/src/lib.rs:2037-2065 register_net_id`

### C body
```c
{
    int ret = 0;
    picohash_item* item;

    /* If registration was present, remove it */
    picoquic_unregister_net_id(cnx, path_x);
    /* Try registering the new address */
    picoquic_store_addr(&path_x->registered_peer_addr, (struct sockaddr *)&path_x->first_tuple->peer_addr);
    item = picohash_retrieve(quic->table_cnx_by_net, path_x);

    if (item != NULL) {
        ret = -1;
    } else {
        ret = picohash_insert(quic->table_cnx_by_net, path_x);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        self.unregister_net_id(connection, path_index);

        let peer_addr = self
            .connections
            .get(connection)
            .and_then(|cnx| cnx.paths.get(path_index))
            .and_then(|path| path.tuples.first())
            .map(|tuple| tuple.peer_addr)
            .ok_or(Error::InvalidArgument)?;
        if socket_addr_is_unspecified(&peer_addr) {
            return Ok(());
        }
        if self.connection_by_net.lookup(&peer_addr).is_some() {
            return Err(Error::Generic);
        }
        let (membership, _) = self.connection_by_net.insert(peer_addr, connection)?;
        if let Some(cnx) = self.connections.get_mut(connection)
            && let Some(path) = cnx.paths.get_mut(path_index)
        {
            path.registered_peer_addr = peer_addr;
            path.connection_by_net_membership = Some(membership);
        }
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_register_net_secret`
C: `picoquic/quicctx.c:1374-1393 picoquic_register_net_secret`
Rust: `rs/fq/src/internal.rs:3997-4026 register_net_secret`

### C body
```c
{
    int ret = 0;

    if (cnx->path[0]->first_tuple->peer_addr.ss_family != 0) {
        picohash_item* item;
        picoquic_unregister_net_secret(cnx);
        picoquic_store_addr(&cnx->registered_secret_addr, (struct sockaddr *)&cnx->path[0]->first_tuple->peer_addr);
        memcpy(&cnx->registered_reset_secret, cnx->path[0]->first_tuple->p_remote_cnxid->reset_secret, PICOQUIC_RESET_SECRET_SIZE);

        item = picohash_retrieve(cnx->quic->table_cnx_by_secret, cnx);
        if (item != NULL) {
            ret = -1;
        } 
        else {
            ret = picohash_insert(cnx->quic->table_cnx_by_secret, cnx);
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn register_net_secret(&mut self) -> Result<(), crate::Error> {
        if !crate::socket_addr_is_unspecified(&self.registered_secret_addr) {
            return Err(crate::Error::Generic);
        }
        let Some(path) = self.paths.first() else {
            return Err(crate::Error::InvalidArgument);
        };
        let Some(tuple) = path.tuples.first() else {
            return Err(crate::Error::InvalidArgument);
        };
        if crate::socket_addr_is_unspecified(&tuple.peer_addr) {
            return Ok(());
        }
        let unique_path_id = if self.is_multipath_enabled {
            path.unique_path_id
        } else {
            0
        };
        let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
        let reset_secret = self
            .remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == unique_path_id)
            .and_then(|stash| stash.connection_ids.get(cid_index))
            .map(|remote_cid| remote_cid.reset_secret)
            .unwrap_or([0u8; RESET_SECRET_SIZE]);
        self.registered_secret_addr = tuple.peer_addr;
        self.registered_reset_secret = reset_secret;
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_next_cnx`
C: `picoquic/quicctx.c:1430-1434 picoquic_get_next_cnx`
Rust: `rs/fq/src/lib.rs:3187-3190 next_cnx`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->next_in_table;
}
```

### Rust body
```rust
    pub fn next_cnx(&mut self, current_token: ConnectionToken) -> Option<&mut Connection> {
        let next_idx = current_token.slot_idx() + 1;
        self.connections.next_after_idx(next_idx)
    }
```

## Pair `picoquic/quicctx.c:picoquic_remove_cnx_from_wake_list`
C: `picoquic/quicctx.c:1505-1508 picoquic_remove_cnx_from_wake_list`
Rust: `rs/fq/src/lib.rs:3378-3386 remove_cnx_from_wake_list`

### C body
```c
{
    picosplay_delete_hint(&cnx->quic->cnx_wake_tree, &cnx->cnx_wake_node);
}
```

### Rust body
```rust
    pub(crate) fn remove_cnx_from_wake_list(&mut self, connection: ConnectionToken) {
        let old_membership = self
            .connections
            .get_mut(connection)
            .and_then(|cnx| cnx.connection_wake_membership.take());
        if let Some(old_membership) = old_membership {
            self.connection_wake_tree.remove(old_membership);
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_next_wake_time`
C: `picoquic/quicctx.c:1533-1551 picoquic_get_next_wake_time`
Rust: `rs/fq/src/lib.rs:3210-3217 next_wake_time`

### C body
```c
{
    uint64_t wake_time = UINT64_MAX;
    PICOQUIC_THREAD_CHECK(quic);

    if (quic->pending_stateless_packet != NULL) {
        wake_time = current_time;
    }
    else{
        picoquic_cnx_t* cnx_wake_first = (picoquic_cnx_t*)picoquic_wake_list_node_value(
            picosplay_first(&quic->cnx_wake_tree));

        if (cnx_wake_first != NULL) {
            wake_time = cnx_wake_first->next_wake_time;
        }
    }

    return wake_time;
}
```

### Rust body
```rust
    pub fn next_wake_time(&self, current_time: Instant) -> u64 {
        let now = current_time.ticks();
        self.connections
            .iter()
            .map(|c| c.next_wake_time.ticks())
            .min()
            .unwrap_or(now)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_version_index`
C: `picoquic/quicctx.c:1616-1628 picoquic_get_version_index`
Rust: `rs/fq/src/internal.rs:311-449 try_from_wire`

### C body
```c
{
    int ret = -1;

    for (size_t i = 0; i < picoquic_nb_supported_versions; i++) {
        if (picoquic_supported_versions[i].version == proposed_version) {
            ret = (int)i;
            break;
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn parameters(self) -> VersionParameters {
        const V1_SALT: &[u8] = &[
            0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8,
            0x0c, 0xad, 0xcc, 0xbb, 0x7f, 0x0a,
        ];
        const V1_RETRY_KEY: &[u8] = &[
            0xd9, 0xc9, 0x94, 0x3e, 0x61, 0x01, 0xfd, 0x20, 0x00, 0x21, 0x50, 0x6b, 0xcc, 0x02,
            0x81, 0x4c, 0x73, 0x03, 0x0f, 0x25, 0xc7, 0x9d, 0x71, 0xce, 0x87, 0x6e, 0xca, 0x87,
            0x6e, 0x6f, 0xca, 0x8e,
        ];
        const V2_SALT: &[u8] = &[
            0x0d, 0xed, 0xe3, 0xde, 0xf7, 0x00, 0xa6, 0xdb, 0x81, 0x93, 0x81, 0xbe, 0x6e, 0x26,
            0x9d, 0xcb, 0xf9, 0xbd, 0x2e, 0xd9,
        ];
        const V2_RETRY_KEY: &[u8] = &[
            0xc4, 0xdd, 0x24, 0x84, 0xd6, 0x81, 0xae, 0xfa, 0x4f, 0xf4, 0xd6, 0x9c, 0x2c, 0x20,
            0x29, 0x99, 0x84, 0xa7, 0x65, 0xa5, 0xd3, 0xc3, 0x19, 0x82, 0xf3, 0x8f, 0xc7, 0x41,
            0x62, 0x15, 0x5e, 0x9f,
        ];
        const V2_DRAFT_SALT: &[u8] = &[
            0xa7, 0x07, 0xc2, 0x03, 0xa5, 0x9b, 0x47, 0x18, 0x4a, 0x1d, 0x62, 0xca, 0x57, 0x04,
            0x06, 0xea, 0x7a, 0xe3, 0xe5, 0xd3,
        ];
        const V2_DRAFT_RETRY_KEY: &[u8] = &[
            0x34, 0x25, 0xc2, 0x0c, 0xf8, 0x87, 0x79, 0xdf, 0x2f, 0xf7, 0x1e, 0x8a, 0xbf, 0xa7,
            0x82, 0x49, 0x89, 0x1e, 0x76, 0x3b, 0xbe, 0xd2, 0xf1, 0x3c, 0x04, 0x83, 0x43, 0xd3,
            0x48, 0xc0, 0x60, 0xe2,
        ];
        const DRAFT_29_SALT: &[u8] = &[
            0xaf, 0xbf, 0xec, 0x28, 0x99, 0x93, 0xd2, 0x4c, 0x9e, 0x97, 0x86, 0xf1, 0x9c, 0x61,
            0x11, 0xe0, 0x43, 0x90, 0xa8, 0x99,
        ];
        const RETRY_KEY_29: &[u8] = &[
            0x8b, 0x0d, 0x37, 0xeb, 0x85, 0x35, 0x02, 0x2e, 0xbc, 0x8d, 0x76, 0xa2, 0x07, 0xd8,
            0x0d, 0xf2, 0x26, 0x46, 0xec, 0x06, 0xdc, 0x80, 0x96, 0x42, 0xc3, 0x0a, 0x8b, 0xaa,
            0x2b, 0xaa, 0xff, 0x4c,
        ];
        const INTERNAL_TEST_1_SALT: &[u8] = &[
            0x30, 0x67, 0x16, 0xd7, 0x63, 0x75, 0xd5, 0x55, 0x4b, 0x2f, 0x60, 0x5e, 0xef, 0x78,
            0xd8, 0x33, 0x3d, 0xc1, 0xca, 0x36,
        ];

        const V1_PREFIX: &str = "tls13 quic ";
        const V2_PREFIX: &str = "tls13 quicv2 ";
        const V1_KU: &str = "quic ku";
        const V2_KU: &str = "quicv2 ku";

        const UPGRADE_FROM_V1: &[Version] = &[Version::V1];

        match self {
            Version::V1 => VersionParameters {
                version: self,
                version_aead_key: V1_SALT,
                version_retry_key: V1_RETRY_KEY,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
            Version::V2 => VersionParameters {
                version: self,
                version_aead_key: V2_SALT,
                version_retry_key: V2_RETRY_KEY,
                tls_prefix_label: V2_PREFIX,
                tls_traffic_update_label: V2_KU,
                packet_type_version: 0x6b33_43cf,
                upgrade_from: UPGRADE_FROM_V1,
            },
            Version::V2Draft => VersionParameters {
                version: self,
                version_aead_key: V2_DRAFT_SALT,
                version_retry_key: V2_DRAFT_RETRY_KEY,
                tls_prefix_label: V2_PREFIX,
                tls_traffic_update_label: V2_KU,
                packet_type_version: 0x6b33_43cf,
                upgrade_from: UPGRADE_FROM_V1,
            },
            Version::PostIesg | Version::TwentyFirstInterop => VersionParameters {
                version: self,
                version_aead_key: V1_SALT,
                version_retry_key: V1_RETRY_KEY,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
            Version::TwentiethInterop
            | Version::TwentiethPreInterop
            | Version::NineteenthBisInterop
            | Version::NineteenthInterop => VersionParameters {
                version: self,
                version_aead_key: DRAFT_29_SALT,
                version_retry_key: RETRY_KEY_29,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
            Version::InternalTest1 | Version::InternalTest2 => VersionParameters {
                version: self,
                version_aead_key: INTERNAL_TEST_1_SALT,
                version_retry_key: V1_RETRY_KEY,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: self as u32,
                upgrade_from: &[],
            },
            Version::SeventeenthInterop | Version::EighteenthInterop => VersionParameters {
                version: self,
                version_aead_key: DRAFT_29_SALT,
                version_retry_key: RETRY_KEY_29,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_sequence_number`
C: `picoquic/quicctx.c:1686-1692 picoquic_get_sequence_number`
Rust: `rs/fq/src/internal.rs:7836-7846 get_sequence_number`

### C body
```c
{
    return (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) ? path_x->pkt_ctx.send_sequence:
        cnx->pkt_ctx[pc].send_sequence;
}
```

### Rust body
```rust
    pub fn get_ack_number(&self, _path_x: &mut Path, pc: PacketContext) -> u64 {
        // C: picoquic_get_ack_number
        self.ack_ctx[pc as usize].sack_list.first()
    }
```

## Pair `picoquic/quicctx.c:picoquic_unchain_tuple`
C: `picoquic/quicctx.c:1733-1751 picoquic_unchain_tuple`
Rust: `rs/fq/src/lib.rs:7523-7526 unchain_tuple`

### C body
```c
{
    picoquic_tuple_t* next = path_x->first_tuple;


    if (next == tuple) {
        path_x->first_tuple = next->next_tuple;
    }
    else {
        while (next->next_tuple != NULL) {
            picoquic_tuple_t* previous = next;
            next = next->next_tuple;
            if (next == tuple) {
                previous->next_tuple = next->next_tuple;
                break;
            }
        }
    }
}
```

### Rust body
```rust
        if index < self.tuples.len() {
            self.tuples.remove(index);
        }
```

## Pair `picoquic/quicctx.c:picoquic_create_path`
C: `picoquic/quicctx.c:1786-1866 picoquic_create_path`
Rust: `rs/fq/src/internal.rs:4145-4332 new`

### C body
```c
{
    int ret = -1;

    if (cnx->nb_paths >= cnx->nb_path_alloc)
    {
        int new_alloc = (cnx->nb_path_alloc == 0) ? 1 : 2 * cnx->nb_path_alloc;
        picoquic_path_t ** new_path = (picoquic_path_t **)malloc(new_alloc * sizeof(picoquic_path_t *));

        if (new_path != NULL)
        {
            if (cnx->path != NULL)
            {
                memset (new_path, 0, new_alloc * sizeof(picoquic_path_t*));
                if (cnx->nb_paths > 0)
                {
                    memcpy(new_path, cnx->path, cnx->nb_paths * sizeof(picoquic_path_t *));
                }
                free(cnx->path);
            }
            cnx->path = new_path;
            cnx->nb_path_alloc = new_alloc;
        }
    }

    if (cnx->nb_paths < cnx->nb_path_alloc)
    {
        uint64_t unique_path_id = picoquic_find_avalaible_unique_path_id(cnx, requested_id);
        picoquic_path_t* path_x = (unique_path_id == UINT64_MAX) ? NULL :
            (picoquic_path_t*)malloc(sizeof(picoquic_path_t));

        if (path_x != NULL)
        {
            memset(path_x, 0, sizeof(picoquic_path_t));
            /* Register the sequence number */
            path_x->unique_path_id = unique_path_id;
            path_x->cnx = cnx;
            picoquic_tuple_t* tuple = picoquic_create_tuple(path_x, local_addr, peer_addr, if_index);

            if (tuple != NULL) {
                /* Initialize per path time measurement */
                path_x->smoothed_rtt = PICOQUIC_INITIAL_RTT;
                path_x->rtt_variant = 0;
                path_x->retransmit_timer = PICOQUIC_INITIAL_RETRANSMIT_TIMER;
                path_x->rtt_min = 0;

                /* Initialize per path congestion control state */
                path_x->cwin = PICOQUIC_CWIN_INITIAL;
                path_x->bytes_in_transit = 0;
                path_x->congestion_alg_state = NULL;

                /* Initialize per path pacing state */
                picoquic_pacing_init(&path_x->pacing, start_time);

                /* Initialize the MTU */
                path_x->send_mtu = (peer_addr == NULL || peer_addr->sa_family == AF_INET) ? PICOQUIC_INITIAL_MTU_IPV4 : PICOQUIC_INITIAL_MTU_IPV6;

                /* initialize the quality reporting thresholds */
                path_x->rtt_update_delta = cnx->rtt_update_delta;
                path_x->pacing_rate_update_delta = cnx->pacing_rate_update_delta;
                picoquic_refresh_path_quality_thresholds(path_x);

                /* In case of unique path_id multipath, initialize the context. We do that systematically,
                 * because path 0 is created before multipath options are negotiated.
                 */
                picoquic_init_ack_ctx(cnx, &path_x->ack_ctx);
                picoquic_init_packet_ctx(cnx, &path_x->pkt_ctx, picoquic_packet_context_application);
                /* Record the path */
                cnx->path[cnx->nb_paths] = path_x;
                ret = cnx->nb_paths++;

                /* Set the challenge used for this path */
                picoquic_set_path_challenge(cnx, cnx->nb_paths - 1, start_time);
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<Self, crate::Error> {
        use core::net::{IpAddr, Ipv4Addr};
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let zero_instant = crate::Instant::from_ticks(0);
        let zero_dur = crate::Duration::from_ticks(0);
        let mut path = Self {
            registered_peer_addr: peer_addr.copied().unwrap_or(default_addr),
            connection_by_net_membership: None,
            unique_path_id,
            app_path_ctx: None,
            ack_ctx: AckContext {
                sack_list: SackList::new(),
                time_stamp_largest_received: zero_instant,
                act: [
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: zero_instant,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: zero_instant,
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
                highest_acknowledged: 0,
                latest_time_acknowledged: zero_instant,
                highest_acknowledged_time: zero_instant,
                pending: BTreeMap::new(),
                retransmitted: BTreeMap::new(),
                preemptive_repeat_seq: None,
                retransmitted_queue_size: 0,
                ecn_ect0_total_remote: 0,
                ecn_ect1_total_remote: 0,
                ecn_ce_total_remote: 0,
                ack_of_ack_requested: false,
            },
            tuples: Vec::new(),
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
            max_ack_delay: crate::internal::ACK_DELAY_MAX_DEFAULT,
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
            rtt_update_delta: connection.rtt_update_delta,
            pacing_rate_update_delta: connection.pacing_rate_update_delta,
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
        // Create the initial tuple.
        path.create_tuple(local_addr, peer_addr, if_index)?;
        let _ = start_time;
        Ok(path)
    }
```

## Pair `picoquic/quicctx.c:picoquic_delete_abandoned_paths`
C: `picoquic/quicctx.c:1967-2050 picoquic_delete_abandoned_paths`
Rust: `rs/fq/src/internal.rs:4756-4760 delete_abandoned_paths`

### C body
```c
{
    int path_index_good = 1;
    int path_index_current = 1;
    unsigned int is_demotion_in_progress = 0;

    if (cnx->is_multipath_enabled && cnx->nb_paths > 1) {
        path_index_good = 0;
        path_index_current = 0;
    }

    while (path_index_current < cnx->nb_paths) {
        /* Demote the path if marked for demotion */
        if (!cnx->path[path_index_current]->path_is_demoted){
            if (cnx->path[path_index_current]->first_tuple->challenge_failed ||
                (path_index_current > 0 && cnx->path[path_index_current]->first_tuple->challenge_verified &&
                    current_time - cnx->path[path_index_current]->latest_sent_time >= cnx->idle_timeout)) {
                picoquic_demote_path(cnx, path_index_current, current_time, 0);
            }
        }
        if (cnx->path[path_index_current]->path_is_demoted &&
            current_time >= cnx->path[path_index_current]->demotion_time) {
            /* Waited enough,should now delete this path. */
            path_index_current++;
            is_demotion_in_progress |= 1;
        } else {
            /* Need to keep this path a bit longer */
            /* First set the wake up timer so we don't miss the coming demotion */
            if (cnx->path[path_index_current]->path_is_demoted &&
                current_time < cnx->path[path_index_current]->demotion_time){
                is_demotion_in_progress |= 1;
                if (*next_wake_time > cnx->path[path_index_current]->demotion_time) {
                    *next_wake_time = cnx->path[path_index_current]->demotion_time;
                    SET_LAST_WAKE(cnx->quic, PICOQUIC_QUICCTX);
                }
            }
            /* Then pack the list of paths */
            if (path_index_current > path_index_good) {
                /* swap the path indexed good with current */
                picoquic_path_t * path_x = cnx->path[path_index_current];
                cnx->path[path_index_current] = cnx->path[path_index_good];
                cnx->path[path_index_good] = path_x;
            }
            /* increment both indices */
            path_index_current++;
            path_index_good++;
        }
    }

    if (cnx->nb_paths > path_index_good) {
        do {
            int d_path = cnx->nb_paths - 1;
            picoquic_dereference_stashed_cnxid(cnx, cnx->path[d_path], 0);
            picoquic_delete_path(cnx, d_path);
        }  while (cnx->nb_paths > path_index_good);
        /* If paths have been deleted, it may become possible to create new ones. */
        picoquic_test_and_signal_new_path_allowed(cnx);
    }

    /* TODO: what if there are no paths left? */
    cnx->path_demotion_needed = is_demotion_in_progress;
    int path_left = -1;
    int path_backup = -1;
    if (is_demotion_in_progress && cnx->is_multipath_enabled) {
        /* Verify that if one path is demoted, the other
         * becomes available */
        for (int i = 0; i < cnx->nb_paths; i++) {
            if (cnx->path[i]->path_is_demoted) {
                continue;
            }
            if (cnx->path[i]->path_is_backup && path_backup < 0) {
                path_backup = i;
            }
            else {
                path_left = i;
                break;
            }
        }
        if (path_left < 0 && path_backup >= 0) {
            cnx->path[path_backup]->path_is_backup = 0;
            (void)picoquic_queue_path_available_or_backup_frame(cnx, cnx->path[path_backup], picoquic_path_status_available);
        }
    }
}
```

### Rust body
```rust
    pub fn delete_abandoned_paths(&mut self, current_time: Instant, next_wake_time: &mut Instant) {
        self.delete_demoted_tuples(current_time, next_wake_time);
        self.paths
            .retain(|path| !(path.path_abandon_received && path.tuples.is_empty()));
    }
```

## Pair `picoquic/quicctx.c:picoquic_find_path_by_address`
C: `picoquic/quicctx.c:2153-2201 picoquic_find_path_by_address`
Rust: `rs/fq/src/internal.rs:4796-4805 find_path_by_address`

### C body
```c
{
    int path_id = -1;
    int is_null_from = 0;
    struct sockaddr_storage null_addr;

    *partial_match = -1;

    if (addr_peer != NULL || addr_local != NULL) {
        if (addr_peer == NULL || addr_local == NULL) {
            memset(&null_addr, 0, sizeof(struct sockaddr_storage));
            if (addr_peer == NULL) {
                addr_peer = (struct sockaddr*) & null_addr;
            }
            else {
                addr_local = (struct sockaddr*) & null_addr;
            }
            is_null_from = 1;
        }
        else if (addr_local->sa_family == 0) {
            is_null_from = 1;
        }

        /* Find whether an existing path matches the  pair of addresses */
        for (int i = 0; i < cnx->nb_paths; i++) {
            if (picoquic_compare_addr((struct sockaddr*) & cnx->path[i]->first_tuple->peer_addr,
                addr_peer) == 0) {
                if (cnx->path[i]->first_tuple->local_addr.ss_family == 0) {
                    *partial_match = i;
                }
                else if (picoquic_compare_addr((struct sockaddr*) & cnx->path[i]->first_tuple->local_addr,
                    addr_local) == 0) {
                    path_id = i;
                    break;
                }
            }

            if (path_id < 0 && is_null_from) {
                path_id = *partial_match;
                *partial_match = -1;
            }
        }
    }

    return path_id;
}
```

### Rust body
```rust
        if addr_peer.is_none() && addr_local.is_none() {
            return -1;
        }
```

## Pair `picoquic/quicctx.c:picoquic_assign_peer_cnxid_to_tuple`
C: `picoquic/quicctx.c:2264-2282 picoquic_assign_peer_cnxid_to_tuple`
Rust: `rs/fq/src/internal.rs:4866-4883 assign_peer_connection_id_to_tuple`

### C body
```c
{
    int ret = -1;
    picoquic_remote_cnxid_stash_t* stash = picoquic_find_or_create_remote_cnxid_stash(cnx, path_x->unique_path_id, 0);

    if (stash != NULL) {
        picoquic_remote_cnxid_t* available_cnxid = picoquic_get_cnxid_from_stash(stash);

        if (available_cnxid != NULL) {
            tuple->p_remote_cnxid = available_cnxid;
            available_cnxid->nb_path_references++;
            stash->is_in_use = 1;
            ret = 0;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let (stash_idx, cid_idx) = self
            .obtain_stashed_connection_id(path_x.unique_path_id)
            .ok_or(crate::Error::Generic)?;
        tuple.remote_connection_id_index = Some(cid_idx);
        tuple.unique_path_id = path_x.unique_path_id;
        if let Some(cid) = self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .get_mut(cid_idx)
        {
            cid.nb_path_references += 1;
        }
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_test_and_signal_new_path_allowed`
C: `picoquic/quicctx.c:2370-2383 picoquic_test_and_signal_new_path_allowed`
Rust: `rs/fq/src/internal.rs:14933-14939 test_and_signal_new_path_allowed`

### C body
```c
{
    if (cnx->is_subscribed_to_path_allowed &&
        !cnx->is_notified_that_path_is_allowed)
    {
        if (picoquic_check_new_path_allowed(cnx, 0) == 0) {
            cnx->is_notified_that_path_is_allowed = 1;
            if (cnx->callback_fn != NULL) {
                (void)cnx->callback_fn(cnx, 0, NULL, 0, picoquic_callback_next_path_allowed, cnx->callback_ctx, NULL);
            }
        }
    }
}
```

### Rust body
```rust
        {
            self.is_notified_that_path_is_allowed = true;
        }
```

## Pair `picoquic/quicctx.c:picoquic_enable_path_callbacks`
C: `picoquic/quicctx.c:2513-2517 picoquic_enable_path_callbacks`
Rust: `rs/fq/src/lib.rs:2656-2669 enable_path_callbacks`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->are_path_callbacks_enabled = are_enabled;
}
```

### Rust body
```rust
    pub fn set_first_if_index(&mut self, if_index: u32) -> Result<(), Error> {
        if let Some(path) = self.paths.first_mut()
            && let Some(tuple) = path.tuples.first_mut()
        {
            tuple.if_index = if_index as core::ffi::c_ulong;
            return Ok(());
        }
        Err(Error::InvalidArgument)
    }
```

## Pair `picoquic/quicctx.c:picoquic_probe_new_path`
C: `picoquic/quicctx.c:2552-2556 picoquic_probe_new_path`
Rust: `rs/fq/src/lib.rs:2436-2444 probe_new_path`

### C body
```c
{
    return picoquic_probe_new_path_ex(cnx, addr_peer, addr_local, 0, current_time, 0);
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves path creation and network probing.
        Err(Error::Generic)
    }
```
