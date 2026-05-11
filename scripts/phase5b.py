#!/usr/bin/env python3
"""phase5b.py -- repair Phase 5A test correspondence mismatches.

Phase 5B consumes Phase 5A `needs_fix` entries and edits Rust tests,
fixtures, or test helpers so the Rust tests line up with the C tests.
It uses file-owned buckets so parallel workers do not edit the same
primary test file.

Outputs:
  - xlate/phase5b_repairs.json
  - xlate/phase5b_report.html
  - updates xlate/phase5a_reviews.json outcomes for repaired entries
  - refreshes xlate/test_translation_map.json after Rust test edits
"""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import re
import subprocess
from collections import Counter, defaultdict
from pathlib import Path

import agent_runner
import phase5a
from phase4_common import REPO_ROOT, RS_CRATE, esc, load_json, save_json
from phase5_common import (
    TEST_MAP,
    XLATE,
    build_test_map,
    load_test_map,
    source_body,
    test_entry_by_id,
    write_html,
)

try:
    import fcntl
except ImportError:  # pragma: no cover - translation tooling runs on POSIX.
    fcntl = None

REPAIRS = XLATE / "phase5b_repairs.json"
REPAIRS_LOCK = XLATE / "phase5b_repairs.lock"
REPORT = XLATE / "phase5b_report.html"
PROMPTS_DIR = XLATE / "prompts" / "phase5b"
PHASE5C_RESULTS = XLATE / "phase5c_revalidation.json"

OUTCOMES = {"fixed", "ok", "blocked"}
ALLOWED_TOOLS = (
    "Read Edit Write Glob Grep "
    "Bash(rg:*) "
    "Bash(git diff:*) "
    "Bash(git status:*) "
    "Bash(cargo check:*) "
    "Bash(cargo fmt:*) "
    "Bash(cargo clippy:*) "
    "Bash(python3 scripts/phase3_check.py:*)"
)


@contextlib.contextmanager
def repairs_file_lock():
    REPAIRS_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with REPAIRS_LOCK.open("a") as lock_file:
        if fcntl is not None:
            fcntl.flock(lock_file, fcntl.LOCK_EX)
        try:
            yield
        finally:
            if fcntl is not None:
                fcntl.flock(lock_file, fcntl.LOCK_UN)


def load_repairs() -> dict:
    with repairs_file_lock():
        repairs = load_json(REPAIRS, {"schema_version": 1, "repairs": {}})
        repairs.setdefault("repairs", {})
        return repairs


