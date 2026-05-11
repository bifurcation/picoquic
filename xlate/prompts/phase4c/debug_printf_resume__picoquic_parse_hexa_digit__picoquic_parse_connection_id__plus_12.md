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

## Pair `picoquic/util.c:debug_printf_resume`
C: `picoquic/util.c:221-224 debug_printf_resume`
Rust: `rs/fq/src/utils.rs:197-199 debug_printf_resume`

### C body
```c
{
    debug_suspended = 0;
}
```

### Rust body
```rust
pub fn debug_printf_resume() {
    DEBUG_SUSPENDED.with(|s| s.set(false));
}
```

## Pair `picoquic/util.c:picoquic_parse_hexa_digit`
C: `picoquic/util.c:270-284 picoquic_parse_hexa_digit`
Rust: `rs/fq/src/utils.rs:239-246 parse_hexa_digit`

### C body
```c
int picoquic_parse_hexa_digit(char x) {
    int ret = -1;

    if (x >= '0' && x <= '9') {
        ret = x - '0';
    }
    else if (x >= 'A' && x <= 'F') {
        ret = x - 'A' + 10;
    }
    else if (x >= 'a' && x <= 'f') {
        ret = x - 'a' + 10;
    }

    return ret;
}
```

### Rust body
```rust
pub fn parse_hexa_digit(x: char) -> i32 {
    match x {
        '0'..='9' => x as i32 - '0' as i32,
        'A'..='F' => x as i32 - 'A' as i32 + 10,
        'a'..='f' => x as i32 - 'a' as i32 + 10,
        _ => -1,
    }
}
```

## Pair `picoquic/util.c:picoquic_parse_connection_id`
C: `picoquic/util.c:333-343 picoquic_parse_connection_id`
Rust: `rs/fq/src/utils.rs:338-349 parse_connection_id`

### C body
```c
{
    if (len <= PICOQUIC_CONNECTION_ID_MAX_SIZE) {
        cnx_id->id_len = len;
        memcpy(cnx_id->id, bytes, len);
    } else {
        len = 0;
        cnx_id->id_len = 0;
    }
    return len;
}
```

### Rust body
```rust
pub fn parse_connection_id(bytes: &[u8]) -> Result<ConnectionId, Error> {
    let len = bytes.len();
    if len > CONNECTION_ID_MAX_SIZE {
        return Err(Error::InvalidArgument);
    }
    let mut id = [0u8; CONNECTION_ID_MAX_SIZE];
    id[..len].copy_from_slice(bytes);
    Ok(ConnectionId {
        id,
        id_len: len as u8,
    })
}
```

## Pair `picoquic/util.c:picoquic_val64_connection_id`
C: `picoquic/util.c:388-409 picoquic_val64_connection_id`
Rust: `rs/fq/src/lib.rs:399-405 val64`

