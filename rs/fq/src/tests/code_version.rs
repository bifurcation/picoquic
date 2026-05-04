//! Test cases for `picoquictest/code_version_test.c`.

#![allow(non_snake_case)]

/// C: `code_version_test` in `picoquictest/code_version_test.c`.
///
/// The C body cross-checks `PICOQUIC_VERSION` against the version
/// recorded in `CMakeLists.txt`'s `project(picoquic VERSION ...)`
/// line.  In Rust the equivalent invariant is that
/// [`crate::VERSION`] is a well-formed four-part dotted version
/// string (`major.minor.patch.build`).
#[test]
fn code_version() {
    let parts: Vec<&str> = crate::VERSION.split('.').collect();
    assert_eq!(
        parts.len(),
        4,
        "VERSION must be `major.minor.patch.build`, got {:?}",
        crate::VERSION,
    );
    for (i, part) in parts.iter().enumerate() {
        part.parse::<u32>().unwrap_or_else(|e| {
            panic!("VERSION component {i} ({part:?}) is not a numeric u32: {e}")
        });
    }
}
