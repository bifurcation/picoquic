#!/usr/bin/env python3
"""phase4c.py — superficial C/Rust function-body correspondence audit.

Phase 4C uses the function map produced by Phase 4A/4B and asks an
agent to do a body-only comparison of each mapped C/Rust pair.  The
prompt includes only function names, source spans, and bodies.  It does
not include dependencies, type definitions, callers, or module context.

Outputs:
  - xlate/phase4c_reviews.json
  - xlate/phase4c_report.html
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
from phase4_common import (
    FUNCTION_MAP,
    REPO_ROOT,
    XLATE,
    esc,
    html_page,
    load_function_map,
    load_json,
    source_body,
)

try:
    import fcntl
except ImportError:  # pragma: no cover - Phase 4 tooling runs on POSIX.
    fcntl = None

REVIEWS = XLATE / "phase4c_reviews.json"
REVIEWS_LOCK = XLATE / "phase4c_reviews.lock"
REPORT = XLATE / "phase4c_report.html"
PROMPTS_DIR = XLATE / "prompts" / "phase4c"

STATUSES = {"ok", "suspect", "definitely_not_ok"}
FALLBACK_RATIONALE_RE = re.compile(
    r"agent response did not include this pair|could not parse agent JSON"
)


def mapped_entries(mapping: dict) -> list[dict]:
    return [
        e for e in mapping.get("entries", [])
        if e.get("rust") and e.get("c")
    ]


def prompt_path(batch: list[dict]) -> Path:
    label = "__".join(e["c"]["name"] for e in batch[:3])
    if len(batch) > 3:
        label += f"__plus_{len(batch) - 3}"
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in label)
    return PROMPTS_DIR / f"{safe}.md"


def log_path(agent: agent_runner.AgentConfig, batch: list[dict]) -> Path:
    return agent_runner.log_dir(REPO_ROOT, agent, "phase4c") / f"{prompt_path(batch).stem}.log"


def compose_prompt(batch: list[dict]) -> str:
    parts = [
        "# Phase 4C body-only translation audit",
        "",
        "Compare each C/Rust pair using only the function bodies shown",
        "below. Do not infer from dependencies, type definitions, callers,",
        "module context, tests, or external knowledge. This is a cheap",
        "superficial check for obvious inconsistencies.",
        "",
        "Return only JSON with this shape:",
        "",
        "```json",
        "{\"reviews\":[{\"c_id\":\"...\",\"status\":\"ok|suspect|definitely_not_ok\",\"rationale\":\"body-visible reason\"}]}",
        "```",
        "",
        "Status meanings:",
        "* `ok`: no obvious body-level concern.",
        "* `suspect`: possible mismatch visible from the bodies.",
        "* `definitely_not_ok`: clear mismatch or placeholder-like code.",
        "",
    ]
    for e in batch:
        c = e["c"]
        rust = e["rust"]
        parts.extend([
            f"## Pair `{e['c_id']}`",
            f"C: `{c['file']}:{c['start_line']}-{c['end_line']} {c['name']}`",
            f"Rust: `{rust['file']}:{rust['start_line']}-{rust['end_line']} {rust['name']}`",
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
        ])
    return "\n".join(parts)


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


def normalize_reviews(raw: dict, batch: list[dict]) -> dict[str, dict]:
    by_id: dict[str, dict] = {}
    for item in raw.get("reviews", []):
        c_id = item.get("c_id")
        status = item.get("status")
        if c_id and status in STATUSES:
            by_id[c_id] = {
                "c_id": c_id,
                "status": status,
                "rationale": str(item.get("rationale", ""))[:1000],
            }
    for e in batch:
        if e["c_id"] not in by_id:
            by_id[e["c_id"]] = {
                "c_id": e["c_id"],
                "status": "suspect",
                "rationale": "agent response did not include this pair",
            }
    return by_id


def is_fallback_review(review: dict | None) -> bool:
    if not review:
        return False
    return bool(FALLBACK_RATIONALE_RE.search(str(review.get("rationale", ""))))


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


def load_reviews_locked() -> dict:
    with reviews_file_lock():
        reviews = load_json(REVIEWS, {"schema_version": 1, "reviews": {}})
        reviews.setdefault("reviews", {})
        return reviews


def update_reviews_locked(updates: dict[str, dict], *, force: bool) -> dict:
    with reviews_file_lock():
        reviews = load_json(REVIEWS, {"schema_version": 1, "reviews": {}})
        review_map = reviews.setdefault("reviews", {})
        for c_id, review in updates.items():
            if force or c_id not in review_map:
                review_map[c_id] = review
        REVIEWS.parent.mkdir(parents=True, exist_ok=True)
        tmp = REVIEWS.with_name(f"{REVIEWS.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(reviews, indent=2, sort_keys=True) + "\n")
        tmp.replace(REVIEWS)
        return reviews


def write_report(mapping: dict, reviews: dict) -> None:
    def report_body(span: dict) -> str:
        return esc("\n".join(line.rstrip() for line in source_body(span).splitlines()))

    entries = mapped_entries(mapping)
    counts = Counter(
        reviews.get(e["c_id"], {}).get("status", "unreviewed")
        for e in entries
    )
    total = len(entries)
    body: list[str] = [
        "<h1>Phase 4C Function Correspondence Audit</h1>",
        f"<p>Map: <code>{esc(FUNCTION_MAP.relative_to(REPO_ROOT))}</code></p>",
        "<h2>Summary</h2>",
        "<table><tr><th>Status</th><th>Count</th><th>Percent</th></tr>",
    ]
    for status in ("ok", "suspect", "definitely_not_ok", "unreviewed"):
        n = counts.get(status, 0)
        pct = 0.0 if total == 0 else 100.0 * n / total
        body.append(f"<tr><td>{esc(status)}</td><td>{n}</td><td>{pct:.1f}%</td></tr>")
    body.append("</table>")

    missing = [e for e in mapping.get("entries", []) if not e.get("rust")]
    body.append("<h2>C Functions Without Rust Equivalents</h2>")
    body.append("<table><tr><th>C function</th><th>C span</th><th>Classification</th><th>Reason</th></tr>")
    if not missing:
        body.append("<tr><td colspan=\"4\"><em>none</em></td></tr>")
    for e in missing:
        c = e["c"]
        body.append(
            "<tr>"
            f"<td><code>{esc(c['name'])}</code></td>"
            f"<td>{esc(c['file'])}:{esc(c['start_line'])}-{esc(c['end_line'])}</td>"
            f"<td>{esc(e.get('required_action'))}</td>"
            f"<td>{esc(e.get('reason', ''))}</td>"
            "</tr>"
        )
    body.append("</table>")

    body.append("<h2>Non-OK Body Comparisons</h2>")
    any_non_ok = False
    for e in entries:
        review = reviews.get(e["c_id"], {})
        status = review.get("status", "unreviewed")
        if status == "ok":
            continue
        any_non_ok = True
        c = e["c"]
        rust = e["rust"]
        css = "bad" if status == "definitely_not_ok" else "suspect"
        body.extend([
            f"<section class=\"{css}\">",
            f"<h3>{esc(status)}: <code>{esc(e['c_id'])}</code></h3>",
            f"<p>{esc(review.get('rationale', ''))}</p>",
            "<div class=\"grid\">",
            f"<div><h4>C {esc(c['file'])}:{esc(c['start_line'])}-{esc(c['end_line'])}</h4><pre>{report_body(c)}</pre></div>",
            f"<div><h4>Rust {esc(rust['file'])}:{esc(rust['start_line'])}-{esc(rust['end_line'])}</h4><pre>{report_body(rust)}</pre></div>",
            "</div></section>",
        ])
    if not any_non_ok:
        body.append("<p><em>No suspect or definitely-not-OK pairs.</em></p>")
    REPORT.write_text(html_page("Phase 4C Audit", "\n".join(body)))


def print_status(mapping: dict, reviews: dict) -> None:
    entries = mapped_entries(mapping)
    counts = Counter(
        reviews.get(e["c_id"], {}).get("status", "unreviewed")
        for e in entries
    )
    print(f"mapped pairs: {len(entries)}")
    for status in ("ok", "suspect", "definitely_not_ok", "unreviewed"):
        print(f"  {status:18s} {counts.get(status, 0)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    agent_runner.add_agent_args(parser, model_help_context="Phase 4C comparison agent")
    parser.add_argument("--status", action="store_true", help="show current review summary")
    parser.add_argument("--dry-run", action="store_true", help="select work but do not invoke an agent")
    parser.add_argument("--force", action="store_true", help="re-review pairs with existing results")
    parser.add_argument("--retry-fallbacks", action="store_true",
                        help="re-review entries with parser/missing-item fallback rationales")
    parser.add_argument("--only", help="limit to one c_id or C function name")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--batch-size", type=int, default=20)
    parser.add_argument("--shard-count", type=int, default=1,
                        help="split mapped pairs across this many parallel workers")
    parser.add_argument("--shard-index", type=int, default=0,
                        help="zero-based worker index when --shard-count is greater than one")
    parser.add_argument("--max-turns", type=int, default=80)
    args = parser.parse_args()
    if args.shard_count < 1:
        parser.error("--shard-count must be at least 1")
    if args.shard_index < 0 or args.shard_index >= args.shard_count:
        parser.error("--shard-index must satisfy 0 <= index < shard-count")

    mapping = load_function_map()
    reviews = load_reviews_locked()
    review_map = reviews.setdefault("reviews", {})

    if args.status:
        if not mapping.get("entries"):
            print("no function map found; run `python3 scripts/phase4a.py` first")
            return 0
        print_status(mapping, review_map)
        return 0

    if not mapping.get("entries"):
        print("no function map found; run `python3 scripts/phase4a.py` first")
        return 2

    selected = mapped_entries(mapping)
    if args.only:
        selected = [
            e for e in selected
            if e["c_id"] == args.only or e["c"]["name"] == args.only
        ]
    if args.retry_fallbacks:
        selected = [
            e for e in selected
            if is_fallback_review(review_map.get(e["c_id"]))
        ]
    if args.shard_count > 1:
        selected = [
            e for i, e in enumerate(selected)
            if i % args.shard_count == args.shard_index
        ]
    if not args.force and not args.retry_fallbacks:
        selected = [e for e in selected if e["c_id"] not in review_map]
    if args.limit is not None:
        selected = selected[: args.limit]
    shard = ""
    if args.shard_count > 1:
        shard = f" for shard {args.shard_index}/{args.shard_count}"
    print(f"selected mapped pairs{shard}: {len(selected)}")
    if args.dry_run:
        for e in selected[:50]:
            print(f"  {e['c_id']} -> {e['rust']['file']}:{e['rust']['start_line']}")
        if len(selected) > 50:
            print(f"  ... and {len(selected) - 50} more")
        return 0

    agent = agent_runner.config_from_args(args)
    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
    for i in range(0, len(selected), max(1, args.batch_size)):
        batch = selected[i:i + max(1, args.batch_size)]
        prompt = compose_prompt(batch)
        pfile = prompt_path(batch)
        pfile.write_text(prompt)
        print(f"[{i + 1}/{len(selected)}] reviewing {len(batch)} pair(s)")
        res = agent_runner.run_capture(
            agent,
            prompt,
            repo_root=REPO_ROOT,
            log_path=log_path(agent, batch),
            phase="phase4c",
            label=pfile.stem,
            prompt_file=pfile,
            allowed_tools=None,
            max_turns=args.max_turns,
        )
        if res.returncode != 0:
            print(f"agent failed with exit {res.returncode}")
            return res.returncode
        try:
            parsed = extract_json(res.stdout + "\n" + res.stderr)
            update_reviews_locked(
                normalize_reviews(parsed, batch),
                force=args.force or args.retry_fallbacks,
            )
        except (ValueError, json.JSONDecodeError) as exc:
            update_reviews_locked({
                e["c_id"]: {
                    "c_id": e["c_id"],
                    "status": "suspect",
                    "rationale": f"could not parse agent JSON: {exc}",
                }
                for e in batch
            }, force=args.force or args.retry_fallbacks)

    reviews = load_reviews_locked()
    review_map = reviews.setdefault("reviews", {})
    write_report(mapping, review_map)
    print_status(mapping, review_map)
    print(f"wrote: {REVIEWS.relative_to(REPO_ROOT)}")
    print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
