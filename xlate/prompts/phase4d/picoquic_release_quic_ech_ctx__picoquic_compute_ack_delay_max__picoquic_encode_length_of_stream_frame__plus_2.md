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

## `picoquic/ech.c:picoquic_release_quic_ech_ctx`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown calls picoquic_release_quic_ech_ctx then enables/configures ECH client/server state; it is not just releasing retry config and opener callback like C.
* C source: `picoquic/ech.c:375-389`
* C signature: `void picoquic_release_quic_ech_ctx(picoquic_quic_t *)`
* Rust source: `rs/fq/src/ech.rs:929-957`
* Rust item: `picoquic_release_quic_ech_ctx`

### C body
```c
{
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;
    PICOQUIC_THREAD_CHECK(quic);

    if (ctx != NULL) {
        ech_opener_callback_t* ech_cb = (ech_opener_callback_t*)ctx->ech.server.create_opener;
        ctx->ech.server.retry_configs.base = NULL;
        ctx->ech.server.retry_configs.len = 0;

        if (ech_cb != NULL) {
            ech_dispose_opener_callback(ech_cb);
        }
    }
}
```

### Rust body
```rust
) -> Result<(), Error> {
    picoquic_release_quic_ech_ctx(quic);
    quic.ech_client_enabled = true;

    if let Some(private_key_file) = private_key_file {
        let config_file_name = config_file_name.ok_or(Error::InvalidArgument)?;
        let opener = ech_init_opener(private_key_file, config_file_name)?;
        quic.ech_server_retry_config = Some(opener.config.clone());
        quic.ech_opener = Some(opener);
    }

    Ok(())
}
```

## `picoquic/frames.c:picoquic_compute_ack_delay_max`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body fragment only divides ack_delay_max by two inside an omitted condition, while C computes rtt/4, applies negotiation/ssthresh, max cap, min floor, and returns the value.
* C source: `picoquic/frames.c:3047-3064`
* C signature: `uint64_t picoquic_compute_ack_delay_max(picoquic_cnx_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:9021-9032`
* Rust item: `compute_ack_delay_max`

### C body
```c
{
    uint64_t ack_delay_max = rtt / 4;

    if (!cnx->is_ack_frequency_negotiated && !cnx->path[0]->is_ssthresh_initialized) {
        ack_delay_max /= 2;
    }

    if (ack_delay_max > PICOQUIC_ACK_DELAY_MAX) {
        ack_delay_max = PICOQUIC_ACK_DELAY_MAX;
    }

    if (ack_delay_max < remote_min_ack_delay) {
        ack_delay_max = remote_min_ack_delay;
    }

    return ack_delay_max;
}
```

### Rust body
```rust
        {
            ack_delay_max /= 2;
        }
```

## `picoquic/frames.c:picoquic_encode_length_of_stream_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body fragment only checks byte_index against bytes length and returns None, while C conditionally encodes length, may insert padding, updates start_index, and returns byte_index.
* C source: `picoquic/frames.c:1810-1838`
* C signature: `size_t picoquic_encode_length_of_stream_frame(uint8_t *, size_t, size_t, size_t, size_t *)`
* Rust source: `rs/fq/src/internal.rs:12081-12092`
* Rust item: `encode_length_of_stream_frame`

### C body
```c
{
    if (length < byte_space) {
        if (length == byte_space - 1) {
            /* Special case: there are N bytes available, the application wants to write N-1 bytes.
             * We can encode N bytes because then we don't need a length field, just a flag in the
             * first byte. But if we had to encode "length=N-1", that would typically require 2
             * bytes, for a total of (2 + N-1)=N+1 bytes, larger than the packet size. We also
             * don't want to avoid the length field, because the encoding would be shorter than
             * the packet size, and other parts of the code might add a byte after that, e.g. padding,
             * which the receiver would mistake as data because of the "implicit length" encoding.
             * So we work against that issue by inserting a single padding byte in front of the
             * stream header.*/
            memmove(bytes + 1, bytes, byte_index);
            bytes[0] = picoquic_frame_type_padding;
            *start_index = 1;
            byte_index++;
        }
        else {
            /* Short frame, length field is required */
            /* We checked above that there are enough bytes to encode length */
            byte_index += picoquic_varint_encode(bytes + byte_index, byte_space, (uint64_t)length);
            bytes[0] |= 2; /* Indicates presence of length */
        }
    }

    return byte_index;
}
```

### Rust body
```rust
            if byte_index >= bytes.len() {
                return None;
            }
```

