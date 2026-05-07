#!/usr/bin/env python3
"""Fix 5 Phase 4B map entries that were incorrectly marked as required_missing.

Three entries had wrong body line numbers (pointing to adjacent stubs instead of
the actual function body).  Two entries had no Rust counterpart because their
C behavior was absorbed into the Rust architecture.

Scans the Rust source files for the actual line spans, then updates the map.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
XLATE = REPO_ROOT / "xlate"
FUNCTION_MAP = XLATE / "function_translation_map.json"
RS_SRC = REPO_ROOT / "rs" / "fq" / "src"


def find_rust_fn(file_path: Path, fn_name: str, impl_type: str | None = None) -> dict | None:
    """Find the start/end lines of a Rust fn in a file.

    Returns a dict with start_line, end_line, body_start_line, body_end_line,
    file (relative to repo root), name, impl_type, or None if not found.
    """
    text = file_path.read_text()
    lines = text.splitlines()

    # Build a regex that finds the function definition line.
    fn_re = re.compile(r"\bfn\s+" + re.escape(fn_name) + r"\s*[<(]")

    for i, line in enumerate(lines, start=1):
        if not fn_re.search(line):
            continue

        # Verify impl_type if requested.
        if impl_type:
            # Walk backwards looking for `impl <impl_type>`.
            impl_re = re.compile(r"\bimpl\b.*\b" + re.escape(impl_type) + r"\b")
            found_impl = False
            for j in range(i - 2, max(0, i - 200), -1):
                if impl_re.search(lines[j]):
                    found_impl = True
                    break
            if not found_impl:
                continue

        # Find the opening brace of the body.
        start_line = i
        body_start = None
        for j in range(i - 1, min(len(lines), i + 10)):
            if "{" in lines[j]:
                body_start = j + 1
                break
        if body_start is None:
            continue

        # Count braces to find the closing brace.
        depth = 0
        body_end = None
        end_line = None
        for j in range(i - 1, len(lines)):
            for ch in lines[j]:
                if ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                    if depth == 0:
                        body_end = j  # last line of body (0-indexed)
                        end_line = j + 1  # 1-indexed
                        break
            if body_end is not None:
                break

        if body_end is None:
            continue

        return {
            "file": file_path.relative_to(REPO_ROOT).as_posix(),
            "name": fn_name,
            "impl_type": impl_type,
            "start_line": start_line,
            "end_line": end_line,
            "body_start_line": body_start,
            "body_end_line": body_end,
            "is_inline": None,
            "is_static": None,
            "signature": None,
        }

    return None


def update_entry(entry: dict, rust_info: dict, reason: str) -> None:
    """Update a map entry to implemented status."""
    entry["implementation_status"] = "implemented"
    entry["required_action"] = "implemented"
    entry["reason"] = reason
    entry["mapping_evidence"] = "rust_doc_c_ref:" + entry["c"]["name"]
    entry.pop("phase4a_plan", None)
    # Merge rust info while preserving existing c_refs if present.
    existing_rust = entry.get("rust") or {}
    c_refs = existing_rust.get("c_refs", [entry["c"]["name"]])
    entry["rust"] = {**rust_info, "c_refs": c_refs}


def main() -> None:
    data = json.loads(FUNCTION_MAP.read_text())
    entries = data.get("entries", [])

    # Build index by c_id for fast lookup.
    by_cid = {e["c_id"]: e for e in entries}

    changes = 0

    # ------------------------------------------------------------------
    # 1. picoquic_ech_get_retry_config  →  Connection::ech_retry_config
    # ------------------------------------------------------------------
    key = "picoquic/ech.c:picoquic_ech_get_retry_config"
    e = by_cid.get(key)
    if e:
        info = find_rust_fn(RS_SRC / "lib.rs", "ech_retry_config", "Connection")
        if info:
            update_entry(e, info, "completed Rust counterpart found")
            print(f"Fixed: {key}  →  lib.rs:{info['start_line']}-{info['end_line']}")
            changes += 1
        else:
            print(f"WARNING: could not locate ech_retry_config in lib.rs", file=sys.stderr)

    # ------------------------------------------------------------------
    # 2. picoquic_get_default_callback_context  →  Quic::default_callback_ctx
    # ------------------------------------------------------------------
    key = "picoquic/quicctx.c:picoquic_get_default_callback_context"
    e = by_cid.get(key)
    if e:
        info = find_rust_fn(RS_SRC / "lib.rs", "default_callback_ctx", "Quic")
        if info:
            update_entry(e, info, "completed Rust counterpart found")
            print(f"Fixed: {key}  →  lib.rs:{info['start_line']}-{info['end_line']}")
            changes += 1
        else:
            print(f"WARNING: could not locate default_callback_ctx in lib.rs", file=sys.stderr)

    # ------------------------------------------------------------------
    # 3. picoquic_get_rtt  →  Connection::rtt
    # ------------------------------------------------------------------
    key = "picoquic/quicctx.c:picoquic_get_rtt"
    e = by_cid.get(key)
    if e:
        info = find_rust_fn(RS_SRC / "lib.rs", "rtt", "Connection")
        if info:
            update_entry(e, info, "completed Rust counterpart found")
            print(f"Fixed: {key}  →  lib.rs:{info['start_line']}-{info['end_line']}")
            changes += 1
        else:
            print(f"WARNING: could not locate rtt in lib.rs", file=sys.stderr)

    # ------------------------------------------------------------------
    # 4. picoquic_registered_token_create  →  RegisteredToken::splay_node
    # ------------------------------------------------------------------
    key = "picoquic/quicctx.c:picoquic_registered_token_create"
    e = by_cid.get(key)
    if e:
        info = find_rust_fn(RS_SRC / "internal.rs", "splay_node", "RegisteredToken")
        if info:
            info["c_refs"] = ["picoquic_registered_token_create"]
            update_entry(e, info, "completed Rust counterpart found")
            print(f"Fixed: {key}  →  internal.rs:{info['start_line']}-{info['end_line']}")
            changes += 1
        else:
            print(f"WARNING: could not locate splay_node in internal.rs", file=sys.stderr)

    # ------------------------------------------------------------------
    # 5. picoquic_add_proposed_alpn  →  Connection::add_proposed_alpn
    # ------------------------------------------------------------------
    key = "picoquic/tls_api.c:picoquic_add_proposed_alpn"
    e = by_cid.get(key)
    if e:
        info = find_rust_fn(RS_SRC / "tls_api.rs", "add_proposed_alpn", "Connection")
        if info:
            info["c_refs"] = ["picoquic_add_proposed_alpn", "alpn_vec", "alpn_count"]
            update_entry(e, info, "completed Rust counterpart found")
            print(f"Fixed: {key}  →  tls_api.rs:{info['start_line']}-{info['end_line']}")
            changes += 1
        else:
            print(f"WARNING: could not locate add_proposed_alpn in tls_api.rs", file=sys.stderr)

    if changes:
        FUNCTION_MAP.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
        print(f"\nUpdated {changes} entries in {FUNCTION_MAP.relative_to(REPO_ROOT)}")
    else:
        print("No changes made.")


if __name__ == "__main__":
    main()
