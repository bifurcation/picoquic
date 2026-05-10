#!/usr/bin/env python3
"""phase4f.py -- post-merge Phase 4 semantic revalidation.

This read-only pass re-triages the final merged Rust tree for the Phase 4E
function IDs handled by the worker worktrees.  It writes separate post-merge
artifacts instead of overwriting Phase 4D/4E evidence:

  - xlate/phase4f_revalidation.json
  - xlate/phase4f_revalidation.md
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
import phase4d
import phase4e
from phase4_common import (
    REPO_ROOT,
    XLATE,
    build_function_map,
    load_function_map,
    load_json,
    source_body,
)

try:
    import fcntl
except ImportError:  # pragma: no cover - tooling runs on POSIX.
    fcntl = None

RESULTS = XLATE / "phase4f_revalidation.json"
RESULTS_LOCK = XLATE / "phase4f_revalidation.lock"
REPORT = XLATE / "phase4f_revalidation.md"
PROMPTS_DIR = XLATE / "prompts" / "phase4f"

OUTCOMES = {"ok", "needs_fix", "blocked"}
READ_ONLY_TOOLS = phase4d.CLASSIFY_ALLOWED_TOOLS


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


def save_results(data: dict) -> None:
    with result_lock():
        tmp = RESULTS.with_name(f"{RESULTS.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
        tmp.replace(RESULTS)


def update_results(updates: dict[str, dict], *, force: bool) -> dict:
    with result_lock():
        results = load_json(RESULTS, {"schema_version": 1, "results": {}})
        result_map = results.setdefault("results", {})
        for c_id, item in updates.items():
            if force or c_id not in result_map:
                result_map[c_id] = item
        tmp = RESULTS.with_name(f"{RESULTS.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n")
        tmp.replace(RESULTS)
        return results


def cluster_ids(worktree_root: Path, clusters: int) -> list[str]:
    ids: list[str] = []
    for index in range(clusters):
        suffix = f"{index:02d}"
        path = (
            worktree_root
            / f"picoquic-4e-{suffix}"
            / "xlate"
            / "clusters"
            / "phase4e"
            / f"cluster-{suffix}.txt"
        )
        if not path.is_file():
            path = XLATE / "clusters" / "phase4e" / f"cluster-{suffix}.txt"
        for raw in path.read_text().splitlines() if path.is_file() else []:
            line = raw.split("#", 1)[0].strip()
            if line:
                ids.append(line)
    return ids


def load_repair_maps(worktree_root: Path, clusters: int) -> list[tuple[str, dict]]:
    worktrees = [worktree_root / f"picoquic-4e-{index:02d}" for index in range(clusters)]
    worktrees.extend(sorted(worktree_root.glob("picoquic-4e-r2-*")))
    maps: list[tuple[str, dict]] = []
    for wt in worktrees:
        repairs = load_json(wt / "xlate" / "phase4e_repairs.json", {"repairs": {}})
        maps.append((str(wt), repairs.get("repairs", {})))
    main_repairs = load_json(XLATE / "phase4e_repairs.json", {"repairs": {}})
    maps.append(("main", main_repairs.get("repairs", {})))
    return maps


def baseline_for_ids(ids: list[str], repair_maps: list[tuple[str, dict]]) -> dict[str, dict]:
    baselines: dict[str, dict] = {}
    for c_id in ids:
        saw_needs_fix: tuple[str, dict] | None = None
        saw_any: tuple[str, dict] | None = None
        for source, repair_map in repair_maps:
            repair = repair_map.get(c_id)
            if not repair:
                continue
            saw_any = saw_any or (source, repair)
            if phase4e.repair_is_terminal(repair):
                baselines[c_id] = {
                    "c_id": c_id,
                    "baseline_outcome": repair.get("outcome", "unknown"),
                    "baseline_source": source,
                    "baseline_analysis": repair.get("analysis", ""),
                    "baseline_fix_summary": repair.get("fix_summary", ""),
                    "baseline_confirmation_outcome": repair.get("confirmation_outcome", ""),
                    "baseline_confirmation_analysis": repair.get("confirmation_analysis", ""),
                }
                break
            if repair.get("outcome") == "needs_fix" and saw_needs_fix is None:
                saw_needs_fix = (source, repair)
        else:
            source, repair = saw_needs_fix or saw_any or ("none", {})
            baselines[c_id] = {
                "c_id": c_id,
                "baseline_outcome": repair.get("outcome", "pending"),
                "baseline_source": source,
                "baseline_analysis": repair.get("analysis", ""),
                "baseline_fix_summary": repair.get("fix_summary", ""),
                "baseline_confirmation_outcome": repair.get("confirmation_outcome", ""),
                "baseline_confirmation_analysis": repair.get("confirmation_analysis", ""),
            }
    return baselines


def mapped_by_id(mapping: dict) -> dict[str, dict]:
    return {entry["c_id"]: entry for entry in phase4d.mapped_entries(mapping)}


def load_id_list(path: str) -> list[str]:
    ids: list[str] = []
    for raw in Path(path).read_text().splitlines():
        line = raw.split("#", 1)[0].strip()
        if line:
            ids.append(line)
    return ids


def select_entries(
    ids: list[str],
    baselines: dict[str, dict],
    results: dict,
    *,
    force: bool,
    id_list: str | None,
    limit: int | None,
    shard_count: int,
    shard_index: int,
) -> list[str]:
    if id_list:
        allowed = set(load_id_list(id_list))
        ids = [c_id for c_id in ids if c_id in allowed]
    if shard_count > 1:
        ids = [c_id for i, c_id in enumerate(ids) if i % shard_count == shard_index]
    if not force:
        done = results.get("results", {})
        ids = [c_id for c_id in ids if c_id not in done]
    ids.sort(key=lambda c_id: (baselines[c_id].get("baseline_outcome", ""), c_id))
    if limit is not None:
        ids = ids[:limit]
    return ids


def prompt_path(batch: list[dict]) -> Path:
    label = "__".join(entry["c"]["name"] for entry in batch[:3])
    if len(batch) > 3:
        label += f"__plus_{len(batch) - 3}"
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in label)
    return PROMPTS_DIR / f"{safe}.md"


def log_path(agent: agent_runner.AgentConfig, batch: list[dict]) -> Path:
    return agent_runner.log_dir(REPO_ROOT, agent, "phase4f") / (
        f"{prompt_path(batch).stem}.log"
    )


def compose_prompt(batch: list[dict]) -> str:
    sections: list[str] = []
    for entry in batch:
        c = entry["c"]
        rust = entry["rust"]
        baseline = entry["baseline"]
        sections.append(
            "\n".join(
                [
                    f"## `{entry['c_id']}`",
                    f"* Worker baseline outcome: `{baseline.get('baseline_outcome', '')}`",
                    f"* Worker baseline source: `{baseline.get('baseline_source', '')}`",
                    f"* Worker analysis: {baseline.get('baseline_analysis', '')}",
                    f"* Worker fix summary: {baseline.get('baseline_fix_summary', '')}",
                    f"* Worker confirmation outcome: `{baseline.get('baseline_confirmation_outcome', '')}`",
                    f"* Worker confirmation analysis: {baseline.get('baseline_confirmation_analysis', '')}",
                    f"* C source: `{c['file']}:{c['start_line']}-{c['end_line']}`",
                    f"* C signature: `{c.get('signature', '')}`",
                    f"* Current merged Rust source: `{rust['file']}:{rust['start_line']}-{rust['end_line']}`",
                    f"* Current merged Rust item: `{rust.get('name', '')}`",
                    "",
                    "### C body",
                    "```c",
                    source_body(c),
                    "```",
                    "",
                    "### Current merged Rust body",
                    "```rust",
                    source_body(rust),
                    "```",
                    "",
                ]
            )
        )
    return "\n".join(
        [
            "# Phase 4F post-merge semantic revalidation",
            "",
            "This is a read-only final-tree audit after worker worktree",
            "merges.  Do not edit files.",
            "",
            "For each C/Rust function pair, inspect the C function and the",
            "current merged Rust implementation.  Decide whether the merged",
            "Rust behavior is acceptable as a safe Rust translation of the",
            "C behavior.  Pay particular attention to whether a prior",
            "worker repair may have been lost or distorted during conflict",
            "resolution.",
            "",
            "Report:",
            "",
            "* `ok` when the current merged Rust behavior is acceptable.",
            "* `needs_fix` when a real C/Rust mismatch remains in the",
            "  current merged tree.",
            "* `blocked` only when a concrete missing external decision or",
            "  dependency prevents classification.",
            "",
            "Return final JSON with this shape:",
            "",
            "```json",
            "{\"results\":[{\"c_id\":\"...\",\"outcome\":\"ok|needs_fix|blocked\","
            "\"analysis\":\"short final-tree conclusion\","
            "\"regression_risk\":\"none|possible|likely\","
            "\"fix_summary\":\"remaining mismatch if any, or empty\","
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
    batch_by_id = {entry["c_id"]: entry for entry in batch}
    out: dict[str, dict] = {}
    for item in raw.get("results", []):
        c_id = item.get("c_id")
        outcome = item.get("outcome")
        if c_id not in batch_by_id or outcome not in OUTCOMES:
            continue
        baseline = batch_by_id[c_id]["baseline"]
        out[c_id] = {
            "c_id": c_id,
            "baseline_outcome": baseline.get("baseline_outcome", ""),
            "baseline_source": baseline.get("baseline_source", ""),
            "outcome": outcome,
            "analysis": str(item.get("analysis", ""))[:2000],
            "regression_risk": str(item.get("regression_risk", ""))[:80],
            "fix_summary": str(item.get("fix_summary", ""))[:2000],
            "verification": [
                str(value)[:300]
                for value in item.get("verification", [])[:20]
                if isinstance(item.get("verification", []), list)
            ],
        }
    for c_id, entry in batch_by_id.items():
        if c_id not in out:
            baseline = entry["baseline"]
            out[c_id] = {
                "c_id": c_id,
                "baseline_outcome": baseline.get("baseline_outcome", ""),
                "baseline_source": baseline.get("baseline_source", ""),
                "outcome": "blocked",
                "analysis": "agent response did not include this entry",
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


def write_report(baselines: dict[str, dict], results: dict) -> None:
    result_map = results.get("results", {})
    baseline_counts = Counter(item.get("baseline_outcome", "unknown") for item in baselines.values())
    outcome_counts = Counter(item.get("outcome", "pending") for item in result_map.values())
    comparison_counts = Counter(comparison_bucket(item) for item in result_map.values())
    pending = len(baselines) - len(result_map)
    lines = [
        "# Phase 4F Post-Merge Revalidation",
        "",
        "## Summary",
        "",
        f"- Total selected functions: {len(baselines)}",
        f"- Revalidated: {len(result_map)}",
        f"- Pending: {pending}",
        "",
        "## Worker Baseline",
        "",
    ]
    for key in ("fixed", "ok", "needs_fix", "blocked", "pending"):
        lines.append(f"- {key}: {baseline_counts.get(key, 0)}")
    lines.extend(["", "## Final Revalidation Outcomes", ""])
    for key in ("ok", "needs_fix", "blocked"):
        lines.append(f"- {key}: {outcome_counts.get(key, 0)}")
    lines.extend(["", "## Regression Comparison", ""])
    for key in ("preserved", "regression", "improved", "still_needs_fix", "blocked", "other"):
        lines.append(f"- {key}: {comparison_counts.get(key, 0)}")
    regressions = [
        item for item in result_map.values()
        if comparison_bucket(item) == "regression"
    ]
    if regressions:
        lines.extend(["", "## Potential Regressions", ""])
        for item in sorted(regressions, key=lambda value: value["c_id"]):
            lines.append(
                f"- `{item['c_id']}`: {item.get('baseline_outcome')} -> "
                f"{item.get('outcome')}; {item.get('analysis', '')}"
            )
    REPORT.write_text("\n".join(lines) + "\n")


def print_status(baselines: dict[str, dict], results: dict) -> None:
    result_map = results.get("results", {})
    baseline_counts = Counter(item.get("baseline_outcome", "unknown") for item in baselines.values())
    outcome_counts = Counter(item.get("outcome", "pending") for item in result_map.values())
    comparison_counts = Counter(comparison_bucket(item) for item in result_map.values())
    print(f"phase4f functions selected: {len(baselines)}")
    print(
        "worker baseline: "
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    agent_runner.add_agent_args(parser, model_help_context="Phase 4F revalidation agent")
    parser.add_argument("--status", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--batch-size", type=int, default=10)
    parser.add_argument("--id-list")
    parser.add_argument("--shard-count", type=int, default=1)
    parser.add_argument("--shard-index", type=int, default=0)
    parser.add_argument("--worktree-root", type=Path, default=Path("/private/tmp"))
    parser.add_argument("--clusters", type=int, default=9)
    parser.add_argument("--max-turns", type=int, default=250)
    args = parser.parse_args()
    if args.shard_count < 1:
        parser.error("--shard-count must be at least 1")
    if args.shard_index < 0 or args.shard_index >= args.shard_count:
        parser.error("--shard-index must satisfy 0 <= index < shard-count")

    ids = cluster_ids(args.worktree_root, args.clusters)
    repair_maps = load_repair_maps(args.worktree_root, args.clusters)
    baselines = baseline_for_ids(ids, repair_maps)
    results = load_results()
    write_report(baselines, results)

    if args.status:
        print_status(baselines, results)
        print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
        return 0

    selected_ids = select_entries(
        ids,
        baselines,
        results,
        force=args.force,
        id_list=args.id_list,
        limit=args.limit,
        shard_count=args.shard_count,
        shard_index=args.shard_index,
    )
    print(f"selected Phase 4F revalidation entries: {len(selected_ids)}")
    if args.dry_run:
        for c_id in selected_ids[:80]:
            print(f"  {baselines[c_id].get('baseline_outcome', '')}: {c_id}")
        if len(selected_ids) > 80:
            print(f"  ... and {len(selected_ids) - 80} more")
        return 0

    mapping = build_function_map(load_function_map())
    entries_by_id = mapped_by_id(mapping)
    agent = agent_runner.config_from_args(args)
    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
    processed = 0
    batch_size = max(1, args.batch_size)
    while processed < len(selected_ids):
        batch_ids = selected_ids[processed:processed + batch_size]
        batch: list[dict] = []
        missing: dict[str, dict] = {}
        for c_id in batch_ids:
            entry = entries_by_id.get(c_id)
            if not entry:
                missing[c_id] = {
                    "c_id": c_id,
                    "baseline_outcome": baselines[c_id].get("baseline_outcome", ""),
                    "baseline_source": baselines[c_id].get("baseline_source", ""),
                    "outcome": "blocked",
                    "analysis": "function is missing from refreshed final-tree map",
                    "regression_risk": "possible",
                    "fix_summary": "",
                    "verification": [],
                }
                continue
            item = dict(entry)
            item["baseline"] = baselines[c_id]
            batch.append(item)
        updates = dict(missing)
        if batch:
            prompt = compose_prompt(batch)
            pfile = prompt_path(batch)
            pfile.write_text(prompt)
            print(
                f"[{processed + 1}/{len(selected_ids)}] revalidating "
                f"{len(batch)} function(s)"
            )
            res = agent_runner.run_capture(
                agent,
                prompt,
                repo_root=REPO_ROOT,
                log_path=log_path(agent, batch),
                phase="phase4f",
                label=pfile.stem,
                prompt_file=pfile,
                allowed_tools=READ_ONLY_TOOLS,
                max_turns=args.max_turns,
            )
            if res.returncode != 0:
                updates.update(
                    {
                        item["c_id"]: {
                            "c_id": item["c_id"],
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
                            item["c_id"]: {
                                "c_id": item["c_id"],
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
        write_report(baselines, results)
        print_status(baselines, results)
        processed += len(batch_ids)

    print(f"wrote: {RESULTS.relative_to(REPO_ROOT)}")
    print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
