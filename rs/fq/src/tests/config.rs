//! Test cases for `picoquictest/config_test.c`.

#![allow(non_snake_case)]

use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::config::Config;
use crate::utils::set_preferred_address;
use crate::{Instant, LossbitVersion, PreferredAddress, SpinbitVersion};

// PICOQUIC_MICROSEC_HANDSHAKE_MAX (30_000_000 µs) / 1000 = 30_000 ms.
const HANDSHAKE_TIMEOUT_MS: i32 = 30_000;

// ECH test-config base64 string (used as the `-K` / `--ech_c` option value).
const ECH_TEST_CONFIG_B64: &str = concat!(
    "AGT+DQBgAgAQAEEE2silQFS6M9oYqUF/SVPfYOamPbaOUzqf3RkUXqsDz7z7NpgWJI8HKW0V2E8",
    "w6Alk+xT8hnzUBsL9neiZP0iMKwAEAAEAAf8QdGVzdC5leGFtcGxlLmNvbQAA"
);

// Binary form of ECH_TEST_CONFIG_B64 (base64-decoded).
// Mirrors the C `ech_test_config_bin[102]` constant.
const ECH_TEST_CONFIG_BIN: [u8; 102] = [
    0x00, 0x64, 0xfe, 0x0d, 0x00, 0x60, 0x02, 0x00, 0x10, 0x00, 0x41, 0x04, 0xda, 0xc8, 0xa5, 0x40,
    0x54, 0xba, 0x33, 0xda, 0x18, 0xa9, 0x41, 0x7f, 0x49, 0x53, 0xdf, 0x60, 0xe6, 0xa6, 0x3d, 0xb6,
    0x8e, 0x53, 0x3a, 0x9f, 0xdd, 0x19, 0x14, 0x5e, 0xab, 0x03, 0xcf, 0xbc, 0xfb, 0x36, 0x98, 0x16,
    0x24, 0x8f, 0x07, 0x29, 0x6d, 0x15, 0xd8, 0x4f, 0x30, 0xe8, 0x09, 0x64, 0xfb, 0x14, 0xfc, 0x86,
    0x7c, 0xd4, 0x06, 0xc2, 0xfd, 0x9d, 0xe8, 0x99, 0x3f, 0x48, 0x8c, 0x2b, 0x00, 0x04, 0x00, 0x01,
    0x00, 0x01, 0xff, 0x10, 0x74, 0x65, 0x73, 0x74, 0x2e, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65,
    0x2e, 0x63, 0x6f, 0x6d, 0x00, 0x00,
];

// Short-form argument vector producing the param1 expected config.
const ARGV1: &[&str] = &[
    "-S",
    "/data/github/picoquic",
    "-c",
    "/data/certs/cert.pem",
    "-k",
    "/data/certs/key.pem",
    "-x",
    "1024",
    "-l",
    "/data/log.txt",
    "-b",
    "/data/log/",
    "-q",
    "/data/qlog/",
    "-p",
    "4433",
    "-e",
    "1",
    "-m",
    "1536",
    "-G",
    "bbr",
    "-H",
    "T250000",
    "-P",
    "3",
    "-O",
    "2",
    "-M",
    "-R",
    "1",
    "-L",
    "-w",
    "/data/www/",
    "-r",
    "-s",
    "0123456789abcdeffedcba9876543210",
    "-B",
    "655360",
    "-F",
    "/data/performance_log.csv",
    "-V",
    "-j",
    "1",
    "-0",
    "-i",
    "0N8C-000123",
    "-J",
    "2",
    "-E",
    "ech_key.pem",
    "ech_config.pem",
    "-y",
    "test.example.com",
    "-Z",
    "1000001",
    "-4",
    "192.0.2.1",
    "-6",
    "2001:db8::1",
];

