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

## Pair `picoquic/util.c:picoquic_sprintf`
C: `picoquic/util.c:233-251 picoquic_sprintf`
Rust: `rs/fq/src/utils.rs:285-299 sprintf`

### C body
```c
{
    va_list args;
    va_start(args, fmt);
#ifdef _WINDOWS
    int res = vsnprintf_s(buf, buf_len, _TRUNCATE, fmt, args);
#else
    int res = vsnprintf(buf, buf_len, fmt, args);
#endif
    va_end(args);

    if (nb_chars != NULL) {
        *nb_chars = res;
    }

    // vsnprintf returns <0 for errors and >=0 for nb of characters required.
    // We return 0 when printing was successful.
    return res >= 0 ? ((size_t)res >= buf_len) : res;
}
```

### Rust body
```rust
pub fn sprintf(buf: &mut [u8], msg: &str) -> Result<usize, Error> {
    let bytes = msg.as_bytes();
    if bytes.len() < buf.len() {
        buf[..bytes.len()].copy_from_slice(bytes);
        buf[bytes.len()] = 0;
        Ok(bytes.len())
    } else if !buf.is_empty() {
        let n = buf.len() - 1;
        buf[..n].copy_from_slice(&bytes[..n]);
        buf[n] = 0;
        Err(Error::BufferTooSmall)
    } else {
        Err(Error::BufferTooSmall)
    }
}
```

## Pair `picoquic/util.c:picoquic_parse_connection_id_hexa`
C: `picoquic/util.c:309-319 picoquic_parse_connection_id_hexa`
Rust: `rs/fq/src/utils.rs:448-455 parse_connection_id_hexa`

### C body
```c
{
    memset(cnx_id, 0, sizeof(picoquic_connection_id_t));
    cnx_id->id_len = (uint8_t) picoquic_parse_hexa(hex_input, input_length, cnx_id->id, 18);

    if (cnx_id->id_len == 0) {
        memset(cnx_id, 0, sizeof(picoquic_connection_id_t));
    }

    return (cnx_id->id_len);
}
```

### Rust body
```rust
pub fn parse_connection_id_hexa(hex_input: &str) -> Result<ConnectionId, Error> {
    let mut id = [0u8; CONNECTION_ID_MAX_SIZE];
    let len = parse_hexa(hex_input, &mut id[..18]);
    Ok(ConnectionId {
        id,
        id_len: len as u8,
    })
}
```

## Pair `picoquic/util.c:picoquic_compare_connection_id`
C: `picoquic/util.c:353-365 picoquic_compare_connection_id`
Rust: `rs/fq/src/utils.rs:354-361 compare_connection_id`

### C body
```c
{
    int ret = -1;

    if (cnx_id1->id_len == cnx_id2->id_len) {
        ret = memcmp(cnx_id1->id, cnx_id2->id, cnx_id1->id_len);
    }
    else if (cnx_id1->id_len > cnx_id2->id_len) {
        ret = 1;
    }

    return ret;
}
```

### Rust body
```rust
pub fn compare_connection_id(id1: &ConnectionId, id2: &ConnectionId) -> Ordering {
    let len1 = id1.as_bytes().len();
    let len2 = id2.as_bytes().len();
    match len1.cmp(&len2) {
        Ordering::Equal => id1.as_bytes().cmp(id2.as_bytes()),
        other => other,
    }
}
```

## Pair `picoquic/util.c:picoquic_hash_addr`
C: `picoquic/util.c:431-439 picoquic_hash_addr`
Rust: `rs/fq/src/utils.rs:402-406 hash_addr`

### C body
```c
{
    uint8_t bytes[18];
    size_t l = picoquic_hash_addr_bytes(addr, bytes);

    /* Using siphash, because secret and IP address are chosen by third parties*/
    uint64_t h = picohash_siphash(bytes, (uint32_t)l, hash_seed);
    return h;
}
```

### Rust body
```rust
pub fn hash_addr(addr: &SocketAddr, hash_seed: &[u8; 16]) -> u64 {
    let mut bytes = [0u8; 18];
    let l = hash_addr_bytes(addr, &mut bytes);
    crate::siphash::siphash(&bytes[..l], hash_seed)
}
```

## Pair `picoquic/util.c:picoquic_set_addr_port`
C: `picoquic/util.c:531-539 picoquic_set_addr_port`
Rust: `rs/fq/src/utils.rs:509-511 set_addr_port`

### C body
```c
{
    if (addr->sa_family == AF_INET6) {
        ((struct sockaddr_in6*)addr)->sin6_port = port;
    }
    else {
        ((struct sockaddr_in*)addr)->sin_port = port;
    }
}
```

