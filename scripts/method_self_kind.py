#!/usr/bin/env python3
"""Audit `&mut self` methods in `rs/fq/src/internal.rs` and rewrite
clearly-read-only ones to `&self`.

Heuristic: methods whose names start with read-only prefixes
(`is_`, `get_`, `find_`, `peek_`, `current_`, `first_`, `last_`,
`next_`, `previous_`, `iter_`, `count_`) plus a hand-curated set
of one-off names known to be reads.

Bodies are `todo!()` (Phase 1 contract); receiver mutability is
purely a signature concern, so this is safe.

Run from rs/fq/.  The script logs every change and prints the
total.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

TARGETS = [
    Path("src/internal.rs"),
    Path("src/lib.rs"),
    Path("src/config.rs"),
    Path("src/lb.rs"),
    Path("src/tls_api.rs"),
    Path("src/binlog.rs"),
    Path("src/logger.rs"),
    Path("src/qlog.rs"),
    Path("src/utils.rs"),
    Path("src/bytestream.rs"),
    Path("src/splay.rs"),
    Path("src/hash.rs"),
    Path("src/arena.rs"),
    Path("src/cc_common.rs"),
    Path("src/header_protection.rs"),
    Path("src/packet_loop.rs"),
    Path("src/socks.rs"),
    Path("src/socks_socket2.rs"),
    Path("src/textlog.rs"),
    Path("src/performance_log.rs"),
    Path("src/tp.rs"),
    Path("src/frames.rs"),
    Path("src/stream.rs"),
    Path("src/errors.rs"),
    Path("src/crypto.rs"),
    Path("src/tls.rs"),
]

IMMUTABLE_PREFIXES = (
    "is_",
    "get_",
    "find_",
    "peek_",
    "current_",
    "first",       # `first`, `first_item`, `first_range`, `first_stream`, `first_data_repeat_packet`
    "last",        # `last`, `last_item`, `last_stream`
    "next_",
    "previous_",
    "iter_",
    "count_",
    "has_",
    "size",
    "len",
    "all_",
    "any_",
    "should_",
    "compute_",
    "encode_time_stamp_length",  # explicit name
    "cc_increased_window",
)

IMMUTABLE_NAMES = {
    "first",
    "last",
    "size",
    "is_empty",
}

# Methods whose name suggests read-only but whose body genuinely
# mutates (verified against the C source, or the signature itself
# requires `&mut self` — e.g. returning `&mut V`).  These are
# kept as `&mut self` regardless of the prefix heuristic.
KEEP_MUT_OVERRIDES = {
    # Returns `&mut V` — needs &mut self even with a `get_*` name.
    "get_mut",
    # Has a `mark_used` flag that flips an entry's `was_used`.
    "get_ticket",
    "get_ticket_and_version",
    "get_token",
    # Lazily creates a path on fallthrough.
    "find_incoming_path",
    # Splays the matched node to the root (mutation as a side effect).
    "first_connection",
    # Stashes computed keys in `self.crypto_context_new`.
    "compute_new_rotated_keys",
    # Lazily allocates an AEAD context per QUIC version on first call.
    "find_retry_protection_context",
    # Returns `Option<&mut Connection>` from a list head — needs &mut self.
    "next_in_list",
    # Allocates a fresh stream id by bumping a counter on Connection.
    "next_local_stream_id",
}

NEEDLE = re.compile(
    r"(\bpub fn (?P<name>[A-Za-z_][A-Za-z0-9_]*)\(\s*)&mut self\b",
)


def is_immutable(name: str) -> bool:
    if name in KEEP_MUT_OVERRIDES:
        return False
    if name in IMMUTABLE_NAMES:
        return True
    return name.startswith(IMMUTABLE_PREFIXES)


def main() -> int:
    total_changed = 0
    for path in TARGETS:
        if not path.exists():
            continue
        text = path.read_text()
        changed_names: list[str] = []

        def repl(m: re.Match[str], names=changed_names) -> str:
            name = m.group("name")
            if is_immutable(name):
                names.append(name)
                return m.group(1) + "&self"
            return m.group(0)

        new_text = NEEDLE.sub(repl, text)
        if changed_names:
            path.write_text(new_text)
            print(f"{path}: flipped {len(changed_names)}:")
            for n in changed_names:
                print(f"  - {n}")
            total_changed += len(changed_names)
    print(f"total: {total_changed}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
