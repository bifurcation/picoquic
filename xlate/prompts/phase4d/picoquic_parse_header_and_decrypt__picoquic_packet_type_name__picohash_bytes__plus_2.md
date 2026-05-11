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

## `picoquic/packet.c:picoquic_parse_header_and_decrypt`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C handles cleartext packet copying, missing-connection initial screening, retry/version-negotiation/error cases, stateless reset checks, duplicate packets, and wrong-version handling; Rust only proceeds when a connection token exists and decrypts through one path.
* C source: `picoquic/packet.c:770-902`
* C signature: `int picoquic_parse_header_and_decrypt(picoquic_quic_t *, const uint8_t *, size_t, size_t, const struct sockaddr *, uint64_t, picoquic_stream_data_node_t *, picoquic_packet_header *, picoquic_cnx_t **, size_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:7762-7829`
* Rust item: `parse_header_and_decrypt`

### C body
```c
{
    /* Parse the clear text header. Ret == 0 means an incorrect packet that could not be parsed */
    int already_received = 0;
    size_t decoded_length = 0;
    int ret = picoquic_parse_packet_header(quic, bytes, length, addr_from, ph, pcnx, 1);

    *new_ctx_created = 0;

    if (ret == 0) {
        if (ph->ptype != picoquic_packet_version_negotiation &&
            ph->ptype != picoquic_packet_retry && ph->ptype != picoquic_packet_error) {
            length = ph->offset + ph->payload_length;
            *consumed = length;

            if (*pcnx != NULL) {
                if (!(*pcnx)->client_mode && ph->ptype == picoquic_packet_initial && packet_length < PICOQUIC_ENFORCED_INITIAL_MTU) {
                    /* Unexpected packet. Reject, drop and log. */
                    ret = PICOQUIC_ERROR_INITIAL_TOO_SHORT;
                }
                /* Test whether we need to do a version upgrade */
                else if (ph->version_index != (*pcnx)->version_index) {
                    if ((*pcnx)->client_mode &&
                        (*pcnx)->cnx_state < picoquic_state_client_almost_ready &&
                        ph->version_index >= 0 &&
                        picoquic_supported_versions[ph->version_index].version == (*pcnx)->desired_version) {
                        /* The server already accepted the version upgrade */
                        ret = picoquic_process_version_upgrade(*pcnx, (*pcnx)->version_index, ph->version_index);
                    }
                    else {
                        ret = PICOQUIC_ERROR_PACKET_WRONG_VERSION;
                    }
                }
                else {
                    /* Remove header protection at this point -- values of bytes will not change */
                    ret = picoquic_remove_header_protection(*pcnx, (uint8_t*)bytes, decrypted_data->data, ph);

                    if (ret == 0) {
                        decoded_length = picoquic_remove_packet_protection(*pcnx, (uint8_t*)bytes,
                            decrypted_data->data, ph, current_time, &already_received);
                    }
                    else {
                        decoded_length = ph->payload_length + 1;
                    }

                    if (decoded_length > (length - ph->offset)) {
                        if (ph->ptype == picoquic_packet_1rtt_protected &&
                            length >= PICOQUIC_RESET_PACKET_MIN_SIZE &&
                            memcmp(bytes + length - PICOQUIC_RESET_SECRET_SIZE,
                                (*pcnx)->path[0]->first_tuple->p_remote_cnxid->reset_secret, PICOQUIC_RESET_SECRET_SIZE) == 0) {
                            ret = PICOQUIC_ERROR_STATELESS_RESET;
                            picoquic_log_app_message(*pcnx, "Decrypt error, matching reset secret, ret = %d", ret);
                        }
                        else {
                            if (ret != PICOQUIC_ERROR_AEAD_NOT_READY) {
                                ret = PICOQUIC_ERROR_AEAD_CHECK;
                            }
                            if (*new_ctx_created) {
                                picoquic_delete_cnx(*pcnx);
                                *pcnx = NULL;
                                *new_ctx_created = 0;
                            }
                        }
                    }
                    else if (already_received != 0) {
                        ret = PICOQUIC_ERROR_DUPLICATE;
                    }
                    else {
                        ph->payload_length = (uint16_t)decoded_length;
                    }
                }
            }
            else {
                if (ph->ptype != picoquic_packet_version_negotiation &&
                    ph->ptype != picoquic_packet_retry && ph->ptype != picoquic_packet_error) {
                    /* Redirect if proxy available -- function returns 0 if the packet was *not* intercepted */
                    if (quic->picomask_fns != NULL) {
                        ret = (quic->picomask_fns->picomask_redirect_fn)(quic->picomask_ctx,
                            bytes, packet_length, addr_from, consumed);
                    }
                    if (ret == 0) {
                        /* If packet was not redirected, it might be an initial packet
                         * for a new connection or a stateless redirect. Any other type
                         * should be treated as an error.
                         */
                        if (ph->ptype == picoquic_packet_initial) {
                            /* Screening the packet for protection against DOS. If successful, this
                             * will decrypt the initial packet and create a new connection context.
                             */
                            ret = picoquic_screen_initial_packet(quic, bytes, packet_length, addr_from, ph, current_time, pcnx,
                                new_ctx_created, decrypted_data);
                        }
                        else if (ph->ptype == picoquic_packet_1rtt_protected)
                        {
                            /* This may be a stateless reset.
                             * We test the address + putative reset secret pair against the hash table
                             * of registered secrets. If there is a match, the corresponding connection is
                             * found and the packet is marked as Stateless Reset */
                            if (length >= PICOQUIC_RESET_PACKET_MIN_SIZE) {
                                *pcnx = picoquic_cnx_by_secret(quic, bytes + length - PICOQUIC_RESET_SECRET_SIZE, addr_from);
                                if (*pcnx != NULL) {
                                    ret = PICOQUIC_ERROR_STATELESS_RESET;
                                    picoquic_log_app_message(*pcnx, "Found connection from reset secret, ret = %d", ret);
                                }
                            }
                        }
                        else {
                            ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
                        }
                    }
                }
            }
        }
        else {
            /* Clear text packet. Copy content to decrypted data */
            memmove(decrypted_data->data, bytes, length);
            *consumed = length;
        }
    }
    
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(Option<ConnectionToken>, bool), crate::Error> {
        let packet_length = packet_length.min(bytes.len());
        let conn_tok = self.parse_packet_header(&bytes[..packet_length], addr_from, ph, true)?;
        let Some(conn_tok) = conn_tok else {
            *consumed = ph.offset.min(packet_length);
            return Ok((None, false));
        };
        let mut packet = bytes[..packet_length].to_vec();
        let cnx = self
            .connections
            .get(conn_tok)
            .ok_or(crate::Error::Generic)?;
        let epoch = ph.epoch as usize;
        let pn_dec = cnx.crypto_context[epoch]
            .pn_dec
            .as_deref()
            .ok_or(crate::Error::Tls)?;
        let aead = cnx.crypto_context[epoch]
            .aead_decrypt
            .as_deref()
            .ok_or(crate::Error::Tls)?;
        let mut header_copy = [0u8; MAX_PACKET_SIZE];
        let is_loss_bit = cnx.is_loss_bit_enabled_incoming;
        let largest = cnx.ack_ctx[ph.packet_context as usize].sack_list.first();
        if remove_header_protection_inner(
            &mut packet,
            packet_length,
            &mut header_copy,
            ph,
            pn_dec,
            is_loss_bit,
            largest,
        ) != 0
        {
            return Err(crate::Error::Tls);
        }
        let pn_length = ((packet[0] & 0x03) + 1) as usize;
        let header_end = ph.packet_number_offset.saturating_add(pn_length);
        let cipher_end = if ph.payload_length > 0 {
            ph.packet_number_offset
                .saturating_add(ph.payload_length)
                .min(packet_length)
        } else {
            packet_length
        };
        if header_end > cipher_end || cipher_end > packet.len() {
            return Err(crate::Error::InvalidArgument);
        }
        let header = packet[..header_end].to_vec();
        let mut payload = packet[header_end..cipher_end].to_vec();
        aead.decrypt(ph.packet_number_full, &header, &mut payload)?;
        let len = payload.len().min(decrypted_data.data.len());
        decrypted_data.stream_data_membership = None;
        decrypted_data.offset = 0;
        decrypted_data.length = len;
        decrypted_data.data[..len].copy_from_slice(&payload[..len]);
        *consumed = cipher_end;
        Ok((Some(conn_tok), false))
    }
```

