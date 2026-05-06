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
  2. Invoke the configured AI agent with a tool allowlist that includes
     Read/Edit/Glob/Grep + Bash(cargo check)/Bash(cargo clippy).
     `Write` is deliberately excluded — refinement only.
  3. The agent iterates `cargo check` + `cargo clippy` itself, then
     the parent runs the same gate as a final check.

State and logs:
  - xlate/phase1a_state.json — per-header status
    (ok / fail / noop).  Re-runs skip ok and noop; failures retry.
  - xlate/phase1a_runs/<timestamp>.log — full stdout for each run.
  - xlate/<agent>_logs/phase1a/<path>.log — per-header transcript.
  - xlate/prompts/phase1a/<path>.md — the prompt sent to the agent.

Usage:
  python3 scripts/phase1a.py
  python3 scripts/phase1a.py --header picoquic/cc_common.h
  python3 scripts/phase1a.py --limit 1
  python3 scripts/phase1a.py --dry-run
  python3 scripts/phase1a.py --status
  python3 scripts/phase1a.py --force            # redo finished
  python3 scripts/phase1a.py --stop-on-failure
  python3 scripts/phase1a.py --agent codex
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path

import agent_runner

REPO_ROOT      = Path(__file__).resolve().parent.parent
INV_PATH       = REPO_ROOT / "xlate" / "inventory.json"
PHASE1_STATE   = REPO_ROOT / "xlate" / "phase1_state.json"
PHASE1A_STATE  = REPO_ROOT / "xlate" / "phase1a_state.json"
PROMPTS_DIR    = REPO_ROOT / "xlate" / "prompts" / "phase1a"
RUNS_DIR       = REPO_ROOT / "xlate" / "phase1a_runs"
RS_CRATE       = REPO_ROOT / "rs" / "fq"
RS_SRC         = RS_CRATE / "src"

# `Write` is intentionally NOT in the allowlist — the agent should
# refine the existing file via Edit, not regenerate from scratch.
# Bash granular permissions: `Bash(cmd:*)` is the prefix-match form
# that allows arguments.  The bare `Bash(cmd)` form is exact-match
# only, which fails for `cargo clippy -- -D warnings` etc.
ALLOWED_TOOLS  = "Read Edit Glob Grep Bash(cargo check:*) Bash(cargo clippy:*)"

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
    """Map `picoquic/<x>.h` to its Rust translation path.

    Reflects the Stage 1 flatten + pico-prefix strip:
    - `picoquic/picoquic.h` (kitchen-sink) lives in `lib.rs`.
    - `picoquic/picoquic_<X>.h` → `rs/fq/src/<X>.rs`.
    - `picoquic/picoquictest_<X>.h` → `rs/fq/src/test_<X>.rs`.
    - `picoquic/pico<X>.h` → `rs/fq/src/<X>.rs` (hash, socks, splay).
    - Everything else (no pico prefix) keeps its name.
    """
    name = Path(header).stem
    if name == "picoquic":
        return RS_SRC / "lib.rs"
    if name.startswith("picoquictest_"):
        name = "test_" + name[len("picoquictest_"):]
    elif name.startswith("picoquic_"):
        name = name[len("picoquic_"):]
    elif name.startswith("pico"):
        name = name[len("pico"):]
    return RS_SRC / f"{name}.rs"


def prompt_path(header: str) -> Path:
    return PROMPTS_DIR / Path(header).with_suffix(".md")


