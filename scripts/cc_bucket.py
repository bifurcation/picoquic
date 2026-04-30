#!/usr/bin/env python3
"""Bucket entries in build/compile_commands.json by source directory.

Used during Phase 0 to decide which translation units are in-scope for
the inventory (the picoquic library itself) and which are external
(vendored picotls, fetched dependencies).
"""

from __future__ import annotations

import json
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CC = REPO_ROOT / "build" / "compile_commands.json"
BUILD_DIR = REPO_ROOT / "build"


def bucket_for(p: Path) -> str:
    """Bucket name for a translation-unit path."""
    try:
        rel = p.relative_to(REPO_ROOT)
    except ValueError:
        return f"(other) {p.parent}"
    parts = rel.parts
    if parts[0] != "build":
        return parts[0]
    # Under build/ — usually fetched deps under build/_deps/<name>-src/...
    if len(parts) >= 3 and parts[1] == "_deps":
        return f"(build/_deps) {parts[2]}"
    return f"(build) {parts[1] if len(parts) > 1 else ''}".rstrip()


def main() -> None:
    entries = json.loads(CC.read_text())
    buckets: Counter[str] = Counter()
    for e in entries:
        buckets[bucket_for(Path(e["file"]))] += 1

    print(f"{len(entries)} total compile_commands.json entries\n")
    for name, count in sorted(buckets.items(), key=lambda x: -x[1]):
        print(f"  {count:5d}  {name}")


if __name__ == "__main__":
    main()