def update_repairs(updates: dict[str, dict], *, force: bool) -> dict:
    with repairs_file_lock():
        repairs = load_json(REPAIRS, {"schema_version": 1, "repairs": {}})
        repair_map = repairs.setdefault("repairs", {})
        for test_id, repair in updates.items():
            if force or test_id not in repair_map:
                repair_map[test_id] = repair
        tmp = REPAIRS.with_name(f"{REPAIRS.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(repairs, indent=2, sort_keys=True) + "\n")
        tmp.replace(REPAIRS)
        return repairs


def needs_fix_entries(
    mapping: dict,
    reviews: dict,
    repairs: dict,
    *,
    only: str | None,
    rust_file: str | None,
    force: bool,
) -> list[dict]:
    entries_by_id = test_entry_by_id(mapping)
    review_map = reviews.get("reviews", {})
    repair_map = repairs.get("repairs", {})
    out: list[dict] = []
    for test_id, review in review_map.items():
        if review.get("outcome") != "needs_fix":
            continue
        entry = entries_by_id.get(test_id)
        if not entry:
            continue
        if only and only not in {
            test_id,
            entry.get("test_name"),
            entry.get("entry_fn"),
            entry.get("rust_test_name"),
        }:
            continue
        if rust_file and entry.get("expected_rust_file") != rust_file:
            continue
        if not force and test_id in repair_map:
            continue
        item = dict(entry)
        item["phase5a_review"] = review
        out.append(item)
    out.sort(key=lambda e: (e.get("expected_rust_file", ""), e.get("rust_test_name", "")))
    return out


def phase5c_deficiency_entries(
    mapping: dict,
    reviews: dict,
    repairs: dict,
    phase5c_results: dict,
    *,
    only: str | None,
    rust_file: str | None,
    force: bool,
) -> list[dict]:
    entries_by_id = test_entry_by_id(mapping)
    review_map = reviews.get("reviews", {})
    repair_map = repairs.get("repairs", {})
    out: list[dict] = []
    for test_id, result in phase5c_results.get("results", {}).items():
        if result.get("outcome") not in {"needs_fix", "blocked"}:
            continue
        entry = entries_by_id.get(test_id)
        if not entry:
            continue
        if only and only not in {
            test_id,
            entry.get("test_name"),
            entry.get("entry_fn"),
            entry.get("rust_test_name"),
        }:
            continue
        if rust_file and entry.get("expected_rust_file") != rust_file:
            continue
        prior = repair_map.get(test_id)
        if not force and prior and prior.get("outcome") in {"fixed", "ok"}:
            continue
        item = dict(entry)
        item["phase5a_review"] = review_map.get(test_id, {})
        item["phase5c_result"] = result
        out.append(item)
    out.sort(key=lambda e: (e.get("expected_rust_file", ""), e.get("rust_test_name", "")))
    return out


def blocked_entries(
    mapping: dict,
    reviews: dict,
    repairs: dict,
    *,
    only: str | None,
    rust_file: str | None,
) -> list[dict]:
    entries_by_id = test_entry_by_id(mapping)
    review_map = reviews.get("reviews", {})
    out: list[dict] = []
    for test_id, repair in repairs.get("repairs", {}).items():
        if repair.get("outcome") != "blocked":
            continue
        entry = entries_by_id.get(test_id)
        if not entry:
            continue
        if only and only not in {
            test_id,
            entry.get("test_name"),
            entry.get("entry_fn"),
            entry.get("rust_test_name"),
        }:
            continue
        if rust_file and entry.get("expected_rust_file") != rust_file:
            continue
        item = dict(entry)
        item["phase5a_review"] = review_map.get(test_id, {})
        item["phase5b_prior_repair"] = repair
        out.append(item)
    out.sort(key=lambda e: (e.get("expected_rust_file", ""), e.get("rust_test_name", "")))
    return out


def load_id_list(path: str) -> list[str]:
    ids: list[str] = []
    for raw_line in Path(path).read_text().splitlines():
        line = raw_line.split("#", 1)[0].strip()
        if line:
            ids.append(line)
    return ids


def apply_id_list(entries: list[dict], ids: list[str]) -> list[dict]:
    rank = {item: index for index, item in enumerate(ids)}

    def entry_rank(entry: dict) -> int | None:
        candidates = (
            entry.get("test_id"),
            entry.get("test_name"),
            entry.get("entry_fn"),
            entry.get("rust_test_name"),
        )
        matches = [rank[candidate] for candidate in candidates if candidate in rank]
        return min(matches) if matches else None

    selected: list[tuple[int, dict]] = []
    for entry in entries:
        index = entry_rank(entry)
        if index is not None:
            selected.append((index, entry))
    selected.sort(key=lambda item: item[0])
    return [entry for _, entry in selected]


def apply_file_bucket(entries: list[dict], *, bucket_count: int, bucket_index: int) -> list[dict]:
    if bucket_count == 1:
        return entries
    by_file: dict[str, list[dict]] = defaultdict(list)
    for entry in entries:
        by_file[entry["expected_rust_file"]].append(entry)

    buckets: list[tuple[int, list[str]]] = [(0, []) for _ in range(bucket_count)]
    for file, group in sorted(by_file.items(), key=lambda item: (-len(item[1]), item[0])):
        idx, (load, files) = min(enumerate(buckets), key=lambda item: (item[1][0], item[1][1]))
        files = [*files, file]
        buckets[idx] = (load + len(group), files)

    selected_files = set(buckets[bucket_index][1])
    return [entry for entry in entries if entry["expected_rust_file"] in selected_files]


def prompt_path(batch: list[dict]) -> Path:
    label = "__".join(entry["entry_fn"] for entry in batch[:3])
    if len(batch) > 3:
        label += f"__plus_{len(batch) - 3}"
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in label)
    return PROMPTS_DIR / f"{safe}.md"


