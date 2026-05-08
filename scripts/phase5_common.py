#!/usr/bin/env python3
"""Shared helpers for Phase 5 test correspondence and repair scripts."""

from __future__ import annotations

import re
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import phase3_check
from phase4_common import (
    REPO_ROOT,
    RS_CRATE,
    XLATE,
    find_matching_brace,
    find_matching_delim,
    find_matching_rust_brace,
    html_page,
    line_for_offset,
    line_offsets,
    load_json,
    rel,
    save_json,
)

PICOQUICTEST = REPO_ROOT / "picoquictest"
DRIVER = REPO_ROOT / "picoquic_t" / "picoquic_t.c"
PICOQUIC_CORE = REPO_ROOT / "picoquic"
RS_TESTS = RS_CRATE / "src" / "tests"
TEST_MAP = XLATE / "test_translation_map.json"
SENTINEL_BUCKETS = {
    "<harness>": "harness",
    "<unknown>": "unclassified",
}


@dataclass
class SourceSpan:
    name: str
    file: str
    start_line: int
    end_line: int
    body_start_line: int | None = None
    body_end_line: int | None = None
    signature: str | None = None

    def as_dict(self) -> dict[str, Any]:
        return asdict(self)


def now() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S")


def source_body(span_info: dict[str, Any] | None) -> str:
    if not span_info:
        return ""
    path = REPO_ROOT / span_info["file"]
    if not path.is_file():
        return ""
    lines = path.read_text(errors="replace").splitlines()
    start = max(1, int(span_info.get("body_start_line") or span_info.get("start_line") or 1))
    end = max(start, int(span_info.get("body_end_line") or span_info.get("end_line") or start))
    return "\n".join(lines[start - 1:end])


def c_source_path(src_basename: str, entry_fn: str) -> Path | None:
    if entry_fn == "sim_link_test":
        return PICOQUIC_CORE / "sim_link.c"
    if src_basename == "<harness>":
        return DRIVER
    if src_basename.startswith("<"):
        return None
    for ext in (".c", ".cpp"):
        candidate = PICOQUICTEST / f"{src_basename}{ext}"
        if candidate.is_file():
            return candidate
    # Some C++ harness entries are manually bucketed as cplusplus.
    if entry_fn == "cplusplustest":
        candidate = PICOQUICTEST / "cplusplus.cpp"
        if candidate.is_file():
            return candidate
    return None


def rust_filename_for_source(src_basename: str) -> str:
    return SENTINEL_BUCKETS.get(src_basename, phase3_check.rust_filename_for(src_basename))


def _find_c_signature_brace(text: str, name_end: int) -> int | None:
    paren = text.find("(", name_end)
    if paren < 0 or paren - name_end > 400:
        return None
    close = find_matching_delim(text, paren, "(", ")")
    if close is None:
        return None
    semi = text.find(";", name_end, min(len(text), close + 1200))
    brace = text.find("{", close)
    if brace < 0:
        return None
    if semi >= 0 and semi < brace:
        return None
    return brace


def find_c_function_span(src: Path, name: str) -> SourceSpan | None:
    text = src.read_text(errors="replace")
    offsets = line_offsets(text)
    pat = re.compile(rf"\b{re.escape(name)}\b")
    for m in pat.finditer(text):
        brace = _find_c_signature_brace(text, m.end())
        if brace is None:
            continue
        close = find_matching_brace(text, brace)
        if close is None:
            continue
        sig_start = text.rfind("\n", 0, m.start()) + 1
        while sig_start > 0:
            prev_end = sig_start - 1
            prev_start = text.rfind("\n", 0, prev_end) + 1
            prev = text[prev_start:prev_end].strip()
            if not prev or prev.endswith(";") or prev.endswith("}"):
                break
            sig_start = prev_start
        signature = text[sig_start:brace].strip()
        return SourceSpan(
            name=name,
            file=rel(src),
            start_line=line_for_offset(offsets, sig_start),
            end_line=line_for_offset(offsets, close),
            body_start_line=line_for_offset(offsets, brace),
            body_end_line=line_for_offset(offsets, close),
            signature=re.sub(r"\s+", " ", signature),
        )
    return None


