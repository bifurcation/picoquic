#!/usr/bin/env python3
"""phase5c.py -- post-merge Phase 5 test revalidation.

This read-only pass revalidates C/Rust test correspondence in the final
merged tree.  It does not run the Rust test suite and does not edit source.

Outputs:
  - xlate/phase5c_revalidation.json
  - xlate/phase5c_revalidation.html
"""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import re
from collections import Counter
from pathlib import Path

import agent_runner
import phase5a
import phase5b
from phase4_common import REPO_ROOT, esc, load_json
from phase5_common import (
    TEST_MAP,
    XLATE,
    load_test_map,
    source_body,
    test_entry_by_id,
    write_html,
)

try:
    import fcntl
except ImportError:  # pragma: no cover - translation tooling runs on POSIX.
    fcntl = None

RESULTS = XLATE / "phase5c_revalidation.json"
RESULTS_LOCK = XLATE / "phase5c_revalidation.lock"
REPORT = XLATE / "phase5c_revalidation.html"
PROMPTS_DIR = XLATE / "prompts" / "phase5c"

OUTCOMES = {"ok", "needs_fix", "blocked"}
READ_ONLY_TOOLS = (
    "Read Glob Grep "
    "Bash(rg:*) "
    "Bash(git diff:*) "
    "Bash(git status:*)"
)


@contextlib.contextmanager
def result_lock():
    RESULTS_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with RESULTS_LOCK.open("a") as lock_file:
        if fcntl is not None:
            fcntl.flock(lock_file, fcntl.LOCK_EX)
        try:
            yield
        finally:
            if fcntl is not None:
                fcntl.flock(lock_file, fcntl.LOCK_UN)


def load_results() -> dict:
    with result_lock():
        results = load_json(RESULTS, {"schema_version": 1, "results": {}})
        results.setdefault("results", {})
        return results


def update_results(updates: dict[str, dict], *, force: bool) -> dict:
    with result_lock():
        results = load_json(RESULTS, {"schema_version": 1, "results": {}})
        result_map = results.setdefault("results", {})
        for test_id, item in updates.items():
            if force or test_id not in result_map:
                result_map[test_id] = item
        tmp = RESULTS.with_name(f"{RESULTS.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n")
        tmp.replace(RESULTS)
        return results


def load_worker_repair_maps(worktree_root: Path) -> list[tuple[str, dict]]:
    maps: list[tuple[str, dict]] = []
    main_repairs = load_json(XLATE / "phase5b_repairs.json", {"repairs": {}})
    maps.append(("main", main_repairs.get("repairs", {})))
    for wt in sorted(worktree_root.glob("picoquic-5b*")):
        repairs_path = wt / "xlate" / "phase5b_repairs.json"
        if repairs_path.is_file():
            repairs = load_json(repairs_path, {"repairs": {}})
            maps.append((str(wt), repairs.get("repairs", {})))
    return maps


def select_baselines(
    mapping: dict,
    reviews: dict,
    repair_maps: list[tuple[str, dict]],
) -> dict[str, dict]:
    entries_by_id = test_entry_by_id(mapping)
    review_map = reviews.get("reviews", {})
    baselines: dict[str, dict] = {}

    def add_from_review(test_id: str, review: dict) -> None:
        if test_id not in entries_by_id or test_id in baselines:
            return
        outcome = review.get("outcome")
        if outcome not in {"needs_fix", "blocked"}:
            return
        baselines[test_id] = {
            "test_id": test_id,
            "baseline_outcome": outcome,
            "baseline_source": "phase5a",
            "phase5a_outcome": outcome,
            "phase5a_analysis": review.get("analysis", ""),
            "phase5a_fix_summary": review.get("fix_summary", ""),
            "phase5b_analysis": "",
            "phase5b_fix_summary": "",
        }

    for source, repair_map in repair_maps:
        for test_id, repair in repair_map.items():
            if test_id not in entries_by_id:
                continue
            review = review_map.get(test_id, {})
            baselines[test_id] = {
                "test_id": test_id,
                "baseline_outcome": repair.get("outcome", "unknown"),
                "baseline_source": source,
                "phase5a_outcome": review.get("outcome", ""),
                "phase5a_analysis": review.get("analysis", ""),
                "phase5a_fix_summary": review.get("fix_summary", ""),
                "phase5b_analysis": repair.get("analysis", ""),
                "phase5b_fix_summary": repair.get("fix_summary", ""),
                "phase5b_files_changed": repair.get("files_changed", []),
                "phase5b_verification": repair.get("verification", []),
            }

    for test_id, review in review_map.items():
        add_from_review(test_id, review)

    return baselines