def log_path(agent: agent_runner.AgentConfig, batch: list[dict]) -> Path:
    return agent_runner.log_dir(REPO_ROOT, agent, "phase5b") / f"{prompt_path(batch).stem}.log"


def compose_prompt(batch: list[dict]) -> str:
    owned_files = sorted({entry["expected_rust_file"] for entry in batch})
    sections: list[str] = []
    for entry in batch:
        c = entry.get("c") or {}
        rust = entry.get("rust") or {}
        review = entry["phase5a_review"]
        phase5c_result = entry.get("phase5c_result", {})
        rust_span = (
            f"{rust.get('file')}:{rust.get('start_line')}-{rust.get('end_line')}"
            if rust else f"{entry.get('expected_rust_file')}:missing"
        )
        sections.append(
            "\n".join(
                [
                    f"## `{entry['test_id']}`",
                    f"* C test-table name: `{entry.get('test_name')}`",
                    f"* C entry function: `{entry.get('entry_fn')}`",
                    f"* Rust test: `{entry.get('rust_test_name')}`",
                    f"* Expected Rust file: `{entry.get('expected_rust_file')}`",
                    f"* Rust span: `{rust_span}`",
                    f"* Phase 5A analysis: {review.get('analysis', '')}",
                    f"* Phase 5A fix note: {review.get('fix_summary', '')}",
                    f"* Phase 5C outcome: {phase5c_result.get('outcome', '')}",
                    f"* Phase 5C analysis: {phase5c_result.get('analysis', '')}",
                    f"* Phase 5C fix note: {phase5c_result.get('fix_summary', '')}",
                    "",
                    "### C test body",
                    "```c",
                    source_body(c),
                    "```",
                    "",
                    "### Current Rust test body",
                    "```rust",
                    source_body(rust) if rust else "",
                    "```",
                    "",
                ]
            )
        )

    return "\n".join(
        [
            "# Phase 5B repair Rust test mismatches",
            "",
            "You are repairing Phase 5A `needs_fix` entries.  The goal",
            "is to make the Rust tests faithfully check the same behavior",
            "as the C tests.",
            "",
            "Rules:",
            "",
            "* Edit Rust tests, Rust test helpers, and Rust test fixtures",
            "  only: `rs/fq/src/tests/` and `rs/fq/tests/fixtures/`.",
            "* Do not edit C sources.",
            "* Do not weaken assertions, skip important C cases, or replace",
            "  tests with placeholders.",
            "* If the test already matches after closer inspection, report",
            "  `ok` and do not edit source.",
            "* Phase 5B is about test/API correspondence, not test success.",
            "  The Rust test must exist, compile as a test, and be runnable",
            "  by the Rust test harness, but it may fail arbitrarily early",
            "  because the Rust library implementation is incomplete.",
            "* Do not report `blocked` merely because the implementation",
            "  returns the wrong state, fails a handshake, lacks protocol",
            "  behavior, or would fail the test. Those are Phase 5C issues.",
            "* Report `blocked` only when the faithful test cannot be",
            "  written, compiled, or exposed as a runnable Rust test because",
            "  the necessary Rust API/test-harness surface is missing or",
            "  ambiguous.",
            "* Do not run full `cargo test` in this pass. Use source review",
            "  and, if needed, `cargo check --tests` for compile validation.",
            "",
            f"Owned Rust test file(s): {', '.join(f'`{f}`' for f in owned_files)}",
            "",
            "Return final JSON with this shape:",
            "",
            "```json",
            "{\"repairs\":[{\"test_id\":\"...\",\"outcome\":\"fixed|ok|blocked\","
            "\"analysis\":\"short repair conclusion\","
            "\"fix_summary\":\"what changed, or empty\","
            "\"files_changed\":[\"rs/fq/src/tests/...\"],"
            "\"verification\":[\"cargo ...\"]}]}",
            "```",
            "",
            "Entries:",
            "",
            "\n".join(sections),
        ]
    )


