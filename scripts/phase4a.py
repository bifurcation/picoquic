#!/usr/bin/env python3
"""phase4a.py — build the C/Rust function map and approval plan.

Phase 4A does not edit Rust implementation files.  It creates the first
complete map from in-scope C functions to translated Rust counterparts,
classifies missing items, and writes a plan for human approval before
Phase 4B implements anything.

Outputs:
  - xlate/function_translation_map.json
  - xlate/phase4a_plan.md
  - xlate/phase4a_plan.html
"""

from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path

from phase4_common import (
    FUNCTION_MAP,
    REPO_ROOT,
    RS_SRC,
    XLATE,
    _prefer_rust_match,
    build_function_map,
    c_functions_from_inventory,
    c_name_candidate_list,
    collect_rust_functions,
    esc,
    html_page,
    load_function_map,
    preferred_impl_for_c,
    rust_impl_tokens,
    save_json,
    semantic_candidate_pool,
    semantic_score,
)

PLAN_MD = XLATE / "phase4a_plan.md"
PLAN_HTML = XLATE / "phase4a_plan.html"


def summarize(mapping: dict) -> Counter:
    return Counter(e.get("required_action", "unknown") for e in mapping.get("entries", []))


def entry_line(e: dict) -> str:
    c = e["c"]
    rust = e.get("rust")
    rust_text = "-"
    if rust:
        rust_text = f"{rust['file']}:{rust['start_line']}-{rust['end_line']} `{rust['name']}`"
    plan = e.get("phase4a_plan") or {}
    dest = plan.get("rust_destination", "-")
    return (
        f"| `{c['name']}` | `{c['file']}:{c['start_line']}-{c['end_line']}` | "
        f"{rust_text} | `{e.get('required_action')}` | `{dest}` | "
        f"{e.get('reason', '')} |\n"
    )


def write_plan_md(mapping: dict) -> None:
    counts = summarize(mapping)
    lines: list[str] = [
        "# Phase 4A Function Map and Implementation Plan",
        "",
        "Status: draft",
        "",
        "This plan is generated from `xlate/function_translation_map.json`.",
        "Review the `required_missing`, `expected_omission`, and `blocked`",
        "entries before approving Phase 4B.",
        "",
        "## Summary",
        "",
    ]
    total = len(mapping.get("entries", []))
    for key in ("implemented", "required_missing", "expected_omission", "blocked"):
        lines.append(f"* `{key}`: {counts.get(key, 0)}")
    lines.extend(["", f"Total in-scope C functions: {total}", ""])

    for section, title in (
        ("required_missing", "Required Missing Implementations"),
        ("blocked", "Blocked Classifications"),
        ("expected_omission", "Expected Omissions"),
    ):
        rows = [e for e in mapping["entries"] if e.get("required_action") == section]
        lines.extend([
            f"## {title}",
            "",
            "| C function | C span | Rust span | Action | Proposed destination | Reason |",
            "| --- | --- | --- | --- | --- | --- |",
        ])
        if rows:
            lines.extend(entry_line(e) for e in rows)
        else:
            lines.append("| _none_ | | | | | |\n")
        lines.append("")
    PLAN_MD.write_text("\n".join(lines))


def write_plan_html(mapping: dict) -> None:
    counts = summarize(mapping)
    total = len(mapping.get("entries", []))
    body = [
        "<h1>Phase 4A Function Map and Implementation Plan</h1>",
        "<p><strong>Status:</strong> draft</p>",
        "<h2>Summary</h2>",
        "<table><tr><th>Classification</th><th>Count</th><th>Percent</th></tr>",
    ]
    for key in ("implemented", "required_missing", "expected_omission", "blocked"):
        n = counts.get(key, 0)
        pct = 0.0 if total == 0 else 100.0 * n / total
        body.append(f"<tr><td>{esc(key)}</td><td>{n}</td><td>{pct:.1f}%</td></tr>")
    body.append("</table>")
    for section, title in (
        ("required_missing", "Required Missing Implementations"),
        ("blocked", "Blocked Classifications"),
        ("expected_omission", "Expected Omissions"),
    ):
        body.append(f"<h2>{esc(title)}</h2>")
        body.append("<table><tr><th>C function</th><th>C span</th><th>Rust span</th><th>Destination</th><th>Reason</th></tr>")
        rows = [e for e in mapping["entries"] if e.get("required_action") == section]
        if not rows:
            body.append("<tr><td colspan=\"5\"><em>none</em></td></tr>")
        for e in rows:
            c = e["c"]
            rust = e.get("rust")
            rust_text = "-"
            if rust:
                rust_text = f"{rust['file']}:{rust['start_line']}-{rust['end_line']} {rust['name']}"
            plan = e.get("phase4a_plan") or {}
            body.append(
                "<tr>"
                f"<td><code>{esc(c['name'])}</code></td>"
                f"<td>{esc(c['file'])}:{esc(c['start_line'])}-{esc(c['end_line'])}</td>"
                f"<td>{esc(rust_text)}</td>"
                f"<td>{esc(plan.get('rust_destination', '-'))}</td>"
                f"<td>{esc(e.get('reason', ''))}</td>"
                "</tr>"
            )
        body.append("</table>")
    PLAN_HTML.write_text(html_page("Phase 4A Plan", "\n".join(body)))


