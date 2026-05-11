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

## Pair `picoquic/bbr1.c:picoquic_bbr1_suspension_exit`
C: `picoquic/bbr1.c:1174-1186 picoquic_bbr1_suspension_exit`
Rust: `rs/fq/src/bbr1.rs:1053-1060 suspension_exit`

### C body
```c
{
    if (bbr1_state->is_suspended &&
        bbr1_state->is_suspension_nearly_over) {
        path_x->cwin = bbr1_state->cwin_before_suspension;
        /* Set the pacing rate in picoquic sender */
        picoquic_update_pacing_rate(path_x, bbr1_state->pacing_rate, bbr1_state->send_quantum);
    }
    bbr1_state->is_suspended = 0;
    bbr1_state->is_suspension_nearly_over = 0;
}
```

### Rust body
```rust
    fn suspension_exit(&mut self, path_x: &mut Path) {
        if self.is_suspended && self.is_suspension_nearly_over {
            path_x.cwin = self.cwin_before_suspension;
            path_x.update_pacing_rate(self.pacing_rate, self.send_quantum);
        }
        self.is_suspended = false;
        self.is_suspension_nearly_over = false;
    }
```

## Pair `picoquic/bytestream.c:bytestream_buf_init`
C: `picoquic/bytestream.c:39-50 bytestream_buf_init`
Rust: `rs/fq/src/bytestream.rs:119-611 stream`

### C body
```c
{
    if (nb_bytes > BYTESTREAM_MAX_BUFFER_SIZE) {
        return NULL;
    }

    s->s.data = s->buf;
    s->s.size = nb_bytes;
    s->s.ptr = 0;

    return &s->s;
}
```

