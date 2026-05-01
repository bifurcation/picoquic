#!/usr/bin/env python3
"""Phase 0 inventory: parse every in-scope C translation unit and emit
inventory.json.

For each TU we record:
  * file path (relative to repo root) and module bucket
  * #include directives (resolved against the include search path)
  * top-level declarations: functions, structs, enums, unions, typedefs
  * for each function definition, the list of called functions (callees)
    and called function-pointer typedefs (indirect_callees)

The result is one big JSON document consumed by:
  * scripts/call_graph.py — derives heights, SCCs, mutually recursive pairs
  * scripts/dashboard.py — renders the progress dashboard

Usage:
  python3 scripts/inventory.py            # writes build/inventory.json
  python3 scripts/inventory.py --limit 5  # parse only first 5 TUs (smoke)
  python3 scripts/inventory.py --file picoquic/quicctx.c   # one TU
"""

from __future__ import annotations

import _clang_setup  # noqa: F401 — must import before clang.cindex use
import argparse
import json
import shlex
import sys
import time
from collections import defaultdict
from pathlib import Path
from typing import Any

import clang.cindex as cc

REPO_ROOT = Path(__file__).resolve().parent.parent
CC_PATH = REPO_ROOT / "build" / "compile_commands.json"
OUT_PATH = REPO_ROOT / "xlate" / "inventory.json"

# Directories whose translation units we treat as in-scope for the port.
# v1 = picoquic-core only.  picoquictest/ comes back in Phase 2 (tests);
# loglib/ and picohttp/ are out of scope for the v1 hard fork.
IN_SCOPE = {"picoquic"}


def relpath(p: Path) -> str:
    """Return p relative to REPO_ROOT, or absolute if outside."""
    try:
        return str(p.resolve().relative_to(REPO_ROOT))
    except ValueError:
        return str(p.resolve())


def parse_compile_commands() -> list[dict]:
    return json.loads(CC_PATH.read_text())


def in_scope(entry: dict) -> bool:
    rel = relpath(Path(entry["file"]))
    return rel.split("/", 1)[0] in IN_SCOPE


def _macos_sdk_path() -> str | None:
    import subprocess
    try:
        return subprocess.check_output(
            ["xcrun", "--show-sdk-path"], text=True
        ).strip()
    except (FileNotFoundError, subprocess.CalledProcessError):
        return None


_SDK = _macos_sdk_path()


def _clang_builtin_include() -> str | None:
    """Homebrew libclang ships its own builtin headers (stdarg.h, etc.).
    Find them so libclang can resolve __builtin_va_list and friends."""
    for d in sorted(Path("/opt/homebrew/opt/llvm/lib/clang").glob("*/include"),
                    reverse=True):
        if (d / "stdarg.h").is_file():
            return str(d)
    return None


_CLANG_BUILTIN = _clang_builtin_include()


def args_for(entry: dict) -> list[str]:
    """Compile flags for libclang, derived from compile_commands.json."""
    cmd = shlex.split(entry["command"])
    # Drop compiler binary, drop the source file, drop -o <out>, drop -c.
    out: list[str] = []
    skip_next = False
    for i, tok in enumerate(cmd[1:], 1):
        if skip_next:
            skip_next = False
            continue
        if tok == "-o":
            skip_next = True
            continue
        if tok == "-c":
            continue
        if tok == entry["file"]:
            continue
        out.append(tok)
    # Force C language so libclang treats .c files consistently.
    if "-x" not in out:
        out.extend(["-x", "c"])
    # Homebrew libclang doesn't auto-discover the macOS SDK; without
    # -isysroot it can't find <stdint.h> etc.
    if _SDK and "-isysroot" not in out:
        out.extend(["-isysroot", _SDK])
    # And its compiler-builtin headers (stdarg.h, etc.) live alongside
    # the dylib, not in the SDK.
    if _CLANG_BUILTIN:
        out.extend(["-isystem", _CLANG_BUILTIN])
    return out


def location_str(loc: cc.SourceLocation) -> str | None:
    if loc.file is None:
        return None
    return f"{relpath(Path(loc.file.name))}:{loc.line}"


def is_in_scope_file(loc: cc.SourceLocation) -> bool:
    if loc.file is None:
        return False
    rel = relpath(Path(loc.file.name))
    return rel.split("/", 1)[0] in IN_SCOPE