## `picoquic/packet_names.c:picoquic_packet_type_name`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body maps packet types to string names; Rust body parses a binlog and writes SVG packet output, with no packet-type-name mapping.
* C source: `picoquic/packet_names.c:25-46`
* C signature: `const char * picoquic_packet_type_name(uint64_t)`
* Rust source: `rs/fq/src/tests/picolog.rs:175-243`
* Rust item: `packet_type_name`

### C body
```c
{
    switch (ptype) {
    case picoquic_packet_error:
        return "error";
    case picoquic_packet_version_negotiation:
        return "version_negotiation";
    case picoquic_packet_initial:
        return "initial";
    case picoquic_packet_retry:
        return "retry";
    case picoquic_packet_handshake:
        return "handshake";
    case picoquic_packet_0rtt_protected:
        return "0RTT";
    case picoquic_packet_1rtt_protected:
        return "1RTT";
    case picoquic_packet_type_max:
    default:
        return "unknown";
    }
}
```

### Rust body
```rust
) -> crate::Result<()> {
    let mut packet_count = 0usize;
    fileread_binlog(f_binlog, |s| {
        let cid = s.read_cid()?;
        if &cid != cid_filter {
            return Ok(());
        }

        let time = s.read_varint()?;
        let path_id = s.read_varint()?;
        let event_id = s.read_varint()?;

        match event_id {
            0x0010 => {
                let _client_mode = s.read_u8()?;
                let _proposed_version = s.read_u32()?;
                let _remote_cnxid = s.read_cid()?;
            }
            0x0008 | 0x0009 => {
                let received = event_id == 0x0009;
                let packet_length = s.read_varint()?;
                let ph = read_packet_header(s)?;
                svg_packet_start(
                    out,
                    &mut packet_count,
                    time,
                    path_id,
                    packet_length,
                    &ph,
                    received,
                )
                .map_err(|_| crate::Error::Generic)?;

                while s.remaining() > 0 {
                    let len = s.read_vlen()?;
                    if s.remaining() < len {
                        return Err(crate::Error::BufferTooSmall);
                    }
                    let mut frame_bytes = s.tail()[..len].to_vec();
                    s.skip(len)?;
                    let mut frame = ByteStream::from_slice(&mut frame_bytes);
                    svg_packet_frame(out, &mut frame).map_err(|_| crate::Error::Generic)?;
                }

                writeln!(out, "</text>").map_err(|_| crate::Error::Generic)?;
            }
            _ => {}
        }

        Ok(())
    })
}
```

