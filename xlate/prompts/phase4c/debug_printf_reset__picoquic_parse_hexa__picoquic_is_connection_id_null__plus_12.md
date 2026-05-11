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

## Pair `picoquic/util.c:debug_printf_reset`
C: `picoquic/util.c:226-231 debug_printf_reset`
Rust: `rs/fq/src/utils.rs:205-211 debug_printf_reset`

### C body
```c
{
    int ret = debug_suspended;
    debug_suspended = suspended;
    return ret;
}
```

### Rust body
```rust
pub fn debug_printf_reset(suspended: bool) -> bool {
    DEBUG_SUSPENDED.with(|s| {
        let old = s.get();
        s.set(suspended);
        old
    })
}
```

## Pair `picoquic/util.c:picoquic_parse_hexa`
C: `picoquic/util.c:286-307 picoquic_parse_hexa`
Rust: `rs/fq/src/utils.rs:414-422 parse_hexa`

### C body
```c
{
    size_t ret = 0;
    if (input_length > 0 && (input_length & 1) == 0 && 2*output_max >= input_length) {
        size_t offset = 0;

        while (offset < input_length) {
            int a = picoquic_parse_hexa_digit(hex_input[offset++]);
            int b = picoquic_parse_hexa_digit(hex_input[offset++]);

            if (a < 0 || b < 0) {
                ret = 0;
                break;
            }
            else {
                bin_output[ret++] = (uint8_t)((a << 4) | b);
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    fn hexa_digit(x: u8) -> Option<u8> {
        match x {
            b'0'..=b'9' => Some(x - b'0'),
            b'A'..=b'F' => Some(x - b'A' + 10),
            b'a'..=b'f' => Some(x - b'a' + 10),
            _ => None,
        }
    }
```

## Pair `picoquic/util.c:picoquic_is_connection_id_null`
C: `picoquic/util.c:348-351 picoquic_is_connection_id_null`
Rust: `rs/fq/src/lib.rs:388-395 is_empty`

### C body
```c
{
    return (cnx_id->id_len == 0) ? 1 : 0;
}
```

### Rust body
```rust
    pub fn hash_with_seed(&self, seed: &[u8; 16]) -> u64 {
        crate::siphash::siphash(self.as_bytes(), seed)
    }
```

## Pair `picoquic/util.c:picoquic_hash_addr_bytes`
C: `picoquic/util.c:413-429 picoquic_hash_addr_bytes`
Rust: `rs/fq/src/utils.rs:376-386 hash_addr_bytes`

### C body
```c
{
    size_t l = 0;
    if (addr->sa_family == AF_INET) {
        memcpy(bytes, &((struct sockaddr_in*)addr)->sin_addr, 4);
        l += 4;
        memcpy(bytes + l, &((struct sockaddr_in*)addr)->sin_port, 2);
        l += 2;
    }
    else if (addr->sa_family == AF_INET6) {
        memcpy(bytes, &((struct sockaddr_in6*)addr)->sin6_addr, 16);
        l += 16;
        memcpy(bytes + l, &((struct sockaddr_in6*)addr)->sin6_port, 2);
        l += 2;
    }
    return l;
}
```

### Rust body
```rust
        SocketAddr::V4(a) => {
            let ip = a.ip().octets();
            bytes[l..l + 4].copy_from_slice(&ip);
            l += 4;
            let port = addr.port().to_ne_bytes();
            bytes[l..l + 2].copy_from_slice(&port);
            l += 2;
        }
```

## Pair `picoquic/util.c:picoquic_compare_addr`
C: `picoquic/util.c:517-529 picoquic_compare_addr`
Rust: `rs/fq/src/utils.rs:477-484 compare_addr`

### C body
```c
{
    int ret = picoquic_compare_ip_addr(expected, actual);
    if (ret == 0 && (expected->sa_family == AF_INET6 || expected->sa_family)){
        uint16_t p_ex = picoquic_get_addr_port(expected);
        uint16_t p_ac = picoquic_get_addr_port(actual);

        if (p_ex != p_ac) {
            ret = (p_ex > p_ac) ? 1 : -1;
        }
    }
    return ret;
}
```

### Rust body
```rust
pub fn compare_addr(expected: &SocketAddr, actual: &SocketAddr) -> Ordering {
    let ip_cmp = compare_ip_addr(expected, actual);
    if ip_cmp == Ordering::Equal {
        expected.port().cmp(&actual.port())
    } else {
        ip_cmp
    }
}
```

