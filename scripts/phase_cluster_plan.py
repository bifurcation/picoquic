#!/usr/bin/env python3
"""Build balanced cluster files and launch commands for repair phases."""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path
from typing import Any

import phase4d
import phase4e
import phase5a
import phase5b
from phase4_common import REPO_ROOT, XLATE, load_function_map, save_json
from phase5_common import load_test_map


DEFAULT_PHASES = ("phase4e", "phase5b")


def entry_id(phase: str, entry: dict[str, Any]) -> str:
    if phase == "phase4e":
        return entry["c_id"]
    if phase == "phase5b":
        return entry["test_id"]
    raise ValueError(f"unknown phase: {phase}")


def entry_file(phase: str, entry: dict[str, Any]) -> str:
    if phase == "phase4e":
        return entry.get("rust", {}).get("file", "<unknown>")
    if phase == "phase5b":
        return entry.get("expected_rust_file", "<unknown>")
    raise ValueError(f"unknown phase: {phase}")


def entry_line(phase: str, entry: dict[str, Any]) -> int:
    if phase == "phase4e":
        return int(entry.get("rust", {}).get("start_line") or 0)
    if phase == "phase5b":
        return int((entry.get("rust") or {}).get("start_line") or 0)
    raise ValueError(f"unknown phase: {phase}")


def selectable_entries(phase: str, *, force: bool) -> list[dict[str, Any]]:
    if phase == "phase4e":
        mapping = load_function_map()
        return phase4e.needs_fix_entries(
            mapping,
            phase4d.phase4c_review_map(),
            phase4d.load_results(),
            phase4e.load_repairs(),
            only=None,
            rust_file=None,
            force=force,
        )
    if phase == "phase5b":
        mapping = load_test_map(refresh=False)
        return phase5b.needs_fix_entries(
            mapping,
            phase5a.load_reviews(),
            phase5b.load_repairs(),
            only=None,
            rust_file=None,
            force=force,
        )
    raise ValueError(f"unknown phase: {phase}")


def chunks_for_entries(
    phase: str,
    entries: list[dict[str, Any]],
    cluster_count: int,
) -> list[list[dict[str, Any]]]:
    target = max(1, math.ceil(len(entries) / cluster_count))
    by_file: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for entry in entries:
        by_file[entry_file(phase, entry)].append(entry)

    chunks: list[list[dict[str, Any]]] = []
    for _, group in sorted(by_file.items(), key=lambda item: (-len(item[1]), item[0])):
        group.sort(key=lambda entry: (entry_line(phase, entry), entry_id(phase, entry)))
        for start in range(0, len(group), target):
            chunks.append(group[start:start + target])
    chunks.sort(
        key=lambda chunk: (
            -len(chunk),
            entry_file(phase, chunk[0]) if chunk else "",
            entry_line(phase, chunk[0]) if chunk else 0,
        )
    )
    return chunks


def assign_clusters(
    phase: str,
    entries: list[dict[str, Any]],
    cluster_count: int,
) -> list[list[dict[str, Any]]]:
    clusters: list[list[dict[str, Any]]] = [[] for _ in range(cluster_count)]
    loads = [0 for _ in range(cluster_count)]
    for chunk in chunks_for_entries(phase, entries, cluster_count):
        index = min(range(cluster_count), key=lambda i: (loads[i], i))
        clusters[index].extend(chunk)
        loads[index] += len(chunk)
    for cluster in clusters:
        cluster.sort(key=lambda entry: (entry_file(phase, entry), entry_line(phase, entry), entry_id(phase, entry)))
    return clusters


def cluster_summary(phase: str, cluster: list[dict[str, Any]]) -> dict[str, Any]:
    files: dict[str, int] = defaultdict(int)
    for entry in cluster:
        files[entry_file(phase, entry)] += 1
    return {
        "entries": len(cluster),
        "files": [
            {"file": file, "entries": count}
            for file, count in sorted(files.items(), key=lambda item: (-item[1], item[0]))
        ],
    }


