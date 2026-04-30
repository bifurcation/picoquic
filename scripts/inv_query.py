#!/usr/bin/env python3
"""Ad-hoc queries against an inventory.json.

Usage:
  python3 scripts/inv_query.py <inv.json> top-callers [N]
  python3 scripts/inv_query.py <inv.json> callee-hist
  python3 scripts/inv_query.py <inv.json> summary
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path


def load(path: str) -> dict:
    return json.loads(Path(path).read_text())


def all_function_defs(inv: dict):
    for f in inv["files"]:
        for d in f["decls"]:
            if d["kind"] == "function" and d.get("is_definition"):
                yield f["file"], d


def cmd_top_callers(inv: dict, n: int = 5) -> None:
    funcs = list(all_function_defs(inv))
    funcs.sort(key=lambda x: len(x[1].get("callees") or []), reverse=True)
    print(f"top {n} functions by direct callee count:")
    for f, fn in funcs[:n]:
        callees = fn.get("callees") or []
        print(f"  [{len(callees):3d}] {f}: {fn['name']}")
        for c in callees[:8]:
            print(f"    -> {c}")
        if len(callees) > 8:
            print(f"    ... and {len(callees) - 8} more")
        print()


def cmd_callee_hist(inv: dict) -> None:
    hist: Counter[int] = Counter()
    for _, fn in all_function_defs(inv):
        hist[len(fn.get("callees") or [])] += 1
    total = sum(hist.values())
    print(f"callee-count histogram (over {total} function defs):")
    cum = 0
    for k in sorted(hist):
        cum += hist[k]
        bar = "#" * min(60, hist[k] // max(1, total // 60))
        print(f"  {k:3d}: {hist[k]:5d}  ({100*cum/total:5.1f}% cum)  {bar}")


def cmd_summary(inv: dict) -> None:
    md = inv["metadata"]
    print(f"generated:   {md['generated_at']}")
    print(f"in-scope:    {', '.join(md['in_scope_dirs'])}")
    print(f"TUs parsed:  {md['tu_count']}")
    print(f"files:       {md['file_count']}")
    by_module: Counter = Counter()
    files_by_module: Counter = Counter()
    func_defs = 0
    func_decls = 0
    structs = 0
    enums = 0
    unions = 0
    typedefs = 0
    for f in inv["files"]:
        files_by_module[f["module"]] += 1
        for d in f["decls"]:
            by_module[f["module"]] += 1
            if d["kind"] == "function":
                if d.get("is_definition"):
                    func_defs += 1
                else:
                    func_decls += 1
            elif d["kind"] == "struct":
                structs += 1
            elif d["kind"] == "enum":
                enums += 1
            elif d["kind"] == "union":
                unions += 1
            elif d["kind"] == "typedef":
                typedefs += 1
    fatal_tus = sum(
        1 for s in inv["tu_status"].values() if s.get("fatal_diagnostics")
    )
    error_tus = sum(1 for s in inv["tu_status"].values() if s.get("error"))
    skipped_tus = sum(1 for s in inv["tu_status"].values() if s.get("skipped"))
    print(f"files by module:  {dict(files_by_module)}")
    print(f"decls by module:  {dict(by_module)}")
    print(f"function defs:    {func_defs}")
    print(f"function decls:   {func_decls}")
    print(f"structs:          {structs}")
    print(f"enums:            {enums}")
    print(f"unions:           {unions}")
    print(f"typedefs:         {typedefs}")
    print(f"call edges:       {len(inv['call_edges'])} unique (caller, callee)")
    print(f"indirect edges:   {len(inv['indirect_call_edges'])} via fn-ptr typedef")
    print(f"TUs w/ fatal:     {fatal_tus}")
    print(f"TUs error:        {error_tus}")
    print(f"TUs skipped:      {skipped_tus}")


def main() -> int:
    if len(sys.argv) < 3:
        print(__doc__, file=sys.stderr)
        return 1
    inv = load(sys.argv[1])
    cmd = sys.argv[2]
    if cmd == "top-callers":
        n = int(sys.argv[3]) if len(sys.argv) > 3 else 5
        cmd_top_callers(inv, n)
    elif cmd == "callee-hist":
        cmd_callee_hist(inv)
    elif cmd == "summary":
        cmd_summary(inv)
    else:
        print(f"unknown subcommand: {cmd}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
