#!/usr/bin/env python3
"""phase5a.py -- audit C/Rust test correspondence.

Phase 5A builds a map from picoquic C test-table entries to Rust
`#[test]` functions, then asks an agent to do a read-only,
context-aware correspondence review.  It classifies each test as:

  - ok: the Rust test is a faithful translation of the C test intent
  - needs_fix: the Rust test is missing or materially mismatched
  - blocked: the review needs a concrete external decision

Outputs:
  - xlate/test_translation_map.json
  - xlate/phase5a_reviews.json
  - xlate/phase5a_report.html
"""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import re
import subprocess
from collections import Counter
from pathlib import Path

import agent_runner
from phase4_common import REPO_ROOT, esc, load_json
from phase5_common import (
    TEST_MAP,
    XLATE,
    load_test_map,
    source_body,
    write_html,
)

try:
    import fcntl
except ImportError:  # pragma: no cover - translation tooling runs on POSIX.
    fcntl = None

REVIEWS = XLATE / "phase5a_reviews.json"
REVIEWS_LOCK = XLATE / "phase5a_reviews.lock"
REPORT = XLATE / "phase5a_report.html"
PROMPTS_DIR = XLATE / "prompts" / "phase5a"

OUTCOMES = {"ok", "needs_fix", "fixed", "blocked"}
PLACEHOLDER_ANALYSIS = "agent response did not include this test"
ALLOWED_TOOLS = (
    "Read Glob Grep "
    "Bash(rg:*) "
    "Bash(git diff:*) "
    "Bash(git status:*)"
)


@contextlib.contextmanager
def reviews_file_lock():
    REVIEWS_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with REVIEWS_LOCK.open("a") as lock_file:
        if fcntl is not None:
            fcntl.flock(lock_file, fcntl.LOCK_EX)
        try:
            yield
        finally:
            if fcntl is not None:
                fcntl.flock(lock_file, fcntl.LOCK_UN)


def load_reviews() -> dict:
    with reviews_file_lock():
        reviews = load_json(REVIEWS, {"schema_version": 1, "reviews": {}})
        reviews.setdefault("reviews", {})
        return reviews


