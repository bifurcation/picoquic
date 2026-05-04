//! Test cases for `picoquictest/config_test.c`.

#![allow(non_snake_case)]

/// C: `config_option_letters_test` in `picoquictest/config_test.c`.
///
/// Compares the getopt-style option string produced by
/// [`crate::config::Config::option_letters`] against a hand-written
/// reference.  When new CLI options are added the reference must be
/// kept in sync.
#[test]
fn config_option_letters() {
    use crate::config::Config;

    // Reference letters.  Note the C source has a `#ifndef
    // PICOQUIC_WITHOUT_SSLKEYLOG` branch toggling whether the `8`
    // flag is present; the Rust default tracks the with-keylog
    // build (i.e. includes `8`).
    let expected = "c:k:p:v:o:w:x:rR:s:XS:G:H:P:O:Me:C:i:l:Lb:q:m:n:a:t:zI:d:DQT:N:B:F:VU:0j:W:8J:E:y:K:Z:4:6:h";
    assert_eq!(Config::option_letters(), expected);
}

/// C: `config_option_test` in `picoquictest/config_test.c`.
///
/// Walks every command-line option through
/// [`crate::config::Config::set_option`] and verifies the parsed
/// fields match a reference [`Config`].  Heavy fixture-driven
/// test; pending the body translation until the option-id table
/// stabilises.
#[test]
fn config_option() {
    todo!("config_option_test")
}

/// C: `config_option_test` letters companion that verifies the
/// command-line short-option mapping doesn't drift.
#[test]
fn config_quic() {
    todo!("config_quic_test")
}

/// C: `config_preferred_test` in `picoquictest/config_test.c`.
///
/// Exercises the preferred-address parsing.  The Rust counterpart
/// is [`crate::utils::set_preferred_address`].
#[test]
fn config_preferred() {
    todo!("config_preferred_test")
}

/// C: `config_set_port_test` in `picoquictest/config_test.c`.
#[test]
fn config_set_port() {
    todo!("config_set_port_test")
}

/// C: `config_usage_test` in `picoquictest/config_test.c`.
#[test]
fn config_usage() {
    todo!("config_usage_test")
}
