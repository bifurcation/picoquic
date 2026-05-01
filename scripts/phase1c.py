#!/usr/bin/env python3
"""phase1c.py — Phase 1C: address `// REVIEW` comments left by humans.

Workflow:
  1. Human reviewer adds `// REVIEW: <instruction>` comments to
     any line in any Rust file under `rs/fq/src/`.
  2. This script discovers files that contain plain
     `// REVIEW: ` markers (the open form,
     `// REVIEW(open): `, is excluded — those are the AI's
     "couldn't auto-resolve" signal and require human attention,
     not another AI pass).
  3. For each such file, invoke `claude -p` with: the file path,
     the extracted REVIEW comments (line + text), and a prompt
     telling claude to apply the requested change and remove the
     comment when done.  When claude can't fully resolve, it
     rewrites the comment as `// REVIEW(open): <reason>`.
  4. After claude finishes, run the gate (cargo fmt + clippy +
     check).  If the gate passes, mark ok.

State and logs:
  - xlate/phase1c_state.json   — per-file status keyed by Rust path.
  - xlate/phase1c_runs/*.log   — full stdout for each run.
  - xlate/claude_logs/phase1c/<path>.log — per-file transcript.
  - xlate/prompts/phase1c/<path>.md      — the prompt sent.

Usage:
  python3 scripts/phase1c.py
  python3 scripts/phase1c.py --file rs/fq/src/picoquic/cc_common.rs
  python3 scripts/phase1c.py --list             # print files w/ REVIEW
  python3 scripts/phase1c.py --limit 1
  python3 scripts/phase1c.py --dry-run
  python3 scripts/phase1c.py --status
  python3 scripts/phase1c.py --force
  python3 scripts/phase1c.py --stop-on-failure
"""

from __future__ import annotations

import argparse
import json
import re
import shlex
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT      = Path(__file__).resolve().parent.parent
PHASE1C_STATE  = REPO_ROOT / "xlate" / "phase1c_state.json"
PROMPTS_DIR    = REPO_ROOT / "xlate" / "prompts" / "phase1c"
LOG_DIR        = REPO_ROOT / "xlate" / "claude_logs" / "phase1c"
RUNS_DIR       = REPO_ROOT / "xlate" / "phase1c_runs"
RS_CRATE       = REPO_ROOT / "rs" / "fq"
RS_SRC         = RS_CRATE / "src"

# `Bash(cmd:*)` is prefix-match (allows args); bare `Bash(cmd)` is
# exact-match.  Use the wildcard so `cargo clippy -- -D warnings`
# is accepted.
ALLOWED_TOOLS  = "Read Edit Glob Grep Bash(cargo check:*) Bash(cargo clippy:*)"

# Plain REVIEW marker — the open form is excluded so we don't
# loop on comments the AI explicitly handed back to the human.
REVIEW_RE      = re.compile(r"//\s*REVIEW:\s*(.+)$")
OPEN_RE        = re.compile(r"//\s*REVIEW\(open\):\s*(.+)$")


# ---------------------------------------------------------------------------
# Discovery

def find_review_comments(path: Path) -> list[tuple[int, str]]:
    """Return a list of (line_number, text) for plain `// REVIEW:`
    comments in `path`.  Excludes the `// REVIEW(open):` form.
    """
    out: list[tuple[int, str]] = []
    try:
        text = path.read_text(errors="replace")
    except OSError:
        return out
    for i, line in enumerate(text.splitlines(), 1):
        if OPEN_RE.search(line):
            continue
        m = REVIEW_RE.search(line)
        if m:
            out.append((i, m.group(1).strip()))
    return out


def find_open_comments(path: Path) -> list[tuple[int, str]]:
    out: list[tuple[int, str]] = []
    try:
        text = path.read_text(errors="replace")
    except OSError:
        return out
    for i, line in enumerate(text.splitlines(), 1):
        m = OPEN_RE.search(line)
        if m:
            out.append((i, m.group(1).strip()))
    return out


def files_with_reviews() -> list[Path]:
    """All Rust files under `rs/fq/src/` containing `// REVIEW:` markers
    (the plain form), sorted by path.
    """
    if not RS_SRC.is_dir():
        return []
    found: list[Path] = []
    for p in sorted(RS_SRC.rglob("*.rs")):
        if find_review_comments(p):
            found.append(p)
    return found


# ---------------------------------------------------------------------------
# State

