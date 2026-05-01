#!/usr/bin/env python3
"""phase1a.py — Phase 1A: AI self-review of translated headers.

For each header that finished Phase 1 cleanly, re-read the
translation and apply quality improvements (safety, consistency,
idiomatic Rust) in place.  This is a refinement pass — the
existing module is the starting point, never overwritten from
scratch.

Per-header:
  1. Compose a review prompt that names the file, the C source,
     and the relevant policy sections.
  2. Invoke `claude -p` with a tool allowlist that includes
     Read/Edit/Glob/Grep + Bash(cargo check)/Bash(cargo clippy).
     `Write` is deliberately excluded — refinement only.
  3. Claude iterates `cargo check` + `cargo clippy` itself, then
     the parent runs the same gate as a final check.

State and logs:
  - xlate/phase1a_state.json — per-header status
    (ok / fail / noop).  Re-runs skip ok and noop; failures retry.
  - xlate/phase1a_runs/<timestamp>.log — full stdout for each run.
  - xlate/claude_logs/phase1a/<path>.log — per-header transcript.
  - xlate/prompts/phase1a/<path>.md — the prompt sent to claude.

Usage:
  python3 scripts/phase1a.py
  python3 scripts/phase1a.py --header picoquic/cc_common.h
  python3 scripts/phase1a.py --limit 1
  python3 scripts/phase1a.py --dry-run
  python3 scripts/phase1a.py --status
  python3 scripts/phase1a.py --force            # redo finished
  python3 scripts/phase1a.py --stop-on-failure
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
INV_PATH       = REPO_ROOT / "xlate" / "inventory.json"
PHASE1_STATE   = REPO_ROOT / "xlate" / "phase1_state.json"
PHASE1A_STATE  = REPO_ROOT / "xlate" / "phase1a_state.json"
PROMPTS_DIR    = REPO_ROOT / "xlate" / "prompts" / "phase1a"
LOG_DIR        = REPO_ROOT / "xlate" / "claude_logs" / "phase1a"
RUNS_DIR       = REPO_ROOT / "xlate" / "phase1a_runs"
RS_CRATE       = REPO_ROOT / "rs" / "fq"
RS_SRC         = RS_CRATE / "src"

# `Write` is intentionally NOT in the allowlist — claude should
# refine the existing file via Edit, not regenerate from scratch.
ALLOWED_TOOLS  = "Read Edit Glob Grep Bash(cargo check) Bash(cargo clippy)"

IN_SCOPE = {"picoquic"}

INCLUDE_RE = re.compile(r'^\s*#\s*include\s+"([^"]+)"', re.M)


# ---------------------------------------------------------------------------
# Header order: same topological order Phase 1 used (leaves-first).

def in_scope_headers(inv: dict) -> list[str]:
    return sorted(
        f["file"] for f in inv["files"] if f["file"].endswith(".h")
    )


def header_local_includes(header_path: Path) -> set[str]:
    try:
        text = header_path.read_text(errors="replace")
    except OSError:
        return set()
    deps: set[str] = set()
    for inc in INCLUDE_RE.findall(text):
        for d in IN_SCOPE:
            cand = (REPO_ROOT / d / inc).resolve()
            try:
                rel = cand.relative_to(REPO_ROOT)
            except ValueError:
                continue
            if rel.parts and rel.parts[0] in IN_SCOPE and cand.is_file():
                deps.add(str(rel))
                break
    return deps


def topological_order(headers: list[str]) -> list[str]:
    edges: dict[str, set[str]] = {h: set() for h in headers}
    for h in headers:
        for d in header_local_includes(REPO_ROOT / h):
            if d in edges and d != h:
                edges[h].add(d)
    in_degree: dict[str, int] = {h: 0 for h in headers}
    for h, deps in edges.items():
        for d in deps:
            in_degree[h] += 1
    ready = sorted(h for h, n in in_degree.items() if n == 0)
    out: list[str] = []
    seen: set[str] = set()
    while ready:
        h = ready.pop(0)
        if h in seen:
            continue
        seen.add(h)
        out.append(h)
        for h2, deps in edges.items():
            if h in deps:
                in_degree[h2] -= 1
                if in_degree[h2] == 0 and h2 not in seen:
                    ready.append(h2)
        ready.sort()
    # Append any cycles in alphabetical order so we don't drop them.
    for h in headers:
        if h not in seen:
            out.append(h)
    return out


# ---------------------------------------------------------------------------
# Path helpers (mirror Phase 1)

def rust_path_for(header: str) -> Path:
    p = Path(header).with_suffix(".rs")
    return RS_SRC / p


def prompt_path(header: str) -> Path:
    return PROMPTS_DIR / Path(header).with_suffix(".md")


def claude_log_path(header: str) -> Path:
    return LOG_DIR / Path(header).with_suffix(".log")


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


def record(state: dict, header: str, status: str, **extra) -> None:
    state[header] = {
        "status": status,
        "at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        **extra,
    }
    save_state(PHASE1A_STATE, state)


def is_done(header: str, state: dict) -> bool:
    s = state.get(header, {}).get("status")
    return s in {"ok", "noop"}


# ---------------------------------------------------------------------------
# Phase 1 eligibility

def phase1_ok_headers() -> set[str]:
    if not PHASE1_STATE.is_file():
        return set()
    s = load_state(PHASE1_STATE)
    return {h for h, v in s.items() if v.get("status") == "ok"}


# ---------------------------------------------------------------------------
# Prompt

def compose_prompt(header: str) -> str:
    rust_target = rust_path_for(header).relative_to(REPO_ROOT)
    src_h = REPO_ROOT / header
    src_c = src_h.with_suffix(".c")
    has_c = src_c.exists()
    deps = sorted(header_local_includes(src_h))

    if has_c:
        c_line = f"4. The matching `.c` file: `{src_c.relative_to(REPO_ROOT)}`."
    else:
        c_line = "4. (No matching `.c` file — header-only module.)"

    if deps:
        deps_lines = ["5. Headers this one directly depends on:"]
        deps_lines.extend(f"   - `{d}` (translation: `rs/fq/src/{d[:-2]}.rs`)"
                          for d in deps)
    else:
        deps_lines = ["5. (No in-scope dependencies.)"]

    parts: list[str] = [
        f"# Phase 1A self-review: `{header}`",
        "",
        "You're doing a self-review pass on a Phase 1 translation.",
        "Re-read the existing translation and apply quality",
        "improvements in place.  The existing module is gate-clean",
        "and represents real work — refine via `Edit`, never rewrite",
        "from scratch.",
        "",
        "## Required reading",
        "1. `TRANSLATE_PLAN.md` — Phase 1A section especially.",
        "2. `CLAUDE.md` — project conventions, edit-scope rules.",
        f"3. The existing translation: `{rust_target}`.",
        c_line,
        *deps_lines,
        "",
        "## What to look for",
        "- **Safety holes** — raw pointers used without a clear",
        "  `// SAFETY:` story, `unsafe` blocks that have a safe",
        "  equivalent, ownership patterns that smell wrong.",
        "- **Type-shape consistency** — same C type translated",
        "  differently in different functions of the same module;",
        "  signed/unsigned that disagrees with caller arithmetic;",
        "  integer widths that drift from the C source.",
        "- **Idiom** — `&[T]` vs raw pointer + length, `Option<&T>`",
        "  for nullable, owned `String` vs borrowed `&str`,",
        "  function-pointer typedef → trait, doc-comment placement.",
        "- **Naming** — Rust traits are `PascalCase` even when the",
        "  C origin is `snake_case`.  Preserve the C name where",
        "  callers reference the typedef identifier; let Rust",
        "  convention win for purely internal traits.",
        "- **Lint allowances** — tighten any `#![allow(…)]` that's",
        "  wider than necessary.",
        "- **Documentation** — every public item should have a doc",
        "  comment citing the C source and explaining non-obvious",
        "  shape decisions.",
        "",
        "## How to work",
        "1. Read the materials above.",
        "2. Identify improvements you'd apply.",
        f"3. Apply them via `Edit` to `{rust_target}` (and sibling",
        "   modules under `rs/fq/src/picoquic/` if a coordinated",
        "   change is needed).",
        "4. From `rs/fq/`, run **both** `cargo check` and",
        "   `cargo clippy -- -D warnings`.  Iterate until both",
        "   pass cleanly.",
        "5. Report on stdout: a one-paragraph summary of what you",
        "   changed and why.",
        "",
        "If nothing needs improvement, say so on stdout and exit",
        "without editing — that is a valid outcome.",
        "",
        "## Constraints",
        "- You may only edit files under `rs/fq/src/picoquic/`.",
        "  Do NOT touch `lib.rs`, `Cargo.toml`, parent `mod` files,",
        "  or anything outside `rs/fq/`.",
        "- Don't create new files (`Write` is not in your allowlist).",
        "- Don't run `cargo test` or `cargo fmt` — the parent",
        "  script handles formatting after you finish.",
        "- This pass should not introduce `// REVIEW` comments;",
        "  those are the human reviewer's tool in Phase 1B.",
    ]
    return "\n".join(parts) + "\n"


def write_prompt_file(header: str) -> Path:
    out = prompt_path(header)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(compose_prompt(header))
    return out


# ---------------------------------------------------------------------------
# Claude invocation

def invoke_claude(header: str, prompt_file: Path,
                  max_turns: int) -> tuple[int, str]:
    if shutil.which("claude") is None:
        return 127, "claude CLI not found on PATH"
    log = claude_log_path(header)
    log.parent.mkdir(parents=True, exist_ok=True)
    prompt = prompt_file.read_text()
    cmd = [
        "claude", "-p", prompt,
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
        f"# claude -p (phase1a) for {header}\n"
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
# Per-header runner

NOOP_MARKERS = (
    "no improvements",
    "nothing to improve",
    "no changes needed",
    "no edits needed",
    "no edits made",
)


def stdout_looks_like_noop(stdout: str) -> bool:
    """Best-effort heuristic for the 'I didn't change anything' outcome."""
    text = stdout.lower()
    return any(m in text for m in NOOP_MARKERS)


