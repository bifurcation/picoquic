#!/usr/bin/env python3
"""Phase 3: generate stub `#[test]` functions for every C test entry.

Driver:

  * Parse picoquic_t/picoquic_t.c's `test_table[]` to extract every
    (test_name, entry_fn) pair.
  * For each entry_fn, locate its definition under picoquictest/.
  * Map the source file to a Rust module (a hand-tuned table; see
    SRC_TO_MOD).  Files we can't classify go to `crate::tests`
    (cross-cutting bucket) — explicit, not a fallthrough.
  * Emit one `#[test] fn <test_name>() { todo!(\"<entry_fn>\") }`
    stub per row, grouped by destination module.
  * Each destination module gets its tests written to
    `rs/fq/src/tests/phase3_<module>.rs`, included from
    `rs/fq/src/tests/mod.rs`.

Phase 3 acceptance gate (per TRANSLATE_PLAN.md):
  * `cargo test --no-run` compiles.
  * `cargo test` runs to completion; every test panics on todo!().

Run from repo root.
"""

from __future__ import annotations

import re
import sys
from collections import defaultdict
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
DRIVER = REPO_ROOT / "picoquic_t" / "picoquic_t.c"
PICOQUICTEST = REPO_ROOT / "picoquictest"
RS_TESTS = REPO_ROOT / "rs" / "fq" / "src" / "tests"

# Map a picoquictest/<file>.c source name (without `.c`) to the Rust
# module that should host the test.  Only files that have a clear
# Rust counterpart are listed; everything else lands in the
# `crate::tests` cross-cutting bucket.
#
# Keys are basenames without extension.  Values are dotted Rust
# module paths *relative to the crate root* (so "bytestream" means
# `crate::bytestream`, "tests::dualq" means `crate::tests::dualq`).
SRC_TO_MOD: dict[str, str] = {
    "bytestream_test": "bytestream",
    "splay_test": "splay",
    "hashtest": "hash",
    "siphash_test": "siphash",
    "config_test": "config",
    "config_option_test": "config",
    "logger_test": "logger",
    "binlog_test": "binlog",
    "qlog_test": "qlog",
    "lb_compat_test": "lb",
    "sockloop_test": "packet_loop",
    "dualq_aqm_test": "tests::dualq",
    "sim_link_test": "tests::util",
    "util_test": "utils",
    "code_version_test": "lib",
    # The remaining picoquictest sources are cross-cutting tests
    # against the QUIC stack; they land in `crate::tests`.
}

# Test-name fixups: a handful of C identifiers aren't valid Rust
# identifiers as-is (`pn2pn64test`, `cplusplus`, `TlsStreamFrame`).
# We sanitize by lowercasing and replacing non-ident chars with
# `_`; if a fixup is also reserved Rust (`type`, `match`, etc.), we
# prefix with `r#`.  None show up today but the loop guards anyway.
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
    """Make `name` a valid Rust identifier."""
    out = re.sub(r"[^A-Za-z0-9_]", "_", name).lower()
    if not out:
        out = "test"
    if out[0].isdigit():
        out = "_" + out
    if out in RUST_KEYWORDS:
        out = f"r#{out}"
    return out


def parse_test_table() -> list[tuple[str, str]]:
    """Return the (test_name, entry_fn) pairs from test_table[]."""
    text = DRIVER.read_text()
    # Find the table block.
    start = re.search(r"static const \S+ test_table\[\]\s*=\s*\{", text)
    if not start:
        raise SystemExit("error: test_table[] not found in picoquic_t.c")
    # Match rows like:  { "name", entry_fn },
    row_pat = re.compile(
        r"\{\s*\"(?P<name>[^\"]+)\"\s*,\s*(?P<fn>[A-Za-z_][A-Za-z0-9_]*)\s*\}",
    )
    rows: list[tuple[str, str]] = []
    pos = start.end()
    while True:
        # Stop when we hit the closing `};` for the table.
        end = text.find("};", pos)
        m = row_pat.search(text, pos, end if end > 0 else None)
        if not m:
            break
        rows.append((m.group("name"), m.group("fn")))
        pos = m.end()
    return rows


