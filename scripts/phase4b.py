#!/usr/bin/env python3
"""phase4b.py — implement approved missing required functions.

Phase 4B consumes the approved Phase 4A map/plan.  It invokes the
configured agent on `required_missing` entries, then refreshes
`xlate/function_translation_map.json` so completed functions move to
`implemented`.

The script intentionally refuses to run unless the Phase 4A plan says
`Status: approved`, unless `--approved` is passed explicitly.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import time
from pathlib import Path

import agent_runner
from phase4_common import (
    FUNCTION_MAP,
    REPO_ROOT,
    RS_CRATE,
    XLATE,
    build_function_map,
    load_function_map,
    save_json,
)

PLAN_MD = XLATE / "phase4a_plan.md"
PROMPTS_DIR = XLATE / "prompts" / "phase4b"
CALL_GRAPH = XLATE / "call_graph.json"


def _load_callgraph_heights() -> dict[str, int]:
    """Load the C call-graph height-of-each-function from Phase 0.

    Higher height = farther from leaves.  Used to order Phase 4B work
    leaves-first so a function's callees are translated before the
    function itself.
    """
    if not CALL_GRAPH.is_file():
        return {}
    try:
        return json.loads(CALL_GRAPH.read_text()).get("height_of", {})
    except (OSError, json.JSONDecodeError):
        return {}


def _sort_key_leaves_first(heights: dict[str, int], entry: dict) -> tuple[int, str]:
    name = entry.get("c", {}).get("name", "")
    h = heights.get(name)
    # Unmapped names (heights miss them) go after all mapped ones.
    return (h if h is not None else 10**6, entry.get("c_id", ""))

ALLOWED_TOOLS = (
    "Read Edit Write Glob Grep "
    "Bash(cargo check:*) "
    "Bash(cargo test:*) "
    "Bash(cargo fmt:*) "
    "Bash(cargo clippy:*) "
    "Bash(python3 scripts/phase4_check.py:*) "
    "Bash(python3 scripts/phase4a.py:*)"
)


def is_plan_approved() -> bool:
    if not PLAN_MD.is_file():
        return False
    for line in PLAN_MD.read_text(errors="replace").splitlines()[:20]:
        if line.strip().lower() in {"status: approved", "approved: true"}:
            return True
    return False


def required_entries(
    mapping: dict,
    only: str | None = None,
    order: str = "leaves-first",
) -> list[dict]:
    """Return required_missing entries.

    `order`:
      * `leaves-first` (default) — sort by C call-graph height
        (`xlate/call_graph.json`'s `height_of`), lowest first, so
        callees land before callers.  Falls back to `c_id` for names
        not in the graph.
      * `original` — keep the map's order.
    """
    out = [
        e for e in mapping.get("entries", [])
        if e.get("required_action") == "required_missing"
    ]
    if only:
        out = [
            e for e in out
            if e.get("c_id") == only or e.get("c", {}).get("name") == only
        ]
    if order == "leaves-first":
        heights = _load_callgraph_heights()
        out.sort(key=lambda e: _sort_key_leaves_first(heights, e))
    return out


def prompt_path(batch: list[dict]) -> Path:
    label = "__".join(e["c"]["name"] for e in batch[:3])
    if len(batch) > 3:
        label += f"__plus_{len(batch) - 3}"
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in label)
    return PROMPTS_DIR / f"{safe}.md"


def log_path(agent: agent_runner.AgentConfig, batch: list[dict]) -> Path:
    label = prompt_path(batch).stem
    return agent_runner.log_dir(REPO_ROOT, agent, "phase4b") / f"{label}.log"


def compose_prompt(batch: list[dict]) -> str:
    sections: list[str] = []
    for e in batch:
        c = e["c"]
        plan = e.get("phase4a_plan") or {}
        sections.append(
            f"## `{e['c_id']}`\n"
            f"* C source: `{c['file']}:{c['start_line']}-{c['end_line']}`\n"
            f"* C signature: `{c.get('signature', '')}`\n"
            f"* Approved Rust destination: `{plan.get('rust_destination', '')}`\n"
            f"* Item shape: {plan.get('item_shape', '')}\n"
            f"* Verification: {plan.get('verification', '')}\n"
            f"* Reason: {e.get('reason', '')}\n"
        )
    return "\n".join([
        "# Phase 4B missing implementation batch",
        "",
        "## NO SHORTCUTS — do the hard work",
        "",
        "Phase 4B implements approved required_missing items.  Past sweeps",
        "produced hundreds of stubs that compile but don't translate the",
        "C body — that is failure, not completion.  The gate flags all",
        "of these patterns identically:",
        "",
        "* `todo!()`",
        "* `unimplemented!()`",
        "* `// SKIP:` or any placeholder marker (None / Err(Generic) /",
        "  Ok(()) returns plus an excuse comment) — forbidden",
        "* Fabricated default returns (0, false, None, empty Vec) when",
        "  the C body computes a real value — forbidden",
        "* Half-translated bodies that punt error paths to `todo!()` —",
        "  forbidden",
        "",
        "**Nothing in picoquic is fundamentally untranslatable.**  Every",
        "function has a body — translate it.  Concrete mappings:",
        "  * `pthread_create` → `std::thread::spawn`",
        "  * `pipe()` wake-up → `std::sync::mpsc` or `Condvar`",
        "  * `select`/`poll`/`io_uring` → `mio` crate",
        "  * loglib helpers → `log` crate + standard file I/O",
        "  * borrow-checker issues → `&mut self`, `RefCell`, restructure",
        "",
        "**Forbidden 'blocker' excuses** (every one is a shortcut):",
        "'out of scope', 'deferred to a later pass', 'multi-threading",
        "not in scope', 'loglib not in scope', 'sub-system not yet",
        "translated', 'needs design thought', 'borrow-checker issue'.",
        "",
        "The ONLY acceptable bare `todo!()` is a concrete external",
        "dependency outside our reach (e.g. 'blocked: needs picotls API",
        "not yet exposed in the Rust binding') — and even then, prefer",
        "translating the dependency first.",
        "",
        "## Implementation rules",
        "",
        "Implement the approved missing Rust functions/items listed below.",
        "Follow `CLAUDE.md`, `TRANSLATE_PLAN.md`, and",
        "`xlate/impl_translation_guide.md`.",
        "",
        "* Translate the C behavior faithfully into safe, idiomatic Rust.",
        "* Keep the approved module structure unless implementation proves",
        "  it wrong; if it is wrong, stop and report the needed Phase 4A",
        "  plan amendment.",
        "* Add `/// C: ` references for implemented functions so Phase 4A",
        "  can map them on the next refresh.",
        "* Edit only `rs/fq/`, `scripts/`, `xlate/`, or markdown files",
        "  allowed by the repository instructions.",
        "",
        "## Verification — run ONLY these cargo commands",
        "",
        "* **`cargo test --no-run`** — compile-only.  Use this as your",
        "  build gate.  Do NOT run `cargo test` (full suite) — it will",
        "  run all 505 tests and take 2+ hours.",
        "* **`cargo test <specific_test_name>`** — running a single",
        "  named test for narrow verification is fine.",
        "* **`cargo fmt`** and **`cargo clippy --tests --all-features",
        "  -- -D warnings`** — final polish.",
        "* **Do NOT prefix with `CARGO_INCREMENTAL=0`** — the allowlist",
        "  blocks env-var prefixes; just run `cargo ...` directly.",
        "",
        "Run these after implementing:",
        "",
        "```sh",
        "cargo fmt",
        "cargo test --no-run",
        "cargo clippy --tests --all-features -- -D warnings",
        "```",
        "",
        "\n".join(sections),
    ])


def run_gate(skip_gate: bool) -> int:
    if skip_gate:
        return 0
    commands = [
        ["cargo", "fmt"],
        ["cargo", "test", "--no-run"],
        ["cargo", "clippy", "--tests", "--all-features", "--", "-D", "warnings"],
    ]
    env = os.environ.copy()
    env["CARGO_INCREMENTAL"] = "0"
    for cmd in commands:
        res = subprocess.run(cmd, cwd=RS_CRATE, env=env)
        if res.returncode != 0:
            return res.returncode
    return 0


def refresh_map() -> dict:
    refreshed = build_function_map(load_function_map())
    save_json(FUNCTION_MAP, refreshed)
    return refreshed


def print_status(mapping: dict) -> None:
    pending = required_entries(mapping)
    print(f"required_missing: {len(pending)}")
    for e in pending[:30]:
        plan = e.get("phase4a_plan") or {}
        print(f"  {e['c_id']} -> {plan.get('rust_destination', '?')}")
    if len(pending) > 30:
        print(f"  ... and {len(pending) - 30} more")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    agent_runner.add_agent_args(parser, model_help_context="Phase 4B implementation agent")
    parser.add_argument("--status", action="store_true", help="show pending required_missing entries")
    parser.add_argument("--dry-run", action="store_true", help="print selected entries without invoking an agent")
    parser.add_argument("--approved", action="store_true", help="run even if phase4a_plan.md is not marked approved")
    parser.add_argument("--only", help="limit to one c_id or C function name")
    parser.add_argument("--limit", type=int, default=None, help="maximum entries to process")
    parser.add_argument("--batch-size", type=int, default=1, help="entries per agent invocation")
    parser.add_argument("--max-turns", type=int, default=250)
    parser.add_argument("--skip-gate", action="store_true", help="skip cargo fmt/test/clippy gate after each batch")
    parser.add_argument("--order", choices=("leaves-first", "original"), default="leaves-first",
                        help="entry order: leaves-first (default, by call-graph height) or original (map order)")
    args = parser.parse_args()

    mapping = load_function_map()
    if args.status:
        if not mapping.get("entries"):
            print("no function map found; run `python3 scripts/phase4a.py` first")
            return 0
        print_status(mapping)
        return 0

    if not mapping.get("entries"):
        print("no function map found; run `python3 scripts/phase4a.py` first")
        return 2

    if not args.approved and not is_plan_approved():
        print("Phase 4A plan is not marked approved. Edit xlate/phase4a_plan.md to `Status: approved` or pass --approved.")
        return 2

    selected = required_entries(mapping, args.only, order=args.order)
    if args.limit is not None:
        selected = selected[: args.limit]
    print(f"selected required_missing entries: {len(selected)} (order={args.order})")
    if args.dry_run:
        for e in selected:
            print(f"  {e['c_id']} -> {(e.get('phase4a_plan') or {}).get('rust_destination', '?')}")
        return 0

    agent = agent_runner.config_from_args(args)
    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
    for i in range(0, len(selected), max(1, args.batch_size)):
        batch = selected[i:i + max(1, args.batch_size)]
        prompt = compose_prompt(batch)
        pfile = prompt_path(batch)
        pfile.write_text(prompt)
        print(f"[{i + 1}/{len(selected)}] implementing {', '.join(e['c']['name'] for e in batch)}")
        res = agent_runner.run_stream(
            agent,
            prompt,
            repo_root=REPO_ROOT,
            log_path=log_path(agent, batch),
            phase="phase4b",
            label=pfile.stem,
            prompt_file=pfile,
            allowed_tools=ALLOWED_TOOLS,
            max_turns=args.max_turns,
        )
        if res.returncode != 0:
            print(f"agent failed with exit {res.returncode}")
            return res.returncode
        gate = run_gate(args.skip_gate)
        if gate != 0:
            print(f"gate failed with exit {gate}")
            return gate
        mapping = refresh_map()
        remaining = required_entries(mapping)
        print(f"map refreshed at {time.strftime('%Y-%m-%dT%H:%M:%S')}; required_missing remaining: {len(remaining)}")
    if args.limit is None and args.only is None:
        remaining = required_entries(load_function_map())
        if remaining:
            print(f"required_missing remaining after Phase 4B run: {len(remaining)}")
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