def load_id_list(path: str) -> list[str]:
    ids: list[str] = []
    for raw_line in Path(path).read_text().splitlines():
        line = raw_line.split("#", 1)[0].strip()
        if line:
            ids.append(line)
    return ids


def entry_aliases(entry: dict) -> set[str]:
    aliases = phase5a.test_id_aliases(entry)
    aliases.add(entry["test_id"])
    return aliases


def filter_by_id_list(ids: list[str], baselines: dict[str, dict], mapping: dict) -> list[str]:
    wanted = set(ids)
    entries_by_id = test_entry_by_id(mapping)
    out: list[str] = []
    for test_id in baselines:
        entry = entries_by_id.get(test_id)
        aliases = entry_aliases(entry) if entry else {test_id}
        if aliases & wanted:
            out.append(test_id)
    return out


def ordered_ids(baselines: dict[str, dict], mapping: dict) -> list[str]:
    entries_by_id = test_entry_by_id(mapping)
    return sorted(
        baselines,
        key=lambda test_id: (
            baselines[test_id].get("baseline_outcome", ""),
            (entries_by_id.get(test_id) or {}).get("expected_rust_file", ""),
            (entries_by_id.get(test_id) or {}).get("rust_test_name", ""),
            test_id,
        ),
    )


def select_entries(
    baselines: dict[str, dict],
    mapping: dict,
    results: dict,
    *,
    only: str | None,
    id_list: str | None,
    force: bool,
    limit: int | None,
    shard_count: int,
    shard_index: int,
) -> list[str]:
    ids = ordered_ids(baselines, mapping)
    if only:
        ids = filter_by_id_list([only], {test_id: baselines[test_id] for test_id in ids}, mapping)
    if id_list:
        ids = filter_by_id_list(load_id_list(id_list), {test_id: baselines[test_id] for test_id in ids}, mapping)
    if shard_count > 1:
        ids = [test_id for index, test_id in enumerate(ids) if index % shard_count == shard_index]
    if not force:
        done = results.get("results", {})
        ids = [test_id for test_id in ids if test_id not in done]
    if limit is not None:
        ids = ids[:limit]
    return ids


def prompt_path(batch: list[dict]) -> Path:
    label = "__".join(entry["entry_fn"] for entry in batch[:3])
    if len(batch) > 3:
        label += f"__plus_{len(batch) - 3}"
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in label)
    return PROMPTS_DIR / f"{safe}.md"


def log_path(agent: agent_runner.AgentConfig, batch: list[dict]) -> Path:
    return agent_runner.log_dir(REPO_ROOT, agent, "phase5c") / f"{prompt_path(batch).stem}.log"