def agent_log_path(agent: agent_runner.AgentConfig, header: str) -> Path:
    return agent_runner.log_dir(REPO_ROOT, agent, "phase1a") / Path(header).with_suffix(".log")


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
        "but is too deferential to the C API — your job is to make",
        "it idiomatic Rust.  Refine via `Edit`, never rewrite from",
        "scratch.",
        "",
        "## Required reading",
        "1. `TRANSLATE_PLAN.md` — Phase 1A section especially.",
        "2. `CLAUDE.md` — project conventions, edit-scope rules.",
        f"3. The existing translation: `{rust_target}`.",
        c_line,
        *deps_lines,
        "",
        "## Top-level rule",
        "",
        "**The translation should not look obviously C-derived.**",
        "Would a Rust programmer who hadn't seen the C source write",
        "this?  If not, change it.  Specifically:",
        "",
        "- **Naming convention is universal.**  `quic_t` → `Quic`.",
        "  `cnx_t` → `Cnx`.  `state_enum` → `State`.",
        "  Drop `_t` and `_enum` suffixes.  PascalCase types,",
        "  traits, and enum variants.  snake_case fields and",
        "  methods.  `SCREAMING_SNAKE` constants.  Rename **every**",
        "  identifier that doesn't match — types, traits, fields,",
        "  enum variants, free functions.",
        "- **Free functions on a primary `&T` / `&mut T` argument",
        "  are methods on `T`.**  `set_low_memory_mode(quic: &mut",
        "  Quic, …)` → `impl Quic { fn set_low_memory_mode(&mut",
        "  self, …) }`.  Same for getters, builders, and any",
        "  function whose first argument is the type's \"self\".",
        "- **Use Rust's memory model.**  `Drop` replaces explicit",
        "  `free`.  `Vec` / owning `String` replace `malloc`/`free`",
        "  pairs.  `Box<T>` parameters when the function doesn't",
        "  need heap-stable storage are wrong — fix them (clippy",
        "  flags this as `boxed_local`).",
        "- **Use Rust's error model.**  `Result<T, Error>`, not",
        "  `Result<T, ()>` or `i32` status codes.  Sentinel return",
        "  values like `-1` map to `Err`, not `Ok(-1)`.",
        "",
        "## Other things to look for",
        "",
        "- **Safety holes** — raw pointers used without a clear",
        "  `// SAFETY:` story, `unsafe` blocks that have a safe",
        "  equivalent, ownership patterns that smell wrong.",
        "- **Type-shape consistency** — same C type translated",
        "  differently in different functions of the same module;",
        "  signed/unsigned that disagrees with caller arithmetic;",
        "  integer widths that drift from the C source.",
        "- **Idiom plumbing** — `&[T]` vs raw pointer + length,",
        "  `Option<&T>` for nullable, owned `String` vs borrowed",
        "  `&str`, function-pointer typedef → trait, doc-comment",
        "  placement.",
        "- **Lint allowances** — `#![allow(…)]` blocks are usually",
        "  a smell.  Fix the underlying code instead of suppressing.",
        "- **Documentation** — every public item needs a doc comment",
        "  explaining what the Rust function does.  A note like",
        "  `C: \\`name\\`` is fine for traceability but isn't a",
        "  substitute for explaining the Rust API.",
        "",
        "## How to work",
        "1. Read the materials above.",
        "2. Identify improvements you'd apply.",
        f"3. Apply them via `Edit` to `{rust_target}` (and sibling",
        "   modules under `rs/fq/src/picoquic/` if a coordinated",
        "   change is needed).",
        "4. Validate with **both** of these (Bash tool's cwd is the",
        "   repo root, so prefix with `cd rs/fq && `):",
        "     cd rs/fq && cargo check",
        "     cd rs/fq && cargo clippy -- -D warnings",
        "   Iterate until both pass cleanly.",
        "5. Report on stdout: a one-paragraph summary of what you",
        "   changed and why.",
        "",
        "If nothing needs improvement, say so on stdout and exit",
        "without editing — that is a valid outcome.",
        "",
        "## Constraints",
        "- You may edit any file under `rs/fq/src/`.  Renaming a",
        "  type or converting a free function to a method may force",
        "  callers in sibling modules to change — that's expected.",
        "- `lib.rs` is the kitchen-sink module (the crate's public",
        "  API); editing it for a coordinated rename or method",
        "  conversion is fine.  Do NOT touch `Cargo.toml` or",
        "  anything outside `rs/fq/`.",
        "- Don't create new files (`Write` is not in your allowlist).",
        "- Don't run `cargo test` or `cargo fmt` — the parent",
        "  script handles formatting after you finish.",
        "- This pass should not introduce `// REVIEW` comments;",
        "  those are the human reviewer's tool in Phase 1C.",
    ]
    return "\n".join(parts) + "\n"


def write_prompt_file(header: str) -> Path:
    out = prompt_path(header)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(compose_prompt(header))
    return out


# ---------------------------------------------------------------------------
# Agent invocation

def invoke_agent(header: str, prompt_file: Path,
                 max_turns: int,
                 agent: agent_runner.AgentConfig) -> tuple[int, str]:
    prompt = prompt_file.read_text()
    run = agent_runner.run_capture(
        agent,
        prompt,
        repo_root=REPO_ROOT,
        log_path=agent_log_path(agent, header),
        phase="phase1a",
        label=header,
        prompt_file=prompt_file,
        allowed_tools=ALLOWED_TOOLS,
        max_turns=max_turns,
    )
    return run.returncode, run.stdout


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
            agent: agent_runner.AgentConfig, state: dict) -> str:
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
        print("  (dry-run; skipping agent + gate)")
        return "skip"

    print(f"  {agent.label:<7} → invoking "
          f"(model={agent.model_label}, max-turns={max_turns}) …")
    before = rs_target.read_text()
    code, stdout = invoke_agent(header, prompt_file, max_turns, agent)
    if code != 0:
        print(f"    FAIL: {agent.label} exit {code} "
              f"(see {agent_log_path(agent, header).relative_to(REPO_ROOT)})")
        record(state, header, "fail", stage=agent.label, exit_code=code)
        return "fail"
    after = rs_target.read_text()
    changed = before != after

    if not changed and stdout_looks_like_noop(stdout):
        print(f"  noop    → {agent.label} reported no improvements needed")
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
                   help="Print plan; do not invoke agent or run the gate.")
    p.add_argument("--stop-on-failure", action="store_true",
                   help="Stop at the first failure (default is to continue).")
    p.add_argument("--force", action="store_true",
                   help="Re-review headers already marked ok or noop.")
    p.add_argument("--status", action="store_true",
                   help="Print progress and exit.")
    p.add_argument("--print-order", action="store_true",
                   help="Print headers in topological review order and exit.")
    p.add_argument("--max-turns", type=int, default=80,
                   help="Per-header turn limit for Claude; included as "
                        "guidance for Codex (default 80).")
    agent_runner.add_agent_args(
        p,
        claude_default_model="sonnet",
        model_help_context="Phase 1A agent",
    )
    args = p.parse_args()
    agent = agent_runner.config_from_args(args, claude_default_model="sonnet")

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
    print(f"  agent:   {agent.label} (model={agent.model_label})")
    print(f"  run log: {log_path.relative_to(REPO_ROOT)}")
    print(f"  state:   {PHASE1A_STATE.relative_to(REPO_ROOT)}")
    failed: list[str] = []
    succeeded: list[str] = []
    for h in targets:
        result = run_one(h, dry_run=args.dry_run,
                         max_turns=args.max_turns, agent=agent,
                         state=state)
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
