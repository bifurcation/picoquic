#!/usr/bin/env python3
"""phase1.py — Run Phase 1 of the C → Rust translation, fully automatically.

For each in-scope C header in topological (leaves-first) order:

  1. Pre-step: run `bindgen` to produce a reference baseline at
     `xlate/bindgen_baseline/<path>.rs`.  Bindgen is allowlisted to
     symbols defined in this header so the output stays focused.
     The baseline is *not* the production module — it's a diff-aid
     for the translator.

  2. Translation: invoke `claude -p` (non-interactive Claude Code)
     with a self-contained prompt that:
       - names the header,
       - points at the bindgen baseline,
       - cites TRANSLATE_PLAN.md / CLAUDE.md as binding,
       - tells Claude to read callers to decide pointer shapes,
       - tells Claude to write `rs/fq/src/<mirrored>.rs` with
         `todo!()` bodies,
       - tells Claude to run `cargo check` from `rs/fq/` itself
         and iterate until it passes.

  3. Post-step: regenerate parent `mod` files so the new module is
     wired into the crate, then run the gate from `rs/fq/`:
       cargo fmt
       cargo clippy -- -D warnings
       cargo check

If the gate passes, advance to the next header.  Otherwise stop
(default), or skip with `--continue-on-failure`.

Headers whose Rust target file already exists are skipped — re-run
with `--force` to redo a header.

Usage:
  python3 scripts/phase1.py                       # run all headers (continue on failure)
  python3 scripts/phase1.py --limit 1             # one header (smoke)
  python3 scripts/phase1.py --header picoquic/foo.h
  python3 scripts/phase1.py --dry-run             # plan only, no claude
  python3 scripts/phase1.py --stop-on-failure     # stop at first failure
  python3 scripts/phase1.py --force               # redo finished headers
  python3 scripts/phase1.py --status              # progress only

Requirements:
  - `bindgen` on PATH (`cargo install bindgen-cli`).  If missing, the
    baseline step writes a stub and the run continues.
  - `claude` on PATH (the Claude Code CLI).  Required unless
    --dry-run.
  - xlate/inventory.json must exist (run `scripts/phase0.py` first).

Claude is invoked with a narrow `--allowedTools` list — Read, Edit,
Write, Glob, Grep, plus `Bash(cargo check)` so it can self-validate.
No `bypassPermissions`; Claude cannot run arbitrary commands or
touch files outside rs/fq/.  The session is a fresh, isolated
`claude -p`; settings from any parent session do not carry over.

State and logs:
  - xlate/phase1_state.json — per-header status ('ok'|'fail').
    Re-runs skip 'ok' headers and retry the rest (use --force to
    redo everything).
  - xlate/phase1_runs/<timestamp>.log — full stdout for each run,
    tee'd from the pipeline.
  - xlate/claude_logs/<path>.log — per-header transcript (claude
    stdout/stderr).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import textwrap
import time
from collections import defaultdict
from pathlib import Path

REPO_ROOT     = Path(__file__).resolve().parent.parent
INV_PATH      = REPO_ROOT / "xlate" / "inventory.json"
CC_PATH       = REPO_ROOT / "build" / "compile_commands.json"
BINDGEN_DIR   = REPO_ROOT / "xlate" / "bindgen_baseline"
PROMPTS_DIR   = REPO_ROOT / "xlate" / "prompts"
LOG_DIR       = REPO_ROOT / "xlate" / "claude_logs"
RUNS_DIR      = REPO_ROOT / "xlate" / "phase1_runs"
STATE_PATH    = REPO_ROOT / "xlate" / "phase1_state.json"
RS_CRATE      = REPO_ROOT / "rs" / "fq"
RS_SRC        = RS_CRATE / "src"

# Tools claude is allowed to use during a per-header session.
# - Read/Edit/Write/Glob/Grep: write the module, search C sources
#   for caller usage.
# - Bash(cargo check) + Bash(cargo clippy): self-validation only.
#   No other shell access.  These match the gate the parent script
#   runs after claude finishes, so claude can converge on a
#   gate-clean translation before returning.
ALLOWED_TOOLS = "Read Edit Write Glob Grep Bash(cargo check) Bash(cargo clippy)"

# v1 scope: picoquic-core only.  See scripts/inventory.py for context.
IN_SCOPE = {"picoquic"}

INCLUDE_RE = re.compile(r'^\s*#\s*include\s+"([^"]+)"', re.M)


# ---------------------------------------------------------------------------
# Header order: topological by #include, leaves first

def in_scope_headers(inv: dict) -> list[str]:
    return sorted(
        f["file"] for f in inv["files"] if f["file"].endswith(".h")
    )


def header_local_includes(header_path: Path) -> set[str]:
    """`#include "..."` paths in this header that resolve to in-scope
    files, returned as repo-relative paths.
    """
    try:
        text = header_path.read_text(errors="replace")
    except OSError:
        return set()
    raw = INCLUDE_RE.findall(text)
    out: set[str] = set()
    candidates = [REPO_ROOT / d for d in IN_SCOPE]
    for inc in raw:
        for d in [header_path.parent, *candidates]:
            cand = (d / inc).resolve()
            if not cand.is_file():
                continue
            try:
                rel = cand.relative_to(REPO_ROOT)
            except ValueError:
                continue
            if rel.parts[0] in IN_SCOPE:
                out.add(str(rel))
                break
    return out


def topological_order(headers: list[str]) -> list[str]:
    """Kahn's algorithm with alphabetical tie-breaking.  Headers in a
    cycle are emitted last in alphabetical order (we don't expect any).
    """
    deps = {h: header_local_includes(REPO_ROOT / h) & set(headers)
            for h in headers}
    indeg = {h: len(deps[h]) for h in headers}
    rev: dict[str, set[str]] = defaultdict(set)
    for h, ds in deps.items():
        for d in ds:
            rev[d].add(h)
    ready = sorted(h for h, n in indeg.items() if n == 0)
    out: list[str] = []
    while ready:
        h = ready.pop(0)
        out.append(h)
        for parent in sorted(rev[h]):
            indeg[parent] -= 1
            if indeg[parent] == 0:
                pos = 0
                while pos < len(ready) and ready[pos] < parent:
                    pos += 1
                ready.insert(pos, parent)
    if len(out) != len(headers):
        out.extend(sorted(set(headers) - set(out)))
    return out


# ---------------------------------------------------------------------------
# Path conventions

def rust_path_for(header: str) -> Path:
    """`picoquic/foo.h` → `rs/fq/src/picoquic/foo.rs`."""
    return RS_SRC / Path(header).with_suffix(".rs")


def bindgen_baseline_path(header: str) -> Path:
    return BINDGEN_DIR / Path(header).with_suffix(".rs")


def prompt_path(header: str) -> Path:
    return PROMPTS_DIR / Path(header).with_suffix(".md")


def claude_log_path(header: str) -> Path:
    return LOG_DIR / Path(header).with_suffix(".log")


# ---------------------------------------------------------------------------
# Bindgen baseline

def find_compile_command_for(header: str, ccs: list[dict]) -> dict | None:
    """Pick a compile_commands entry whose flags will work for this
    header.  Prefer same-dir same-stem; otherwise any .c in the same dir.
    """
    target = Path(header)
    for e in ccs:
        ep = Path(e["file"])
        if ep.parent.name == target.parent.name and ep.stem == target.stem:
            return e
    for e in ccs:
        ep = Path(e["file"])
        try:
            rel = ep.relative_to(REPO_ROOT)
        except ValueError:
            continue
        if rel.parent == target.parent:
            return e
    return None


# Cached topological order of all in-scope headers.  Computed once;
# reused for every bindgen pre-include list.
_HEADER_ORDER_CACHE: list[str] | None = None


def all_headers_in_order() -> list[str]:
    global _HEADER_ORDER_CACHE
    if _HEADER_ORDER_CACHE is None:
        inv = json.loads(INV_PATH.read_text())
        _HEADER_ORDER_CACHE = topological_order(in_scope_headers(inv))
    return _HEADER_ORDER_CACHE


def clang_flags_for(entry: dict) -> list[str]:
    cmd = shlex.split(entry["command"])
    out: list[str] = []
    skip = False
    for tok in cmd[1:]:
        if skip:
            skip = False
            continue
        if tok == "-o":
            skip = True
            continue
        if tok == "-c":
            continue
        if tok == entry["file"]:
            continue
        # `-arch X` is Apple-Clang-specific.  When `bindgen` calls
        # libclang via a rustup toolchain whose default target isn't
        # Apple (e.g. ESP/RISC-V), `-arch` is rejected outright.  Drop
        # it; libclang infers the host target from the SDK + LIBCLANG.
        if tok == "-arch":
            skip = True
            continue
        out.append(tok)
    try:
        sdk = subprocess.check_output(
            ["xcrun", "--show-sdk-path"], text=True
        ).strip()
        if sdk and "-isysroot" not in out:
            out.extend(["-isysroot", sdk])
    except (FileNotFoundError, subprocess.CalledProcessError):
        pass
    return out


# Force bindgen's libclang to be the Homebrew one rather than whatever
# the active rustup toolchain bundles.  Same dylib our Phase 0 scripts
# use.
HOMEBREW_LIBCLANG = "/opt/homebrew/opt/llvm/lib/libclang.dylib"


def run_bindgen(header: str) -> tuple[Path, str]:
    """Run bindgen for one header.  Returns (output_path, status).
    Status is one of 'ok', 'fail', 'no-bindgen'.

    Many headers in `picoquic/` don't `#include` their own type
    prerequisites — they assume a consumer has done so first.
    Running bindgen on such a header alone fails with `unknown type`.
    Workaround: pre-include `picoquic.h` and `picoquic_internal.h`
    via clang `-include` flags before the target.  Together those
    two are the project's kitchen-sink headers — `picoquic.h`
    pulls in <stdint.h>, sockets, etc. and the public API types;
    `picoquic_internal.h` extends with internal types (it includes
    picoquic.h, picohash.h, picosplay.h, picoquic_utils.h).  Every
    in-scope picoquic-core type is defined after these two.

    `--allowlist-file` then restricts the bindgen *output* to only
    symbols whose source location is the target header.
    """
    out = bindgen_baseline_path(header)
    out.parent.mkdir(parents=True, exist_ok=True)
    if shutil.which("bindgen") is None:
        out.write_text(
            "// bindgen not installed.\n"
            "// install with: cargo install bindgen-cli\n"
        )
        return out, "no-bindgen"
    ccs = json.loads(CC_PATH.read_text())
    cc = find_compile_command_for(header, ccs)
    flags = clang_flags_for(cc) if cc else []

    # Pre-includes: the two kitchen-sink headers, except don't
    # include the target itself (causes a benign re-include via
    # the guard, but cleaner to skip).
    KITCHEN_SINK = ("picoquic/picoquic.h", "picoquic/picoquic_internal.h")
    pre_includes: list[str] = []
    for h in KITCHEN_SINK:
        if h == header:
            continue
        pre_includes.extend(["-include", str(REPO_ROOT / h)])

    cmd = [
        "bindgen",
        str(REPO_ROOT / header),
        "--allowlist-file", f".*{re.escape(header)}$",
        "--no-layout-tests",
        "-o", str(out),
        "--",
        *pre_includes,
        *flags,
    ]
    env = {**os.environ}
    if Path(HOMEBREW_LIBCLANG).is_file():
        env["LIBCLANG_PATH"] = HOMEBREW_LIBCLANG
    res = subprocess.run(cmd, capture_output=True, text=True, env=env)
    if res.returncode != 0:
        out.write_text(
            f"// bindgen failed (exit {res.returncode}).\n"
            f"// command: {' '.join(shlex.quote(c) for c in cmd)}\n"
            f"// stderr:\n//   "
            + "\n//   ".join(res.stderr.splitlines())
            + "\n"
        )
        return out, "fail"
    return out, "ok"


# ---------------------------------------------------------------------------
# Parent-module wiring

def regenerate_parent_modules() -> None:
    """For every directory under rs/fq/src/ that contains .rs files,
    write/overwrite a parent .rs file with `pub mod <name>;` lines for
    each child file/dir.  Idempotent.
    """
    if not RS_SRC.is_dir():
        return

    def submods_for(dir_path: Path) -> list[str]:
        names: set[str] = set()
        for child in dir_path.iterdir():
            if child.name.startswith("."):
                continue
            if child.is_dir():
                names.add(child.name)
            elif child.suffix == ".rs":
                stem = child.stem
                if stem in {"lib", "main"}:
                    continue
                # Skip files that are themselves parent module files
                # (i.e., a .rs file with a sibling directory of the
                # same name); those are already declared via the dir.
                if (dir_path / stem).is_dir():
                    continue
                names.add(stem)
        return sorted(names)

    # Walk the dir tree under src/, regenerate each parent .rs.
    for dir_path in [RS_SRC, *sorted(p for p in RS_SRC.rglob("*") if p.is_dir())]:
        if dir_path == RS_SRC:
            target = RS_SRC / "lib.rs"
        else:
            # rs/fq/src/picoquic/  →  rs/fq/src/picoquic.rs
            target = dir_path.with_suffix(".rs")
        names = submods_for(dir_path)
        if not names:
            continue
        # Preserve any non-`pub mod` content the user added (e.g.,
        # crate-level attributes in lib.rs).  We rewrite only the
        # contiguous block of `pub mod ...;` lines, leaving the rest.
        existing = target.read_text() if target.is_file() else ""
        pubmod_block = "".join(f"pub mod {n};\n" for n in names)
        # Remove any existing pub mod lines and re-insert the block.
        cleaned = re.sub(
            r"(?m)^pub mod\s+[A-Za-z_]\w*;\s*\n", "", existing
        )
        if target == RS_SRC / "lib.rs":
            # Keep crate-level attrs at top, then mod block, then rest.
            new_text = cleaned.rstrip() + "\n\n" + pubmod_block
            new_text = new_text.lstrip("\n")
        else:
            new_text = pubmod_block
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.is_file() or target.read_text() != new_text:
            target.write_text(new_text)


# ---------------------------------------------------------------------------
# Prompt

def compose_prompt(header: str) -> str:
    rust_target = rust_path_for(header).relative_to(REPO_ROOT)
    bindgen = bindgen_baseline_path(header).relative_to(REPO_ROOT)
    src = REPO_ROOT / header
    matching_c = src.with_suffix(".c")
    deps = sorted(header_local_includes(src))
    has_c = matching_c.exists()

    if has_c:
        c_line = f"4. The matching `.c` file: `{matching_c.relative_to(REPO_ROOT)}`."
    else:
        c_line = "4. (No matching `.c` file — header-only module.)"

    if deps:
        deps_lines = ["5. Headers this one directly depends on (for context):"]
        deps_lines.extend(f"   - `{d}`" for d in deps)
    else:
        deps_lines = ["5. (No in-scope dependencies.)"]

    parts: list[str] = [
        f"# Phase 1 translation: `{header}` → Rust",
        "",
        "You are running as part of a fully-automated translation pipeline.",
        "Your job is to translate **one C header** to a Rust module,",
        "following Phase 1 of the project's translation plan.",
        "",
        "## Required reading (in this order)",
        "1. `TRANSLATE_PLAN.md` — the binding plan and rules.  Phase 1",
        "   is the section that applies here.",
        "2. `CLAUDE.md` — project conventions, especially the rule that",
        "   you may only edit files under `rs/`, `scripts/`, or `*.md`.",
        f"3. The C header you are translating: `{header}`.",
        c_line,
        *deps_lines,
        "6. Reference scaffolding (do NOT ship; for diff comparison only):",
        f"   `{bindgen}`",
        "",
        "## What to produce",
        "Write a Rust module at:",
        "",
        f"    {rust_target}",
        "",
        "A placeholder file already exists at that path and is wired",
        "into the crate via `lib.rs` and parent `mod` files — your",
        "edits will be picked up by `cargo check` immediately.",
        "",
        "**Note on pre-existing stubs.**  Earlier headers in this run",
        "may have left minimal `todo!()` stubs at sibling paths under",
        "`rs/fq/src/picoquic/` so they could compile.  If the file at",
        "your target path is one of those stubs (rather than the",
        "untouched placeholder), treat it as throw-away scaffolding:",
        "replace it with the full translation of your header.  The",
        "earlier dependents may need updates after that — fix any",
        "compile breakage their files now show during your `cargo",
        "check` loop, but only when the fix is mechanical (e.g.",
        "filling in a now-defined field, adjusting an import path).",
        "Do not redesign other modules.",
        "",
        "### Phase 1 contract (binding)",
        "- All function bodies are `todo!()`.  Phase 3 fills them in.",
        "- Add an empty `#[cfg(test)] mod test {}` at the end of the",
        "  module (Phase 2 fills it).",
        "- Drop `#[repr(C)]` *except* on types that cross an external",
        "  boundary (FFI, syscall, wire format).",
        "- Pointer fields and parameters get a Rust shape decided by",
        "  reading callers: `&T`, `&mut T`, `Box<T>`, `Vec<T>`, `&[T]`,",
        "  `Option<T>` for nullable.  `Rc<RefCell<T>>` is the",
        "  last-resort escape hatch.  Raw pointers only inside `unsafe`,",
        "  and only with a `// SAFETY:` comment.",
        "- Function pointers map to traits.",
        "- Bitfields → integer field with mask/shift accessors (or",
        "  `bitflags!` for flag bitsets).",
        "- Unions → Rust `enum`.",
        "- Flexible array members → `Box<[T]>` or `Vec<T>`.",
        "- Macros: `const` for value `#define`s, `fn` for function-like",
        "  macros, `macro_rules!` only when there's no alternative.",
        "- The crate is `#![no_std] + alloc` with an `std` Cargo",
        "  feature.  Use `core::` / `alloc::` paths; do not introduce",
        "  `std::` references except behind a feature gate.",
        "- One top-level `crate::Error` enum; functions return",
        "  `Result<T, Error>`.  v1 may not have one yet — if so, do",
        "  not invent it; use `Result<T, ()>` or skip the error type",
        "  and leave a TODO comment naming the gap.",
        "- `Send`/`Sync` are NOT required (single-threaded scope).",
        "",
        "### Procedure",
        "1. Read `TRANSLATE_PLAN.md` and the materials above.",
        "2. For each pointer field/parameter, grep the C source under",
        "   `picoquic/` to see how callers pass it (NULL? `&local`?",
        "   owning new allocation?  freed by callee?).  Use the",
        "   evidence to pick the Rust shape.  Comment your choice",
        "   briefly when non-obvious.",
        f"3. Replace the placeholder content of `{rust_target}`.",
        "4. From `rs/fq/`, run **both** `cargo check` and",
        "   `cargo clippy -- -D warnings`.  Iterate until **both**",
        "   pass cleanly.  These match the parent script's gate, so",
        "   converging here means you're done.  If a callee module",
        "   doesn't exist yet, create a minimal stub at the right",
        "   path with `todo!()` placeholders for the types you need.",
        "   Don't translate other headers.",
        "5. Do NOT run `cargo test` or `cargo fmt` — the parent",
        "   script runs `cargo fmt` after you finish.",
        "",
        "### Constraints",
        "- You may only create / edit files under `rs/fq/src/`.  In",
        "  particular, do NOT touch C sources, CMake, `xlate/`, or",
        "  anything outside `rs/fq/`.",
        "- Do NOT modify `rs/fq/Cargo.toml` unless the translation",
        "  genuinely requires a new dependency.  If it does, add it",
        "  with `default-features = false` and note why in your",
        "  response.",
        "- The parent script regenerates `pub mod` lines in `lib.rs`",
        "  and parent `mod` files.  Don't waste turns editing them; the",
        "  pre-wired layout is enough for `cargo check`.",
        "",
        "Report when done with a one-line summary of any non-obvious",
        "choices you made.",
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
    """Run `claude -p` with the prompt for one header.

    Returns (exit_code, transcript). The transcript is also tee'd to
    xlate/claude_logs/<path>.log.
    """
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
        stdin=subprocess.DEVNULL,  # otherwise `claude -p` waits 3s on stdin
    )
    elapsed = time.monotonic() - t0
    transcript = (
        f"# claude -p for {header}\n"
        f"# elapsed: {elapsed:.1f}s, exit: {res.returncode}\n"
        f"# command: {' '.join(shlex.quote(c) for c in cmd[:1] + cmd[3:])}\n"
        f"# (prompt omitted; see xlate/prompts/{header.replace('.h', '.md')})\n\n"
        f"## stdout\n{res.stdout}\n\n## stderr\n{res.stderr}\n"
    )
    log.write_text(transcript)
    return res.returncode, transcript


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
# State (xlate/phase1_state.json)

def load_state() -> dict:
    if STATE_PATH.is_file():
        try:
            return json.loads(STATE_PATH.read_text())
        except json.JSONDecodeError:
            return {}
    return {}


def save_state(state: dict) -> None:
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    STATE_PATH.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n")


def record(state: dict, header: str, status: str, **extra) -> None:
    state[header] = {
        "status": status,
        "at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        **extra,
    }
    save_state(state)


def is_done(header: str, state: dict) -> bool:
    """A header is 'done' if state explicitly says so.

    Status meanings:
      ok    — translated and gate-clean.  Skip on subsequent runs.
      fail  — last run failed.  Retry.
      stub  — claude wrote a minimal `todo!()` stub for this header
              while translating a *dependent* header so its module
              could compile.  Not done — replace with the real
              translation when this header's turn comes.

    For headers with no state record (e.g., translated manually
    outside the script, or a fresh clone where state was lost) but
    whose Rust file exists, fall back to file existence — that
    skips committed/manual translations without forcing a re-run.
    """
    s = state.get(header, {}).get("status")
    if s == "ok":
        return True
    if s in {"fail", "stub"}:
        return False
    return rust_path_for(header).is_file()


def header_for_rs_path(rs_path: Path, all_headers: set[str]) -> str | None:
    """Reverse of `rust_path_for`: given an `rs/fq/src/.../foo.rs` path,
    return the header it would correspond to (e.g., `picoquic/foo.h`),
    or None if the .rs path is not a translation target (e.g., lib.rs,
    a parent module file like picoquic.rs sitting next to picoquic/,
    or under a directory not in IN_SCOPE).
    """
    try:
        rel = rs_path.relative_to(RS_SRC)
    except ValueError:
        return None
    if rel.name in {"lib.rs", "main.rs"}:
        return None
    # A parent module file foo.rs has a sibling directory foo/.
    if (rs_path.parent / rs_path.stem).is_dir():
        return None
    # `rs/fq/src/<dir>/<file>.rs` ↔ `<dir>/<file>.h`
    parts = rel.with_suffix("").parts
    if len(parts) < 2 or parts[0] not in IN_SCOPE:
        return None
    header = "/".join(parts) + ".h"
    return header if header in all_headers else None


def existing_rs_files() -> set[Path]:
    return {p for p in RS_SRC.rglob("*.rs") if p.is_file()}


def cmd_status(headers: list[str]) -> None:
    state = load_state()
    ok = [h for h in headers if state.get(h, {}).get("status") == "ok"]
    failed = [h for h in headers if state.get(h, {}).get("status") == "fail"]
    stubs = [h for h in headers if state.get(h, {}).get("status") == "stub"]
    untracked_with_file = [
        h for h in headers
        if h not in state and rust_path_for(h).is_file()
    ]
    pending = [
        h for h in headers
        if not is_done(h, state)
        and state.get(h, {}).get("status") not in {"fail", "stub"}
    ]
    print(f"Phase 1 progress: {len(headers)} total")
    print(f"  ok (state):           {len(ok)}")
    print(f"  failed (state):       {len(failed)}")
    print(f"  stub (state):         {len(stubs)}")
    print(f"  manual / untracked:   {len(untracked_with_file)}")
    print(f"  pending:              {len(pending)}")
    if failed:
        print("\nFailed headers (will retry on re-run):")
        for h in failed:
            at = state[h].get("at", "?")
            print(f"  - {h}  (last: {at})")
    if stubs:
        print("\nStubbed headers (placeholder left by a dependent's run; "
              "will translate when their turn comes):")
        for h in stubs:
            by = state[h].get("stubbed_by", "?")
            print(f"  - {h}  (stubbed by: {by})")


# ---------------------------------------------------------------------------
# Run log: tee stdout/stderr to xlate/phase1_runs/<timestamp>.log

class _Tee:
    def __init__(self, *streams):
        self._streams = streams

    def write(self, s: str) -> int:
        n = 0
        for st in self._streams:
            n = st.write(s)
            try:
                st.flush()
            except Exception:
                pass
        return n

    def flush(self) -> None:
        for st in self._streams:
            try:
                st.flush()
            except Exception:
                pass


def setup_run_log() -> Path:
    RUNS_DIR.mkdir(parents=True, exist_ok=True)
    path = RUNS_DIR / f"{time.strftime('%Y%m%dT%H%M%S')}.log"
    f = open(path, "w", buffering=1)  # line-buffered
    sys.stdout = _Tee(sys.__stdout__, f)
    sys.stderr = _Tee(sys.__stderr__, f)
    return path


# ---------------------------------------------------------------------------
# Main loop

PLACEHOLDER_TEMPLATE = (
    "//! Phase 1 placeholder for `{header}`.\n"
    "//! The driver script seeds this file and wires it into the crate\n"
    "//! so `cargo check` exercises your translation as you iterate.\n"
    "//! Replace this whole file with the translation.\n"
)


def seed_placeholder(header: str) -> None:
    """Pre-wire: write a fresh placeholder at the target path.

    Always overwrites: we're about to translate this header, so any
    pre-existing content (real translation, stub from a dependent's
    run, leftover from a failed iteration) is throw-away.  By the
    time we get here, `is_done()` has already filtered out headers
    marked "ok" in state, so we won't clobber gate-clean output.
    """
    rs_target = rust_path_for(header)
    rs_target.parent.mkdir(parents=True, exist_ok=True)
    rs_target.write_text(PLACEHOLDER_TEMPLATE.format(header=header))


def is_still_placeholder(header: str) -> bool:
    rs_target = rust_path_for(header)
    if not rs_target.is_file():
        return True
    return rs_target.read_text() == PLACEHOLDER_TEMPLATE.format(header=header)


def record_new_stubs(state: dict, *, header: str,
                     before: set[Path], after: set[Path],
                     all_headers: set[str]) -> list[str]:
    """Mark any newly-created .rs files (other than `header`'s target)
    as stubs in state, so future runs know to translate them properly.

    Returns the list of header paths that got recorded.
    """
    target = rust_path_for(header)
    new_files = (after - before) - {target}
    recorded: list[str] = []
    for p in sorted(new_files):
        h = header_for_rs_path(p, all_headers)
        if h is None:
            continue
        # Don't downgrade an already-ok translation if claude touched it.
        if state.get(h, {}).get("status") == "ok":
            continue
        state[h] = {
            "status": "stub",
            "stubbed_by": header,
            "at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        }
        recorded.append(h)
    if recorded:
        save_state(state)
    return recorded


def run_one(header: str, *, dry_run: bool, max_turns: int,
            state: dict, all_headers: set[str]) -> str:
    """Translate one header.  Returns 'ok', 'skip', or 'fail'.

    Updates `state` (and persists it) at every transition so a
    Ctrl-C between headers leaves the state file consistent.
    """
    print(f"\n=== {header} ===")
    print(f"  bindgen → {bindgen_baseline_path(header).relative_to(REPO_ROOT)}")
    if not dry_run:
        bg, status = run_bindgen(header)
        print(f"    {status}")
    prompt_file = write_prompt_file(header)
    print(f"  prompt  → {prompt_file.relative_to(REPO_ROOT)}")
    if dry_run:
        print("  (dry-run; skipping claude + gate)")
        return "skip"

    seed_placeholder(header)
    regenerate_parent_modules()
    print(f"  prewire → {rust_path_for(header).relative_to(REPO_ROOT)} "
          "+ parent mod files")

    rs_before = existing_rs_files()

    print(f"  claude  → invoking (max-turns={max_turns}) …")
    code, _transcript = invoke_claude(header, prompt_file, max_turns)
    if code != 0:
        print(f"    FAIL: claude exit {code} "
              f"(see {claude_log_path(header).relative_to(REPO_ROOT)})")
        record(state, header, "fail", stage="claude", exit_code=code)
        return "fail"
    if is_still_placeholder(header):
        msg = (f"claude finished but "
               f"{rust_path_for(header).relative_to(REPO_ROOT)} "
               "is still the placeholder")
        print(f"    FAIL: {msg}")
        record(state, header, "fail", stage="claude", reason=msg)
        return "fail"

    rs_after = existing_rs_files()
    new_stubs = record_new_stubs(state, header=header,
                                 before=rs_before, after=rs_after,
                                 all_headers=all_headers)
    if new_stubs:
        print(f"  stubs   → recorded {len(new_stubs)}: "
              f"{', '.join(new_stubs)}")

    print("  rewire  → regenerate parent mod files")
    regenerate_parent_modules()

    print("  gate    → cargo fmt + clippy + check")
    rc = run_gate()
    if rc != 0:
        print(f"    FAIL: gate exit {rc}")
        record(state, header, "fail", stage="gate", exit_code=rc)
        return "fail"
    print("  ok")
    record(state, header, "ok")
    return "ok"


def main() -> int:
    p = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    p.add_argument("--header",
                   help="Translate only this header (relative path).")
    p.add_argument("--limit", type=int, default=None,
                   help="Stop after translating N headers.")
    p.add_argument("--dry-run", action="store_true",
                   help="Print plan; do not invoke claude or run the gate.")
    p.add_argument("--stop-on-failure", action="store_true",
                   help="Stop at the first failure (default is to continue).")
    p.add_argument("--force", action="store_true",
                   help="Re-translate headers already marked 'ok' or with "
                        "existing rs/fq/src/ files.")
    p.add_argument("--status", action="store_true",
                   help="Print progress and exit.")
    p.add_argument("--print-order", action="store_true",
                   help="Print headers in topological translation order and exit.")
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

    state = load_state()

    if args.header:
        if args.header not in all_headers:
            print(f"not in scope: {args.header}", file=sys.stderr)
            return 1
        targets = [args.header]
    else:
        targets = order

    if not args.force:
        targets = [h for h in targets if not is_done(h, state)]

    if args.limit is not None:
        targets = targets[: args.limit]

    if not targets:
        print("nothing to do (use --force to redo finished headers)")
        return 0

    log_path = setup_run_log()
    t_run_start = time.monotonic()
    print(f"Phase 1: {len(targets)} header(s) to translate")
    print(f"  run log: {log_path.relative_to(REPO_ROOT)}")
    print(f"  state:   {STATE_PATH.relative_to(REPO_ROOT)}")
    failed: list[str] = []
    succeeded: list[str] = []
    headers_set = set(all_headers)
    for h in targets:
        result = run_one(h, dry_run=args.dry_run, max_turns=args.max_turns,
                         state=state, all_headers=headers_set)
        if result == "fail":
            failed.append(h)
            if args.stop_on_failure:
                print(f"\nStopped at {h} (--stop-on-failure).  Re-run to retry.")
                break
        elif result == "ok":
            succeeded.append(h)

    elapsed = time.monotonic() - t_run_start
    print(f"\n=== Phase 1 run summary ===")
    print(f"  elapsed:   {elapsed:.1f}s")
    print(f"  succeeded: {len(succeeded)}")
    print(f"  failed:    {len(failed)}")
    for h in failed:
        print(f"    - {h}")
    print(f"  log:       {log_path.relative_to(REPO_ROOT)}")
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