def collect_callees(cursor: cc.Cursor) -> tuple[list[str], list[str]]:
    """Walk a function-definition body, return (direct_callees,
    indirect_callees).

    Direct: ordinary function calls whose target is a FUNCTION_DECL.
    Indirect: calls through a value of function-pointer type. Recorded
    as the typedef name when we can resolve it; otherwise '<unknown>'.
    """
    direct: set[str] = set()
    indirect: set[str] = set()

    def walk(c: cc.Cursor) -> None:
        if c.kind == cc.CursorKind.CALL_EXPR:
            ref = c.referenced
            if ref is not None and ref.kind == cc.CursorKind.FUNCTION_DECL:
                direct.add(ref.spelling)
            else:
                # Indirect call. Try to recover a typedef name from the
                # callee expression's type.
                t = c.type
                td = "<unknown>"
                # The first child of CALL_EXPR is the callee expression.
                children = list(c.get_children())
                if children:
                    callee = children[0]
                    ctype = callee.type
                    # Drill through pointer-to-function and typedef layers.
                    name = None
                    cur_type: cc.Type | None = ctype
                    while cur_type is not None:
                        if cur_type.kind == cc.TypeKind.TYPEDEF:
                            name = cur_type.spelling
                            break
                        if cur_type.kind == cc.TypeKind.POINTER:
                            cur_type = cur_type.get_pointee()
                            continue
                        break
                    if name:
                        td = name
                indirect.add(td)
        for ch in c.get_children():
            walk(ch)

    walk(cursor)
    return sorted(direct), sorted(indirect)


def function_signature(c: cc.Cursor) -> str:
    """A best-effort textual signature: 'ret_type name(arg_type, …)'."""
    ret = c.result_type.spelling
    args = ", ".join(a.type.spelling for a in c.get_arguments()) or "void"
    return f"{ret} {c.spelling}({args})"


def record_for_struct(c: cc.Cursor) -> dict:
    fields = []
    for ch in c.get_children():
        if ch.kind == cc.CursorKind.FIELD_DECL:
            fields.append({"name": ch.spelling, "type": ch.type.spelling})
    return {
        "kind": "struct",
        "name": c.spelling or "<anonymous>",
        "fields": fields,
        "location": location_str(c.location),
    }


def record_for_enum(c: cc.Cursor) -> dict:
    values = []
    for ch in c.get_children():
        if ch.kind == cc.CursorKind.ENUM_CONSTANT_DECL:
            values.append({"name": ch.spelling, "value": ch.enum_value})
    return {
        "kind": "enum",
        "name": c.spelling or "<anonymous>",
        "values": values,
        "location": location_str(c.location),
    }


def record_for_union(c: cc.Cursor) -> dict:
    fields = []
    for ch in c.get_children():
        if ch.kind == cc.CursorKind.FIELD_DECL:
            fields.append({"name": ch.spelling, "type": ch.type.spelling})
    return {
        "kind": "union",
        "name": c.spelling or "<anonymous>",
        "fields": fields,
        "location": location_str(c.location),
    }


def record_for_typedef(c: cc.Cursor) -> dict:
    return {
        "kind": "typedef",
        "name": c.spelling,
        "underlying": c.underlying_typedef_type.spelling,
        "location": location_str(c.location),
    }


def record_for_function(c: cc.Cursor) -> dict:
    is_def = c.is_definition()
    direct: list[str] = []
    indirect: list[str] = []
    if is_def:
        direct, indirect = collect_callees(c)
    storage = c.storage_class
    return {
        "kind": "function",
        "name": c.spelling,
        "signature": function_signature(c),
        "is_definition": is_def,
        "is_static": storage == cc.StorageClass.STATIC,
        "is_inline": c.is_definition() and any(
            t.spelling in ("inline", "__inline", "__inline__")
            for t in c.get_tokens()
        ) if False else False,  # we don't bother detecting inline here
        "location": location_str(c.location),
        "callees": direct if is_def else None,
        "indirect_callees": indirect if is_def else None,
    }