def write_phase_clusters(
    phase: str,
    entries: list[dict[str, Any]],
    *,
    cluster_count: int,
    out_root: Path,
    worktree_prefix: str,
    branch_prefix: str,
    target_prefix: str,
    agent: str,
    batch_size: int,
    max_turns: int,
) -> dict[str, Any]:
    clusters = assign_clusters(phase, entries, cluster_count)
    out_dir = out_root / phase
    out_dir.mkdir(parents=True, exist_ok=True)

    cluster_records: list[dict[str, Any]] = []
    command_lines: list[str] = [
        f"# {phase} launch commands",
        "# Generated only; do not run automatically.",
        "",
    ]
    script = "scripts/phase4e.py" if phase == "phase4e" else "scripts/phase5b.py"
    phase_suffix = "4e" if phase == "phase4e" else "5b"

    for index, cluster in enumerate(clusters):
        cluster_name = f"cluster-{index:02d}"
        cluster_file = out_dir / f"{cluster_name}.txt"
        cluster_file.write_text(
            "".join(f"{entry_id(phase, entry)}\n" for entry in cluster)
        )
        worktree = f"{worktree_prefix}-{phase_suffix}-{index:02d}"
        branch = f"{branch_prefix}-{phase_suffix}-cluster-{index:02d}"
        target_dir = f"{target_prefix}-{phase_suffix}-{index:02d}"
        rel_cluster = cluster_file.relative_to(REPO_ROOT).as_posix()
        command = (
            f"cd {worktree} && "
            f"CARGO_TARGET_DIR={target_dir} "
            f"python3 {script} --agent {agent} --batch-size {batch_size} "
            f"--max-turns {max_turns} --id-list {rel_cluster}"
        )
        command_lines.append(command)
        cluster_records.append(
            {
                "index": index,
                "cluster_file": rel_cluster,
                "worktree": worktree,
                "branch": branch,
                "target_dir": target_dir,
                "command": command,
                **cluster_summary(phase, cluster),
            }
        )

    (out_dir / "RUN_COMMANDS.md").write_text("\n".join(command_lines) + "\n")
    manifest = {
        "schema_version": 1,
        "phase": phase,
        "cluster_count": cluster_count,
        "total_entries": len(entries),
        "agent": agent,
        "batch_size": batch_size,
        "max_turns": max_turns,
        "clusters": cluster_records,
    }
    save_json(out_dir / "manifest.json", manifest)
    return manifest


def parse_phase_counts(values: list[str]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for value in values:
        if "=" not in value:
            raise argparse.ArgumentTypeError("--phase-count entries must be phase=count")
        phase, count_text = value.split("=", 1)
        if phase not in DEFAULT_PHASES:
            raise argparse.ArgumentTypeError(f"unknown phase: {phase}")
        count = int(count_text)
        if count < 1:
            raise argparse.ArgumentTypeError("cluster counts must be positive")
        counts[phase] = count
    return counts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--phase",
        choices=[*DEFAULT_PHASES, "both"],
        default="both",
        help="phase to plan",
    )
    parser.add_argument(
        "--phase-count",
        action="append",
        default=[],
        metavar="PHASE=N",
        help="cluster count for a phase, e.g. phase4e=9",
    )
    parser.add_argument("--out-root", default=str(XLATE / "clusters"))
    parser.add_argument("--worktree-prefix", default="/private/tmp/picoquic")
    parser.add_argument("--branch-prefix", default="c2rust")
    parser.add_argument("--target-prefix", default="/private/tmp/fq-target")
    parser.add_argument("--agent", default="codex")
    parser.add_argument("--batch-size", type=int, default=1)
    parser.add_argument("--max-turns", type=int, default=250)
    parser.add_argument("--force", action="store_true", help="include entries with existing terminal repairs")
    args = parser.parse_args()

    phase_counts = {"phase4e": 9, "phase5b": 11}
    phase_counts.update(parse_phase_counts(args.phase_count))
    phases = DEFAULT_PHASES if args.phase == "both" else (args.phase,)
    out_root = Path(args.out_root)
    if not out_root.is_absolute():
        out_root = REPO_ROOT / out_root

    manifests: dict[str, Any] = {}
    for phase in phases:
        entries = selectable_entries(phase, force=args.force)
        manifest = write_phase_clusters(
            phase,
            entries,
            cluster_count=phase_counts[phase],
            out_root=out_root,
            worktree_prefix=args.worktree_prefix,
            branch_prefix=args.branch_prefix,
            target_prefix=args.target_prefix,
            agent=args.agent,
            batch_size=args.batch_size,
            max_turns=args.max_turns,
        )
        manifests[phase] = manifest
        print(
            f"{phase}: {manifest['total_entries']} entries across "
            f"{manifest['cluster_count']} clusters"
        )
        for cluster in manifest["clusters"]:
            dominant = cluster["files"][0] if cluster["files"] else {"file": "", "entries": 0}
            print(
                f"  {cluster['index']:02d}: {cluster['entries']:3d} entries, "
                f"top file {dominant['file']} ({dominant['entries']})"
            )

    save_json(out_root / "manifest.json", {"schema_version": 1, "phases": manifests})
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
