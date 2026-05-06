//! Test cases for `picoquictest/picolog*.c`.
//!
//! `picolog_basic_test` exercises `cidset`, `logreader`, and `svg_convert`
//! from `loglib/`, which is out of scope for v1.  The body is translated
//! faithfully with `todo!()` stubs standing in for those out-of-scope symbols.

#![allow(non_snake_case)]

use crate::Instant;

// ---------------------------------------------------------------------------
// Out-of-scope loglib stubs.

/// Opaque handle to a set of connection IDs read from a binlog.
/// C: `picohash_table*` (cidset).
struct CidSet;

/// Create an empty [`CidSet`].  C: `cidset_create`.
fn cidset_create() -> Option<Box<CidSet>> {
    todo!("cidset_create — loglib out of scope")
}

/// Parse `f_binlog` and populate `cids` with every CID observed.
/// C: `binlog_list_cids`.
fn binlog_list_cids(_f_binlog: &mut std::fs::File, _cids: &mut CidSet) {
    todo!("binlog_list_cids — loglib out of scope")
}

/// Write a human-readable dump of `cids` to `out`.
/// C: `cidset_print`.
fn cidset_print(_out: &mut std::fs::File, _cids: &CidSet) {
    todo!("cidset_print — loglib out of scope")
}

/// Return `true` if `cid` appears in `cids`.
/// C: `cidset_has_cid`.
fn cidset_has_cid(_cids: &CidSet, _cid: &crate::ConnectionId) -> bool {
    todo!("cidset_has_cid — loglib out of scope")
}

/// Iterate over every CID in `cids`, calling `f(cid, ctx)` for each.
/// C: `cidset_iterate`.
fn cidset_iterate(
    _cids: &CidSet,
    _f: fn(&crate::ConnectionId, &mut SvgConvertCtx) -> i32,
    _ctx: &mut SvgConvertCtx,
) -> i32 {
    todo!("cidset_iterate — loglib out of scope")
}

/// Context passed to the per-CID conversion callback.
/// C: `app_conversion_context_t`.
#[allow(dead_code)]
struct SvgConvertCtx {
    binlog_name: String,
    out_dir: &'static str,
    f_binlog: Option<std::fs::File>,
    f_template: Option<std::fs::File>,
}

/// Convert one connection's trace from binlog to SVG.
/// C: `svg_convert` (loglib).
fn svg_convert(_cid: &crate::ConnectionId, _ctx: &mut SvgConvertCtx) -> i32 {
    todo!("svg_convert — loglib out of scope")
}

/// Compare two text files byte-for-byte.
/// C: `picoquic_test_compare_text_files`.
fn compare_text_files(_a: &str, _b: &str) -> crate::Result<()> {
    todo!("compare_text_files — test helper out of scope")
}

/// Expand a path relative to the picoquic solution directory.
/// C: `picoquic_get_input_path`.
fn get_input_path(_fragment: &str) -> crate::Result<String> {
    todo!("get_input_path — test helper out of scope")
}

// ---------------------------------------------------------------------------

/// C: `picolog_basic_test` in `picoquictest/picolog_test.c`.
#[test]
fn picolog_basic() {
    let mut _simulated_time = Instant::from_ticks(0);

    let log_input =
        get_input_path("picoquictest/picolog_test_input.log").expect("get log input path");
    let svg_template_path = get_input_path("loglib/template.svg").expect("get svg template path");

    let mut f_binlog = std::fs::File::open(&log_input).expect("open binlog");
    let f_template = std::fs::File::open(&svg_template_path).expect("open svg template");

    let mut cids = cidset_create().expect("cidset_create");

    binlog_list_cids(&mut f_binlog, &mut cids);
    assert!(!matches!(cids, _), "cids must not be empty");

    {
        let mut cid_prints = std::fs::File::create("./cidset.txt").expect("create cidset output");
        cidset_print(&mut cid_prints, &cids);
    }

    let cid_test = crate::ConnectionId::clone_from_slice(&[11, 12, 13, 14, 15, 16, 17, 18])
        .expect("build test CID");
    assert!(!cidset_has_cid(&cids, &cid_test), "unexpected CID in set");

    let mut ctx = SvgConvertCtx {
        binlog_name: log_input.clone(),
        out_dir: ".",
        f_binlog: Some(std::fs::File::open(&log_input).expect("reopen binlog")),
        f_template: Some(f_template),
    };
    let ret = cidset_iterate(&cids, svg_convert, &mut ctx);
    assert_eq!(ret, 0, "cidset_iterate failed");

    let svglog_ref = get_input_path("picoquictest/svglog_ref.svg").expect("get svglog ref path");
    compare_text_files("./0102030405060708.svg", &svglog_ref)
        .expect("svg output matches reference");
}