def update_reviews(updates: dict[str, dict], *, force: bool) -> dict:
    with reviews_file_lock():
        reviews = load_json(REVIEWS, {"schema_version": 1, "reviews": {}})
        review_map = reviews.setdefault("reviews", {})
        for test_id, review in updates.items():
            if force or test_id not in review_map:
                review_map[test_id] = review
        tmp = REVIEWS.with_name(f"{REVIEWS.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(reviews, indent=2, sort_keys=True) + "\n")
        tmp.replace(REVIEWS)
        return reviews


def phase5a_log_paths() -> list[Path]:
    paths: list[Path] = []
    for provider in agent_runner.PROVIDERS:
        log_dir = XLATE / f"{provider}_logs" / "phase5a"
        if log_dir.is_dir():
            paths.extend(sorted(log_dir.glob("*.log")))
    return sorted(paths)


def clear_placeholder_reviews() -> tuple[dict, int]:
    with reviews_file_lock():
        reviews = load_json(REVIEWS, {"schema_version": 1, "reviews": {}})
        review_map = reviews.setdefault("reviews", {})
        stale_ids = [
            test_id
            for test_id, review in review_map.items()
            if (
                isinstance(review, dict)
                and review.get("outcome") == "blocked"
                and review.get("analysis") == PLACEHOLDER_ANALYSIS
            )
        ]
        for test_id in stale_ids:
            del review_map[test_id]
        if stale_ids:
            tmp = REVIEWS.with_name(f"{REVIEWS.name}.{os.getpid()}.tmp")
            tmp.write_text(json.dumps(reviews, indent=2, sort_keys=True) + "\n")
            tmp.replace(REVIEWS)
        return reviews, len(stale_ids)


def prune_stale_reviews(mapping: dict) -> tuple[dict, int]:
    live_ids = {entry["test_id"] for entry in mapping.get("entries", [])}
    with reviews_file_lock():
        reviews = load_json(REVIEWS, {"schema_version": 1, "reviews": {}})
        review_map = reviews.setdefault("reviews", {})
        stale_ids = [test_id for test_id in review_map if test_id not in live_ids]
        for test_id in stale_ids:
            del review_map[test_id]
        if stale_ids:
            tmp = REVIEWS.with_name(f"{REVIEWS.name}.{os.getpid()}.tmp")
            tmp.write_text(json.dumps(reviews, indent=2, sort_keys=True) + "\n")
            tmp.replace(REVIEWS)
        return reviews, len(stale_ids)


def prompt_file_for_log(log_path: Path) -> Path | None:
    for line in log_path.read_text(errors="replace").splitlines()[:20]:
        match = re.search(r"# \(prompt omitted; see ([^)]+)\)", line)
        if match:
            candidate = REPO_ROOT / match.group(1)
            return candidate if candidate.is_file() else None
        match = re.search(r"# \(prompt: ([^)]+)\)", line)
        if match:
            candidate = REPO_ROOT / match.group(1)
            return candidate if candidate.is_file() else None
    return None


def prompt_test_ids(prompt_path: Path) -> list[str]:
    ids: list[str] = []
    for match in re.finditer(
        r"^## `([^`]+)`$",
        prompt_path.read_text(errors="replace"),
        re.MULTILINE,
    ):
        ids.append(match.group(1))
    return ids


def replay_logs(mapping: dict, *, force: bool = False) -> tuple[dict, Counter]:
    by_id = {entry["test_id"]: entry for entry in mapping.get("entries", [])}
    stats: Counter = Counter()
    for log_path in phase5a_log_paths():
        stats["logs_seen"] += 1
        prompt = prompt_file_for_log(log_path)
        if prompt is None:
            stats["missing_prompt"] += 1
            continue
        batch_ids = prompt_test_ids(prompt)
        batch = [by_id[test_id] for test_id in batch_ids if test_id in by_id]
        if not batch:
            stats["empty_batch"] += 1
            continue
        try:
            parsed = extract_json(log_path.read_text(errors="replace"))
            normalized = normalize_reviews(parsed, batch)
        except (ValueError, json.JSONDecodeError):
            stats["parse_failed"] += 1
            continue
        updates = {
            test_id: review
            for test_id, review in normalized.items()
            if review.get("analysis") != PLACEHOLDER_ANALYSIS
        }
        if not updates:
            stats["no_updates"] += 1
            continue
        before = load_reviews().get("reviews", {})
        update_reviews(updates, force=force)
        after = load_reviews().get("reviews", {})
        added = sum(
            1 for test_id in updates
            if force or test_id not in before or before.get(test_id) != after.get(test_id)
        )
        stats["logs_replayed"] += 1
        stats["reviews_recovered"] += added
    reviews = load_reviews()
    return reviews, stats


def auto_gap_reviews(mapping: dict) -> dict[str, dict]:
    updates: dict[str, dict] = {}
    for entry in mapping.get("entries", []):
        test_id = entry["test_id"]
        status = entry.get("status")
        if status == "missing_rust":
            updates[test_id] = {
                "test_id": test_id,
                "outcome": "needs_fix",
                "analysis": (
                    "C test entry is mapped, but the expected Rust "
                    f"`#[test] fn {entry.get('rust_test_name')}` is missing."
                ),
                "fix_summary": f"Add or restore the Rust test in {entry.get('expected_rust_file')}.",
                "verification": [],
            }
        elif status in {"missing_c", "unmapped"}:
            updates[test_id] = {
                "test_id": test_id,
                "outcome": "blocked",
                "analysis": (
                    "The C test entry could not be located in picoquictest "
                    "or the Rust mapping could not be reconstructed."
                ),
                "fix_summary": "Refresh the test map or add a manual mapping before repair.",
                "verification": [],
            }
    return updates


def mapped_entries(mapping: dict) -> list[dict]:
    return [
        entry for entry in mapping.get("entries", [])
        if entry.get("status") == "mapped" and entry.get("c") and entry.get("rust")
    ]


def work_entries(
    mapping: dict,
    reviews: dict,
    *,
    only: str | None,
    force: bool,
    shard_count: int,
    shard_index: int,
) -> list[dict]:
    done = reviews.get("reviews", {})
    out: list[dict] = []
    for entry in mapped_entries(mapping):
        if only and only not in {
            entry.get("test_id"),
            entry.get("test_name"),
            entry.get("entry_fn"),
            entry.get("rust_test_name"),
        }:
            continue
        if not force and entry["test_id"] in done:
            continue
        out.append(entry)
    out.sort(key=lambda e: (e.get("expected_rust_file", ""), e.get("rust_test_name", "")))
    if shard_count > 1:
        out = [entry for i, entry in enumerate(out) if i % shard_count == shard_index]
    return out


def prompt_path(batch: list[dict]) -> Path:
    label = "__".join(entry["entry_fn"] for entry in batch[:3])
    if len(batch) > 3:
        label += f"__plus_{len(batch) - 3}"
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in label)
    return PROMPTS_DIR / f"{safe}.md"


