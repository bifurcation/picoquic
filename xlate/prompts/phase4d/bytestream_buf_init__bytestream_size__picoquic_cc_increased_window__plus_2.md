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

## `picoquic/bytestream.c:bytestream_buf_init`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C initializes a bounded internal buffer and returns it/null; Rust body is a large impl block with many methods and no visible equivalent max-size check, internal buffer assignment, or return of initialized stream.
* C source: `picoquic/bytestream.c:39-50`
* C signature: `bytestream * bytestream_buf_init(bytestream_buf *, size_t)`
* Rust source: `rs/fq/src/bytestream.rs:119-611`
* Rust item: `stream`

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

## `picoquic/bytestream.c:bytestream_size`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the bytestream size, while the shown Rust body is SackItem range/count accessors and contains no bytestream size return.
* C source: `picoquic/bytestream.c:83-86`
* C signature: `size_t bytestream_size(bytestream *)`
* Rust source: `rs/fq/src/internal.rs:8653-8676`
* Rust item: `size`

### C body
```c
{
    return s->size;
}
```

### Rust body
```rust
impl SackItem {
    /// Inclusive start of this SACK range.  C:
    /// `sack_item_range_start`.
    pub fn range_start(&self) -> u64 {
        self.start_of_sack_range
    }

    /// Exclusive end of this SACK range.  C:
    /// `sack_item_range_end`.
    pub fn range_end(&self) -> u64 {
        self.end_of_sack_range
    }

    /// Number of times this range has been sent in an ACK frame.
    /// C: `sack_item_nb_times_sent`.
    pub fn nb_times_sent(&self, is_opportunistic: i32) -> i32 {
        self.nb_times_sent[is_opportunistic.clamp(0, 1) as usize]
    }
}
```

## `picoquic/cc_common.c:picoquic_cc_increased_window`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is unrelated stream limit frame encoding and returns a byte slice, while C computes and returns a congestion window from previous_window and rtt_min.
* C source: `picoquic/cc_common.c:291-304`
* C signature: `uint64_t picoquic_cc_increased_window(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:12888-12941`
* Rust item: `cc_increased_window`

### C body
```c
{
    uint64_t new_window;
    if (cnx->path[0]->rtt_min <= PICOQUIC_TARGET_RENO_RTT) {
        new_window = previous_window * 2;
    }
    else {
        double w = (double)previous_window;
        w /= (double)PICOQUIC_TARGET_RENO_RTT;
        w *= (cnx->path[0]->rtt_min > PICOQUIC_TARGET_SATELLITE_RTT) ? PICOQUIC_TARGET_SATELLITE_RTT : (double)cnx->path[0]->rtt_min;
        new_window = (uint64_t)w;
    }
    return new_window;
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

## `picoquic/config.c:picoquic_config_usage`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body only writes usage to stderr; Rust body is a test comparing generated usage to a fixture.
* C source: `picoquic/config.c:609-612`
* C signature: `void picoquic_config_usage(void)`
* Rust source: `rs/fq/src/tests/config.rs:582-591`
* Rust item: `config_usage`

### C body
```c
{
    picoquic_config_usage_file(stderr);
}
```

### Rust body
```rust
fn config_usage() {
    config_test_register_cc_algorithms();
    let mut buf = String::new();
    Config::write_usage(&mut buf);
    let expected = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/config_usage_ref.txt"
    ));
    assert_eq!(buf, expected);
}
```

## `picoquic/ech.c:picoquic_base64_decode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is just assigning entries in a table; it does not decode base64, allocate/copy output, set length, or return a decode status.
* C source: `picoquic/ech.c:46-75`
* C signature: `int picoquic_base64_decode(uint8_t **, size_t *, const char *)`
* Rust source: `rs/fq/src/config.rs:607-614`
* Rust item: `base64_decode`

### C body
```c
{
    int ret = 0;
    ptls_buffer_t config;
    uint8_t short_buf[256];
    ptls_base64_decode_state_t d_state;
    *v = NULL;
    *v_len = 0;
    ptls_buffer_init(&config, short_buf, sizeof(short_buf));
    ptls_base64_decode_init(&d_state);
    ret = ptls_base64_decode(b64_txt, &d_state, &config);
    if (ret == 0 && (d_state.status == PTLS_BASE64_DECODE_DONE || (d_state.status == PTLS_BASE64_DECODE_IN_PROGRESS && d_state.nbc == 0))) {
        ret = 0;
        if (config.off > 0) {
            if ((*v = (uint8_t*)malloc(config.off)) == NULL) {
                ret = PICOQUIC_ERROR_MEMORY;
            }
            else {
                memcpy(*v, config.base, config.off);
                *v_len = config.off;
            }
        }
    }
    ptls_buffer_dispose(&config);
    return ret;
}
```

### Rust body
```rust
    {
        table[c as usize] = i as u8;
    }
```