def compose_prompt(batch: list[dict]) -> str:
    sections: list[str] = []
    for entry in batch:
        c = entry.get("c") or {}
        rust = entry.get("rust") or {}
        baseline = entry["baseline"]
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
                    f"* Current Rust span: `{rust_span}`",
                    f"* Baseline outcome: `{baseline.get('baseline_outcome', '')}`",
                    f"* Baseline source: `{baseline.get('baseline_source', '')}`",
                    f"* Phase 5A outcome: `{baseline.get('phase5a_outcome', '')}`",
                    f"* Phase 5A analysis: {baseline.get('phase5a_analysis', '')}",
                    f"* Phase 5A fix note: {baseline.get('phase5a_fix_summary', '')}",
                    f"* Phase 5B analysis: {baseline.get('phase5b_analysis', '')}",
                    f"* Phase 5B fix note: {baseline.get('phase5b_fix_summary', '')}",
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
            "# Phase 5C post-merge test revalidation",
            "",
            "This is a read-only final-tree audit after test repair",
            "worktree merges. Do not edit files. Do not run full",
            "`cargo test`.",
            "",
            "For each C/Rust test pair, decide whether the current",
            "merged Rust test faithfully expresses the C test's intent",
            "and calls the right Rust API or test-harness surface.",
            "",
            "Important standard:",
            "",
            "* Phase 5C is about test/API correspondence, not runtime",
            "  success.",
            "* Report `ok` if the Rust test is present and faithfully",
            "  checks the C test's API-visible behavior, even if the",
            "  current library implementation would make it fail.",
            "* Report `needs_fix` if the Rust test is missing checks,",
            "  checks materially different behavior, weakens assertions,",
            "  skips C cases, calls the wrong API/harness surface, or a",
            "  worker repair appears lost in the merge.",
            "* Report `blocked` only when the faithful Rust test cannot be",
            "  written, compiled, or exposed as a runnable Rust test",
            "  because the required Rust API or harness surface is missing",
            "  or ambiguous.",
            "* Do not report `blocked` for incomplete handshake behavior,",
            "  wrong state transitions, callback counters not updating,",
            "  or other implementation failures; those are Phase 6.",
            "",
            "Return final JSON with this shape:",
            "",
            "```json",
            "{\"results\":[{\"test_id\":\"...\",\"outcome\":\"ok|needs_fix|blocked\","
            "\"analysis\":\"short final-tree conclusion\","
            "\"regression_risk\":\"none|possible|likely\","
            "\"fix_summary\":\"remaining test mismatch if any, or empty\","
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
        if isinstance(obj, dict) and "results" in obj:
            candidates.append(obj)

    decoder = json.JSONDecoder()
    for match in re.finditer(r"\{", text):
        try:
            obj, _ = decoder.raw_decode(text[match.start():])
        except json.JSONDecodeError:
            continue
        if isinstance(obj, dict) and "results" in obj:
            candidates.append(obj)
    if candidates:
        return candidates[-1]
    raise ValueError("no JSON object found")


def normalize_results(raw: dict, batch: list[dict]) -> dict[str, dict]:
    def string_list(value: object) -> list[str]:
        if not isinstance(value, list):
            return []
        return [str(item)[:300] for item in value[:20]]

    batch_ids = {entry["test_id"] for entry in batch}
    aliases = phase5a.batch_test_id_aliases(batch)
    out: dict[str, dict] = {}
    for item in raw.get("results", []):
        raw_test_id = item.get("test_id")
        test_id = aliases.get(raw_test_id.strip()) if isinstance(raw_test_id, str) else None
        outcome = item.get("outcome")
        if test_id not in batch_ids or outcome not in OUTCOMES:
            continue
        baseline = next(entry["baseline"] for entry in batch if entry["test_id"] == test_id)
        result = {
            "test_id": test_id,
            "baseline_outcome": baseline.get("baseline_outcome", ""),
            "baseline_source": baseline.get("baseline_source", ""),
            "outcome": outcome,
            "analysis": str(item.get("analysis", ""))[:2000],
            "regression_risk": str(item.get("regression_risk", ""))[:80],
            "fix_summary": str(item.get("fix_summary", ""))[:2000],
            "verification": string_list(item.get("verification")),
        }
        if raw_test_id != test_id:
            result["raw_test_id"] = str(raw_test_id)[:300]
        out[test_id] = result

    for entry in batch:
        if entry["test_id"] not in out:
            baseline = entry["baseline"]
            out[entry["test_id"]] = {
                "test_id": entry["test_id"],
                "baseline_outcome": baseline.get("baseline_outcome", ""),
                "baseline_source": baseline.get("baseline_source", ""),
                "outcome": "blocked",
                "analysis": "agent response did not include this test",
                "regression_risk": "possible",
                "fix_summary": "",
                "verification": [],
            }
    return out