// Short-form argument vector producing the param2 expected config.
const ARGV2: &[&str] = &[
    "-n",
    "test.example.com",
    "-a",
    "test",
    "-o",
    "/data/w_out",
    "-t",
    "data/certs/root.pem",
    "-C",
    "20",
    "-v",
    "fF000020",
    "-z",
    "-d",
    "1234567",
    "-D",
    "-Q",
    "-X",
    "-8",
    "-I",
    "5",
    "-T",
    "/data/tickets.bin",
    "-N",
    "/data/tokens.bin",
    "-U",
    "00000002",
    "-W",
    "1000000",
    "-K",
    ECH_TEST_CONFIG_B64,
    "-Z",
    "0",
];

// Long-form argument vector producing the same expected result as ARGV2.
const CONFIG_TWO: &[&str] = &[
    "--sni",
    "test.example.com",
    "--alpn",
    "test",
    "--outdir",
    "/data/w_out",
    "--root_trust_file",
    "data/certs/root.pem",
    "--cipher_suite",
    "20",
    "--proposed_version",
    "ff000020",
    "--force_zero_share",
    "--idle_timeout",
    "1234567",
    "--no_disk",
    "--large_client_hello",
    "--disable_block",
    "--sslkeylog",
    "--cnxid_length",
    "5",
    "--ticket_file",
    "/data/tickets.bin",
    "--token_file",
    "/data/tokens.bin",
    "--version_upgrade",
    "00000002",
    "--cwin_max",
    "1000000",
    "--ech_c",
    ECH_TEST_CONFIG_B64,
    "--flow_control_max",
    "0",
];

// Each sub-slice is an independent option sequence expected to fail parsing.
// Canonical build (sslkeylog supported): the `-8` missing-value error case is
// excluded because `-8` is a valid flag when sslkeylog is enabled.
const ERROR_CASES: &[&[&str]] = &[
    &["-A"],
    &["-S"],
    &["-c"],
    &["-k"],
    &["-x"],
    &["-x", "nb_cnx"],
    &["-l"],
    &["-b"],
    &["-q"],
    &["-p", "port"],
    &["-p"],
    &["-e"],
    &["-e", "a"],
    &["-m"],
    &["-m"], // C nb_args=1, second element "-1" is unused
    &["-m", "15360"],
    &["-P", "33"],
    &["-O", "22"],
    &["-R", "17"],
    &["-w"],
    &["-s", "0123456789abcdexyedcba9876543210"],
    &["-s", "0123456789abcdeffedcba987654321"],
    &["-s", "0123456789abcdeffedcba98765432"],
    &["-B", "buffer"],
    &["-F"],
    &["-j", "3"],
    &["-i"],
    &["-I", "-1"],
    &["-I", "255"],
    &["-U", "XY000002"],
    &["-W", "cwin"],
    &["-d", "idle"],
    &["-Z"],
    &["-Z", "0123456789abcdexyedcba9876543210"],
];

// Initialize a Config and parse all arguments in `argv` through
// Config::command_line (short-form only).  Mirrors the C helper
// `config_parse_command_line` in `picoquictest/config_test.c`.
fn parse_argv(argv: &[&str]) -> Result<Config, ()> {
    let mut config = Config::default();
    let mut i = 0;
    while i < argv.len() {
        let x = argv[i];
        if x.len() != 2 || x.as_bytes()[0] != b'-' {
            return Err(());
        }
        let opt = char::from(x.as_bytes()[1]);
        i += 1;
        let optarg = if i < argv.len() && !argv[i].starts_with('-') {
            let v = argv[i];
            i += 1;
            Some(v)
        } else {
            None
        };
        config
            .command_line(opt, &mut i, argv, optarg)
            .map_err(|_| ())?;
    }
    Ok(config)
}