### Rust body
```rust
impl<'a> ByteStream<'a> {
    fn data_ref(&self) -> &[u8] {
        match &self.data {
            ByteStreamData::Borrowed(s) => s,
            ByteStreamData::Owned(v) => v.as_slice(),
        }
    }

    fn data_mut(&mut self) -> &mut [u8] {
        match &mut self.data {
            ByteStreamData::Borrowed(s) => s,
            ByteStreamData::Owned(v) => v.as_mut_slice(),
        }
    }

    /// C: `bytestream_error` (picoquic/bytestream.c:433)
    ///
    /// Sets the cursor to end-of-buffer, disabling all subsequent operations.
    /// Callers additionally return `Err(Error::BufferTooSmall)` — the Rust
    /// equivalent of the C `-1` return value.
    fn set_error(&mut self) {
        let cap = self.data_ref().len();
        self.ptr = cap;
    }

    /// Wrap a caller-owned mutable byte slice as a bytestream
    /// cursor.  C: `bytestream_ref_init`.
    ///
    /// The C function took `const void*` and cast away the const
    /// because the same struct served read and write callers.
    /// Rust takes `&mut [u8]` for both — read-only callers can
    /// still pass `&mut` to a buffer they treat as read-only.  A
    /// read-only-only split can be revisited once the read/write
    /// callers are fully translated.
    pub fn from_slice(bytes: &'a mut [u8]) -> Self {
        ByteStream {
            data: ByteStreamData::Borrowed(bytes),
            ptr: 0,
        }
    }

    /// Full underlying buffer (offsets `0..size`).  C:
    /// `bytestream_data` returning `const uint8_t*`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data_ref()[..self.ptr]
    }

    /// Slice from the cursor to the end of the buffer (offsets
    /// `len()..capacity()`).  C: `bytestream_ptr` returning
    /// `const uint8_t*`.
    ///
    /// REVIEW(open): only the encrypt/decrypt helpers in
    /// `quic/quicctx.c` use this — they want a writable tail to
    /// hand to the AEAD layer.  Keep until those callers are
    /// translated; if Phase 4 finds no users, drop it.
    pub fn tail(&self) -> &[u8] {
        &self.data_ref()[self.ptr..]
    }

    /// Total active capacity (the C `size` field).  C:
    /// `bytestream_size`.
    pub fn capacity(&self) -> usize {
        self.data_ref().len()
    }

    /// Bytes consumed so far (the C `ptr` field).  C:
    /// `bytestream_length`.
    pub fn len(&self) -> usize {
        self.ptr
    }

    /// `true` when no bytes have been consumed yet.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Bytes left between the cursor and the end of the buffer.
    /// C: `bytestream_remain`.
    pub fn remaining(&self) -> usize {
        self.data_ref().len() - self.ptr
    }

    /// Rewind the cursor to offset 0 without touching the buffer.
    /// C: `bytestream_reset`.
    pub fn reset(&mut self) {
        self.ptr = 0;
    }

    /// Rewind and zero the entire buffer.  C: `bytestream_clear` —
    /// `memset(s->data, 0, s->size)` then `s->ptr = 0`.
    pub fn clear(&mut self) {
        self.ptr = 0;
        self.data_mut().fill(0);
    }

    /// `true` when the cursor has reached the end of the buffer.
    /// C: `bytestream_finished` returning `int` (0/1).
    pub fn is_finished(&self) -> bool {
        self.ptr >= self.data_ref().len()
    }

    /// Advance the cursor by `nb_bytes` without reading or
    /// writing.  On underflow, the C version sets `ptr = size`
    /// (forcing all subsequent operations to fail) and returns
    /// `-1`; the Rust version preserves that side effect and
    /// returns `Err`.  C: `bytestream_skip`.
    pub fn skip(&mut self, nb_bytes: usize) -> Result<(), Error> {
        if self.data_ref().len() - self.ptr < nb_bytes {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        self.ptr += nb_bytes;
        Ok(())
    }

    /// Write one byte.  C: `bytewrite_int8`.
    pub fn write_u8(&mut self, value: u8) -> Result<(), Error> {
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr] = value;
        self.ptr += 1;
        Ok(())
    }

    /// Read one byte, advancing the cursor.  C: `byteread_int8`.
    pub fn read_u8(&mut self) -> Result<u8, Error> {
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let v = self.data_ref()[self.ptr];
        self.ptr += 1;
        Ok(v)
    }

    /// Read one byte without advancing the cursor.  C:
    /// `byteshow_int8`.
    ///
    /// The C version took a non-`const` pointer because every
    /// other function did; the Rust version takes `&self`.
    pub fn peek_u8(&self) -> Result<u8, Error> {
        if self.ptr >= self.data_ref().len() {
            return Err(Error::BufferTooSmall);
        }
        Ok(self.data_ref()[self.ptr])
    }

    /// Write a big-endian `u16`.  C: `bytewrite_int16`.
    pub fn write_u16(&mut self, value: u16) -> Result<(), Error> {
        if self.data_ref().len() - self.ptr < 2 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr..ptr + 2].copy_from_slice(&value.to_be_bytes());
        self.ptr += 2;
        Ok(())
    }

    /// Read a big-endian `u16`.  C: `byteread_int16`.
    pub fn read_u16(&mut self) -> Result<u16, Error> {
        if self.data_ref().len() - self.ptr < 2 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let p = self.ptr;
        let d = self.data_ref();
        let v = u16::from_be_bytes([d[p], d[p + 1]]);
        self.ptr += 2;
        Ok(v)
    }

    /// Write a big-endian `u32`.  C: `bytewrite_int32`.
    pub fn write_u32(&mut self, value: u32) -> Result<(), Error> {
        if self.data_ref().len() - self.ptr < 4 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr..ptr + 4].copy_from_slice(&value.to_be_bytes());
        self.ptr += 4;
        Ok(())
    }

    /// Read a big-endian `u32`.  C: `byteread_int32`.
    pub fn read_u32(&mut self) -> Result<u32, Error> {
        if self.data_ref().len() - self.ptr < 4 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let p = self.ptr;
        let d = self.data_ref();
        let v = u32::from_be_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]);
        self.ptr += 4;
        Ok(v)
    }

    /// Write a big-endian `u64`.  C: `bytewrite_int64`.
    pub fn write_u64(&mut self, value: u64) -> Result<(), Error> {
        if self.data_ref().len() - self.ptr < 8 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr..ptr + 8].copy_from_slice(&value.to_be_bytes());
        self.ptr += 8;
        Ok(())
    }

    /// Read a big-endian `u64`.  C: `byteread_int64`.
    pub fn read_u64(&mut self) -> Result<u64, Error> {
        if self.data_ref().len() - self.ptr < 8 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let p = self.ptr;
        let d = self.data_ref();
        let v = u64::from_be_bytes([
            d[p],
            d[p + 1],
            d[p + 2],
            d[p + 3],
            d[p + 4],
            d[p + 5],
            d[p + 6],
            d[p + 7],
        ]);
        self.ptr += 8;
        Ok(v)
    }

    /// Write a QUIC variable-length integer (RFC 9000 §16).  C:
    /// `bytewrite_vint`.
    pub fn write_varint(&mut self, value: u64) -> Result<(), Error> {
        let len = Self::varint_encoded_len(value);
        if self.data_ref().len() - self.ptr < len {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        let d = self.data_mut();
        match len {
            1 => d[ptr] = value as u8,
            2 => {
                d[ptr] = 0x40 | (value >> 8) as u8;
                d[ptr + 1] = value as u8;
            }
            4 => {
                d[ptr] = 0x80 | (value >> 24) as u8;
                d[ptr + 1] = (value >> 16) as u8;
                d[ptr + 2] = (value >> 8) as u8;
                d[ptr + 3] = value as u8;
            }
            _ => {
                d[ptr] = 0xC0 | (value >> 56) as u8;
                d[ptr + 1] = (value >> 48) as u8;
                d[ptr + 2] = (value >> 40) as u8;
                d[ptr + 3] = (value >> 32) as u8;
                d[ptr + 4] = (value >> 24) as u8;
                d[ptr + 5] = (value >> 16) as u8;
                d[ptr + 6] = (value >> 8) as u8;
                d[ptr + 7] = value as u8;
            }
        }
        self.ptr += len;
        Ok(())
    }

    /// Read a QUIC variable-length integer.  C: `byteread_varint`.
    pub fn read_varint(&mut self) -> Result<u64, Error> {
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let first = self.data_ref()[self.ptr];
        let len = 1usize << ((first >> 6) & 3);
        if self.data_ref().len() - self.ptr < len {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let p = self.ptr;
        let d = self.data_ref();
        let value = match len {
            1 => (first & 0x3F) as u64,
            2 => (((first & 0x3F) as u64) << 8) | (d[p + 1] as u64),
            4 => {
                (((first & 0x3F) as u64) << 24)
                    | ((d[p + 1] as u64) << 16)
                    | ((d[p + 2] as u64) << 8)
                    | (d[p + 3] as u64)
            }
            _ => {
                (((first & 0x3F) as u64) << 56)
                    | ((d[p + 1] as u64) << 48)
                    | ((d[p + 2] as u64) << 40)
                    | ((d[p + 3] as u64) << 32)
                    | ((d[p + 4] as u64) << 24)
                    | ((d[p + 5] as u64) << 16)
                    | ((d[p + 6] as u64) << 8)
                    | (d[p + 7] as u64)
            }
        };
        self.ptr += len;
        Ok(value)
    }

    /// Skip past a QUIC variable-length integer without decoding
    /// it.  C: `byteread_skip_varint`.
    pub fn skip_varint(&mut self) -> Result<(), Error> {
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let first = self.data_ref()[self.ptr];
        let len = 1usize << ((first >> 6) & 3);
        self.skip(len)
    }

    /// Read a varint and downcast to `usize`.  C: `byteread_vlen`.
    /// Returns `Err` when the value doesn't fit in a `usize` on
    /// the target.  Mirrors the C check `*value != val_read`.
    pub fn read_vlen(&mut self) -> Result<usize, Error> {
        let val = self.read_varint()?;
        let as_usize = val as usize;
        if as_usize as u64 != val {
            return Err(Error::InvalidArgument);
        }
        Ok(as_usize)
    }

    /// Copy `buffer.len()` bytes into the stream.  C:
    /// `bytewrite_buffer(s, buffer, length)` — the explicit
    /// `length` is taken from the slice.
    pub fn write_bytes(&mut self, buffer: &[u8]) -> Result<(), Error> {
        let length = buffer.len();
        if self.data_ref().len() - self.ptr < length {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr..ptr + length].copy_from_slice(buffer);
        self.ptr += length;
        Ok(())
    }

    /// Copy `buffer.len()` bytes out of the stream.  C:
    /// `byteread_buffer`.
    pub fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        let length = buffer.len();
        if self.data_ref().len() - self.ptr < length {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        buffer.copy_from_slice(&self.data_ref()[self.ptr..self.ptr + length]);
        self.ptr += length;
        Ok(())
    }

    /// Encode a connection id as `u8` length + `id_len` raw bytes.
    /// C: `bytewrite_cid`.
    pub fn write_cid(&mut self, cid: &ConnectionId) -> Result<(), Error> {
        self.write_u8(cid.id_len)?;
        self.write_bytes(&cid.id[..cid.id_len as usize])
    }

    /// Decode a connection id.  C: `byteread_cid` (the C version
    /// filled an out parameter; the Rust version returns the
    /// value).  Returns `Err` when the encoded length exceeds
    /// [`crate::CONNECTION_ID_MAX_SIZE`].
    pub fn read_cid(&mut self) -> Result<ConnectionId, Error> {
        let id_len = self.read_u8()?;
        if id_len as usize > crate::CONNECTION_ID_MAX_SIZE {
            self.set_error();
            return Err(Error::InvalidArgument);
        }
        let mut cid = ConnectionId {
            id_len,
            ..ConnectionId::default()
        };
        self.read_bytes(&mut cid.id[..id_len as usize])?;
        Ok(cid)
    }

    /// Skip past an encoded connection id.  C: `byteskip_cid`.
    pub fn skip_cid(&mut self) -> Result<(), Error> {
        let id_len = self.read_u8()?;
        self.skip(id_len as usize)
    }

    /// Encode a string as varint-length + raw bytes (no NUL on the
    /// wire).  C: `bytewrite_cstr` — took a `const char*` and
    /// called `strlen`; the Rust version takes a `&str` and uses
    /// its length directly.
    pub fn write_str(&mut self, s: &str) -> Result<(), Error> {
        let bytes = s.as_bytes();
        self.write_varint(bytes.len() as u64)?;
        self.write_bytes(bytes)
    }

    /// Decode a length-prefixed string into `dst`, returning the
    /// number of bytes written.  C: `byteread_cstr(s, char* cstr,
    /// size_t max_len)` wrote a NUL-terminated string and required
    /// `length + 1 <= max_len`; the Rust version writes raw bytes
    /// (callers reconstruct a `&str`/`&CStr` themselves) and
    /// returns `Err` when `length > dst.len()`.
    pub fn read_str(&mut self, dst: &mut [u8]) -> Result<usize, Error> {
        let l_read = self.read_varint()?;
        let l = l_read as usize;
        if (l_read as usize as u64) != l_read {
            self.set_error();
            return Err(Error::InvalidArgument);
        }
        if l > dst.len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        self.read_bytes(&mut dst[..l])?;
        Ok(l)
    }

    /// Skip past a length-prefixed string.  C: `byteskip_cstr`.
    pub fn skip_str(&mut self) -> Result<(), Error> {
        let l_read = self.read_varint()?;
        let l = l_read as usize;
        if (l_read as usize as u64) != l_read {
            self.set_error();
            return Err(Error::InvalidArgument);
        }
        self.skip(l)
    }

    /// Encode a socket address as varint(family) + 4 or 16 raw
    /// bytes (address) + big-endian `u16` (port).  C:
    /// `bytewrite_addr`, which branched on `sa_family`; the Rust
    /// version uses [`SocketAddr`].
    pub fn write_addr(&mut self, addr: &SocketAddr) -> Result<(), Error> {
        match addr {
            SocketAddr::V4(a) => {
                self.write_varint(WIRE_AF_INET)?;
                self.write_bytes(&a.ip().octets())?;
                self.write_u16(a.port())?;
            }
            SocketAddr::V6(a) => {
                self.write_varint(WIRE_AF_INET6)?;
                self.write_bytes(&a.ip().octets())?;
                self.write_u16(a.port())?;
            }
        }
        Ok(())
    }

    /// Decode a socket address.  C: `byteread_addr` (the C version
    /// filled a `sockaddr_storage` out parameter; the Rust version
    /// returns the value).
    pub fn read_addr(&mut self) -> Result<SocketAddr, Error> {
        let family = self.read_varint()?;
        if family == WIRE_AF_INET {
            let mut octets = [0u8; 4];
            self.read_bytes(&mut octets)?;
            let port = self.read_u16()?;
            Ok(SocketAddr::from((octets, port)))
        } else {
            let mut octets = [0u8; 16];
            self.read_bytes(&mut octets)?;
            let port = self.read_u16()?;
            Ok(SocketAddr::from((octets, port)))
        }
    }

    /// Skip past an encoded socket address.  C: `byteskip_addr`.
    pub fn skip_addr(&mut self) -> Result<(), Error> {
        let family = self.read_varint()?;
        if family == WIRE_AF_INET {
            self.skip(4 + 2)
        } else {
            self.skip(16 + 2)
        }
    }
}
```

## Pair `picoquic/bytestream.c:bytestream_ptr`
C: `picoquic/bytestream.c:78-81 bytestream_ptr`
Rust: `rs/fq/src/bytestream.rs:185-193 tail`

### C body
```c
{
    return s->data + s->ptr;
}
```

### Rust body
```rust
    pub fn capacity(&self) -> usize {
        self.data_ref().len()
    }
