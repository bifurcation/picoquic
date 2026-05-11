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

## Pair `picoquic/util.c:debug_printf_suspend`
C: `picoquic/util.c:216-219 debug_printf_suspend`
Rust: `rs/fq/src/utils.rs:191-193 debug_printf_suspend`

### C body
```c
{
    debug_suspended = 1;
}
```

### Rust body
```rust
pub fn debug_printf_suspend() {
    DEBUG_SUSPENDED.with(|s| s.set(true));
}
```

## Pair `picoquic/util.c:picoquic_print_connection_id_hexa`
C: `picoquic/util.c:253-268 picoquic_print_connection_id_hexa`
Rust: `rs/fq/src/utils.rs:464-472 print_connection_id_hexa`

### C body
```c
{
    static const char hex_to_char[16] = { '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f' };
    if (buf_len < ((size_t)cnxid->id_len) * 2u + 1u) {
        return -1;  
    }

    for (unsigned i = 0; i < cnxid->id_len; i++) {
        buf[i * 2u] = hex_to_char[cnxid->id[i] >> 4];
        buf[i * 2u + 1u] = hex_to_char[cnxid->id[i] & 0x0f];
    }

    buf[cnxid->id_len * 2u] = 0;

    return 0;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    for b in connection_id.as_bytes() {
        write!(w, "{:02x}", b).map_err(|_| Error::Generic)?;
    }
    Ok(())
}
```

## Pair `picoquic/util.c:picoquic_format_connection_id`
C: `picoquic/util.c:321-331 picoquic_format_connection_id`
Rust: `rs/fq/src/utils.rs:319-327 format_connection_id`

### C body
```c
{
    uint8_t copied = cnx_id.id_len;
    if (copied > bytes_max || copied == 0) {
        copied = 0;
    } else {
        memcpy(bytes, cnx_id.id, copied);
    }

    return copied;
}
```

### Rust body
```rust
pub fn format_connection_id(bytes: &mut [u8], cnx_id: ConnectionId) -> u8 {
    let id_bytes = cnx_id.as_bytes();
    let copied = id_bytes.len();
    if copied == 0 || copied > bytes.len() {
        return 0;
    }
    bytes[..copied].copy_from_slice(id_bytes);
    copied as u8
}
```

## Pair `picoquic/util.c:picoquic_connection_id_hash`
C: `picoquic/util.c:367-386 picoquic_connection_id_hash`
Rust: `rs/fq/src/lib.rs:393-405 hash_with_seed`

### C body
```c
{
    uint64_t val64 = 0;
    size_t i = 0;

    for (; i < cid->id_len && i < 8; i++) {
        val64 <<= 8;
        val64 += cid->id[i];
    }

    for (; i < cid->id_len; i++) {
        uint64_t top = val64 >> 56;
        val64 <<= 8;
        val64 += cid->id[i];
        val64 += top * 0x10001;
    }

    return val64;
}
```

### Rust body
```rust
    pub fn val64(&self) -> u64 {
        let bytes = self.as_bytes();
        let len = bytes.len().min(8);
        let mut buf = [0u8; 8];
        buf[..len].copy_from_slice(&bytes[..len]);
        u64::from_be_bytes(buf)
    }
```

## Pair `picoquic/util.c:picoquic_compare_ip_addr`
C: `picoquic/util.c:473-508 picoquic_compare_ip_addr`
Rust: `rs/fq/src/utils.rs:489-496 compare_ip_addr`

### C body
```c
{
    int ret = 0;

    if (expected->sa_family == actual->sa_family) {
        if (expected->sa_family == AF_UNSPEC) {
            ret = 0;
        }
        else if (expected->sa_family == AF_INET6) {
            struct sockaddr_in6* ex = (struct sockaddr_in6*)expected;
            struct sockaddr_in6* ac = (struct sockaddr_in6*)actual;

            ret = memcmp(&ex->sin6_addr, &ac->sin6_addr, 16);
        }
        else if (expected->sa_family == AF_INET) {
            struct sockaddr_in* ex = (struct sockaddr_in*)expected;
            struct sockaddr_in* ac = (struct sockaddr_in*)actual;
#if 1
            ret = memcmp(&ex->sin_addr, &ac->sin_addr, 4);
#else
#ifdef _WINDOWS
            ret = (ex->sin_addr.S_un.S_addr == ac->sin_addr.S_un.S_addr);
#else
            ret = (ex->sin_addr.s_addr == ac->sin_addr.s_addr) ? 0 : -1;
#endif
#endif
        }
        else {
            ret = 0;
        }
    }
    else {
        ret = (expected->sa_family > actual->sa_family) ? 1 : -1;
    }
    return ret;
}
```