def log_path(agent: agent_runner.AgentConfig, batch: list[dict]) -> Path:
    return agent_runner.log_dir(REPO_ROOT, agent, "phase5a") / f"{prompt_path(batch).stem}.log"


def compose_prompt(batch: list[dict]) -> str:
    sections: list[str] = []
    for entry in batch:
        c = entry["c"]
        rust = entry["rust"]
        sections.append(
            "\n".join(
                [
                    f"## `{entry['test_id']}`",
                    f"* Canonical test_id: `{entry['test_id']}`",
                    f"* C test-table name: `{entry.get('test_name')}`",
                    f"* C entry function: `{entry.get('entry_fn')}`",
                    f"* Rust test: `{entry.get('rust_test_name')}`",
                    f"* C source: `{c['file']}:{c['start_line']}-{c['end_line']}`",
                    f"* Rust source: `{rust['file']}:{rust['start_line']}-{rust['end_line']}`",
                    "",
                    "### C test body",
                    "```c",
                    source_body(c),
                    "```",
                    "",
                    "### Rust test body",
                    "```rust",
                    source_body(rust),
                    "```",
                    "",
                ]
            )
        )
    return "\n".join(
        [
            "# Phase 5A C/Rust test correspondence audit",
            "",
            "Review each C/Rust test pair and decide whether the Rust",
            "`#[test]` checks the same behavior as the C test.  This is",
            "a read-only pass: do not edit files.",
            "",
            "You may inspect directly relevant context when needed:",
            "Rust test helpers, fixtures, translated implementation under",
            "`rs/fq/`, C helper functions in `picoquictest/`, constants,",
            "and nearby tests.  Do not require byte-for-byte structure;",
            "idiomatic Rust is fine when it preserves the C test intent,",
            "inputs, expected results, and important edge cases.",
            "",
            "Classify each entry as:",
            "",
            "* `ok` when the Rust test is an acceptable translation.",
            "* `needs_fix` when the Rust test is missing checks, checks",
            "  materially different behavior, weakens assertions, skips",
            "  cases the C test covers, or has placeholder-like logic.",
            "* `blocked` only when a concrete external decision or missing",
            "  dependency prevents classification.",
            "",
            "Return final JSON with this shape:",
            "The `test_id` field MUST be one of the exact canonical",
            "test_id values shown below. Do not substitute C function",
            "names, Rust test names, or table names.",
            "",
            "```json",
            "{\"reviews\":[{\"test_id\":\"...\",\"outcome\":\"ok|needs_fix|blocked\","
            "\"analysis\":\"short conclusion\","
            "\"fix_summary\":\"what 5B should change, or empty\","
            "\"verification\":[\"read-only context inspected\"]}]}",
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
        if isinstance(obj, dict) and "reviews" in obj:
            candidates.append(obj)

    decoder = json.JSONDecoder()
    for match in re.finditer(r"\{", text):
        try:
            obj, _ = decoder.raw_decode(text[match.start():])
        except json.JSONDecodeError:
            continue
        if isinstance(obj, dict) and "reviews" in obj:
            candidates.append(obj)
    if candidates:
        return candidates[-1]
    raise ValueError("no JSON object found")


def test_id_aliases(entry: dict) -> set[str]:
    aliases: set[str] = set()

    def add(value: object) -> None:
        if isinstance(value, str):
            value = value.strip()
            if value:
                aliases.add(value)

    test_id = entry.get("test_id")
    add(test_id)
    add(entry.get("test_name"))
    add(entry.get("entry_fn"))
    add(entry.get("rust_test_name"))
    add((entry.get("c") or {}).get("name"))
    add((entry.get("rust") or {}).get("name"))

    if isinstance(test_id, str):
        add(test_id.removeprefix("picoquictest/"))
        if ":" in test_id:
            _, fn_name = test_id.rsplit(":", 1)
            add(fn_name)

    c_file = (entry.get("c") or {}).get("file")
    entry_fn = entry.get("entry_fn")
    if isinstance(c_file, str) and isinstance(entry_fn, str):
        c_path = Path(c_file)
        add(f"{c_path.name}:{entry_fn}")
        add(f"{c_path.stem}:{entry_fn}")

    rust_file = entry.get("expected_rust_file")
    rust_name = entry.get("rust_test_name")
    if isinstance(rust_file, str) and isinstance(rust_name, str):
        rust_path = Path(rust_file)
        add(f"{rust_file}::{rust_name}")
        add(f"{rust_path.name}::{rust_name}")
        add(f"{rust_path.stem}::{rust_name}")

    return aliases


def batch_test_id_aliases(batch: list[dict]) -> dict[str, str]:
    candidates: dict[str, set[str]] = {}
    for entry in batch:
        test_id = entry["test_id"]
        for alias in test_id_aliases(entry):
            candidates.setdefault(alias, set()).add(test_id)
    return {
        alias: next(iter(test_ids))
        for alias, test_ids in candidates.items()
        if len(test_ids) == 1
    }


def normalize_reviews(raw: dict, batch: list[dict]) -> dict[str, dict]:
    def string_list(value: object) -> list[str]:
        if not isinstance(value, list):
            return []
        return [str(item)[:300] for item in value[:20]]

    batch_ids = {entry["test_id"] for entry in batch}
    aliases = batch_test_id_aliases(batch)
    by_id: dict[str, dict] = {}
    for item in raw.get("reviews", []):
        raw_test_id = item.get("test_id")
        test_id = aliases.get(raw_test_id.strip()) if isinstance(raw_test_id, str) else None
        outcome = item.get("outcome")
        if test_id not in batch_ids or outcome not in OUTCOMES:
            continue
        review = {
            "test_id": test_id,
            "outcome": outcome,
            "analysis": str(item.get("analysis", ""))[:2000],
            "fix_summary": str(item.get("fix_summary", ""))[:2000],
            "verification": string_list(item.get("verification")),
        }
        if raw_test_id != test_id:
            review["raw_test_id"] = str(raw_test_id)[:300]
        by_id[test_id] = review
    for entry in batch:
        if entry["test_id"] not in by_id:
            by_id[entry["test_id"]] = {
                "test_id": entry["test_id"],
                "outcome": "blocked",
                "analysis": PLACEHOLDER_ANALYSIS,
                "fix_summary": "",
                "verification": [],
            }
    return by_id


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


def write_report(mapping: dict, reviews: dict) -> None:
    review_map = reviews.get("reviews", {})
    counts = Counter(
        review_map.get(entry["test_id"], {}).get("outcome", "pending")
        for entry in mapping.get("entries", [])
    )
    map_counts = Counter(entry.get("status", "unknown") for entry in mapping.get("entries", []))
    total = len(mapping.get("entries", []))
    body: list[str] = [
        "<h1>Phase 5A Test Correspondence Audit</h1>",
        f"<p>Map: <code>{esc(TEST_MAP.relative_to(REPO_ROOT))}</code></p>",
        "<h2>Summary</h2>",
        "<table><tr><th>Outcome</th><th>Count</th><th>Percent</th></tr>",
    ]
    for outcome in ("ok", "needs_fix", "fixed", "blocked", "pending"):
        n = counts.get(outcome, 0)
        pct = 0.0 if total == 0 else 100.0 * n / total
        body.append(f"<tr><td>{esc(outcome)}</td><td>{n}</td><td>{pct:.1f}%</td></tr>")
    body.append("</table>")

    body.append("<h2>Map Coverage</h2>")
    body.append("<table><tr><th>Status</th><th>Count</th></tr>")
    for status in ("mapped", "missing_rust", "missing_c", "unmapped"):
        body.append(f"<tr><td>{esc(status)}</td><td>{map_counts.get(status, 0)}</td></tr>")
    body.append("</table>")

    for section, title in (
        ("needs_fix", "Needs Test Repair"),
        ("fixed", "Fixed By Phase 5B"),
        ("blocked", "Blocked"),
        ("pending", "Pending"),
        ("ok", "Confirmed OK"),
    ):
        body.append(f"<h2>{esc(title)}</h2>")
        body.append(
            "<table><tr><th>Test</th><th>C span</th><th>Rust span</th>"
            "<th>Analysis</th><th>Fix</th></tr>"
        )
        rows = []
        for entry in mapping.get("entries", []):
            review = review_map.get(entry["test_id"])
            outcome = review.get("outcome") if review else "pending"
            if outcome == section:
                rows.append((entry, review or {}))
        if not rows:
            body.append("<tr><td colspan=\"5\"><em>none</em></td></tr>")
        for entry, review in rows:
            c = entry.get("c") or {}
            rust = entry.get("rust") or {}
            body.append(
                "<tr>"
                f"<td><code>{esc(entry.get('test_name'))}</code><br>{esc(entry.get('entry_fn'))}</td>"
                f"<td>{esc(c.get('file', ''))}:{esc(c.get('start_line', ''))}-{esc(c.get('end_line', ''))}</td>"
                f"<td>{esc(rust.get('file', entry.get('expected_rust_file', '')))}:{esc(rust.get('start_line', ''))}-{esc(rust.get('end_line', ''))}</td>"
                f"<td>{esc(review.get('analysis', ''))}</td>"
                f"<td>{esc(review.get('fix_summary', ''))}</td>"
                "</tr>"
            )
        body.append("</table>")
    write_html(REPORT, "Phase 5A Test Audit", "\n".join(body))


def print_status(mapping: dict, reviews: dict) -> None:
    map_counts = Counter(entry.get("status", "unknown") for entry in mapping.get("entries", []))
    review_map = reviews.get("reviews", {})
    outcome_counts = Counter(
        review_map.get(entry["test_id"], {}).get("outcome", "pending")
        for entry in mapping.get("entries", [])
    )
    print(f"phase5 test entries: {len(mapping.get('entries', []))}")
    print("phase5 map:")
    for status in ("mapped", "missing_rust", "missing_c", "unmapped"):
        print(f"  {status:14s} {map_counts.get(status, 0)}")
    print("phase5a reviews:")
    for outcome in ("ok", "needs_fix", "fixed", "blocked", "pending"):
        print(f"  {outcome:14s} {outcome_counts.get(outcome, 0)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    agent_runner.add_agent_args(parser, model_help_context="Phase 5A test audit agent")
    parser.add_argument("--status", action="store_true", help="show Phase 5A progress")
    parser.add_argument("--dry-run", action="store_true", help="print selected tests without invoking an agent")
    parser.add_argument("--force", action="store_true", help="re-review tests with existing Phase 5A results")
    parser.add_argument("--only", help="limit to one test_id, C entry function, C test name, or Rust test name")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--batch-size", type=int, default=5)
    parser.add_argument("--shard-count", type=int, default=1)
    parser.add_argument("--shard-index", type=int, default=0)
    parser.add_argument("--max-turns", type=int, default=250)
    parser.add_argument("--no-refresh-map", action="store_true", help="reuse existing test map")
    parser.add_argument(
        "--allow-concurrent-rust-changes",
        action="store_true",
        help="do not fail the read-only guard when another phase edits rs/fq concurrently",
    )
    parser.add_argument(
        "--clear-placeholders",
        action="store_true",
        help="remove parser-bug placeholder blocked reviews, rewrite report, and exit",
    )
    parser.add_argument(
        "--prune-stale-reviews",
        action="store_true",
        help="remove reviews whose test_id is not present in the current test map",
    )
    parser.add_argument(
        "--replay-logs",
        action="store_true",
        help="reprocess existing Phase 5A logs with the current normalizer",
    )
    args = parser.parse_args()
    if args.shard_count < 1:
        parser.error("--shard-count must be at least 1")
    if args.shard_index < 0 or args.shard_index >= args.shard_count:
        parser.error("--shard-index must satisfy 0 <= index < shard-count")

    mapping = load_test_map(refresh=not args.no_refresh_map)
    reviews = load_reviews()
    if args.clear_placeholders:
        reviews, removed = clear_placeholder_reviews()
        write_report(mapping, reviews)
        print(f"removed placeholder Phase 5A reviews: {removed}")
        print_status(mapping, reviews)
        print(f"wrote: {REVIEWS.relative_to(REPO_ROOT)}")
        print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
        return 0

    if args.prune_stale_reviews:
        reviews, removed = prune_stale_reviews(mapping)
        write_report(mapping, reviews)
        print(f"removed stale Phase 5A reviews: {removed}")
        print_status(mapping, reviews)
        print(f"wrote: {REVIEWS.relative_to(REPO_ROOT)}")
        print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
        return 0

    if args.replay_logs:
        reviews, stats = replay_logs(mapping, force=args.force)
        write_report(mapping, reviews)
        print("replayed Phase 5A logs:")
        for key in (
            "logs_seen",
            "logs_replayed",
            "reviews_recovered",
            "parse_failed",
            "missing_prompt",
            "empty_batch",
            "no_updates",
        ):
            print(f"  {key:17s} {stats.get(key, 0)}")
        print_status(mapping, reviews)
        print(f"wrote: {REVIEWS.relative_to(REPO_ROOT)}")
        print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
        return 0

    if not args.status:
        gap_updates = auto_gap_reviews(mapping)
        if gap_updates:
            reviews = update_reviews(gap_updates, force=False)

    if args.status:
        print_status(mapping, reviews)
        return 0

    selected = work_entries(
        mapping,
        reviews,
        only=args.only,
        force=args.force,
        shard_count=args.shard_count,
        shard_index=args.shard_index,
    )
    if args.limit is not None:
        selected = selected[: args.limit]
    shard = ""
    if args.shard_count > 1:
        shard = f" for shard {args.shard_index}/{args.shard_count}"
    print(f"selected Phase 5A mapped tests{shard}: {len(selected)}")
    if args.dry_run:
        for entry in selected[:80]:
            print(
                f"  {entry['entry_fn']} -> "
                f"{entry['expected_rust_file']}::{entry['rust_test_name']}"
            )
        if len(selected) > 80:
            print(f"  ... and {len(selected) - 80} more")
        return 0

    agent = agent_runner.config_from_args(args)
    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
    processed = 0
    processed_ids: set[str] = set()
    batch_size = max(1, args.batch_size)
    while True:
        if args.limit is not None and processed >= args.limit:
            break
        reviews = load_reviews()
        pending = work_entries(
            mapping,
            reviews,
            only=args.only,
            force=args.force,
            shard_count=args.shard_count,
            shard_index=args.shard_index,
        )
        pending = [entry for entry in pending if entry["test_id"] not in processed_ids]
        if args.limit is not None:
            pending = pending[: args.limit - processed]
        if not pending:
            break

        batch = pending[:batch_size]
        prompt = compose_prompt(batch)
        pfile = prompt_path(batch)
        pfile.write_text(prompt)
        before_rs = rs_diff_names()
        print(f"[{processed + 1}/{len(selected)}] reviewing {len(batch)} test(s)")
        res = agent_runner.run_capture(
            agent,
            prompt,
            repo_root=REPO_ROOT,
            log_path=log_path(agent, batch),
            phase="phase5a",
            label=pfile.stem,
            prompt_file=pfile,
            allowed_tools=ALLOWED_TOOLS,
            max_turns=args.max_turns,
        )
        if res.returncode != 0:
            print(f"agent failed with exit {res.returncode}")
            return res.returncode

        after_rs = rs_diff_names()
        if after_rs != before_rs and not args.allow_concurrent_rust_changes:
            print("error: Phase 5A is read-only, but Rust files changed")
            return 3
        if after_rs != before_rs:
            print("warning: Rust files changed during Phase 5A; assuming concurrent phase activity")

        try:
            parsed = extract_json(res.stdout + "\n" + res.stderr)
            updates = normalize_reviews(parsed, batch)
        except (ValueError, json.JSONDecodeError) as exc:
            updates = {
                entry["test_id"]: {
                    "test_id": entry["test_id"],
                    "outcome": "blocked",
                    "analysis": f"could not parse agent JSON: {exc}",
                    "fix_summary": "",
                    "verification": [],
                }
                for entry in batch
            }

        reviews = update_reviews(updates, force=args.force)
        processed += len(batch)
        processed_ids.update(entry["test_id"] for entry in batch)

    reviews = load_reviews()
    write_report(mapping, reviews)
    print_status(mapping, reviews)
    print(f"wrote: {TEST_MAP.relative_to(REPO_ROOT)}")
    print(f"wrote: {REVIEWS.relative_to(REPO_ROOT)}")
    print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