```

## Pair `picoquic/bytestream.c:bytestream_reset`
C: `picoquic/bytestream.c:98-101 bytestream_reset`
Rust: `rs/fq/src/bbr1.rs:1089-1139 reset`

### C body
```c
{
    s->ptr = 0;
}
```

### Rust body
```rust
        *self = Self {
            state: Bbr1AlgState::Startup,
            btl_bw: 0,
            next_round_delivered: 0,
            btl_bw_filter: [0; BBR1_BTL_BW_FILTER_LENGTH],
            full_bw: 0,
            rt_prop: 0,
            rt_prop_stamp: 0,
            cycle_stamp: 0,
            probe_rtt_done_stamp: 0,
            prior_cwnd: 0,
            prior_in_flight: 0,
            bytes_delivered: 0,
            send_quantum: 0,
            rtt_filter: MinMaxRtt::default(),
            target_cwnd: 0,
            pacing_gain: 0.0,
            cwnd_gain: 0.0,
            pacing_rate: 0.0,
            cycle_index: 0,
            cycle_start: 0,
            round_count: 0,
            full_bw_count: 0,
            lt_rtt_cnt: 0,
            lt_bw: 0,
            lt_last_stamp: 0,
            previous_round_lost: 0,
            previous_sampling_delivered: 0,
            previous_sampling_lost: 0,
            loss_interval_start: 0,
            congestion_sequence: 0,
            cwin_before_suspension: 0,
            option_string: None,
            wifi_shadow_rtt: 0,
            quantum_ratio: 0.0,
            filled_pipe: false,
            round_start: false,
            rt_prop_expired: false,
            probe_rtt_round_done: false,
            idle_restart: false,
            packet_conservation: false,
            btl_bw_increased: false,
            lt_use_bw: false,
            lt_is_sampling: false,
            last_loss_was_timeout: false,
            cycle_on_loss: false,
            is_suspended: false,
            is_suspension_nearly_over: false,
        };