# Hard-coded source overrides for entry fns the regex misses
# (defined inside `extern "C"` blocks, in headers, or in
# the picoquic_t driver itself).
EXTRA_FN_SOURCES = {
    "cplusplustest": "cplusplus",
    # `sim_link_test` is only declared in picoquictest.h; the
    # implementation lives in the picoquic_t driver harness
    # itself.  Map to a sentinel; it falls into `crate::tests`.
    "sim_link_test": "<harness>",
}


def index_entry_fns() -> dict[str, str]:
    """Map entry_fn name -> source-file basename (no .c) under picoquictest/."""
    # Match top-level function *definitions* only: a name followed by
    # `(...) {` (with optional whitespace / newlines).  Declarations
    # in headers end with `;` and are filtered out.
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
            # Record the first definition site for each name.  C/C++
            # control-flow keywords (`if`, `while`, `for`, `switch`,
            # `return`, etc.) can appear in this position too — skip
            # them.
            if fn in fn_to_src or fn in {
                "if", "while", "for", "switch", "return", "do", "else",
                "case", "default", "break", "continue", "goto", "sizeof",
            }:
                continue
            fn_to_src[fn] = src.stem
    for fn, src_name in EXTRA_FN_SOURCES.items():
        fn_to_src.setdefault(fn, src_name)
    return fn_to_src


def module_for(src_basename: str) -> str:
    """Return the Rust module path for a picoquictest source file."""
    return SRC_TO_MOD.get(src_basename, "tests")


def main() -> int:
    rows = parse_test_table()
    fn_to_src = index_entry_fns()

    # Group (test_name, entry_fn, src_file) by destination module.
    by_module: dict[str, list[tuple[str, str, str]]] = defaultdict(list)
    unresolved: list[tuple[str, str]] = []
    for name, entry_fn in rows:
        src = fn_to_src.get(entry_fn)
        if src is None:
            unresolved.append((name, entry_fn))
            module = "tests"
            src_label = "<unknown>"
        else:
            module = module_for(src)
            src_label = src
        by_module[module].append((name, entry_fn, src_label))

    # Emit one file per module under rs/fq/src/tests/, including
    # one for `tests` itself (the cross-cutting bucket).
    RS_TESTS.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for module, entries in sorted(by_module.items()):
        # Filename: `phase3_<module-with-underscores>.rs`.  For
        # nested paths like `tests::dualq` we use the leaf.
        leaf = module.split("::")[-1]
        filename = RS_TESTS / f"phase3_{leaf}.rs"
        lines = [
            f"//! Phase 3 test stubs for `crate::{module}`.\n",
            "//!\n",
            "//! Auto-generated by `scripts/phase3.py` from\n",
            "//! `picoquic_t/picoquic_t.c`'s `test_table[]`.  Each stub\n",
            "//! corresponds to one C test entry; bodies are `todo!()`\n",
            "//! until Phase 3 body translation lands.\n",
            "\n",
            "#![allow(non_snake_case)]\n",
            "\n",
        ]
        seen: set[str] = set()
        for name, entry_fn, src in sorted(entries):
            ident = sanitize_test_name(name)
            # Disambiguate collisions (different C names that
            # sanitize to the same Rust identifier).
            base = ident
            n = 2
            while ident in seen:
                ident = f"{base}_{n}"
                n += 1
            seen.add(ident)
            lines.append(f"/// C: `{entry_fn}` in `picoquictest/{src}.c`.\n")
            lines.append("#[test]\n")
            lines.append(f"fn {ident}() {{\n")
            lines.append(f'    todo!("{entry_fn}")\n')
            lines.append("}\n\n")
        filename.write_text("".join(lines))
        written.append(filename)

    # Print a summary.
    total = sum(len(v) for v in by_module.values())
    print(f"wrote {total} stubs across {len(written)} files:")
    for path in written:
        n = sum(1 for line in path.read_text().splitlines() if line.startswith("#[test]"))
        print(f"  {path.relative_to(REPO_ROOT)}: {n} stubs")
    if unresolved:
        print()
        print(f"warning: {len(unresolved)} entry fns not located under picoquictest/:")
        for name, fn in unresolved[:10]:
            print(f"  {name} -> {fn}")
        if len(unresolved) > 10:
            print(f"  ... and {len(unresolved) - 10} more")
    return 0


if __name__ == "__main__":
    sys.exit(main())
