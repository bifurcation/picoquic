//! Test cases for `picoquictest/util_test.c`.

#![allow(non_snake_case)]

use crate::ConnectionId;

const EXPECTED_CIDS: &[(&[u8], &str)] = &[
    (
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        "000102030405060708090a0b0c0d0e0f",
    ),
    (
        &[0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10],
        "fedcba9876543210",
    ),
    (&[0xca, 0xfe], "cafe"),
    (&[0x77], "77"),
];

/// C: `util_connection_id_print_test` in `picoquictest/util_test.c`.
///
/// `picoquic_print_connection_id_hexa(buf, sizeof, &cid)` formats
/// the connection-id bytes as lowercase hex.  Rust callers do this
/// inline with `core::fmt::Write` over `cid.as_bytes()`.
#[test]
fn connection_id_print() {
    use core::fmt::Write;
    for (bytes, expected) in EXPECTED_CIDS {
        let cid = ConnectionId::clone_from_slice(bytes).expect("CID under cap");
        let mut got = String::new();
        for b in cid.as_bytes() {
            write!(&mut got, "{b:02x}").unwrap();
        }
        assert_eq!(got, *expected, "CID hex round-trip");
    }
}

/// C: `util_connection_id_parse_test` in `picoquictest/util_test.c`.
///
/// Parse a hex string back into a [`ConnectionId`].  Picoquic exposes
/// `picoquic_parse_connection_id_hexa(text, len, &cid)`; the Rust
/// equivalent is a small `from_str_radix` loop over byte pairs.
#[test]
fn connection_id_parse() {
    for (bytes, hex) in EXPECTED_CIDS {
        let mut decoded = [0u8; 20];
        let n = hex.len() / 2;
        for i in 0..n {
            decoded[i] = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("hex byte");
        }
        let parsed = ConnectionId::clone_from_slice(&decoded[..n]).expect("CID under cap");
        let expected = ConnectionId::clone_from_slice(bytes).expect("CID under cap");
        assert_eq!(parsed, expected, "CID parse roundtrip for {hex}");
    }
}

/// C: `util_uint8_to_str_test` in `picoquictest/util_test.c`.
///
/// Exercises [`crate::utils::uint8_to_str`] at three buffer sizes
/// (full, truncated to 7, truncated to 2).  The function copies
/// printable bytes verbatim and substitutes `'.'` for unprintable
/// ones, terminating with a NUL the C version stripped via
/// `strcmp`.
#[test]
fn util_uint8_to_str() {
    let input: &[u8] = b"azAZ09.\xff";

    let mut buf = [0u8; 16];
    let out = crate::utils::uint8_to_str(&mut buf, input);
    assert_eq!(out, b"azAZ09.?", "full-width output");

    let mut buf7 = [0u8; 7];
    let out = crate::utils::uint8_to_str(&mut buf7, input);
    assert_eq!(out, b"azA...", "7-byte output");

    let mut buf2 = [0u8; 2];
    let out = crate::utils::uint8_to_str(&mut buf2, input);
    assert_eq!(out, b".", "2-byte output");
}

/// C: `util_memcmp_test` in `picoquictest/util_test.c`.
///
/// The C body verifies [`crate::utils::constant_time_memcmp`]
/// returns equal/unequal for matching/differing 16-byte slices over
/// a 1 MB working set, plus rough timing measurements (skipped — the
/// timing portion was always informational and noise-prone).  We
/// keep just the correctness loop.
#[test]
fn util_memcmp() {
    use core::cmp::Ordering;

    use crate::utils::constant_time_memcmp;

    let nb_words = (1 << 17) / 8;
    let l_total = nb_words * 8;
    let mut x = vec![0u8; l_total];
    // Borrow a deterministic test RNG so the data isn't all zeros.
    let mut seed = 0xbabac001u64;
    let next = |seed: &mut u64| -> u64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *seed
    };
    for chunk in x[32..].chunks_mut(8) {
        chunk.copy_from_slice(&next(&mut seed).to_le_bytes());
    }
    x[16..32].copy_from_slice(&[0xff; 16]);

    let y = x.clone();

    // Equality detection.
    let mut j = 0;
    while j < l_total {
        assert_eq!(
            constant_time_memcmp(&x[j..j + 16], &y[j..j + 16]),
            Ordering::Equal,
            "unexpected mismatch at byte {j}",
        );
        j += 16;
    }

    // Difference detection: flip one byte per 16-byte group.
    for offset in 0..16 {
        let mut y = x.clone();
        let mut j = offset;
        while j < l_total {
            y[j] ^= 0xff;
            j += 16;
        }
        let mut j = 0;
        while j < l_total {
            assert_ne!(
                constant_time_memcmp(&x[j..j + 16], &y[j..j + 16]),
                Ordering::Equal,
                "unexpected match at offset={offset}, byte={j}",
            );
            j += 16;
        }
    }
}

