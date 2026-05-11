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

## `picoquic/logwriter.c:picoquic_log_varint`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body is a log-varint wrapper returning null on zero-length decode, while Rust body is a full frames_varint_decode implementation with no bytes_max argument.
* C source: `picoquic/logwriter.c:47-51`
* C signature: `const uint8_t * picoquic_log_varint(const uint8_t *, const uint8_t *, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:6419-6434`
* Rust item: `frames_varint_decode`

### C body
```c
{
    size_t len = (bytes == NULL) ? 0 : picoquic_varint_decode(bytes, bytes_max - bytes, n64);
    return len == 0 ? NULL : bytes + len;
}
```

### Rust body
```rust
pub fn frames_varint_decode<'a>(bytes: &'a [u8], n64: &mut u64) -> Option<&'a [u8]> {
    if bytes.is_empty() {
        return None;
    }
    let length = 1usize << ((bytes[0] & 0xC0) >> 6);
    if length > bytes.len() {
        return None;
    }
    let mut v = (bytes[0] & 0x3F) as u64;
    for b in bytes.iter().take(length).skip(1) {
        v <<= 8;
        v += *b as u64;
    }
    *n64 = v;
    Some(&bytes[length..])
}
```

## `picoquic/pacing.c:picoquic_is_authorized_by_pacing`
* Phase 4C status: `suspect`
* Phase 4C rationale: Main pacing calculation matches, but C updates quic wake state when next_time changes; Rust has no visible equivalent side effect.
* C source: `picoquic/pacing.c:62-105`
* C signature: `int picoquic_is_authorized_by_pacing(picoquic_pacing_t *, uint64_t, uint64_t *, unsigned int, picoquic_quic_t *)`
* Rust source: `rs/fq/src/internal.rs:5933-5961`
* Rust item: `is_authorized`

### C body
```c
{
    int ret = 1;

    picoquic_update_pacing_bucket(pacing, current_time);

    if (pacing->bucket_nanosec < pacing->packet_time_nanosec) {
        uint64_t next_pacing_time;
        int64_t bucket_required;

        if (packet_train_mode || pacing->bandwidth_pause) {
            bucket_required = pacing->bucket_max;

            if (bucket_required > 10 * pacing->packet_time_nanosec) {
                bucket_required = 10 * pacing->packet_time_nanosec;
            }

            bucket_required -= pacing->bucket_nanosec;
        }
        else {
            bucket_required = pacing->packet_time_nanosec - pacing->bucket_nanosec;
        }

        next_pacing_time = current_time + 1 + bucket_required / 1000;
        if (next_pacing_time < *next_time) {
            pacing->bandwidth_pause = 0;
            *next_time = next_pacing_time;
            if (quic != NULL) {
                SET_LAST_WAKE(quic, PICOQUIC_SENDER);
            }
        }
        ret = 0;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> bool {
        self.update_bucket(current_time);
        if self.bucket_nanosec < self.packet_time_nanosec {
            let bucket_required = if packet_train_mode || self.bandwidth_pause != 0 {
                let mut br = self.bucket_max;
                if br > 10 * self.packet_time_nanosec {
                    br = 10 * self.packet_time_nanosec;
                }
                br - self.bucket_nanosec
            } else {
                self.packet_time_nanosec - self.bucket_nanosec
            };
            let next_pacing_ticks = current_time.ticks() + 1 + (bucket_required as u64) / 1000;
            let next_pacing = crate::Instant::from_ticks(next_pacing_ticks);
            if next_pacing_ticks < next_time.ticks() {
                self.bandwidth_pause = 0;
                *next_time = next_pacing;
            }
            false
        } else {
            true
        }
    }
```

## `picoquic/packet.c:picoquic_get_packet_number64`
* Phase 4C status: `suspect`
* Phase 4C rationale: C computes expected as highest + 1 and bases reconstruction on expected; Rust bases candidate on highest and compares against highest, which is visibly different.
* C source: `picoquic/packet.c:500-524`
* C signature: `uint64_t picoquic_get_packet_number64(uint64_t, uint64_t, uint32_t)`
* Rust source: `rs/fq/src/internal.rs:7062-7077`
* Rust item: `get_packet_number64`