def _rust_test_fn_regex(name: str) -> re.Pattern[str]:
    return re.compile(
        rf"(?P<attrs>(?:\s*#\[[^\n]+\]\s*)*)"
        rf"\bfn\s+{re.escape(name)}\s*\(\s*\)\s*\{{",
        re.MULTILINE,
    )


def _has_test_attr(text: str, attrs_start: int, fn_start: int, attrs: str) -> bool:
    if "#[test]" in attrs:
        return True
    prefix_start = max(0, attrs_start - 400)
    prefix = text[prefix_start:fn_start]
    return bool(re.search(r"#\[test\]\s*(?:#\[[^\n]+\]\s*)*$", prefix))


def find_rust_test_span(rust_path: Path, rust_name: str) -> SourceSpan | None:
    if not rust_path.is_file():
        return None
    text = rust_path.read_text(errors="replace")
    offsets = line_offsets(text)
    for m in _rust_test_fn_regex(rust_name).finditer(text):
        fn_pos = text.find("fn", m.start(), m.end())
        if fn_pos < 0:
            continue
        if not _has_test_attr(text, m.start(), fn_pos, m.group("attrs")):
            continue
        brace = text.rfind("{", m.start(), m.end())
        if brace < 0:
            continue
        close = find_matching_rust_brace(text, brace)
        if close is None:
            continue
        return SourceSpan(
            name=rust_name,
            file=rel(rust_path),
            start_line=line_for_offset(offsets, fn_pos),
            end_line=line_for_offset(offsets, close),
            body_start_line=line_for_offset(offsets, brace),
            body_end_line=line_for_offset(offsets, close),
        )
    return None


def build_test_map() -> dict[str, Any]:
    rows = phase3_check.parse_test_table()
    fn_to_src = phase3_check.index_entry_fns()
    entries: list[dict[str, Any]] = []
    seen_rust_names: dict[str, set[str]] = {}
    for test_name, entry_fn in rows:
        src_basename = fn_to_src.get(entry_fn, "<unknown>")
        rust_file_base = rust_filename_for_source(src_basename)
        rust_name = phase3_check.sanitize_test_name(test_name)
        used = seen_rust_names.setdefault(rust_file_base, set())
        base = rust_name
        suffix = 2
        while rust_name in used:
            rust_name = f"{base}_{suffix}"
            suffix += 1
        used.add(rust_name)

        c_path = c_source_path(src_basename, entry_fn)
        c_span = find_c_function_span(c_path, entry_fn) if c_path else None
        rust_path = RS_TESTS / f"{rust_file_base}.rs"
        rust_span = find_rust_test_span(rust_path, rust_name)

        if c_span and rust_span:
            status = "mapped"
        elif c_span and not rust_span:
            status = "missing_rust"
        elif rust_span and not c_span:
            status = "missing_c"
        else:
            status = "unmapped"

        entries.append(
            {
                "test_id": f"{c_span.file if c_span else 'picoquictest/' + src_basename}:{entry_fn}",
                "test_name": test_name,
                "entry_fn": entry_fn,
                "source_basename": src_basename,
                "rust_test_name": rust_name,
                "expected_rust_file": rel(rust_path),
                "status": status,
                "c": c_span.as_dict() if c_span else None,
                "rust": rust_span.as_dict() if rust_span else None,
            }
        )

    summary: dict[str, int] = {}
    for entry in entries:
        summary[entry["status"]] = summary.get(entry["status"], 0) + 1
    return {
        "schema_version": 1,
        "generated_at": now(),
        "repo_root": str(REPO_ROOT),
        "entries": entries,
        "summary": summary,
    }


def load_test_map(*, refresh: bool = False) -> dict[str, Any]:
    if refresh or not TEST_MAP.is_file():
        mapping = build_test_map()
        save_json(TEST_MAP, mapping)
        return mapping
    return load_json(TEST_MAP, {"schema_version": 1, "entries": [], "summary": {}})


def test_entry_by_id(mapping: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {entry["test_id"]: entry for entry in mapping.get("entries", [])}


def write_html(path: Path, title: str, body: str) -> None:
    path.write_text(html_page(title, body))
