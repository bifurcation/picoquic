#!/usr/bin/env python3
"""phase2a.py — single-shot AI pass that drafts the Phase 2 plan.

Per `TRANSLATE_PLAN.md` Phase 2A:

* Read `TRANSLATE_PLAN.md` and `CLAUDE.md`.
* Walk `rs/fq/src/` for places the current translation touches
  an external dependency.
* Cross-reference the seed inventory in the plan.
* Write `xlate/phase2_plan.md` with one section per capability:
  boundary description, evidence, provider trait sketch,
  callback trait sketch, default implementation plan, open
  questions.
* Conclude with a Cargo feature layout and an implementation
  order for Phase 2C.

The script invokes `claude -p` with `Read Glob Grep Write` —
no source changes, no cargo gates.  Re-running overwrites the
plan; humans add `// REVIEW:` markers via Phase 2B
(`scripts/phase2b.py`) to revise it.

Usage:
  python3 scripts/phase2a.py               # drive the pass
  python3 scripts/phase2a.py --dry-run     # print prompt only
  python3 scripts/phase2a.py --model opus  # override model

Logs at `xlate/claude_logs/phase2a.log`; commands logged to
`COMMANDS.log`.
"""

from __future__ import annotations

import argparse
import shlex
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PLAN_PATH = REPO_ROOT / "xlate" / "phase2_plan.md"
LOG_PATH = REPO_ROOT / "xlate" / "claude_logs" / "phase2a.log"
COMMANDS_LOG = REPO_ROOT / "COMMANDS.log"

# Read-only tools plus `Write` for the plan document itself.
ALLOWED_TOOLS = "Read Glob Grep Write"


PROMPT = """\
You are drafting Phase 2A of the picoquic c2rust translation
(the dependency-abstraction plan).

Read these documents first, then walk the source:

* `TRANSLATE_PLAN.md` — the master plan.  Phase 2 is at
  `## Phase 2 — Dependency abstraction`; sub-section
  `## Phase 2A — Draft the abstraction plan` enumerates
  exactly what this pass must produce.
* `CLAUDE.md` — branch conventions.
* `rs/fq/src/` — the Phase-1 stub crate.  Pay attention to
  raw `*mut c_void` fields, `unsafe fn` wrappers, and the
  partial trait scaffolding in `tls_api.rs` and
  `crypto_provider_api.rs`.

Your single output is `xlate/phase2_plan.md`.  Overwrite it.
The structure is fixed by the Phase 2A spec in
`TRANSLATE_PLAN.md`:

1. Capability inventory table.
2. One section per capability (TLS state machine, AEAD/PN/hash
   crypto, random source, clock, sockets, logging output).
   Each section: boundary description, evidence (file:line
   citations), provider trait sketch, callback trait sketch
   (if applicable), default implementation plan, open
   questions.
3. Cargo feature layout — proposed `[features]` table plus
   the feature combinations Phase 2C must `cargo check`.
4. Implementation order — the order Phase 2C should attack
   capabilities, with a one-line rationale per capability.
5. A consolidated open-questions list at the end so the
   human reviewer in 2B can address them.

Be specific.  Cite file paths and line numbers.  Don't
hand-wave: every "we'll wrap this" needs to name what's
being wrapped.

Don't change any source under `rs/fq/`.  This is a planning
pass.
"""


def log_command(cmd: str) -> None:
    COMMANDS_LOG.parent.mkdir(parents=True, exist_ok=True)
    with COMMANDS_LOG.open("a") as f:
        f.write(f"# phase2a.py: invoke claude\n{cmd}\n")


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--model", default="opus", help="Claude model")
    p.add_argument("--max-turns", type=int, default=30,
                   help="`claude -p --max-turns` (default: 30)")
    p.add_argument("--dry-run", action="store_true",
                   help="Print prompt + command and exit")
    args = p.parse_args()

    LOG_PATH.parent.mkdir(parents=True, exist_ok=True)

    cmd = [
        "claude", "-p",
        "--model", args.model,
        "--allowedTools", ALLOWED_TOOLS,
        "--max-turns", str(args.max_turns),
        PROMPT,
    ]

    if args.dry_run:
        print("would run:", " ".join(shlex.quote(c) for c in cmd))
        print("---prompt---")
        print(PROMPT)
        return 0

    log_command(" ".join(shlex.quote(c) for c in cmd))

    start = time.monotonic()
    print(f"phase2a: invoking claude (model={args.model}) …")
    with LOG_PATH.open("w") as logf:
        logf.write(f"# claude -p (phase2a)\n# model={args.model} max-turns={args.max_turns}\n\n")
        logf.flush()
        result = subprocess.run(cmd, stdout=logf, stderr=subprocess.STDOUT)
    elapsed = time.monotonic() - start

    print(f"phase2a: claude exited {result.returncode} after {elapsed:.0f}s")
    print(f"phase2a: log -> {LOG_PATH.relative_to(REPO_ROOT)}")
    if PLAN_PATH.exists():
        size = PLAN_PATH.stat().st_size
        print(f"phase2a: plan -> {PLAN_PATH.relative_to(REPO_ROOT)} ({size} bytes)")
    else:
        print(f"phase2a: warning: plan not produced at {PLAN_PATH}", file=sys.stderr)
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
