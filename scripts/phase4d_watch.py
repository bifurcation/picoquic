#!/usr/bin/env python3
"""Stream concise Phase 4D result updates.

Usage:
  python3 scripts/phase4d_watch.py
  python3 scripts/phase4d_watch.py --log xlate/phase4d_stream.log
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
RESULTS = REPO_ROOT / "xlate" / "phase4d_results.json"


def load_results() -> dict:
    if not RESULTS.is_file():
        return {}
    try:
        return json.loads(RESULTS.read_text()).get("results", {})
    except (OSError, json.JSONDecodeError):
        return {}


def short(text: str, limit: int) -> str:
    text = " ".join(str(text).split())
    if len(text) <= limit:
        return text
    return text[: limit - 1] + "..."


def format_line(c_id: str, result: dict, *, analysis_width: int) -> str:
    return (
        f"{time.strftime('%Y-%m-%dT%H:%M:%S')}\t"
        f"{result.get('outcome', 'unknown')}\t"
        f"{c_id}\t"
        f"{short(result.get('analysis', ''), analysis_width)}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--interval", type=float, default=5.0)
    parser.add_argument("--log", type=Path, help="append lines to this file instead of stdout")
    parser.add_argument("--existing", action="store_true", help="also print existing results at startup")
    parser.add_argument("--analysis-width", type=int, default=180)
    args = parser.parse_args()

    seen: set[str] = set()
    if not args.existing:
        seen.update(load_results().keys())

    log_f = None
    if args.log:
        path = args.log if args.log.is_absolute() else REPO_ROOT / args.log
        path.parent.mkdir(parents=True, exist_ok=True)
        log_f = path.open("a", buffering=1)

    try:
        while True:
            results = load_results()
            for c_id in sorted(set(results) - seen):
                line = format_line(c_id, results[c_id], analysis_width=args.analysis_width)
                if log_f:
                    log_f.write(line + "\n")
                else:
                    print(line, flush=True)
                seen.add(c_id)
            time.sleep(args.interval)
    finally:
        if log_f:
            log_f.close()


if __name__ == "__main__":
    raise SystemExit(main())
