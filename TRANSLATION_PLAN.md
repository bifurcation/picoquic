# TRANSLATION_PLAN.md

This document describes the plan for translating picoquic from C to Rust in the `fq` crate.

## Overview

The goal is a comprehensive 1:1 translation of picoquic's functionality to safe, idiomatic Rust. The translation preserves the existing C implementation with `#ifdef` guards, allowing gradual migration and fallback.

See [MEMORY_ARCH.md](MEMORY_ARCH.md) for detailed analysis of ownership patterns.
See [CLAUDE.md](CLAUDE.md) for build instructions and project structure.

## Approach

### 1. Bindgen for FFI Declarations
Use `bindgen` to generate Rust FFI bindings for the C API. This provides:
- Type-safe declarations for all public types
- Function signatures for interop during transition
- Reference implementation to validate translation

### 2. Incremental Translation
Translate each `.c` file to a corresponding `.rs` module:
- Add `#ifdef FQ_USE_RUST` guards around replaced C code
- Expose Rust functions via `#[no_mangle] extern "C"` initially
- Gradually convert call sites to pure Rust

### 3. Ownership Analysis
Apply ownership patterns from MEMORY_ARCH.md:
- Owned pointers become `Box<T>`, `Vec<T>`, or arena indices
- Borrowed pointers become `&T` or `&mut T`
- Optional pointers become `Option<T>`
- Callback patterns become trait objects

### 4. Minimize Unsafe
Unsafe code only where necessary:
- FFI boundaries with TLS/crypto libraries
- Performance-critical hot paths (after profiling)
- Network address handling

### 5. Test Translation
Convert picoquic tests to Cargo tests:
- Unit tests for individual modules
- Integration tests using the test framework
- Property-based tests where applicable

## File Translation Checklist

### Core Protocol (High Priority)
- [ ] `quicctx.c` - QUIC context creation/management
- [ ] `transport.c` - Transport parameter handling
- [x] `packet.c` - Packet formatting and parsing (partial: get_packet_number64 - C retains header parsing, protection, incoming packet handling)
- [x] `frames.c` - Frame encoding/decoding (partial: 15 skip, 12 parse, stream header, ACK header, decode_max_data/max_path_id/ack_frequency/paths_blocked/path_cid_blocked/immediate_ack - C retains encode/check_repeat functions)
- [ ] `sender.c` - Packet sending logic
- [ ] `paths.c` - Path management
- [x] `sacks.c` - SACK list management (full: SackList with BTreeMap, range merging, ACK horizon, send count tracking - C retains cnx context functions)
- [ ] `loss_recovery.c` - Loss detection and recovery

### Congestion Control
- [x] `cc_common.c` - Common CC utilities (full safe Rust: MinMaxRtt, NewRenoSimState with enter_recovery/notify, slow_start/cwin functions, PerAckState)
- [x] `newreno.c` - NewReno algorithm (partial: sim_reset FFI - full sim_notify/enter_recovery in safe Rust, C retains path/cnx wrapper)
- [x] `cubic.c` - CUBIC algorithm (partial: CubicState with reset/enter_recovery/correct_spurious/w_cubic, cubic_root - C retains notify wrapper)
- [x] `bbr.c` - BBRv3 algorithm (partial: BbrState with state machine, probe BW/RTT phases, recovery handling, bandwidth/RTT estimation - C retains notify/observe wrappers)
- [x] `bbr1.c` - BBRv1 algorithm (partial: Bbr1State with state machine, bandwidth tracking, pacing, suspension handling - C retains notify/observe wrappers)
- [x] `fastcc.c` - Fast CC algorithm (partial: FastCcState with reset/seed_cwin/notify_congestion/check_exit_freeze/on_ack/on_rtt_measurement - C retains notify wrapper)
- [x] `prague.c` - Prague (L4S) algorithm (partial: PragueState with alpha EWMA, enter_recovery, process_ack, update_alpha - C retains notify wrapper)
- [x] `dualq_aqm.c` - DualQ AQM support (full: DualqState with PI2 controller, L4S/classic queue management, RFC 9332 marking/dropping - C retains sim link integration)
- [x] `c4.c` - C4 algorithm (partial: C4State with state machine, sensitivity functions, ECN/loss tracking, probe levels - C retains notify wrapper)
- [x] `pacing.c` - Pacing implementation (partial: standalone functions - path functions remain in C)
- [x] `register_all_cc_algorithms.c` - CC registration (full: CcRegistry with CcAlgoNumber enum, algorithm lookup by name/number)

### TLS/Crypto Integration
- [ ] `tls_api.c` - TLS integration layer
- [ ] `ticket_store.c` - Session ticket storage
- [ ] `token_store.c` - Token storage
- [ ] `ech.c` - ECH support

### Crypto Backend (choose one to start)
- [ ] `picoquic_ptls_openssl.c` - OpenSSL backend
- [ ] `picoquic_ptls_fusion.c` - AES-NI fusion backend
- [ ] `picoquic_ptls_minicrypto.c` - Minicrypto backend
- [ ] `picoquic_mbedtls.c` - mbedTLS backend