### C body
```c
{
    uint64_t val64 = 0;

    if (cnx_id.id_len < 8)
    {
        for (size_t i = 0; i < cnx_id.id_len; i++) {
            val64 <<= 8;
            val64 |= cnx_id.id[i];
        }
        for (size_t i = cnx_id.id_len; i < 8; i++) {
            val64 <<= 8;
        }
    } else {
        for (size_t i = 0; i < 8; i++) {
            val64 <<= 8;
            val64 |= cnx_id.id[i];
        }
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

## Pair `picoquic/util.c:picoquic_get_addr_port`
C: `picoquic/util.c:510-515 picoquic_get_addr_port`
Rust: `rs/fq/src/utils.rs:501-511 get_addr_port`

### C body
```c
{
    uint16_t port = (addr->sa_family == AF_INET6) ? ((struct sockaddr_in6*)addr)->sin6_port : ((struct sockaddr_in*)addr)->sin_port;

    return port;
}
```

### Rust body
```rust
pub fn set_addr_port(addr: &mut SocketAddr, port: u16) {
    addr.set_port(port);
}
```

## Pair `picoquic/util.c:picoquic_store_addr`
C: `picoquic/util.c:551-562 picoquic_store_addr`
Rust: `rs/fq/src/utils.rs:532-534 store_addr`

### C body
```c
{
    int len = 0;

    if (addr == NULL || (len = picoquic_addr_length(addr)) == 0) {
        stored_addr->ss_family = 0;
    }
    else {
        memcpy(stored_addr, addr, len);
    }
}
```

### Rust body
```rust
pub fn store_addr(addr: Option<&SocketAddr>) -> Option<SocketAddr> {
    addr.copied()
}
```

## Pair `picoquic/util.c:picoquic_store_loopback_addr`
C: `picoquic/util.c:640-651 picoquic_store_loopback_addr`
Rust: `rs/fq/src/utils.rs:580-595 store_loopback_addr`

### C body
```c
{
    int ret = -1;
    if (addr_family == AF_INET) {
        ret = picoquic_store_text_addr(stored_addr, "127.0.0.1", port);
    }
    else if (addr_family == AF_INET6) {
        ret = picoquic_store_text_addr(stored_addr, "::1", port);
    }
    return ret;
}
```

### Rust body
```rust
pub fn store_loopback_addr(addr_family: i32, port: u16) -> Result<SocketAddr, Error> {
    // AF_INET=2 is universal; AF_INET6=10 on Linux, 30 on macOS.
    const AF_INET: i32 = 2;
    #[cfg(target_os = "macos")]
    const AF_INET6: i32 = 30;
    #[cfg(not(target_os = "macos"))]
    const AF_INET6: i32 = 10;

    if addr_family == AF_INET {
        store_text_addr("127.0.0.1", port)
    } else if addr_family == AF_INET6 {
        store_text_addr("::1", port)
    } else {
        Err(Error::InvalidArgument)
    }
}
```

## Pair `picoquic/util.c:picoquic_file_delete`
C: `picoquic/util.c:765-782 picoquic_file_delete`
Rust: `rs/fq/src/utils.rs:681-683 file_delete`

### C body
```c
{
    int ret;

#ifdef _WINDOWS
    ret = _unlink(file_name);
    if (last_err != NULL && ret != 0) {
        *last_err = errno;
    }
#else
    ret = unlink(file_name);
    if (last_err != NULL && ret != 0) {
        *last_err = errno;
    }
#endif
    return ret;
}
```

### Rust body
```rust
pub fn file_delete(file_name: &(impl AsRef<std::path::Path> + ?Sized)) -> Result<(), i32> {
    std::fs::remove_file(file_name).map_err(|e| e.raw_os_error().unwrap_or(-1))
}
```

## Pair `picoquic/util.c:picoquic_frames_varlen_decode`
C: `picoquic/util.c:829-835 picoquic_frames_varlen_decode`
Rust: `rs/fq/src/utils.rs:730-734 frames_varlen_decode`

### C body
```c
{
    uint64_t len = 0;
    bytes = picoquic_frames_varint_decode(bytes, bytes_max, &len);
    *n = (size_t)len;
    return (*n == len) ? bytes : NULL;
}
```

### Rust body
```rust
pub fn frames_varlen_decode(bytes: &[u8]) -> Option<(&[u8], usize)> {
    let (rest, len) = frames_varint_decode(bytes)?;
    let n = usize::try_from(len).ok()?;
    Some((rest, n))
}
```

## Pair `picoquic/util.c:picoquic_frames_uint64_decode`
C: `picoquic/util.c:873-883 picoquic_frames_uint64_decode`
Rust: `rs/fq/src/utils.rs:761-767 frames_uint64_decode`

### C body
```c
{
    if (bytes + sizeof(*n) <= bytes_max) {
        *n = PICOPARSE_64(bytes);
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
pub fn frames_uint64_decode(bytes: &[u8]) -> Option<(&[u8], u64)> {
    if bytes.len() < 8 {
        return None;
    }
    let n = u64::from_be_bytes(bytes[..8].try_into().unwrap());
    Some((&bytes[8..], n))
}
```

## Pair `picoquic/util.c:picoquic_frames_varint_encode`
C: `picoquic/util.c:932-983 picoquic_frames_varint_encode`
Rust: `rs/fq/src/utils.rs:813-817 frames_varint_encode`

### C body
```c
{
    if (n64 < 16384) {
        if (n64 < 64) {
            if (bytes + 1 <= bytes_max) {
                *bytes++ = (uint8_t)(n64);
            }
            else {
                bytes = NULL;
            }
        }
        else {
            if (bytes + 2 <= bytes_max) {
                *bytes++ = (uint8_t)((n64 >> 8) | 0x40);
                *bytes++ = (uint8_t)(n64);
            }
            else {
                bytes = NULL;
            }
        }
    }
    else if (n64 < 1073741824) {
        if (bytes + 4 <= bytes_max) {
            *bytes++ = (uint8_t)((n64 >> 24) | 0x80);
            *bytes++ = (uint8_t)(n64 >> 16);
            *bytes++ = (uint8_t)(n64 >> 8);
            *bytes++ = (uint8_t)(n64);
        }
        else {
            bytes = NULL;
        }
    }
    else {
        if (bytes + 8 <= bytes_max) {
            *bytes++ = (uint8_t)((n64 >> 56) | 0xC0);
            *bytes++ = (uint8_t)(n64 >> 48);
            *bytes++ = (uint8_t)(n64 >> 40);
            *bytes++ = (uint8_t)(n64 >> 32);
            *bytes++ = (uint8_t)(n64 >> 24);
            *bytes++ = (uint8_t)(n64 >> 16);
            *bytes++ = (uint8_t)(n64 >> 8);
            *bytes++ = (uint8_t)(n64);
        }
        else {
            bytes = NULL;
        }
    }

    return bytes;
}
```

### Rust body
```rust
        if bytes.is_empty() {
            return None;
        }
```

## Pair `picoquic/util.c:picoquic_frames_uint24_encode`
C: `picoquic/util.c:1014-1025 picoquic_frames_uint24_encode`
Rust: `rs/fq/src/utils.rs:879-882 frames_uint24_encode`

### C body
```c
{
    if (bytes + 3 > bytes_max) {
        bytes = NULL;
    }
    else {
        *bytes++ = (uint8_t)(n >> 16);
        *bytes++ = (uint8_t)(n >> 8);
        *bytes++ = (uint8_t)n;
    }
    return (bytes);
}
```

### Rust body
```rust
    if bytes.len() < 3 {
        return None;
    }
```

## Pair `picoquic/util.c:picoquic_frames_cid_encode`
C: `picoquic/util.c:1074-1077 picoquic_frames_cid_encode`
Rust: `rs/fq/src/utils.rs:933-935 frames_cid_encode`

### C body
```c
{
    return picoquic_frames_length_data_encode(bytes, bytes_max, cid->id_len, cid->id);
}
```

### Rust body
```rust
pub fn frames_cid_encode<'a>(bytes: &'a mut [u8], cid: &ConnectionId) -> Option<&'a mut [u8]> {
    frames_length_data_encode(bytes, cid.as_bytes())
}
```

## Pair `picoquic/util.c:picoquic_create_mutex`
C: `picoquic/util.c:1176-1188 picoquic_create_mutex`
Rust: `rs/fq/src/tests/util_test.rs:225-232 threading`

### C body
```c
{
#ifdef _WINDOWS
    int ret = 0;
    *mutex = CreateMutex(NULL, FALSE, NULL);
    if (*mutex == NULL) {
        ret = -1;
    }
#else
    int ret = pthread_mutex_init(mutex, NULL);
#endif
    return ret;
}
```

### Rust body
```rust
    struct ThreadTestData {
        data: u64,
    }
```

## Pair `picoquic/util.c:picoquic_test_gauss_random`
C: `picoquic/util.c:1352-1371 picoquic_test_gauss_random`
Rust: `rs/fq/src/tests/util.rs:88-98 test_gauss_random`

### C body
```c
{
    double dx = 0;

    /* Sum of 12 variables in [0..1], provides
     * average = 6.0, stdev = 3.0 */
    for (int i = 0; i < 12; i++) {
        double d;
        uint64_t r = picoquic_test_random(random_context);
        r ^= r >> 17;
        r ^= r >> 34;
        d = (double)(r & 0x1ffff) + 0.5;
        d /= (double)(0x20000);
        dx += d;
    }

    dx -= 6.0;

    return dx;
}
```

### Rust body
```rust
pub fn test_gauss_random(random_context: &mut u64) -> f64 {
    let mut dx = 0.0f64;
    for _ in 0..12 {
        let mut r = test_random(random_context);
        r ^= r >> 17;
        r ^= r >> 34;
        let d = (r & 0x1ffff) as f64 + 0.5;
        dx += d / (0x20000u64 as f64);
    }
    dx - 6.0
}
```
