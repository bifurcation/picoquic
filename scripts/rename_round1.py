#!/usr/bin/env python3
"""Round-1 mechanical renames called out by REVIEW comments.

Operates on every .rs under rs/fq/src/ — bulk rename, word-bounded,
applied via regex.  Run from rs/fq/.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Word-boundary pairs, applied in order.  Both struct/type names and
# field names get bounded with `\b` so substrings (e.g. `Tp0rttKindFoo`)
# don't collide.
RENAMES = [
    # Type renames.
    ("LocalCnxidToken", "LocalConnectionIdToken"),
    ("LocalCnxidList", "LocalConnectionIdList"),
    ("LocalCnxid", "LocalConnectionId"),
    ("RemoteCnxidStash", "RemoteConnectionIdStash"),
    ("RemoteCnxid", "RemoteConnectionId"),
    ("StreamDataCb", "StreamDataCallback"),
    ("ConnectionIdCb", "ConnectionIdCallback"),
    ("Tp0rttKind", "TransportParameter0RttKind"),
    # Field/value renames (these are also valid Rust idents, so word-bounded
    # regex catches both definition and use).
    ("dest_cnx_id", "dest_connection_id"),
    ("srce_cnx_id", "src_connection_id"),
    ("initial_cid", "initial_connection_id"),
    ("l_cid", "local_connection_id"),
    ("cnxids", "connection_ids"),
    # PacketHeader abbreviations.  These are common parameter names too,
    # so the substitution is by full word.  Bodies are still `todo!()`
    # so risk is contained.
    ("pn_offset", "packet_number_offset"),
    ("pnmask", "packet_number_mask"),
    ("pn64", "packet_number_full"),
    ("pl_val", "payload_length_value"),
]


def apply_rename(text: str, old: str, new: str) -> tuple[str, int]:
    pat = re.compile(rf"\b{re.escape(old)}\b")
    return pat.subn(new, text)


def main() -> int:
    root = Path("src")
    if not root.is_dir():
        print("error: run from rs/fq/", file=sys.stderr)
        return 2

    total = 0
    for path in sorted(root.rglob("*.rs")):
        text = path.read_text()
        original = text
        for old, new in RENAMES:
            text, n = apply_rename(text, old, new)
            total += n
        if text != original:
            path.write_text(text)
            print(f"wrote: {path}")
    print(f"total replacements: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
