#!/usr/bin/env python3
"""phase4e.py -- repair Phase 4D `needs_fix` entries.

Phase 4E consumes `xlate/phase4d_results.json` entries classified as
`needs_fix`, applies Rust repairs, and records the repair outcome.  It
can split work into parallel file-owned buckets so multiple workers do
not edit the same Rust file at the same time.

Outputs:
  - xlate/phase4e_repairs.json
  - xlate/phase4e_report.html
  - updates `xlate/phase4d_results.json` outcomes for repaired entries
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
import phase4d
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

try:
    import fcntl
except ImportError:  # pragma: no cover - Phase 4 tooling runs on POSIX.
    fcntl = None

REPAIRS = XLATE / "phase4e_repairs.json"
REPAIRS_LOCK = XLATE / "phase4e_repairs.lock"
REPORT = XLATE / "phase4e_report.html"
PROMPTS_DIR = XLATE / "prompts" / "phase4e"

OUTCOMES = {"fixed", "ok", "blocked"}

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
        for c_id, repair in updates.items():
            if force or c_id not in repair_map:
                repair_map[c_id] = repair
        tmp = REPAIRS.with_name(f"{REPAIRS.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(repairs, indent=2, sort_keys=True) + "\n")
        tmp.replace(REPAIRS)
        return repairs


def mapped_by_id(mapping: dict) -> dict[str, dict]:
    return {e["c_id"]: e for e in phase4d.mapped_entries(mapping)}


def needs_fix_entries(
    mapping: dict,
    phase4c_reviews: dict,
    phase4d_results: dict,
    repairs: dict,
    *,
    only: str | None = None,
    rust_file: str | None = None,
    force: bool = False,
) -> list[dict]:
    by_id = mapped_by_id(mapping)
    repair_map = repairs.get("repairs", {})
    out: list[dict] = []
    for c_id, result in phase4d_results.get("results", {}).items():
        if result.get("outcome") != "needs_fix":
            continue
        e = by_id.get(c_id)
        if not e:
            continue
        if only and c_id != only and e.get("c", {}).get("name") != only:
            continue
        if rust_file and e.get("rust", {}).get("file") != rust_file:
            continue
        if not force and c_id in repair_map:
            continue
        item = dict(e)
        item["phase4c_review"] = phase4c_reviews.get(c_id, {})
        item["phase4d_result"] = result
        out.append(item)
    out.sort(key=lambda e: (e.get("rust", {}).get("file", ""), e.get("c_id", "")))
    return out


def apply_file_bucket(
    entries: list[dict],
    *,
    bucket_count: int,
    bucket_index: int,
) -> list[dict]:
    if bucket_count == 1:
        return entries
    by_file: dict[str, list[dict]] = defaultdict(list)
    for e in entries:
        by_file[e["rust"]["file"]].append(e)

    buckets: list[tuple[int, list[str]]] = [(0, []) for _ in range(bucket_count)]
    for file, group in sorted(by_file.items(), key=lambda item: (-len(item[1]), item[0])):
        load, files = min(buckets, key=lambda item: (item[0], item[1]))
        files.append(file)
        buckets[buckets.index((load, files))] = (load + len(group), files)

    selected_files = set(buckets[bucket_index][1])
    return [e for e in entries if e["rust"]["file"] in selected_files]


def prompt_path(batch: list[dict]) -> Path:
    label = "__".join(e["c"]["name"] for e in batch[:3])
    if len(batch) > 3:
        label += f"__plus_{len(batch) - 3}"
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in label)
    return PROMPTS_DIR / f"{safe}.md"


def log_path(agent: agent_runner.AgentConfig, batch: list[dict]) -> Path:
    return agent_runner.log_dir(REPO_ROOT, agent, "phase4e") / (
        f"{prompt_path(batch).stem}.log"
    )


def compose_prompt(batch: list[dict]) -> str:
    rust_files = sorted({e["rust"]["file"] for e in batch})
    sections: list[str] = []
    for e in batch:
        c = e["c"]
        rust = e["rust"]
        review = e["phase4c_review"]
        result = e["phase4d_result"]
        sections.append(
            "\n".join(
                [
                    f"## `{e['c_id']}`",
                    f"* Phase 4C status: `{review.get('status', '')}`",
                    f"* Phase 4C rationale: {review.get('rationale', '')}",
                    f"* Phase 4D analysis: {result.get('analysis', '')}",
                    f"* Phase 4D fix note: {result.get('fix_summary', '')}",
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
            "# Phase 4E repair confirmed translation mismatches",
            "",
            "You are repairing Phase 4D `needs_fix` entries.  Phase 4D",
            "already performed deeper classification and concluded that",
            "these Rust translations need repair.",
            "",
            "Rules:",
            "",
            "* Edit Rust only.  Do not edit C sources.",
            "* Keep edits limited to the owned Rust file(s) for this batch",
            "  unless a directly related helper in `rs/fq/` must change.",
            "* Preserve safe, idiomatic Rust and existing public API shape",
            "  unless the current shape cannot express the C behavior.",
            "* Do not replace code with stubs, placeholders, fabricated",
            "  defaults, or weaker behavior.",
            "* If deeper repair inspection proves Phase 4D was mistaken,",
            "  report outcome `ok` and do not edit source.",
            "* Report `blocked` only with a concrete human-actionable",
            "  reason.",
            "",
            f"Owned Rust file(s): {', '.join(f'`{f}`' for f in rust_files)}",
            "",
            "Return final JSON with this shape:",
            "",
            "```json",
            "{\"repairs\":[{\"c_id\":\"...\",\"outcome\":\"fixed|ok|blocked\","
            "\"analysis\":\"short repair conclusion\","
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

    batch_ids = {e["c_id"] for e in batch}
    by_id: dict[str, dict] = {}
    for item in raw.get("repairs", []):
        c_id = item.get("c_id")
        outcome = item.get("outcome")
        if c_id not in batch_ids or outcome not in OUTCOMES:
            continue
        phase4d_result = next(e["phase4d_result"] for e in batch if e["c_id"] == c_id)
        by_id[c_id] = {
            "c_id": c_id,
            "previous_phase4d_outcome": phase4d_result.get("outcome", ""),
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
                "previous_phase4d_outcome": e["phase4d_result"].get("outcome", ""),
                "outcome": "blocked",
                "analysis": "agent response did not include this entry",
                "fix_summary": "",
                "files_changed": [],
                "verification": [],
            }
    return by_id


def phase4d_updates(repairs: dict[str, dict], batch: list[dict]) -> dict[str, dict]:
    by_id = {e["c_id"]: e for e in batch}
    updates: dict[str, dict] = {}
    for c_id, repair in repairs.items():
        e = by_id[c_id]
        updates[c_id] = {
            "c_id": c_id,
            "phase4c_status": e["phase4c_review"].get("status", ""),
            "outcome": repair["outcome"],
            "analysis": repair.get("analysis", ""),
            "fix_summary": repair.get("fix_summary", ""),
            "files_changed": repair.get("files_changed", []),
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


def write_report(mapping: dict, phase4d_results: dict, repairs: dict) -> None:
    needs_fix_total = sum(
        1 for result in phase4d_results.get("results", {}).values()
        if result.get("outcome") == "needs_fix"
    )
    repair_map = repairs.get("repairs", {})
    counts = Counter(item.get("outcome", "unknown") for item in repair_map.values())
    body: list[str] = [
        "<h1>Phase 4E Repairs</h1>",
        f"<p>Map: <code>{esc(FUNCTION_MAP.relative_to(REPO_ROOT))}</code></p>",
        "<h2>Summary</h2>",
        "<table><tr><th>Outcome</th><th>Count</th></tr>",
    ]
    for outcome in ("fixed", "ok", "blocked"):
        body.append(f"<tr><td>{esc(outcome)}</td><td>{counts.get(outcome, 0)}</td></tr>")
    body.append(f"<tr><td>remaining needs_fix</td><td>{needs_fix_total}</td></tr>")
    body.append("</table>")

    by_id = mapped_by_id(mapping)
    for section, title in (
        ("blocked", "Blocked"),
        ("fixed", "Fixed"),
        ("ok", "Confirmed OK"),
    ):
        body.append(f"<h2>{esc(title)}</h2>")
        body.append(
            "<table><tr><th>C function</th><th>Rust span</th>"
            "<th>Analysis</th><th>Fix</th></tr>"
        )
        rows = [
            (c_id, repair) for c_id, repair in repair_map.items()
            if repair.get("outcome") == section
        ]
        if not rows:
            body.append("<tr><td colspan=\"4\"><em>none</em></td></tr>")
        for c_id, repair in sorted(rows):
            e = by_id.get(c_id, {})
            c = e.get("c", {})
            rust = e.get("rust", {})
            body.append(
                "<tr>"
                f"<td><code>{esc(c.get('name', c_id))}</code><br>{esc(c.get('file', ''))}:{esc(c.get('start_line', ''))}-{esc(c.get('end_line', ''))}</td>"
                f"<td>{esc(rust.get('file', ''))}:{esc(rust.get('start_line', ''))}-{esc(rust.get('end_line', ''))}</td>"
                f"<td>{esc(repair.get('analysis', ''))}</td>"
                f"<td>{esc(repair.get('fix_summary', ''))}</td>"
                "</tr>"
            )
        body.append("</table>")
    REPORT.write_text(html_page("Phase 4E Repairs", "\n".join(body)))


def print_status(phase4d_results: dict, repairs: dict) -> None:
    d_counts = Counter(
        result.get("outcome", "unknown")
        for result in phase4d_results.get("results", {}).values()
    )
    r_counts = Counter(
        repair.get("outcome", "unknown")
        for repair in repairs.get("repairs", {}).values()
    )
    print(f"phase4d needs_fix remaining: {d_counts.get('needs_fix', 0)}")
    print("phase4e repairs:")
    for outcome in ("fixed", "ok", "blocked"):
        print(f"  {outcome:8s} {r_counts.get(outcome, 0)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    agent_runner.add_agent_args(parser, model_help_context="Phase 4E repair agent")
    parser.add_argument("--status", action="store_true", help="show Phase 4E repair progress")
    parser.add_argument("--dry-run", action="store_true", help="print selected entries without invoking an agent")
    parser.add_argument("--force", action="store_true", help="re-run entries with existing Phase 4E repair results")
    parser.add_argument("--only", help="limit to one c_id or C function name")
    parser.add_argument("--rust-file", help="limit to one mapped Rust file")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--batch-size", type=int, default=1)
    parser.add_argument("--bucket-count", type=int, default=1,
                        help="split repair work into file-owned buckets")
    parser.add_argument("--bucket-index", type=int, default=0,
                        help="zero-based file-owned repair bucket index")
    parser.add_argument("--max-turns", type=int, default=250)
    parser.add_argument("--skip-gate", action="store_true", help="skip cargo gates after a batch")
    args = parser.parse_args()
    if args.bucket_count < 1:
        parser.error("--bucket-count must be at least 1")
    if args.bucket_index < 0 or args.bucket_index >= args.bucket_count:
        parser.error("--bucket-index must satisfy 0 <= index < bucket-count")

    mapping = load_function_map()
    phase4c_reviews = phase4d.phase4c_review_map()
    phase4d_results = phase4d.load_results()
    repairs = load_repairs()

    if args.status:
        print_status(phase4d_results, repairs)
        return 0

    selected = needs_fix_entries(
        mapping,
        phase4c_reviews,
        phase4d_results,
        repairs,
        only=args.only,
        rust_file=args.rust_file,
        force=args.force,
    )
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
    print(f"selected Phase 4E needs_fix entries{bucket}: {len(selected)}")
    if args.dry_run:
        for e in selected[:80]:
            print(
                f"  {e['rust']['file']}: {e['c_id']} -> "
                f"{e['rust']['start_line']}"
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
        prompt = compose_prompt(batch)
        pfile = prompt_path(batch)
        pfile.write_text(prompt)
        before_rs = rs_diff_names()
        print(f"[{processed + 1}/{len(selected)}] repairing {len(batch)} entry/entries")
        res = agent_runner.run_capture(
            agent,
            prompt,
            repo_root=REPO_ROOT,
            log_path=log_path(agent, batch),
            phase="phase4e",
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
                e["c_id"]: {
                    "c_id": e["c_id"],
                    "previous_phase4d_outcome": "needs_fix",
                    "outcome": "blocked",
                    "analysis": f"could not parse agent JSON: {exc}",
                    "fix_summary": "",
                    "files_changed": [],
                    "verification": [],
                }
                for e in batch
            }

        repairs = update_repairs(updates, force=args.force)
        phase4d.update_results(phase4d_updates(updates, batch), force=True)
        processed += len(batch)

        after_rs = rs_diff_names()
        if after_rs != before_rs or any(
            item.get("outcome") == "fixed" for item in updates.values()
        ):
            gate = run_gate(args.skip_gate)
            if gate != 0:
                print(f"gate failed with exit {gate}")
                return gate
            refresh = refresh_map()
            if refresh != 0:
                print(f"map refresh failed with exit {refresh}")
                return refresh
            mapping = load_function_map()

    phase4d_results = phase4d.load_results()
    repairs = load_repairs()
    write_report(mapping, phase4d_results, repairs)
    print_status(phase4d_results, repairs)
    print(f"wrote: {REPAIRS.relative_to(REPO_ROOT)}")
    print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