```

## Pair `picoquic/bytestream.c:byteread_skip_vint`
C: `picoquic/bytestream.c:125-134 byteread_skip_vint`
Rust: `rs/fq/src/bytestream.rs:441-449 skip_varint`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 1) {
        return bytestream_error(s);
    }

    size_t len = picoquic_decode_varint_length(s->data[s->ptr]);
    return bytestream_skip(s, len);
}
```

### Rust body
```rust
    pub fn skip_varint(&mut self) -> Result<(), Error> {
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let first = self.data_ref()[self.ptr];
        let len = 1usize << ((first >> 6) & 3);
        self.skip(len)
    }
```

## Pair `picoquic/bytestream.c:byteread_vlen`
C: `picoquic/bytestream.c:168-175 byteread_vlen`
Rust: `rs/fq/src/bytestream.rs:454-461 read_vlen`

### C body
```c
{
    uint64_t val_read = 0;
    int ret = byteread_vint(s, &val_read);

    *value = (size_t)val_read;
    return *value != val_read ? -1 : ret;
}
```

### Rust body
```rust
    pub fn read_vlen(&mut self) -> Result<usize, Error> {
        let val = self.read_varint()?;
        let as_usize = val as usize;
        if as_usize as u64 != val {
            return Err(Error::InvalidArgument);
        }
        Ok(as_usize)
    }
```

## Pair `picoquic/bytestream.c:bytewrite_int16`
C: `picoquic/bytestream.c:210-220 bytewrite_int16`
Rust: `rs/fq/src/bytestream.rs:281-290 write_u16`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 2) {
        return bytestream_error(s);
    } else {
        picoformat_16(s->data + s->ptr, value);
        s->ptr += 2;
        return 0;
    }
}
```

### Rust body
```rust
    pub fn write_u16(&mut self, value: u16) -> Result<(), Error> {
        if self.data_ref().len() - self.ptr < 2 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr..ptr + 2].copy_from_slice(&value.to_be_bytes());
        self.ptr += 2;
        Ok(())
    }
```

## Pair `picoquic/bytestream.c:bytewrite_int64`
C: `picoquic/bytestream.c:262-272 bytewrite_int64`
Rust: `rs/fq/src/bytestream.rs:331-340 write_u64`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 8) {
        return bytestream_error(s);
    } else {
        picoformat_64(s->data + s->ptr, value);
        s->ptr += 8;
        return 0;
    }
}
```

### Rust body
```rust
    pub fn write_u64(&mut self, value: u64) -> Result<(), Error> {
        if self.data_ref().len() - self.ptr < 8 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        let ptr = self.ptr;
        self.data_mut()[ptr..ptr + 8].copy_from_slice(&value.to_be_bytes());
        self.ptr += 8;
        Ok(())
    }
```