// Initialize a Config and parse all arguments in `argv` through
// Config::command_line_ex (short- and long-form options).  Mirrors the C
// helper `config_test_parse_command_line_ex` in `picoquictest/config_test.c`.
fn parse_argv_ex(argv: &[&str]) -> Result<Config, ()> {
    let mut config = Config::default();
    let mut i = 0;
    while i < argv.len() {
        let x = argv[i];
        let is_short = x.len() == 2 && x.as_bytes()[0] == b'-' && x.as_bytes()[1] != b'-';
        let is_long = x.len() > 2 && x.starts_with("--");
        if !is_short && !is_long {
            return Err(());
        }
        i += 1;
        let optarg = if i < argv.len() && !argv[i].starts_with('-') {
            let v = argv[i];
            i += 1;
            Some(v)
        } else {
            None
        };
        config
            .command_line_ex(x, &mut i, argv, optarg)
            .map_err(|_| ())?;
    }
    Ok(config)
}

// Assert the fields that config_test_compare checks for param1.
fn assert_param1(c: &Config) {
    assert_eq!(c.nb_connections, 1024, "nb_connections");
    assert_eq!(
        c.solution_dir.as_deref(),
        Some("/data/github/picoquic"),
        "solution_dir"
    );
    assert_eq!(
        c.server_cert_file.as_deref(),
        Some("/data/certs/cert.pem"),
        "server_cert_file"
    );
    assert_eq!(
        c.server_key_file.as_deref(),
        Some("/data/certs/key.pem"),
        "server_key_file"
    );
    assert_eq!(c.log_file.as_deref(), Some("/data/log.txt"), "log_file");
    assert_eq!(c.bin_dir.as_deref(), Some("/data/log/"), "bin_dir");
    assert_eq!(c.qlog_dir.as_deref(), Some("/data/qlog/"), "qlog_dir");
    assert_eq!(
        c.performance_log.as_deref(),
        Some("/data/performance_log.csv"),
        "performance_log"
    );
    assert_eq!(c.server_port, 4433, "server_port");
    assert_eq!(c.dest_if, 1, "dest_if");
    assert_eq!(c.mtu_max, 1536, "mtu_max");
    assert_eq!(c.socket_buffer_size, 655_360, "socket_buffer_size");
    assert_eq!(c.cc_algo_id.as_deref(), Some("bbr"), "cc_algo_id");
    assert_eq!(
        c.connection_id_cbdata.as_deref(),
        Some("0N8C-000123"),
        "cnx_id_cbdata"
    );
    assert_eq!(c.spinbit_policy, SpinbitVersion::On, "spinbit");
    assert_eq!(c.lossbit_policy, LossbitVersion::SendReceive, "lossbit");
    assert_eq!(c.multipath_option, 1, "multipath");
    assert_eq!(c.initial_random, 1, "initial_random");
    assert!(c.use_long_log, "use_long_log");
    assert!(c.do_preemptive_repeat, "do_preemptive_repeat");
    assert!(c.do_not_use_gso, "do_not_use_gso");
    assert_eq!(c.www_dir.as_deref(), Some("/data/www/"), "www_dir");
    assert!(c.do_retry, "do_retry");
    assert_eq!(c.sni.as_deref(), None, "sni");
    assert_eq!(c.alpn.as_deref(), None, "alpn");
    assert_eq!(c.out_dir.as_deref(), None, "out_dir");
    assert_eq!(c.root_trust_file.as_deref(), None, "root_trust_file");
    assert_eq!(c.cipher_suite_id, 0, "cipher_suite_id");
    assert_eq!(c.proposed_version, 0, "proposed_version");
    assert_eq!(c.desired_version, 0, "desired_version");
    assert!(!c.force_zero_share, "force_zero_share");
    assert!(!c.no_disk, "no_disk");
    assert!(!c.large_client_hello, "large_client_hello");
    assert_eq!(c.connection_id_length, -1, "cnx_id_length");
    assert_eq!(c.bdp_frame_option, 1, "bdp");
    assert_eq!(c.idle_timeout, HANDSHAKE_TIMEOUT_MS, "idle_timeout");
    assert_eq!(c.cwin_max, u64::MAX, "cwin_max");
    assert!(!c.enable_sslkeylog, "enable_sslkeylog");
    assert_eq!(
        c.ech_key_file.as_deref(),
        Some("ech_key.pem"),
        "ech_key_file"
    );
    assert_eq!(
        c.ech_config_file.as_deref(),
        Some("ech_config.pem"),
        "ech_config_file"
    );
    assert_eq!(
        c.ech_public_name.as_deref(),
        Some("test.example.com"),
        "ech_public_name"
    );
    assert_eq!(c.flow_control_max, 1_000_001, "flow_control_max");
    assert_eq!(
        c.preferred_address_v4.as_deref(),
        Some("192.0.2.1"),
        "preferred_address_v4"
    );
    assert_eq!(
        c.preferred_address_v6.as_deref(),
        Some("2001:db8::1"),
        "preferred_address_v6"
    );
    assert!(c.ech_target.is_none(), "ech_target should be None");
}

