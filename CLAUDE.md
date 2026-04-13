# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Picoquic is a minimalist QUIC protocol implementation (RFC 9000) in C. The core library (`picoquic-core`) provides QUIC transport with multiple congestion control algorithms (BBR, CUBIC, NewReno, Prague, FastCC).

The `fq/` directory contains a Rust crate that will eventually provide a Rust equivalent of picoquic. It is built via [Corrosion](https://github.com/corrosion-rs/corrosion) and linked into picoquic's test suite.

## Dependencies and Build System

### Dependency Chain
```
OpenSSL (system) -> picotls (TLS 1.3) -+
                                       |-> picoquic
                        fq (Rust) -----+
```

### Installing Dependencies

**Ubuntu/Debian:**
```bash
sudo apt-get install -y libssl-dev
```

**macOS:**
```bash
brew install openssl
```

### Building with Auto-Fetched Picotls (Recommended)

```bash
cmake -S . -B build -DPICOQUIC_FETCH_PTLS=ON
cmake --build build
```

This uses CMake's `FetchContent` to download picotls from GitHub (pinned commit in CMakeLists.txt line 287-288) and builds it alongside picoquic.

### Building with Pre-Installed Picotls

Clone picotls as a sibling directory (the build system looks in `../picotls/`):
```bash
cd ..
git clone https://github.com/h2o/picotls.git
cd picotls && cmake . && make
cd ../picoquic
cmake -S . -B build
cmake --build build
```

The `cmake/FindPTLS.cmake` module searches for picotls in `../picotls/` or standard system paths.

### Build Options

| Option | Default | Description |
|--------|---------|-------------|
| `PICOQUIC_FETCH_PTLS` | OFF | Auto-fetch picotls via FetchContent |
| `picoquic_BUILD_TESTS` | ON (standalone) / OFF (subproject) | Build test executables |
| `BUILD_FQ` | ON | Build the fq Rust crate via Corrosion |
| `ENABLE_ASAN` | OFF | AddressSanitizer |
| `ENABLE_UBSAN` | OFF | UndefinedBehaviorSanitizer |
| `CMAKE_BUILD_TYPE` | - | Set to `Debug` for debug symbols |
| `WITH_OPENSSL` | ON | Build with OpenSSL support |
| `WITH_MBEDTLS` | OFF | Build with mbedTLS support |

### Build Targets

```bash
cmake --build build --target <target>
```

| Target | Description |
|--------|-------------|
| `picoquic-core` | Core QUIC library |
| `picoquic-log` | Logging library |
| `picohttp-core` | HTTP/3 library |
| `fq` | Rust crate (built via Corrosion) |
| `picoquic_ct` | QUIC test runner |
| `picohttp_ct` | HTTP test runner |
| `picoquicdemo` | Demo client/server |
| `picoquic_sample` | Simple example app |

## Verifying the Build

A complete verification ensures:
1. The fq Rust crate builds without warnings, clippy lints, or formatting errors
2. The fq crate passes its Rust tests
3. Picoquic builds without errors
4. All picoquic tests pass

### Quick Verification (from repository root)

```bash
# 1. Check fq crate: formatting, lints, warnings, and tests
cd fq
cargo fmt --check
cargo clippy -- -D warnings
cargo build --release
cargo test
cd ..

# 2. Build picoquic (includes fq via Corrosion)
cmake -S . -B build -DPICOQUIC_FETCH_PTLS=ON
cmake --build build

# 3. Run all picoquic tests
cd build
./picoquic_ct -S .. -n -r
./picohttp_ct -S .. -n -r
```

### All-in-One Verification Script

```bash
set -e  # Exit on first error

# Verify fq Rust crate
(cd fq && cargo fmt --check && cargo clippy -- -D warnings && cargo test)

# Build and test picoquic
cmake -S . -B build -DPICOQUIC_FETCH_PTLS=ON
cmake --build build
(cd build && ./picoquic_ct -S .. -n -r && ./picohttp_ct -S .. -n -r)

echo "All checks passed."
```

## Test Suite

### Test Architecture

Tests are organized as:
- **Test functions**: Declared in `picoquictest/picoquictest.h`, implemented in `picoquictest/*.c`
- **Test registry**: `picoquic_t/picoquic_t.c` contains the `test_table[]` array mapping test names to functions
- **Test runners**: `picoquic_ct` (QUIC tests) and `picohttp_ct` (HTTP tests)

Each test function returns 0 on success, non-zero on failure.

### Running Tests

```bash
cd build

# Run all QUIC tests (with debug output disabled, retry failures with debug)
./picoquic_ct -S .. -n -r

# Run all HTTP tests
./picohttp_ct -S .. -n -r
```

The `-S ..` flag sets the solution directory to find test data files (certificates, etc.) in the source tree.

### Running Specific Tests

```bash
# Run single test by name
./picoquic_ct tls_api

# Run multiple specific tests
./picoquic_ct tls_api zero_rtt bbr

# Exclude tests
./picoquic_ct -x stress -x fuzz

# Run test range by number (see test_table[] ordering)
./picoquic_ct -o 10 20
```

### Test Runner Options

| Flag | Description |
|------|-------------|
| `-n` | Disable debug printf (faster execution) |
| `-r` | Retry failed tests with debug enabled |
| `-S <path>` | Set solution/source directory for test files |
| `-x <name>` | Exclude test(s) by name |
| `-o <n1> <n2>` | Only run tests numbered n1 to n2 |
| `-s <minutes>` | Run stress test for N minutes |
| `-f <minutes>` | Run fuzz test for N minutes |
| `-c <min> <cnx>` | Connection stress: minutes and connection count |

### Adding a New Test

1. Declare the test function in `picoquictest/picoquictest.h`:
   ```c
   int my_new_test(void);
   ```

2. Implement in appropriate `picoquictest/*.c` file (or create new file and add to `PICOQUIC_TEST_LIBRARY_FILES` in CMakeLists.txt)

3. Register in `picoquic_t/picoquic_t.c` by adding to `test_table[]`:
   ```c
   { "my_new_test", my_new_test },
   ```

### Test Simulation Infrastructure

Tests use a simulated network (`picoquic/sim_link.c`) with virtual time, enabling:
- Deterministic packet loss injection
- Configurable latency and bandwidth
- Time-controlled test scenarios without wall-clock delays

## Library Structure

### Core Library (`picoquic/`)
- `picoquic.h` - Public API (stable)
- `picoquic_internal.h` - Internal structures (may change)
- `quicctx.c` - QUIC context management
- `tls_api.c` - Picotls integration
- `frames.c` - Frame encoding/decoding
- `sender.c` - Packet transmission
- `sockloop.c` - Socket event loop

### Rust Crate (`fq/`)
- `Cargo.toml` - Crate configuration (staticlib for C FFI)
- `src/lib.rs` - Rust implementation

The fq crate is built automatically by CMake via Corrosion. Functions exported from Rust with `#[no_mangle] extern "C"` can be declared in a C header and called from picoquic code.

## C-to-Rust Translation

The `fq` crate will eventually contain a complete Rust implementation of picoquic. See:
- [TRANSLATION_PLAN.md](TRANSLATION_PLAN.md) - Overall translation strategy and file checklist
- [MEMORY_ARCH.md](MEMORY_ARCH.md) - Ownership analysis of picoquic data structures

Translation approach:
1. Use `bindgen` for FFI type declarations
2. Translate each `.c` file to `.rs` with both:
   - Safe Rust API for internal use
   - FFI exports (`#[no_mangle] extern "C"`) matching the C function signatures
3. Guard the C implementation with `#ifndef FQ_USE_RUST ... #endif`
4. CMake defines `FQ_USE_RUST` when `BUILD_FQ=ON`, linking the Rust library
5. Minimize unsafe code; use safe Rust idioms (Vec, HashMap, BTreeMap) for collections
6. Port tests from `picoquictest/` to `fq/tests/`

### FFI Integration Pattern

**NO LOGIC IN FFI.** This is a hard rule. FFI functions contain only:
1. Pointer dereferences to access the C struct
2. Calls to safe Rust methods
3. Return value conversion (Result to c_int, Option to nullable pointer)

All logic - including allocation, validation, conditionals, loops - lives in safe Rust. If you find yourself writing an `if` statement or any computation in an FFI function, move it to a safe Rust method.

For each translated file:

**Rust side** (`fq/src/example.rs`):
```rust
// ===================
// Safe Rust API (ALL logic lives here)
// ===================

pub struct MyType<'a> {
    data: &'a mut [u8],
    pos: usize,
}

impl<'a> MyType<'a> {
    /// Wrap a C struct, borrowing its buffer.
    pub unsafe fn from_c(c: &'a mut CMyType) -> Self {
        let data = std::slice::from_raw_parts_mut(c.data, c.size);
        Self { data, pos: c.ptr }
    }

    /// Sync position back to C struct.
    pub fn sync_to_c(&self, c: &mut CMyType) {
        c.ptr = self.pos;
    }

    pub fn write_u16(&mut self, value: u16) -> Result<(), Error> {
        // All logic here
    }
}

// C-compatible struct - put methods here for init/alloc operations
#[repr(C)]
pub struct CMyType {
    pub data: *mut u8,
    pub size: usize,
    pub ptr: usize,
}

impl CMyType {
    /// Allocate buffer. Logic lives HERE, not in FFI function.
    pub fn alloc(&mut self, size: usize) -> bool {
        let layout = match std::alloc::Layout::from_size_align(size, 1) {
            Ok(l) => l,
            Err(_) => return false,
        };
        let data = unsafe { std::alloc::alloc(layout) };
        if data.is_null() { return false; }
        self.data = data;
        self.size = size;
        self.ptr = 0;
        true
    }
}

// ===================
// FFI Layer (THIN WRAPPERS ONLY - no logic)
// ===================

#[no_mangle]
pub unsafe extern "C" fn mytype_alloc(s: *mut CMyType, size: usize) -> *mut CMyType {
    if (*s).alloc(size) { s } else { std::ptr::null_mut() }
}

#[no_mangle]
pub unsafe extern "C" fn mytype_write_u16(s: *mut CMyType, value: u16) -> c_int {
    let mut wrapper = MyType::from_c(&mut *s);
    let result = match wrapper.write_u16(value) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    wrapper.sync_to_c(&mut *s);
    result
}
```

**C side** (`picoquic/example.c`):
```c
/* When FQ_USE_RUST is defined, these functions are provided by the fq Rust crate */
#ifndef FQ_USE_RUST

int mytype_write_u16(mytype* s, uint16_t value) {
    // C implementation
}

#endif /* !FQ_USE_RUST */
```

This pattern ensures:
1. **NO LOGIC IN FFI** - FFI functions are trivial wrappers; all logic (including allocation, validation, error handling) lives in safe Rust methods
2. **Single source of truth** - safe Rust implementation, not duplicated between Rust and FFI
3. **Minimal unsafe** - only pointer derefs at FFI boundary
4. **Testability** - safe Rust API can be unit tested without FFI
5. **Gradual migration** - Rust replaces C at link time when `BUILD_FQ=ON`

### IMPORTANT: Verification After Each Translation

**Every translation step MUST be followed by full verification.** Do not commit until all checks pass:

```bash
# From repository root - run ALL of these after each translation:
(cd fq && cargo fmt --check && cargo clippy -- -D warnings && cargo test)
cmake --build build
(cd build && ./picoquic_ct -S .. -n -r && ./picohttp_ct -S .. -n -r)
```

This ensures the Rust code compiles, passes its own tests, links correctly with picoquic, and doesn't break any existing functionality.

### Key Design Patterns

**Single-threaded**: The library is not thread-safe. Use separate QUIC contexts for parallelism.

**Virtual time**: API calls receive `current_time` as a parameter rather than reading system time, enabling deterministic testing.

**Callback-driven**: Data delivery uses `picoquic_stream_data_cb_fn` callbacks registered per connection.

## Code Style

- C11 standard (set in CMakeLists.txt)
- Timestamps are `uint64_t` in microseconds
- Error codes: `PICOQUIC_ERROR_*` constants in `picoquic.h`
- Public API prefix: `picoquic_`

## Workflow Notes

- **No compound commands**: Never use compound commands like `cd foo && command`. Use separate Bash tool calls instead. Each command should be a separate invocation.
- **Git commands**: Always run git commands from the repository root (`/Users/richbarn/Projects/link/picoquic`).