## Pair `picoquic/bytestream.c:bytewrite_cid`
C: `picoquic/bytestream.c:317-322 bytewrite_cid`
Rust: `rs/fq/src/bytestream.rs:493-496 write_cid`

### C body
```c
{
    int ret = bytewrite_int8(s, cid->id_len);
    ret |= bytewrite_buffer(s, cid->id, cid->id_len);
    return ret;
}
```

### Rust body
```rust
    pub fn write_cid(&mut self, cid: &ConnectionId) -> Result<(), Error> {
        self.write_u8(cid.id_len)?;
        self.write_bytes(&cid.id[..cid.id_len as usize])
    }
```

## Pair `picoquic/bytestream.c:byteread_cstr`
C: `picoquic/bytestream.c:354-369 byteread_cstr`
Rust: `rs/fq/src/bytestream.rs:538-551 read_str`

### C body
```c
{
    uint64_t l_read = 0;
    int ret = byteread_vint(s, &l_read);

    size_t l_cstr = (size_t)l_read;

    if (ret != 0 || l_cstr != l_read || l_cstr + 1 > max_len) {
        ret = -1;
    } else {
        ret |= byteread_buffer(s, cstr, l_cstr);
        cstr[l_cstr] = 0;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn read_str(&mut self, dst: &mut [u8]) -> Result<usize, Error> {
        let l_read = self.read_varint()?;
        let l = l_read as usize;
        if (l_read as usize as u64) != l_read {
            self.set_error();
            return Err(Error::InvalidArgument);
        }
        if l > dst.len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
        self.read_bytes(&mut dst[..l])?;
        Ok(l)
    }
```

## Pair `picoquic/bytestream.c:byteskip_addr`
C: `picoquic/bytestream.c:420-431 byteskip_addr`
Rust: `rs/fq/src/bytestream.rs:603-610 skip_addr`

### C body
```c
{
    uint64_t family = 0;
    int ret = byteread_vint(s, &family);

    if (ret == 0 && family == AF_INET) {
        ret |= bytestream_skip(s, 4 + 2);
    } else {
        ret |= bytestream_skip(s, 16 + 2);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn skip_addr(&mut self) -> Result<(), Error> {
        let family = self.read_varint()?;
        if family == WIRE_AF_INET {
            self.skip(4 + 2)
        } else {
            self.skip(16 + 2)
        }
    }
```

## Pair `picoquic/c4.c:c4_delay_threshold`
C: `picoquic/c4.c:266-275 c4_delay_threshold`
Rust: `rs/fq/src/c4.rs:260-265 delay_threshold`

### C body
```c
{
    uint64_t sensitivity = c4_sensitivity_1024(c4_state);
    uint64_t fraction = 64 + MULT1024(1024 - sensitivity, 196);
    uint64_t delay = MULT1024(fraction, c4_state->nominal_max_rtt);
    if (delay > C4_DELAY_THRESHOLD_MAX) {
        delay = C4_DELAY_THRESHOLD_MAX;
    }
    return delay;
}
```

### Rust body
```rust
    pub fn delay_threshold(&self) -> u64 {
        let sensitivity = self.sensitivity_1024();
        let fraction = 64 + mult1024(1024 - sensitivity, 196);
        let delay = mult1024(fraction, self.nominal_max_rtt);
        delay.min(C4_DELAY_THRESHOLD_MAX)
    }
```

## Pair `picoquic/c4.c:c4_update_ecn_alpha`
C: `picoquic/c4.c:321-350 c4_update_ecn_alpha`
Rust: `rs/fq/src/c4.rs:228-251 ecn_alpha_update_counts`

### C body
```c
{
    uint64_t frac = 0;
    picoquic_packet_context_t* pkt_ctx = (path_x->cnx->is_multipath_enabled)?
        &path_x->pkt_ctx : &path_x->cnx->pkt_ctx[picoquic_packet_context_application];
    int64_t delta_ect1 = pkt_ctx->ecn_ect1_total_remote - c4_state->ecn_ect1;
    int64_t delta_ce = pkt_ctx->ecn_ce_total_remote - c4_state->ecn_ce;

    c4_state->ecn_ect1 = pkt_ctx->ecn_ect1_total_remote;
    c4_state->ecn_ce = pkt_ctx->ecn_ce_total_remote;

    if (delta_ce > 0 || delta_ect1 > 0) {
        frac = (delta_ce * 1024) / (delta_ce + delta_ect1);

        if (frac > c4_state->ecn_alpha && frac >= 512) {
            c4_state->ecn_alpha = frac;
        }
        else
        {
            uint64_t alpha_shifted = c4_state->ecn_alpha << C4_ECN_SHIFT_G;
            alpha_shifted -= c4_state->ecn_alpha;
            alpha_shifted += frac;
            c4_state->ecn_alpha = alpha_shifted >> C4_ECN_SHIFT_G;
        }
    }
}
```

### Rust body
```rust
    fn ecn_alpha_update_counts(&mut self, ecn_ect1_remote: u64, ecn_ce_remote: u64) {
        let delta_ect1 = ecn_ect1_remote as i64 - self.ecn_ect1 as i64;
        let delta_ce = ecn_ce_remote as i64 - self.ecn_ce as i64;

        self.ecn_ect1 = ecn_ect1_remote;
        self.ecn_ce = ecn_ce_remote;

        if delta_ce > 0 || delta_ect1 > 0 {
            let sum = delta_ce + delta_ect1;
            let frac: u64 = if sum > 0 {
                ((delta_ce * 1024) / sum) as u64
            } else {
                0
            };

            if frac > self.ecn_alpha && frac >= 512 {
                self.ecn_alpha = frac;
            } else {
                // EWMA: alpha = alpha*(1 - 1/2^shift) + frac*(1/2^shift)
                let alpha_shifted = (self.ecn_alpha << C4_ECN_SHIFT_G) - self.ecn_alpha + frac;
                self.ecn_alpha = alpha_shifted >> C4_ECN_SHIFT_G;
            }
        }
    }
```

