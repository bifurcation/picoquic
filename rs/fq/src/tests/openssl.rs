//! Test case for `picoquictest/openssl_test.c`.
//!
//! Exercises the OpenSSL certificate-loading helper by verifying that
//! PEM files with one, one, and two certificates are parsed correctly.

#![allow(non_snake_case)]

use crate::tls_api::get_certs_from_file;

/// Load a certificate file and assert the correct number of DER blobs.
/// C: `openssl_cert_test_one`.
fn openssl_cert_test_one(cert_file_name: &str, expected_count: usize) {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../certs/tests")
        .join(cert_file_name);
    let path = base.to_string_lossy();
    let certs = get_certs_from_file(&path);
    let count = certs.as_ref().map(|v| v.len()).unwrap_or(0);
    if count != expected_count {
        panic!("Expected {expected_count} certs for {cert_file_name}, got {count}");
    }
}

/// C: `openssl_cert_test`.
#[test]
fn openssl_cert() {
    openssl_cert_test_one("cert.pem", 1);
    openssl_cert_test_one("chain.pem", 1);
    openssl_cert_test_one("fullchain.pem", 2);
}