## `picoquic/picohash.c:picohash_bytes`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body implements the hash algorithm; Rust body is a test function that checks expected hash values and does not implement the algorithm.
* C source: `picoquic/picohash.c:180-202`
* C signature: `uint64_t picohash_bytes(const uint8_t *, size_t, const uint8_t *)`
* Rust source: `rs/fq/src/tests/hashtest.rs:140-167`
* Rust item: `picohash_bytes`

### C body
```c
{
    uint64_t hash =
        ((uint64_t)hash_seed[8]) +
        (((uint64_t)hash_seed[9]) << 8) +
        (((uint64_t)hash_seed[10]) << 16) +
        (((uint64_t)hash_seed[11]) << 24) +
        (((uint64_t)hash_seed[12]) << 32) +
        (((uint64_t)hash_seed[13]) << 40) +
        (((uint64_t)hash_seed[14]) << 48) +
        (((uint64_t)hash_seed[15]) << 56);
    int rotate = 11;

    for (uint32_t i = 0; i < length; i++) {
        hash ^= bytes[i];
        hash ^= hash_seed[i & 15];
        hash ^= (hash << 8);
        hash += (hash >> rotate);
        rotate = (int)(hash & 31) + 11;
    }
    hash ^= (hash >> rotate);
    return hash;
}
```