## Pair `picoquic/util.c:picoquic_get_ip_addr`
C: `picoquic/util.c:564-579 picoquic_get_ip_addr`
Rust: `rs/fq/src/utils.rs:541-547 get_ip_addr`

### C body
```c
{
    if (addr->sa_family == AF_INET) {
        *ip_addr = (uint8_t *)&((struct sockaddr_in *)addr)->sin_addr;
        *ip_addr_len = 4;
    }
    else if(addr->sa_family == AF_INET6) {
        *ip_addr = (uint8_t *)&((struct sockaddr_in6 *)addr)->sin6_addr;
        *ip_addr_len = 16;
    }
    else {
        *ip_addr = NULL;
        *ip_addr_len = 0;
    }
}
```

### Rust body
```rust
pub fn get_ip_addr(_addr: &SocketAddr) -> Option<&[u8]> {
    // `SocketAddr` exposes IP octets only by value (`IpAddr::octets()`),
    // not by reference; safe Rust cannot produce the required borrow.
    // Callers should use `addr.ip()` with `Ipv4Addr::octets()` /
    // `Ipv6Addr::octets()` directly.
    None
}
```

## Pair `picoquic/util.c:picoquic_set_preferred_address`
C: `picoquic/util.c:653-698 picoquic_set_preferred_address`
Rust: `rs/fq/src/utils.rs:606-628 set_preferred_address`

### C body
```c
{
    int ret = 0;
    struct sockaddr_storage v4_addr = { 0 };
    struct sockaddr_storage v6_addr = { 0 };

    memset(preferred, 0, sizeof(picoquic_tp_preferred_address_t));

    if (v4_text != NULL) {
        ret = picoquic_store_text_addr(&v4_addr, v4_text, preferred_port);
    }
    if (ret == 0 && v6_text != NULL) {
        ret = picoquic_store_text_addr(&v6_addr, v6_text, preferred_port);
    }
    if (ret == 0) {
        if (v4_text != NULL) {
            if (v4_addr.ss_family != AF_INET) {
                ret = -1;
            }
            else {
                memcpy(preferred->ipv4Address, &((struct sockaddr_in*)&v4_addr)->sin_addr, 4);
                preferred->ipv4Port = ((struct sockaddr_in*)&v4_addr)->sin_port;
                if (preferred->ipv4Port == 0) {
                    preferred->ipv4Port = preferred_port;
                }
                preferred->is_defined |= 1;
            }
        }
        if (v6_text != NULL) {
            if (v6_addr.ss_family != AF_INET6) {
                ret = -1;
            }
            else {
                memcpy(preferred->ipv6Address, &((struct sockaddr_in6*)&v6_addr)->sin6_addr, 16);
                preferred->ipv6Port = ((struct sockaddr_in6*)&v6_addr)->sin6_port;
                if (preferred->ipv6Port == 0) {
                    preferred->ipv6Port = preferred_port;
                }
                preferred->is_defined |= 1;
            }
        }
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    *preferred = PreferredAddress::default();
    if let Some(v4) = v4_text {
        let addr = store_text_addr(v4, preferred_port)?;
        if !matches!(addr, SocketAddr::V4(_)) {
            return Err(Error::InvalidArgument);
        }
        preferred.v4 = Some(addr);
    }
    if let Some(v6) = v6_text {
        let addr = store_text_addr(v6, preferred_port)?;
        if !matches!(addr, SocketAddr::V6(_)) {
            return Err(Error::InvalidArgument);
        }
        preferred.v6 = Some(addr);
    }
    Ok(())
}
```

## Pair `picoquic/util.c:picoquic_frames_fixed_skip`
C: `picoquic/util.c:788-792 picoquic_frames_fixed_skip`
Rust: `rs/fq/src/utils.rs:699-702 frames_fixed_skip`

### C body
```c
{
    /* Write this test so as to avoid integer overflows, especially on 32 bit arch. */
    return size <= (uint64_t)(bytes_max - bytes) ? (bytes + size) : NULL;
}
```

### Rust body
```rust
pub fn frames_fixed_skip(bytes: &[u8], size: u64) -> Option<&[u8]> {
    let size = usize::try_from(size).ok()?;
    bytes.get(size..)
}
```

## Pair `picoquic/util.c:picoquic_frames_uint8_decode`
C: `picoquic/util.c:837-846 picoquic_frames_uint8_decode`
Rust: `rs/fq/src/utils.rs:737-740 frames_uint8_decode`

### C body
```c
{
    if (bytes < bytes_max) {
        *n = *bytes++;
    }
    else {
        bytes = NULL;
    }
    return bytes;
}
```