def compose_reclassify_prompt(batch: list[dict]) -> str:
    owned_files = sorted({entry["expected_rust_file"] for entry in batch})
    sections: list[str] = []
    for entry in batch:
        c = entry.get("c") or {}
        rust = entry.get("rust") or {}
        review = entry["phase5a_review"]
        prior = entry.get("phase5b_prior_repair", {})
        rust_span = (
            f"{rust.get('file')}:{rust.get('start_line')}-{rust.get('end_line')}"
            if rust else f"{entry.get('expected_rust_file')}:missing"
        )
        sections.append(
            "\n".join(
                [
                    f"## `{entry['test_id']}`",
                    f"* C test-table name: `{entry.get('test_name')}`",
                    f"* C entry function: `{entry.get('entry_fn')}`",
                    f"* Rust test: `{entry.get('rust_test_name')}`",
                    f"* Expected Rust file: `{entry.get('expected_rust_file')}`",
                    f"* Rust span: `{rust_span}`",
                    f"* Phase 5A analysis: {review.get('analysis', '')}",
                    f"* Prior Phase 5B blocked analysis: {prior.get('analysis', '')}",
                    f"* Prior Phase 5B fix note: {prior.get('fix_summary', '')}",
                    "",
                    "### C test body",
                    "```c",
                    source_body(c),
                    "```",
                    "",
                    "### Current Rust test body",
                    "```rust",
                    source_body(rust) if rust else "",
                    "```",
                    "",
                ]
            )
        )

    return "\n".join(
        [
            "# Phase 5B reclassify blocked Rust tests",
            "",
            "You are revisiting Phase 5B entries previously marked",
            "`blocked`. The prior pass used the wrong standard: runtime",
            "library failures were sometimes recorded as Phase 5B blocks.",
            "",
            "Correct Phase 5B standard:",
            "",
            "* Phase 5B is about the Rust test matching the C test's API",
            "  calls and API-visible assertions.",
            "* The Rust test must be present, compile as a Rust test, and",
            "  be runnable by the Rust test harness.",
            "* The Rust test does not need to pass. It may fail arbitrarily",
            "  early because the Rust library implementation is incomplete;",
            "  that belongs to Phase 5C.",
            "* If the current Rust test already expresses the same API-level",
            "  contract as C, report `ok`, even if it would fail at runtime.",
            "* If the test needs edits to express the same API-level contract",
            "  and to compile/run as a test, make those edits and report",
            "  `fixed`.",
            "* Report `blocked` only if the necessary Rust API or test-harness",
            "  surface is missing/ambiguous such that a faithful compiling",
            "  runnable test cannot be written without an API/design change.",
            "* Do not report `blocked` for incomplete handshake behavior,",
            "  missing protocol side effects, wrong state transitions,",
            "  wrong error codes, callback counters not updating, or other",
            "  library behavior failures. Mention those as Phase 5C notes",
            "  in `analysis` while reporting `ok` or `fixed`.",
            "* Do not run full `cargo test` in this pass. Use source review",
            "  and, if needed, `cargo check --tests` for compile validation.",
            "",
            f"Owned Rust test file(s): {', '.join(f'`{f}`' for f in owned_files)}",
            "",
            "Return final JSON with this shape:",
            "",
            "```json",
            "{\"repairs\":[{\"test_id\":\"...\",\"outcome\":\"fixed|ok|blocked\","
            "\"analysis\":\"short reclassification conclusion\","
            "\"fix_summary\":\"what changed, or empty\","
            "\"files_changed\":[\"rs/fq/src/tests/...\"],"
            "\"verification\":[\"cargo check --tests\", \"source review\"]}]}",
            "```",
            "",
            "Entries:",
            "",
            "\n".join(sections),
        ]
    )