// Assert the fields that config_test_compare checks for param2.
fn assert_param2(c: &Config) {
    assert_eq!(c.nb_connections, 256, "nb_connections");
    assert_eq!(c.solution_dir.as_deref(), None, "solution_dir");
    assert_eq!(c.server_cert_file.as_deref(), None, "server_cert_file");
    assert_eq!(c.server_key_file.as_deref(), None, "server_key_file");
    assert_eq!(c.log_file.as_deref(), None, "log_file");
    assert_eq!(c.bin_dir.as_deref(), None, "bin_dir");
    assert_eq!(c.qlog_dir.as_deref(), None, "qlog_dir");
    assert_eq!(c.performance_log.as_deref(), None, "performance_log");
    assert_eq!(c.server_port, 0, "server_port");
    assert_eq!(c.dest_if, 0, "dest_if");
    assert_eq!(c.mtu_max, 0, "mtu_max");
    assert_eq!(c.socket_buffer_size, 0, "socket_buffer_size");
    assert_eq!(c.cc_algo_id.as_deref(), None, "cc_algo_id");
    assert_eq!(c.connection_id_cbdata.as_deref(), None, "cnx_id_cbdata");
    assert_eq!(c.spinbit_policy, SpinbitVersion::Basic, "spinbit");
    assert_eq!(c.lossbit_policy, LossbitVersion::None, "lossbit");
    assert_eq!(c.multipath_option, 0, "multipath");
    assert_eq!(c.initial_random, 3, "initial_random (default)");
    assert!(!c.use_long_log, "use_long_log");
    assert!(!c.do_preemptive_repeat, "do_preemptive_repeat");
    assert!(!c.do_not_use_gso, "do_not_use_gso");
    assert_eq!(c.www_dir.as_deref(), None, "www_dir");
    assert!(!c.do_retry, "do_retry");
    assert_eq!(c.sni.as_deref(), Some("test.example.com"), "sni");
    assert_eq!(c.alpn.as_deref(), Some("test"), "alpn");
    assert_eq!(c.out_dir.as_deref(), Some("/data/w_out"), "out_dir");
    assert_eq!(
        c.root_trust_file.as_deref(),
        Some("data/certs/root.pem"),
        "root_trust_file"
    );
    assert_eq!(c.cipher_suite_id, 20, "cipher_suite_id");
    assert_eq!(c.proposed_version, 0xff00_0020, "proposed_version");
    assert_eq!(c.desired_version, 0x0000_0002, "desired_version");
    assert!(c.force_zero_share, "force_zero_share");
    assert!(c.no_disk, "no_disk");
    assert!(c.large_client_hello, "large_client_hello");
    assert_eq!(c.connection_id_length, 5, "cnx_id_length");
    assert_eq!(c.bdp_frame_option, 0, "bdp");
    assert_eq!(c.idle_timeout, 1_234_567, "idle_timeout");
    assert_eq!(c.cwin_max, 1_000_000, "cwin_max");
    assert!(c.enable_sslkeylog, "enable_sslkeylog");
    assert_eq!(c.ech_key_file.as_deref(), None, "ech_key_file");
    assert_eq!(c.ech_config_file.as_deref(), None, "ech_config_file");
    assert_eq!(c.ech_public_name.as_deref(), None, "ech_public_name");
    assert_eq!(c.flow_control_max, 0, "flow_control_max");
    assert_eq!(
        c.preferred_address_v4.as_deref(),
        None,
        "preferred_address_v4"
    );
    assert_eq!(
        c.preferred_address_v6.as_deref(),
        None,
        "preferred_address_v6"
    );
    assert_eq!(
        c.ech_target.as_deref(),
        Some(&ECH_TEST_CONFIG_BIN[..]),
        "ech_target"
    );
}

