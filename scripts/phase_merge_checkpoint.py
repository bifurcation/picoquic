#!/usr/bin/env python3
"""Create checkpoint commits for Phase 4E/5B worker worktrees."""

from __future__ import annotations

import argparse
import subprocess
from dataclasses import dataclass
from pathlib import Path


DEFAULT_ROOT = Path("/private/tmp")


@dataclass(frozen=True)
class Worker:
    phase: str
    name: str
    worktree: Path
    paths: tuple[str, ...]


PHASE4E_PATHS = (
    "rs/fq/src",
    "xlate/function_translation_map.json",
    "xlate/phase4d_results.json",
    "xlate/phase4e_repairs.json",
    "xlate/phase4e_report.html",
)

PHASE5B_PATHS = (
    "rs/fq/src/tests",
    "xlate/test_translation_map.json",
    "xlate/phase5a_reviews.json",
    "xlate/phase5b_repairs.json",
    "xlate/phase5b_report.html",
)


def run(worktree: Path, args: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", "-C", str(worktree), *args],
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        raise RuntimeError(
            f"git -C {worktree} {' '.join(args)} failed\n"
            f"stdout:\n{result.stdout}\n\nstderr:\n{result.stderr}"
        )
    return result


def workers(root: Path) -> list[Worker]:
    out: list[Worker] = []
    for index in range(9):
        suffix = f"{index:02d}"
        out.append(Worker("4E", suffix, root / f"picoquic-4e-{suffix}", PHASE4E_PATHS))
    out.append(Worker("4E", "r2-09", root / "picoquic-4e-r2-09", PHASE4E_PATHS))
    for index in range(11):
        suffix = f"{index:02d}"
        out.append(Worker("5B", suffix, root / f"picoquic-5b-{suffix}", PHASE5B_PATHS))
    return out


def has_changes(worker: Worker) -> bool:
    status = run(worker.worktree, ["status", "--porcelain", "--", *worker.paths]).stdout
    return any(line.strip() for line in status.splitlines())


def checkpoint(worker: Worker, *, dry_run: bool) -> str:
    if not worker.worktree.is_dir():
        return f"missing {worker.phase} {worker.name}: {worker.worktree}"
    branch = run(worker.worktree, ["branch", "--show-current"]).stdout.strip()
    if not has_changes(worker):
        return f"skip {worker.phase} {worker.name}: no selected changes on {branch}"
    shortstat = run(worker.worktree, ["diff", "HEAD", "--shortstat", "--", *worker.paths]).stdout.strip()
    if dry_run:
        return f"would commit {worker.phase} {worker.name} on {branch}: {shortstat}"

    run(worker.worktree, ["restore", "--staged", "--", "."])
    run(worker.worktree, ["add", "--", *worker.paths])
    message = f"Checkpoint Phase {worker.phase} worker {worker.name}"
    run(worker.worktree, ["commit", "-m", message])
    commit = run(worker.worktree, ["rev-parse", "--short", "HEAD"]).stdout.strip()
    return f"committed {worker.phase} {worker.name} on {branch}: {commit} ({shortstat})"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    for worker in workers(args.root):
        print(checkpoint(worker, dry_run=args.dry_run), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
