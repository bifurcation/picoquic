#!/usr/bin/env python3
"""Print timestamped Phase 4E/5B worker status.

This script is intentionally read-only.  It summarizes the active worker
worktrees created for Phase 4E and Phase 5B, including process liveness,
repair deltas since launch, and the current function-map count.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from collections import Counter
from datetime import datetime
from pathlib import Path
from typing import Any

import phase4e


REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_WORKTREE_ROOT = Path("/private/tmp")
DEFAULT_PHASE5B_ROOT = Path("/private/tmp/picoquic-5b")


def load_json(path: Path, default: Any) -> Any:
    if not path.is_file():
        return default
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError:
        return default


def function_counts(repo: Path) -> Counter:
    mapping = load_json(repo / "xlate" / "function_translation_map.json", {"entries": []})
    return Counter(entry.get("required_action", "unknown") for entry in mapping.get("entries", []))


def outcome_counts(path: Path, key: str) -> Counter:
    data = load_json(path, {key: {}})
    return Counter(item.get("outcome", "unknown") for item in data.get(key, {}).values())


def cluster_size(path: Path) -> int:
    if not path.is_file():
        return 0
    return sum(1 for line in path.read_text().splitlines() if line.strip() and not line.lstrip().startswith("#"))


def cluster_ids(path: Path) -> list[str]:
    if not path.is_file():
        return []
    ids: list[str] = []
    for raw in path.read_text().splitlines():
        line = raw.split("#", 1)[0].strip()
        if line:
            ids.append(line)
    return ids


def screen_sessions() -> Counter:
    result = subprocess.run(["screen", "-ls"], capture_output=True, text=True)
    text = result.stdout + result.stderr
    counts: Counter = Counter()
    for match in re.finditer(r"\d+\.(fq(?:4e|5b)[A-Za-z0-9]*)\s+\(", text):
        name = match.group(1)
        if name.startswith("fq4e"):
            counts["phase4e"] += 1
        elif name.startswith("fq5b"):
            counts["phase5b"] += 1
    return counts


def process_counts() -> tuple[Counter, str | None]:
    result = subprocess.run(["ps", "-axo", "pid,comm,args"], capture_output=True, text=True)
    if result.returncode != 0:
        return Counter(), (result.stderr or result.stdout or "ps failed").strip()
    counts: Counter = Counter()
    for line in result.stdout.splitlines()[1:]:
        parts = line.split(maxsplit=2)
        if len(parts) < 3:
            continue
        _, comm, args = parts
        if comm == "python3" and "scripts/phase4e.py" in args:
            counts["phase4e_driver"] += 1
        elif comm == "python3" and "scripts/phase5b.py" in args:
            counts["phase5b_driver"] += 1
        elif comm == "codex" and "/private/tmp/picoquic-4e-" in args:
            counts["phase4e_codex"] += 1
        elif comm == "codex" and "/private/tmp/picoquic-5b-" in args:
            counts["phase5b_codex"] += 1
    return counts, None


def latest_line(path: Path) -> str:
    if not path.is_file():
        return ""
    lines = path.read_text(errors="replace").splitlines()
    return lines[-1] if lines else ""


def phase4e_status(repo: Path, worktree_root: Path, clusters: int) -> dict[str, Any]:
    all_ids: list[str] = []
    for index in range(clusters):
        suffix = f"{index:02d}"
        wt = worktree_root / f"picoquic-4e-{suffix}"
        all_ids.extend(cluster_ids(wt / "xlate" / "clusters" / "phase4e" / f"cluster-{suffix}.txt"))

    worktrees = [worktree_root / f"picoquic-4e-{index:02d}" for index in range(clusters)]
    worktrees.extend(sorted(worktree_root.glob("picoquic-4e-r2-*")))
    repair_maps: list[dict[str, Any]] = []
    active_logs = 0
    for wt in worktrees:
        repairs = load_json(wt / "xlate" / "phase4e_repairs.json", {"repairs": {}})
        repair_maps.append(repairs.get("repairs", {}))
        for log in (wt / "xlate" / "clusters").glob("phase4e*/worker*.out"):
            if latest_line(log):
                active_logs += 1

    delta: Counter = Counter()
    for c_id in all_ids:
        saw_needs_fix = False
        for repair_map in repair_maps:
            repair = repair_map.get(c_id)
            if not repair:
                continue
            if phase4e.repair_is_terminal(repair):
                delta[repair.get("outcome", "unknown")] += 1
                break
            if repair.get("outcome") == "needs_fix":
                saw_needs_fix = True
        else:
            if saw_needs_fix:
                delta["needs_fix"] += 1
            else:
                delta["pending"] += 1
    terminal = delta["fixed"] + delta["ok"] + delta["blocked"]
    return {
        "entries": len(all_ids),
        "delta": delta,
        "remaining": max(0, len(all_ids) - terminal),
        "active_logs": active_logs,
    }


def phase5b_status(phase5b_root: Path, worktree_root: Path, clusters: int) -> dict[str, Any]:
    baseline = outcome_counts(phase5b_root / "xlate" / "phase5b_repairs.json", "repairs")
    total_counts: Counter = Counter()
    total_entries = 0
    active_logs = 0
    for index in range(clusters):
        suffix = f"{index:02d}"
        wt = worktree_root / f"picoquic-5b-{suffix}"
        total_counts.update(outcome_counts(wt / "xlate" / "phase5b_repairs.json", "repairs"))
        total_entries += cluster_size(wt / "xlate" / "clusters" / "phase5b" / f"cluster-{suffix}.txt")
        if latest_line(wt / "xlate" / "clusters" / "phase5b" / f"worker-{suffix}.out"):
            active_logs += 1
    delta = Counter({
        outcome: total_counts.get(outcome, 0) - clusters * baseline.get(outcome, 0)
        for outcome in ("fixed", "ok", "blocked")
    })
    terminal = delta["fixed"] + delta["ok"] + delta["blocked"]
    return {
        "entries": total_entries,
        "delta": delta,
        "remaining": max(0, total_entries - terminal),
        "active_logs": active_logs,
    }


def print_report(args: argparse.Namespace) -> None:
    timestamp = datetime.now().astimezone().strftime("%Y-%m-%d %H:%M:%S %Z")
    f_counts = function_counts(args.repo)
    screens = screen_sessions()
    proc_counts, proc_error = process_counts()
    p4e = phase4e_status(args.repo, args.worktree_root, args.phase4e_clusters)
    p5b = phase5b_status(args.phase5b_root, args.worktree_root, args.phase5b_clusters)

    print(f"Status at {timestamp}")
    print(
        "Functions: "
        f"{f_counts.get('implemented', 0)} implemented / "
        f"{f_counts.get('required_missing', 0)} required missing "
        f"({f_counts.get('expected_omission', 0)} expected omissions)"
    )
    print("Cargo tests: not run here; deferred to Phase 5C")
    print(
        "Workers: "
        f"screens {screens.get('phase4e', 0)}/{args.phase4e_workers} 4E, "
        f"{screens.get('phase5b', 0)}/{args.phase5b_workers} 5B"
    )
    if proc_error:
        print(f"Processes: unavailable ({proc_error})")
    else:
        print(
            "Processes: "
            f"drivers {proc_counts.get('phase4e_driver', 0)}/{args.phase4e_workers} 4E, "
            f"{proc_counts.get('phase5b_driver', 0)}/{args.phase5b_workers} 5B; "
            f"codex {proc_counts.get('phase4e_codex', 0)}/{args.phase4e_workers} 4E, "
            f"{proc_counts.get('phase5b_codex', 0)}/{args.phase5b_workers} 5B"
        )
    print(
        "Phase 4E since launch: "
        f"{p4e['delta'].get('fixed', 0)} fixed / "
        f"{p4e['delta'].get('ok', 0)} ok / "
        f"{p4e['delta'].get('blocked', 0)} blocked / "
        f"{p4e['delta'].get('needs_fix', 0)} still needs_fix; "
        f"unresolved estimate {p4e['remaining']}/{p4e['entries']}; "
        f"nonempty logs {p4e['active_logs']}"
    )
    print(
        "Phase 5B since launch: "
        f"{p5b['delta'].get('fixed', 0)} fixed / "
        f"{p5b['delta'].get('ok', 0)} ok / "
        f"{p5b['delta'].get('blocked', 0)} blocked; "
        f"blocked remaining {p5b['delta'].get('blocked', 0)}/{p5b['entries']}; "
        f"unprocessed estimate {p5b['remaining']}/{p5b['entries']}; "
        f"nonempty logs {p5b['active_logs']}/{args.phase5b_clusters}"
    )
    print("", flush=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO_ROOT)
    parser.add_argument("--phase5b-root", type=Path, default=DEFAULT_PHASE5B_ROOT)
    parser.add_argument("--worktree-root", type=Path, default=DEFAULT_WORKTREE_ROOT)
    parser.add_argument("--phase4e-clusters", type=int, default=9)
    parser.add_argument("--phase4e-workers", type=int, default=10)
    parser.add_argument("--phase5b-clusters", type=int, default=11)
    parser.add_argument("--phase5b-workers", type=int, default=10)
    parser.add_argument("--watch", type=int, metavar="SECONDS", help="repeat forever at this interval")
    args = parser.parse_args()

    while True:
        print_report(args)
        if not args.watch:
            return 0
        time.sleep(args.watch)


if __name__ == "__main__":
    raise SystemExit(main())
