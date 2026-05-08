#!/usr/bin/env python3
"""Stream concise Phase 4D result updates.

Usage:
  python3 scripts/phase4d_watch.py
  python3 scripts/phase4d_watch.py --existing
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
RESULTS = REPO_ROOT / "xlate" / "phase4d_results.json"
PHASE4C_REVIEWS = REPO_ROOT / "xlate" / "phase4c_reviews.json"
PHASE4C_WORK_STATUSES = {"suspect", "definitely_not_ok"}


def load_results() -> dict:
    if not RESULTS.is_file():
        return {}
    try:
        return json.loads(RESULTS.read_text()).get("results", {})
    except (OSError, json.JSONDecodeError):
        return {}


def load_phase4c_total() -> int:
    if not PHASE4C_REVIEWS.is_file():
        return 0
    try:
        reviews = json.loads(PHASE4C_REVIEWS.read_text()).get("reviews", {})
    except (OSError, json.JSONDecodeError):
        return 0
    return sum(
        1 for review in reviews.values()
        if review.get("status") in PHASE4C_WORK_STATUSES
    )


def outcome_counts(results: dict, total: int) -> str:
    ok = sum(1 for item in results.values() if item.get("outcome") == "ok")
    needs_fix = sum(1 for item in results.values() if item.get("outcome") == "needs_fix")
    fixed = sum(1 for item in results.values() if item.get("outcome") == "fixed")
    blocked = sum(1 for item in results.values() if item.get("outcome") == "blocked")
    remaining = max(total - len(results), 0)
    parts = [
        f"ok={ok}",
        f"needs_fix={needs_fix}",
        f"remaining={remaining}",
    ]
    if fixed:
        parts.append(f"fixed={fixed}")
    if blocked:
        parts.append(f"blocked={blocked}")
    return " ".join(parts)


def short(text: str, limit: int) -> str:
    text = " ".join(str(text).split())
    if len(text) <= limit:
        return text
    return text[: limit - 1] + "..."


def format_line(
    c_id: str,
    result: dict,
    *,
    counts: str,
    analysis_width: int,
) -> str:
    return (
        f"{time.strftime('%Y-%m-%dT%H:%M:%S')}\t"
        f"{counts}\t"
        f"{result.get('outcome', 'unknown')}\t"
        f"{c_id}\t"
        f"{short(result.get('analysis', ''), analysis_width)}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--interval", type=float, default=5.0)
    parser.add_argument("--log", type=Path, help="append lines to this file instead of stdout")
    parser.add_argument("--existing", action="store_true", help="also print existing results at startup")
    parser.add_argument("--once", action="store_true", help="print current new/existing results once and exit")
    parser.add_argument("--limit", type=int, help="maximum lines to print before exiting")
    parser.add_argument("--analysis-width", type=int, default=180)
    args = parser.parse_args()

    total = load_phase4c_total()
    seen: set[str] = set()
    if not args.existing:
        seen.update(load_results().keys())

    log_f = None
    if args.log:
        path = args.log if args.log.is_absolute() else REPO_ROOT / args.log
        path.parent.mkdir(parents=True, exist_ok=True)
        log_f = path.open("a", buffering=1)

    try:
        printed = 0
        while True:
            results = load_results()
            counts = outcome_counts(results, total)
            for c_id in sorted(set(results) - seen):
                line = format_line(
                    c_id,
                    results[c_id],
                    counts=counts,
                    analysis_width=args.analysis_width,
                )
                if log_f:
                    log_f.write(line + "\n")
                else:
                    print(line, flush=True)
                seen.add(c_id)
                printed += 1
                if args.limit is not None and printed >= args.limit:
                    return 0
            if args.once:
                break
            time.sleep(args.interval)
    finally:
        if log_f:
            log_f.close()


if __name__ == "__main__":
    raise SystemExit(main())