### Rust body
```rust
pub fn set_addr_port(addr: &mut SocketAddr, port: u16) {
    addr.set_port(port);
}
```

## Pair `picoquic/util.c:picoquic_store_text_addr`
C: `picoquic/util.c:581-606 picoquic_store_text_addr`
Rust: `rs/fq/src/utils.rs:553-558 store_text_addr`

### C body
```c
{
    int ret = 0;
    struct sockaddr_in* ipv4_addr = (struct sockaddr_in*)stored_addr;
    struct sockaddr_in6* ipv6_addr = (struct sockaddr_in6*)stored_addr;

    /* get the IP address of the server */
    memset(stored_addr, 0, sizeof(struct sockaddr_storage));

    if (inet_pton(AF_INET, ip_address_text, &ipv4_addr->sin_addr) == 1) {
        /* Valid IPv4 address */
        ipv4_addr->sin_family = AF_INET;
        ipv4_addr->sin_port = (unsigned short)port;
    }
    else if (inet_pton(AF_INET6, ip_address_text, &ipv6_addr->sin6_addr) == 1) {
        /* Valid IPv6 address */
        ipv6_addr->sin6_family = AF_INET6;
        ipv6_addr->sin6_port = port;
    }
    else {
        ret = -1;
    }

    return ret;
}
```

### Rust body
```rust
pub fn store_text_addr(ip_address_text: &str, port: u16) -> Result<SocketAddr, Error> {
    let ip: IpAddr = ip_address_text
        .parse()
        .map_err(|_| Error::InvalidArgument)?;
    Ok(SocketAddr::new(ip, port))
}
```

## Pair `picoquic/util.c:picoquic_set_solution_dir`
C: `picoquic/util.c:703-706 picoquic_set_solution_dir`
Rust: `rs/fq/src/utils.rs:636-639 set_solution_dir`

### C body
```c
{
    picoquic_solution_dir = solution_dir;
}
```

### Rust body
```rust
pub fn set_solution_dir(solution_dir: Option<&str>) {
    let mut lock = SOLUTION_DIR.lock().unwrap();
    *lock = solution_dir.map(|s| -> &'static str { Box::leak(s.to_owned().into_boxed_str()) });
}
```

## Pair `picoquic/util.c:picoquic_frames_varint_skip`
C: `picoquic/util.c:795-804 picoquic_frames_varint_skip`
Rust: `rs/fq/src/internal.rs:6438-6441 frames_varint_skip`