def print_status(mapping: dict) -> None:
    counts = summarize(mapping)
    total = len(mapping.get("entries", []))
    print(f"map: {FUNCTION_MAP.relative_to(REPO_ROOT)}")
    print(f"functions: {total}")
    for key in ("implemented", "required_missing", "expected_omission", "blocked"):
        print(f"  {key:18s} {counts.get(key, 0)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--status", action="store_true", help="show current map summary without rebuilding")
    parser.add_argument("--dry-run", action="store_true", help="build and summarize without writing files")
    parser.add_argument("--debug-name", help="debug Rust candidate matching for one C function name")
    args = parser.parse_args()

    if args.debug_name:
        c_match = next((fn for fn in c_functions_from_inventory() if fn.name == args.debug_name), None)
        candidates = c_name_candidate_list(args.debug_name)
        rust = collect_rust_functions()
        by_name = {}
        by_file = {}
        by_impl_token = {}
        for rf in rust:
            by_name.setdefault(rf.name, []).append(rf)
            by_file.setdefault(rf.file, []).append(rf)
            for token in rust_impl_tokens(rf):
                by_impl_token.setdefault(token, []).append(rf)
        print(f"C name: {args.debug_name}")
        print("candidates, in matching order:")
        for candidate in candidates:
            matches = by_name.get(candidate, [])
            preferred_impl = preferred_impl_for_c(c_match, candidate) if c_match else None
            selected = _prefer_rust_match(matches, preferred_impl)
            suffix = f", preferred impl {preferred_impl}" if preferred_impl else ""
            print(f"  {candidate}: {len(matches)}{suffix}")
            if selected:
                impl = f" impl {selected.impl_type}" if selected.impl_type else ""
                print(f"    selected {selected.file}:{selected.start_line}-{selected.end_line} {selected.name}{impl}")
            for m in matches[:5]:
                impl = f" impl {m.impl_type}" if m.impl_type else ""
                print(f"    {m.file}:{m.start_line}-{m.end_line} {m.name}{impl}")
            if not matches:
                raw_hits = []
                for path in sorted(RS_SRC.rglob("*.rs")):
                    text = path.read_text(errors="replace")
                    for i, line in enumerate(text.splitlines(), start=1):
                        if f"fn {candidate}" in line:
                            raw_hits.append((path.relative_to(REPO_ROOT).as_posix(), i, line.strip()))
                for file, line, text in raw_hits[:5]:
                    print(f"    raw {file}:{line}: {text}")
        if c_match:
            semantic = []
            for m in semantic_candidate_pool(c_match, by_file, by_name, by_impl_token):
                score, reason = semantic_score(c_match, m)
                if score:
                    semantic.append((score, reason, m))
            print("semantic candidates:")
            for score, reason, m in sorted(semantic, key=lambda x: x[0], reverse=True)[:10]:
                impl = f" impl {m.impl_type}" if m.impl_type else ""
                print(f"  {score:3d} {m.file}:{m.start_line}-{m.end_line} {m.name}{impl} -- {reason}")
        return 0

    if args.status:
        print_status(load_function_map())
        return 0

    mapping = build_function_map(load_function_map())
    if args.dry_run:
        print_status(mapping)
        return 0

    save_json(FUNCTION_MAP, mapping)
    write_plan_md(mapping)
    write_plan_html(mapping)
    print_status(mapping)
    print(f"wrote: {PLAN_MD.relative_to(REPO_ROOT)}")
    print(f"wrote: {PLAN_HTML.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