// Register congestion control algorithms for config tests.
// Mirrors `config_test_register_cc_algorithms` in C.
fn config_test_register_cc_algorithms() {
    crate::register_all_congestion_control_algorithms();
}

// Construct a path to a test fixture relative to the project root.
// Mirrors `picoquic_get_input_path(..., picoquic_solution_dir, ...)` in C.
fn fixture_path(relative: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), relative)
}

/// C: `config_option_letters_test` in `picoquictest/config_test.c`.
///
/// Verifies the getopt-style option string produced by
/// [`crate::config::Config::option_letters`] against a hard-coded
/// reference.  The canonical (sslkeylog-enabled) reference is used;
/// see the `#ifndef PICOQUIC_WITHOUT_SSLKEYLOG` note in the C source.
#[test]
fn config_option_letters() {
    let expected = "c:k:p:v:o:w:x:rR:s:XS:G:H:P:O:Me:C:i:l:Lb:q:m:n:a:t:zI:d:DQT:N:B:F:VU:0j:W:8J:E:y:K:Z:4:6:h";
    assert_eq!(Config::option_letters(), expected);
}

/// C: `config_option_test` in `picoquictest/config_test.c`.
///
/// Parses three option vectors through the Config command-line API and
/// compares the results against the expected field values from the C
/// `param1` and `param2` reference structs.  Also exercises a table of
/// known-bad option sequences that must all return errors.
#[test]
fn config_option() {
    // ARGV1 → param1 expected values.
    assert_param1(&parse_argv(ARGV1).expect("parse ARGV1"));

    // ARGV2 → param2 expected values.
    assert_param2(&parse_argv(ARGV2).expect("parse ARGV2"));

    // Long-form CONFIG_TWO → same param2 expected values.
    assert_param2(&parse_argv_ex(CONFIG_TWO).expect("parse CONFIG_TWO"));

    // Every entry in ERROR_CASES must fail to parse.
    for (i, args) in ERROR_CASES.iter().enumerate() {
        assert!(
            parse_argv(args).is_err(),
            "Expected parse error for case {i}: {:?}",
            args[0],
        );
    }
}

