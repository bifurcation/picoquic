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

## `picoquic/quicctx.c:picoquic_get_ack_number`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns a highest_acknowledged value selected from path or connection packet context; Rust body is get_last_packet, ignores the path argument, and returns the last pending packet token instead.
* C source: `picoquic/quicctx.c:1694-1698`
* C signature: `uint64_t picoquic_get_ack_number(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_context_enum)`
* Rust source: `rs/fq/src/internal.rs:7843-7858`
* Rust item: `get_ack_number`

### C body
```c
{
    return (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) ? path_x->pkt_ctx.highest_acknowledged :
        cnx->pkt_ctx[pc].highest_acknowledged;
}
```

### Rust body
```rust
impl Connection {
    pub fn get_last_packet(&self, _path_x: &mut Path, pc: PacketContext) -> Option<PacketToken> {
        // C: picoquic_get_last_packet — last (highest seq) in pending queue
        self.pkt_ctx[pc as usize]
            .pending
            .values()
            .next_back()
            .copied()
    }
}
```

## `picoquic/quicctx.c:picoquic_get_cwin`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns path[0] congestion window; Rust returns first path smoothed RTT.
* C source: `picoquic/quicctx.c:5432-5436`
* C signature: `uint64_t picoquic_get_cwin(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4666-4677`
* Rust item: `cwin`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->cwin;
}
```

### Rust body
```rust
    pub fn rtt(&self) -> u64 {
        self.paths
            .first()
            .map(|p| p.smoothed_rtt.ticks())
            .unwrap_or(0)
    }
```

## `picoquic/quicctx.c:picoquic_get_local_addr`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body returns whether a path is backup, not the local address of path 0.
* C source: `picoquic/quicctx.c:4447-4451`
* C signature: `void picoquic_get_local_addr(picoquic_cnx_t *, struct sockaddr **)`
* Rust source: `rs/fq/src/lib.rs:4961-4972`
* Rust item: `get_local_addr`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    *addr = (struct sockaddr*)&cnx->path[0]->first_tuple->local_addr;
}
```

### Rust body
```rust
    pub fn path_is_backup(&self, index: usize) -> bool {
        self.paths
            .get(index)
            .map(|p| p.path_is_backup)
            .unwrap_or(false)
    }
```

## `picoquic/quicctx.c:picoquic_get_pacing_rate`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns path[0] pacing rate; Rust body returns congestion window cwin.
* C source: `picoquic/quicctx.c:5426-5430`
* C signature: `uint64_t picoquic_get_pacing_rate(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4661-4668`
* Rust item: `pacing_rate`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->pacing.rate;
}
```

### Rust body
```rust
    pub fn cwin(&self) -> u64 {
        self.paths.first().map(|p| p.cwin).unwrap_or(0)
    }