### Rust body
```rust
fn picohash_bytes() {
    use crate::hash::hash_bytes;

    let mut test = [0u8; 1024];
    let mut k = [0u8; 16];
    hash_test_init(&mut test, &mut k);

    let lengths: [usize; 12] = [1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024];
    let expected: [u64; 12] = [
        0x0301_6721_e32d_7aa7,
        0x6420_8401_ad85_bed5,
        0x4458_7b02_0947_9519,
        0x14a4_8174_8ee6_d77e,
        0x9a44_370f_d1b8_c1ee,
        0x2708_1725_c416_4c1a,
        0x2f1f_325d_a756_df85,
        0x2aa4_fda7_96f9_ffff,
        0x8ded_0692_d703_8037,
        0x7893_f939_9f50_7284,
        0x47a0_65db_eea7_7343,
        0xb543_a5b3_c675_127d,
    ];

    for (i, &len) in lengths.iter().enumerate() {
        let h = hash_bytes(&test[..len], &k);
        assert_eq!(h, expected[i], "picohash_bytes[{i}] for len={len}");
    }
}
```

## `picoquic/picoquic_ptls_openssl.c:picoquic_openssl_get_certs_from_file`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C allocates a chain and loops parsing PEM X509 certs into DER; Rust shown only reads the file or returns an empty Vec.
* C source: `picoquic/picoquic_ptls_openssl.c:210-235`
* C signature: `ptls_iovec_t * picoquic_openssl_get_certs_from_file(const char *, size_t *)`
* Rust source: `rs/fq/src/sys/openssl.rs:346-350`
* Rust item: `get_certs_from_file`

### C body
```c
{
    BIO* bio_key = BIO_new_file(file_name, "rb");
    size_t const max_count = 16;
    ptls_iovec_t* chain = malloc(sizeof(ptls_iovec_t) * max_count);
    *count = 0;
    if (chain != NULL) {
        X509* cert = NULL;
        memset(chain, 0, sizeof(ptls_iovec_t) * max_count);
        /* Load cert and convert to DER */
        while (*count < max_count && (cert = PEM_read_bio_X509(bio_key, NULL, NULL, NULL)) != NULL) {
            int length = i2d_X509(cert, NULL);
            unsigned char* cert_der = (unsigned char*)malloc(length);
            unsigned char* tmp = cert_der;
            i2d_X509(cert, &tmp);
            X509_free(cert);
            chain[*count] = ptls_iovec_init(cert_der, length);
            *count += 1;
        }
    }
    BIO_free(bio_key);
    return chain;
}
```

### Rust body
```rust
    let pem = match std::fs::read(file_name) {
        Ok(data) => data,
        Err(_) => return Vec::new(),
    };
```

## `picoquic/picosocks.c:picoquic_socket_set_pmtud_options`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C may call setsockopt and return its result; Rust always returns Ok(()) with no socket option work.
* C source: `picoquic/picosocks.c:230-248`
* C signature: `int picoquic_socket_set_pmtud_options(int, int)`
* Rust source: `rs/fq/src/socks.rs:157-159`
* Rust item: `set_pmtud_options`

### C body
```c
{
    int ret = 0;
#if defined __linux && defined(IP_MTU_DISCOVER) && defined(IPV6_MTU_DISCOVER) && defined(IP_PMTUDISC_PROBE)
    int val = IP_PMTUDISC_PROBE;
    if (af == AF_INET6) {
        ret = setsockopt(sd, IPPROTO_IPV6, IPV6_MTU_DISCOVER, &val, sizeof(int));
    }
    else {
        ret = setsockopt(sd, IPPROTO_IP, IP_MTU_DISCOVER, &val, sizeof(int));
    }
#else
#ifdef UNREFERENCED_PARAMETER
    UNREFERENCED_PARAMETER(af);
    UNREFERENCED_PARAMETER(sd);
#endif
#endif  /* #if defined __linux && ... */
    return ret;
}
```

### Rust body
```rust
    fn set_pmtud_options(&mut self) -> Result<(), Error> {
        Ok(())
    }
```