### C body
```c
{
    uint64_t expected = highest + 1;
    uint64_t not_mask_plus_one = (~mask) + 1;
    uint64_t pn64 = (expected & mask) | pn;

    if (pn64 < expected) {
        uint64_t delta1 = expected - pn64;
        uint64_t delta2 = not_mask_plus_one - delta1;
        if (delta2 < delta1) {
            pn64 += not_mask_plus_one;
        }
    } else {
        uint64_t delta1 = pn64 - expected;
        uint64_t delta2 = not_mask_plus_one - delta1;

        if (delta2 <= delta1 && (pn64 & mask) > 0) {
            /* Out of sequence packet from previous roll */
            pn64 -= not_mask_plus_one;
        }
    }

    return pn64;
}
```

### Rust body
```rust
pub fn get_packet_number64(highest: u64, mask: u64, pn: u32) -> u64 {
    // C: picoquic_get_packet_number64 — reconstruct full 64-bit PN from truncated `pn`
    // using `highest` (last seen full PN) and `mask` (which bits are carried).
    // The truncated PN has (mask+1) possible values; pick the one closest to highest.
    let candidate_base = highest & !mask;
    let candidate = candidate_base | (pn as u64 & mask);
    // Adjust if candidate is too far from highest.
    let half_window = (mask + 1).div_ceil(2);
    if candidate + half_window < highest {
        candidate + mask + 1
    } else if candidate > highest + half_window && candidate > mask {
        candidate - (mask + 1)
    } else {
        candidate
    }
}
```

## `picoquic/packet.c:picoquic_remove_header_protection_inner`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust performs header protection removal, but visible differences include fixed 16-byte sample size, no pn_enc-null branch, no payload_length decrement, no key_phase update, and no reserved-bit check.
* C source: `picoquic/packet.c:526-615`
* C signature: `int picoquic_remove_header_protection_inner(uint8_t *, size_t, uint8_t *, picoquic_packet_header *, void *, unsigned int, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:7079-7128`
* Rust item: `remove_header_protection_inner`

