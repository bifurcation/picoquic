//! Test cases for `picoquictest/qlog_frame_test.c`.

#![allow(non_snake_case)]

use crate::qlog::qlog_frames as qlog_frames_write;

/// One entry from the C `test_skip_list[]` array in `skip_frame_test.c`.
/// C: `test_skip_frames_t`.
struct SkipFrameEntry {
    name: &'static str,
    val: &'static [u8],
}

/// The frame test vectors from `picoquictest/skip_frame_test.c`.
/// C: `test_skip_list[]` / `nb_test_skip_list`.
///
/// Not yet translated to Rust — returns `todo!()` until `skip_frame.rs`
/// is fully implemented.
fn test_skip_list() -> Vec<SkipFrameEntry> {
    todo!("test_skip_list — skip_frame_test.c not yet translated")
}

/// Compare two text files byte-for-byte.
/// C: `picoquic_test_compare_text_files`.
fn compare_text_files(_a: &str, _b: &str) -> crate::Result<()> {
    todo!("compare_text_files — test helper")
}

/// Expand a path relative to the picoquic solution directory.
/// C: `picoquic_get_input_path`.
fn get_input_path(_fragment: &str) -> crate::Result<String> {
    todo!("get_input_path — test helper")
}

/// C: `qlog_frames_test` in `picoquictest/qlog_frame_test.c`.
#[test]
fn qlog_frames() {
    let test_ref = get_input_path("picoquictest/qlog_frames_test_ref.txt")
        .expect("resolve qlog frames ref path");

    let mut out: Vec<u8> = Vec::new();
    let list = test_skip_list();

    let mut need_comma = "";
    out.extend_from_slice(b"[\n");
    for entry in &list {
        out.extend_from_slice(need_comma.as_bytes());
        out.extend_from_slice(format!("{{ \"test\": \"{}\", \"frame\": ", entry.name).as_bytes());
        need_comma = ",\n";

        qlog_frames_write(&mut out, entry.val, false);
        out.extend_from_slice(b"}");
    }
    out.extend_from_slice(b"\n]\n");

    let output_file = "qlog_frames_test.json";
    std::fs::write(output_file, &out).expect("write qlog frames test output");

    compare_text_files(&test_ref, output_file).expect("qlog frames output matches reference");
}