def comparison_bucket(item: dict) -> str:
    before = item.get("baseline_outcome")
    after = item.get("outcome")
    if before in {"fixed", "ok"} and after == "ok":
        return "preserved"
    if before in {"fixed", "ok"} and after in {"needs_fix", "blocked"}:
        return "regression"
    if before in {"needs_fix", "blocked", "pending"} and after == "ok":
        return "improved"
    if before == "needs_fix" and after == "needs_fix":
        return "still_needs_fix"
    if after == "blocked":
        return "blocked"
    return "other"


def write_report(mapping: dict, baselines: dict[str, dict], results: dict) -> None:
    result_map = results.get("results", {})
    baseline_counts = Counter(item.get("baseline_outcome", "unknown") for item in baselines.values())
    outcome_counts = Counter(item.get("outcome", "pending") for item in result_map.values())
    comparison_counts = Counter(comparison_bucket(item) for item in result_map.values())
    pending = len(baselines) - len(result_map)
    entries_by_id = test_entry_by_id(mapping)
    body: list[str] = [
        "<h1>Phase 5C Post-Merge Test Revalidation</h1>",
        f"<p>Map: <code>{esc(TEST_MAP.relative_to(REPO_ROOT))}</code></p>",
        "<h2>Summary</h2>",
        "<table><tr><th>Metric</th><th>Count</th></tr>",
        f"<tr><td>Total selected tests</td><td>{len(baselines)}</td></tr>",
        f"<tr><td>Revalidated</td><td>{len(result_map)}</td></tr>",
        f"<tr><td>Pending</td><td>{pending}</td></tr>",
        "</table>",
        "<h2>Baseline</h2>",
        "<table><tr><th>Outcome</th><th>Count</th></tr>",
    ]
    for key in ("fixed", "ok", "needs_fix", "blocked", "pending", "unknown"):
        body.append(f"<tr><td>{esc(key)}</td><td>{baseline_counts.get(key, 0)}</td></tr>")
    body.extend(["</table>", "<h2>Final Revalidation</h2>", "<table><tr><th>Outcome</th><th>Count</th></tr>"])
    for key in ("ok", "needs_fix", "blocked", "pending"):
        body.append(f"<tr><td>{esc(key)}</td><td>{outcome_counts.get(key, 0)}</td></tr>")
    body.extend(["</table>", "<h2>Comparison</h2>", "<table><tr><th>Bucket</th><th>Count</th></tr>"])
    for key in ("preserved", "regression", "improved", "still_needs_fix", "blocked", "other"):
        body.append(f"<tr><td>{esc(key)}</td><td>{comparison_counts.get(key, 0)}</td></tr>")
    body.append("</table>")

    for bucket, title in (
        ("regression", "Potential Merge Regressions"),
        ("needs_fix", "Final Needs Fix"),
        ("blocked", "Final Blocked"),
        ("ok", "Final OK"),
    ):
        body.append(f"<h2>{esc(title)}</h2>")
        body.append(
            "<table><tr><th>Test</th><th>Rust file</th><th>Baseline</th>"
            "<th>Final</th><th>Analysis</th><th>Fix</th></tr>"
        )
        rows: list[tuple[str, dict]] = []
        for test_id, item in result_map.items():
            if bucket == "regression":
                match = comparison_bucket(item) == "regression"
            else:
                match = item.get("outcome") == bucket
            if match:
                rows.append((test_id, item))
        if not rows:
            body.append("<tr><td colspan=\"6\"><em>none</em></td></tr>")
        for test_id, item in sorted(rows):
            entry = entries_by_id.get(test_id, {})
            body.append(
                "<tr>"
                f"<td><code>{esc(entry.get('test_name', test_id))}</code><br>{esc(entry.get('entry_fn', ''))}</td>"
                f"<td>{esc(entry.get('expected_rust_file', ''))}</td>"
                f"<td>{esc(item.get('baseline_outcome', ''))}</td>"
                f"<td>{esc(item.get('outcome', ''))}</td>"
                f"<td>{esc(item.get('analysis', ''))}</td>"
                f"<td>{esc(item.get('fix_summary', ''))}</td>"
                "</tr>"
            )
        body.append("</table>")
    write_html(REPORT, "Phase 5C Test Revalidation", "\n".join(body))