### Rust body
```rust
pub fn frames_uint8_decode(bytes: &[u8]) -> Option<(&[u8], u8)> {
    let (&n, rest) = bytes.split_first()?;
    Some((rest, n))
}
```

## Pair `picoquic/util.c:picoquic_frames_length_data_skip`
C: `picoquic/util.c:885-892 picoquic_frames_length_data_skip`
Rust: `rs/fq/src/utils.rs:771-774 frames_length_data_skip`

### C body
```c
{
    uint64_t length;
    if ((bytes = picoquic_frames_varint_decode(bytes, bytes_max, &length)) != NULL) {
        bytes = picoquic_frames_fixed_skip(bytes, bytes_max, length);
    }
    return bytes;
}
```

### Rust body
```rust
pub fn frames_length_data_skip(bytes: &[u8]) -> Option<&[u8]> {
    let (rest, length) = frames_varint_decode(bytes)?;
    frames_fixed_skip(rest, length)
}
```

## Pair `picoquic/util.c:picoquic_frames_varlen_encode`
C: `picoquic/util.c:985-988 picoquic_frames_varlen_encode`
Rust: `rs/fq/src/utils.rs:854-865 frames_varlen_encode`

### C body
```c
{
    return picoquic_frames_varint_encode(bytes, bytes_max, n);
}
```

### Rust body
```rust
pub fn frames_uint8_encode(bytes: &mut [u8], n: u8) -> Option<&mut [u8]> {
    if bytes.is_empty() {
        return None;
    }
    bytes[0] = n;
    Some(&mut bytes[1..])
}
```

## Pair `picoquic/util.c:picoquic_frames_uint32_encode`
C: `picoquic/util.c:1027-1039 picoquic_frames_uint32_encode`
Rust: `rs/fq/src/utils.rs:890-893 frames_uint32_encode`

### C body
```c
{
    if (bytes + sizeof(n) > bytes_max) {
        bytes = NULL;
    }
    else {
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
    if bytes.len() < 4 {
        return None;
    }
```

## Pair `picoquic/util.c:picoquic_frames_charz_encode`
C: `picoquic/util.c:1079-1089 picoquic_frames_charz_encode`
Rust: `rs/fq/src/utils.rs:939-941 frames_charz_encode`

### C body
```c
{
    if (s == NULL) {
        bytes = picoquic_frames_varlen_encode(bytes, bytes_max, 0);
    }
    else {
        size_t l = strlen(s);
        bytes = picoquic_frames_length_data_encode(bytes, bytes_max, l, (const uint8_t*)s);
    }
    return bytes;
}
```

### Rust body
```rust
pub fn frames_charz_encode<'a>(bytes: &'a mut [u8], s: &str) -> Option<&'a mut [u8]> {
    frames_length_data_encode(bytes, s.as_bytes())
}
```

## Pair `picoquic/util.c:picoquic_test_random`
C: `picoquic/util.c:1312-1320 picoquic_test_random`
Rust: `rs/fq/src/tests/util.rs:44-50 test_random`

### C body
```c
{
    uint64_t z;
    *random_context += 0x9e3779b97f4a7c15;
    z = *random_context;
    z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ull;
    z = (z ^ (z >> 27)) * 0x94d049bb133111ebull;
    return z ^ (z >> 31);
}
```

### Rust body
```rust
pub fn test_random(random_context: &mut u64) -> u64 {
    *random_context = random_context.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *random_context;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9u64);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111ebu64);
    z ^ (z >> 31)
}
```

## Pair `picoquic/util.c:picoquic_test_poisson_random`
C: `picoquic/util.c:1390-1404 picoquic_test_poisson_random`
Rust: `rs/fq/src/tests/util.rs:103-116 test_poisson_random`

### C body
```c
{
    uint64_t k = 0;
    uint64_t p = 0x40000000;

    do {
        uint64_t r = picoquic_test_random(random_context);
        r ^= r >> 30;
        r &= 0x3fffffff;
        p = (p * r) >> 30;
        k = k + 1;
    } while (p > exp_minus_lambda_2_30);

    return (k - 1);
}
```

### Rust body
```rust
pub fn test_poisson_random(random_context: &mut u64, exp_minus_lambda_2_30: u64) -> u64 {
    let mut k = 0u64;
    let mut p = 0x40000000u64;
    loop {
        let r = test_random(random_context);
        let r = (r ^ (r >> 30)) & 0x3fffffff;
        p = (p * r) >> 30;
        k += 1;
        if p <= exp_minus_lambda_2_30 {
            break;
        }
    }
    k - 1
}
```