def run_one(header: str, *, dry_run: bool, max_turns: int,
            state: dict) -> str:
    print(f"\n=== {header} ===")
    rs_target = rust_path_for(header)
    if not rs_target.is_file():
        msg = f"missing translation at {rs_target.relative_to(REPO_ROOT)}"
        print(f"  SKIP: {msg}")
        record(state, header, "fail", reason=msg)
        return "fail"

    prompt_file = write_prompt_file(header)
    print(f"  prompt  → {prompt_file.relative_to(REPO_ROOT)}")
    if dry_run:
        print("  (dry-run; skipping claude + gate)")
        return "skip"

    print(f"  claude  → invoking (max-turns={max_turns}) …")
    before = rs_target.read_text()
    code, stdout = invoke_claude(header, prompt_file, max_turns)
    if code != 0:
        print(f"    FAIL: claude exit {code} "
              f"(see {claude_log_path(header).relative_to(REPO_ROOT)})")
        record(state, header, "fail", stage="claude", exit_code=code)
        return "fail"
    after = rs_target.read_text()
    changed = before != after

    if not changed and stdout_looks_like_noop(stdout):
        print("  noop    → claude reported no improvements needed")
        record(state, header, "noop")
        return "ok"

    print("  gate    → cargo fmt + clippy + check")
    rc = run_gate()
    if rc != 0:
        print(f"    FAIL: gate exit {rc}")
        record(state, header, "fail", stage="gate", exit_code=rc)
        return "fail"
    print("  ok" + ("" if changed else " (no edits)"))
    record(state, header, "ok", changed=changed)
    return "ok"


