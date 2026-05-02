#!/usr/bin/env python3
"""phase1b.py — Phase 1B: cross-module consistency report.

Pure inspection.  Scans the Rust translation under `rs/fq/src/`
and writes a markdown report at `xlate/consistency_report.md`
cataloguing patterns that vary across modules — naming
conventions, lint allowances, type definitions, cross-module
imports.  No claude, no edits.

The human reviewer reads the report, decides on consistency
policies, then sprinkles `// REVIEW: <instruction>` markers in
the source for Phase 1C to address.

Usage:
  python3 scripts/phase1b.py             # write the report
  python3 scripts/phase1b.py --json      # also emit raw data
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
from collections import defaultdict
from pathlib import Path

REPO_ROOT  = Path(__file__).resolve().parent.parent
RS_SRC     = REPO_ROOT / "rs" / "fq" / "src"
OUT_MD     = REPO_ROOT / "xlate" / "consistency_report.md"
OUT_JSON   = REPO_ROOT / "xlate" / "consistency_report.json"

# Patterns.  Anchored at start-of-line (after stripping leading
# whitespace for some) so we don't false-positive on text inside
# doc comments or string literals.
PUB_TRAIT_RE  = re.compile(r"^\s*pub\s+trait\s+(\w+)")
PUB_STRUCT_RE = re.compile(r"^\s*pub\s+struct\s+(\w+)")
PUB_ENUM_RE   = re.compile(r"^\s*pub\s+enum\s+(\w+)")
PUB_TYPE_RE   = re.compile(r"^\s*pub\s+type\s+(\w+)")
PUB_FN_RE     = re.compile(r"^\s*pub\s+(?:async\s+)?fn\s+(\w+)")
INNER_ATTR_RE = re.compile(r"^\s*#!\[(?:allow|warn|deny|forbid)\(([^)]+)\)\]")
USE_CRATE_RE  = re.compile(r"^\s*use\s+crate::([\w:]+?)(?:\s*::\s*\{([^}]+)\})?\s*;")


# ---------------------------------------------------------------------------
# Case style

def case_style(name: str) -> str:
    """Bucket a Rust identifier into a case style."""
    if not name:
        return "other"
    if name.startswith("_"):
        # _private, _padding, etc. — strip the lead and re-classify.
        return case_style(name.lstrip("_"))
    has_lower = any(c.islower() for c in name)
    has_upper = any(c.isupper() for c in name)
    has_under = "_" in name
    if name.isupper() and has_under:
        return "SCREAMING_SNAKE"
    if has_lower and not has_upper:
        return "snake_case"
    if name[0].isupper() and not has_under and has_lower:
        return "PascalCase"
    if has_upper and has_lower and has_under:
        return "Mixed_Case"
    return "other"


# ---------------------------------------------------------------------------
# Per-file extraction

def rel(p: Path) -> str:
    return str(p.relative_to(REPO_ROOT))


def scan_file(path: Path) -> dict:
    """Extract declarations and attributes from a single .rs file."""
    text = path.read_text(errors="replace")
    decls = {
        "traits":  [],   # list of {name, line}
        "structs": [],
        "enums":   [],
        "types":   [],
        "fns":     [],
    }
    inner_attrs: list[dict] = []
    imports: list[dict] = []  # {item, source_path, line}

    # Strip block comments so we don't catch "pub fn" inside them.
    # Approximate: drop /* ... */ pairs (won't handle nested but Rust
    # block comments can nest — close enough for a heuristic report).
    cleaned = re.sub(r"/\*.*?\*/", "", text, flags=re.S)

    for i, line in enumerate(cleaned.splitlines(), 1):
        # Skip line comments (after stripping) so doc-comment text
        # doesn't trip the public-decl regex.
        line_no_comment = line.split("//", 1)[0]
        for kind, rx in (("traits",  PUB_TRAIT_RE),
                         ("structs", PUB_STRUCT_RE),
                         ("enums",   PUB_ENUM_RE),
                         ("types",   PUB_TYPE_RE),
                         ("fns",     PUB_FN_RE)):
            m = rx.match(line_no_comment)
            if m:
                decls[kind].append({"name": m.group(1), "line": i})
                break  # one decl per line
        m = INNER_ATTR_RE.match(line)
        if m:
            for lint in (s.strip() for s in m.group(1).split(",")):
                if lint:
                    inner_attrs.append({"lint": lint, "line": i})
        m = USE_CRATE_RE.match(line_no_comment)
        if m:
            mod_path = m.group(1)
            items = m.group(2)
            if items:
                for item in (s.strip() for s in items.split(",")):
                    if item:
                        imports.append({"from": mod_path, "item": item, "line": i})
            else:
                imports.append({"from": mod_path, "item": "*", "line": i})

    return {
        "path":   rel(path),
        "lines":  len(text.splitlines()),
        "decls":  decls,
        "inner_attrs": inner_attrs,
        "imports": imports,
    }


def scan_all() -> list[dict]:
    if not RS_SRC.is_dir():
        return []
    return [scan_file(p) for p in sorted(RS_SRC.rglob("*.rs"))]


# ---------------------------------------------------------------------------
# Aggregation

def aggregate(files: list[dict]) -> dict:
    """Build the cross-module views from per-file data."""
    # Trait names → list of (case_style, file, line)
    traits = []
    for f in files:
        for t in f["decls"]["traits"]:
            traits.append({
                "name": t["name"],
                "case": case_style(t["name"]),
                "file": f["path"],
                "line": t["line"],
            })

    # Inner attrs by lint name → list of files mentioning it.
    inner_by_lint: dict[str, list[dict]] = defaultdict(list)
    for f in files:
        for a in f["inner_attrs"]:
            inner_by_lint[a["lint"]].append({"file": f["path"], "line": a["line"]})

    # Type definitions (struct + enum + type alias) by name → list of
    # (kind, file, line).  Names with >1 entry are duplicates worth
    # flagging — same C type defined in multiple Rust modules.
    type_defs: dict[str, list[dict]] = defaultdict(list)
    for f in files:
        for kind in ("structs", "enums", "types"):
            for d in f["decls"][kind]:
                type_defs[d["name"]].append({
                    "kind": kind[:-1],  # struct/enum/type
                    "file": f["path"],
                    "line": d["line"],
                })

    # Cross-module imports: which modules use which items, grouped
    # by source.
    imports_by_source: dict[str, list[dict]] = defaultdict(list)
    for f in files:
        for imp in f["imports"]:
            imports_by_source[imp["from"]].append({
                "file": f["path"],
                "item": imp["item"],
                "line": imp["line"],
            })

    return {
        "traits": traits,
        "inner_by_lint": inner_by_lint,
        "type_defs": type_defs,
        "imports_by_source": imports_by_source,
    }


# ---------------------------------------------------------------------------
# Report rendering

def render_md(files: list[dict], agg: dict) -> str:
    out: list[str] = []
    out.append("# Cross-module consistency report")
    out.append("")
    out.append(f"Generated: {time.strftime('%Y-%m-%dT%H:%M:%S')}")
    out.append(f"Files scanned: {len(files)}")
    out.append(f"Total lines: {sum(f['lines'] for f in files)}")
    out.append("")
    out.append("**How to use this report.**  Read each section.  Where you")
    out.append("see inconsistency that should be reconciled, decide a")
    out.append("policy, then sprinkle `// REVIEW: <instruction>` markers in")
    out.append("the offending files.  Run `scripts/phase1c.py` to apply.")
    out.append("")

    # ------------- Trait names -------------
    out.append("## Trait names by case style")
    out.append("")
    by_case: dict[str, list[dict]] = defaultdict(list)
    for t in agg["traits"]:
        by_case[t["case"]].append(t)
    out.append(f"Total traits: {len(agg['traits'])}.  "
               + ", ".join(f"{c}: {len(v)}" for c, v in sorted(by_case.items())))
    out.append("")
    out.append("Rust convention says traits are `PascalCase`.  Anything")
    out.append("else is a refactor candidate.")
    out.append("")
    for case in ("snake_case", "PascalCase", "Mixed_Case",
                 "SCREAMING_SNAKE", "other"):
        items = by_case.get(case, [])
        if not items:
            continue
        out.append(f"### {case} ({len(items)})")
        out.append("")
        out.append("| Trait | File | Line |")
        out.append("|---|---|---|")
        for t in sorted(items, key=lambda x: (x["file"], x["line"])):
            out.append(f"| `{t['name']}` | `{t['file']}` | {t['line']} |")
        out.append("")

    # ------------- Lint allowances -------------
    out.append("## Module-level lint allowances")
    out.append("")
    if not agg["inner_by_lint"]:
        out.append("(none)")
        out.append("")
    else:
        out.append(f"Lints suppressed at module scope across {len(files)} files:")
        out.append("")
        out.append("| Lint | Modules using it | Modules NOT using it |")
        out.append("|---|---|---|")
        all_files = sorted(f["path"] for f in files)
        for lint, occs in sorted(agg["inner_by_lint"].items()):
            using = sorted({o["file"] for o in occs})
            not_using = [p for p in all_files if p not in set(using)]
            using_short = ", ".join(Path(u).name for u in using)
            not_using_short = (
                "(all)" if not not_using
                else ", ".join(Path(u).name for u in not_using[:8])
                + ("…" if len(not_using) > 8 else "")
            )
            out.append(f"| `{lint}` | {len(using)}: {using_short} | "
                       f"{len(not_using)}: {not_using_short} |")
        out.append("")
        out.append("Lints used in only some modules are the interesting ones.")
        out.append("Either the lint is appropriate for those modules and not")
        out.append("the others (fine — but worth a line of comment), or the")
        out.append("application is inconsistent.")
        out.append("")

    # ------------- Type definitions -------------
    out.append("## Type definitions across modules")
    out.append("")
    duplicates = {n: defs for n, defs in agg["type_defs"].items()
                  if len(defs) > 1}
    if not duplicates:
        out.append("No name is defined in more than one module.  Good.")
        out.append("")
    else:
        out.append(f"**{len(duplicates)} type name(s) defined in more than one module.**")
        out.append("Usually this means an opaque stub somewhere should be")
        out.append("replaced by a `use crate::other_module::Type;`")
        out.append("import — Rust resolves the reference fine, but the")
        out.append("duplicate `pub struct X { _private: () }` is dead weight.")
        out.append("")
        out.append("| Type | Definitions |")
        out.append("|---|---|")
        for name in sorted(duplicates):
            defs = duplicates[name]
            cell = "<br>".join(
                f"`{d['kind']}` in `{d['file']}:{d['line']}`"
                for d in sorted(defs, key=lambda x: x["file"])
            )
            out.append(f"| `{name}` | {cell} |")
        out.append("")

    # All type defs (collapsed)
    all_types = {n: defs for n, defs in agg["type_defs"].items() if defs}
    out.append(f"### All `pub struct` / `pub enum` / `pub type` declarations "
               f"({sum(len(v) for v in all_types.values())} total)")
    out.append("")
    out.append("Single definitions are shown collapsed by source file.")
    out.append("Use this to see at a glance which module owns each type.")
    out.append("")
    out.append("| Type | Kind | Source |")
    out.append("|---|---|---|")
    for name in sorted(all_types):
        defs = all_types[name]
        if len(defs) == 1:
            d = defs[0]
            out.append(f"| `{name}` | `{d['kind']}` | "
                       f"`{d['file']}:{d['line']}` |")
        else:
            d = defs[0]
            out.append(f"| `{name}` | `{d['kind']}` | "
                       f"**{len(defs)} definitions — see above** |")
    out.append("")

    # ------------- Imports -------------
    out.append("## Cross-module imports")
    out.append("")
    out.append("Items pulled in via `use crate::…`, grouped by")
    out.append("source module.  A type imported by many modules but defined")
    out.append("in one place is the healthy pattern; a type imported via")
    out.append("two different source paths is a smell.")
    out.append("")
    out.append("| Source module | Items imported | Importers |")
    out.append("|---|---|---|")
    for src in sorted(agg["imports_by_source"]):
        occs = agg["imports_by_source"][src]
        items = sorted({o["item"] for o in occs})
        importers = sorted({o["file"] for o in occs})
        items_str = ", ".join(f"`{i}`" for i in items[:6]) + (
            f", … ({len(items) - 6} more)" if len(items) > 6 else ""
        )
        importers_str = ", ".join(Path(p).name for p in importers[:5]) + (
            f", … ({len(importers) - 5} more)" if len(importers) > 5 else ""
        )
        out.append(f"| `{src}` | {items_str} | {len(importers)}: "
                   f"{importers_str} |")
    out.append("")

    # ------------- Per-file summary -------------
    out.append("## Per-file summary")
    out.append("")
    out.append("| File | LOC | Traits | Structs | Enums | Type aliases | Fns | Inner #![allow] |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|")
    for f in files:
        d = f["decls"]
        out.append(
            f"| `{f['path']}` | {f['lines']} | "
            f"{len(d['traits'])} | {len(d['structs'])} | "
            f"{len(d['enums'])} | {len(d['types'])} | "
            f"{len(d['fns'])} | {len(f['inner_attrs'])} |"
        )
    out.append("")

    return "\n".join(out) + "\n"


# ---------------------------------------------------------------------------
# Main

def main() -> int:
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--json", action="store_true",
                   help="Also write raw aggregated data to "
                        "xlate/consistency_report.json")
    args = p.parse_args()

    files = scan_all()
    if not files:
        print(f"no .rs files under {rel(RS_SRC)}", file=sys.stderr)
        return 1
    agg = aggregate(files)
    md = render_md(files, agg)
    OUT_MD.parent.mkdir(parents=True, exist_ok=True)
    OUT_MD.write_text(md)
    print(f"Wrote {rel(OUT_MD)} "
          f"({len(files)} files, "
          f"{sum(f['lines'] for f in files)} total lines, "
          f"{len(agg['traits'])} traits, "
          f"{len(agg['type_defs'])} unique type names)")

    if args.json:
        # Convert defaultdicts to plain dicts and Paths to strings.
        agg_plain = {
            "traits": agg["traits"],
            "inner_by_lint": dict(agg["inner_by_lint"]),
            "type_defs": dict(agg["type_defs"]),
            "imports_by_source": dict(agg["imports_by_source"]),
        }
        OUT_JSON.write_text(json.dumps(
            {"files": files, "aggregated": agg_plain},
            indent=2,
        ) + "\n")
        print(f"Wrote {rel(OUT_JSON)}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