### C body
```c
{
    if (bytes < bytes_max) {
        uint8_t v_len = VARINT_LEN(bytes);
        return  picoquic_frames_fixed_skip(bytes, bytes_max, v_len);
    }
    else {
        return NULL;
    }
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## Pair `picoquic/util.c:picoquic_frames_uint16_decode`
C: `picoquic/util.c:849-859 picoquic_frames_uint16_decode`
Rust: `rs/fq/src/utils.rs:743-749 frames_uint16_decode`

### C body
```c
{
    if (bytes + sizeof(*n) <= bytes_max) {
        *n = PICOPARSE_16(bytes);
        bytes += sizeof(*n);
    }
    else {
        bytes = NULL;
    }
    return bytes;
}
```

### Rust body
```rust
pub fn frames_uint16_decode(bytes: &[u8]) -> Option<(&[u8], u16)> {
    if bytes.len() < 2 {
        return None;
    }
    let n = u16::from_be_bytes([bytes[0], bytes[1]]);
    Some((&bytes[2..], n))
}
```

## Pair `picoquic/util.c:picoquic_frames_cid_decode`
C: `picoquic/util.c:894-909 picoquic_frames_cid_decode`
Rust: `rs/fq/src/utils.rs:779-787 frames_cid_decode`

### C body
```c
{
    bytes = picoquic_frames_uint8_decode(bytes, bytes_max, &cid->id_len);

    if (cid->id_len > PICOQUIC_CONNECTION_ID_MAX_SIZE ||
        bytes + cid->id_len > bytes_max) {
        bytes = NULL;
    }
    else {
        memset(cid->id, 0, sizeof(cid->id));
        memcpy(cid->id, bytes, cid->id_len);
        bytes += cid->id_len;
    }

    return bytes;
}
```

### Rust body
```rust
pub fn frames_cid_decode(bytes: &[u8]) -> Option<(&[u8], ConnectionId)> {
    let (rest, id_len) = frames_uint8_decode(bytes)?;
    if id_len as usize > CONNECTION_ID_MAX_SIZE || rest.len() < id_len as usize {
        return None;
    }
    let mut id = [0u8; CONNECTION_ID_MAX_SIZE];
    id[..id_len as usize].copy_from_slice(&rest[..id_len as usize]);
    Some((&rest[id_len as usize..], ConnectionId { id, id_len }))
}
```

## Pair `picoquic/util.c:picoquic_frames_uint8_encode`
C: `picoquic/util.c:990-1000 picoquic_frames_uint8_encode`
Rust: `rs/fq/src/utils.rs:859-862 frames_uint8_encode`

### C body
```c
{
    if (bytes + sizeof(n) > bytes_max) {
        bytes = NULL;
    }
    else {
        *bytes++ = n;
    }

    return (bytes);
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## Pair `picoquic/util.c:picoquic_frames_uint64_encode`
C: `picoquic/util.c:1041-1058 picoquic_frames_uint64_encode`
Rust: `rs/fq/src/utils.rs:902-905 frames_uint64_encode`

### C body
```c
{
    if (bytes + sizeof(n) > bytes_max) {
        bytes = NULL;
    }
    else {
        *bytes++ = (uint8_t)(n >> 56);
        *bytes++ = (uint8_t)(n >> 48);
        *bytes++ = (uint8_t)(n >> 40);
        *bytes++ = (uint8_t)(n >> 32);
        *bytes++ = (uint8_t)(n >> 24);
        *bytes++ = (uint8_t)(n >> 16);
        *bytes++ = (uint8_t)(n >> 8);
        *bytes++ = (uint8_t)n;
    }
    return (bytes);

}
```

### Rust body
```rust
    if bytes.len() < 8 {
        return None;
    }
```

## Pair `picoquic/util.c:picoquic_constant_time_memcmp`
C: `picoquic/util.c:1100-1110 picoquic_constant_time_memcmp`
Rust: `rs/fq/src/utils.rs:995-999 constant_time_memcmp`

### C body
```c
{
    uint64_t ret = 0;

    while (l > 0) {
        ret += (*x++ ^ *y++);
        l--;
    }

    return (ret == 0)?0:-1;
}
```

### Rust body
```rust
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        acc += (xi ^ yi) as u64;
    }
```

## Pair `picoquic/util.c:picoquic_test_random_bytes`
C: `picoquic/util.c:1322-1334 picoquic_test_random_bytes`
Rust: `rs/fq/src/tests/util.rs:56-68 test_random_bytes`

### C body
```c
{
    size_t byte_index = 0;

    while (byte_index < bytes_max) {
        uint64_t v = picoquic_test_random(random_context);

        for (int i = 0; i < 8 && byte_index < bytes_max; i++) {
            bytes[byte_index++] = v & 0xFF;
            v >>= 8;
        }
    }
}
```

### Rust body
```rust
    while byte_index < bytes.len() {
        let mut v = test_random(random_context);
        for _ in 0..8 {
            if byte_index >= bytes.len() {
                break;
            }
            bytes[byte_index] = (v & 0xFF) as u8;
            byte_index += 1;
            v >>= 8;
        }
    }
```

## Pair `picoquic/util.c:picoquic_uint8_to_str`
C: `picoquic/util.c:1406-1438 picoquic_uint8_to_str`
Rust: `rs/fq/src/utils.rs:1038-1063 uint8_to_str`

### C body
```c
{
    size_t render_length = data_len;
    size_t rendered;

    if (render_length + 1 > text_len) {
        if (text_len > 4) {
            render_length = text_len - 4;
        }
        else {
            render_length = 0;
        }
    }

    for (rendered = 0; rendered < render_length; rendered++) {
        int c = data[rendered];
        if (c < ' ' || c >= 127) {
            c = '?';
        }
        text[rendered] = (char)c;
    }

    if (rendered < data_len) {
        for (size_t i = 0; i < 3 && rendered + 1 < text_len; i++, rendered++) {
            text[rendered] = '.';
        }
    }
    text[rendered] = 0;

    return text;
}
```

### Rust body
```rust
pub fn uint8_to_str<'a>(text: &'a mut [u8], data: &[u8]) -> &'a [u8] {
    let text_len = text.len();
    let data_len = data.len();
    let render_length = if data_len >= text_len {
        text_len.saturating_sub(4)
    } else {
        data_len
    };
    let mut rendered = 0usize;
    for &c in &data[..render_length] {
        text[rendered] = if (b' '..127).contains(&c) { c } else { b'?' };
        rendered += 1;
    }
    if rendered < data_len {
        let mut i = 0usize;
        while i < 3 && rendered + 1 < text_len {
            text[rendered] = b'.';
            rendered += 1;
            i += 1;
        }
    }
    if rendered < text_len {
        text[rendered] = 0;
    }
    &text[..rendered]
}
```