## Pair `picoquic/c4.c:c4_era_check`
C: `picoquic/c4.c:459-473 c4_era_check`
Rust: `rs/fq/src/c4.rs:299-319 era_check`

### C body
```c
{
    if (path_x->cnx->cnx_state < picoquic_state_ready) {
        return 0;
    }
    else {
        return (picoquic_cc_get_lowest_not_ack(path_x) > c4_state->era_sequence);
    }
}
```

### Rust body
```rust
    fn era_reset(&mut self, path_x: &Path, connection: &Connection) {
        self.era_sequence = connection.sequence_number(path_x);
        self.era_max_rtt = 0;
        self.era_min_rtt = u64::MAX;
        self.alpha_1024_previous = self.alpha_1024_current;
        self.update_ecn_alpha(path_x, connection);
    }
```

## Pair `picoquic/c4.c:c4_reset`
C: `picoquic/c4.c:517-525 c4_reset`
Rust: `rs/fq/src/c4.rs:834-873 reset`

### C body
```c
{
    memset(c4_state, 0, sizeof(c4_state_t));
    c4_state->option_string = option_string;
    c4_state->running_min_rtt = UINT64_MAX;
    c4_state->alpha_1024_current = C4_ALPHA_INITIAL;
    c4_set_options(c4_state);
    c4_enter_initial(path_x, c4_state);
}
```

### Rust body
```rust
    pub fn reset(&mut self, path_x: &mut Path, connection: &Connection) {
        let option_string = self.option_string.take();
        // Zero all fields.
        *self = C4State {
            alg_state: C4AlgState::default(),
            nominal_rate: 0,
            nominal_max_rtt: 0,
            initial_cwnd: 0,
            running_min_rtt: u64::MAX,
            alpha_1024_current: C4_ALPHA_INITIAL,
            alpha_1024_previous: 0,
            nb_packets_in_startup: 0,
            era_sequence: 0,
            nb_cruise_left_before_push: 0,
            seed_cwin: 0,
            seed_rate: 0,
            probe_level: 0,
            nb_eras_no_increase: 0,
            push_rate_old: 0,
            push_alpha: 0,
            era_max_rtt: 0,
            era_min_rtt: 0,
            delay_threshold: 0,
            recent_delay_excess: 0,
            last_lost_packet_number: 0,
            smoothed_drop_rate: 0.0,
            ecn_alpha: 0,
            ecn_ect1: 0,
            ecn_ce: 0,
            ecn_threshold: 0,
            congestion_notified: false,
            push_was_not_limited: false,
            use_seed_cwin: false,
            initial_after_jitter: false,
            excess_ce_after_push: false,
            option_string,
        };
        self.set_options();
        self.enter_initial(path_x, connection);
    }
```

## Pair `picoquic/c4.c:c4_initial_handle_loss`
C: `picoquic/c4.c:568-574 c4_initial_handle_loss`
Rust: `rs/fq/src/c4.rs:625-630 initial_handle_loss`

### C body
```c
{
    c4_state->nb_packets_in_startup += 1;
    if (c4_state->nb_packets_in_startup > C4_NB_PACKETS_BEFORE_LOSS) {
        c4_exit_initial(path_x, c4_state);
    }
}
```

### Rust body
```rust
    fn initial_handle_loss(&mut self, path_x: &mut Path, connection: &Connection) {
        self.nb_packets_in_startup += 1;
        if self.nb_packets_in_startup > C4_NB_PACKETS_BEFORE_LOSS {
            self.exit_initial(path_x, connection);
        }
    }
```

## Pair `picoquic/c4.c:c4_exit_recovery`
C: `picoquic/c4.c:674-709 c4_exit_recovery`
Rust: `rs/fq/src/c4.rs:423-445 exit_recovery`

### C body
```c
{
    /* Assess growth */
    int is_growing = c4_growth_evaluate(c4_state);
    if (is_growing) {
        if (!c4_state->excess_ce_after_push) {
            c4_state->probe_level++;
        }
    }
    else {
        if (c4_state->push_was_not_limited) {
            c4_state->probe_level = 1;
            if (c4_state->excess_ce_after_push) {
                c4_state->probe_level = 0;
            }
        }
    }
    c4_growth_reset(c4_state);
    /* Reset the delay excess to avoid bounces of delay event */
    c4_state->recent_delay_excess = 0;
    /* Reset the smoothed drop rate at the end of recovery.
    * so that the next measurements reflect the new parameters.
    */
    c4_state->smoothed_drop_rate = 0;
    /* Reset the ecn_alpha */
    c4_state->ecn_alpha = 0;

    if (c4_state->probe_level > C4_PROBE_LEVEL_MAX) {
        c4_enter_initial(path_x, c4_state);
    }
    else {
        c4_enter_cruise(path_x, c4_state);
    }
}
```

### Rust body
```rust
    fn exit_recovery(&mut self, path_x: &mut Path, connection: &Connection) {
        let is_growing = self.growth_evaluate();
        if is_growing {
            if !self.excess_ce_after_push {
                self.probe_level += 1;
            }
        } else if self.push_was_not_limited {
            self.probe_level = 1;
            if self.excess_ce_after_push {
                self.probe_level = 0;
            }
        }
        self.growth_reset();
        self.recent_delay_excess = 0;
        self.smoothed_drop_rate = 0.0;
        self.ecn_alpha = 0;

        if self.probe_level > C4_PROBE_LEVEL_MAX {
            self.enter_initial(path_x, connection);
        } else {
            self.enter_cruise(path_x, connection);
        }
    }
```