### Rust body
```rust
    match (expected, actual) {
        (SocketAddr::V4(ex), SocketAddr::V4(ac)) => ex.ip().octets().cmp(&ac.ip().octets()),
        (SocketAddr::V6(ex), SocketAddr::V6(ac)) => ex.ip().octets().cmp(&ac.ip().octets()),
        // AF_INET (2) < AF_INET6 (10 on Linux), so V4 < V6.
        (SocketAddr::V4(_), SocketAddr::V6(_)) => Ordering::Less,
        (SocketAddr::V6(_), SocketAddr::V4(_)) => Ordering::Greater,
    }
```

## Pair `picoquic/util.c:picoquic_addr_length`
C: `picoquic/util.c:541-550 picoquic_addr_length`
Rust: `rs/fq/src/utils.rs:518-534 addr_length`

### C body
```c
{
    int len = 0;
    if (addr->sa_family == AF_INET) {
        len = (int)sizeof(struct sockaddr_in);
    } else if (addr->sa_family == AF_INET6) {
        len = (int)sizeof(struct sockaddr_in6);
    }
    return len;
}
```

### Rust body
```rust
pub fn store_addr(addr: Option<&SocketAddr>) -> Option<SocketAddr> {
    addr.copied()
}
```

## Pair `picoquic/util.c:picoquic_addr_text`
C: `picoquic/util.c:608-638 picoquic_addr_text`
Rust: `rs/fq/src/utils.rs:565-570 addr_text`

### C body
```c
{
    char addr_buffer[128];
    char const* addr_text;
    char const* ret_text = "?:?";

    if (addr != NULL) {
        switch (addr->sa_family) {
        case AF_INET:
            addr_text = inet_ntop(AF_INET,
                (const void*)(&((struct sockaddr_in*)addr)->sin_addr),
                addr_buffer, sizeof(addr_buffer));
            if (picoquic_sprintf(text, text_size, NULL, "%s:%d", addr_text, ((struct sockaddr_in*)addr)->sin_port) == 0) {
                ret_text = text;
            }
            break;
        case AF_INET6:
            addr_text = inet_ntop(AF_INET6,
                (const void*)(&((struct sockaddr_in6*)addr)->sin6_addr),
                addr_buffer, sizeof(addr_buffer));
            if (picoquic_sprintf(text, text_size, NULL, "[%s]:%d", addr_text, ((struct sockaddr_in6*)addr)->sin6_port) == 0) {
                ret_text = text;
            }
        default:
            break;
        }
    }

    return ret_text;
}
```

### Rust body
```rust
pub fn addr_text(addr: &SocketAddr, w: &mut dyn core::fmt::Write) -> Result<(), core::fmt::Error> {
    match addr {
        SocketAddr::V4(a) => write!(w, "{}:{}", a.ip(), a.port()),
        SocketAddr::V6(a) => write!(w, "[{}]:{}", a.ip(), a.port()),
    }
}
```

## Pair `picoquic/util.c:picoquic_get_input_path`
C: `picoquic/util.c:708-724 picoquic_get_input_path`
Rust: `rs/fq/src/utils.rs:653-665 get_input_path`

### C body
```c
{
    if (solution_path == NULL) {
        solution_path = PICOQUIC_DEFAULT_SOLUTION_DIR;
    }

    const char * separator = PICOQUIC_FILE_SEPARATOR;
    size_t solution_path_length = strlen(solution_path);
    if (solution_path_length != 0 && solution_path[solution_path_length - 1] == separator[0]) {
        separator = "";
    }

    int ret = picoquic_sprintf(target_file_path, file_path_max, NULL, "%s%s%s",
        solution_path, separator, file_name);

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    let solution_path = solution_path.unwrap_or(DEFAULT_SOLUTION_DIR);
    let sep = if solution_path.ends_with(FILE_SEPARATOR) {
        ""
    } else {
        FILE_SEPARATOR
    };
    write!(target_file_path, "{}{}{}", solution_path, sep, file_name).map_err(|_| Error::Generic)
}
```

## Pair `picoquic/util.c:picoquic_frames_varint_decode`
C: `picoquic/util.c:807-827 picoquic_frames_varint_decode`
Rust: `rs/fq/src/internal.rs:6419-6434 frames_varint_decode`

