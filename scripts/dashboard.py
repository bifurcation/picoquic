#!/usr/bin/env python3
"""Render a single-page HTML progress dashboard.

Inputs (from Phase 0 scripts):
  build/inventory.json
  build/call_graph.json
  build/ifdef_manifest.json

Plus a scan of rs/fq/src/ for todo!() markers and translated function
counts. (For now the Rust crate is a stub — every C file's status is
'C-only'. The dashboard is structured to evolve.)

Output: build/dashboard.html

The dashboard is a static, hand-rendered HTML page (no JS, no
external assets) so it's easy to inspect, diff, and check in.
"""

from __future__ import annotations

import json
import re
import sys
from collections import Counter, defaultdict
from html import escape
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
INV_PATH = REPO_ROOT / "xlate" / "inventory.json"
CG_PATH = REPO_ROOT / "xlate" / "call_graph.json"
IFDEF_PATH = REPO_ROOT / "xlate" / "ifdef_manifest.json"
RUST_SRC = REPO_ROOT / "rs" / "fq" / "src"
OUT_PATH = REPO_ROOT / "xlate" / "dashboard.html"

TODO_RE = re.compile(r"\btodo!\s*\(")
RUST_FN_RE = re.compile(r"^\s*(?:pub(?:\s*\([^)]*\))?\s+)?fn\s+([A-Za-z_]\w*)",
                        re.MULTILINE)
RUST_TEST_RE = re.compile(r"#\[test\]\s*(?:\n[^\n]*)*?\bfn\s+([A-Za-z_]\w*)")


def scan_rust_crate() -> dict:
    """Return a snapshot of rs/fq/src state."""
    out = {
        "src_files": [],
        "todo_count": 0,
        "fn_count": 0,
        "test_count": 0,
    }
    if not RUST_SRC.is_dir():
        return out
    for f in sorted(RUST_SRC.rglob("*.rs")):
        text = f.read_text(errors="replace")
        todos = len(TODO_RE.findall(text))
        fns = len(RUST_FN_RE.findall(text))
        tests = len(RUST_TEST_RE.findall(text))
        out["src_files"].append({
            "path": str(f.relative_to(REPO_ROOT)),
            "lines": text.count("\n"),
            "todos": todos,
            "fns": fns,
            "tests": tests,
        })
        out["todo_count"] += todos
        out["fn_count"] += fns
        out["test_count"] += tests
    return out


def file_status(c_file: dict, rust_index: dict) -> str:
    """Classify a C file's translation status.

    For v1, since we don't yet have a name-mapping, every file is
    'C-only' until rs/fq/src/ has *anything* under a path that mirrors
    the C file.  Future versions will refine.
    """
    if not RUST_SRC.is_dir() or not rust_index["src_files"]:
        return "C-only"
    # Mirror convention: picoquic/quicctx.c -> rs/fq/src/picoquic/quicctx.rs
    rust_path = (
        RUST_SRC
        / Path(c_file["file"]).with_suffix(".rs")
    )
    if not rust_path.exists():
        return "C-only"
    text = rust_path.read_text(errors="replace")
    has_todo = bool(TODO_RE.search(text))
    has_test = "#[test]" in text or "mod test" in text
    if has_todo:
        return "stubbed"
    if has_test:
        return "translated-tested"
    return "translated"


CSS = """
body { font: 14px/1.4 -apple-system, BlinkMacSystemFont, sans-serif;
       max-width: 1100px; margin: 2em auto; padding: 0 1em; color: #222; }
h1, h2, h3 { font-weight: 600; }
h1 { border-bottom: 2px solid #444; padding-bottom: .25em; }
h2 { margin-top: 2em; border-bottom: 1px solid #ccc; padding-bottom: .15em; }
table { border-collapse: collapse; margin-top: .5em; }
th, td { border: 1px solid #ddd; padding: .25em .5em; text-align: left;
         vertical-align: top; }
th { background: #f4f4f4; }
.right { text-align: right; }
.mono { font-family: "Menlo", monospace; font-size: 0.92em; }
.badge { display: inline-block; padding: 1px 6px; border-radius: 3px;
         font-size: .82em; font-weight: 600; }
.b-c-only           { background: #f8d7da; color: #842029; }
.b-stubbed          { background: #fff3cd; color: #664d03; }
.b-translated       { background: #d1e7dd; color: #0f5132; }
.b-translated-tested{ background: #198754; color: white; }
.muted { color: #888; }
.small { font-size: .9em; }
.section-summary { background: #f9f9f9; padding: .5em 1em; border-radius: 4px;
                   margin: .5em 0; }
"""


