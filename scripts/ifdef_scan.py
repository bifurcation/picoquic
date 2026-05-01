#!/usr/bin/env python3
"""Scan in-scope C/H files for #if/#ifdef/#ifndef symbols.

Per TRANSLATE_PLAN.md, the preprocess-then-parse pipeline (used
implicitly by libclang via compile_commands.json flags) drops every
non-target #ifdef branch. We need to know which symbols those branches
were guarded on so the translator knows what got silently excluded.

Output: build/ifdef_manifest.json with each symbol classified as:
  * always_on:  defined via -D in compile_commands.json — always taken.
  * known_off:  matches a known-other-platform pattern (e.g., _WIN32) on
                this build — always not taken.
  * depends:    everything else (could come from system headers,
                feature tests, etc.).

The translator should review the "depends" bucket before Phase 1 to
decide which branches matter.
"""

from __future__ import annotations

import json
import re
import shlex
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CC_PATH = REPO_ROOT / "build" / "compile_commands.json"
OUT_PATH = REPO_ROOT / "xlate" / "ifdef_manifest.json"

# v1 scope: picoquic-core only.  See scripts/inventory.py for context.
IN_SCOPE = {"picoquic"}

# Patterns that indicate a non-target platform on a macOS / linux build.
KNOWN_OFF_PATTERNS = [
    re.compile(r"^_WIN32$"),
    re.compile(r"^_WIN64$"),
    re.compile(r"^WIN32$"),
    re.compile(r"^_MSC_VER$"),
    re.compile(r"^_MSC_FULL_VER$"),
    re.compile(r"^__MINGW32__$"),
    re.compile(r"^__MINGW64__$"),
    re.compile(r"^__CYGWIN__$"),
    re.compile(r"^_USE_32BIT_TIME_T$"),
]

# Lines we want to scan. Comments stripped to a first approximation.
DIRECTIVE_RE = re.compile(
    r"^\s*#\s*(if|ifdef|ifndef|elif)\b(.*)$"
)
DEFINED_RE = re.compile(r"\bdefined\s*(?:\(\s*)?([A-Za-z_]\w*)\s*\)?")
IDENT_RE = re.compile(r"\b([A-Za-z_]\w*)\b")


def collect_defined_macros() -> set[str]:
    """Read all -D flags from compile_commands.json and union them.

    Each TU's flags may differ slightly; we conservatively treat any
    macro defined in any TU as 'always on' from the inventory's POV.
    """
    defined: set[str] = set()
    if not CC_PATH.exists():
        return defined
    for entry in json.loads(CC_PATH.read_text()):
        for tok in shlex.split(entry["command"]):
            if tok.startswith("-D"):
                name = tok[2:].split("=", 1)[0]
                if name:
                    defined.add(name)
    return defined


def in_scope_files() -> list[Path]:
    files: list[Path] = []
    for d in IN_SCOPE:
        root = REPO_ROOT / d
        if not root.is_dir():
            continue
        for ext in (".c", ".h"):
            files.extend(root.rglob(f"*{ext}"))
    return sorted(files)


def extract_symbols(line: str) -> list[str]:
    """Symbols referenced by a single preprocessor condition."""
    syms: list[str] = []
    syms.extend(DEFINED_RE.findall(line))
    # For #ifdef / #ifndef, the rest of the line is just the symbol.
    # For #if X, X may itself be a macro (we add bare identifiers, not
    # numeric literals or operator keywords).
    keywords = {"defined", "true", "false", "and", "or", "not"}
    for m in IDENT_RE.findall(line):
        if m in keywords:
            continue
        if m[0].isdigit():
            continue
        syms.append(m)
    return syms


DEFINE_RE = re.compile(r"^\s*#\s*define\s+([A-Za-z_]\w*)\b")


def find_include_guards(files: list[Path]) -> set[str]:
    """An include-guard symbol is one that appears in an #ifndef line
    followed (anywhere in the same file) by a #define of the same name.
    """
    guards: set[str] = set()
    for f in files:
        try:
            text = f.read_text(errors="replace")
        except OSError:
            continue
        ifndef_syms: set[str] = set()
        defined_syms: set[str] = set()
        for line in text.splitlines():
            m = DIRECTIVE_RE.match(line)
            if m and m.group(1) == "ifndef":
                rest = m.group(2).strip()
                idm = re.match(r"^([A-Za-z_]\w*)\b", rest)
                if idm:
                    ifndef_syms.add(idm.group(1))
            m2 = DEFINE_RE.match(line)
            if m2:
                defined_syms.add(m2.group(1))
        guards.update(ifndef_syms & defined_syms)
    return guards


def main() -> int:
    defined = collect_defined_macros()
    files = in_scope_files()
    include_guards = find_include_guards(files)
    occurrences: dict[str, list[dict]] = defaultdict(list)
    for f in files:
        try:
            text = f.read_text(errors="replace")
        except OSError:
            continue
        for lineno, line in enumerate(text.splitlines(), 1):
            m = DIRECTIVE_RE.match(line)
            if not m:
                continue
            directive = m.group(1)
            rest = m.group(2)
            for sym in extract_symbols(rest):
                occurrences[sym].append({
                    "file": str(f.relative_to(REPO_ROOT)),
                    "line": lineno,
                    "directive": directive,
                })

    classified = {
        "always_on": [],
        "known_off": [],
        "include_guard": [],
        "depends": [],
    }
    for sym, occs in sorted(occurrences.items()):
        # -D from compile_commands.json wins; an "if not yet defined"
        # idiom in a header may also match the include-guard pattern,
        # but the -D status is authoritative.
        if sym in defined:
            bucket = "always_on"
        elif sym in include_guards:
            bucket = "include_guard"
        elif any(pat.match(sym) for pat in KNOWN_OFF_PATTERNS):
            bucket = "known_off"
        else:
            bucket = "depends"
        classified[bucket].append({
            "symbol": sym,
            "occurrences": len(occs),
            "files": sorted({o["file"] for o in occs}),
            "sample": occs[:3],
        })

    out = {
        "metadata": {
            "in_scope_dirs": sorted(IN_SCOPE),
            "files_scanned": len(files),
            "symbols_seen": len(occurrences),
            "minus_d_macros_in_compile_commands": sorted(defined),
        },
        "classified": classified,
    }
    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(json.dumps(out, indent=2) + "\n")
    print(f"Wrote {OUT_PATH}")
    print(f"  files scanned:       {len(files)}")
    print(f"  symbols seen:        {len(occurrences)}")
    print(f"  always_on (-D):      {len(classified['always_on'])}")
    print(f"  known_off (Windows): {len(classified['known_off'])}")
    print(f"  include_guards:      {len(classified['include_guard'])}")
    print(f"  depends (review me): {len(classified['depends'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
