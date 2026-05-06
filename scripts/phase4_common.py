#!/usr/bin/env python3
"""Shared helpers for Phase 4A/4B/4C function mapping.

The helpers intentionally use lightweight source scanning rather than a
new parser dependency.  Phase 0 inventory is the source of truth for the
C function list; this module only recovers source spans and best-effort
Rust correspondences.
"""

from __future__ import annotations

import html
import json
import re
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
RS_CRATE = REPO_ROOT / "rs" / "fq"
RS_SRC = RS_CRATE / "src"
XLATE = REPO_ROOT / "xlate"
INVENTORY = XLATE / "inventory.json"
FUNCTION_MAP = XLATE / "function_translation_map.json"

INCOMPLETE_RE = re.compile(
    r"\btodo!\s*\("
    r"|\bunimplemented!\s*\("
    r"|//\s*SKIP\s*:"
    r"|\bTODO\b"
    r"|\bFIXME\b"
    r"|\bXXX\b"
    r"|\bplaceholder\b"
    r"|\bstubs?\b"
    r"|\bstubbed\b"
    r"|\bdeferred\b"
    r"|\bdefer(?:red)?\s+to\b"
    r"|\bPhase\s+4\s+body\b"
    r"|\bnot\s+yet\s+wired\b"
    r"|\bnot\s+yet\s+implemented\b"
    r"|\bnot\s+implemented\b"
    r"|\bout\s+of\s+scope\b",
    re.IGNORECASE,
)


@dataclass
class FunctionSpan:
    name: str
    file: str
    start_line: int
    end_line: int
    body_start_line: int | None = None
    body_end_line: int | None = None
    signature: str | None = None
    is_static: bool | None = None
    is_inline: bool | None = None
    impl_type: str | None = None
    body: str = ""
    c_refs: list[str] = field(default_factory=list)

    def without_body(self) -> dict[str, Any]:
        d = asdict(self)
        d.pop("body", None)
        if not d.get("c_refs"):
            d.pop("c_refs", None)
        if not d.get("impl_type"):
            d.pop("impl_type", None)
        return d


@dataclass(frozen=True)
class RustImplSpan:
    start: int
    end: int
    impl_type: str


@dataclass(frozen=True)
class SemanticMatch:
    c_key: str
    rust: FunctionSpan
    score: int
    reason: str


def load_json(path: Path, default: Any) -> Any:
    if not path.is_file():
        return default
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError:
        return default


