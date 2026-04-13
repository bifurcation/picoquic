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
- [ ] `packet.c` - Packet formatting and parsing
- [ ] `frames.c` - Frame encoding/decoding
- [ ] `sender.c` - Packet sending logic
- [ ] `paths.c` - Path management
- [ ] `sacks.c` - SACK list management
- [ ] `loss_recovery.c` - Loss detection and recovery

### Congestion Control
- [ ] `cc_common.c` - Common CC utilities
- [ ] `newreno.c` - NewReno algorithm
- [ ] `cubic.c` - CUBIC algorithm
- [ ] `bbr.c` - BBRv3 algorithm
- [ ] `bbr1.c` - BBRv1 algorithm
- [ ] `fastcc.c` - Fast CC algorithm
- [ ] `prague.c` - Prague (L4S) algorithm
- [ ] `dualq_aqm.c` - DualQ AQM support
- [ ] `c4.c` - C4 algorithm
- [ ] `pacing.c` - Pacing implementation
- [ ] `register_all_cc_algorithms.c` - CC registration

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
- [ ] `intformat.c` - Integer encoding/decoding
- [ ] `bytestream.c` - Byte stream utilities
- [ ] `util.c` - General utilities
- [ ] `siphash.c` - SipHash implementation
- [ ] `picohash.c` - Hash table implementation
- [ ] `picosplay.c` - Splay tree implementation
- [ ] `spinbit.c` - Spin bit handling
- [ ] `timing.c` - Timing utilities
- [ ] `port_blocking.c` - Port blocking detection
- [ ] `picoquic_lb.c` - Load balancer integration
- [ ] `config.c` - Configuration handling

### Logging
- [ ] `logger.c` - General logging
- [ ] `logwriter.c` - Log file writing
- [ ] `unified_log.c` - Unified logging API
- [ ] `memory_log.c` - Memory logging
- [ ] `performance_log.c` - Performance logging

### Name/String Tables
- [ ] `error_names.c` - Error code names
- [ ] `frame_names.c` - Frame type names
- [ ] `packet_names.c` - Packet type names
- [ ] `tp_names.c` - Transport parameter names

### Network/Sockets
- [ ] `picosocks.c` - Socket utilities
- [ ] `sockloop.c` - Unix socket loop
- [ ] `winsockloop.c` - Windows socket loop (if needed)
- [ ] `sim_link.c` - Simulated network link

## Translation Order

### Phase 1: Foundation
1. `intformat.c` - Pure functions, no dependencies
2. `bytestream.c` - Simple data structures
3. `util.c` - Utility functions
4. `siphash.c` - Pure hash function
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