## `picoquic/frames.c:picoquic_format_max_streams_frame_if_needed`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: If bidirectional encoding succeeds but unidirectional encoding fails, C preserves the first frame by advancing bytes0; Rust returns the original buffer position despite already updating bidirectional state.
* C source: `picoquic/frames.c:4664-4700`
* C signature: `uint8_t * picoquic_format_max_streams_frame_if_needed(picoquic_cnx_t *, uint8_t *, uint8_t *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:12893-12941`
* Rust item: `format_max_streams_frame_if_needed`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if (cnx->max_stream_id_bidir_local_computed + 
        2*cnx->local_parameters.initial_max_stream_id_bidir > cnx->max_stream_id_bidir_local) {
        uint64_t new_bidir_local = cnx->max_stream_id_bidir_local +
            4 * cnx->local_parameters.initial_max_stream_id_bidir;
        if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_max_streams_bidir)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, STREAM_RANK_FROM_ID(new_bidir_local))) != NULL) {
            cnx->max_stream_id_bidir_local = new_bidir_local;
            *is_pure_ack = 0;
            bytes0 = bytes;
        } else {
            *more_data = 1;
            bytes = bytes0;
        }
    }
    
    if (cnx->max_stream_id_unidir_local_computed +
        2*cnx->local_parameters.initial_max_stream_id_unidir > cnx->max_stream_id_unidir_local) {
        uint64_t new_unidir_local = cnx->max_stream_id_unidir_local + 4*cnx->local_parameters.initial_max_stream_id_unidir;

        if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_max_streams_unidir)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, STREAM_RANK_FROM_ID(new_unidir_local))) != NULL) {
            cnx->max_stream_id_unidir_local = new_unidir_local;
            *is_pure_ack = 0;
        }
        else {
            *more_data = 1;
            bytes = bytes0;
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if connection.max_stream_id_bidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_bidir
        > connection.max_stream_id_bidir_local
    {
        let new_bidir = connection.max_stream_id_bidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_bidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsBidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_bidir).rank()) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.max_stream_id_bidir_local = new_bidir;
        *is_pure_ack = 0;
    }

    if connection.max_stream_id_unidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_unidir
        > connection.max_stream_id_unidir_local
    {
        let new_unidir = connection.max_stream_id_unidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_unidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsUnidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_unidir).rank()) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.max_stream_id_unidir_local = new_unidir;
        *is_pure_ack = 0;
    }

    Some(&mut bytes[off..])
}
```

## `picoquic/frames.c:picoquic_prepare_observed_address_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C gates emission on ack/repeat/address/timing conditions, uses observed_sequence_sent, and increments repeat count only after successful formatting; Rust always formats from peer_addr, increments before formatting, and uses nb_observed_repeat as the sequence value.
* C source: `picoquic/frames.c:6409-6465`
* C signature: `uint8_t * picoquic_prepare_observed_address_frame(uint8_t *, const uint8_t *, picoquic_path_t *, picoquic_tuple_t *, uint64_t, uint64_t *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:14536-14568`
* Rust item: `prepare_observed_address_frame`

### C body
```c
{
    if (!path_x->observed_addr_acked &&
        tuple->nb_observed_repeat < 4 &&
        tuple->peer_addr.ss_family != AF_UNSPEC) {
        int is_needed = 0;

        if (tuple->nb_observed_repeat == 0) {
            is_needed = 1;
            path_x->observed_sequence_sent = path_x->cnx->observed_number++;
        }
        else {
            uint64_t repeat_time = tuple->observed_time + path_x->retransmit_timer;

            if (repeat_time <= current_time) {
                is_needed = 1;
            }
            else if (*next_wake_time > repeat_time) {
                *next_wake_time = repeat_time;
            }
        }

        if (is_needed) {
            uint64_t ftype = 0;
            uint8_t* ip_addr = NULL;
            uint16_t port = 0;

            if (tuple->peer_addr.ss_family == AF_INET6) {
                struct sockaddr_in6* addr = (struct sockaddr_in6*)&tuple->peer_addr;
                ftype = picoquic_frame_type_observed_address_v6;
                ip_addr = (uint8_t*)&addr->sin6_addr;
                port = ntohs(addr->sin6_port);
            }
            else {
                struct sockaddr_in* addr = (struct sockaddr_in*)&tuple->peer_addr;
                ftype = picoquic_frame_type_observed_address_v4;
                ip_addr = (uint8_t*)&addr->sin_addr;
                port = ntohs(addr->sin_port);
            }

            uint8_t *bytes_next = picoquic_format_observed_address_frame(
                bytes, bytes_max, ftype, path_x->observed_sequence_sent,
                ip_addr, port, more_data);
            if (bytes_next > bytes) {
                *is_pure_ack = 0;
                bytes = bytes_next;
                tuple->nb_observed_repeat += 1;
                tuple->observed_time = current_time;
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let (ftype, addr_bytes) = match tuple.peer_addr.ip() {
        core::net::IpAddr::V4(v4) => (
            crate::frames::FrameType::ObservedAddressV4 as u64,
            v4.octets().to_vec(),
        ),
        core::net::IpAddr::V6(v6) => (
            crate::frames::FrameType::ObservedAddressV6 as u64,
            v6.octets().to_vec(),
        ),
    };
    tuple.observed_time = current_time;
    tuple.nb_observed_repeat += 1;
    format_observed_address_frame(
        bytes,
        ftype,
        tuple.nb_observed_repeat as u64,
        &addr_bytes,
        tuple.peer_addr.port(),
        more_data,
    )
    .inspect(|_| {
        *is_pure_ack = 0;
    })
}
```