### Utilities
- [x] `intformat.c` - Integer encoding/decoding
- [x] `bytestream.c` - Byte stream utilities (includes CID read/write/skip - excludes addr functions)
- [x] `util.c` - General utilities (partial: hex, frame encode/decode, random, memcmp, ConnectionId type with format/parse/compare/hash/val64/hexa - excludes addr functions)
- [x] `siphash.c` - SipHash implementation
- [x] `picohash.c` - Hash functions (partial: picohash_bytes, picohash_siphash - hash table remains in C)
- [x] `port_blocking.c` - Port blocking (partial: check_port_blocked - addr check uses internal types)
- [x] `picosplay.c` - Splay tree (full: PicosplayNode, PicosplayTree with callback function pointers, splay/zig/zigzig/zigzag operations, insert/find/delete/iteration)
- [x] `spinbit.c` - Spin bit handling (full: SpinBitVariant enum, SpinBitState with basic/null/random variants, standalone functions)
- [x] `timing.c` - Timing utilities (full: RttState, ConnectionTiming structs, current_retransmit_timer, update_rtt, update_one_way_delay)
- [ ] `picoquic_lb.c` - Load balancer integration
- [ ] `config.c` - Configuration handling

### Logging
- [x] `logger.c` - General logging (partial: state_name lookup - remaining logging functions require FILE* and complex formatting)
- [ ] `logwriter.c` - Log file writing
- [ ] `unified_log.c` - Unified logging API
- [ ] `memory_log.c` - Memory logging
- [x] `performance_log.c` - Performance logging (partial: perflog_param_name lookup - remaining functions require FILE* and quic context)

### Name/String Tables
- [x] `error_names.c` - Error code names (full: transport/tls/error modules, error_name lookup, crypto alert range handling, FFI export)
- [x] `frame_names.c` - Frame type names (full: frame_type constants module, frame_name lookup, is_stream_frame helper, FFI export)
- [x] `packet_names.c` - Packet type names (full: PacketType enum, packet_type_name lookup, FFI export)
- [x] `tp_names.c` - Transport parameter names (full: tp constants module, tp_name lookup function, FFI export)

### Network/Sockets
- [ ] `picosocks.c` - Socket utilities (platform-specific, lower priority)
- [ ] `sockloop.c` - Unix socket loop (platform-specific, lower priority)
- [ ] `winsockloop.c` - Windows socket loop (platform-specific, lower priority)
- [x] `sim_link.c` - Simulated network link (full: SimLink, SimPacket, bandwidth/latency/loss/jitter simulation, queue delay, burst loss)

## Translation Order

### Phase 1: Foundation
1. `intformat.c` - Pure functions, no dependencies - **DONE** (`fq/src/intformat.rs`)
2. `bytestream.c` - Simple data structures - **DONE** (`fq/src/bytestream.rs`)
3. `util.c` - Utility functions
4. `siphash.c` - Pure hash function - **DONE** (`fq/src/siphash.rs`)
5. `picohash.c` - Direct translation with unsafe, or use `HashMap`
6. `picosplay.c` - Direct translation with unsafe, or use `BTreeMap`

Note: For picohash and picosplay, direct translation preserves structural equivalence
and makes verification easier. See MEMORY_ARCH.md Phase 0 for the approach.

### Phase 2: Core Structures
7. `quicctx.c` - Main context structures
8. `transport.c` - Transport parameters
9. `paths.c` - Path structures
10. `sacks.c` - SACK structures

### Phase 3: Protocol
11. `packet.c` - Packet handling
12. `frames.c` - Frame handling
13. `sender.c` - Send logic
14. `loss_recovery.c` - Loss handling

### Phase 4: Congestion Control
15. `cc_common.c` - CC foundation
16. `pacing.c` - Pacing
17. `newreno.c` - Simple CC first
18. Other CC algorithms

### Phase 5: TLS Integration
19. `tls_api.c` - Main TLS layer
20. One crypto backend (OpenSSL recommended)
21. `ticket_store.c`, `token_store.c`

### Phase 6: Ancillary
22. Logging modules
23. Name tables
24. Socket handling

## Rust Module Structure

```
fq/src/
├── lib.rs              # Crate root, public API
├── context.rs          # picoquic_quic_t equivalent
├── connection.rs       # picoquic_cnx_t equivalent
├── path.rs             # picoquic_path_t equivalent
├── stream.rs           # picoquic_stream_head_t equivalent
├── packet/
│   ├── mod.rs
│   ├── header.rs
│   ├── frames.rs
│   └── protection.rs
├── transport/
│   ├── mod.rs
│   └── parameters.rs
├── congestion/
│   ├── mod.rs
│   ├── common.rs
│   ├── pacing.rs
│   ├── newreno.rs
│   ├── cubic.rs
│   └── bbr.rs
├── crypto/
│   ├── mod.rs
│   ├── tls.rs
│   └── backends/
├── recovery/
│   ├── mod.rs
│   ├── loss.rs
│   └── sack.rs
├── util/
│   ├── mod.rs
│   ├── varint.rs
│   ├── bytestream.rs
│   └── hash.rs
└── tests/              # Integration tests
```

## Testing Strategy

### Unit Tests
Each translated module includes unit tests matching C test coverage:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_varint_encode_decode() { ... }
}
```

### Integration Tests
Port `picoquictest/` tests to `fq/tests/`:
- Connection establishment
- Stream transfer
- Loss recovery
- Congestion control behavior

### Compatibility Tests
Verify C and Rust implementations produce identical results:
- Same packets for same inputs
- Same state transitions
- Same timing behavior

## Success Criteria

1. All picoquic tests pass when using Rust implementation
2. No unsafe code outside designated unsafe modules
3. Clippy passes with no warnings
4. Documentation for public API
5. Benchmark shows comparable or better performance
