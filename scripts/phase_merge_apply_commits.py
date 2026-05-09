#!/usr/bin/env python3
"""Apply selected paths from worker checkpoint commits to this worktree."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent


def run(args: list[str], *, input_text: str | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", *args],
        cwd=REPO_ROOT,
        input=input_text,
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(args)} failed\n"
            f"stdout:\n{result.stdout}\n\nstderr:\n{result.stderr}"
        )
    return result


def tracked_status() -> str:
    return run(["status", "--porcelain", "--untracked-files=no"]).stdout


def patch_for(commit: str, paths: list[str]) -> str:
    return run(["show", "--format=", "--no-ext-diff", "--no-renames", commit, "--", *paths]).stdout


def apply_commit(commit: str, paths: list[str], message_prefix: str) -> str:
    before = tracked_status()
    if before.strip():
        raise RuntimeError(
            "tracked worktree changes are present before applying "
            f"{commit}; commit or resolve them first\n{before}"
        )
    patch = patch_for(commit, paths)
    if not patch.strip():
        return f"skip {commit}: no selected-path patch"
    apply = run(["apply", "--3way", "--index", "-"], input_text=patch, check=False)
    if apply.returncode != 0:
        failure_path = REPO_ROOT / "xlate" / f"phase_merge_failed_{commit}.patch"
        failure_path.parent.mkdir(parents=True, exist_ok=True)
        failure_path.write_text(patch)
        raise RuntimeError(
            f"failed to apply {commit}; patch saved to {failure_path}\n"
            f"stdout:\n{apply.stdout}\n\nstderr:\n{apply.stderr}"
        )
    shortstat = run(["diff", "--cached", "--shortstat"]).stdout.strip()
    if not shortstat:
        run(["reset", "--quiet"])
        return f"skip {commit}: selected patch applied empty"
    message = f"{message_prefix} {commit}"
    run(["commit", "-m", message])
    new_commit = run(["rev-parse", "--short", "HEAD"]).stdout.strip()
    return f"committed {new_commit}: {message} ({shortstat})"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit", action="append", required=True, help="worker checkpoint commit to apply")
    parser.add_argument("--path", action="append", required=True, help="pathspec to take from each commit")
    parser.add_argument("--message-prefix", required=True)
    args = parser.parse_args()

    for commit in args.commit:
        print(apply_commit(commit, args.path, args.message_prefix), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