def render(inv: dict, cg: dict, ifdef: dict, rust_index: dict) -> str:
    md = inv["metadata"]
    files = inv["files"]
    h: list[str] = []
    out = h.append

    out(f"<!doctype html><html><head><meta charset='utf-8'>")
    out(f"<title>picoquic → fq translation dashboard</title>")
    out(f"<style>{CSS}</style></head><body>")
    out(f"<h1>picoquic → fq translation dashboard</h1>")
    out(f"<p class='muted small'>Generated {escape(md['generated_at'])} "
        f"from inventory.json + call_graph.json + ifdef_manifest.json + "
        f"a scan of rs/fq/src.</p>")

    # ------ Overall progress ------
    total_fns = cg["metadata"]["function_count"]
    rust_fns = rust_index["fn_count"]
    rust_todos = rust_index["todo_count"]
    rust_tests = rust_index["test_count"]
    out("<h2>Overall progress</h2><div class='section-summary'>")
    out("<table>")
    out("<tr><th>C functions in scope</th><td class='right mono'>"
        f"{total_fns}</td></tr>")
    out("<tr><th>Rust fns under rs/fq/src/</th><td class='right mono'>"
        f"{rust_fns}</td></tr>")
    out("<tr><th>Remaining <code>todo!()</code> in Rust</th>"
        f"<td class='right mono'>{rust_todos}</td></tr>")
    out("<tr><th>Translated <code>#[test]</code></th>"
        f"<td class='right mono'>{rust_tests}</td></tr>")
    out("<tr><th>Files in scope (C)</th><td class='right mono'>"
        f"{md['file_count']}</td></tr>")
    out("<tr><th>Translation units parsed</th>"
        f"<td class='right mono'>{md['tu_count']}</td></tr>")
    out("</table></div>")

    # ------ Per-module rollup ------
    out("<h2>Per-module rollup</h2>")
    by_mod: dict[str, dict] = defaultdict(lambda: {
        "files": 0, "fns": 0, "decls": 0, "structs": 0,
        "translated": 0, "stubbed": 0, "tested": 0, "c_only": 0,
    })
    for f in files:
        m = f["module"]
        d = by_mod[m]
        d["files"] += 1
        for x in f["decls"]:
            d["decls"] += 1
            if x["kind"] == "function" and x.get("is_definition"):
                d["fns"] += 1
            elif x["kind"] == "struct":
                d["structs"] += 1
        st = file_status(f, rust_index)
        if st == "C-only":
            d["c_only"] += 1
        elif st == "stubbed":
            d["stubbed"] += 1
        elif st == "translated":
            d["translated"] += 1
        elif st == "translated-tested":
            d["tested"] += 1
    out("<table><tr><th>module</th><th>files</th><th>fn defs</th>"
        "<th>structs</th><th>C-only</th><th>stubbed</th>"
        "<th>translated</th><th>tested</th></tr>")
    for m in sorted(by_mod):
        d = by_mod[m]
        out(f"<tr><td class='mono'>{m}</td>"
            f"<td class='right'>{d['files']}</td>"
            f"<td class='right'>{d['fns']}</td>"
            f"<td class='right'>{d['structs']}</td>"
            f"<td class='right'>{d['c_only']}</td>"
            f"<td class='right'>{d['stubbed']}</td>"
            f"<td class='right'>{d['translated']}</td>"
            f"<td class='right'>{d['tested']}</td></tr>")
    out("</table>")

    # ------ File-level table ------
    out("<h2>Files (in scope)</h2>")
    out("<table><tr><th>file</th><th>module</th><th>kind</th>"
        "<th>fn defs</th><th>structs</th><th>typedefs</th>"
        "<th>status</th></tr>")
    for f in sorted(files, key=lambda x: x["file"]):
        ds = f["decls"]
        n_fns = sum(1 for d in ds if d["kind"] == "function" and d.get("is_definition"))
        n_structs = sum(1 for d in ds if d["kind"] == "struct")
        n_typedefs = sum(1 for d in ds if d["kind"] == "typedef")
        kind = "header" if f["file"].endswith(".h") else "source"
        st = file_status(f, rust_index)
        cls = "b-" + st.replace(" ", "-")
        out(f"<tr><td class='mono'>{escape(f['file'])}</td>"
            f"<td class='mono'>{f['module']}</td>"
            f"<td class='small'>{kind}</td>"
            f"<td class='right'>{n_fns}</td>"
            f"<td class='right'>{n_structs}</td>"
            f"<td class='right'>{n_typedefs}</td>"
            f"<td><span class='badge {cls}'>{st}</span></td></tr>")
    out("</table>")

    # ------ Mutually recursive groups ------
    groups = cg.get("mutually_recursive", [])
    out(f"<h2>Mutually recursive function groups ({len(groups)})</h2>")
    if not groups:
        out("<p class='muted'>None.</p>")
    else:
        out("<p class='muted small'>These groups must be translated as a "
            "unit (per TRANSLATE_PLAN.md, Phase 3).</p>")
        for g in groups:
            out(f"<ul class='mono small'>")
            for fn in sorted(g):
                out(f"<li>{escape(fn)}</li>")
            out("</ul>")

    # ------ Top callers / most-called ------
    out("<h2>Top 20 functions by direct callee count</h2>")
    out("<p class='muted small'>Useful for identifying orchestrator "
        "functions — high callee count = a good late-Phase-3 target.</p>")
    rev = cg["adjacency"]
    most_callees = sorted(
        ((k, len(v)) for k, v in rev.items()), key=lambda x: -x[1]
    )[:20]
    out("<table><tr><th>function</th><th>direct callees</th></tr>")
    for name, n in most_callees:
        out(f"<tr><td class='mono'>{escape(name)}</td>"
            f"<td class='right'>{n}</td></tr>")
    out("</table>")

    out("<h2>Top 20 functions by incoming-call count (most called)</h2>")
    out("<p class='muted small'>High in-degree = these are foundational "
        "leaves; if they're translated wrong, lots of downstream work "
        "breaks. Also good Phase 3 starting points.</p>")
    incoming: Counter = Counter()
    for caller, callees in cg["adjacency"].items():
        for c in callees:
            incoming[c] += 1
    out("<table><tr><th>function</th><th>incoming</th><th>height</th></tr>")
    for name, n in incoming.most_common(20):
        h_ = cg["height_of"].get(name, "—")
        out(f"<tr><td class='mono'>{escape(name)}</td>"
            f"<td class='right'>{n}</td>"
            f"<td class='right'>{h_}</td></tr>")
    out("</table>")

    # ------ External call targets ------
    out("<h2>External call targets (top 30)</h2>")
    out("<p class='muted small'>Functions called from in-scope code "
        "but defined elsewhere (libc, picotls, system). The translator "
        "needs Rust equivalents (stdlib, crates, or sys bindings) for "
        "each.</p>")
    ext = cg["external_call_target_counts"]
    items = sorted(ext.items(), key=lambda x: -x[1])[:30]
    out("<table><tr><th>name</th><th>callers</th></tr>")
    for name, n in items:
        out(f"<tr><td class='mono'>{escape(name)}</td>"
            f"<td class='right'>{n}</td></tr>")
    out("</table>")

    # ------ Ifdef review list ------
    depends = ifdef["classified"].get("depends", [])
    out(f"<h2>#ifdef symbols requiring review ({len(depends)})</h2>")
    out("<p class='muted small'>These appeared in <code>#if</code> / "
        "<code>#ifdef</code> directives but aren't covered by a "
        "<code>-D</code> flag, a known-off platform pattern, or an "
        "include guard. Review what they do and whether their "
        "branches should be translated.</p>")
    out("<table><tr><th>symbol</th><th>occurrences</th>"
        "<th>files (sample)</th></tr>")
    for s in sorted(depends, key=lambda x: -x["occurrences"])[:40]:
        files_short = ", ".join(s["files"][:3])
        if len(s["files"]) > 3:
            files_short += f", … (+{len(s['files']) - 3})"
        out(f"<tr><td class='mono'>{escape(s['symbol'])}</td>"
            f"<td class='right'>{s['occurrences']}</td>"
            f"<td class='small mono'>{escape(files_short)}</td></tr>")
    out("</table>")
    if len(depends) > 40:
        out(f"<p class='muted small'>... and {len(depends) - 40} more "
            f"in <code>build/ifdef_manifest.json</code>.</p>")

    out("</body></html>")
    return "\n".join(h)


def main() -> int:
    inv = json.loads(INV_PATH.read_text())
    cg = json.loads(CG_PATH.read_text())
    ifdef = json.loads(IFDEF_PATH.read_text())
    rust_index = scan_rust_crate()
    html = render(inv, cg, ifdef, rust_index)
    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(html)
    print(f"Wrote {OUT_PATH}")
    print(f"  C functions in scope:  {cg['metadata']['function_count']}")
    print(f"  Rust fns:              {rust_index['fn_count']}")
    print(f"  Remaining todo!():     {rust_index['todo_count']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
