#!/usr/bin/env python3
"""Build Phase 4E retry id-lists from first-round worker worktrees."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import phase4e


DEFAULT_WORKTREE_ROOT = Path("/private/tmp")


@dataclass
class RetryPlan:
    name: str
    source: str
    worktree: Path
    ids: list[str]


def load_json(path: Path, default: Any) -> Any:
    if not path.is_file():
        return default
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError:
        return default


def load_ids(path: Path) -> list[str]:
    if not path.is_file():
        return []
    ids: list[str] = []
    for raw in path.read_text().splitlines():
        line = raw.split("#", 1)[0].strip()
        if line:
            ids.append(line)
    return ids


def unresolved_ids(worktree: Path, phase: str, cluster: str) -> list[str]:
    cluster_ids = load_ids(worktree / "xlate" / "clusters" / phase / f"cluster-{cluster}.txt")
    repairs = load_json(worktree / "xlate" / "phase4e_repairs.json", {"repairs": {}})
    repair_map = repairs.get("repairs", {})
    out: list[str] = []
    for c_id in cluster_ids:
        repair = repair_map.get(c_id)
        if repair is None or not phase4e.repair_is_terminal(repair):
            out.append(c_id)
    return out


def split_ids(ids: list[str]) -> tuple[list[str], list[str]]:
    midpoint = (len(ids) + 1) // 2
    return ids[:midpoint], ids[midpoint:]


def build_plan(worktree_root: Path, clusters: int, workers: int) -> list[RetryPlan]:
    per_cluster: list[tuple[str, Path, list[str]]] = []
    for index in range(clusters):
        cluster = f"{index:02d}"
        worktree = worktree_root / f"picoquic-4e-{cluster}"
        ids = unresolved_ids(worktree, "phase4e", cluster)
        if ids:
            per_cluster.append((cluster, worktree, ids))
    if not per_cluster:
        return []

    plans: list[RetryPlan] = []
    split_budget = max(0, workers - len(per_cluster))
    split_sources = set()
    for cluster, _, ids in sorted(per_cluster, key=lambda item: (-len(item[2]), item[0])):
        if split_budget == 0 or len(ids) < 2:
            break
        split_sources.add(cluster)
        split_budget -= 1

    for cluster, worktree, ids in per_cluster:
        if cluster in split_sources:
            first, second = split_ids(ids)
            plans.append(RetryPlan(f"r2-{cluster}a", cluster, worktree, first))
            plans.append(RetryPlan(f"r2-{cluster}b", cluster, worktree, second))
        else:
            plans.append(RetryPlan(f"r2-{cluster}", cluster, worktree, ids))
    return plans


def write_plan(plans: list[RetryPlan], out_dir_name: str) -> None:
    for plan in plans:
        out_dir = plan.worktree / "xlate" / "clusters" / out_dir_name
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / f"{plan.name}.txt").write_text("".join(f"{c_id}\n" for c_id in plan.ids))
    manifest = {
        "schema_version": 1,
        "clusters": [
            {
                "name": plan.name,
                "source": plan.source,
                "worktree": str(plan.worktree),
                "id_list": str(
                    plan.worktree
                    / "xlate"
                    / "clusters"
                    / out_dir_name
                    / f"{plan.name}.txt"
                ),
                "entries": len(plan.ids),
            }
            for plan in plans
        ],
    }
    if plans:
        manifest_path = plans[0].worktree.parent / "picoquic-4e-round2-manifest.json"
        manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worktree-root", type=Path, default=DEFAULT_WORKTREE_ROOT)
    parser.add_argument("--clusters", type=int, default=9)
    parser.add_argument("--workers", type=int, default=10)
    parser.add_argument("--out-dir-name", default="phase4e_round2")
    args = parser.parse_args()

    plans = build_plan(args.worktree_root, args.clusters, args.workers)
    write_plan(plans, args.out_dir_name)
    total = sum(len(plan.ids) for plan in plans)
    print(f"round2 Phase 4E entries: {total} across {len(plans)} worker(s)")
    for plan in plans:
        print(f"{plan.name}\tsource=4e-{plan.source}\tentries={len(plan.ids)}\tworktree={plan.worktree}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