def save_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def rel(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def now() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S")


def line_offsets(text: str) -> list[int]:
    offsets = [0]
    for m in re.finditer("\n", text):
        offsets.append(m.end())
    return offsets


def offset_for_line(offsets: list[int], line: int) -> int:
    if line <= 1:
        return 0
    if line - 1 >= len(offsets):
        return offsets[-1]
    return offsets[line - 1]


def line_for_offset(offsets: list[int], offset: int) -> int:
    lo = 0
    hi = len(offsets)
    while lo + 1 < hi:
        mid = (lo + hi) // 2
        if offsets[mid] <= offset:
            lo = mid
        else:
            hi = mid
    return lo + 1


def find_matching_delim(text: str, open_idx: int, open_ch: str, close_ch: str) -> int | None:
    depth = 0
    i = open_idx
    state = "normal"
    quote = ""
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""
        if state == "normal":
            if ch == "/" and nxt == "/":
                state = "line_comment"
                i += 2
                continue
            if ch == "/" and nxt == "*":
                state = "block_comment"
                i += 2
                continue
            if ch in {"\"", "'"}:
                state = "string"
                quote = ch
                i += 1
                continue
            if ch == open_ch:
                depth += 1
            elif ch == close_ch:
                depth -= 1
                if depth == 0:
                    return i
        elif state == "line_comment":
            if ch == "\n":
                state = "normal"
        elif state == "block_comment":
            if ch == "*" and nxt == "/":
                state = "normal"
                i += 2
                continue
        elif state == "string":
            if ch == "\\":
                i += 2
                continue
            if ch == quote:
                state = "normal"
        i += 1
    return None


def find_matching_delim_rust(text: str, open_idx: int, open_ch: str, close_ch: str) -> int | None:
    depth = 0
    i = open_idx
    state = "normal"
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""
        if state == "normal":
            if ch == "/" and nxt == "/":
                state = "line_comment"
                i += 2
                continue
            if ch == "/" and nxt == "*":
                state = "block_comment"
                i += 2
                continue
            if ch == "\"":
                state = "string"
                i += 1
                continue
            if ch == open_ch:
                depth += 1
            elif ch == close_ch:
                depth -= 1
                if depth == 0:
                    return i
        elif state == "line_comment":
            if ch == "\n":
                state = "normal"
        elif state == "block_comment":
            if ch == "*" and nxt == "/":
                state = "normal"
                i += 2
                continue
        elif state == "string":
            if ch == "\\":
                i += 2
                continue
            if ch == "\"":
                state = "normal"
        i += 1
    return None


def find_matching_brace(text: str, open_idx: int) -> int | None:
    return find_matching_delim(text, open_idx, "{", "}")


def find_matching_rust_brace(text: str, open_idx: int) -> int | None:
    return find_matching_delim_rust(text, open_idx, "{", "}")


def find_matching_brace_simple(text: str, open_idx: int) -> int | None:
    depth = 0
    for i in range(open_idx, len(text)):
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i
    return None


def _skip_ws(text: str, idx: int) -> int:
    while idx < len(text):
        if text[idx].isspace():
            idx += 1
            continue
        if text.startswith("//", idx):
            end = text.find("\n", idx + 2)
            idx = len(text) if end < 0 else end + 1
            continue
        if text.startswith("/*", idx):
            end = text.find("*/", idx + 2)
            idx = len(text) if end < 0 else end + 2
            continue
        break
    return idx


def _find_signature_brace(text: str, name_start: int, name_end: int) -> int | None:
    paren = text.find("(", name_end)
    if paren < 0 or paren - name_end > 300:
        return None
    close = find_matching_delim_rust(text, paren, "(", ")")
    if close is None:
        return None
    semi = text.find(";", name_end, min(len(text), close + 1000))
    brace = text.find("{", close)
    if brace < 0:
        return None
    if semi >= 0 and semi < brace:
        return None
    return brace


def _find_rust_signature_brace(text: str, name_end: int) -> int | None:
    paren = text.find("(", name_end)
    if paren < 0 or paren - name_end > 300:
        return None
    close = find_matching_delim_rust(text, paren, "(", ")")
    if close is None:
        return None
    brace = text.find("{", close)
    if brace < 0:
        return None
    semi = text.find(";", close, brace)
    if semi >= 0:
        return None
    return brace


def find_c_function_span(src: Path, name: str, location_line: int, decl: dict[str, Any]) -> FunctionSpan:
    text = src.read_text(errors="replace")
    offsets = line_offsets(text)
    start = max(0, offset_for_line(offsets, max(1, location_line - 3)))
    window_end = min(len(text), start + 50_000)
    pat = re.compile(rf"\b{re.escape(name)}\b")
    for m in pat.finditer(text, start, window_end):
        brace = _find_signature_brace(text, m.start(), m.end())
        if brace is None:
            continue
        close = find_matching_brace(text, brace)
        if close is None:
            continue
        sig_start = text.rfind("\n", 0, m.start()) + 1
        while sig_start > 0:
            prev_line_end = sig_start - 1
            prev_line_start = text.rfind("\n", 0, prev_line_end) + 1
            prev = text[prev_line_start:prev_line_end].strip()
            if not prev or prev.endswith(";") or prev.endswith("}"):
                break
            sig_start = prev_line_start
        return FunctionSpan(
            name=name,
            file=rel(src),
            start_line=line_for_offset(offsets, sig_start),
            end_line=line_for_offset(offsets, close),
            body_start_line=line_for_offset(offsets, brace),
            body_end_line=line_for_offset(offsets, close),
            signature=decl.get("signature"),
            is_static=decl.get("is_static"),
            is_inline=decl.get("is_inline"),
            body=text[brace:close + 1],
        )
    return FunctionSpan(
        name=name,
        file=rel(src),
        start_line=location_line,
        end_line=location_line,
        signature=decl.get("signature"),
        is_static=decl.get("is_static"),
        is_inline=decl.get("is_inline"),
    )


def c_functions_from_inventory() -> list[FunctionSpan]:
    inv = load_json(INVENTORY, {})
    out: list[FunctionSpan] = []
    for f in inv.get("files", []):
        c_file = f.get("file")
        if not c_file or not c_file.startswith("picoquic/"):
            continue
        src = REPO_ROOT / c_file
        if not src.is_file():
            continue
        for d in f.get("decls", []):
            if d.get("kind") != "function" or not d.get("is_definition"):
                continue
            loc = str(d.get("location", ""))
            try:
                line = int(loc.rsplit(":", 1)[1])
            except (IndexError, ValueError):
                line = 1
            out.append(find_c_function_span(src, d["name"], line, d))
    out.sort(key=lambda fn: (fn.file, fn.start_line, fn.name))
    return out


def _extract_c_refs(text: str) -> list[str]:
    refs: list[str] = []
    seen: set[str] = set()

    def add(name: str) -> None:
        if name and name not in seen:
            refs.append(name)
            seen.add(name)

    for raw in re.findall(r"`([^`]+)`", text):
        cleaned = raw.strip()
        if "(" in cleaned:
            before = cleaned.split("(", 1)[0].strip()
            ids = re.findall(r"[A-Za-z_][A-Za-z0-9_]*", before)
            if ids:
                add(ids[-1])
        else:
            ids = re.findall(r"[A-Za-z_][A-Za-z0-9_]*", cleaned)
            if len(ids) == 1:
                add(ids[0])
    for line in text.splitlines():
        if "C:" not in line:
            continue
        tail = line.split("C:", 1)[1]
        for ident in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", tail):
            add(ident)
    return refs


def _preceding_comment(lines: list[str], start_line: int) -> str:
    idx = start_line - 2
    collected: list[str] = []
    blanks = 0
    while idx >= 0 and len(collected) < 40:
        s = lines[idx].strip()
        if not s:
            blanks += 1
            if blanks > 1:
                break
            collected.append(lines[idx])
            idx -= 1
            continue
        blanks = 0
        if s.startswith("//!"):
            break
        if (
            s.startswith("///")
            or s.startswith("// C:")
            or s.startswith("// C ")
            or s.startswith("#[")
            or s.startswith("*")
            or s.startswith("/*")
            or s.endswith("*/")
        ):
            collected.append(lines[idx])
            idx -= 1
            continue
        break
    collected.reverse()
    return "\n".join(collected)


def rust_code_mask(text: str) -> bytearray:
    mask = bytearray(b"\x01") * len(text)
    i = 0
    state = "normal"
    block_depth = 0
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ""
        if state == "normal":
            if ch == "/" and nxt == "/":
                end = text.find("\n", i + 2)
                end = len(text) if end < 0 else end
                mask[i:end] = b"\x00" * (end - i)
                i = end
                continue
            if ch == "/" and nxt == "*":
                mask[i:i + 2] = b"\x00\x00"
                state = "block_comment"
                block_depth = 1
                i += 2
                continue
            if ch == "\"":
                mask[i] = 0
                state = "string"
                i += 1
                continue
        elif state == "block_comment":
            mask[i] = 0
            if ch == "/" and nxt == "*":
                mask[i + 1] = 0
                block_depth += 1
                i += 2
                continue
            if ch == "*" and nxt == "/":
                mask[i + 1] = 0
                block_depth -= 1
                i += 2
                if block_depth == 0:
                    state = "normal"
                continue
        elif state == "string":
            mask[i] = 0
            if ch == "\\":
                if i + 1 < len(text):
                    mask[i + 1] = 0
                i += 2
                continue
            if ch == "\"":
                state = "normal"
        i += 1
    return mask


def _rust_impl_type_from_head(head: str) -> str | None:
    head = re.sub(r"\s+", " ", head).strip()
    if not head:
        return None
    if " where " in head:
        head = head.split(" where ", 1)[0].strip()
    if " for " in head:
        head = head.rsplit(" for ", 1)[1].strip()
    target = head.split("<", 1)[0].strip()
    if "::" in target:
        target = target.rsplit("::", 1)[1].strip()
    ids = re.findall(r"[A-Za-z_][A-Za-z0-9_]*", target)
    return ids[-1] if ids else None


def collect_rust_impl_spans(text: str, code_mask: bytearray | None = None) -> list[RustImplSpan]:
    spans: list[RustImplSpan] = []
    impl_re = re.compile(r"\bimpl\b(?:\s*<[^{};]*>)?\s+(?P<head>[^{};=]+?)\s*\{", re.S)
    for m in impl_re.finditer(text):
        if code_mask is not None and not code_mask[m.start()]:
            continue
        brace = text.find("{", m.start(), m.end() + 1)
        if brace < 0:
            continue
        close = find_matching_rust_brace(text, brace)
        if close is None:
            continue
        impl_type = _rust_impl_type_from_head(m.group("head"))
        if impl_type:
            spans.append(RustImplSpan(start=m.start(), end=close, impl_type=impl_type))
    spans.sort(key=lambda s: (s.start, s.end))
    return spans


def _rust_impl_type_for_offset(impls: list[RustImplSpan], offset: int) -> str | None:
    matches = [span for span in impls if span.start < offset < span.end]
    if not matches:
        return None
    matches.sort(key=lambda s: s.start, reverse=True)
    return matches[0].impl_type


def collect_rust_functions() -> list[FunctionSpan]:
    out: list[FunctionSpan] = []
    fn_re = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<|\()")
    for src in sorted(RS_SRC.rglob("*.rs")):
        text = src.read_text(errors="replace")
        offsets = line_offsets(text)
        lines = text.splitlines()
        code_mask = rust_code_mask(text)
        impls = collect_rust_impl_spans(text, code_mask)
        for m in fn_re.finditer(text):
            if not code_mask[m.start()]:
                continue
            brace = _find_rust_signature_brace(text, m.end())
            if brace is None:
                brace = text.find("{", m.end(), min(len(text), m.end() + 4000))
                if brace < 0:
                    continue
            close = find_matching_rust_brace(text, brace)
            if close is None:
                close = find_matching_brace_simple(text, brace)
                if close is None:
                    continue
            start_line = line_for_offset(offsets, m.start())
            comment = _preceding_comment(lines, start_line)
            c_refs = _extract_c_refs(comment)
            out.append(FunctionSpan(
                name=m.group(1),
                file=rel(src),
                start_line=start_line,
                end_line=line_for_offset(offsets, close),
                body_start_line=line_for_offset(offsets, brace),
                body_end_line=line_for_offset(offsets, close),
                impl_type=_rust_impl_type_for_offset(impls, m.start()),
                body=text[brace:close + 1],
                c_refs=c_refs,
            ))
    return out


def camel_to_snake(name: str) -> str:
    s1 = re.sub("(.)([A-Z][a-z]+)", r"\1_\2", name)
    s2 = re.sub("([a-z0-9])([A-Z])", r"\1_\2", s1)
    return s2.lower()


MANUAL_C_ALIASES: dict[str, list[str]] = {
    "picoquic_create": ["new"],
    "picoquic_create_cnx": ["create_connection"],
    "picoquic_create_client_cnx": ["create_client_connection"],
    "picoquic_create_and_configure": ["create_and_configure"],
    "picoquic_log_fixed_skip": ["skip_fixed"],
    "picoquic_log_varint_skip": ["frames_varint_skip"],
    "picoquic_log_varint": ["frames_varint_decode"],
    "picoquic_log_length": ["read_length"],
    "binlog_pdu_ex": ["pdu"],
    "binlog_packet_ex": ["packet"],
    "binlog_picotls_ticket_ex": ["tls_ticket"],
    "picoquic_parse_long_packet_header": ["parse_long_packet_header_inner"],
    "picoquic_parse_short_packet_header": ["parse_short_packet_header_inner"],
    "picoquic_incoming_version_negotiation": ["incoming_packet_ex"],
    "picoquic_ignore_incoming_handshake": ["incoming_packet_ex"],
    "picoquic_incoming_client_initial": ["incoming_packet_ex"],
    "picoquic_incoming_retry": ["incoming_packet_ex"],
    "picoquic_incoming_server_initial": ["incoming_packet_ex"],
    "picoquic_incoming_server_handshake": ["incoming_packet_ex"],
    "picoquic_incoming_client_handshake": ["incoming_packet_ex"],
    "picoquic_incoming_stateless_reset": ["incoming_packet_ex"],
    "picoquic_incoming_0rtt": ["incoming_packet_ex"],
    "picoquic_incoming_1rtt": ["incoming_packet_ex"],
    "picoquic_incoming_not_decrypted": ["incoming_packet_ex"],
    "picoquic_incoming_segment": ["incoming_packet_ex"],
}

MANUAL_C_ALIAS_IMPLS: dict[str, dict[str, str]] = {
    "picoquic_create": {"new": "Quic"},
    "picoquic_create_cnx": {"create_connection": "Quic"},
    "picoquic_create_client_cnx": {"create_client_connection": "Quic"},
    "picoquic_create_and_configure": {"create_and_configure": "Quic"},
}

C_TYPE_IMPLS: list[tuple[str, str]] = [
    ("picoquic_quic_t", "Quic"),
    ("picoquic_cnx_t", "Connection"),
    ("picoquic_connection_id_t", "ConnectionId"),
    ("picoquic_path_t", "Path"),
    ("picoquic_packet_context_t", "PacketContext"),
    ("picoquic_stream_head", "StreamHead"),
    ("picoquic_stream_head_t", "StreamHead"),
    ("picoquic_sack_item_t", "SackItem"),
    ("picoquic_local_cnxid_t", "LocalConnectionId"),
    ("picoquic_remote_cnxid_t", "RemoteConnectionId"),
    ("picoquic_stateless_packet_t", "StatelessPacket"),
    ("picoquic_stored_ticket_t", "StoredTicket"),
    ("picoquic_stored_token_t", "StoredToken"),
    ("picoquic_packet_t", "Packet"),
    ("picoquic_packet_header", "PacketHeader"),
    ("picosplay_tree_t", "SplayTree"),
    ("picohash_table", "HashTable"),
]


def c_name_candidate_list(name: str) -> list[str]:
    candidates: list[str] = []

    def add(candidate: str) -> None:
        if candidate and candidate not in candidates:
            candidates.append(candidate)

    for alias in MANUAL_C_ALIASES.get(name, []):
        add(alias)
    for candidate in (name, name.lower(), camel_to_snake(name)):
        add(candidate)
    prefixes = [
        "picoquic_",
        "picosocks_",
        "picohash_",
        "picosplay_",
        "bytestream_",
    ]
    for prefix in prefixes:
        if name.startswith(prefix):
            stripped = name[len(prefix):]
            for candidate in (stripped, stripped.lower(), camel_to_snake(stripped)):
                add(candidate)
    for candidate in list(candidates):
        expanded = candidate
        expanded = re.sub(r"\bcnxid\b", "connection_id", expanded)
        expanded = re.sub(r"\bcnx_id\b", "connection_id", expanded)
        expanded = re.sub(r"\bcnx\b", "connection", expanded)
        expanded = expanded.replace("_cnxid_", "_connection_id_")
        expanded = expanded.replace("_cnx_id_", "_connection_id_")
        expanded = expanded.replace("_cnx_", "_connection_")
        if expanded != candidate:
            add(expanded)
        if candidate.startswith("get_") and len(candidate) > 4:
            add(candidate[4:])
        if candidate.startswith("set_") and len(candidate) > 4:
            add(candidate)
    return candidates


def c_name_candidates(name: str) -> set[str]:
    return set(c_name_candidate_list(name))


def c_id(fn: FunctionSpan) -> str:
    return f"{fn.file}:{fn.name}"


def rust_id(fn: FunctionSpan) -> str:
    return f"{fn.file}:{fn.start_line}:{fn.name}"


def map_rust_functions_by_c_ref(rust_functions: list[FunctionSpan]) -> dict[str, list[FunctionSpan]]:
    by_ref: dict[str, list[FunctionSpan]] = {}
    for rf in rust_functions:
        for ref_name in rf.c_refs:
            by_ref.setdefault(ref_name, []).append(rf)
    return by_ref


def map_rust_functions_by_name(rust_functions: list[FunctionSpan]) -> dict[str, list[FunctionSpan]]:
    by_name: dict[str, list[FunctionSpan]] = {}
    for rf in rust_functions:
        by_name.setdefault(rf.name, []).append(rf)
    return by_name


def _split_c_parameters(params: str) -> list[str]:
    out: list[str] = []
    start = 0
    depth = 0
    for i, ch in enumerate(params):
        if ch in "([{":
            depth += 1
        elif ch in ")]}" and depth > 0:
            depth -= 1
        elif ch == "," and depth == 0:
            out.append(params[start:i].strip())
            start = i + 1
    tail = params[start:].strip()
    if tail and tail != "void":
        out.append(tail)
    return out


def _signature_return_and_params(signature: str | None) -> tuple[str, list[str]]:
    if not signature:
        return "", []
    paren = signature.find("(")
    if paren < 0:
        return signature, []
    close = find_matching_delim(signature, paren, "(", ")")
    if close is None:
        close = signature.rfind(")")
    if close < paren:
        return signature[:paren], []
    return signature[:paren], _split_c_parameters(signature[paren + 1:close])


def _impl_from_c_type(fragment: str) -> str | None:
    normalized = re.sub(r"\b(const|struct|enum|volatile|restrict)\b", " ", fragment)
    normalized = re.sub(r"\s+", " ", normalized)
    for c_type, impl_type in C_TYPE_IMPLS:
        if c_type in normalized:
            return impl_type
    return None


def preferred_impl_for_c(c_fn: FunctionSpan, candidate: str | None = None) -> str | None:
    if candidate:
        manual = MANUAL_C_ALIAS_IMPLS.get(c_fn.name, {}).get(candidate)
        if manual:
            return manual
    ret, params = _signature_return_and_params(c_fn.signature)
    ret_impl = _impl_from_c_type(ret)
    name_tokens = set(semantic_name_tokens(c_fn.name))
    if ret_impl and (candidate == "new" or "create" in name_tokens):
        return ret_impl
    if params:
        receiver = _impl_from_c_type(params[0])
        if receiver:
            return receiver
    return ret_impl


def _prefer_rust_match(matches: list[FunctionSpan], preferred_impl: str | None = None) -> FunctionSpan | None:
    if not matches:
        return None
    if preferred_impl:
        for m in matches:
            if m.impl_type == preferred_impl and not m.file.startswith("rs/fq/src/tests/"):
                return m
        for m in matches:
            if m.impl_type == preferred_impl:
                return m
    for m in matches:
        if not m.file.startswith("rs/fq/src/tests/"):
            return m
    return matches[0]


TOKEN_REPLACEMENTS = {
    "cnxid": "connection_id",
    "cnx": "connection",
    "vint": "varint",
    "vlen": "varint_length",
    "cwin": "congestion_window",
    "btlbw": "bottleneck_bandwidth",
    "rtt": "rtt",
    "ecn": "ecn",
}

PREFIX_REPLACEMENTS = {
    "byteread_": "read_",
    "bytewrite_": "write_",
    "byteshow_": "peek_",
    "byteskip_": "skip_",
    "bytestream_": "",
}

IGNORED_NAME_TOKENS = {
    "picoquic",
    "picosocks",
    "picohash",
    "picosplay",
    "picoquictest",
}

GENERIC_RUST_NAMES = {
    "new",
    "default",
    "create",
    "delete",
    "clear",
    "reset",
    "init",
    "open",
    "close",
    "read",
    "write",
    "next",
    "first",
    "last",
    "remove",
}


def semantic_name_tokens(name: str) -> list[str]:
    s = camel_to_snake(name)
    for old, new in PREFIX_REPLACEMENTS.items():
        if s.startswith(old):
            s = new + s[len(old):]
            break
    raw_tokens = [t for t in re.split(r"[^A-Za-z0-9]+", s) if t]
    tokens: list[str] = []
    for raw in raw_tokens:
        replacement = TOKEN_REPLACEMENTS.get(raw, raw)
        for token in replacement.split("_"):
            if token and token not in IGNORED_NAME_TOKENS:
                tokens.append(token)
    return tokens


def semantic_join(tokens: list[str]) -> str:
    return "_".join(tokens)


def primary_action(tokens: set[str]) -> str | None:
    for action in ("skip", "peek", "read", "write", "encode", "decode", "parse", "format", "set", "get"):
        if action in tokens:
            return action
    return None


def semantic_c_aliases(c_fn: FunctionSpan) -> set[str]:
    aliases = set(c_name_candidate_list(c_fn.name))
    tokens = semantic_name_tokens(c_fn.name)
    if tokens:
        aliases.add(semantic_join(tokens))
    if len(tokens) >= 2 and tokens[0] in {"read", "write", "peek", "skip"}:
        aliases.add(semantic_join(tokens[1:]))
    if tokens[-2:] == ["skip", "varint"]:
        aliases.add("skip_varint")
    if tokens[-2:] == ["read", "varint"]:
        aliases.add("read_varint")
    if tokens[-2:] == ["write", "varint"]:
        aliases.add("write_varint")
    if tokens[-3:] == ["stream", "varint", "length"]:
        aliases.add("varint_encoded_len")
    if is_congestion_control_source(c_fn) and tokens and tokens[-1] in {"init", "notify", "delete", "observe"}:
        aliases.add(f"alg_{tokens[-1]}")
    ret, _params = _signature_return_and_params(c_fn.signature)
    if "create" in tokens and _impl_from_c_type(ret):
        aliases.add("new")
    return {a for a in aliases if a}


def candidate_rust_files_for_c(c_fn: FunctionSpan) -> list[str]:
    stem = Path(c_fn.file).stem
    known_multi = {
        "frames": ["internal.rs", "utils.rs"],
        "frame_names": ["frames.rs"],
        "intformat": ["utils.rs"],
        "util": ["utils.rs"],
        "sacks": ["internal.rs"],
        "timing": ["internal.rs"],
        "pacing": ["internal.rs"],
        "quicctx": ["lib.rs", "internal.rs"],
        "sender": ["internal.rs", "lib.rs"],
        "packet": ["internal.rs", "lib.rs"],
        "paths": ["internal.rs", "lib.rs"],
        "transport": ["internal.rs"],
        "loss_recovery": ["internal.rs"],
        "tls_api": ["tls_api.rs", "lib.rs"],
        "ticket_store": ["tls_api.rs", "internal.rs", "lib.rs"],
        "token_store": ["tls_api.rs", "internal.rs", "lib.rs"],
        "logwriter": ["binlog.rs", "internal.rs", "utils.rs", "logger.rs"],
        "dualq_aqm": ["tests/dualq.rs", "tests/dualq_aqm.rs"],
    }
    files: list[str] = []

    def add(path: str) -> None:
        full = RS_SRC / path
        rel_path = rel(full)
        if full.is_file() and rel_path not in files:
            files.append(rel_path)

    for mapped in known_multi.get(stem, []):
        add(mapped)
    suggested, _ = suggested_destination(c_fn)
    if (REPO_ROOT / suggested).is_file() and suggested not in files:
        files.append(suggested)
    direct = RS_SRC / f"{stem}.rs"
    if direct.is_file():
        direct_rel = rel(direct)
        if direct_rel not in files:
            files.append(direct_rel)
    return files


def expected_rust_file(c_fn: FunctionSpan) -> str:
    files = candidate_rust_files_for_c(c_fn)
    return files[0] if files else suggested_destination(c_fn)[0]


def is_congestion_control_source(c_fn: FunctionSpan) -> bool:
    return Path(c_fn.file).stem in {
        "bbr",
        "bbr1",
        "c4",
        "prague",
        "cubic",
        "fastcc",
        "newreno",
        "register_all_cc_algorithms",
        "dualq_aqm",
        "pacing",
        "loss_recovery",
    }


def c_module_tokens(c_fn: FunctionSpan) -> set[str]:
    return set(semantic_name_tokens(Path(c_fn.file).stem))


def rust_impl_tokens(r_fn: FunctionSpan) -> set[str]:
    if not r_fn.impl_type:
        return set()
    return set(semantic_name_tokens(r_fn.impl_type))


def semantic_score(c_fn: FunctionSpan, r_fn: FunctionSpan) -> tuple[int, str]:
    aliases = semantic_c_aliases(c_fn)
    c_tokens = set(semantic_name_tokens(c_fn.name))
    r_tokens = set(semantic_name_tokens(r_fn.name))
    if not r_tokens:
        return 0, "no rust name tokens"

    score = 0
    reasons: list[str] = []
    expected_file = expected_rust_file(c_fn)
    if r_fn.file == expected_file:
        score += 25
        reasons.append("same proposed module")
    elif c_module_tokens(c_fn) & set(semantic_name_tokens(Path(r_fn.file).stem)):
        score += 12
        reasons.append("related module name")

    preferred_impl = preferred_impl_for_c(c_fn, r_fn.name)
    if preferred_impl and r_fn.impl_type == preferred_impl:
        score += 22
        reasons.append(f"receiver impl {preferred_impl}")
    elif r_fn.impl_type and c_module_tokens(c_fn) & rust_impl_tokens(r_fn):
        score += 22
        reasons.append(f"impl/module tokens {r_fn.impl_type}")

    if r_fn.name in aliases:
        score += 55
        reasons.append(f"semantic alias {r_fn.name}")
    else:
        common = c_tokens & r_tokens
        if common:
            union = c_tokens | r_tokens
            score += int(30 * len(common) / len(union))
            reasons.append("token overlap " + ",".join(sorted(common)))
        if len(r_tokens) >= 2 and r_tokens <= c_tokens:
            score += 28
            reasons.append("rust tokens subset of C name")

    c_action = primary_action(c_tokens)
    r_action = primary_action(r_tokens)
    if c_action and r_action and c_action != r_action:
        score -= 30
        reasons.append(f"action mismatch {c_action}!={r_action}")

    if r_fn.name.startswith("alg_"):
        if not is_congestion_control_source(c_fn) or r_fn.name not in aliases or not r_fn.impl_type:
            return 0, "trait method without concrete impl/module evidence"
        score += 10
        reasons.append("trait method alias")

    if r_fn.name in GENERIC_RUST_NAMES and not (
        preferred_impl and r_fn.impl_type == preferred_impl
    ) and r_fn.file != expected_file:
        score -= 35
        reasons.append("generic rust name penalty")

    return score, "; ".join(reasons)


def semantic_candidate_pool(
    c_fn: FunctionSpan,
    rust_by_file: dict[str, list[FunctionSpan]],
    rust_by_name: dict[str, list[FunctionSpan]],
    rust_by_impl_token: dict[str, list[FunctionSpan]],
) -> list[FunctionSpan]:
    candidates: dict[str, FunctionSpan] = {}

    def add(fn: FunctionSpan) -> None:
        candidates.setdefault(rust_id(fn), fn)

    for file in candidate_rust_files_for_c(c_fn):
        for fn in rust_by_file.get(file, []):
            add(fn)
    for alias in semantic_c_aliases(c_fn):
        for fn in rust_by_name.get(alias, []):
            add(fn)
    stem_tokens = c_module_tokens(c_fn)
    if stem_tokens:
        for token in stem_tokens:
            for fn in rust_by_impl_token.get(token, []):
                add(fn)
    return list(candidates.values())


def semantic_unmatched_matches(
    c_functions: list[FunctionSpan],
    rust_functions: list[FunctionSpan],
    claimed_rust_ids: set[str],
) -> dict[str, SemanticMatch]:
    rust_unclaimed = [fn for fn in rust_functions if rust_id(fn) not in claimed_rust_ids]
    rust_by_file: dict[str, list[FunctionSpan]] = {}
    rust_by_name: dict[str, list[FunctionSpan]] = {}
    rust_by_impl_token: dict[str, list[FunctionSpan]] = {}
    for rf in rust_unclaimed:
        rust_by_file.setdefault(rf.file, []).append(rf)
        rust_by_name.setdefault(rf.name, []).append(rf)
        for token in rust_impl_tokens(rf):
            rust_by_impl_token.setdefault(token, []).append(rf)

    candidates: list[SemanticMatch] = []
    for c_fn in c_functions:
        for r_fn in semantic_candidate_pool(c_fn, rust_by_file, rust_by_name, rust_by_impl_token):
            score, reason = semantic_score(c_fn, r_fn)
            if score >= 78:
                candidates.append(SemanticMatch(c_id(c_fn), r_fn, score, reason))

    by_c: dict[str, list[SemanticMatch]] = {}
    by_r: dict[str, list[SemanticMatch]] = {}
    for candidate in candidates:
        by_c.setdefault(candidate.c_key, []).append(candidate)
        by_r.setdefault(rust_id(candidate.rust), []).append(candidate)

    accepted: dict[str, SemanticMatch] = {}
    for candidate in sorted(candidates, key=lambda m: m.score, reverse=True):
        if candidate.c_key in accepted:
            continue
        if any(rust_id(m.rust) == rust_id(candidate.rust) for m in accepted.values()):
            continue
        best_for_c = max(by_c[candidate.c_key], key=lambda m: m.score)
        best_for_r = max(by_r[rust_id(candidate.rust)], key=lambda m: m.score)
        if best_for_c != candidate or best_for_r != candidate:
            continue
        accepted[candidate.c_key] = candidate
    return accepted


def suggested_destination(c_fn: FunctionSpan) -> tuple[str, str]:
    stem = Path(c_fn.file).stem
    known = {
        "frames": "internal.rs",
        "frame_names": "frames.rs",
        "intformat": "utils.rs",
        "util": "utils.rs",
        "sacks": "internal.rs",
        "timing": "internal.rs",
        "pacing": "internal.rs",
        "quicctx": "lib.rs",
        "transport": "internal.rs",
        "packet": "internal.rs",
        "paths": "internal.rs",
        "sender": "internal.rs",
        "loss_recovery": "internal.rs",
        "tls_api": "tls_api.rs",
        "picoquic_ptls_openssl": "sys/openssl.rs",
        "picoquic_ptls_minicrypto": "tls_api.rs",
        "picoquic_ptls_fusion": "tls_api.rs",
        "picoquic_mbedtls": "sys/mod.rs",
        "picosocks": "socks.rs",
        "sockloop": "packet_loop.rs",
        "sim_link": "tests/harness.rs",
        "picoquic_lb": "lb.rs",
        "picohash": "hash.rs",
        "picosplay": "splay.rs",
        "ticket_store": "tls_api.rs",
        "token_store": "tls_api.rs",
        "logger": "logger.rs",
        "logwriter": "binlog.rs",
        "unified_log": "textlog.rs",
        "performance_log": "performance_log.rs",
    }
    if stem in known:
        return rel(RS_SRC / known[stem]), "known module mapping"
    direct = RS_SRC / f"{stem}.rs"
    if direct.is_file():
        return rel(direct), "same-name module"
    return rel(RS_SRC / f"{stem}.rs"), "new same-name module candidate"


EXPECTED_OMISSION_RE = re.compile(
    r"(^|_)(free|delete|dispose|destroy|release|malloc|calloc|realloc)(_|$)",
    re.IGNORECASE,
)


def folded_helper_reason(c_fn: FunctionSpan) -> str | None:
    name = c_fn.name
    stem = Path(c_fn.file).stem

    collection_suffixes = (
        "_hash",
        "_compare",
        "_to_item",
        "_node_value",
        "_node_compare",
        "_node_create",
        "_create_node",
        "_value",
    )
    if stem in {"quicctx", "sacks", "picosplay"} and name.endswith(collection_suffixes):
        return "C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function"

    if stem == "frames":
        frame_prefixes = (
            "picoquic_skip_",
            "picoquic_parse_",
            "picoquic_decode_",
            "picoquic_process_ack_of_",
            "picoquic_check_",
        )
        frame_suffixes = (
            "_needs_repeat",
            "_need_repeat",
            "_frame_needs_repeat",
            "_frame_need_repeat",
        )
        if name.startswith(frame_prefixes) or name.endswith(frame_suffixes):
            return "C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item"
        if name in {
            "picoquic_is_stream_acked",
            "picoquic_find_or_create_stream",
            "picoquic_flow_control_check_stream_offset",
            "picoquic_stream_data_chunk_callback",
            "picoquic_stream_data_callback",
            "add_chunk_node",
            "picoquic_stream_network_input",
            "picoquic_is_last_stream_frame",
            "picoquic_crypto_stream_from_ptype",
            "picoquic_check_spurious_retransmission",
            "picoquic_dequeue_old_retransmitted_packets",
            "process_decoded_packet_data",
            "picoquic_find_acked_packet",
            "picoquic_process_ack_range",
        }:
            return "C frame/stream dispatch helper likely folded into Rust connection or frame processing"

    if stem == "logger" and (
        name.startswith("textlog_")
        or name.startswith("picoquic_textlog_")
        or name.startswith("picoquic_txtlog_")
    ):
        return "C textlog detail helper folded into Rust logger/textlog traits and methods"

    if stem == "logwriter":
        if name.startswith("picoquic_log_") and name.endswith("_frame"):
            return "C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper"

    if stem == "tls_api":
        if (
            name.startswith("picoquic_register_")
            or name.endswith("_cb")
            or name.endswith("_call_back")
            or name.endswith("_callback")
            or name in {
                "picoquic_tls_api_init_providers",
                "picoquic_tls_api_zero",
                "picoquic_tls_api_log_versions",
                "picoquic_tls_api_unload",
                "picoquic_set_cipher_suite_list",
                "picoquic_set_cipher_suite_in_ctx",
                "picoquic_set_key_exchange_in_ctx",
                "picoquic_set_random_provider_in_ctx",
                "picoquic_get_certificate_verifier",
                "picoquic_explain_crypto_error",
                "picoquic_clear_crypto_errors",
                "picoquic_log_crypto_errors",
                "picoquic_tls_get_quic_extension_id",
                "picoquic_tls_collect_extensions_cb",
                "picoquic_tls_set_extensions",
                "picoquic_tls_collected_extensions_cb",
            }
        ):
            return "C TLS/provider callback or registry hook folded into Rust TLS provider traits and state"

    if stem == "util" and (
        name.startswith("picoquic_create_thread")
        or name.startswith("picoquic_wait_thread")
        or name.startswith("picoquic_create_mutex")
        or name.startswith("picoquic_lock_mutex")
        or name.startswith("picoquic_unlock_mutex")
        or name.startswith("picoquic_create_event")
        or name.startswith("picoquic_signal_event")
        or name.startswith("picoquic_wait_for_event")
        or name in {"picoquic_file_open", "picoquic_file_open_ex", "picoquic_file_close"}
    ):
        return "C portability wrapper folded into Rust standard library types or Drop"

    return None


def classify_missing(c_fn: FunctionSpan) -> tuple[str, str]:
    if EXPECTED_OMISSION_RE.search(c_fn.name):
        return (
            "expected_omission",
            "name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it",
        )
    folded = folded_helper_reason(c_fn)
    if folded:
        return ("expected_omission", folded)
    return (
        "required_missing",
        "no Rust counterpart found by C doc reference or conservative name matching",
    )


def build_function_map(previous: dict[str, Any] | None = None) -> dict[str, Any]:
    previous = previous or {}
    prev_entries = {
        e.get("c_id"): e
        for e in previous.get("entries", [])
        if isinstance(e, dict) and e.get("c_id")
    }
    c_functions = c_functions_from_inventory()
    c_lookup = {c_id(fn): fn for fn in c_functions}
    rust_functions = collect_rust_functions()
    by_ref = map_rust_functions_by_c_ref(rust_functions)
    by_name = map_rust_functions_by_name(rust_functions)
    entries: list[dict[str, Any]] = []

    for c_fn in c_functions:
        evidence = ""
        rust_match: FunctionSpan | None = None
        for candidate in c_name_candidate_list(c_fn.name):
            matches = by_name.get(candidate)
            rust_match = _prefer_rust_match(matches or [], preferred_impl_for_c(c_fn, candidate))
            if rust_match is not None:
                evidence = f"name_match:{candidate}"
                break
        if rust_match is None:
            ref_names = [c_fn.name]
            for ref_name in c_name_candidate_list(c_fn.name):
                if ref_name not in ref_names:
                    ref_names.append(ref_name)
            for ref_name in ref_names:
                refs = by_ref.get(ref_name, [])
                rust_match = _prefer_rust_match(refs, preferred_impl_for_c(c_fn, ref_name))
                if rust_match is not None:
                    evidence = f"rust_doc_c_ref:{ref_name}"
                    break

        prev = prev_entries.get(c_id(c_fn), {})
        entry: dict[str, Any] = {
            "c_id": c_id(c_fn),
            "c": c_fn.without_body(),
            "rust": rust_match.without_body() if rust_match else None,
            "mapping_evidence": evidence or None,
        }
        if rust_match is not None:
            incomplete = bool(INCOMPLETE_RE.search(rust_match.body))
            entry["implementation_status"] = "incomplete" if incomplete else "implemented"
            entry["required_action"] = "required_missing" if incomplete else "implemented"
            entry["reason"] = (
                "mapped Rust function still contains incomplete markers"
                if incomplete
                else "completed Rust counterpart found"
            )
            if incomplete:
                entry["phase4a_plan"] = {
                    "rust_destination": rust_match.file,
                    "destination_reason": "existing mapped Rust item is incomplete",
                    "item_shape": "complete existing Rust function or method",
                    "verification": "targeted cargo test if known; otherwise Phase 4 cargo gates",
                }
        else:
            prev_action = prev.get("required_action")
            if prev_action in {"expected_omission", "blocked"} and prev.get("manual"):
                action = prev_action
                reason = prev.get("reason", "preserved manual classification")
            else:
                action, reason = classify_missing(c_fn)
            destination, destination_reason = suggested_destination(c_fn)
            entry.update({
                "implementation_status": "missing",
                "required_action": action,
                "reason": reason,
                "phase4a_plan": {
                    "rust_destination": destination,
                    "destination_reason": destination_reason,
                    "item_shape": "private helper" if c_fn.is_static else "public API function or method",
                    "verification": "targeted cargo test if known; otherwise Phase 4 cargo gates",
                },
            })
        if prev.get("notes"):
            entry["notes"] = prev["notes"]
        if prev.get("manual"):
            entry["manual"] = prev["manual"]
        entries.append(entry)

    claimed_rust_ids = {
        f"{e['rust']['file']}:{e['rust']['start_line']}:{e['rust']['name']}"
        for e in entries
        if e.get("rust")
    }
    unmatched_c = [c_lookup[e["c_id"]] for e in entries if not e.get("rust") and e["c_id"] in c_lookup]
    semantic_matches = semantic_unmatched_matches(unmatched_c, rust_functions, claimed_rust_ids)
    for entry in entries:
        match = semantic_matches.get(entry["c_id"])
        if match is None:
            continue
        entry["rust"] = match.rust.without_body()
        entry["mapping_evidence"] = f"semantic_unmatched_match:{match.score}"
        entry["semantic_match_reason"] = match.reason
        incomplete = bool(INCOMPLETE_RE.search(match.rust.body))
        entry["implementation_status"] = "incomplete" if incomplete else "implemented"
        entry["required_action"] = "required_missing" if incomplete else "implemented"
        entry["reason"] = (
            "mapped Rust function still contains incomplete markers"
            if incomplete
            else "completed Rust counterpart found"
        )
        if incomplete:
            entry["phase4a_plan"] = {
                "rust_destination": match.rust.file,
                "destination_reason": "existing mapped Rust item is incomplete",
                "item_shape": "complete existing Rust function or method",
                "verification": "targeted cargo test if known; otherwise Phase 4 cargo gates",
            }
        else:
            entry.pop("phase4a_plan", None)

    summary: dict[str, int] = {}
    for e in entries:
        summary[e["required_action"]] = summary.get(e["required_action"], 0) + 1
    return {
        "schema_version": 1,
        "generated_at": now(),
        "repo_root": str(REPO_ROOT),
        "entries": entries,
        "summary": summary,
    }


def load_function_map() -> dict[str, Any]:
    return load_json(FUNCTION_MAP, {"schema_version": 1, "entries": [], "summary": {}})


def html_page(title: str, body: str) -> str:
    return (
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\n"
        f"<title>{html.escape(title)}</title>\n"
        "<style>\n"
        "body{font-family:system-ui,-apple-system,sans-serif;margin:24px;line-height:1.35;}\n"
        "table{border-collapse:collapse;width:100%;margin:16px 0;}\n"
        "th,td{border:1px solid #ddd;padding:6px 8px;vertical-align:top;}\n"
        "th{background:#f5f5f5;text-align:left;}\n"
        "pre{white-space:pre-wrap;margin:0;font:12px ui-monospace,SFMono-Regular,Menlo,monospace;}\n"
        ".grid{display:grid;grid-template-columns:1fr 1fr;gap:12px;}\n"
        ".bad{background:#fff0f0}.suspect{background:#fffbe6}.ok{background:#f0fff4}\n"
        "</style></head><body>\n"
        f"{body}\n</body></html>\n"
    )


def esc(value: Any) -> str:
    return html.escape("" if value is None else str(value))


def source_body(span_info: dict[str, Any] | None) -> str:
    if not span_info:
        return ""
    path = REPO_ROOT / span_info["file"]
    if not path.is_file():
        return ""
    text = path.read_text(errors="replace")
    lines = text.splitlines()
    start = max(1, int(span_info.get("body_start_line") or span_info.get("start_line") or 1))
    end = max(start, int(span_info.get("body_end_line") or span_info.get("end_line") or start))
    return "\n".join(lines[start - 1:end])
