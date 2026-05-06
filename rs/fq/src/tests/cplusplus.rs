//! Tests from `picoquictest/cplusplus.cpp`.
//!
//! The C++ test exists solely to verify that all picoquic headers compile
//! as C++.  The body is an unconditional `return 0;`.  In Rust the
//! equivalent is an empty test — if this file compiles, the test passes.

#[test]
fn cplusplus() {
    // C: `cplusplustest` — no-op; proves C++ compilation succeeds.
}
