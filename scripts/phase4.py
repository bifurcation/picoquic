#!/usr/bin/env python3
"""phase4.py — AI-driven Phase 4 implementation translator.

For each Rust source file under `rs/fq/src/`, fill in `todo!()`
function bodies by translating the corresponding C body from
`picoquic/`.  The Phase 3 tests are the gate: a function is "done"
when the tests that exercise it stop panicking on `todo!()`.

Pipeline modelled on `scripts/phase3a.py`; the differences:

* Targets are Rust source files (not picoquictest files).
* Build gate is `cargo test` (not just `--no-run`) — the test
  panic count is the truth.
* Allowlist also lets the agent run `cargo test` and the
  Phase 4 completion checker.
* Default ordering is bottom-up by Rust file size (rough proxy
  for call-graph depth — small leaf modules first).

Usage:
  python3 scripts/phase4.py
  python3 scripts/phase4.py --src rs/fq/src/siphash.rs
  python3 scripts/phase4.py --batch 3
  python3 scripts/phase4.py --status
  python3 scripts/phase4.py --dry-run
  python3 scripts/phase4.py --max-turns 250 --model opus

State / logs:
  - xlate/phase4_state.json
  - xlate/phase4_runs/<timestamp>.log
  - xlate/claude_logs/phase4/<basename>.log
  - xlate/prompts/phase4/<basename>.md
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

REPO_ROOT = Path(__file__).resolve().parent.parent
RS_CRATE = REPO_ROOT / "rs" / "fq"
RS_SRC = RS_CRATE / "src"

PHASE4_STATE = REPO_ROOT / "xlate" / "phase4_state.json"
PROMPTS_DIR = REPO_ROOT / "xlate" / "prompts" / "phase4"
LOG_DIR = REPO_ROOT / "xlate" / "claude_logs" / "phase4"
RUNS_DIR = REPO_ROOT / "xlate" / "phase4_runs"

# Build gate now exercises tests, not just compile.  But cargo test
# can take a while; phase4 agents prefer to run individual tests
# rather than the full suite.  The parent script's gate runs the
# full `cargo test` once at the end of each batch.
ALLOWED_TOOLS = (
    "Read Edit Glob Grep "
    "Bash(cargo check:*) "
    "Bash(cargo test:*) "
    "Bash(cargo fmt:*) "
    "Bash(cargo clippy:*) "
    "Bash(python3 scripts/phase4_check.py:*) "
    "Bash(python3 scripts/phase3_check.py:*)"
)

# Files we don't try to translate this phase: pure-design modules
# with no C counterpart, and loglib stuff which is out of v1 scope.
# Note: tests/util.rs IS in scope — its todo!() bodies are
# translations of C test helpers from picoquictest_internal.h.
SKIP_FILES = {
    # Phase-3 dual-q simulator stub; not a picoquic translation target.
    "tests/dualq.rs",
    # No C counterpart (Phase 2 design modules).
    "arena.rs",
    "tls.rs",
    "crypto.rs",
    "sys/mod.rs",
}


INCOMPLETE_RE = re.compile(
    r"\btodo!\s*\(|\bunimplemented!\s*\(|//\s*SKIP\s*:",
)


def collect_targets(order: str = "size") -> list[Path]:
    """All `.rs` under `rs/fq/src/` that contain at least one
    incomplete body, ordered bottom-up.

    "Incomplete" means any of:
      * `todo!()`
      * `unimplemented!()`
      * `// SKIP:` markers (agent-stubbed bodies that compile but
        return placeholder values)

    `order`:
      * `size` (default) — smallest file first.  Crude proxy for
        leaf-first; works because leaf modules (siphash, splay,
        hash) are short and Connection / Quic core is huge.
      * `callgraph` — primary sort by max C call-graph height of
        any function whose name appears in `/// C: \\`name\\``
        doc comments inside the file; secondary by size.  Lower
        height = closer to leaves.  Unmatched files fall through
        to the size-based bucket at the end.
    """
    out: list[Path] = []
    for path in sorted(RS_SRC.rglob("*.rs")):
        rel = path.relative_to(RS_SRC).as_posix()
        if rel in SKIP_FILES:
            continue
        # Skip Phase 3 test-body files except tests/util.rs (which
        # contains test-helper bodies that need C-side translations).
        if rel.startswith("tests/") and rel != "tests/util.rs":
            continue
        if INCOMPLETE_RE.search(path.read_text()):
            out.append(path)

    if order == "callgraph":
        height = _load_callgraph_heights()
        c_ref_re = re.compile(
            r"//[/!]?\s*C:\s*`?(?P<name>[A-Za-z_][A-Za-z0-9_]*)`?",
        )
        # Sentinel for "no matches": pushed to end of bucket order.
        UNMATCHED = 10**6

        def keyfn(p: Path) -> tuple[int, int]:
            text = p.read_text()
            refs = set(c_ref_re.findall(text))
            heights = [height[n] for n in refs if n in height]
            primary = min(heights) if heights else UNMATCHED
            return primary, p.stat().st_size

        out.sort(key=keyfn)
    else:
        out.sort(key=lambda p: p.stat().st_size)
    return out


def _load_callgraph_heights() -> dict[str, int]:
    cg = REPO_ROOT / "xlate" / "call_graph.json"
    if not cg.is_file():
        return {}
    try:
        return json.loads(cg.read_text()).get("height_of", {})
    except (OSError, json.JSONDecodeError):
        return {}


def todo_count(path: Path) -> int:
    """Count incomplete bodies in `path`: todo!(), unimplemented!(),
    and // SKIP: markers.  Each is functionally equivalent — none
    is a real translation.
    """
    return sum(1 for _ in INCOMPLETE_RE.finditer(path.read_text()))


def prompt_file_path(path: Path) -> Path:
    rel = path.relative_to(RS_SRC).as_posix().replace("/", "__")
    return PROMPTS_DIR / f"{rel.removesuffix('.rs')}.md"


def claude_log_path(path: Path) -> Path:
    rel = path.relative_to(RS_SRC).as_posix().replace("/", "__")
    return LOG_DIR / f"{rel.removesuffix('.rs')}.log"


# ---------------------------------------------------------------------------
# State.

def load_state() -> dict:
    if PHASE4_STATE.is_file():
        try:
            return json.loads(PHASE4_STATE.read_text())
        except json.JSONDecodeError:
            return {}
    return {}


def save_state(state: dict) -> None:
    PHASE4_STATE.parent.mkdir(parents=True, exist_ok=True)
    PHASE4_STATE.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n")


def record(state: dict, key: str, status: str, **extra) -> None:
    state[key] = {
        "status": status,
        "at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        **extra,
    }
    save_state(state)


def is_done(key: str, state: dict) -> bool:
    return state.get(key, {}).get("status") == "ok"


# ---------------------------------------------------------------------------
# Prompt composition.

def compose_prompt(target: Path) -> str:
    rel = target.relative_to(REPO_ROOT)
    n_todos = todo_count(target)
    parts: list[str] = [
        f"# Phase 4 implementation translation: `{rel}`",
        "",
        "## NO SHORTCUTS — do the hard work",
        "",
        "Phase 4 translates the **whole library**.  Past sweeps",
        "produced hundreds of stubs that compile but don't translate",
        "the C body — this is failure, not completion.  The gate",
        "now flags all three forms of incompleteness:",
        "",
        "* `todo!()`",
        "* `unimplemented!()`  ← also forbidden",
        "* `// SKIP:` markers with stub returns (`None`,",
        "  `Err(Generic)`, `Ok(())`, fabricated default values) ← worst",
        "",
        "**If a body is hard, do the hard work**: translate the helpers",
        "it depends on first; add the missing struct fields the C body",
        "uses; port the static tables the C side has; extend Phase 1/2",
        "traits when a body needs it; use Rust ownership idioms",
        "(`&mut self`, `RefCell`, restructure callers) instead of",
        "`None` shortcuts.",
        "",
        "**Nothing is fundamentally untranslatable.**  Every function",
        "in picoquic has a body — translate it.  Phase 4 IS the",
        "translation: it covers the whole library, not 'the easy",
        "parts.'  There is no 'phase 5' or 'v2' that picks up later.",
        "",
        "Concrete translations of common 'hard' patterns:",
        "  * `pthread_create` → `std::thread::spawn`",
        "  * `pipe()` + `write(1)` wake-up → `std::sync::mpsc`",
        "    or `std::sync::Condvar`",
        "  * `select(2)` / `poll(2)` / `io_uring` → `mio` crate",
        "  * loglib helpers (binlog/qlog/textlog) → `log` crate +",
        "    standard file I/O",
        "  * borrow-checker issues → `&mut self`, `RefCell`, or",
        "    restructure the call site",
        "",
        "**Forbidden 'blocker' excuses** (every one is a shortcut):",
        "  * 'out of scope' / 'deferred to a later pass' / 'v2'",
        "  * 'multi-threading not in scope' (translate with",
        "    `std::thread`)",
        "  * 'loglib not in scope' (translate the helpers)",
        "  * 'sub-system not yet translated' (translate the sub-system",
        "    FIRST, then this function)",
        "  * 'needs more design thought'",
        "  * 'borrow-checker issue' / 'lifetime issue' (use the",
        "    correct ownership pattern; restructure if needed)",
        "",
        "The ONLY acceptable `todo!()` blocker is a concrete external",
        "dependency genuinely outside our reach, e.g. 'blocked: needs",
        "picotls API not yet exposed in the Rust binding'.",
        "",
        "No `unimplemented!()`.  No `// SKIP:` comments.  No fake",
        "stub returns.  See `xlate/impl_translation_guide.md` section",
        "'Anti-patterns' and 'Nothing is fundamentally untranslatable'.",
        "",
        "Translate function bodies in this Rust source file by",
        "mirroring the C bodies in `picoquic/`.  The Phase 3 test",
        "suite is the gate: a function is done when tests that",
        "exercise it stop panicking.",
        "",
        f"This file currently has **{n_todos}** incomplete bodies to fill",
        "(counted across `todo!()` + `unimplemented!()` + `// SKIP:`).",
        "",
        "## What this is",
        "",
        "Phase 1 / 2 designed the Rust API surface.  Phase 3 translated",
        "the C tests against that surface (they currently panic on",
        "`todo!()`).  **Phase 4 fills the bodies** so those tests",
        "pass.  The C source is the spec; produce idiomatic Rust",
        "that satisfies the same invariants.",
        "",
        "## Required reading (in order)",
        "",
        "1. **`xlate/impl_translation_guide.md`** — Rust idioms for",
        "   common C body patterns (memory, control flow, errors,",
        "   strings, time, network, logging, crypto).  Read this once.",
        f"2. The Rust target: `{rel}` — every `todo!()` body in here",
        "   is your work.",
        "3. The matching C source.  Most modules under `rs/fq/src/`",
        "   correspond to one or more files under `picoquic/`; the",
        "   guide names the mapping.  Use `Glob` / `Grep` against",
        "   `picoquic/*.c` to find the C bodies for each function.",
        "4. The tests that exercise this module: `rs/fq/src/tests/`.",
        "",
        "## Translation rules",
        "",
        "- **Faithful, idiomatic.**  Walk the C body and produce",
        "  the equivalent Rust.  Don't reshape the algorithm.",
        "- **Don't change signatures.**  The Phase 1 / 2 / 3 design",
        "  is settled.  If a body genuinely cannot be expressed under",
        "  the existing signature, surface it on stdout and skip the",
        "  function — that is a human design call, not yours.",
        "- **No `unsafe`.**  No edits outside `rs/fq/src/`.",
        "- **Idiomatic Rust over C-mirroring.**  See the guide for",
        "  the canonical mappings (linked list -> Vec/VecDeque,",
        "  malloc -> Box, void* -> Box<dyn Trait>, int 0/-1 -> Result).",
        "- **Don't introduce new public items.**  Helpers are fine",
        "  but should be private (`fn` without `pub`).",
        "- **DO NOT stub bodies — see the NO SHORTCUTS section",
        "  above.**  No `unimplemented!()`, no `// SKIP:` comments",
        "  with placeholder returns, no `Err(Generic)` shortcuts,",
        "  no `Ok(())` no-ops for functions with real side effects,",
        "  no fabricated defaults.  Do the hard work.  If a body",
        "  truly can't be translated this pass, leave a single bare",
        "  `todo!()` with a one-line blocker note above it.",
        "- **Tests are the gate.**  After each meaningful chunk, run",
        f"     cd rs/fq && cargo test --no-run",
        f"     python3 scripts/phase4_check.py {rel}",
        "  to confirm the file still compiles and how many `todo!()`s",
        "  remain.  Iterate until zero.",
        "",
        "## Process",
        "",
        "1. Read the impl translation guide (once).",
        f"2. Read the Rust target `{rel}` end-to-end.",
        "3. Identify the corresponding C source(s) under `picoquic/`.",
        "   The first line of each `todo!()` function's doc comment",
        "   typically names the C function (`/// C: \\`picoquic_xxx\\``).",
        "4. For each `todo!()`:",
        "   a. Find the C body.",
        "   b. Translate idiomatically per the guide.",
        "   c. Run `cargo test --no-run` to confirm it still compiles.",
        "5. After every `todo!()` in this file is replaced, run:",
        "     cd rs/fq && cargo fmt",
        "     cd rs/fq && cargo test --no-run",
        "     cd rs/fq && cargo clippy --tests --all-features -- -D warnings",
        f"     python3 scripts/phase4_check.py {rel}",
        "   All four must pass.",
        "6. Optionally run `cargo test` (full) and report which",
        "   previously-panicking tests now pass.",
        "7. Report on stdout: a one-paragraph summary.",
        "",
        "## Stop spinning",
        "",
        "If 5 grep/read tool calls into the same file haven't found",
        "what you need, **stop searching**.  Open the C source and",
        "the Rust target side by side; the C function name is in",
        "the doc comment.",
    ]
    return "\n".join(parts) + "\n"


def write_prompt(target: Path) -> Path:
    out = prompt_file_path(target)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(compose_prompt(target))
    return out


# ---------------------------------------------------------------------------
# Batch prompt.

def compose_batch_prompt(batch: list[Path]) -> str:
    """Compose a single-claude-invocation prompt that translates
    multiple Rust source files in one go.  Amortises the guide read,
    `cargo` boots, and any cross-module type lookups across the
    whole batch.
    """
    sources_section: list[str] = []
    total_todos = 0
    for target in batch:
        rel = target.relative_to(REPO_ROOT)
        n = todo_count(target)
        total_todos += n
        sources_section.append(
            f"### `{rel}` ({n} todo!() bod{'y' if n == 1 else 'ies'})\n"
        )
    rel_paths = [str(t.relative_to(REPO_ROOT)) for t in batch]

    parts: list[str] = [
        f"# Phase 4 batch translation: {len(batch)} source files",
        "",
        "## NO SHORTCUTS — do the hard work",
        "",
        "Phase 4 translates the **whole library**.  Past sweeps",
        "produced hundreds of stubs that compile but don't translate",
        "the C body — this is failure, not completion.  The gate",
        "flags all three forms of incompleteness identically:",
        "",
        "* `todo!()`",
        "* `unimplemented!()` ← forbidden",
        "* `// SKIP:` markers with stub returns (`None`,",
        "  `Err(Generic)`, `Ok(())`, fabricated defaults) ← forbidden",
        "",
        "If a body is hard: translate the helpers it needs first;",
        "add missing struct fields; port C static tables; extend",
        "the Phase 1/2 traits as needed; use Rust ownership idioms",
        "instead of `None` shortcuts.",
        "",
        "**Nothing is fundamentally untranslatable.**  Every function",
        "in picoquic has a body — translate it.  Phase 4 IS the",
        "translation: it covers the whole library, not 'the easy",
        "parts.'  Concrete mappings:",
        "  * `pthread_create` → `std::thread::spawn`",
        "  * `pipe()` wake-up → `std::sync::mpsc` or `Condvar`",
        "  * `select`/`poll`/`io_uring` → `mio` crate",
        "  * loglib helpers → `log` crate + standard file I/O",
        "",
        "**Forbidden 'blocker' excuses** (every one is a shortcut):",
        "'out of scope', 'deferred to a later pass', 'multi-threading",
        "not in scope', 'loglib not in scope', 'sub-system not yet",
        "translated', 'needs design thought', 'borrow-checker issue'.",
        "",
        "The ONLY acceptable `todo!()` blocker is a concrete external",
        "dependency genuinely outside our reach (e.g. 'needs picotls",
        "API not yet exposed in the Rust binding').",
        "",
        "Fill in incomplete function bodies in the Rust files listed",
        "below by translating the matching C bodies in `picoquic/`.",
        "This is a **batch** invocation — handle every file in one",
        "session, amortising the read of the translation guide and",
        "any cross-module API lookups.",
        "",
        f"Total incomplete bodies in this batch: **{total_todos}** "
        "(counted across `todo!()` + `unimplemented!()` + `// SKIP:`).",
        "",
        "## What this is",
        "",
        "Phase 1 / 2 designed the Rust API surface.  Phase 3",
        "translated the C tests against it (currently panic on",
        "`todo!()`).  **Phase 4 fills the bodies** so those tests",
        "pass.  The C source is the spec.",
        "",
        "## Required reading (once for the whole batch)",
        "",
        "1. **`xlate/impl_translation_guide.md`** — Rust idioms for",
        "   common C body patterns (memory, control flow, errors,",
        "   strings, time, network, logging, crypto).  Read once.",
        "2. The shared module map at the bottom of that guide tells",
        "   you which C source(s) match each Rust file in this batch.",
        "3. Skim `rs/fq/src/lib.rs` re-exports + `rs/fq/src/internal.rs`",
        "   only as needed; the guide should cover most signatures.",
        "",
        "Do **not** re-read the guide once per file — read it once",
        "and use what you remember across the whole batch.",
        "",
        "## Sources to translate (this batch)",
        "",
        *sources_section,
        "## Translation rules",
        "",
        "- **Faithful, idiomatic.**  Walk the C body and produce the",
        "  equivalent Rust.  Don't reshape the algorithm.",
        "- **Don't change signatures.**  Phase 1 / 2 / 3 settled them.",
        "  If a body genuinely cannot be expressed under the existing",
        "  signature, surface it on stdout and skip the function.",
        "- **No `unsafe`.**  No edits outside `rs/fq/src/` (except the",
        "  Phase 3 test suite, which you should not touch).",
        "- **Idiomatic Rust over C-mirroring.**  See the guide.",
        "- **No new public items.**  Helpers are fine but private.",
        "- **Tests are the gate.**  Run `cargo test --no-run` after",
        "  each meaningful chunk to verify the file still compiles.",
        "",
        "## Stop spinning",
        "",
        "If 5 grep/read calls into the same file haven't found what",
        "you need, **stop searching**.  Open the C source and the",
        "Rust target side by side; the C function name is in the",
        "doc comment.  Aim to start writing edits within the first",
        "10 tool calls of EACH file.",
        "",
        "## Verification",
        "",
        "After translating each file, run:",
        "    python3 scripts/phase4_check.py <rs file>",
        "to confirm 0 `todo!()` bodies remain.  If any do, fix them",
        "before moving on.",
        "",
        "After the whole batch, run:",
        f"    python3 scripts/phase4_check.py {' '.join(rel_paths)}",
        "    cd rs/fq && cargo fmt",
        "    cd rs/fq && cargo test --no-run",
        "    cd rs/fq && cargo clippy --tests --all-features -- -D warnings",
        "All four must succeed.  Iterate until they do.",
        "",
        "## Process",
        "",
        "1. Read the impl translation guide once.",
        "2. For each file in this batch:",
        "   a. Read the Rust target end-to-end.",
        "   b. Identify matching C source(s) (doc comments name them).",
        "   c. For each `todo!()` body: find the C body, translate.",
        "   d. Run `python3 scripts/phase4_check.py <file>`; iterate.",
        "3. Run the gate (fmt + cargo test --no-run + clippy) once at",
        "   the end of the batch.",
        "4. Report on stdout: a one-paragraph summary per file.",
    ]
    return "\n".join(parts) + "\n"


def write_batch_prompt(batch: list[Path]) -> Path:
    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
    keys = [t.relative_to(RS_SRC).as_posix().replace("/", "__")
            .removesuffix(".rs") for t in batch]
    name = "batch_" + "_".join(keys)[:120] + ".md"
    out = PROMPTS_DIR / name
    out.write_text(compose_batch_prompt(batch))
    return out


# ---------------------------------------------------------------------------
# Claude invocation.

def invoke_claude(target: Path | str, prompt_file: Path,
                  max_turns: int, model: str,
                  log_path_override: Path | None = None,
                  ) -> tuple[int, str]:
    if shutil.which("claude") is None:
        return 127, "claude CLI not found on PATH"
    log = log_path_override or claude_log_path(target)  # type: ignore[arg-type]
    log.parent.mkdir(parents=True, exist_ok=True)
    label = (target.relative_to(REPO_ROOT) if isinstance(target, Path)
             else target)
    cmd = [
        "claude", "-p", prompt_file.read_text(),
        "--model", model,
        "--allowedTools", ALLOWED_TOOLS,
        "--max-turns", str(max_turns),
        "--output-format", "stream-json",
        "--verbose",
    ]
    t0 = time.monotonic()
    log_f = open(log, "w", buffering=1)
    log_f.write(
        f"# claude -p (phase4) for {label}\n"
        f"# started: {time.strftime('%Y-%m-%dT%H:%M:%S')}\n"
        f"# command: {' '.join(shlex.quote(c) for c in cmd[:1] + cmd[3:])}\n"
        f"# (prompt: {prompt_file.relative_to(REPO_ROOT)})\n\n"
    )
    final_summary = ""
    last_assistant_text = ""
    rate_limited = False
    proc = subprocess.Popen(
        cmd, cwd=REPO_ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        stdin=subprocess.DEVNULL, bufsize=1,
    )
    assert proc.stdout is not None
    for line in proc.stdout:
        log_f.write(line)
        line = line.rstrip()
        if not line:
            continue
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        if ev.get("type") == "assistant":
            for block in ev.get("message", {}).get("content", []):
                if block.get("type") == "tool_use":
                    name = block.get("name", "?")
                    inp = block.get("input", {})
                    label = ""
                    if name in {"Read", "Edit", "Write"}:
                        label = inp.get("file_path", "")
                    elif name == "Bash":
                        label = inp.get("command", "")[:80]
                    elif name in {"Glob", "Grep"}:
                        label = inp.get("pattern", inp.get("query", ""))
                    print(f"  · {name}({label})", flush=True)
                elif block.get("type") == "text":
                    last_assistant_text = block.get("text", "")
        elif ev.get("type") == "result":
            final_summary = ev.get("result", "") or ""
            if ev.get("api_error_status") == 429 and ev.get("is_error"):
                rate_limited = True
    rc = proc.wait()
    elapsed = time.monotonic() - t0
    log_f.write(f"\n# elapsed: {elapsed:.1f}s, exit: {rc}\n")
    log_f.close()
    if rate_limited:
        rc = 99
    return rc, final_summary or last_assistant_text


# ---------------------------------------------------------------------------
# Build gate.

def run_gate() -> int:
    steps = [
        ["cargo", "fmt"],
        ["cargo", "test", "--no-run"],
        ["cargo", "clippy", "--tests", "--all-features", "--", "-D", "warnings"],
    ]
    for step in steps:
        sys.stdout.write(f"  $ (cd rs/fq && {' '.join(step)})\n")
        sys.stdout.flush()
        r = subprocess.run(step, cwd=RS_CRATE)
        if r.returncode != 0:
            return r.returncode
    return 0


def todos_remaining(target: Path) -> int:
    return todo_count(target)


# ---------------------------------------------------------------------------
# Per-target runner.

def run_one(target: Path, *, dry_run: bool, max_turns: int,
            model: str, state: dict) -> str:
    rel_key = target.relative_to(RS_SRC).as_posix()
    print(f"\n=== {rel_key} ({todo_count(target)} todos) ===")
    prompt_file = write_prompt(target)
    print(f"  prompt  → {prompt_file.relative_to(REPO_ROOT)}")
    if dry_run:
        print("  (dry-run; skipping claude + gate)")
        return "skip"

    print(f"  claude  → invoking ({model}, max-turns={max_turns}) …")
    code, stdout = invoke_claude(target, prompt_file, max_turns, model)
    if code == 99:
        print(f"    RATE-LIMITED: {stdout.strip()[:200]}")
        return "rate-limited"
    if code != 0:
        log_rel = claude_log_path(target).relative_to(REPO_ROOT)
        print(f"    FAIL: claude exit {code} (see {log_rel})")
        record(state, rel_key, "fail", stage="claude", exit_code=code)
        return "fail"

    # Check whether todo!()s remain.
    remaining = todos_remaining(target)
    if remaining > 0:
        print(f"    PARTIAL: {remaining} todo!()s remain")
        record(state, rel_key, "partial", remaining=remaining)
        return "partial"

    print("  gate    → cargo fmt + cargo test --no-run + cargo clippy")
    rc = run_gate()
    if rc != 0:
        print(f"    FAIL: gate exit {rc}")
        record(state, rel_key, "fail", stage="gate", exit_code=rc)
        return "fail"
    print("  ok")
    record(state, rel_key, "ok", changed=True)
    return "ok"


def run_batch(batch: list[Path], *, dry_run: bool, max_turns: int,
              model: str, state: dict) -> tuple[list[str], list[str], bool]:
    """Run one batched claude invocation over `batch` Rust files.

    Returns (succeeded, failed, rate_limited).  `succeeded` and
    `failed` lists are keyed by relative path (rs/fq/src/X.rs).
    `rate_limited` is True if the run aborted on HTTP 429 — the
    caller should stop the sweep cleanly without polluting state.
    """
    rel_paths = [t.relative_to(RS_SRC).as_posix() for t in batch]
    print(f"\n=== batch ({len(batch)} src): {', '.join(rel_paths)} ===")
    prompt_file = write_batch_prompt(batch)
    print(f"  prompt  → {prompt_file.relative_to(REPO_ROOT)}")
    if dry_run:
        print("  (dry-run; skipping claude + gate)")
        return [], [], False

    keys = [t.relative_to(RS_SRC).as_posix().replace("/", "__")
            .removesuffix(".rs") for t in batch]
    batch_log = LOG_DIR / f"batch_{'_'.join(keys)[:120]}.log"
    print(f"  claude  → invoking ({model}, max-turns={max_turns}) …")
    code, stdout = invoke_claude(
        f"batch_{','.join(rel_paths)}", prompt_file, max_turns, model,
        log_path_override=batch_log,
    )
    if code == 99:
        print(f"    RATE-LIMITED: {stdout.strip()[:200]}")
        return [], [], True
    if code != 0:
        print(f"    WARNING: claude exit {code} (see {batch_log})")
        # Don't bail yet — let the per-file checker decide.

    # Per-file completion check via file-by-file todo!() count.
    print("  check   → counting todo!() in each batch source")
    succeeded: list[str] = []
    failed: list[str] = []
    for target, rel in zip(batch, rel_paths):
        n = todo_count(target)
        if n == 0:
            print(f"    OK   {rel}: 0 todo!() left")
            succeeded.append(rel)
        else:
            print(f"    FAIL {rel}: {n} todo!() remain")
            failed.append(rel)

    # Run the build gate; failure penalises the whole batch.
    print("  gate    → cargo fmt + cargo test --no-run + cargo clippy")
    if run_gate() != 0:
        print("    FAIL: gate failed; treating any 'ok' as 'fail'")
        for rel in list(succeeded):
            succeeded.remove(rel)
            if rel not in failed:
                failed.append(rel)

    for rel in succeeded:
        record(state, rel, "ok", changed=True, batch=True)
    for rel in failed:
        n = todo_count(RS_SRC / rel)
        if n > 0:
            record(state, rel, "partial", remaining=n, batch=True)
        else:
            record(state, rel, "fail", stage="gate", batch=True)
    return succeeded, failed, False


# ---------------------------------------------------------------------------
# Status / driver glue.

def cmd_status(targets: list[Path]) -> None:
    state = load_state()
    print(f"Phase 4 progress: {len(targets)} source file(s) "
          f"with todo!() bodies")
    ok = [t for t in targets if is_done(t.relative_to(RS_SRC).as_posix(), state)]
    partial = [t for t in targets
               if state.get(t.relative_to(RS_SRC).as_posix(), {}).get("status") == "partial"]
    failed = [t for t in targets
              if state.get(t.relative_to(RS_SRC).as_posix(), {}).get("status") == "fail"]
    pending = [t for t in targets if t not in ok and t not in partial and t not in failed]
    print(f"  ok:      {len(ok)}")
    print(f"  partial: {len(partial)}")
    print(f"  failed:  {len(failed)}")
    print(f"  pending: {len(pending)}")
    total_todos = sum(todo_count(t) for t in targets)
    done_todos = sum(todo_count(t) for t in ok)  # ok files have 0 todos
    pending_todos = total_todos - done_todos
    print(f"  todo!() count: {pending_todos} remaining "
          f"(of {total_todos} initial)")
    if partial:
        print("\nPartial:")
        for t in partial:
            rel = t.relative_to(RS_SRC).as_posix()
            n = state[rel].get("remaining", "?")
            print(f"  - {rel}  ({n} todo!() remaining)")


# ---------------------------------------------------------------------------
# Run-log tee.

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
# Main.

def main() -> int:
    p = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    p.add_argument("--src",
                   help="Translate one Rust source file (path relative "
                        "to repo root or rs/fq/src/).")
    p.add_argument("--limit", type=int, default=None,
                   help="Stop after N source files.")
    p.add_argument("--dry-run", action="store_true",
                   help="Print plan; do not invoke claude or run the gate.")
    p.add_argument("--stop-on-failure", action="store_true",
                   help="Stop at the first failure (default is continue).")
    p.add_argument("--force", action="store_true",
                   help="Re-translate sources already marked ok.")
    p.add_argument("--status", action="store_true",
                   help="Print progress and exit.")
    p.add_argument("--max-turns", type=int, default=200,
                   help="Per-source turn limit for claude (default 200).")
    p.add_argument("--model", default="sonnet",
                   help="Model passed to `claude -p --model` "
                        "(default: sonnet).")
    p.add_argument("--batch", type=int, default=1,
                   help="Pack N sources per claude invocation "
                        "(default: 1, i.e. per-source).  Larger values "
                        "amortise context-warmup but require higher "
                        "max-turns and may hit context-window limits.")
    p.add_argument("--order", choices=("size", "callgraph"),
                   default="size",
                   help="Target ordering: 'size' (default, smallest "
                        "first) or 'callgraph' (min C call-graph "
                        "height first; unmatched files fall back to "
                        "size).")
    args = p.parse_args()

    targets = collect_targets(order=args.order)

    if args.status:
        cmd_status(targets)
        return 0

    state = load_state()
    if args.src:
        wanted = Path(args.src)
        if not wanted.is_absolute():
            # Accept rs/fq/src-relative or repo-relative paths.
            if wanted.exists():
                wanted = wanted.resolve()
            else:
                wanted = (RS_SRC / args.src).resolve()
        targets = [t for t in targets if t.resolve() == wanted]
        if not targets:
            print(f"unknown source: {args.src}", file=sys.stderr)
            return 1

    if not args.force:
        targets = [t for t in targets
                   if not is_done(t.relative_to(RS_SRC).as_posix(), state)]

    if args.limit is not None:
        targets = targets[: args.limit]

    if not targets:
        print("nothing to do (use --force to redo finished sources)")
        return 0

    log_path = setup_run_log()
    t_run_start = time.monotonic()
    print(f"Phase 4: {len(targets)} target(s) to translate "
          f"(batch={args.batch})")
    print(f"  run log: {log_path.relative_to(REPO_ROOT)}")
    print(f"  state:   {PHASE4_STATE.relative_to(REPO_ROOT)}")
    failed: list[str] = []
    succeeded: list[str] = []
    rate_limited_at: str | None = None
    if args.batch > 1:
        idx = 0
        while idx < len(targets):
            batch = targets[idx:idx + args.batch]
            ok, bad, rl = run_batch(
                batch, dry_run=args.dry_run,
                max_turns=args.max_turns, model=args.model, state=state,
            )
            succeeded.extend(ok)
            failed.extend(bad)
            if rl:
                rel_keys = [t.relative_to(RS_SRC).as_posix() for t in batch]
                rate_limited_at = ",".join(rel_keys)
                print(f"\nABORTING SWEEP: rate-limited mid-batch.  "
                      f"Re-run later.")
                break
            if bad and args.stop_on_failure:
                print(f"\nStopped after batch with failures "
                      f"(--stop-on-failure).")
                break
            idx += args.batch
    else:
        for target in targets:
            result = run_one(target, dry_run=args.dry_run,
                             max_turns=args.max_turns, model=args.model,
                             state=state)
            rel_key = target.relative_to(RS_SRC).as_posix()
            if result == "rate-limited":
                rate_limited_at = rel_key
                print(f"\nABORTING SWEEP: rate-limited at {rel_key}.  "
                      f"Re-run later.")
                break
            if result == "fail":
                failed.append(rel_key)
                if args.stop_on_failure:
                    print(f"\nStopped at {rel_key} (--stop-on-failure).")
                    break
            elif result == "ok":
                succeeded.append(rel_key)

    elapsed = time.monotonic() - t_run_start
    print(f"\n=== Phase 4 run summary ===")
    print(f"  elapsed:   {elapsed:.1f}s")
    print(f"  succeeded: {len(succeeded)}")
    print(f"  failed:    {len(failed)}")
    for s in failed:
        print(f"    - {s}")
    if rate_limited_at:
        print(f"  rate-limited at: {rate_limited_at}")
    print(f"  log:       {log_path.relative_to(REPO_ROOT)}")
    if rate_limited_at:
        return 2
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
