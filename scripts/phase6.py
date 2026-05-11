#!/usr/bin/env python3
"""phase6.py -- debug remaining failing Rust tests.

Phase 6 runs the Rust test suite, records failing tests, and invokes
an agent to debug the failures.  Phase 5A/5B/5C have already handled
known C/Rust test-correspondence mismatches, so remaining failures
are typically Rust implementation bugs, fixture issues, harness
problems, or test translation bugs that slipped through.

Test code edits are narrowly scoped: the only legitimate changes to
Rust test files are fixes to translation bugs that bring the Rust
test CLOSER to the C test it mirrors.  Edits that move the Rust test
away from the C behavior (relaxing assertions, ignoring tests,
loosening tolerances, etc.) are out of bounds — the implementation
must be fixed instead.

Outputs:
  - xlate/phase6_failures.json
  - xlate/phase6_report.html
  - xlate/phase6_runs/<timestamp>.log
"""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import re
import subprocess
import time
from collections import Counter
from pathlib import Path

import agent_runner
from phase4_common import REPO_ROOT, RS_CRATE, esc, load_json
from phase5_common import XLATE, load_test_map, source_body, write_html

try:
    import fcntl
except ImportError:  # pragma: no cover - translation tooling runs on POSIX.
    fcntl = None

FAILURES = XLATE / "phase6_failures.json"
FAILURES_LOCK = XLATE / "phase6_failures.lock"
REPORT = XLATE / "phase6_report.html"
RUNS_DIR = XLATE / "phase6_runs"
PROMPTS_DIR = XLATE / "prompts" / "phase6"
CLUSTERS_JSON = XLATE / "phase6_clusters.json"
CLUSTERS_SCRIPT = REPO_ROOT / "scripts" / "phase6_clusters.py"

# The test suite needs a real TLS provider (PEM cert loading, certificate
# verification, etc.).  The minicrypto provider that ships under the
# default feature set has no cert-chain loader, so every test that opens
# a real certificate fails at Quic::new.  Phase 4/5 historically ran the
# gate with `--features sys-openssl`; do the same here.
CARGO_FEATURES = ["--features", "sys-openssl"]

# Per-test timeout: tests exceeding this are SIGKILL'd by nextest (see
# rs/fq/.config/nextest.toml).  Phase 6 needs fast feedback; legitimately
# slow tests should be marked rather than running unbounded.
PER_TEST_TIMEOUT_SECONDS = 120

# Overall safety net for the whole nextest invocation.  Should never fire
# in practice — per-test killing is the primary mechanism.
SUITE_TIMEOUT_SECONDS = 1800

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
    "Bash(python3 scripts/phase3_check.py:*)"
)


@contextlib.contextmanager
def failures_file_lock():
    FAILURES_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with FAILURES_LOCK.open("a") as lock_file:
        if fcntl is not None:
            fcntl.flock(lock_file, fcntl.LOCK_EX)
        try:
            yield
        finally:
            if fcntl is not None:
                fcntl.flock(lock_file, fcntl.LOCK_UN)


def load_failures() -> dict:
    with failures_file_lock():
        failures = load_json(
            FAILURES,
            {"schema_version": 1, "summary": {}, "failures": {}},
        )
        failures.setdefault("summary", {})
        failures.setdefault("failures", {})
        return failures