def print_status(baselines: dict[str, dict], results: dict) -> None:
    result_map = results.get("results", {})
    baseline_counts = Counter(item.get("baseline_outcome", "unknown") for item in baselines.values())
    outcome_counts = Counter(item.get("outcome", "pending") for item in result_map.values())
    comparison_counts = Counter(comparison_bucket(item) for item in result_map.values())
    print(f"phase5c tests selected: {len(baselines)}")
    print(
        "baseline: "
        f"fixed={baseline_counts.get('fixed', 0)} "
        f"ok={baseline_counts.get('ok', 0)} "
        f"needs_fix={baseline_counts.get('needs_fix', 0)} "
        f"blocked={baseline_counts.get('blocked', 0)}"
    )
    print(
        "final revalidation: "
        f"ok={outcome_counts.get('ok', 0)} "
        f"needs_fix={outcome_counts.get('needs_fix', 0)} "
        f"blocked={outcome_counts.get('blocked', 0)} "
        f"remaining={len(baselines) - len(result_map)}"
    )
    print(
        "comparison: "
        f"preserved={comparison_counts.get('preserved', 0)} "
        f"regression={comparison_counts.get('regression', 0)} "
        f"improved={comparison_counts.get('improved', 0)} "
        f"still_needs_fix={comparison_counts.get('still_needs_fix', 0)}"
    )