```

## `picoquic/quicctx.c:picoquic_get_version_index`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C searches supported versions and returns an index or -1; Rust returns VersionParameters constants for a Version.
* C source: `picoquic/quicctx.c:1616-1628`
* C signature: `int picoquic_get_version_index(uint32_t)`
* Rust source: `rs/fq/src/internal.rs:311-449`
* Rust item: `try_from_wire`

### C body
```c
{
    int ret = -1;

    for (size_t i = 0; i < picoquic_nb_supported_versions; i++) {
        if (picoquic_supported_versions[i].version == proposed_version) {
            ret = (int)i;
            break;
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn parameters(self) -> VersionParameters {
        const V1_SALT: &[u8] = &[
            0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8,
            0x0c, 0xad, 0xcc, 0xbb, 0x7f, 0x0a,
        ];
        const V1_RETRY_KEY: &[u8] = &[
            0xd9, 0xc9, 0x94, 0x3e, 0x61, 0x01, 0xfd, 0x20, 0x00, 0x21, 0x50, 0x6b, 0xcc, 0x02,
            0x81, 0x4c, 0x73, 0x03, 0x0f, 0x25, 0xc7, 0x9d, 0x71, 0xce, 0x87, 0x6e, 0xca, 0x87,
            0x6e, 0x6f, 0xca, 0x8e,
        ];
        const V2_SALT: &[u8] = &[
            0x0d, 0xed, 0xe3, 0xde, 0xf7, 0x00, 0xa6, 0xdb, 0x81, 0x93, 0x81, 0xbe, 0x6e, 0x26,
            0x9d, 0xcb, 0xf9, 0xbd, 0x2e, 0xd9,
        ];
        const V2_RETRY_KEY: &[u8] = &[
            0xc4, 0xdd, 0x24, 0x84, 0xd6, 0x81, 0xae, 0xfa, 0x4f, 0xf4, 0xd6, 0x9c, 0x2c, 0x20,
            0x29, 0x99, 0x84, 0xa7, 0x65, 0xa5, 0xd3, 0xc3, 0x19, 0x82, 0xf3, 0x8f, 0xc7, 0x41,
            0x62, 0x15, 0x5e, 0x9f,
        ];
        const V2_DRAFT_SALT: &[u8] = &[
            0xa7, 0x07, 0xc2, 0x03, 0xa5, 0x9b, 0x47, 0x18, 0x4a, 0x1d, 0x62, 0xca, 0x57, 0x04,
            0x06, 0xea, 0x7a, 0xe3, 0xe5, 0xd3,
        ];
        const V2_DRAFT_RETRY_KEY: &[u8] = &[
            0x34, 0x25, 0xc2, 0x0c, 0xf8, 0x87, 0x79, 0xdf, 0x2f, 0xf7, 0x1e, 0x8a, 0xbf, 0xa7,
            0x82, 0x49, 0x89, 0x1e, 0x76, 0x3b, 0xbe, 0xd2, 0xf1, 0x3c, 0x04, 0x83, 0x43, 0xd3,
            0x48, 0xc0, 0x60, 0xe2,
        ];
        const DRAFT_29_SALT: &[u8] = &[
            0xaf, 0xbf, 0xec, 0x28, 0x99, 0x93, 0xd2, 0x4c, 0x9e, 0x97, 0x86, 0xf1, 0x9c, 0x61,
            0x11, 0xe0, 0x43, 0x90, 0xa8, 0x99,
        ];
        const RETRY_KEY_29: &[u8] = &[
            0x8b, 0x0d, 0x37, 0xeb, 0x85, 0x35, 0x02, 0x2e, 0xbc, 0x8d, 0x76, 0xa2, 0x07, 0xd8,
            0x0d, 0xf2, 0x26, 0x46, 0xec, 0x06, 0xdc, 0x80, 0x96, 0x42, 0xc3, 0x0a, 0x8b, 0xaa,
            0x2b, 0xaa, 0xff, 0x4c,
        ];
        const INTERNAL_TEST_1_SALT: &[u8] = &[
            0x30, 0x67, 0x16, 0xd7, 0x63, 0x75, 0xd5, 0x55, 0x4b, 0x2f, 0x60, 0x5e, 0xef, 0x78,
            0xd8, 0x33, 0x3d, 0xc1, 0xca, 0x36,
        ];

        const V1_PREFIX: &str = "tls13 quic ";
        const V2_PREFIX: &str = "tls13 quicv2 ";
        const V1_KU: &str = "quic ku";
        const V2_KU: &str = "quicv2 ku";

        const UPGRADE_FROM_V1: &[Version] = &[Version::V1];

        match self {
            Version::V1 => VersionParameters {
                version: self,
                version_aead_key: V1_SALT,
                version_retry_key: V1_RETRY_KEY,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
            Version::V2 => VersionParameters {
                version: self,
                version_aead_key: V2_SALT,
                version_retry_key: V2_RETRY_KEY,
                tls_prefix_label: V2_PREFIX,
                tls_traffic_update_label: V2_KU,
                packet_type_version: 0x6b33_43cf,
                upgrade_from: UPGRADE_FROM_V1,
            },
            Version::V2Draft => VersionParameters {
                version: self,
                version_aead_key: V2_DRAFT_SALT,
                version_retry_key: V2_DRAFT_RETRY_KEY,
                tls_prefix_label: V2_PREFIX,
                tls_traffic_update_label: V2_KU,
                packet_type_version: 0x6b33_43cf,
                upgrade_from: UPGRADE_FROM_V1,
            },
            Version::PostIesg | Version::TwentyFirstInterop => VersionParameters {
                version: self,
                version_aead_key: V1_SALT,
                version_retry_key: V1_RETRY_KEY,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
            Version::TwentiethInterop
            | Version::TwentiethPreInterop
            | Version::NineteenthBisInterop
            | Version::NineteenthInterop => VersionParameters {
                version: self,
                version_aead_key: DRAFT_29_SALT,
                version_retry_key: RETRY_KEY_29,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
            Version::InternalTest1 | Version::InternalTest2 => VersionParameters {
                version: self,
                version_aead_key: INTERNAL_TEST_1_SALT,
                version_retry_key: V1_RETRY_KEY,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: self as u32,
                upgrade_from: &[],
            },
            Version::SeventeenthInterop | Version::EighteenthInterop => VersionParameters {
                version: self,
                version_aead_key: DRAFT_29_SALT,
                version_retry_key: RETRY_KEY_29,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
        }
    }
```
