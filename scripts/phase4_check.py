#!/usr/bin/env python3
"""phase4_check.py — verify that Rust source files have no
incomplete bodies left.

Detects three incompleteness patterns (all are functionally
equivalent — runtime panic or silent stub):
  * `todo!()`
  * `unimplemented!()`
  * `// SKIP:` markers (used by Phase 4 agents to stub a body with
    `None` / `Err(Generic)` / `Ok(())` while the *real* C body
    isn't translated; pervasive in internal.rs)

Usage:
  python3 scripts/phase4_check.py                          # all
  python3 scripts/phase4_check.py rs/fq/src/siphash.rs ... # named files
  python3 scripts/phase4_check.py --quiet                  # only failures
  python3 scripts/phase4_check.py --detail                 # name unfinished fns

Exit code 0 if every named file has zero incomplete bodies; non-zero
otherwise.

The script reports per-file:
  - Number of incompleteness markers found, broken down by kind.
  - Function names that contain at least one (best-effort regex
    walk back from each match to the enclosing `fn`).
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
RS_SRC = REPO_ROOT / "rs" / "fq" / "src"

PATTERNS = [
    ("todo!()", re.compile(r"\btodo!\s*\(", re.MULTILINE)),
    ("unimplemented!()", re.compile(r"\bunimplemented!\s*\(", re.MULTILINE)),
    ("// SKIP:", re.compile(r"//\s*SKIP\s*:", re.MULTILINE)),
]
FN_RE = re.compile(
    r"\bfn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*[<(]",
)


def fn_at(text: str, offset: int) -> str | None:
    """Find the most recent `fn name` before `offset`."""
    last = None
    for m in FN_RE.finditer(text, 0, offset):
        last = m
    return last.group("name") if last else None


def check(path: Path, quiet: bool = False) -> tuple[int, list[str]]:
    text = path.read_text()
    by_kind: dict[str, list[tuple[int, str | None]]] = {}
    for kind, pat in PATTERNS:
        hits: list[tuple[int, str | None]] = []
        for m in pat.finditer(text):
            line = text.count("\n", 0, m.start()) + 1
            hits.append((line, fn_at(text, m.start())))
        if hits:
            by_kind[kind] = hits
    total = sum(len(v) for v in by_kind.values())
    if total == 0:
        if not quiet:
            print(f"OK    {path.relative_to(REPO_ROOT)}: "
                  f"0 incomplete bodies")
        return 0, []
    parts = ", ".join(
        f"{len(v)} {k}" for k, v in by_kind.items()
    )
    fns = sorted({
        fn for hits in by_kind.values() for _, fn in hits if fn
    })
    print(f"FAIL  {path.relative_to(REPO_ROOT)}: "
          f"{total} incomplete ({parts}) in {len(fns)} fn(s)")
    return total, fns


def collect_targets(args: list[str]) -> list[Path]:
    if args:
        return [Path(a) if Path(a).is_absolute() else REPO_ROOT / a
                for a in args]
    return sorted(RS_SRC.rglob("*.rs"))


def main() -> int:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument("files", nargs="*",
                   help="Rust source files (relative to repo root) to "
                        "check.  Default: every .rs under rs/fq/src/.")
    p.add_argument("--quiet", action="store_true",
                   help="Print only files with remaining todo!()s.")
    p.add_argument("--detail", action="store_true",
                   help="List the unfinished function names per file.")
    a = p.parse_args()

    targets = collect_targets(a.files)
    total = 0
    files_with_todos = 0
    for path in targets:
        if not path.exists():
            print(f"WARNING: missing {path}", file=sys.stderr)
            continue
        n, fns = check(path, quiet=a.quiet)
        total += n
        if n:
            files_with_todos += 1
            if a.detail:
                for fn in fns:
                    print(f"        - {fn}")

    print(f"\nSummary: {len(targets) - files_with_todos}/{len(targets)} "
          f"files complete, {total} incomplete bodies remain "
          f"(todo!() + unimplemented!() + // SKIP:).")
    return 0 if total == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