def extract_json(text: str) -> dict:
    candidates: list[dict] = []
    for fenced in re.finditer(r"```(?:json)?\s*(\{.*?\})\s*```", text, re.DOTALL):
        try:
            obj = json.loads(fenced.group(1))
        except json.JSONDecodeError:
            continue
        if isinstance(obj, dict) and "repairs" in obj:
            candidates.append(obj)

    decoder = json.JSONDecoder()
    for match in re.finditer(r"\{", text):
        try:
            obj, _ = decoder.raw_decode(text[match.start():])
        except json.JSONDecodeError:
            continue
        if isinstance(obj, dict) and "repairs" in obj:
            candidates.append(obj)
    if candidates:
        return candidates[-1]
    raise ValueError("no JSON object found")


def normalize_repairs(raw: dict, batch: list[dict]) -> dict[str, dict]:
    def string_list(value: object) -> list[str]:
        if not isinstance(value, list):
            return []
        return [str(item)[:300] for item in value[:20]]

    batch_ids = {entry["test_id"] for entry in batch}
    aliases: dict[str, str] = {}
    for entry in batch:
        test_id = entry["test_id"]
        for key in ("test_id", "test_name", "entry_fn", "rust_test_name"):
            value = entry.get(key)
            if value:
                aliases[str(value)] = test_id
        if ":" in test_id:
            aliases[test_id.rsplit(":", 1)[1]] = test_id
    by_id: dict[str, dict] = {}
    for item in raw.get("repairs", []):
        test_id = aliases.get(str(item.get("test_id")), item.get("test_id"))
        outcome = item.get("outcome")
        if test_id not in batch_ids or outcome not in OUTCOMES:
            continue
        by_id[test_id] = {
            "test_id": test_id,
            "previous_phase5a_outcome": "needs_fix",
            "outcome": outcome,
            "analysis": str(item.get("analysis", ""))[:2000],
            "fix_summary": str(item.get("fix_summary", ""))[:2000],
            "files_changed": string_list(item.get("files_changed")),
            "verification": string_list(item.get("verification")),
        }
    for entry in batch:
        if entry["test_id"] not in by_id:
            by_id[entry["test_id"]] = {
                "test_id": entry["test_id"],
                "previous_phase5a_outcome": "needs_fix",
                "outcome": "blocked",
                "analysis": "agent response did not include this test",
                "fix_summary": "",
                "files_changed": [],
                "verification": [],
            }
    return by_id


def phase5a_updates(repairs: dict[str, dict]) -> dict[str, dict]:
    updates: dict[str, dict] = {}
    for test_id, repair in repairs.items():
        updates[test_id] = {
            "test_id": test_id,
            "outcome": repair["outcome"],
            "analysis": repair.get("analysis", ""),
            "fix_summary": repair.get("fix_summary", ""),
            "verification": repair.get("verification", []),
        }
    return updates


