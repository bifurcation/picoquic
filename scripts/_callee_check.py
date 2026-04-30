#!/usr/bin/env python3
"""Quick check: count CALL_EXPR cursors in one TU vs. unique-per-caller.

Used once during Phase 0 to validate that the callee walk in
inventory.py is finding all calls.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import _clang_setup  # noqa: F401
import clang.cindex as cc

from inventory import args_for, parse_compile_commands

target = sys.argv[1]
entry = next(e for e in parse_compile_commands() if e["file"].endswith(target))
idx = cc.Index.create()
tu = idx.parse(entry["file"], args=args_for(entry))

call_count = 0
unresolved = 0
fdef_count = 0
total_callees_summed = 0
total_unique_callees = 0


def walk(c: cc.Cursor) -> None:
    global call_count, unresolved
    if c.kind == cc.CursorKind.CALL_EXPR:
        call_count += 1
        ref = c.referenced
        if ref is None or ref.kind != cc.CursorKind.FUNCTION_DECL:
            unresolved += 1
    for ch in c.get_children():
        walk(ch)


for c in tu.cursor.get_children():
    if (
        c.kind == cc.CursorKind.FUNCTION_DECL
        and c.is_definition()
        and c.location.file is not None
        and target.split("/")[-1] in c.location.file.name
    ):
        fdef_count += 1
        before_call = call_count
        before_unresolved = unresolved
        walk(c)
        n_calls = call_count - before_call
        total_callees_summed += n_calls

print("file:", target)
print("function definitions:", fdef_count)
print("total CALL_EXPR cursors:", call_count)
print("of which unresolved (indirect):", unresolved)
print("calls per fn (mean):", call_count / max(fdef_count, 1))
