#!/usr/bin/env python3
"""strip_pico.py — one-shot rename: drop pico/picoquic from all
identifiers in rs/fq/src/.

Per the consistency-review policy, the Rust translation should
have no `pico` or `picoquic` substrings anywhere — Rust modules
provide the namespace, the prefixes are vestigial C-name
disambiguation.

Approach: walk every .rs file, apply three substitutions in
order, with `\\b` word-boundary anchoring:

  1. `picoquic_<X>` → `<X>`   (most common: snake_case identifiers)
  2. `picoquic<X>`  → `<X>`   (handles `picoquictest_…`, `Picoquic…`)
  3. `pico<X>`      → `<X>`   (handles `picohash`, `picosocks`,
                              `picosplay`, `picoquic` alone)

Order matters: the longer prefixes are stripped first so we
never partially-strip a word.

Usage:
  python3 scripts/strip_pico.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
RS_SRC    = REPO_ROOT / "rs" / "fq" / "src"

# Match `picoquic_X`, `picoquicX`, `picoX` where X is the remainder
# of an identifier.  Word boundary at the start ensures we don't
# strip middle-of-word matches.  At the end we don't anchor — the
# captured group continues until something that's not an identifier
# character.

# `\b` works on identifier boundaries in Python's re, treating
# letters / digits / underscore as word chars.

PASSES = [
    # Word-start (\b) cases.
    # 1. picoquic_<rest> / PICOQUIC_<rest> / Picoquic_<rest> —
    #    snake-case and SCREAMING_SNAKE_CASE identifiers.
    (re.compile(r"\bpicoquic_(\w+)", re.IGNORECASE), r"\1"),
    # 2. picoquic<rest> with no underscore (`picoquictest`,
    #    `PicoquicError` if any).  Strip the 8-letter prefix.
    (re.compile(r"\bpicoquic(\w+)",  re.IGNORECASE), r"\1"),
    # 3. pico<rest> — `picohash`, `picosocks`, `picosplay`,
    #    `picoquic` alone, `picotls` from prose; plus uppercase /
    #    PascalCase variants.
    (re.compile(r"\bpico(\w+)",      re.IGNORECASE), r"\1"),

    # Middle-of-identifier cases (preceded by underscore — \b
    # doesn't fire between two word chars).  Examples:
    # `textlog_picotls_ticket`, `st_picoquic_network_thread_ctx_t`.
    (re.compile(r"_picoquic_(\w+)", re.IGNORECASE), r"_\1"),
    (re.compile(r"_picoquic(\w+)",  re.IGNORECASE), r"_\1"),
    (re.compile(r"_pico(\w+)",      re.IGNORECASE), r"_\1"),
]


def strip(text: str) -> str:
    for rx, repl in PASSES:
        text = rx.sub(repl, text)
    return text


def main() -> int:
    if not RS_SRC.is_dir():
        print(f"missing {RS_SRC}", file=sys.stderr)
        return 1
    changed = 0
    skipped = 0
    for path in sorted(RS_SRC.rglob("*.rs")):
        before = path.read_text()
        after = strip(before)
        if before == after:
            skipped += 1
            continue
        path.write_text(after)
        changed += 1
        # Per-file diff stat — count lines that changed.
        bl = before.splitlines()
        al = after.splitlines()
        same_len = len(bl) == len(al)
        diff_lines = sum(1 for x, y in zip(bl, al) if x != y) if same_len else "?"
        print(f"  edited {path.relative_to(REPO_ROOT)}  "
              f"({diff_lines} lines changed)")
    print(f"done: {changed} edited, {skipped} unchanged")
    return 0


if __name__ == "__main__":
    sys.exit(main())
