#!/usr/bin/env python3
"""phase4d.py -- deep review and repair of Phase 4C non-OK entries.

Phase 4D consumes the body-only Phase 4C audit.  For each `suspect` or
`definitely_not_ok` C/Rust pair, it asks an agent to inspect the broader
source context, decide whether the Phase 4C concern is real, and fix the
Rust translation when it is real.

Outputs:
  - xlate/phase4d_results.json
  - xlate/phase4d_report.html
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
from collections import Counter
from pathlib import Path

import agent_runner
from phase4_common import (
    FUNCTION_MAP,
    REPO_ROOT,
    RS_CRATE,
    XLATE,
    build_function_map,
    esc,
    html_page,
    load_function_map,
    load_json,
    save_json,
    source_body,
)

PHASE4C_REVIEWS = XLATE / "phase4c_reviews.json"
RESULTS = XLATE / "phase4d_results.json"
REPORT = XLATE / "phase4d_report.html"
PROMPTS_DIR = XLATE / "prompts" / "phase4d"

PHASE4C_WORK_STATUSES = {"suspect", "definitely_not_ok"}
OUTCOMES = {"ok", "fixed", "blocked"}

ALLOWED_TOOLS = (
    "Read Edit Write Glob Grep "
    "Bash(rg:*) "
    "Bash(git diff:*) "
    "Bash(git status:*) "
    "Bash(cargo check:*) "
    "Bash(cargo test:*) "
    "Bash(cargo fmt:*) "
    "Bash(cargo clippy:*) "
    "Bash(python3 scripts/phase4_check.py:*)"
)


def mapped_entries(mapping: dict) -> list[dict]:
    return [
        e for e in mapping.get("entries", [])
        if e.get("rust") and e.get("c")
    ]


def phase4c_review_map() -> dict:
    return load_json(PHASE4C_REVIEWS, {"schema_version": 1, "reviews": {}}).get(
        "reviews", {}
    )


def load_results() -> dict:
    results = load_json(RESULTS, {"schema_version": 1, "results": {}})
    results.setdefault("results", {})
    return results


def save_results(results: dict) -> None:
    save_json(RESULTS, results)


def work_entries(
    mapping: dict,
    reviews: dict,
    results: dict,
    *,
    only: str | None = None,
    force: bool = False,
    order: str = "severity",
) -> list[dict]:
    done = results.get("results", {})
    out: list[dict] = []
    for e in mapped_entries(mapping):
        c_id = e["c_id"]
        review = reviews.get(c_id, {})
        if review.get("status") not in PHASE4C_WORK_STATUSES:
            continue
        if only and c_id != only and e.get("c", {}).get("name") != only:
            continue
        if not force and c_id in done:
            continue
        item = dict(e)
        item["phase4c_review"] = review
        out.append(item)
    if order == "severity":
        rank = {"definitely_not_ok": 0, "suspect": 1}
        out.sort(
            key=lambda e: (
                rank.get(e["phase4c_review"].get("status"), 9),
                e.get("c_id", ""),
            )
        )
    return out


def prompt_path(batch: list[dict]) -> Path:
    label = "__".join(e["c"]["name"] for e in batch[:3])
    if len(batch) > 3:
        label += f"__plus_{len(batch) - 3}"
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in label)
    return PROMPTS_DIR / f"{safe}.md"


def log_path(agent: agent_runner.AgentConfig, batch: list[dict]) -> Path:
    return agent_runner.log_dir(REPO_ROOT, agent, "phase4d") / (
        f"{prompt_path(batch).stem}.log"
    )


def compose_prompt(batch: list[dict]) -> str:
    sections: list[str] = []
    for e in batch:
        c = e["c"]
        rust = e["rust"]
        review = e["phase4c_review"]
        sections.append(
            "\n".join(
                [
                    f"## `{e['c_id']}`",
                    f"* Phase 4C status: `{review.get('status')}`",
                    f"* Phase 4C rationale: {review.get('rationale', '')}",
                    f"* C source: `{c['file']}:{c['start_line']}-{c['end_line']}`",
                    f"* C signature: `{c.get('signature', '')}`",
                    f"* Rust source: `{rust['file']}:{rust['start_line']}-{rust['end_line']}`",
                    f"* Rust item: `{rust.get('name', '')}`",
                    "",
                    "### C body",
                    "```c",
                    source_body(c),
                    "```",
                    "",
                    "### Rust body",
                    "```rust",
                    source_body(rust),
                    "```",
                    "",
                ]
            )
        )

    return "\n".join(
        [
            "# Phase 4D deep translation review and repair",
            "",
            "You are resolving Phase 4C non-OK C/Rust function-pair audit",
            "entries.  Phase 4C was intentionally body-only and shallow;",
            "Phase 4D is allowed to inspect broader context and edit Rust.",
            "",
            "For each entry:",
            "",
            "1. Read the C function and any directly relevant C context:",
            "   types, constants/macros, helper callees, and callers when",
            "   needed to understand observable behavior.",
            "2. Read the Rust function in context, including local types,",
            "   helpers, tests, and nearby translated functions.",
            "3. Decide whether the Phase 4C concern is a false positive.",
            "   If the Rust behavior is acceptable, do not edit source and",
            "   report outcome `ok`.",
            "4. If the Rust translation is actually wrong, fix it now in",
            "   `rs/fq/` while preserving safe, idiomatic Rust and the",
            "   repository translation rules.",
            "5. Only report `blocked` if a real mismatch remains impossible",
            "   to fix in this batch, and give a concrete human-actionable",
            "   reason.  Do not use vague deferrals.",
            "",
            "Do not edit C sources.  Do not replace working Rust with a stub,",
            "placeholder, fabricated default, or weaker behavior.  Keep public",
            "API shape stable unless the existing shape cannot express the C",
            "behavior safely.",
            "",
            "If you fix anything, run the narrowest useful test if one is",
            "obvious.  The driver will run the standard cargo gates after",
            "the batch when needed.",
            "",
            "Return final JSON with this shape:",
            "",
            "```json",
            "{\"results\":[{\"c_id\":\"...\",\"outcome\":\"ok|fixed|blocked\","
            "\"analysis\":\"short deeper-review conclusion\","
            "\"fix_summary\":\"what changed, or empty\","
            "\"files_changed\":[\"rs/fq/src/...\"],"
            "\"verification\":[\"cargo ...\"]}]}",
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

    batch_ids = {e["c_id"] for e in batch}
    by_id: dict[str, dict] = {}
    for item in raw.get("results", []):
        c_id = item.get("c_id")
        outcome = item.get("outcome")
        if c_id not in batch_ids or outcome not in OUTCOMES:
            continue
        by_id[c_id] = {
            "c_id": c_id,
            "phase4c_status": next(
                e["phase4c_review"].get("status", "") for e in batch if e["c_id"] == c_id
            ),
            "outcome": outcome,
            "analysis": str(item.get("analysis", ""))[:2000],
            "fix_summary": str(item.get("fix_summary", ""))[:2000],
            "files_changed": string_list(item.get("files_changed")),
            "verification": string_list(item.get("verification")),
        }
    for e in batch:
        if e["c_id"] not in by_id:
            by_id[e["c_id"]] = {
                "c_id": e["c_id"],
                "phase4c_status": e["phase4c_review"].get("status", ""),
                "outcome": "blocked",
                "analysis": "agent response did not include this entry",
                "fix_summary": "",
                "files_changed": [],
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


def refresh_map() -> int:
    refreshed = build_function_map(load_function_map())
    save_json(FUNCTION_MAP, refreshed)
    fixer = REPO_ROOT / "scripts" / "fix_phase4b_map_entries.py"
    if fixer.is_file():
        return subprocess.run(["python3", str(fixer)], cwd=REPO_ROOT).returncode
    return 0


def write_report(mapping: dict, reviews: dict, results: dict) -> None:
    entries = [
        e for e in mapped_entries(mapping)
        if reviews.get(e["c_id"], {}).get("status") in PHASE4C_WORK_STATUSES
    ]
    result_map = results.get("results", {})
    counts = Counter(
        result_map.get(e["c_id"], {}).get("outcome", "pending") for e in entries
    )
    total = len(entries)
    body: list[str] = [
        "<h1>Phase 4D Deep Review and Repair</h1>",
        f"<p>Map: <code>{esc(FUNCTION_MAP.relative_to(REPO_ROOT))}</code></p>",
        f"<p>Input: <code>{esc(PHASE4C_REVIEWS.relative_to(REPO_ROOT))}</code></p>",
        "<h2>Summary</h2>",
        "<table><tr><th>Outcome</th><th>Count</th><th>Percent</th></tr>",
    ]
    for status in ("ok", "fixed", "blocked", "pending"):
        n = counts.get(status, 0)
        pct = 0.0 if total == 0 else 100.0 * n / total
        body.append(f"<tr><td>{esc(status)}</td><td>{n}</td><td>{pct:.1f}%</td></tr>")
    body.append("</table>")

    for section, title in (
        ("blocked", "Blocked"),
        ("fixed", "Fixed"),
        ("ok", "Confirmed OK"),
        ("pending", "Pending"),
    ):
        body.append(f"<h2>{esc(title)}</h2>")
        body.append(
            "<table><tr><th>C function</th><th>Phase 4C</th>"
            "<th>Rust span</th><th>Analysis</th><th>Fix</th></tr>"
        )
        rows = []
        for e in entries:
            result = result_map.get(e["c_id"])
            outcome = result.get("outcome") if result else "pending"
            if outcome == section:
                rows.append((e, result or {}))
        if not rows:
            body.append("<tr><td colspan=\"5\"><em>none</em></td></tr>")
        for e, result in rows:
            c = e["c"]
            rust = e["rust"]
            review = reviews.get(e["c_id"], {})
            body.append(
                "<tr>"
                f"<td><code>{esc(c['name'])}</code><br>{esc(c['file'])}:{esc(c['start_line'])}-{esc(c['end_line'])}</td>"
                f"<td>{esc(review.get('status', ''))}<br>{esc(review.get('rationale', ''))}</td>"
                f"<td>{esc(rust['file'])}:{esc(rust['start_line'])}-{esc(rust['end_line'])}</td>"
                f"<td>{esc(result.get('analysis', ''))}</td>"
                f"<td>{esc(result.get('fix_summary', ''))}</td>"
                "</tr>"
            )
        body.append("</table>")
    REPORT.write_text(html_page("Phase 4D Deep Review", "\n".join(body)))


def print_status(mapping: dict, reviews: dict, results: dict) -> None:
    entries = [
        e for e in mapped_entries(mapping)
        if reviews.get(e["c_id"], {}).get("status") in PHASE4C_WORK_STATUSES
    ]
    c_counts = Counter(reviews.get(e["c_id"], {}).get("status") for e in entries)
    r_counts = Counter(
        results.get("results", {}).get(e["c_id"], {}).get("outcome", "pending")
        for e in entries
    )
    print(f"phase4c non-ok pairs: {len(entries)}")
    for status in ("suspect", "definitely_not_ok"):
        print(f"  phase4c {status:17s} {c_counts.get(status, 0)}")
    print("phase4d outcomes:")
    for status in ("ok", "fixed", "blocked", "pending"):
        print(f"  {status:24s} {r_counts.get(status, 0)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    agent_runner.add_agent_args(parser, model_help_context="Phase 4D repair agent")
    parser.add_argument("--status", action="store_true", help="show Phase 4D progress")
    parser.add_argument("--dry-run", action="store_true", help="print selected entries without invoking an agent")
    parser.add_argument("--force", action="store_true", help="re-run entries with existing Phase 4D results")
    parser.add_argument("--only", help="limit to one c_id or C function name")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--batch-size", type=int, default=1)
    parser.add_argument("--max-turns", type=int, default=250)
    parser.add_argument("--skip-gate", action="store_true", help="skip cargo gates after a batch that changes Rust")
    parser.add_argument(
        "--order",
        choices=("severity", "original"),
        default="severity",
        help="severity reviews definitely_not_ok entries before suspect entries",
    )
    args = parser.parse_args()

    mapping = load_function_map()
    reviews = phase4c_review_map()
    results = load_results()

    if args.status:
        if not mapping.get("entries"):
            print("no function map found; run `python3 scripts/phase4a.py` first")
            return 0
        if not reviews:
            print("no Phase 4C reviews found; run `python3 scripts/phase4c.py` first")
            return 0
        print_status(mapping, reviews, results)
        return 0

    if not mapping.get("entries"):
        print("no function map found; run `python3 scripts/phase4a.py` first")
        return 2
    if not reviews:
        print("no Phase 4C reviews found; run `python3 scripts/phase4c.py` first")
        return 2

    selected = work_entries(
        mapping,
        reviews,
        results,
        only=args.only,
        force=args.force,
        order=args.order,
    )
    if args.limit is not None:
        selected = selected[: args.limit]
    print(f"selected Phase 4C non-OK entries: {len(selected)}")
    if args.dry_run:
        for e in selected[:50]:
            review = e["phase4c_review"]
            print(
                f"  {review.get('status')}: {e['c_id']} -> "
                f"{e['rust']['file']}:{e['rust']['start_line']}"
            )
        if len(selected) > 50:
            print(f"  ... and {len(selected) - 50} more")
        return 0

    agent = agent_runner.config_from_args(args)
    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
    processed = 0
    processed_ids: set[str] = set()
    batch_size = max(1, args.batch_size)
    while True:
        if args.limit is not None and processed >= args.limit:
            break
        mapping = load_function_map()
        results = load_results()
        pending = work_entries(
            mapping,
            reviews,
            results,
            only=args.only,
            force=args.force,
            order=args.order,
        )
        pending = [e for e in pending if e["c_id"] not in processed_ids]
        if args.limit is not None:
            pending = pending[: args.limit - processed]
        if not pending:
            break

        batch = pending[:batch_size]
        prompt = compose_prompt(batch)
        pfile = prompt_path(batch)
        pfile.write_text(prompt)
        before_rs = rs_diff_names()
        print(
            f"[{processed + 1}/{len(selected)}] "
            f"deep-reviewing {len(batch)} entry/entries"
        )
        res = agent_runner.run_capture(
            agent,
            prompt,
            repo_root=REPO_ROOT,
            log_path=log_path(agent, batch),
            phase="phase4d",
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
            updates = normalize_results(parsed, batch)
        except (ValueError, json.JSONDecodeError) as exc:
            updates = {
                e["c_id"]: {
                    "c_id": e["c_id"],
                    "phase4c_status": e["phase4c_review"].get("status", ""),
                    "outcome": "blocked",
                    "analysis": f"could not parse agent JSON: {exc}",
                    "fix_summary": "",
                    "files_changed": [],
                    "verification": [],
                }
                for e in batch
            }

        result_map = results.setdefault("results", {})
        result_map.update(updates)
        save_results(results)
        processed += len(batch)
        processed_ids.update(e["c_id"] for e in batch)

        after_rs = rs_diff_names()
        fixed = any(item.get("outcome") == "fixed" for item in updates.values())
        if fixed or after_rs != before_rs:
            gate = run_gate(args.skip_gate)
            if gate != 0:
                print(f"gate failed with exit {gate}")
                return gate
            refresh = refresh_map()
            if refresh != 0:
                print(f"map refresh failed with exit {refresh}")
                return refresh
            mapping = load_function_map()

    write_report(mapping, reviews, results)
    print_status(mapping, reviews, results)
    print(f"wrote: {RESULTS.relative_to(REPO_ROOT)}")
    print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