def rs_diff_names() -> set[str]:
    res = subprocess.run(
        ["git", "diff", "--name-only", "--", "rs/fq"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    if res.returncode != 0:
        return set()
    return {line.strip() for line in res.stdout.splitlines() if line.strip()}


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
    env.setdefault("CARGO_TARGET_DIR", "/private/tmp/fq-target")
    for cmd in commands:
        res = subprocess.run(cmd, cwd=RS_CRATE, env=env)
        if res.returncode != 0:
            return res.returncode
    return 0


def refresh_test_map() -> dict:
    mapping = build_test_map()
    save_json(TEST_MAP, mapping)
    return mapping


def write_report(mapping: dict, reviews: dict, repairs: dict) -> None:
    review_map = reviews.get("reviews", {})
    repair_map = repairs.get("repairs", {})
    remaining = sum(1 for review in review_map.values() if review.get("outcome") == "needs_fix")
    counts = Counter(repair.get("outcome", "unknown") for repair in repair_map.values())
    entries_by_id = test_entry_by_id(mapping)
    body: list[str] = [
        "<h1>Phase 5B Test Repairs</h1>",
        "<h2>Summary</h2>",
        "<table><tr><th>Outcome</th><th>Count</th></tr>",
    ]
    for outcome in ("fixed", "ok", "blocked"):
        body.append(f"<tr><td>{esc(outcome)}</td><td>{counts.get(outcome, 0)}</td></tr>")
    body.append(f"<tr><td>remaining needs_fix</td><td>{remaining}</td></tr>")
    body.append("</table>")

    for section, title in (
        ("blocked", "Blocked"),
        ("fixed", "Fixed"),
        ("ok", "Confirmed OK"),
    ):
        body.append(f"<h2>{esc(title)}</h2>")
        body.append(
            "<table><tr><th>Test</th><th>Rust file</th>"
            "<th>Analysis</th><th>Fix</th></tr>"
        )
        rows = [
            (test_id, repair) for test_id, repair in repair_map.items()
            if repair.get("outcome") == section
        ]
        if not rows:
            body.append("<tr><td colspan=\"4\"><em>none</em></td></tr>")
        for test_id, repair in sorted(rows):
            entry = entries_by_id.get(test_id, {})
            body.append(
                "<tr>"
                f"<td><code>{esc(entry.get('test_name', test_id))}</code><br>{esc(entry.get('entry_fn', ''))}</td>"
                f"<td>{esc(entry.get('expected_rust_file', ''))}</td>"
                f"<td>{esc(repair.get('analysis', ''))}</td>"
                f"<td>{esc(repair.get('fix_summary', ''))}</td>"
                "</tr>"
            )
        body.append("</table>")
    write_html(REPORT, "Phase 5B Test Repairs", "\n".join(body))


def print_status(reviews: dict, repairs: dict) -> None:
    review_counts = Counter(
        review.get("outcome", "unknown")
        for review in reviews.get("reviews", {}).values()
    )
    repair_counts = Counter(
        repair.get("outcome", "unknown")
        for repair in repairs.get("repairs", {}).values()
    )
    print(f"phase5a needs_fix remaining: {review_counts.get('needs_fix', 0)}")
    print("phase5b repairs:")
    for outcome in ("fixed", "ok", "blocked"):
        print(f"  {outcome:8s} {repair_counts.get(outcome, 0)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    agent_runner.add_agent_args(parser, model_help_context="Phase 5B test repair agent")
    parser.add_argument("--status", action="store_true", help="show Phase 5B repair progress")
    parser.add_argument("--dry-run", action="store_true", help="print selected tests without invoking an agent")
    parser.add_argument("--reclassify-blocked", action="store_true",
                        help="revisit prior blocked results using the narrowed Phase 5B standard")
    parser.add_argument("--from-phase5c", action="store_true",
                        help="repair Phase 5C needs_fix/blocked revalidation results")
    parser.add_argument("--force", action="store_true", help="re-run tests with existing Phase 5B results")
    parser.add_argument("--only", help="limit to one test_id, C entry function, C test name, or Rust test name")
    parser.add_argument("--rust-file", help="limit to one expected Rust test file")
    parser.add_argument("--id-list", help="limit to newline-delimited test ids/names")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--batch-size", type=int, default=1)
    parser.add_argument("--bucket-count", type=int, default=1)
    parser.add_argument("--bucket-index", type=int, default=0)
    parser.add_argument("--max-turns", type=int, default=250)
    parser.add_argument("--skip-gate", action="store_true", help="skip cargo gates after a batch")
    args = parser.parse_args()
    if args.bucket_count < 1:
        parser.error("--bucket-count must be at least 1")
    if args.bucket_index < 0 or args.bucket_index >= args.bucket_count:
        parser.error("--bucket-index must satisfy 0 <= index < bucket-count")

    mapping = load_test_map(refresh=True)
    reviews = phase5a.load_reviews()
    repairs = load_repairs()

    if args.status:
        print_status(reviews, repairs)
        return 0

    if args.from_phase5c:
        selected = phase5c_deficiency_entries(
            mapping,
            reviews,
            repairs,
            load_json(PHASE5C_RESULTS, {"results": {}}),
            only=args.only,
            rust_file=args.rust_file,
            force=args.force,
        )
    elif args.reclassify_blocked:
        selected = blocked_entries(
            mapping,
            reviews,
            repairs,
            only=args.only,
            rust_file=args.rust_file,
        )
    else:
        selected = needs_fix_entries(
            mapping,
            reviews,
            repairs,
            only=args.only,
            rust_file=args.rust_file,
            force=args.force,
        )
    if args.id_list:
        selected = apply_id_list(selected, load_id_list(args.id_list))
    selected = apply_file_bucket(
        selected,
        bucket_count=args.bucket_count,
        bucket_index=args.bucket_index,
    )
    if args.limit is not None:
        selected = selected[: args.limit]

    bucket = ""
    if args.bucket_count > 1:
        bucket = f" for bucket {args.bucket_index}/{args.bucket_count}"
    mode_label = "blocked tests for reclassification" if args.reclassify_blocked else "needs_fix tests"
    print(f"selected Phase 5B {mode_label}{bucket}: {len(selected)}")
    if args.dry_run:
        for entry in selected[:80]:
            print(
                f"  {entry['expected_rust_file']}: "
                f"{entry['entry_fn']} -> {entry['rust_test_name']}"
            )
        if len(selected) > 80:
            print(f"  ... and {len(selected) - 80} more")
        return 0

    agent = agent_runner.config_from_args(args)
    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
    processed = 0
    batch_size = max(1, args.batch_size)
    while processed < len(selected):
        batch = selected[processed:processed + batch_size]
        prompt = compose_reclassify_prompt(batch) if args.reclassify_blocked else compose_prompt(batch)
        pfile = prompt_path(batch)
        pfile.write_text(prompt)
        before_rs = rs_diff_names()
        print(f"[{processed + 1}/{len(selected)}] repairing {len(batch)} test(s)")
        res = agent_runner.run_capture(
            agent,
            prompt,
            repo_root=REPO_ROOT,
            log_path=log_path(agent, batch),
            phase="phase5b",
            label=pfile.stem,
            prompt_file=pfile,
            allowed_tools=ALLOWED_TOOLS,
            max_turns=args.max_turns,
        )
        if res.returncode != 0:
            print(f"agent failed with exit {res.returncode}")
            return res.returncode

        try:
            parsed = extract_json(res.stdout + "\n" + res.stderr)
            updates = normalize_repairs(parsed, batch)
        except (ValueError, json.JSONDecodeError) as exc:
            updates = {
                entry["test_id"]: {
                    "test_id": entry["test_id"],
                    "previous_phase5a_outcome": "needs_fix",
                    "outcome": "blocked",
                    "analysis": f"could not parse agent JSON: {exc}",
                    "fix_summary": "",
                    "files_changed": [],
                    "verification": [],
                }
                for entry in batch
            }

        repairs = update_repairs(updates, force=args.force or args.reclassify_blocked)
        phase5a.update_reviews(phase5a_updates(updates), force=True)
        processed += len(batch)

        after_rs = rs_diff_names()
        if after_rs != before_rs or any(
            repair.get("outcome") == "fixed" for repair in updates.values()
        ):
            gate = run_gate(args.skip_gate)
            if gate != 0:
                print(f"gate failed with exit {gate}")
                return gate
            mapping = refresh_test_map()

    reviews = phase5a.load_reviews()
    repairs = load_repairs()
    write_report(mapping, reviews, repairs)
    print_status(reviews, repairs)
    print(f"wrote: {REPAIRS.relative_to(REPO_ROOT)}")
    print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
