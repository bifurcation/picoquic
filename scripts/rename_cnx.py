#!/usr/bin/env python3
"""Rename `Cnx` → `Connection` (word-boundary, case-preserving uppercase only).

Leaves lowercase `cnx` identifiers (cnx_id, cnx_state, etc.) untouched —
only the type/trait identifier changes.  Run from repo root."""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "rs" / "fq" / "src"

# Match `Cnx` only as a whole word (not inside other PascalCase tokens).
PATTERN = re.compile(r"\bCnx\b")

changed = 0
for path in sorted(SRC.rglob("*.rs")):
    text = path.read_text()
    new = PATTERN.sub("Connection", text)
    if new != text:
        path.write_text(new)
        # Count replacements:
        n = len(PATTERN.findall(text))
        print(f"{path.relative_to(ROOT)}: {n} replacement(s)")
        changed += n

print(f"\nTotal: {changed} replacement(s) across the crate.")