/// C: `util_sprintf_test` in `picoquictest/util_test.c`.
///
/// The C `picoquic_sprintf` is a thin `snprintf` wrapper with a
/// "buffer too small" failure indicator.  Rust call sites pass an
/// already formatted string to [`crate::utils::sprintf`], which keeps
/// the same success/failure boundary around the terminating NUL.
#[test]
fn util_sprintf() {
    use crate::utils::{FILE_SEPARATOR, sprintf};

    let mut str_buf = [0u8; 8];
    let n = sprintf(&mut str_buf, &format!("{}{}", "foo", "bar")).expect("'foobar'");
    assert_eq!(n, 6);
    assert_eq!(&str_buf[..7], b"foobar\0");

    let mut str_buf = [0u8; 8];
    let n = sprintf(
        &mut str_buf,
        &format!("{}{}{}", "foo", FILE_SEPARATOR, "bar"),
    )
    .expect("'foo/bar'");
    assert_eq!(n, 7);
    assert_eq!(&str_buf, b"foo/bar\0");

    let mut str_buf = [0u8; 8];
    assert!(
        sprintf(
            &mut str_buf,
            &format!("{}{}{}", "fooo", FILE_SEPARATOR, "bar")
        )
        .is_err(),
        "'fooo/bar' should not fit"
    );

    let mut str_buf = [0u8; 8];
    assert!(
        sprintf(
            &mut str_buf,
            &format!("{}{}{}", "fooo", FILE_SEPARATOR, "barr")
        )
        .is_err(),
        "'fooo/barr' should not fit"
    );
}

/// C: `util_debug_print_test` in `picoquictest/util_test.c`.
///
/// The C body redirects the global `debug_set_stream` sink to a
/// file and verifies a write goes through.  The Rust debug sink owns
/// a `core::fmt::Write`, so the test wraps a `std::fs::File`.
#[test]
fn util_debug_print() {
    use std::io::Write as _;

    struct DebugFile(std::fs::File);

    impl core::fmt::Write for DebugFile {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.0.write_all(s.as_bytes()).map_err(|_| core::fmt::Error)
        }
    }

    const FILE_TEST_DEBUG: &str = "file_test_debug.txt";

    let was_suspended = crate::utils::debug_printf_reset(false);
    let file = std::fs::File::create(FILE_TEST_DEBUG).expect("create debug output");
    crate::utils::debug_set_stream(Some(Box::new(DebugFile(file))));
    crate::utils::debug_printf("debug set stream: 0\n");
    crate::utils::debug_set_stream(None);
    crate::utils::debug_printf_reset(was_suspended);

    let written = std::fs::read_to_string(FILE_TEST_DEBUG).expect("read debug output");
    assert_eq!(written, "debug set stream: 0\n");
}

/// C: `util_threading_test` in `picoquictest/util_test.c`.
///
/// The C body exercises `picoquic_create_mutex` / `_event` /
/// `_thread`.  Rust uses the corresponding standard primitives:
/// `Mutex` for the shared counter, `Condvar` for the event, and
/// `std::thread::spawn` for the worker.
#[test]
fn threading() {
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    #[derive(Default)]
    struct ThreadTestData {
        data: u64,
    }

    let ctx = Arc::new((Mutex::new(ThreadTestData::default()), Condvar::new()));
    let worker_ctx = Arc::clone(&ctx);
    let thread = std::thread::spawn(move || {
        let (mutex, event) = &*worker_ctx;
        for _ in 0..20 {
            let mut guard = mutex.lock().expect("worker mutex");
            let x = guard.data;
            guard.data = x + 1;
            drop(guard);
            event.notify_one();
        }
    });

    let (mutex, event) = &*ctx;
    let mut guard = mutex.lock().expect("main mutex");
    while guard.data == 0 {
        let wait = event
            .wait_timeout(guard, Duration::from_micros(10_000))
            .expect("wait for first event");
        guard = wait.0;
        assert!(
            !wait.1.timed_out() || guard.data != 0,
            "first event timed out"
        );
    }
    drop(guard);

    for _ in 0..10 {
        let mut guard = mutex.lock().expect("main increment mutex");
        let x = guard.data;
        guard.data = x + 1;
    }

    let mut guard = mutex.lock().expect("final wait mutex");
    while guard.data < 30 {
        let wait = event
            .wait_timeout(guard, Duration::from_micros(10_000))
            .expect("wait for final event");
        guard = wait.0;
        assert!(
            !wait.1.timed_out() || guard.data >= 30,
            "final event timed out"
        );
    }
    let data = guard.data;
    drop(guard);

    thread.join().expect("join worker thread");
    assert_eq!(data, 30);
}