## Pair `picoquic/c4.c:c4_handle_ack`
C: `picoquic/c4.c:802-883 c4_handle_ack`
Rust: `rs/fq/src/c4.rs:752-780 handle_ack`

### C body
```c
{
    uint64_t previous_rate = c4_state->nominal_rate;
    uint64_t rate_measurement = 0;

    if (ack_state->rtt_measurement > 0 && ack_state->nb_bytes_delivered_since_packet_sent > 0) {

        rate_measurement = path_x->bandwidth_estimate;
        C4_LOGGER(path_x, rate_measurement, c4_state, ack_state, 0, 0);

        /* Assessment of rate limited status */
        if (rate_measurement > c4_state->nominal_rate &&
            !(c4_state->alg_state == c4_recovery && c4_state->congestion_notified != 0)) {
            c4_state->push_was_not_limited = 1;
            c4_state->nominal_rate = rate_measurement;
            c4_state->delay_threshold = c4_delay_threshold(c4_state);
        }
        else {
            /* The ACK rate did not grow, but that's not a proof.
                * If the number of bytes sent are larger than the corrected bytes,
                * we know the delivery was slowed by the network, not the app.
                */
            uint64_t target_cwin = PICOQUIC_BYTES_FROM_RATE(c4_state->running_min_rtt, previous_rate);
            if (ack_state->nb_bytes_delivered_since_packet_sent > target_cwin) {
                c4_state->push_was_not_limited = 1;
            }
        }
    }

    if (c4_state->alg_state == c4_initial) {
        c4_initial_handle_ack(path_x, c4_state, ack_state);
    }
    else {
        if (c4_era_check(path_x, c4_state)) {
            /* Update max rtt and running min rtt */
            c4_update_min_max_rtt(path_x, c4_state);
            /* The initial phase may have exited too early if we have both high jitter and competition
            * from other flows. Finding an RTT higher than the previous max is an indication that
            * the previous initial might have exited too soon, especially if the difference between
            * max RTT and min RTT is large. Reentering Initial remedies that.
            * However, reentering Initial is a bit of a hack. It is OK in the high jitter or
            * competition secnarios, but it can backfire and cause congestion and losses. So we don't
            * do that if the RTT is low (lower than 50ms) or if the data rate is high enough
            * (higher than 1Mbps, i.e., 8Mbps). And we only do that once per connection.
            */
            if (!c4_state->initial_after_jitter &&
                c4_state->nominal_max_rtt > 50000 &&
                c4_state->nominal_rate < 1000000 &&
                5 * c4_state->running_min_rtt < 2 * c4_state->nominal_max_rtt) {
                c4_state->initial_after_jitter = 1;
                c4_enter_initial(path_x, c4_state);
            }
            else
            {
                /* Manage the transition to the next state */
                switch (c4_state->alg_state) {
                case c4_recovery:
                    c4_exit_recovery(path_x, c4_state);
                    break;
                case c4_cruising:
                    if (c4_state->nb_cruise_left_before_push > 0) {
                        c4_state->nb_cruise_left_before_push--;
                    }
                    c4_era_reset(path_x, c4_state);
                    if (c4_state->nb_cruise_left_before_push <= 0 &&
                        path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                        c4_enter_push(path_x, c4_state);
                    }
                    break;
                case c4_pushing:
                    c4_enter_recovery(path_x, c4_state, c4_congestion_none);
                    break;
                default:
                    c4_era_reset(path_x, c4_state);
                    break;
                }
            }
        }
    }
}
```

### Rust body
```rust
        {
            let rate_measurement = path_x.bandwidth_estimate;
            c4_logger(
                path_x,
                rate_measurement,
                self,
                Some(ack_state),
                0,
                C4Congestion::None,
            );

            if rate_measurement > self.nominal_rate
                && !(self.alg_state == C4AlgState::Recovery && self.congestion_notified)
            {
                self.push_was_not_limited = true;
                self.nominal_rate = rate_measurement;
                self.delay_threshold = self.delay_threshold();
            } else {
                let target_cwin = bytes_from_rate(self.running_min_rtt, previous_rate);
                if ack_state.nb_bytes_delivered_since_packet_sent > target_cwin {
                    self.push_was_not_limited = true;
                }
            }
        }
```

## Pair `picoquic/c4.c:c4_notify`
C: `picoquic/c4.c:1019-1109 c4_notify`
Rust: `rs/fq/src/c4.rs:880-894 notify`