def parse_tu(entry: dict, decls_by_file: dict[str, list[dict]],
             seen_keys: set, includes_by_tu: dict[str, list[str]],
             call_edges: set[tuple[str, str]],
             indirect_edges: set[tuple[str, str]],
             tu_status: dict[str, dict]) -> None:
    """Parse one TU and merge its findings into the global accumulators.

    decls_by_file: maps source-file path -> list of declaration records.
                   Decls are deduped by (file, line, kind, name) across TUs.
    includes_by_tu: maps TU path -> list of direct includes.
    call_edges: set of (caller_name, callee_name) pairs.
    indirect_edges: set of (caller_name, typedef_name_or_unknown) pairs.
    tu_status: per-TU metadata (errors, skips, fatal-diag count, parse time).
    """
    src = Path(entry["file"]).resolve()
    rel = relpath(src)
    module = rel.split("/", 1)[0]
    if not src.exists():
        tu_status[rel] = {"module": module, "error": "source not found"}
        return
    if src.suffix not in (".c", ".h"):
        tu_status[rel] = {"module": module, "skipped": "non-C source"}
        return
    args = args_for(entry)
    index = cc.Index.create()
    try:
        tu = index.parse(
            str(src),
            args=args,
            options=cc.TranslationUnit.PARSE_DETAILED_PROCESSING_RECORD,
        )
    except cc.TranslationUnitLoadError as e:
        tu_status[rel] = {"module": module, "error": f"parse failed: {e}"}
        return

    fatal_diags = [
        f"{d.location}: {d.spelling}"
        for d in tu.diagnostics
        if d.severity >= cc.Diagnostic.Error
    ]
    tu_status[rel] = {
        "module": module,
        "fatal_diagnostics": fatal_diags,
    }

    includes: list[str] = []
    for inc in tu.get_includes():
        if inc.depth == 1:
            includes.append(relpath(Path(inc.include.name)))
    includes_by_tu[rel] = sorted(set(includes))

    for c in tu.cursor.get_children():
        if not is_in_scope_file(c.location):
            continue
        kind = c.kind
        rec: dict | None = None
        if kind == cc.CursorKind.FUNCTION_DECL:
            rec = record_for_function(c)
            # Even non-definition decls go in (declarations matter for the
            # API surface), but we only collect call edges from definitions.
            if c.is_definition():
                callees = rec.get("callees") or []
                indirect = rec.get("indirect_callees") or []
                for callee in callees:
                    call_edges.add((c.spelling, callee))
                for ind in indirect:
                    indirect_edges.add((c.spelling, ind))
        elif kind == cc.CursorKind.STRUCT_DECL and c.is_definition():
            rec = record_for_struct(c)
        elif kind == cc.CursorKind.ENUM_DECL and c.is_definition():
            rec = record_for_enum(c)
        elif kind == cc.CursorKind.UNION_DECL and c.is_definition():
            rec = record_for_union(c)
        elif kind == cc.CursorKind.TYPEDEF_DECL:
            rec = record_for_typedef(c)
        if rec is None:
            continue
        # Dedupe by (file, line, kind, name) — for function decls we treat
        # the decl-with-definition as winning if we've already seen a decl
        # at the same location.
        loc = rec.get("location")
        if loc is None:
            continue
        key = (loc, rec["kind"], rec.get("name"))
        # Prefer function definitions over forward decls (replace in place
        # if we now have the definition for a previously-seen forward).
        if rec["kind"] == "function" and rec.get("is_definition"):
            # Remove any existing forward-decl for the same key and re-add.
            existing = next(
                (
                    (i, ex)
                    for i, ex in enumerate(decls_by_file.get(loc.split(":", 1)[0], []))
                    if (ex.get("location"), ex["kind"], ex.get("name")) == key
                ),
                None,
            )
            if existing is not None:
                i, ex = existing
                if not ex.get("is_definition"):
                    decls_by_file[loc.split(":", 1)[0]][i] = rec
                # If existing already is a definition, skip duplicate.
                seen_keys.add(key)
                continue
        if key in seen_keys:
            continue
        seen_keys.add(key)
        file_part = loc.split(":", 1)[0]
        decls_by_file.setdefault(file_part, []).append(rec)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=None,
                    help="Parse only the first N in-scope TUs (smoke test).")
    ap.add_argument("--file", default=None,
                    help="Parse only this one source file (relative path).")
    ap.add_argument("--out", default=str(OUT_PATH),
                    help="Output JSON path (default: xlate/inventory.json).")
    args = ap.parse_args()

    entries = parse_compile_commands()
    in_scope_entries = [e for e in entries if in_scope(e)]
    if args.file:
        target = (REPO_ROOT / args.file).resolve()
        in_scope_entries = [
            e for e in in_scope_entries if Path(e["file"]).resolve() == target
        ]
        if not in_scope_entries:
            print(f"No compile_commands entry for {args.file}", file=sys.stderr)
            return 1
    if args.limit is not None:
        in_scope_entries = in_scope_entries[: args.limit]

    print(f"Parsing {len(in_scope_entries)} TU(s)…", file=sys.stderr)
    decls_by_file: dict[str, list[dict]] = {}
    seen_keys: set = set()
    includes_by_tu: dict[str, list[str]] = {}
    call_edges: set[tuple[str, str]] = set()
    indirect_edges: set[tuple[str, str]] = set()
    tu_status: dict[str, dict] = {}

    t0 = time.monotonic()
    for i, e in enumerate(in_scope_entries, 1):
        parse_tu(
            e, decls_by_file, seen_keys, includes_by_tu,
            call_edges, indirect_edges, tu_status,
        )
        if i % 10 == 0 or i == len(in_scope_entries):
            print(
                f"  [{i}/{len(in_scope_entries)}] "
                f"{time.monotonic() - t0:.1f}s — {e['file']}",
                file=sys.stderr,
            )

    files_out = []
    for f in sorted(decls_by_file):
        files_out.append({
            "file": f,
            "module": f.split("/", 1)[0],
            "decls": decls_by_file[f],
        })

    inventory: dict[str, Any] = {
        "metadata": {
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "repo_root": str(REPO_ROOT),
            "in_scope_dirs": sorted(IN_SCOPE),
            "tu_count": len(in_scope_entries),
            "file_count": len(files_out),
        },
        "files": files_out,
        "tu_status": tu_status,
        "includes_by_tu": includes_by_tu,
        "call_edges": sorted(call_edges),
        "indirect_call_edges": sorted(indirect_edges),
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(inventory, indent=2) + "\n")
    print(f"Wrote {out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