def load_state(path: Path) -> dict:
    if path.is_file():
        try:
            return json.loads(path.read_text())
        except json.JSONDecodeError:
            return {}
    return {}


def save_state(path: Path, state: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n")


def record(state: dict, key: str, status: str, **extra) -> None:
    state[key] = {
        "status": status,
        "at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        **extra,
    }
    save_state(PHASE1C_STATE, state)


# ---------------------------------------------------------------------------
# Path helpers

def rel(path: Path) -> str:
    return str(path.relative_to(REPO_ROOT))


def prompt_path_for(file: Path) -> Path:
    rel_rs = file.relative_to(RS_SRC)
    return PROMPTS_DIR / rel_rs.with_suffix(".md")


def claude_log_path_for(file: Path) -> Path:
    rel_rs = file.relative_to(RS_SRC)
    return LOG_DIR / rel_rs.with_suffix(".log")


# ---------------------------------------------------------------------------
# Prompt

def compose_prompt(file: Path, comments: list[tuple[int, str]]) -> str:
    rel_path = rel(file)
    lines: list[str] = []
    for line_no, text in comments:
        lines.append(f"  - line {line_no}: {text}")

    parts: list[str] = [
        f"# Phase 1C: address `// REVIEW` comments in `{rel_path}`",
        "",
        "A human reviewer left actionable `// REVIEW: <instruction>`",
        "comments in this file.  Apply each requested change and",
        "remove the comment when you're done.",
        "",
        "## Required reading",
        "1. The file: `" + rel_path + "`",
        "2. `TRANSLATE_PLAN.md` — Phase 1C section (and the Phase",
        "   1 contract that still applies: `todo!()` bodies, no",
        "   `Send`/`Sync`, no_std + alloc, etc.)",
        "3. `CLAUDE.md` — edit-scope rules.",
        "",
        f"## REVIEW comments in {rel_path}",
        "",
        *lines,
        "",
        "## Procedure",
        "1. Read the file and the related C source (the matching",
        "   `picoquic/<stem>.h` / `.c` files) where context helps.",
        "2. For each `// REVIEW:` comment in the file, apply the",
        "   requested change.",
        "   - If you fully address it, **remove the comment line**.",
        "   - If you can't fully address it (the request needs more",
        "     context, would break the gate, or the human's intent",
        "     is unclear), **rewrite the line as**",
        "     `// REVIEW(open): <one-line reason>` and leave it",
        "     for the human to revisit.  Don't drop a request",
        "     silently.",
        "3. Validate with **both** of these (Bash tool's cwd is the",
        "   repo root, so prefix with `cd rs/fq && `):",
        "     cd rs/fq && cargo check",
        "     cd rs/fq && cargo clippy -- -D warnings",
        "   Iterate until both pass cleanly.",
        "4. Report on stdout: how many comments resolved vs. left",
        "   open, and one sentence per non-trivial change.",
        "",
        "## Constraints",
        "- You may edit any file under `rs/fq/src/picoquic/` if a",
        "  REVIEW asks for a coordinated change.  Don't touch",
        "  `lib.rs`, `Cargo.toml`, parent `mod` files, or anything",
        "  outside `rs/fq/`.",
        "- Don't introduce new `// REVIEW:` comments.  Use",
        "  `// REVIEW(open):` for things you couldn't address.",
        "- Don't run `cargo test` or `cargo fmt`; the parent script",
        "  runs `fmt` after you.",
    ]
    return "\n".join(parts) + "\n"


def write_prompt_file(file: Path, comments: list[tuple[int, str]]) -> Path:
    out = prompt_path_for(file)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(compose_prompt(file, comments))
    return out


# ---------------------------------------------------------------------------
# Claude invocation

def invoke_claude(file: Path, prompt_file: Path,
                  max_turns: int, model: str) -> tuple[int, str]:
    if shutil.which("claude") is None:
        return 127, "claude CLI not found on PATH"
    log = claude_log_path_for(file)
    log.parent.mkdir(parents=True, exist_ok=True)
    prompt = prompt_file.read_text()
    cmd = [
        "claude", "-p", prompt,
        "--model", model,
        "--allowedTools", ALLOWED_TOOLS,
        "--max-turns", str(max_turns),
    ]
    t0 = time.monotonic()
    res = subprocess.run(
        cmd, cwd=REPO_ROOT,
        capture_output=True, text=True,
        stdin=subprocess.DEVNULL,
    )
    elapsed = time.monotonic() - t0
    transcript = (
        f"# claude -p (phase1c) for {rel(file)}\n"
        f"# elapsed: {elapsed:.1f}s, exit: {res.returncode}\n"
        f"# command: {' '.join(shlex.quote(c) for c in cmd[:1] + cmd[3:])}\n"
        f"# (prompt omitted; see "
        f"{prompt_file.relative_to(REPO_ROOT)})\n\n"
        f"## stdout\n{res.stdout}\n\n## stderr\n{res.stderr}\n"
    )
    log.write_text(transcript)
    return res.returncode, res.stdout


# ---------------------------------------------------------------------------
# Gate

def run_gate() -> int:
    steps = [
        ["cargo", "fmt"],
        ["cargo", "clippy", "--", "-D", "warnings"],
        ["cargo", "check"],
    ]
    for step in steps:
        sys.stdout.write(f"  $ (cd rs/fq && {' '.join(step)})\n")
        sys.stdout.flush()
        r = subprocess.run(step, cwd=RS_CRATE)
        if r.returncode != 0:
            return r.returncode
    return 0


# ---------------------------------------------------------------------------
# Per-file runner

def run_one(file: Path, *, dry_run: bool, max_turns: int,
            model: str, state: dict) -> str:
    key = rel(file)
    print(f"\n=== {key} ===")
    before = find_review_comments(file)
    if not before:
        print("  SKIP: no // REVIEW: comments")
        return "skip"
    print(f"  reviews → {len(before)} comment(s)")
    for line_no, text in before:
        snippet = text if len(text) <= 78 else text[:75] + "…"
        print(f"    line {line_no}: {snippet}")

    prompt_file = write_prompt_file(file, before)
    print(f"  prompt  → {prompt_file.relative_to(REPO_ROOT)}")
    if dry_run:
        print("  (dry-run; skipping claude + gate)")
        return "skip"

    print(f"  claude  → invoking ({model}, max-turns={max_turns}) …")
    code, _stdout = invoke_claude(file, prompt_file, max_turns, model)
    if code != 0:
        print(f"    FAIL: claude exit {code} "
              f"(see {claude_log_path_for(file).relative_to(REPO_ROOT)})")
        record(state, key, "fail", stage="claude", exit_code=code,
               reviews_before=len(before))
        return "fail"

    after = find_review_comments(file)
    open_after = find_open_comments(file)
    resolved = len(before) - len(after)
    opened = max(0, len(open_after))

    print(f"  result  → {resolved} resolved, {len(after)} remaining, "
          f"{opened} open")

    print("  gate    → cargo fmt + clippy + check")
    rc = run_gate()
    if rc != 0:
        print(f"    FAIL: gate exit {rc}")
        record(state, key, "fail", stage="gate", exit_code=rc,
               reviews_before=len(before),
               reviews_after=len(after), reviews_open=opened)
        return "fail"

    if after:
        print("  ok (partial — some REVIEW: comments remain; will retry "
              "on next run)")
        record(state, key, "partial",
               reviews_before=len(before),
               reviews_after=len(after), reviews_open=opened)
        return "ok"

    print("  ok")
    record(state, key, "ok",
           reviews_before=len(before),
           reviews_after=0, reviews_open=opened)
    return "ok"


# ---------------------------------------------------------------------------
# Status / list

def cmd_status() -> None:
    files = files_with_reviews()
    state = load_state(PHASE1C_STATE)
    print(f"Phase 1C status")
    print(f"  files with // REVIEW: comments now: {len(files)}")
    if files:
        for p in files:
            n = len(find_review_comments(p))
            print(f"    - {rel(p)}  ({n} comment{'s' if n != 1 else ''})")

    # Files with open comments but no plain REVIEW: — needs human attention.
    open_files: list[Path] = []
    for p in sorted(RS_SRC.rglob("*.rs")):
        if find_review_comments(p):
            continue
        if find_open_comments(p):
            open_files.append(p)
    if open_files:
        print(f"\n  files with REVIEW(open): only (need human attention):")
        for p in open_files:
            n = len(find_open_comments(p))
            print(f"    - {rel(p)}  ({n} open)")

    # Recent state entries.
    if state:
        recent = sorted(state.items(), key=lambda kv: kv[1].get("at", ""),
                        reverse=True)[:10]
        print(f"\n  last 10 runs:")
        for k, v in recent:
            print(f"    [{v.get('status'):7}] {k}  "
                  f"(at: {v.get('at')}; resolved: "
                  f"{v.get('reviews_before', '?')}→"
                  f"{v.get('reviews_after', '?')}, "
                  f"open: {v.get('reviews_open', '?')})")


def cmd_list() -> None:
    for p in files_with_reviews():
        print(rel(p))


# ---------------------------------------------------------------------------
# Run log

class _Tee:
    def __init__(self, *streams):
        self._streams = streams

    def write(self, s):
        for st in self._streams:
            st.write(s)
        return len(s)

    def flush(self):
        for st in self._streams:
            st.flush()


def setup_run_log() -> Path:
    RUNS_DIR.mkdir(parents=True, exist_ok=True)
    path = RUNS_DIR / time.strftime("%Y%m%dT%H%M%S.log")
    f = open(path, "w", buffering=1)
    sys.stdout = _Tee(sys.__stdout__, f)
    sys.stderr = _Tee(sys.__stderr__, f)
    return path


# ---------------------------------------------------------------------------
# Main

def main() -> int:
    p = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    p.add_argument("--file",
                   help="Process only this Rust file (path under rs/fq/src/).")
    p.add_argument("--limit", type=int, default=None,
                   help="Stop after processing N files.")
    p.add_argument("--dry-run", action="store_true",
                   help="Print plan; do not invoke claude or run the gate.")
    p.add_argument("--stop-on-failure", action="store_true",
                   help="Stop at the first failure (default is to continue).")
    p.add_argument("--force", action="store_true",
                   help="Process even files with no plain REVIEW: comments "
                        "(no-op unless --file is given too).")
    p.add_argument("--status", action="store_true",
                   help="Print progress and exit.")
    p.add_argument("--list", action="store_true",
                   help="Print files containing // REVIEW: comments and exit.")
    p.add_argument("--max-turns", type=int, default=80,
                   help="Per-file turn limit for claude (default 80).")
    p.add_argument("--model", default="sonnet",
                   help="Model passed to `claude -p --model` "
                        "(default: sonnet — REVIEW execution is "
                        "judgement-light).")
    args = p.parse_args()

    if args.status:
        cmd_status()
        return 0

    if args.list:
        cmd_list()
        return 0

    state = load_state(PHASE1C_STATE)

    if args.file:
        f = (REPO_ROOT / args.file).resolve()
        try:
            f.relative_to(RS_SRC)
        except ValueError:
            print(f"file must be under {RS_SRC.relative_to(REPO_ROOT)}: "
                  f"{args.file}", file=sys.stderr)
            return 1
        if not f.is_file():
            print(f"no such file: {args.file}", file=sys.stderr)
            return 1
        if not args.force and not find_review_comments(f):
            print(f"no // REVIEW: comments in {args.file} "
                  "(pass --force to invoke claude anyway)")
            return 0
        targets = [f]
    else:
        targets = files_with_reviews()

    if args.limit is not None:
        targets = targets[: args.limit]

    if not targets:
        print("no files contain // REVIEW: comments — nothing to do")
        return 0

    log_path = setup_run_log()
    t_run_start = time.monotonic()
    print(f"Phase 1C: {len(targets)} file(s) to process")
    print(f"  run log: {log_path.relative_to(REPO_ROOT)}")
    print(f"  state:   {PHASE1C_STATE.relative_to(REPO_ROOT)}")
    failed: list[str] = []
    succeeded: list[str] = []
    for f in targets:
        result = run_one(f, dry_run=args.dry_run,
                         max_turns=args.max_turns, model=args.model,
                         state=state)
        if result == "fail":
            failed.append(rel(f))
            if args.stop_on_failure:
                print(f"\nStopped at {rel(f)} (--stop-on-failure).  "
                      "Re-run to retry.")
                break
        elif result == "ok":
            succeeded.append(rel(f))

    elapsed = time.monotonic() - t_run_start
    print(f"\n=== Phase 1C run summary ===")
    print(f"  elapsed:   {elapsed:.1f}s")
    print(f"  succeeded: {len(succeeded)}")
    print(f"  failed:    {len(failed)}")
    for f in failed:
        print(f"    - {f}")
    print(f"  log:       {log_path.relative_to(REPO_ROOT)}")
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