### C body
```c
{
    uint8_t length;

    if (bytes < bytes_max && bytes + (length = VARINT_LEN_T(bytes, uint8_t)) <= bytes_max) {
        uint64_t v = *bytes++ & 0x3F;

        while (--length > 0) {
            v <<= 8;
            v += *bytes++;
        }

        *n64 = v;
    }
    else {
        bytes = NULL;
    }

    return bytes;
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

## Pair `picoquic/util.c:picoquic_frames_uint32_decode`
C: `picoquic/util.c:861-871 picoquic_frames_uint32_decode`
Rust: `rs/fq/src/utils.rs:752-758 frames_uint32_decode`

### C body
```c
{
    if (bytes + sizeof(*n) <= bytes_max) {
        *n = PICOPARSE_32(bytes);
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
pub fn frames_uint32_decode(bytes: &[u8]) -> Option<(&[u8], u32)> {
    if bytes.len() < 4 {
        return None;
    }
    let n = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    Some((&bytes[4..], n))
}
```

## Pair `picoquic/util.c:picoquic_frames_varint_encode_length`
C: `picoquic/util.c:911-929 picoquic_frames_varint_encode_length`
Rust: `rs/fq/src/internal.rs:6475-6483 frames_varint_encode_length`

### C body
```c
{
    size_t len = 8;

    if (n64 < 16384) {
        if (n64 < 64) {
            len = 1;
        }
        else {
            len = 2;
        }
    }
    else if (n64 < 1073741824) {
        len = 4;
    }

    return len;
}
```

### Rust body
```rust
pub fn decode_varint_length(byte: u8) -> usize {
    1usize << ((byte & 0xC0) >> 6)
}
```

## Pair `picoquic/util.c:picoquic_frames_uint16_encode`
C: `picoquic/util.c:1002-1012 picoquic_frames_uint16_encode`
Rust: `rs/fq/src/utils.rs:868-871 frames_uint16_encode`

### C body
```c
{
    if (bytes + sizeof(n) > bytes_max) {
        bytes = NULL;
    }
    else {
        *bytes++ = (uint8_t)(n >> 8);
        *bytes++ = (uint8_t)n;
    }
    return (bytes);
}
```

### Rust body
```rust
    if bytes.len() < 2 {
        return None;
    }
```

## Pair `picoquic/util.c:picoquic_frames_length_data_encode`
C: `picoquic/util.c:1060-1072 picoquic_frames_length_data_encode`
Rust: `rs/fq/src/utils.rs:921-929 frames_length_data_encode`

### C body
```c
{
    if ((bytes = picoquic_frames_varlen_encode(bytes, bytes_max, l)) != NULL &&
        (bytes + l) <= bytes_max) {
        memcpy(bytes, v, l);
        bytes += l;
    }
    else {
        bytes = NULL;
    }

    return bytes;
}
```

### Rust body
```rust
pub fn frames_length_data_encode<'a>(bytes: &'a mut [u8], v: &[u8]) -> Option<&'a mut [u8]> {
    let l = v.len();
    let rest = frames_varlen_encode(bytes, l)?;
    if rest.len() < l {
        return None;
    }
    rest[..l].copy_from_slice(v);
    Some(&mut rest[l..])
}
```

## Pair `picoquic/util.c:picoquic_set_abs_delay`
C: `picoquic/util.c:1115-1124 picoquic_set_abs_delay`
Rust: `rs/fq/src/utils.rs:266-268 set_abs_delay`

### C body
```c
static void picoquic_set_abs_delay(struct timespec* ts, uint64_t microsec_wait) {
    clock_gettime(CLOCK_REALTIME, ts);
    ts->tv_sec += (unsigned long)(microsec_wait / 1000000);
    ts->tv_nsec += (unsigned long)((microsec_wait % 1000000)*1000);
    if (ts->tv_nsec > 1000000000) {
        ts->tv_sec++;
        ts->tv_nsec -= 1000000000;
    }
}
```

### Rust body
```rust
pub fn set_abs_delay(microsec_wait: u64) -> std::time::SystemTime {
    std::time::SystemTime::now() + std::time::Duration::from_micros(microsec_wait)
}
```

## Pair `picoquic/util.c:picoquic_test_uniform_random`
C: `picoquic/util.c:1336-1350 picoquic_test_uniform_random`
Rust: `rs/fq/src/tests/util.rs:73-84 test_uniform_random`

### C body
```c
{
    uint64_t rnd = 0;

    if (rnd_max > 0) {
        uint64_t rnd_min = UINT64_MAX % rnd_max;

        do {
            rnd = picoquic_test_random(random_context);
        } while (rnd < rnd_min);
        rnd %= rnd_max;
    }

    return rnd;
}
```

### Rust body
```rust
pub fn test_uniform_random(random_context: &mut u64, rnd_max: u64) -> u64 {
    if rnd_max == 0 {
        return 0;
    }
    let rnd_min = u64::MAX % rnd_max;
    loop {
        let rnd = test_random(random_context);
        if rnd >= rnd_min {
            return rnd % rnd_max;
        }
    }
}
```