def save_failures(data: dict) -> dict:
    with failures_file_lock():
        tmp = FAILURES.with_name(f"{FAILURES.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
        tmp.replace(FAILURES)
        return data


def run_cargo_test() -> dict:
    RUNS_DIR.mkdir(parents=True, exist_ok=True)
    stamp = time.strftime("%Y%m%d-%H%M%S")
    log_path = RUNS_DIR / f"{stamp}.log"
    env = os.environ.copy()
    env["CARGO_INCREMENTAL"] = "0"
    env.setdefault("CARGO_TARGET_DIR", "/private/tmp/fq-target")
    # nextest enforces the per-test timeout via .config/nextest.toml
    # (slow-timeout, terminate-after).  We still pass an overall
    # subprocess.run timeout as a belt-and-suspenders measure.
    # Per-test slow-timeout + terminate-after live in
    # rs/fq/.config/nextest.toml; nextest 0.9 does not accept those as
    # CLI flags.  PER_TEST_TIMEOUT_SECONDS is kept here for visibility.
    _ = PER_TEST_TIMEOUT_SECONDS
    cmd = [
        "cargo",
        "nextest",
        "run",
        *CARGO_FEATURES,
        "--no-fail-fast",
        "--final-status-level=fail",
    ]
    try:
        res = subprocess.run(
            cmd,
            cwd=RS_CRATE,
            env=env,
            capture_output=True,
            text=True,
            timeout=SUITE_TIMEOUT_SECONDS,
        )
        output = res.stdout + "\n" + res.stderr
        returncode = res.returncode
    except subprocess.TimeoutExpired as exc:
        output = (exc.stdout or "") + "\n" + (exc.stderr or "")
        output += f"\n\nphase6: nextest exceeded {SUITE_TIMEOUT_SECONDS}s and was killed.\n"
        returncode = 124
    log_path.write_text(output)
    return parse_nextest_output(output, returncode, log_path)


SUMMARY_RE = re.compile(
    r"test result: (?P<status>ok|FAILED)\. "
    r"(?P<passed>\d+) passed; (?P<failed>\d+) failed; "
    r"(?P<ignored>\d+) ignored; (?P<measured>\d+) measured; "
    r"(?P<filtered>\d+) filtered out",
)

# Nextest summary line, e.g.:
#   Summary [   29.5s] 740 tests run: 336 passed, 404 failed, 1 skipped
NEXTEST_SUMMARY_RE = re.compile(
    r"Summary \[\s*[\d.]+s\] (?P<total>\d+) tests run:"
    r"\s*(?P<passed>\d+) passed"
    r"(?:, (?P<flaky>\d+) flaky)?"
    r"(?:, (?P<failed>\d+) failed)?"
    r"(?:, (?P<timed_out>\d+) timed out)?"
    r"(?:, (?P<leaky>\d+) leaky)?"
    r"(?:, (?P<skipped>\d+) skipped)?",
)

# Nextest per-test outcome line, e.g.:
#         FAIL [   0.025s] (3/3) fq tests::tls_api::af_undef
#         PASS [   0.001s] fq tests::tls_api::af_undef
#         TMOUT [ 120.000s] fq tests::tls_api::integrity_limit
# The crate-name token (`fq`) is the package name; everything after it
# is the test path used by `cargo test`.
NEXTEST_OUTCOME_RE = re.compile(
    r"^\s*(?P<outcome>PASS|FAIL|TMOUT|TIMEOUT|LEAK|SLOW|SKIP)"
    r"(?:\s*\[[^\]]*\])?"
    r"\s*(?:\(\d+/\d+\))?"
    r"\s*\S+\s+"
    r"(?P<name>\S+)\s*$",
    re.MULTILINE,
)

NEXTEST_FAILURE_OUTCOMES = {"FAIL", "TMOUT", "TIMEOUT", "LEAK"}


def parse_nextest_output(output: str, returncode: int, log_path: Path) -> dict:
    summaries = list(NEXTEST_SUMMARY_RE.finditer(output))
    if summaries:
        last = summaries[-1]

        def _int(name: str) -> int:
            val = last.group(name)
            return int(val) if val else 0

        summary = {
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "command": "cargo nextest run",
            "returncode": returncode,
            "passed": _int("passed"),
            "failed": _int("failed") + _int("timed_out") + _int("leaky"),
            "timed_out": _int("timed_out"),
            "flaky": _int("flaky"),
            "skipped": _int("skipped"),
            "log": log_path.relative_to(REPO_ROOT).as_posix(),
        }
    else:
        summary = {
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "command": "cargo nextest run",
            "returncode": returncode,
            "passed": 0,
            "failed": 1 if returncode != 0 else 0,
            "timed_out": 0,
            "flaky": 0,
            "skipped": 0,
            "log": log_path.relative_to(REPO_ROOT).as_posix(),
        }

    names: set[str] = set()
    for match in NEXTEST_OUTCOME_RE.finditer(output):
        if match.group("outcome") in NEXTEST_FAILURE_OUTCOMES:
            names.add(match.group("name"))

    failure_map: dict[str, dict] = {}
    if not names and returncode != 0:
        names.add("cargo_test_build_or_harness_failure")
    for name in sorted(names):
        failure_map[name] = {
            "name": name,
            "status": "pending",
            "excerpt": failure_excerpt(output, name),
            "log": log_path.relative_to(REPO_ROOT).as_posix(),
            "analysis": "",
            "fix_summary": "",
            "files_changed": [],
            "verification": [],
        }
    return {"schema_version": 1, "summary": summary, "failures": failure_map}


def failure_excerpt(output: str, name: str) -> str:
    header = f"---- {name} stdout ----"
    start = output.find(header)
    if start >= 0:
        next_header = output.find("\n---- ", start + len(header))
        failures = output.find("\nfailures:", start + len(header))
        ends = [idx for idx in (next_header, failures) if idx >= 0]
        end = min(ends) if ends else min(len(output), start + 6000)
        return output[start:end].strip()[:6000]

    line_match = re.search(rf"^test {re.escape(name)} \.\.\. FAILED$", output, re.MULTILINE)
    if line_match:
        start = max(0, line_match.start() - 2500)
        end = min(len(output), line_match.end() + 3500)
        return output[start:end].strip()[:6000]
    return output[-6000:]


def test_module_file(test_name: str) -> str | None:
    parts = test_name.split("::")
    if "tests" not in parts:
        return None
    idx = parts.index("tests")
    if idx + 1 >= len(parts):
        return None
    module = parts[idx + 1]
    return f"rs/fq/src/tests/{module}.rs"


def related_test_entry(mapping: dict, failure_name: str) -> dict | None:
    rust_name = failure_name.split("::")[-1]
    module_file = test_module_file(failure_name)
    candidates = [
        entry for entry in mapping.get("entries", [])
        if entry.get("rust_test_name") == rust_name
    ]
    if module_file:
        for entry in candidates:
            if entry.get("expected_rust_file") == module_file:
                return entry
    return candidates[0] if candidates else None


def work_failures(failures: dict, *, only: str | None, force: bool) -> list[dict]:
    out: list[dict] = []
    for name, failure in failures.get("failures", {}).items():
        if only and only != name and only not in name:
            continue
        if not force and failure.get("status") in {"fixed", "ok"}:
            continue
        out.append(failure)
    out.sort(key=lambda f: f.get("name", ""))
    return out


def prompt_path(batch: list[dict]) -> Path:
    label = "__".join(failure["name"].split("::")[-1] for failure in batch[:3])
    if len(batch) > 3:
        label += f"__plus_{len(batch) - 3}"
    safe = "".join(ch if ch.isalnum() or ch in "._-" else "_" for ch in label)
    return PROMPTS_DIR / f"{safe}.md"


def log_path(agent: agent_runner.AgentConfig, batch: list[dict]) -> Path:
    return agent_runner.log_dir(REPO_ROOT, agent, "phase6") / f"{prompt_path(batch).stem}.log"


def compose_prompt(batch: list[dict], mapping: dict) -> str:
    sections: list[str] = []
    for failure in batch:
        entry = related_test_entry(mapping, failure["name"])
        c = entry.get("c") if entry else None
        rust = entry.get("rust") if entry else None
        sections.extend(
            [
                f"## `{failure['name']}`",
                f"* Related Rust test file: `{test_module_file(failure['name']) or 'unknown'}`",
                f"* Cargo log: `{failure.get('log', '')}`",
                "",
                "### Failure excerpt",
                "```text",
                failure.get("excerpt", ""),
                "```",
                "",
            ]
        )
        if entry:
            sections.extend(
                [
                    f"* C test-table name: `{entry.get('test_name')}`",
                    f"* C entry function: `{entry.get('entry_fn')}`",
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

    return "\n".join(
        [
            "# Phase 6 debug failing Rust tests",
            "",
            "Debug the failing Rust tests below.  Phase 5A/5B are meant",
            "to have addressed known C/Rust test translation mismatches",
            "and Phase 5C revalidates that correspondence after merges;",
            "remaining failures may be Rust implementation bugs, fixture",
            "issues, harness problems, or test translation bugs that",
            "slipped through.",
            "",
            "Rules:",
            "",
            "* Do not edit C sources.",
            "* Prefer the smallest faithful Rust fix.  If the implementation",
            "  is wrong, fix the implementation.",
            "* The ONLY legitimate edits to Rust test code are fixes to",
            "  translation bugs that bring the Rust test CLOSER to the C",
            "  test it mirrors (wrong constant, off-by-one in setup,",
            "  miswired fixture, harness call whose semantics diverge from",
            "  the C counterpart, etc.).",
            "* Do NOT weaken tests to make them pass: do not delete or",
            "  relax assertions, loosen tolerances, skip cases, mark tests",
            "  `#[ignore]`, or otherwise reduce coverage.  Any edit that",
            "  moves the Rust test AWAY from the C test's behavior is out",
            "  of bounds — fix the implementation (or fixture/harness)",
            "  instead.",
            "* Run the narrowest useful `cargo test <name>` command after",
            "  edits when practical.",
            "* Report `blocked` only with a concrete human-actionable reason.",
            "",
            "Return final JSON with this shape:",
            "",
            "```json",
            "{\"debug\":[{\"name\":\"test::path\",\"outcome\":\"fixed|ok|blocked\","
            "\"analysis\":\"root cause\","
            "\"fix_summary\":\"what changed, or empty\","
            "\"files_changed\":[\"rs/fq/src/...\"],"
            "\"verification\":[\"cargo test ...\"]}]}",
            "```",
            "",
            "Failures:",
            "",
            "\n".join(sections),
        ]
    )


def refresh_clusters() -> list[dict]:
    """Rebuild the cluster snapshot by invoking scripts/phase6_clusters.py."""
    subprocess.run(
        ["python3", str(CLUSTERS_SCRIPT)],
        cwd=REPO_ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    return load_clusters_from_disk()


def load_clusters_from_disk() -> list[dict]:
    if not CLUSTERS_JSON.is_file():
        return []
    data = json.loads(CLUSTERS_JSON.read_text())
    return data.get("clusters", [])


def select_cluster(clusters: list[dict], *, index: int | None, match: str | None) -> dict | None:
    """Pick one cluster by 1-based index or by substring match against the location."""
    candidates = [c for c in clusters if c.get("size", 0) >= 2]
    if not candidates:
        return None
    if index is not None:
        if 1 <= index <= len(candidates):
            return candidates[index - 1]
        return None
    if match is not None:
        for c in candidates:
            if match in c.get("location", "") or any(match in t for t in c.get("tests", [])):
                return c
        return None
    return candidates[0]


def cluster_failure_records(cluster: dict, failures: dict) -> list[dict]:
    """Return the failure-map entries (pending only) that belong to this cluster."""
    records = []
    failure_map = failures.get("failures", {})
    for name in cluster.get("tests", []):
        entry = failure_map.get(name)
        if entry and entry.get("status") == "pending":
            records.append(entry)
    return records


def compose_cluster_prompt(cluster: dict, failures: dict, mapping: dict) -> str:
    """Build the agent prompt for a whole cluster of failures sharing a panic."""
    representative_name = cluster.get("representative") or cluster["tests"][0]
    representative = failures["failures"].get(representative_name, {"name": representative_name, "excerpt": ""})
    entry = related_test_entry(mapping, representative_name)
    c_body = entry.get("c") if entry else None
    rust_body = entry.get("rust") if entry else None

    sibling_lines = "\n".join(f"* `{name}`" for name in cluster["tests"])

    rep_sections = [
        f"## Representative failure: `{representative_name}`",
        f"* Related Rust test file: `{test_module_file(representative_name) or 'unknown'}`",
        f"* Cargo log: `{representative.get('log', '')}`",
        "",
        "### Failure excerpt",
        "```text",
        representative.get("excerpt", ""),
        "```",
        "",
    ]
    if entry:
        rep_sections.extend(
            [
                f"* C test-table name: `{entry.get('test_name')}`",
                f"* C entry function: `{entry.get('entry_fn')}`",
                "",
                "### C test body",
                "```c",
                source_body(c_body),
                "```",
                "",
                "### Rust test body",
                "```rust",
                source_body(rust_body),
                "```",
                "",
            ]
        )

    return "\n".join(
        [
            "# Phase 6 cluster debug",
            "",
            f"You are debugging a **cluster** of {cluster['size']} Rust tests that all",
            f"panic at **`{cluster['location']}`** with the same message:",
            "",
            f"> {cluster['message']}",
            "",
            "Tests sharing a panic fingerprint almost always share a single",
            "underlying root cause — fix the implementation (or test-helper)",
            "once and the whole cluster turns green.  Do not try to debug",
            "each test individually; investigate the shared code path.",
            "",
            "## All tests in this cluster",
            "",
            sibling_lines,
            "",
            "\n".join(rep_sections),
            "",
            "## Rules",
            "",
            "* Do not edit C sources.",
            "* Prefer the smallest faithful Rust fix to the implementation,",
            "  fixture setup, or test-side helper that is shared across the",
            "  whole cluster.  A fix that touches only one test is almost",
            "  always wrong here.",
            "* The ONLY legitimate edits to Rust test code are fixes to",
            "  translation bugs that bring the Rust test CLOSER to the C",
            "  test it mirrors.  Edits that move a test AWAY from the C",
            "  behavior (relaxing assertions, ignoring tests, loosening",
            "  tolerances) are out of bounds — fix the implementation",
            "  instead.",
            "* Compare against the C source.  Look at the C twin of the",
            "  representative test, walk through its setup, and inspect the",
            "  code path the panic message implicates.",
            "* Verify the fix by running the representative test plus at",
            "  least 3 other sibling tests from this cluster.",
            "* Report `blocked` only with a concrete human-actionable reason.",
            "",
            "## Return format",
            "",
            "End your response with a JSON object using this shape so",
            "phase6.py can update its failure map.  Include an entry for",
            "every cluster member you verified passing or determined to",
            "share the fix:",
            "",
            "```json",
            "{\"debug\":[{\"name\":\"test::path\",\"outcome\":\"fixed|ok|blocked\","
            "\"analysis\":\"root cause shared across cluster\","
            "\"fix_summary\":\"what changed, or empty\","
            "\"files_changed\":[\"rs/fq/src/...\"],"
            "\"verification\":[\"cargo test ...\"]}]}",
            "```",
        ]
    )


def extract_json(text: str) -> dict:
    candidates: list[dict] = []
    for fenced in re.finditer(r"```(?:json)?\s*(\{.*?\})\s*```", text, re.DOTALL):
        try:
            obj = json.loads(fenced.group(1))
        except json.JSONDecodeError:
            continue
        if isinstance(obj, dict) and "debug" in obj:
            candidates.append(obj)
    decoder = json.JSONDecoder()
    for match in re.finditer(r"\{", text):
        try:
            obj, _ = decoder.raw_decode(text[match.start():])
        except json.JSONDecodeError:
            continue
        if isinstance(obj, dict) and "debug" in obj:
            candidates.append(obj)
    if candidates:
        return candidates[-1]
    raise ValueError("no JSON object found")


def normalize_debug(raw: dict, batch: list[dict]) -> dict[str, dict]:
    def string_list(value: object) -> list[str]:
        if not isinstance(value, list):
            return []
        return [str(item)[:300] for item in value[:20]]

    batch_names = {failure["name"] for failure in batch}
    by_name: dict[str, dict] = {}
    for item in raw.get("debug", []):
        name = item.get("name")
        outcome = item.get("outcome")
        if name not in batch_names or outcome not in OUTCOMES:
            continue
        by_name[name] = {
            "status": outcome,
            "analysis": str(item.get("analysis", ""))[:2000],
            "fix_summary": str(item.get("fix_summary", ""))[:2000],
            "files_changed": string_list(item.get("files_changed")),
            "verification": string_list(item.get("verification")),
        }
    for failure in batch:
        if failure["name"] not in by_name:
            by_name[failure["name"]] = {
                "status": "blocked",
                "analysis": "agent response did not include this failure",
                "fix_summary": "",
                "files_changed": [],
                "verification": [],
            }
    return by_name


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


def run_targeted_tests(batch: list[dict], *, skip_gate: bool) -> int:
    if skip_gate:
        return 0
    env = os.environ.copy()
    env["CARGO_INCREMENTAL"] = "0"
    env.setdefault("CARGO_TARGET_DIR", "/private/tmp/fq-target")
    for failure in batch:
        name = failure["name"]
        if name == "cargo_test_build_or_harness_failure":
            cmd = ["cargo", "test", *CARGO_FEATURES, "--no-run"]
        else:
            cmd = ["cargo", "test", *CARGO_FEATURES, name, "--", "--exact"]
        res = subprocess.run(cmd, cwd=RS_CRATE, env=env)
        if res.returncode != 0:
            return res.returncode
    for cmd in (
        ["cargo", "fmt"],
        ["cargo", "test", *CARGO_FEATURES, "--no-run"],
        ["cargo", "clippy", "--tests", "--all-features", "--", "-D", "warnings"],
    ):
        res = subprocess.run(cmd, cwd=RS_CRATE, env=env)
        if res.returncode != 0:
            return res.returncode
    return 0


def apply_debug_updates(failures: dict, updates: dict[str, dict]) -> dict:
    failure_map = failures.setdefault("failures", {})
    for name, update in updates.items():
        if name not in failure_map:
            continue
        failure_map[name].update(update)
    return save_failures(failures)


def write_report(failures: dict) -> None:
    summary = failures.get("summary", {})
    failure_map = failures.get("failures", {})
    counts = Counter(failure.get("status", "pending") for failure in failure_map.values())
    body: list[str] = [
        "<h1>Phase 6 Failing Test Debug</h1>",
        "<h2>Latest Cargo Test Summary</h2>",
        "<table><tr><th>Metric</th><th>Value</th></tr>",
    ]
    for key in ("passed", "failed", "ignored", "measured", "filtered_out", "returncode", "log"):
        body.append(f"<tr><td>{esc(key)}</td><td>{esc(summary.get(key, ''))}</td></tr>")
    body.append("</table>")

    body.append("<h2>Debug Status</h2>")
    body.append("<table><tr><th>Status</th><th>Count</th></tr>")
    for status in ("pending", "fixed", "ok", "blocked"):
        body.append(f"<tr><td>{esc(status)}</td><td>{counts.get(status, 0)}</td></tr>")
    body.append("</table>")

    body.append("<h2>Failures</h2>")
    body.append(
        "<table><tr><th>Test</th><th>Status</th><th>Analysis</th>"
        "<th>Fix</th><th>Excerpt</th></tr>"
    )
    if not failure_map:
        body.append("<tr><td colspan=\"5\"><em>none</em></td></tr>")
    for name, failure in sorted(failure_map.items()):
        body.append(
            "<tr>"
            f"<td><code>{esc(name)}</code></td>"
            f"<td>{esc(failure.get('status', ''))}</td>"
            f"<td>{esc(failure.get('analysis', ''))}</td>"
            f"<td>{esc(failure.get('fix_summary', ''))}</td>"
            f"<td><pre>{esc(failure.get('excerpt', '')[:1200])}</pre></td>"
            "</tr>"
        )
    body.append("</table>")
    write_html(REPORT, "Phase 6 Test Debug", "\n".join(body))


def print_status(failures: dict) -> None:
    summary = failures.get("summary", {})
    failure_map = failures.get("failures", {})
    counts = Counter(failure.get("status", "pending") for failure in failure_map.values())
    print(
        "phase6 latest cargo test: "
        f"passed={summary.get('passed', 0)} failed={summary.get('failed', 0)} "
        f"returncode={summary.get('returncode', 'unknown')}"
    )
    print("phase6 debug:")
    for status in ("pending", "fixed", "ok", "blocked"):
        print(f"  {status:8s} {counts.get(status, 0)}")
    if summary.get("log"):
        print(f"log: {summary.get('log')}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    agent_runner.add_agent_args(parser, model_help_context="Phase 6 debug agent")
    parser.add_argument("--status", action="store_true", help="show Phase 6 status")
    parser.add_argument("--refresh-failures", action="store_true",
                        help="run cargo test and refresh the failure list before debugging")
    parser.add_argument("--dry-run", action="store_true", help="print selected failures without invoking an agent")
    parser.add_argument("--force", action="store_true", help="debug failures even if already fixed/ok")
    parser.add_argument("--only", help="limit to one failure name or substring")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--batch-size", type=int, default=1)
    parser.add_argument("--max-turns", type=int, default=250)
    parser.add_argument("--skip-gate", action="store_true", help="skip targeted cargo gates after a batch")
    parser.add_argument("--full-gate", action="store_true", help="run full cargo test after all debug batches")
    parser.add_argument(
        "--cluster",
        nargs="?",
        const="top",
        default=None,
        metavar="TARGET",
        help=(
            "dispatch one agent against an entire shared-panic cluster instead "
            "of one failure at a time.  TARGET may be 'top' (largest cluster), a "
            "1-based index into the cluster list, or a substring of the cluster's "
            "panic location.  Reads xlate/phase6_clusters.json; pass "
            "--refresh-clusters to rebuild it first."
        ),
    )
    parser.add_argument(
        "--refresh-clusters",
        action="store_true",
        help="run scripts/phase6_clusters.py before dispatching (only with --cluster)",
    )
    args = parser.parse_args()

    if (args.status or args.dry_run) and not args.refresh_failures and not FAILURES.is_file():
        print("no Phase 6 failure snapshot; run with --refresh-failures to collect one")
        return 0

    if args.refresh_failures or not FAILURES.is_file():
        failures = save_failures(run_cargo_test())
    else:
        failures = load_failures()

    if args.status:
        print_status(failures)
        return 0

    if args.cluster is not None:
        if args.refresh_clusters or not CLUSTERS_JSON.is_file():
            clusters = refresh_clusters()
        else:
            clusters = load_clusters_from_disk()
        if not clusters:
            print(
                "no clusters available; run scripts/phase6_clusters.py "
                "or pass --refresh-clusters",
            )
            return 1
        index: int | None = None
        match: str | None = None
        target = args.cluster
        if target == "top":
            index = 1
        else:
            try:
                index = int(target)
            except ValueError:
                match = target
        cluster = select_cluster(clusters, index=index, match=match)
        if cluster is None:
            print(f"no cluster matched --cluster={target}")
            return 1
        batch = cluster_failure_records(cluster, failures)
        if not batch:
            print(
                f"cluster `{cluster['location']}` has no pending failures; "
                "re-run --refresh-failures or pick a different cluster",
            )
            return 1
        print(
            f"cluster: {cluster['size']} tests at "
            f"`{cluster['location']}` — {cluster['message'][:70]}"
        )
        print(f"representative: {cluster.get('representative') or batch[0]['name']}")
        if args.dry_run:
            for failure in batch[:80]:
                print(f"  {failure['status']:8s} {failure['name']}")
            if len(batch) > 80:
                print(f"  ... and {len(batch) - 80} more")
            return 0
        agent = agent_runner.config_from_args(args)
        PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
        mapping = load_test_map(refresh=True)
        prompt = compose_cluster_prompt(cluster, failures, mapping)
        pfile = PROMPTS_DIR / f"cluster__{prompt_path(batch[:3]).stem}.md"
        pfile.write_text(prompt)
        before_rs = rs_diff_names()
        print(f"dispatching cluster agent on {len(batch)} pending failure(s)")
        res = agent_runner.run_capture(
            agent,
            prompt,
            repo_root=REPO_ROOT,
            log_path=log_path(agent, batch[:3]),
            phase="phase6",
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
        except ValueError:
            print("agent did not return parseable JSON; leaving failures untouched")
            parsed = {"debug": []}
        updates = normalize_debug(parsed, batch)
        failures = apply_debug_updates(failures, updates)
        after_rs = rs_diff_names()
        if after_rs != before_rs or any(u.get("status") == "fixed" for u in updates.values()):
            gate = run_targeted_tests(batch, skip_gate=args.skip_gate)
            if gate != 0:
                print(f"targeted gate failed with exit {gate}")
                return gate
        if args.full_gate:
            failures = save_failures(run_cargo_test())
        write_report(failures)
        print_status(failures)
        print(f"wrote: {FAILURES.relative_to(REPO_ROOT)}")
        print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
        return 0

    selected = work_failures(failures, only=args.only, force=args.force)
    if args.limit is not None:
        selected = selected[: args.limit]

    print(f"selected Phase 6 failing tests: {len(selected)}")
    if args.dry_run:
        for failure in selected[:80]:
            print(f"  {failure['status']:8s} {failure['name']}")
        if len(selected) > 80:
            print(f"  ... and {len(selected) - 80} more")
        return 0

    agent = agent_runner.config_from_args(args)
    PROMPTS_DIR.mkdir(parents=True, exist_ok=True)
    mapping = load_test_map(refresh=True)
    processed = 0
    batch_size = max(1, args.batch_size)
    while processed < len(selected):
        batch = selected[processed:processed + batch_size]
        prompt = compose_prompt(batch, mapping)
        pfile = prompt_path(batch)
        pfile.write_text(prompt)
        before_rs = rs_diff_names()
        print(f"[{processed + 1}/{len(selected)}] debugging {len(batch)} failure(s)")
        res = agent_runner.run_capture(
            agent,
            prompt,
            repo_root=REPO_ROOT,
            log_path=log_path(agent, batch),
            phase="phase6",
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
            updates = normalize_debug(parsed, batch)
        except (ValueError, json.JSONDecodeError) as exc:
            updates = {
                failure["name"]: {
                    "status": "blocked",
                    "analysis": f"could not parse agent JSON: {exc}",
                    "fix_summary": "",
                    "files_changed": [],
                    "verification": [],
                }
                for failure in batch
            }

        failures = apply_debug_updates(failures, updates)
        after_rs = rs_diff_names()
        if after_rs != before_rs or any(
            update.get("status") == "fixed" for update in updates.values()
        ):
            gate = run_targeted_tests(batch, skip_gate=args.skip_gate)
            if gate != 0:
                print(f"targeted gate failed with exit {gate}")
                return gate
        processed += len(batch)

    if args.full_gate:
        failures = save_failures(run_cargo_test())
    write_report(failures)
    print_status(failures)
    print(f"wrote: {FAILURES.relative_to(REPO_ROOT)}")
    print(f"wrote: {REPORT.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