### C body
```c
{
    c4_state_t* c4_state = (c4_state_t*)path_x->congestion_alg_state;
    path_x->is_cc_data_updated = 1;

    if (ack_state != NULL && ack_state->pc != picoquic_packet_context_application) {
        return;
    }

    if (c4_state != NULL) {
        switch (notification) {
        case picoquic_congestion_notification_acknowledgement:
            c4_handle_ack(path_x, c4_state, ack_state);
            c4_apply_rate_and_cwin(path_x, c4_state);
            break;
        case picoquic_congestion_notification_ecn_ec:
            /* TODO: ECN is special? Implement the prague logic */
            c4_state->ecn_threshold = c4_ecn_threshold(c4_state);
            c4_update_ecn_alpha(path_x, c4_state);
            if (c4_state->ecn_alpha > c4_state->ecn_threshold) {
                if (c4_state->alg_state == c4_initial) {
                    if (c4_state->recent_delay_excess > 0
                        && c4_state->nb_eras_no_increase > 1
                        && c4_state->push_rate_old >= c4_state->nominal_rate) {

                        c4_exit_initial(path_x, c4_state);
                    }
                }
                else {
                    c4_notify_congestion(path_x, c4_state, c4_congestion_ecn);
                }
            }
            break;
        case picoquic_congestion_notification_repeat:
            if (c4_state->alg_state == c4_recovery && ack_state->lost_packet_number < c4_state->era_sequence) {
                /* Do not worry about loss of packets sent before entering recovery */
                break;
            }
            c4_update_loss_rate(c4_state, ack_state->lost_packet_number);

            if (c4_state->smoothed_drop_rate > c4_loss_threshold(c4_state)) {
                if (c4_state->alg_state == c4_initial) {
                    c4_initial_handle_loss(path_x, c4_state);
                }
                else {
                    c4_notify_congestion(path_x, c4_state, c4_congestion_loss);
                }
            }
            break;
        case picoquic_congestion_notification_timeout:
            /* Treat timeout as PTO: no impact on congestion control */
            break;
        case picoquic_congestion_notification_spurious_repeat:
            /* Remove handling of spurious repeat, as it was tied to timeout */
            break;
        case picoquic_congestion_notification_rtt_measurement:
            c4_update_rtt(c4_state, ack_state->rtt_measurement);
            if (c4_state->alg_state == c4_initial) {
                c4_initial_handle_rtt_excess(path_x, c4_state);
                c4_apply_rate_and_cwin(path_x, c4_state);
            }
            else {
                c4_handle_rtt_excess(path_x, c4_state);
            }
            break;
        case picoquic_congestion_notification_lost_feedback:
            break;
        case picoquic_congestion_notification_cwin_blocked:
            break;
        case picoquic_congestion_notification_reset:
            c4_reset(c4_state, path_x, c4_state->option_string);
            break;
        case picoquic_congestion_notification_seed_cwin:
            c4_seed_cwin(c4_state, ack_state->nb_bytes_acknowledged);
            break;
        default:
            /* ignore */
            break;
        }
    }
}
```

### Rust body
```rust
        {
            return;
        }
```

## Pair `picoquic/cc_common.c:picoquic_cc_get_lowest_not_ack`
C: `picoquic/cc_common.c:55-61 picoquic_cc_get_lowest_not_ack`
Rust: `rs/fq/src/cc_common.rs:311-416 lowest_not_ack`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = (path_x->cnx->is_multipath_enabled) ? &path_x->pkt_ctx : &path_x->cnx->pkt_ctx[picoquic_packet_context_application];
    uint64_t lowest_not_ack = (pkt_ctx->pending_first != NULL) ? pkt_ctx->pending_first->sequence_number : pkt_ctx->highest_acknowledged + 1;

    return lowest_not_ack;
}
```

### Rust body
```rust
impl PathCc for Path {
    fn lowest_not_ack(&self) -> u64 {
        // C reads cnx->pkt_ctx[app] for single-path, path->pkt_ctx for multipath.
        // Path has no back-pointer to Connection, so we always use path->pkt_ctx
        // (exact for multipath; conservative approximation for single-path).
        self.pkt_ctx
            .pending
            .keys()
            .next()
            .copied()
            .unwrap_or(self.pkt_ctx.highest_acknowledged + 1)
    }

    fn slow_start_increase(&self, nb_delivered: u64) -> u64 {
        // C body checks cnx->cwin_blocked.  Path has no back-pointer to
        // Connection, so approximate with bytes_in_transit >= cwin, which is
        // the condition that sets cwin_blocked in the C library.
        if self.bytes_in_transit < self.cwin {
            0
        } else {
            nb_delivered
        }
    }

    fn slow_start_increase_ex(&self, nb_delivered: u64, in_css: bool) -> u64 {
        if in_css {
            self.slow_start_increase(nb_delivered / HYSTART_PP_CSS_GROWTH_DIVISOR)
        } else {
            self.slow_start_increase(nb_delivered)
        }
    }

    fn slow_start_increase_ex2(&self, nb_delivered: u64, in_css: bool, prague_alpha: u64) -> u64 {
        if prague_alpha != 0 {
            let delta = if self.smoothed_rtt <= TARGET_RENO_RTT {
                nb_delivered * (1024 - prague_alpha) / 1024
            } else {
                nb_delivered * self.smoothed_rtt.ticks() * (1024 - prague_alpha)
                    / TARGET_RENO_RTT.ticks()
                    / 1024
            };
            self.slow_start_increase_ex(delta, in_css)
        } else {
            self.slow_start_increase_ex(nb_delivered, in_css)
        }
    }

    fn update_target_cwin_estimation(&self) -> u64 {
        // BYTES_FROM_RATE(smoothed_rtt, peak_bandwidth_estimate) = rtt_us * bps / 1_000_000
        let max_win = self.smoothed_rtt.ticks() * self.peak_bandwidth_estimate / 1_000_000;
        let min_win = max_win / 2;
        if min_win > self.cwin {
            min_win
        } else {
            self.cwin
        }
    }

    fn update_cwin_for_long_rtt(&self) -> u64 {
        let rtt_cap = if self.rtt_min > TARGET_SATELLITE_RTT {
            TARGET_SATELLITE_RTT
        } else {
            self.rtt_min
        };
        let min_cwnd =
            (CWIN_INITIAL as f64 * rtt_cap.ticks() as f64 / TARGET_RENO_RTT.ticks() as f64) as u64;
        if min_cwnd > self.cwin {
            min_cwnd
        } else {
            self.cwin
        }
    }
}
```