### C body
```c
{
    int ret = 0;

    if (pn_enc != NULL)
    {
        /* The header length is not yet known, will only be known after the sequence number is decrypted */
        size_t mask_length = 5;
        size_t sample_offset = ph->pn_offset + 4;
        size_t sample_size = picoquic_pn_iv_size(pn_enc);
        uint8_t mask_bytes[5] = { 0, 0, 0, 0, 0 };

        if (sample_offset + sample_size > length)
        {
            /* return an error */
            /* Invalid packet format. Avoid crash! */
            ph->pn = 0xFFFFFFFF;
            ph->pnmask = 0xFFFFFFFF00000000ull;
            ph->offset = ph->pn_offset;

            DBG_PRINTF("Invalid packet length, type: %d, epoch: %d, pc: %d, pn-offset: %d, length: %d\n",
                ph->ptype, ph->epoch, ph->pc, (int)ph->pn_offset, (int)length);
        }
        else
        {   /* Decode */
            uint8_t first_byte = bytes[0];
            uint8_t first_mask = ((first_byte & 0x80) == 0x80) ? 0x0F : (is_loss_bit_enabled_incoming)?0x07:0x1F;
            uint8_t pn_l;
            uint32_t pn_val = 0;

            memcpy(decrypted_bytes, bytes, ph->pn_offset);
            picoquic_pn_encrypt(pn_enc, bytes + sample_offset, mask_bytes, mask_bytes, mask_length);
            /* Decode the first byte */
            first_byte ^= (mask_bytes[0] & first_mask);
            pn_l = (first_byte & 3) + 1;
            ph->pnmask = (0xFFFFFFFFFFFFFFFFull);
            decrypted_bytes[0] = first_byte;

            /* Packet encoding is 1 to 4 bytes */
            for (uint8_t i = 1; i <= pn_l; i++) {
                pn_val <<= 8;
                decrypted_bytes[ph->offset] = bytes[ph->offset]^mask_bytes[i];
                pn_val += decrypted_bytes[ph->offset++];
                ph->pnmask <<= 8;
            }

            ph->pn = pn_val;
            ph->payload_length -= pn_l;
            /* Only set the key phase byte if short header */
            if (ph->ptype == picoquic_packet_1rtt_protected) {
                ph->key_phase = ((first_byte >> 2) & 1);
            }

            /* Build a packet number to 64 bits */
            ph->pn64 = picoquic_get_packet_number64(sack_list_last, ph->pnmask, ph->pn);

            /* Check the reserved bits */
            if ((first_byte & 0x80) == 0) {
                ph->has_reserved_bit_set = !is_loss_bit_enabled_incoming && (first_byte & 0x18) != 0;
            }
            else{
                ph->has_reserved_bit_set = (first_byte & 0x0c) != 0;
            }
        }
    }
    else {
        /* The pn_enc algorithm was not initialized. Avoid crash! */
        ph->pn = 0xFFFFFFFF;
        ph->pnmask = 0xFFFFFFFF00000000ull;
        ph->offset = ph->pn_offset;
        ph->pn64 = 0xFFFFFFFFFFFFFFFFull;

        DBG_PRINTF("PN dec not ready, type: %d, epoch: %d, pc: %d, pn: %d\n",
            ph->ptype, ph->epoch, ph->pc, (int)ph->pn);

        ret = PICOQUIC_ERROR_AEAD_NOT_READY;
    }

    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let length = length.min(bytes.len());
    if ph.packet_number_offset >= length {
        return -1;
    }
    let sample_offset = ph.packet_number_offset.saturating_add(4);
    if sample_offset + 16 > length {
        return -1;
    }
    let mut sample = [0u8; 16];
    sample.copy_from_slice(&bytes[sample_offset..sample_offset + 16]);
    let mask = pn_enc.mask(sample);
    let first_mask = if (bytes[0] & 0x80) != 0 { 0x0f } else { 0x1f };
    bytes[0] ^= mask[0] & first_mask;

    if is_loss_bit_enabled_incoming && (bytes[0] & 0x80) == 0 {
        ph.has_loss_bits = true;
        ph.loss_bit_l = (bytes[0] & 0x08) != 0;
        ph.loss_bit_q = (bytes[0] & 0x10) != 0;
    }

    let pn_length = ((bytes[0] & 0x03) + 1) as usize;
    if ph.packet_number_offset + pn_length > length {
        return -1;
    }
    let mut truncated = 0u32;
    for i in 0..pn_length {
        let b = bytes[ph.packet_number_offset + i] ^ mask[i + 1];
        bytes[ph.packet_number_offset + i] = b;
        truncated = (truncated << 8) | b as u32;
    }
    ph.packet_number_truncated = truncated;
    ph.packet_number_mask = if pn_length == 4 {
        u32::MAX as u64
    } else {
        (1u64 << (8 * pn_length)) - 1
    };
    ph.packet_number_full = get_packet_number64(sack_list_last, ph.packet_number_mask, truncated);
    let copy_len = length.min(decrypted_bytes.len());
    decrypted_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);
    0
}
```

## `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_generate_block_cipher`
* Phase 4C status: `suspect`
* Phase 4C rationale: C copies lb_ctx->server_id before encrypting, while Rust copies self.server_id_encoded before encrypting.
* C source: `picoquic/picoquic_lb.c:110-121`
* C signature: `void picoquic_lb_compat_cid_generate_block_cipher(picoquic_quic_t *, picoquic_load_balancer_cid_context_t *, picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/lb.rs:379-386`
* Rust item: `generate_block_cipher`

### C body
```c
{
    picoquic_lb_compat_cid_generate_first_byte(quic, lb_ctx, cnx_id_returned);
    /* Copy the server ID */
    memcpy(cnx_id_returned->id + 1, lb_ctx->server_id, lb_ctx->server_id_length);
    /* encrypt 16 bytes */
    picoquic_aes128_ecb_encrypt(lb_ctx->cid_encryption_context, cnx_id_returned->id + 1, cnx_id_returned->id + 1, 16);
}
```

### Rust body
```rust
    fn generate_block_cipher(&self, quic: &Quic, bytes: &mut [u8]) {
        self.set_first_byte(quic, bytes);
        bytes[1..1 + self.server_id_length]
            .copy_from_slice(&self.server_id_encoded[..self.server_id_length]);
        let enc = self.cid_encryption_context.as_ref().unwrap();
        let block = <&mut [u8; 16]>::try_from(&mut bytes[1..17]).unwrap();
        enc.process(block);
    }
```
