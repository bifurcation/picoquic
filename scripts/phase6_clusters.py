#!/usr/bin/env python3
"""phase6_clusters.py -- group Phase 6 failing tests by shared panic.

Reads `xlate/phase6_failures.json` plus the linked cargo-test log,
extracts each failure's (panic-location, panic-message) fingerprint,
and groups tests that share a fingerprint into clusters.  A cluster of
N tests sharing the same panic is almost always a single underlying
bug — fix that root cause once and the whole cluster turns green.

Outputs:
  - xlate/phase6_clusters.json
  - xlate/phase6_clusters.md (human-readable summary)

Typical use:
  python3 scripts/phase6_clusters.py            # rebuild + print top 10
  python3 scripts/phase6_clusters.py --top 20   # show 20 instead
  python3 scripts/phase6_clusters.py --json     # dump JSON to stdout
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import re
import sys

from phase4_common import REPO_ROOT
from phase5_common import XLATE

FAILURES = XLATE / "phase6_failures.json"
CLUSTERS_JSON = XLATE / "phase6_clusters.json"
CLUSTERS_MD = XLATE / "phase6_clusters.md"

# Capture only "pending" failures.  Tests already marked fixed/ok/blocked
# are excluded so cluster sizes reflect current work-remaining.
PENDING_STATUSES = {"pending"}

PANIC_INLINE_RE = re.compile(
    r"panicked at (\S+?):\s*\n\s*(.+)",
)


def fingerprint(name: str, excerpt: str, log_text: str) -> tuple[str, str] | None:
    """Return (location, message) for a failing test, or None if unknown.

    The excerpt usually contains the panicked-at line; if not, we fall back
    to scanning the full cargo-test log for the test's thread name.
    """
    match = PANIC_INLINE_RE.search(excerpt)
    if not match:
        pattern = (
            rf"thread '{re.escape(name)}'[^\n]*panicked at "
            rf"([^:\n]+:\d+:\d+):\s*\n\s*(.+)"
        )
        match = re.search(pattern, log_text)
    if not match:
        return None
    location = match.group(1).strip().rstrip(":")
    message = match.group(2).strip()
    return location, message[:200]


def build_clusters() -> list[dict]:
    if not FAILURES.is_file():
        sys.exit(
            "phase6_failures.json not found; run "
            "`python3 scripts/phase6.py --refresh-failures --status` first.",
        )
    data = json.loads(FAILURES.read_text())
    summary = data.get("summary", {})
    log_rel = summary.get("log")
    log_text = ""
    if log_rel:
        log_path = REPO_ROOT / log_rel
        if log_path.is_file():
            log_text = log_path.read_text()

    buckets: dict[tuple[str, str], list[str]] = collections.defaultdict(list)
    unclassified: list[str] = []
    failures = data.get("failures", {})
    for name, failure in failures.items():
        if failure.get("status") not in PENDING_STATUSES:
            continue
        excerpt = failure.get("excerpt", "")
        fp = fingerprint(name, excerpt, log_text)
        if fp is None:
            unclassified.append(name)
            continue
        buckets[fp].append(name)

    clusters = [
        {
            "size": len(names),
            "location": loc,
            "message": msg,
            "tests": sorted(names),
            "representative": sorted(names)[0],
        }
        for (loc, msg), names in buckets.items()
    ]
    clusters.sort(key=lambda c: (-c["size"], c["location"], c["message"]))
    return clusters, unclassified, summary


def write_outputs(clusters: list[dict], unclassified: list[str], summary: dict) -> None:
    payload = {
        "schema_version": 1,
        "generated_from": summary.get("log", ""),
        "summary": summary,
        "clusters": clusters,
        "unclassified": sorted(unclassified),
    }
    CLUSTERS_JSON.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")

    lines = [
        "# Phase 6 failure clusters",
        "",
        f"Source: `{summary.get('log', '')}`",
        f"Pending failures: {sum(c['size'] for c in clusters) + len(unclassified)}"
        f"  (classified into {len(clusters)} clusters,"
        f" {len(unclassified)} unclassified)",
        "",
        "| Size | Location | Message | Representative |",
        "| ---- | -------- | ------- | -------------- |",
    ]
    for cluster in clusters:
        msg = cluster["message"].replace("|", "\\|").replace("\n", " ")
        if len(msg) > 80:
            msg = msg[:77] + "..."
        lines.append(
            f"| {cluster['size']} | `{cluster['location']}` | {msg} |"
            f" `{cluster['representative']}` |",
        )
    if unclassified:
        lines.append("")
        lines.append("## Unclassified (no panic line matched)")
        for name in sorted(unclassified)[:50]:
            lines.append(f"* `{name}`")
        if len(unclassified) > 50:
            lines.append(f"* ... and {len(unclassified) - 50} more")
    CLUSTERS_MD.write_text("\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--top", type=int, default=10, help="how many clusters to print")
    parser.add_argument("--json", action="store_true", help="emit JSON to stdout instead of summary")
    parser.add_argument("--min-size", type=int, default=2, help="omit singletons by default")
    args = parser.parse_args()

    clusters, unclassified, summary = build_clusters()
    write_outputs(clusters, unclassified, summary)

    if args.json:
        json.dump(
            {"clusters": clusters, "unclassified": unclassified},
            sys.stdout,
            indent=2,
            sort_keys=True,
        )
        sys.stdout.write("\n")
        return 0

    print(f"pending failures total: {sum(c['size'] for c in clusters) + len(unclassified)}")
    print(
        f"clusters: {len(clusters)} "
        f"(size>=2: {sum(1 for c in clusters if c['size'] >= 2)}, "
        f"unclassified: {len(unclassified)})",
    )
    print()
    shown = [c for c in clusters if c["size"] >= args.min_size][: args.top]
    print(f"top {len(shown)} clusters (size>={args.min_size}):")
    for cluster in shown:
        msg = cluster["message"]
        if len(msg) > 70:
            msg = msg[:67] + "..."
        print(f"  {cluster['size']:3d}  {cluster['location']:45s}  {msg}")
    print()
    print(f"wrote: {CLUSTERS_JSON.relative_to(REPO_ROOT)}")
    print(f"wrote: {CLUSTERS_MD.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
