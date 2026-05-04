#!/usr/bin/env python3
"""Audit `&mut self` methods in `rs/fq/src/internal.rs` and rewrite
clearly-read-only ones to `&self`.

Heuristic: methods whose names start with read-only prefixes
(`is_`, `get_`, `find_`, `peek_`, `current_`, `first_`, `last_`,
`next_`, `previous_`, `iter_`, `count_`) plus a hand-curated set
of one-off names known to be reads.

Bodies are `todo!()` (Phase 1 contract); receiver mutability is
purely a signature concern, so this is safe.

Run from rs/fq/.  The script logs every change and prints the
total.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

INTERNAL_RS = Path("src/internal.rs")

IMMUTABLE_PREFIXES = (
    "is_",
    "get_",
    "find_",
    "peek_",
    "current_",
    "first",       # `first`, `first_item`, `first_range`, `first_stream`, `first_data_repeat_packet`
    "last",        # `last`, `last_item`, `last_stream`
    "next_",
    "previous_",
    "iter_",
    "count_",
    "has_",
    "size",
    "len",
    "all_",
    "any_",
    "should_",
    "compute_",
    "encode_time_stamp_length",  # explicit name
    "cc_increased_window",
)

IMMUTABLE_NAMES = {
    "first",
    "last",
    "size",
    "is_empty",
}

NEEDLE = re.compile(
    r"(\bpub fn (?P<name>[A-Za-z_][A-Za-z0-9_]*)\(\s*)&mut self\b",
)


def is_immutable(name: str) -> bool:
    if name in IMMUTABLE_NAMES:
        return True
    return name.startswith(IMMUTABLE_PREFIXES)


def main() -> int:
    if not INTERNAL_RS.exists():
        print(f"error: {INTERNAL_RS} not found; run from rs/fq/", file=sys.stderr)
        return 1
    text = INTERNAL_RS.read_text()
    changed = 0
    changed_names: list[str] = []

    def repl(m: re.Match[str]) -> str:
        nonlocal changed
        name = m.group("name")
        if is_immutable(name):
            changed += 1
            changed_names.append(name)
            return m.group(1) + "&self"
        return m.group(0)

    new_text = NEEDLE.sub(repl, text)
    INTERNAL_RS.write_text(new_text)
    print(f"flipped {changed} `&mut self` -> `&self`:")
    for n in changed_names:
        print(f"  - {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
