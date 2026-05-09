#!/usr/bin/env python3
"""Build Phase 5B blocked-entry reclassification id-lists."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


DEFAULT_WORKTREE_ROOT = Path("/private/tmp")


@dataclass
class SourcePlan:
    cluster: str
    worktree: Path
    ids: list[str]


@dataclass
class RunnerPlan:
    name: str
    sources: list[SourcePlan]

    @property
    def entries(self) -> int:
        return sum(len(source.ids) for source in self.sources)


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


def blocked_ids(worktree: Path, cluster: str) -> list[str]:
    cluster_path = worktree / "xlate" / "clusters" / "phase5b" / f"cluster-{cluster}.txt"
    cluster_ids = load_ids(cluster_path)
    repairs = load_json(worktree / "xlate" / "phase5b_repairs.json", {"repairs": {}})
    repair_map = repairs.get("repairs", {})
    return [
        test_id
        for test_id in cluster_ids
        if repair_map.get(test_id, {}).get("outcome") == "blocked"
    ]


def build_sources(worktree_root: Path, clusters: int) -> list[SourcePlan]:
    sources: list[SourcePlan] = []
    for index in range(clusters):
        cluster = f"{index:02d}"
        worktree = worktree_root / f"picoquic-5b-{cluster}"
        ids = blocked_ids(worktree, cluster)
        if ids:
            sources.append(SourcePlan(cluster, worktree, ids))
    return sources


def assign_runners(sources: list[SourcePlan], runner_count: int) -> list[RunnerPlan]:
    runners = [RunnerPlan(f"r2-{index:02d}", []) for index in range(runner_count)]
    for source in sorted(sources, key=lambda item: (-len(item.ids), item.cluster)):
        runner = min(runners, key=lambda item: (item.entries, item.name))
        runner.sources.append(source)
    return [runner for runner in runners if runner.sources]


def write_source_lists(sources: list[SourcePlan], out_dir_name: str) -> None:
    for source in sources:
        out_dir = source.worktree / "xlate" / "clusters" / out_dir_name
        out_dir.mkdir(parents=True, exist_ok=True)
        path = out_dir / f"r2-{source.cluster}.txt"
        path.write_text("".join(f"{test_id}\n" for test_id in source.ids))


def write_manifest(runners: list[RunnerPlan], out_dir_name: str, manifest_path: Path) -> None:
    manifest = {
        "schema_version": 1,
        "phase": "phase5b_reclassify_blocked",
        "total_entries": sum(runner.entries for runner in runners),
        "runners": [
            {
                "name": runner.name,
                "entries": runner.entries,
                "sources": [
                    {
                        "cluster": source.cluster,
                        "worktree": str(source.worktree),
                        "entries": len(source.ids),
                        "id_list": str(
                            source.worktree
                            / "xlate"
                            / "clusters"
                            / out_dir_name
                            / f"r2-{source.cluster}.txt"
                        ),
                    }
                    for source in runner.sources
                ],
            }
            for runner in runners
        ],
    }
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worktree-root", type=Path, default=DEFAULT_WORKTREE_ROOT)
    parser.add_argument("--clusters", type=int, default=11)
    parser.add_argument("--runners", type=int, default=10)
    parser.add_argument("--out-dir-name", default="phase5b_reclassify")
    parser.add_argument(
        "--manifest",
        type=Path,
        default=DEFAULT_WORKTREE_ROOT / "picoquic-5b-reclassify-manifest.json",
    )
    args = parser.parse_args()

    sources = build_sources(args.worktree_root, args.clusters)
    runners = assign_runners(sources, args.runners)
    write_source_lists(sources, args.out_dir_name)
    write_manifest(runners, args.out_dir_name, args.manifest)

    total = sum(len(source.ids) for source in sources)
    print(f"Phase 5B blocked entries: {total} across {len(runners)} runner(s)")
    for runner in runners:
        source_text = ", ".join(
            f"5b-{source.cluster}:{len(source.ids)}" for source in runner.sources
        )
        print(f"{runner.name}\tentries={runner.entries}\t{source_text}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