def rs_diff_names() -> set[str]:
    import subprocess

    res = subprocess.run(
        ["git", "diff", "--name-only", "--", "rs/fq"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    if res.returncode != 0:
        return set()
    return {line.strip() for line in res.stdout.splitlines() if line.strip()}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    agent_runner.add_agent_args(parser, model_help_context="Phase 5C revalidation agent")
    parser.add_argument("--status", action="store_true", help="show Phase 5C revalidation progress")
    parser.add_argument("--dry-run", action="store_true", help="print selected tests without invoking an agent")
    parser.add_argument("--force", action="store_true", help="re-run tests with existing Phase 5C results")
    parser.add_argument("--only", help="limit to one test_id, C entry function, C test name, or Rust test name")
    parser.add_argument("--id-list", help="limit to newline-delimited test ids/names")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--batch-size", type=int, default=5)
    parser.add_argument("--shard-count", type=int, default=1)
    parser.add_argument("--shard-index", type=int, default=0)
    parser.add_argument("--worktree-root", type=Path, default=Path("/private/tmp"))
    parser.add_argument("--max-turns", type=int, default=250)
    parser.add_argument(
        "--no-refresh-map",
        action="store_true",
        help="reuse existing test map instead of refreshing from the current tree",
    )
    parser.add_argument(
        "--allow-concurrent-rust-changes",
        action="store_true",
        help="do not fail the read-only guard when another phase edits rs/fq concurrently",
    )
    args = parser.parse_args()
    if args.shard_count < 1:
        parser.error("--shard-count must be at least 1")
    if args.shard_index < 0 or args.shard_index >= args.shard_count:
        parser.error("--shard-index must satisfy 0 <= index < shard-count")

    mapping = load_test_map(refresh=not args.no_refresh_map)
    reviews = phase5a.load_reviews()
    repair_maps = load_worker_repair_maps(args.worktree_root)
    baselines = select_baselines(mapping, reviews, repair_maps)
    results = load_results()
    write_report(mapping, baselines, results)

    if args.status:
        print_status(baselines, results)
        print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
        return 0

    selected_ids = select_entries(
        baselines,
        mapping,
        results,
        only=args.only,
        id_list=args.id_list,
        force=args.force,
        limit=args.limit,
        shard_count=args.shard_count,
        shard_index=args.shard_index,
    )
    shard = f" for shard {args.shard_index}/{args.shard_count}" if args.shard_count > 1 else ""
    print(f"selected Phase 5C revalidation tests{shard}: {len(selected_ids)}")
    if args.dry_run:
        entries_by_id = test_entry_by_id(mapping)
        for test_id in selected_ids[:80]:
            entry = entries_by_id.get(test_id, {})
            print(
                f"  {baselines[test_id].get('baseline_outcome', '')}: "
                f"{entry.get('expected_rust_file', '')}:"
                f"{entry.get('rust_test_name', test_id)}"
            )
        if len(selected_ids) > 80:
            print(f"  ... and {len(selected_ids) - 80} more")
        return 0

    agent = agent_runner.config_from_args(args)
    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
    entries_by_id = test_entry_by_id(mapping)
    processed = 0
    batch_size = max(1, args.batch_size)
    while processed < len(selected_ids):
        batch_ids = selected_ids[processed:processed + batch_size]
        batch: list[dict] = []
        missing: dict[str, dict] = {}
        for test_id in batch_ids:
            entry = entries_by_id.get(test_id)
            baseline = baselines[test_id]
            if not entry or entry.get("status") != "mapped":
                missing[test_id] = {
                    "test_id": test_id,
                    "baseline_outcome": baseline.get("baseline_outcome", ""),
                    "baseline_source": baseline.get("baseline_source", ""),
                    "outcome": "blocked",
                    "analysis": "test is missing from refreshed final-tree test map",
                    "regression_risk": "possible",
                    "fix_summary": "Refresh the map or restore the missing Rust/C test mapping.",
                    "verification": [],
                }
                continue
            item = dict(entry)
            item["baseline"] = baseline
            batch.append(item)

        updates = dict(missing)
        if batch:
            prompt = compose_prompt(batch)
            pfile = prompt_path(batch)
            pfile.write_text(prompt)
            before_rs = rs_diff_names()
            print(f"[{processed + 1}/{len(selected_ids)}] revalidating {len(batch)} test(s)")
            res = agent_runner.run_capture(
                agent,
                prompt,
                repo_root=REPO_ROOT,
                log_path=log_path(agent, batch),
                phase="phase5c",
                label=pfile.stem,
                prompt_file=pfile,
                allowed_tools=READ_ONLY_TOOLS,
                max_turns=args.max_turns,
            )
            after_rs = rs_diff_names()
            if after_rs != before_rs and not args.allow_concurrent_rust_changes:
                print("error: Phase 5C is read-only, but Rust files changed")
                return 3
            if after_rs != before_rs:
                print("warning: Rust files changed during Phase 5C; assuming concurrent phase activity")

            if res.returncode != 0:
                updates.update(
                    {
                        item["test_id"]: {
                            "test_id": item["test_id"],
                            "baseline_outcome": item["baseline"].get("baseline_outcome", ""),
                            "baseline_source": item["baseline"].get("baseline_source", ""),
                            "outcome": "blocked",
                            "analysis": f"revalidation agent failed with exit {res.returncode}",
                            "regression_risk": "possible",
                            "fix_summary": "",
                            "verification": [],
                        }
                        for item in batch
                    }
                )
            else:
                try:
                    updates.update(normalize_results(extract_json(res.stdout + "\n" + res.stderr), batch))
                except (ValueError, json.JSONDecodeError) as exc:
                    updates.update(
                        {
                            item["test_id"]: {
                                "test_id": item["test_id"],
                                "baseline_outcome": item["baseline"].get("baseline_outcome", ""),
                                "baseline_source": item["baseline"].get("baseline_source", ""),
                                "outcome": "blocked",
                                "analysis": f"could not parse revalidation JSON: {exc}",
                                "regression_risk": "possible",
                                "fix_summary": "",
                                "verification": [],
                            }
                            for item in batch
                        }
                    )

        results = update_results(updates, force=args.force)
        write_report(mapping, baselines, results)
        print_status(baselines, results)
        processed += len(batch_ids)

    print(f"wrote: {RESULTS.relative_to(REPO_ROOT)}")
    print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
