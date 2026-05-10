#!/usr/bin/env python3
"""Record manually reviewed Phase 4D results from a JSON file.

The input JSON may be either:

  {"results": [{...}, ...]}

or a mapping of c_id to result objects.  Each result must include
`c_id`, `outcome`, and `analysis`.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import phase4d


def load_updates(path: Path) -> dict[str, dict]:
    raw = json.loads(path.read_text())
    items = raw.get("results", raw) if isinstance(raw, dict) else raw
    if isinstance(items, dict):
        iterable = items.values()
    elif isinstance(items, list):
        iterable = items
    else:
        raise SystemExit("input must contain a results list or mapping")

    updates: dict[str, dict] = {}
    for item in iterable:
        c_id = item.get("c_id")
        outcome = item.get("outcome")
        analysis = item.get("analysis")
        if not c_id or outcome not in phase4d.OUTCOMES or not analysis:
            raise SystemExit(f"invalid Phase 4D result: {item!r}")
        updates[c_id] = {
            "c_id": c_id,
            "phase4c_status": item.get("phase4c_status", ""),
            "outcome": outcome,
            "analysis": str(analysis)[:2000],
            "fix_summary": str(item.get("fix_summary", ""))[:2000],
            "files_changed": [str(v)[:300] for v in item.get("files_changed", [])[:20]],
            "verification": [str(v)[:300] for v in item.get("verification", [])[:20]],
        }
    return updates


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()

    updates = load_updates(args.input)
    phase4d.update_results(updates, force=args.force)
    print(f"recorded {len(updates)} Phase 4D result(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
