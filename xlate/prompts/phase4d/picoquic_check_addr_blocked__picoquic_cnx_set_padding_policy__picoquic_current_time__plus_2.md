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

## `picoquic/port_blocking.c:picoquic_check_addr_blocked`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C extracts a port from IPv4 or IPv6 address and checks whether it is blocked; Rust returns an unspecified address.
* C source: `picoquic/port_blocking.c:146-160`
* C signature: `int picoquic_check_addr_blocked(const struct sockaddr *)`
* Rust source: `rs/fq/src/lib.rs:1239-1245`
* Rust item: `check_addr_blocked`

### C body
```c
{
    /* The sockaddr is always in network order. We must translate to
     * host order before performaing the check */
    uint16_t port = UINT16_MAX;

    if (addr_from->sa_family == AF_INET) {
        port = ntohs(((struct sockaddr_in*)addr_from)->sin_port);
    }
    else if (addr_from->sa_family == AF_INET6) {
        /* configure an IPv6 sockaddr */
        port = ntohs(((struct sockaddr_in6*)addr_from)->sin6_port);
    }
    return picoquic_check_port_blocked(port);
}
```

### Rust body
```rust
pub(crate) fn unspecified_socket_addr() -> SocketAddr {
    SocketAddr::new(core::net::IpAddr::V4(core::net::Ipv4Addr::UNSPECIFIED), 0)
}
```

## `picoquic/quicctx.c:picoquic_cnx_set_padding_policy`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets padding_multiple and padding_minsize; Rust body returns the current padding policy instead of setting it.
* C source: `picoquic/quicctx.c:4519-4524`
* C signature: `void picoquic_cnx_set_padding_policy(picoquic_cnx_t *, uint32_t, uint32_t)`
* Rust source: `rs/fq/src/lib.rs:2840-2850`
* Rust item: `set_padding_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->padding_multiple = padding_multiple;
    cnx->padding_minsize = padding_minsize;
}
```

### Rust body
```rust
    pub fn padding_policy(&self) -> (u32, u32) {
        (self.padding_multiple, self.padding_minsize)
    }
```

## `picoquic/quicctx.c:picoquic_current_time`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body implements platform time retrieval and returns now; Rust body shown is a Quic::time wrapper that just calls current_time().
* C source: `picoquic/quicctx.c:4569-4612`
* C signature: `uint64_t picoquic_current_time(void)`
* Rust source: `rs/fq/src/lib.rs:483-498`
* Rust item: `current_time`

### C body
```c
{
    uint64_t now;
#ifdef _WINDOWS
    FILETIME ft;
    /*
    * The GetSystemTimeAsFileTime API returns  the number
    * of 100-nanosecond intervals since January 1, 1601 (UTC),
    * in FILETIME format.
    */
    GetSystemTimePreciseAsFileTime(&ft);

    /*
    * Convert to plain 64 bit format, without making
    * assumptions about the FILETIME structure alignment.
    */
    now = ft.dwHighDateTime;
    now <<= 32;
    now |= ft.dwLowDateTime;
    /*
    * Convert units from 100ns to 1us
    */
    now /= 10;
    /*
    * Account for microseconds elapsed between 1601 and 1970.
    */
    now -= 11644473600000000ULL;
#elif defined(CLOCK_MONOTONIC)
    /*
    * Use CLOCK_MONOTONIC if exists (more accurate)
    */
    struct timespec currentTime;
    (void)clock_gettime(CLOCK_MONOTONIC, &currentTime);
    now = (currentTime.tv_sec * 1000000ull) + currentTime.tv_nsec / 1000ull;
#else
    struct timeval tv;
    (void)gettimeofday(&tv, NULL);
    now = (tv.tv_sec * 1000000ull) + tv.tv_usec;
#endif
    return now;
}
```

### Rust body
```rust
impl Quic {
    /// Virtual time used by this QUIC context (wall-clock or
    /// simulated, depending on whether a simulated-time pointer was
    /// supplied at creation).  C: `get_quic_time`.
    pub fn time(&self) -> u64 {
        current_time()
    }
}
```

## `picoquic/quicctx.c:picoquic_demote_local_cnxid_list`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C conditionally queues an abandon frame, deletes a remote stash, sets is_demoted only on success, and returns ret; Rust only sets is_demoted and returns 1.
* C source: `picoquic/quicctx.c:2558-2580`
* C signature: `int picoquic_demote_local_cnxid_list(picoquic_cnx_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:13157-13166`
* Rust item: `demote_local_connection_id_list`

### C body
```c
{
    int ret = 0;
    picoquic_local_cnxid_list_t* local_cnxid_list =
        picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 0);

    if (local_cnxid_list != NULL &&
        !local_cnxid_list->is_demoted) {
        if ((ret = picoquic_queue_path_abandon_frame(cnx, unique_path_id, reason)) == 0) {
            picoquic_remote_cnxid_stash_t* remote_cnxid_stash =
                picoquic_find_or_create_remote_cnxid_stash(cnx, unique_path_id, 0);
            if (remote_cnxid_stash != NULL) {
                picoquic_delete_remote_cnxid_stash(cnx, remote_cnxid_stash);
            }
            local_cnxid_list->is_demoted = 1;
        }
        else {
            DBG_PRINTF("Cannot abandon path %" PRIu64, unique_path_id);
        }
    }
    return ret;
}
```

### Rust body
```rust
        {
            list.is_demoted = true;
            return 1;
        }
```

## `picoquic/quicctx.c:picoquic_find_avalaible_unique_path_id`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C computes and returns an available unique path ID; Rust body returns nb_crypto_key_rotations.
* C source: `picoquic/quicctx.c:1651-1684`
* C signature: `uint64_t picoquic_find_avalaible_unique_path_id(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:4937-4945`
* Rust item: `path_unique_id`

### C body
```c
{
    uint64_t unique_path_id = requested_id;

    if (!cnx->is_multipath_enabled) {
        if (requested_id != 0 && requested_id != UINT64_MAX) {
            unique_path_id = UINT64_MAX;
        }
        else {
            unique_path_id = 0;
        }
    }
    else {
        /* Unique path ID are allocated in sequence on the client. The server should
         * always use the number proposed by the client in incoming packets */
        if (requested_id == UINT64_MAX && (cnx->client_mode || cnx->nb_paths == 0)) {
            while (cnx->unique_path_id_next <= cnx->max_path_id_remote &&
                cnx->unique_path_id_next <= cnx->max_path_id_local &&
                cnx->unique_path_id_next <= cnx->max_path_id_in_cnxid_lists) {
                /* Find next non used CID */
                unique_path_id = cnx->unique_path_id_next++;
                /* There should be an available of CNX_ID for this path_id, 
                * and that path_id should not be already created.
                */
                if (picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 0) != NULL &&
                    picoquic_find_path_by_unique_id(cnx, unique_path_id) < 0) {
                    /* this CID was not already deleted */
                    break;
                }
             }
        }
    }
    return unique_path_id;
}
```

### Rust body
```rust
    pub fn nb_crypto_key_rotations(&self) -> u64 {
        self.nb_crypto_key_rotations
    }
```
