#!/usr/bin/env python3
"""phase3_check.py — verify that every C test in the named source
files has a translated Rust body.

Usage:
  python3 scripts/phase3_check.py                     # check all sources
  python3 scripts/phase3_check.py sacktest config_test # check named sources

Exit code 0 if every test has a real body; non-zero otherwise.

A test counts as "translated" when its `#[test] fn <name>()` body
in `rs/fq/src/tests/<rust>.rs` is anything other than the bare
auto-stub pattern:

    #[test]
    fn <name>() {
        todo!("<entry_fn>")
    }

Tests with a non-todo body (assertions, real calls, even an
explicit `todo!()` followed by a comment / multiple lines that
*do* make the test signal richer) all count as translated.

The script reads `picoquic_t/picoquic_t.c`'s `test_table[]` and
the `xlate/prompts/phase3a/<src>.md` files (which list each
source's expected entries) to know which `#[test] fn` names to
expect in each `tests/<rust>.rs`.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DRIVER = REPO_ROOT / "picoquic_t" / "picoquic_t.c"
PICOQUICTEST = REPO_ROOT / "picoquictest"
RS_TESTS = REPO_ROOT / "rs" / "fq" / "src" / "tests"

# Same map as phase3a.py.
RUST_FILENAME_OVERRIDES = {
    "util_test": "util_test",
}

RUST_KEYWORDS = {
    "as", "break", "const", "continue", "crate", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
    "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
    "super", "trait", "true", "type", "unsafe", "use", "where", "while",
    "async", "await", "dyn", "abstract", "become", "box", "do", "final",
    "macro", "override", "priv", "typeof", "unsized", "virtual", "yield",
    "try",
}


def sanitize_test_name(name: str) -> str:
    out = re.sub(r"[^A-Za-z0-9_]", "_", name).lower()
    if not out:
        out = "test"
    if out[0].isdigit():
        out = "_" + out
    if out in RUST_KEYWORDS:
        out = f"r#{out}"
    return out


def rust_filename_for(src: str) -> str:
    if src in RUST_FILENAME_OVERRIDES:
        return RUST_FILENAME_OVERRIDES[src]
    name = src
    for suffix in ("_tests", "_test"):
        if name.endswith(suffix):
            name = name[: -len(suffix)]
            break
    return name


def parse_test_table() -> list[tuple[str, str]]:
    text = DRIVER.read_text()
    start = re.search(r"static const \S+ test_table\[\]\s*=\s*\{", text)
    if not start:
        raise SystemExit("error: test_table[] not found")
    row_pat = re.compile(
        r"\{\s*\"(?P<name>[^\"]+)\"\s*,\s*(?P<fn>[A-Za-z_][A-Za-z0-9_]*)\s*\}",
    )
    rows: list[tuple[str, str]] = []
    pos = start.end()
    while True:
        end = text.find("};", pos)
        m = row_pat.search(text, pos, end if end > 0 else None)
        if not m:
            break
        rows.append((m.group("name"), m.group("fn")))
        pos = m.end()
    return rows


def index_entry_fns() -> dict[str, str]:
    fn_to_src: dict[str, str] = {}
    pat = re.compile(
        r"^[A-Za-z_][A-Za-z0-9_ \t\*]*\b([A-Za-z_][A-Za-z0-9_]*)\s*\([^;]*?\)\s*\{",
        re.MULTILINE | re.DOTALL,
    )
    sources = sorted(PICOQUICTEST.glob("*.c")) + sorted(PICOQUICTEST.glob("*.cpp"))
    for src in sources:
        text = src.read_text(errors="replace")
        for m in pat.finditer(text):
            fn = m.group(1)
            if fn in fn_to_src or fn in {
                "if", "while", "for", "switch", "return", "do", "else",
                "case", "default", "break", "continue", "goto", "sizeof",
            }:
                continue
            fn_to_src[fn] = src.stem
    fn_to_src.setdefault("cplusplustest", "cplusplus")
    fn_to_src.setdefault("sim_link_test", "<harness>")
    return fn_to_src


def group_by_source() -> dict[str, list[tuple[str, str]]]:
    rows = parse_test_table()
    fn_to_src = index_entry_fns()
    by_src: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for name, fn in rows:
        src = fn_to_src.get(fn, "<unknown>")
        if src.startswith("<"):
            continue
        by_src[src].append((name, fn))
    return dict(by_src)


# ---------------------------------------------------------------------------
# The actual check: parse a Rust test file and return the set of
# `#[test] fn` names whose bodies are still the auto-stub.

# Match the bare auto-stub: `#[test]\nfn <name>() {\n    todo!("<fn>")\n}`.
# Tolerant of extra `#[allow(...)]` lines above and minor formatting.
STUB_RE = re.compile(
    r"#\[test\]\s*\n"                       # the attribute
    r"\s*fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_#]*)\s*\(\s*\)\s*\{\s*"
    r"todo!\s*\(\s*\"[^\"]*\"\s*\)\s*\n?\s*\}",
    re.MULTILINE,
)

# Regex that finds *every* `#[test] fn <name>` declaration so we can
# verify the file even has the test (we don't want to accept "test
# missing entirely" as success).
TEST_DECL_RE = re.compile(
    r"#\[test\]\s*\n\s*fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_#]*)\s*\(",
    re.MULTILINE,
)


def stub_names_in(rust_path: Path) -> set[str]:
    text = rust_path.read_text()
    return {m.group("name") for m in STUB_RE.finditer(text)}


def declared_names_in(rust_path: Path) -> set[str]:
    text = rust_path.read_text()
    return {m.group("name") for m in TEST_DECL_RE.finditer(text)}


def check_source(src: str, by_src: dict[str, list[tuple[str, str]]]) -> tuple[int, list[str]]:
    """Return (missing_count, lines) where lines is human-readable."""
    rust_path = RS_TESTS / f"{rust_filename_for(src)}.rs"
    if not rust_path.exists():
        return 1, [f"  {src}: missing test file at {rust_path.relative_to(REPO_ROOT)}"]
    expected = {sanitize_test_name(name): fn for name, fn in by_src.get(src, [])}
    if not expected:
        return 1, [f"  {src}: no entries known"]
    declared = declared_names_in(rust_path)
    stubbed = stub_names_in(rust_path)
    issues: list[str] = []
    for ident, fn in sorted(expected.items()):
        if ident not in declared:
            issues.append(f"  {src}::{ident} (C `{fn}`): MISSING `#[test] fn`")
        elif ident in stubbed:
            issues.append(f"  {src}::{ident} (C `{fn}`): still a `todo!()` stub")
    return len(issues), issues


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("sources", nargs="*",
                   help="Source basenames (e.g. sacktest, config_test).  "
                        "Default: all 56 sources.")
    p.add_argument("--quiet", action="store_true",
                   help="Print only sources with missing translations.")
    args = p.parse_args()

    by_src = group_by_source()
    targets = args.sources if args.sources else sorted(by_src.keys())

    total_missing = 0
    total_done = 0
    issues_by_src: dict[str, list[str]] = {}
    for src in targets:
        if src not in by_src:
            print(f"WARNING: unknown source {src!r}", file=sys.stderr)
            continue
        missing, lines = check_source(src, by_src)
        total_missing += missing
        if missing == 0:
            total_done += 1
            if not args.quiet:
                print(f"OK    {src}")
        else:
            issues_by_src[src] = lines
            print(f"FAIL  {src}  ({missing} missing)")

    if issues_by_src:
        print("\nDetails:")
        for src, lines in sorted(issues_by_src.items()):
            for line in lines:
                print(line)

    print(f"\nSummary: {total_done}/{len(targets)} sources fully translated.  "
          f"{total_missing} test stubs still need bodies.")
    return 0 if total_missing == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
