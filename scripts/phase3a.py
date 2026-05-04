#!/usr/bin/env python3
"""phase3a.py — Phase 3A: AI-driven test body translation.

For each `picoquictest/*.c` source file, replace the auto-stubbed
`todo!("<entry_fn>")` bodies in `rs/fq/src/tests/<name>.rs` with
real translations of the C test bodies.

This is **test-driven development**: the tests don't have to pass
yet.  They have to be *expressed* against the Rust API as designed,
so Phase 4 (the implementation pass) has a concrete spec to satisfy.
A test that panics on the first `todo!()` call inside the API it
exercises is a clean fail and meets the gate.

Per source file:
  1. Compose a prompt that:
       * names the C source, the Rust test file, and the
         entry-fn list extracted from `picoquic_t/picoquic_t.c`;
       * points at the relevant Rust API surface (`lib.rs`,
         `internal.rs`, `tls.rs`, `tests/util.rs`, etc.);
       * spells out the translation contract (TDD; panic-on-todo
         is fine; do not skip with placeholder todo!s).
  2. Invoke `claude -p` with Read / Edit / Write / Bash(cargo
     check / cargo fmt) tools.
  3. After Claude returns, run `cargo test --no-run` as the gate.

State / logs:
  - xlate/phase3a_state.json
  - xlate/phase3a_runs/<timestamp>.log
  - xlate/claude_logs/phase3a/<src>.log
  - xlate/prompts/phase3a/<src>.md

Usage:
  python3 scripts/phase3a.py
  python3 scripts/phase3a.py --src picoquictest/sacktest.c
  python3 scripts/phase3a.py --limit 1
  python3 scripts/phase3a.py --dry-run
  python3 scripts/phase3a.py --status
  python3 scripts/phase3a.py --force
  python3 scripts/phase3a.py --stop-on-failure
  python3 scripts/phase3a.py --max-turns 200 --model opus
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
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DRIVER = REPO_ROOT / "picoquic_t" / "picoquic_t.c"
PICOQUICTEST = REPO_ROOT / "picoquictest"
RS_CRATE = REPO_ROOT / "rs" / "fq"
RS_TESTS = RS_CRATE / "src" / "tests"

PHASE3A_STATE = REPO_ROOT / "xlate" / "phase3a_state.json"
PROMPTS_DIR = REPO_ROOT / "xlate" / "prompts" / "phase3a"
LOG_DIR = REPO_ROOT / "xlate" / "claude_logs" / "phase3a"
RUNS_DIR = REPO_ROOT / "xlate" / "phase3a_runs"

# Tool allowlist for claude -p.  Includes Write because the agent
# may overwrite the entire stub file.  cargo test --no-run is the
# build gate; cargo fmt cleans up after.
ALLOWED_TOOLS = (
    "Read Edit Write Glob Grep "
    "Bash(cargo check:*) "
    "Bash(cargo test:*) "
    "Bash(cargo fmt:*) "
    "Bash(cargo clippy:*)"
)

# Filename mapping for picoquictest/<src>.c → rs/fq/src/tests/<rust>.rs.
# Keep in sync with `scripts/phase3.py`'s `RUST_FILENAME_OVERRIDES` /
# stripping rule: drop `_test` / `_tests` suffix unless that creates
# a collision with existing test-infrastructure modules.
RUST_FILENAME_OVERRIDES = {
    "util_test": "util_test",
}


def rust_filename_for(src_basename: str) -> str:
    if src_basename in RUST_FILENAME_OVERRIDES:
        return RUST_FILENAME_OVERRIDES[src_basename]
    name = src_basename
    for suffix in ("_tests", "_test"):
        if name.endswith(suffix):
            name = name[: -len(suffix)]
            break
    return name


# ---------------------------------------------------------------------------
# Test-table parsing (mirrors scripts/phase3.py).

def parse_test_table() -> list[tuple[str, str]]:
    text = DRIVER.read_text()
    start = re.search(r"static const \S+ test_table\[\]\s*=\s*\{", text)
    if not start:
        raise SystemExit("error: test_table[] not found in picoquic_t.c")
    row_pat = re.compile(
        r"\{\s*\"(?P<name>[^\"]+)\"\s*,\s*(?P<fn>[A-Za-z_][A-Za-z0-9_]*)\s*\}",
    )
    rows: list[tuple[str, str]] = []
    pos = start.end()
    while True:
        end = text.find("};", pos)
        m = row_pat.search(text, pos, end if end > 0 else None)
        if not m:
            break
        rows.append((m.group("name"), m.group("fn")))
        pos = m.end()
    return rows


def index_entry_fns() -> dict[str, str]:
    """Return entry_fn → C source basename (no .c)."""
    fn_to_src: dict[str, str] = {}
    pat = re.compile(
        r"^[A-Za-z_][A-Za-z0-9_ \t\*]*\b([A-Za-z_][A-Za-z0-9_]*)\s*\([^;]*?\)\s*\{",
        re.MULTILINE | re.DOTALL,
    )
    sources = sorted(PICOQUICTEST.glob("*.c")) + sorted(PICOQUICTEST.glob("*.cpp"))
    for src in sources:
        text = src.read_text(errors="replace")
        for m in pat.finditer(text):
            fn = m.group(1)
            if fn in fn_to_src or fn in {
                "if", "while", "for", "switch", "return", "do", "else",
                "case", "default", "break", "continue", "goto", "sizeof",
            }:
                continue
            fn_to_src[fn] = src.stem
    # Hard-coded sentinels (parallel to phase3.py).
    fn_to_src.setdefault("cplusplustest", "cplusplus")
    fn_to_src.setdefault("sim_link_test", "<harness>")
    return fn_to_src


def group_by_source() -> list[tuple[str, list[tuple[str, str]]]]:
    """[(c_src_basename, [(test_name, entry_fn), ...]), ...]."""
    rows = parse_test_table()
    fn_to_src = index_entry_fns()
    by_src: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for name, fn in rows:
        src = fn_to_src.get(fn, "<unknown>")
        by_src[src].append((name, fn))
    return sorted(by_src.items())


# ---------------------------------------------------------------------------
# Path helpers.

def c_source_path(src_basename: str) -> Path | None:
    """Return picoquictest/<src>.c if it exists; None for sentinels."""
    if src_basename.startswith("<"):
        return None
    p = PICOQUICTEST / f"{src_basename}.c"
    if p.is_file():
        return p
    p = PICOQUICTEST / f"{src_basename}.cpp"
    return p if p.is_file() else None


def rust_test_path(src_basename: str) -> Path:
    return RS_TESTS / f"{rust_filename_for(src_basename)}.rs"


def prompt_file_path(src_basename: str) -> Path:
    return PROMPTS_DIR / f"{src_basename}.md"


def claude_log_path(src_basename: str) -> Path:
    return LOG_DIR / f"{src_basename}.log"


# ---------------------------------------------------------------------------
# State.

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


def record(state: dict, src: str, status: str, **extra) -> None:
    state[src] = {
        "status": status,
        "at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        **extra,
    }
    save_state(PHASE3A_STATE, state)


def is_done(src: str, state: dict) -> bool:
    return state.get(src, {}).get("status") == "ok"


# ---------------------------------------------------------------------------
# Prompt composition.

def compose_prompt(src_basename: str, entries: list[tuple[str, str]]) -> str:
    rust_target = rust_test_path(src_basename).relative_to(REPO_ROOT)
    c_src = c_source_path(src_basename)
    c_src_rel = c_src.relative_to(REPO_ROOT) if c_src else "(no C source — driver-defined)"

    entry_list = "\n".join(
        f"   - `#[test] fn {name}()` ← C `{fn}`"
        for name, fn in entries
    )

    parts: list[str] = [
        f"# Phase 3A test-body translation: `{src_basename}`",
        "",
        "You're translating the C test bodies in `picoquictest/`",
        f"into Rust `#[test]` bodies in `{rust_target}`.",
        "",
        "## What this is",
        "",
        "**Test-driven development.**  The tests don't have to *pass*",
        "yet.  They have to be *expressed* against the Rust API as it",
        "is designed today (in `rs/fq/src/`).  Phase 4 fills in API",
        "bodies; once it does, your tests light up.  A test that",
        "compiles and panics inside `Quic::new` (or any other",
        "currently-`todo!()` API) on its first call is a clean fail.",
        "**That is fine.**  Do **not** sidestep work by leaving the",
        "body as `todo!()` — translate the C body faithfully and let",
        "the panic happen wherever it naturally does.",
        "",
        "## Required reading (in order)",
        "",
        "1. **`xlate/test_translation_guide.md`** — focused summary of",
        "   the Rust API surface, naming conventions, helper",
        "   inventory, and translation patterns.  Substitutes for",
        "   grepping `lib.rs` / `internal.rs`; only fall back to",
        "   reading those files when this guide doesn't have the",
        "   answer.",
        f"2. The C source: `{c_src_rel}`.",
        f"3. The Rust target: `{rust_target}` — currently has",
        "   auto-generated stubs (`todo!(\"<entry_fn>\")`) that you",
        "   replace with translations.",
        f"4. `rs/fq/src/tests/util.rs` — test infrastructure;",
        "   add helpers here when multiple translations would need",
        "   them.",
        "5. Module sources (`rs/fq/src/<X>.rs`) only when you need to",
        "   verify a specific signature the guide didn't quote.",
        "",
        "## Test entries to translate",
        "",
        entry_list,
        "",
        "## Translation rules",
        "",
        "- **Faithful, idiomatic.**  Walk the C body and write the",
        "  equivalent Rust.  Use Rust idioms (`assert_eq!`, `Vec`,",
        "  `Option`, iterators, `?`-propagation) — don't transcribe",
        "  the C control flow with goto-style ret tracking.",
        "- **Use the public API, not internal field access.**  C",
        "  tests routinely poke `cnx->path[0]->first_tuple->...`;",
        "  the Rust translation uses methods.  If the method doesn't",
        "  exist yet, that's OK — call the *intended* method and let",
        "  it panic; this documents the API Phase 4 must supply.",
        "- **Helpers go in `tests/util.rs`.**  C uses",
        "  `picoquic_test_set_minimal_cnx`, `tls_api_init_ctx*`,",
        "  `picoquic_test_random*`, simulator helpers, etc.  Port",
        "  the helper bodies into `tests/util.rs` once and have",
        "  every translation that needs them call through.",
        "- **Golden / fixture files** — if the C test reads a binlog",
        "  reference or qlog template, copy the file to",
        "  `rs/fq/tests/fixtures/` (creating the directory if needed)",
        "  and reach it via `concat!(env!(\"CARGO_MANIFEST_DIR\"),",
        "  \"/tests/fixtures/<name>\")`.",
        "- **Do not skip with placeholder bodies.**  `todo!(\"…\")`",
        "  in your test body is acceptable *only* when the C body",
        "  itself is a no-op or a build-flag-gated stub (`#if 0`).",
        "  Every other test gets a translated body.",
        "- **Compile is the gate, not pass.**  After your edits run",
        "  `cd rs/fq && cargo test --no-run` to verify the test",
        "  compiles.  Don't run `cargo test` itself — most translated",
        "  tests will fail with `todo!()` panics, which is expected.",
        "- **`cargo fmt` and `cargo clippy --tests --all-features --",
        "  -D warnings`** must also pass.",
        "",
        "## Process",
        "",
        f"1. Read the C source and the existing `{rust_target}`.",
        "2. Read whichever Rust modules expose the API the C body",
        "   exercises (use `Glob` / `Grep` to navigate).",
        "3. If common helpers are needed (Quic context creation,",
        "   simulator setup, certificate fixture paths), put them",
        "   in `rs/fq/src/tests/util.rs`.",
        "4. Translate every test in the entry list above.  Keep",
        "   the existing module's `//!` doc-comment header (or",
        "   write a better one) describing what this file covers.",
        "5. Validate:",
        "     cd rs/fq && cargo fmt",
        "     cd rs/fq && cargo test --no-run",
        "     cd rs/fq && cargo clippy --tests --all-features -- -D warnings",
        "   Iterate until all three are clean.",
        "6. Report on stdout: a one-paragraph summary naming each",
        "   translated entry and any helpers added to `tests/util.rs`.",
        "",
        "## Constraints",
        "",
        "- Edit / write under `rs/fq/src/tests/` and `rs/fq/tests/`",
        "  (for fixtures) only — **with one exception**: if a test",
        "  body needs to call an API method that doesn't yet exist",
        "  on a public Rust type (e.g. `Connection::record_pn_received`",
        "  isn't there but the test calls it), **add the method as a",
        "  `todo!()` stub** in the appropriate source file (typically",
        "  `rs/fq/src/internal.rs` or `rs/fq/src/lib.rs`).  Match the",
        "  shape of the C function being translated — same parameter",
        "  list (translated to Rust types), `Result<…, Error>` for",
        "  fallible C `int` returns, etc.  Add a doc comment `/// C:",
        "  `picoquic_xxx``.  This grows the API surface incrementally",
        "  as tests demand it; Phase 4 then fills the bodies.",
        "- That's the **only** API-source change allowed.  Do not",
        "  rename existing items, change existing signatures, or",
        "  reshape existing types.  If an existing method has the",
        "  wrong signature, document it on stdout and use a wrapper",
        "  helper in `tests/util.rs`.",
        "- Do **not** modify `Cargo.toml` or anything outside `rs/fq/`.",
        "- Do **not** introduce `unsafe`.",
        "",
        "## Stop spinning",
        "",
        "If 5 grep/read tool calls into the same file haven't found",
        "what you need, **stop searching**.  The API doesn't exist.",
        "Add it as a `todo!()` stub per the rule above and move on.",
        "Time spent re-grepping `internal.rs` is time not spent",
        "translating.  Aim to start writing edits within the first",
        "10 tool calls.",
    ]
    return "\n".join(parts) + "\n"


def write_prompt(src_basename: str, entries: list[tuple[str, str]]) -> Path:
    out = prompt_file_path(src_basename)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(compose_prompt(src_basename, entries))
    return out


# ---------------------------------------------------------------------------
# Claude invocation.

def invoke_claude(src: str, prompt_file: Path,
                  max_turns: int, model: str) -> tuple[int, str]:
    if shutil.which("claude") is None:
        return 127, "claude CLI not found on PATH"
    log = claude_log_path(src)
    log.parent.mkdir(parents=True, exist_ok=True)
    prompt = prompt_file.read_text()
    cmd = [
        "claude", "-p", prompt,
        "--model", model,
        "--allowedTools", ALLOWED_TOOLS,
        "--max-turns", str(max_turns),
        "--output-format", "stream-json",
        "--verbose",
    ]
    t0 = time.monotonic()
    # Stream the events to disk in real time so the run can be
    # monitored externally (`tail -f xlate/claude_logs/phase3a/<src>.log`).
    log_f = open(log, "w", buffering=1)
    log_f.write(
        f"# claude -p (phase3a) for {src}\n"
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
        # Surface a one-line summary for each tool use / message step.
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
            if ev.get("api_error_status") == 429 or "limit" in final_summary.lower():
                rate_limited = True
    rc = proc.wait()
    elapsed = time.monotonic() - t0
    log_f.write(f"\n# elapsed: {elapsed:.1f}s, exit: {rc}\n")
    log_f.close()
    if rate_limited:
        # Surface the rate-limit signal with a distinctive exit code so
        # the outer loop aborts rather than marking the source fail.
        rc = 99
    return rc, final_summary or last_assistant_text


# ---------------------------------------------------------------------------
# Gate (cargo fmt + cargo test --no-run + cargo clippy).

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


# ---------------------------------------------------------------------------
# Per-source-file runner.

def run_one(src: str, entries: list[tuple[str, str]], *,
            dry_run: bool, max_turns: int, model: str,
            state: dict) -> str:
    print(f"\n=== {src} ({len(entries)} test(s)) ===")
    rs_target = rust_test_path(src)
    if not rs_target.is_file():
        msg = f"missing test stub at {rs_target.relative_to(REPO_ROOT)}"
        print(f"  SKIP: {msg}")
        record(state, src, "fail", reason=msg)
        return "fail"

    prompt_file = write_prompt(src, entries)
    print(f"  prompt  → {prompt_file.relative_to(REPO_ROOT)}")
    if dry_run:
        print("  (dry-run; skipping claude + gate)")
        return "skip"

    print(f"  claude  → invoking ({model}, max-turns={max_turns}) …")
    before = rs_target.read_text()
    code, stdout = invoke_claude(src, prompt_file, max_turns, model)
    if code == 99:
        # Rate limit hit; do NOT mark the source as failed (the work
        # never started).  Surface a distinct status so the outer
        # loop aborts the sweep cleanly.
        print(f"    RATE-LIMITED: {stdout.strip()[:200]}")
        return "rate-limited"
    if code != 0:
        log_rel = claude_log_path(src).relative_to(REPO_ROOT)
        print(f"    FAIL: claude exit {code} (see {log_rel})")
        record(state, src, "fail", stage="claude", exit_code=code)
        return "fail"
    after = rs_target.read_text()
    changed = before != after

    print("  gate    → cargo fmt + cargo test --no-run + cargo clippy")
    rc = run_gate()
    if rc != 0:
        print(f"    FAIL: gate exit {rc}")
        record(state, src, "fail", stage="gate", exit_code=rc)
        return "fail"
    print("  ok" + ("" if changed else " (no edits)"))
    record(state, src, "ok", changed=changed)
    return "ok"


# ---------------------------------------------------------------------------
# Status / driver glue.

def cmd_status(by_src: list[tuple[str, list[tuple[str, str]]]]) -> None:
    state = load_state(PHASE3A_STATE)
    eligible = [s for s, _ in by_src]
    ok = [s for s in eligible if state.get(s, {}).get("status") == "ok"]
    failed = [s for s in eligible if state.get(s, {}).get("status") == "fail"]
    pending = [s for s in eligible if not is_done(s, state)
               and state.get(s, {}).get("status") != "fail"]
    print(f"Phase 3A progress: {len(eligible)} source files")
    print(f"  ok:      {len(ok)}")
    print(f"  failed:  {len(failed)}")
    print(f"  pending: {len(pending)}")
    if failed:
        print("\nFailed sources (will retry on re-run):")
        for s in failed:
            at = state[s].get("at", "?")
            print(f"  - {s}  (last: {at})")


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
                   help="Translate one source file (path or basename, "
                        "e.g. picoquictest/sacktest.c or sacktest).")
    p.add_argument("--limit", type=int, default=None,
                   help="Stop after N source files.")
    p.add_argument("--dry-run", action="store_true",
                   help="Print plan; do not invoke claude or run the gate.")
    p.add_argument("--stop-on-failure", action="store_true",
                   help="Stop at the first failure (default is to continue).")
    p.add_argument("--force", action="store_true",
                   help="Re-translate sources already marked ok.")
    p.add_argument("--status", action="store_true",
                   help="Print progress and exit.")
    p.add_argument("--max-turns", type=int, default=200,
                   help="Per-source turn limit for claude (default 200).")
    p.add_argument("--model", default="sonnet",
                   help="Model passed to `claude -p --model` "
                        "(default: sonnet).")
    args = p.parse_args()

    by_src = group_by_source()

    if args.status:
        cmd_status(by_src)
        return 0

    state = load_state(PHASE3A_STATE)
    if args.src:
        # Accept either basename ("sacktest") or path
        # ("picoquictest/sacktest.c").
        wanted = Path(args.src).stem
        wanted = wanted[:-2] if wanted.endswith(".c") else wanted
        targets = [(s, entries) for s, entries in by_src if s == wanted]
        if not targets:
            print(f"unknown source: {args.src}", file=sys.stderr)
            return 1
    else:
        targets = list(by_src)

    # Skip <unknown> / <harness> sentinel buckets — they aren't
    # backed by a real C file (sim_link_test, etc.).
    targets = [(s, e) for s, e in targets if not s.startswith("<")]

    if not args.force:
        targets = [(s, e) for s, e in targets if not is_done(s, state)]

    if args.limit is not None:
        targets = targets[: args.limit]

    if not targets:
        print("nothing to do (use --force to redo finished sources)")
        return 0

    log_path = setup_run_log()
    t_run_start = time.monotonic()
    print(f"Phase 3A: {len(targets)} source file(s) to translate")
    print(f"  run log: {log_path.relative_to(REPO_ROOT)}")
    print(f"  state:   {PHASE3A_STATE.relative_to(REPO_ROOT)}")
    failed: list[str] = []
    succeeded: list[str] = []
    rate_limited_at: str | None = None
    for src, entries in targets:
        result = run_one(src, entries, dry_run=args.dry_run,
                         max_turns=args.max_turns, model=args.model,
                         state=state)
        if result == "rate-limited":
            rate_limited_at = src
            print(f"\nABORTING SWEEP: rate-limited at {src}.  "
                  f"Re-run later; state file is unchanged for this "
                  f"and pending sources.")
            break
        if result == "fail":
            failed.append(src)
            if args.stop_on_failure:
                print(f"\nStopped at {src} (--stop-on-failure).  "
                      f"Re-run to retry.")
                break
        elif result == "ok":
            succeeded.append(src)

    elapsed = time.monotonic() - t_run_start
    print(f"\n=== Phase 3A run summary ===")
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