/// C: `config_quic_test` in `picoquictest/config_test.c`.
///
/// Builds a QUIC context from a parsed config and verifies that key
/// fields (max connections, ALPN, reset seed, congestion algorithm,
/// flow-control limits, and preferred address) are reflected correctly
/// in the resulting context.  Mirrors `config_quic_test_one` for two
/// distinct configs.
#[test]
fn config_quic() {
    fn config_quic_test_one(mut config: Config) {
        use crate::tests::util::{
            TEST_ECH_CONFIG, TEST_ECH_PRIVATE_KEY, TEST_FILE_CERT_STORE, TEST_FILE_SERVER_CERT,
            TEST_FILE_SERVER_KEY,
        };

        // Swap in test-fixture paths where the config has non-None file fields,
        // mirroring the C `picoquic_get_input_path` substitutions.
        if config.server_cert_file.is_some() {
            config.server_cert_file = Some(fixture_path(TEST_FILE_SERVER_CERT));
        }
        if config.server_key_file.is_some() {
            config.server_key_file = Some(fixture_path(TEST_FILE_SERVER_KEY));
        }
        if config.root_trust_file.is_some() {
            config.root_trust_file = Some(fixture_path(TEST_FILE_CERT_STORE));
        }
        if config.ech_key_file.is_some() {
            config.ech_key_file = Some(fixture_path(TEST_ECH_PRIVATE_KEY));
        }
        if config.ech_config_file.is_some() {
            config.ech_config_file = Some(fixture_path(TEST_ECH_CONFIG));
        }

        let quic = config
            .create_and_configure(None, Instant::from_ticks(0), None)
            .expect("create_and_configure");

        // Check max connections.
        if config.nb_connections > 0 {
            assert_eq!(
                quic.max_nb_connections(),
                config.nb_connections,
                "max_nb_connections"
            );
        }

        // Check default ALPN.
        if let Some(alpn) = config.alpn.as_deref() {
            assert_eq!(quic.default_alpn_string(), Some(alpn), "default_alpn");
        }

        // Check reset seed.
        if config.has_reset_seed {
            assert_eq!(
                quic.reset_seed_bytes(),
                config.reset_seed.as_ref(),
                "reset_seed"
            );
        }

        // Check default congestion algorithm.
        if let Some(cc_id) = config.cc_algo_id.as_deref() {
            assert_eq!(
                quic.default_congestion_algorithm_id(),
                Some(cc_id),
                "cc_algo_id"
            );
        }

        // Check flow-control / initial max data.
        if config.flow_control_max != 0 {
            assert_eq!(
                quic.max_data_limit(),
                config.flow_control_max,
                "max_data_limit"
            );
            assert_eq!(
                quic.default_tp().initial_max_data,
                config.flow_control_max,
                "initial_max_data"
            );
        } else {
            assert_eq!(quic.max_data_limit(), 0, "max_data_limit (default)");
            assert_eq!(
                quic.default_tp().initial_max_data,
                0x0010_0000,
                "initial_max_data (default)"
            );
        }

        // Check preferred address is populated when either V4 or V6 is set.
        if config.preferred_address_v4.is_some() || config.preferred_address_v6.is_some() {
            let pa = &quic.default_tp().preferred_address;
            assert!(
                pa.v4.is_some() || pa.v6.is_some(),
                "preferred_address should be defined"
            );
        }
    }

    config_test_register_cc_algorithms();
    config_quic_test_one(parse_argv(ARGV1).expect("param1 config"));
    config_quic_test_one(parse_argv(ARGV2).expect("param2 config"));
}

/// C: `config_usage_test` in `picoquictest/config_test.c`.
///
/// Writes the option help text via [`Config::write_usage`] and compares
/// it against the golden reference at
/// `rs/fq/tests/fixtures/config_usage_ref.txt` (copied from
/// `picoquictest/config_usage_ref.txt`).
#[test]
fn config_usage() {
    config_test_register_cc_algorithms();
    let mut buf = String::new();
    Config::write_usage(&mut buf);
    let expected = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/config_usage_ref.txt"
    ));
    assert_eq!(buf, expected);
}