# ---------------------------------------------------------------------------
# Status

def cmd_status(headers: list[str]) -> None:
    state = load_state(PHASE1A_STATE)
    p1_ok = phase1_ok_headers()
    eligible = [h for h in headers if h in p1_ok]
    ok = [h for h in eligible if state.get(h, {}).get("status") == "ok"]
    noop = [h for h in eligible if state.get(h, {}).get("status") == "noop"]
    failed = [h for h in eligible if state.get(h, {}).get("status") == "fail"]
    pending = [h for h in eligible if not is_done(h, state)
               and state.get(h, {}).get("status") != "fail"]

    print(f"Phase 1A progress: {len(eligible)} eligible "
          f"(of {len(headers)} total in scope)")
    print(f"  ok (changed):         {len(ok)}")
    print(f"  noop (no changes):    {len(noop)}")
    print(f"  failed:               {len(failed)}")
    print(f"  pending:              {len(pending)}")
    if failed:
        print("\nFailed headers (will retry on re-run):")
        for h in failed:
            at = state[h].get("at", "?")
            print(f"  - {h}  (last: {at})")


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
    p.add_argument("--header",
                   help="Review only this header (relative path).")
    p.add_argument("--limit", type=int, default=None,
                   help="Stop after reviewing N headers.")
    p.add_argument("--dry-run", action="store_true",
                   help="Print plan; do not invoke claude or run the gate.")
    p.add_argument("--stop-on-failure", action="store_true",
                   help="Stop at the first failure (default is to continue).")
    p.add_argument("--force", action="store_true",
                   help="Re-review headers already marked ok or noop.")
    p.add_argument("--status", action="store_true",
                   help="Print progress and exit.")
    p.add_argument("--print-order", action="store_true",
                   help="Print headers in topological review order and exit.")
    p.add_argument("--max-turns", type=int, default=80,
                   help="Per-header turn limit for claude (default 80).")
    args = p.parse_args()

    if not INV_PATH.is_file():
        print(f"missing {INV_PATH}; run scripts/phase0.py first",
              file=sys.stderr)
        return 1

    inv = json.loads(INV_PATH.read_text())
    all_headers = in_scope_headers(inv)
    order = topological_order(all_headers)

    if args.status:
        cmd_status(order)
        return 0

    if args.print_order:
        for h in order:
            print(h)
        return 0

    state = load_state(PHASE1A_STATE)
    p1_ok = phase1_ok_headers()
    if not p1_ok:
        print("no headers are marked ok in Phase 1 state — nothing to review",
              file=sys.stderr)
        return 1

    if args.header:
        if args.header not in all_headers:
            print(f"not in scope: {args.header}", file=sys.stderr)
            return 1
        if args.header not in p1_ok:
            print(f"not Phase-1-ok: {args.header}", file=sys.stderr)
            return 1
        targets = [args.header]
    else:
        targets = [h for h in order if h in p1_ok]

    if not args.force:
        targets = [h for h in targets if not is_done(h, state)]

    if args.limit is not None:
        targets = targets[: args.limit]

    if not targets:
        print("nothing to do (use --force to redo finished headers)")
        return 0

    log_path = setup_run_log()
    t_run_start = time.monotonic()
    print(f"Phase 1A: {len(targets)} header(s) to review")
    print(f"  run log: {log_path.relative_to(REPO_ROOT)}")
    print(f"  state:   {PHASE1A_STATE.relative_to(REPO_ROOT)}")
    failed: list[str] = []
    succeeded: list[str] = []
    for h in targets:
        result = run_one(h, dry_run=args.dry_run,
                         max_turns=args.max_turns, state=state)
        if result == "fail":
            failed.append(h)
            if args.stop_on_failure:
                print(f"\nStopped at {h} (--stop-on-failure).  Re-run to retry.")
                break
        elif result == "ok":
            succeeded.append(h)

    elapsed = time.monotonic() - t_run_start
    print(f"\n=== Phase 1A run summary ===")
    print(f"  elapsed:   {elapsed:.1f}s")
    print(f"  succeeded: {len(succeeded)}")
    print(f"  failed:    {len(failed)}")
    for h in failed:
        print(f"    - {h}")
    print(f"  log:       {log_path.relative_to(REPO_ROOT)}")
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