/// C: `config_preferred_test` in `picoquictest/config_test.c`.
///
/// Exercises [`crate::utils::set_preferred_address`] with a table of
/// IPv4/IPv6 text strings and validates the resulting
/// [`PreferredAddress`] against expected socket addresses.
#[test]
fn config_preferred() {
    struct Case {
        name: &'static str,
        v4_text: Option<&'static str>,
        v6_text: Option<&'static str>,
        port: u16,
        is_valid: bool,
        expected_v4: Option<SocketAddr>,
        expected_v6: Option<SocketAddr>,
    }

    let v4_192_0_2_1: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 4433);
    let v6_2001_db8_1: SocketAddr = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1)),
        4433,
    );

    let cases = [
        Case {
            name: "none",
            v4_text: None,
            v6_text: None,
            port: 0,
            is_valid: true,
            expected_v4: None,
            expected_v6: None,
        },
        Case {
            name: "v4_only",
            v4_text: Some("192.0.2.1"),
            v6_text: None,
            port: 4433,
            is_valid: true,
            expected_v4: Some(v4_192_0_2_1),
            expected_v6: None,
        },
        Case {
            name: "v6_only",
            v4_text: None,
            v6_text: Some("2001:db8::1"),
            port: 4433,
            is_valid: true,
            expected_v4: None,
            expected_v6: Some(v6_2001_db8_1),
        },
        Case {
            name: "both",
            v4_text: Some("192.0.2.1"),
            v6_text: Some("2001:db8::1"),
            port: 4433,
            is_valid: true,
            expected_v4: Some(v4_192_0_2_1),
            expected_v6: Some(v6_2001_db8_1),
        },
        Case {
            name: "bad v4",
            v4_text: Some("192.a.b.c"),
            v6_text: Some("2001:db8::1"),
            port: 4433,
            is_valid: false,
            expected_v4: None,
            expected_v6: None,
        },
        Case {
            name: "bad v6",
            v4_text: Some("192.0.2.1"),
            v6_text: Some("2001:local"),
            port: 4433,
            is_valid: false,
            expected_v4: None,
            expected_v6: None,
        },
    ];

    for case in &cases {
        let mut preferred = PreferredAddress::default();
        let result = set_preferred_address(&mut preferred, case.v4_text, case.v6_text, case.port);
        let is_valid = result.is_ok();
        assert_eq!(
            is_valid, case.is_valid,
            "Test case '{}': validity mismatch",
            case.name
        );
        if is_valid {
            assert_eq!(
                preferred.v4, case.expected_v4,
                "Test case '{}': v4 mismatch",
                case.name
            );
            assert_eq!(
                preferred.v6, case.expected_v6,
                "Test case '{}': v6 mismatch",
                case.name
            );
        }
    }
}

/// C: `config_set_port_test` in `picoquictest/config_test.c`.
///
/// Exercises [`Config::set_port`] with a table of port-string formats and
/// verifies that `server_port`, `local_port`, and `is_port_shared` are
/// set correctly for each valid input, and that invalid inputs are
/// rejected.
#[test]
fn config_set_port() {
    // (port_string, is_valid, server_port, local_port, is_port_shared)
    let cases: &[(&str, bool, u16, u16, bool)] = &[
        ("4433", true, 4433, 0, false),
        ("443:4434", true, 443, 4434, false),
        ("S4433", true, 4433, 0, true),
        ("S443:4434", true, 443, 4434, true),
        ("S443:4434*", true, 443, 4434, true),
        ("S443:4434*1", true, 443, 4434, true),
        ("S443:4434*256", true, 443, 4434, true),
        ("4433*7", true, 4433, 0, false),
        ("", true, 0, 0, false),
        ("0", true, 0, 0, false),
        ("*5", true, 0, 0, false),
        ("0*3", true, 0, 0, false),
        ("65535", true, 65535, 0, false),
        ("S65535", true, 65535, 0, true),
        ("S65534:65535", true, 65534, 65535, true),
        ("65536", false, 0, 0, false),
        ("-1", false, 0, 0, false),
        ("abc", false, 0, 0, false),
        ("4433:abc", false, 0, 0, false),
        ("abc:4433", false, 0, 0, false),
        ("S4433:abc", false, 0, 0, false),
        ("Sabc:4433", false, 0, 0, false),
    ];

    for (i, &(port_string, is_valid, exp_server_port, exp_local_port, exp_is_port_shared)) in
        cases.iter().enumerate()
    {
        let mut config = Config::default();
        let result = config.set_port(port_string);
        let got_valid = result.is_ok();
        assert_eq!(
            got_valid, is_valid,
            "Case {i} ({port_string:?}): validity mismatch"
        );
        if got_valid {
            assert_eq!(
                config.server_port, exp_server_port,
                "Case {i} ({port_string:?}): server_port"
            );
            assert_eq!(
                config.local_port, exp_local_port,
                "Case {i} ({port_string:?}): local_port"
            );
            assert_eq!(
                config.is_port_shared, exp_is_port_shared,
                "Case {i} ({port_string:?}): is_port_shared"
            );
        }
    }
}
