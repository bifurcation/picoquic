#!/usr/bin/env python3
"""Rename lowercase `cnx` -> `connection` (word-boundary).

Touches:
  - bare `cnx` (parameter / variable / receiver names)
  - `cnx_FOO` prefix identifiers (`cnx_id`, `cnx_state`, `cnx_by_id`,
    `cnx_wake_tree`, etc.)
  - `FOO_cnx` suffix identifiers (`set_default_cnx`, etc.)

Skips (kept verbatim because they are C-source references in doc
comments):
  - `picoquic_cnx_t`
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "rs" / "fq" / "src"

# Skip C type references in documentation.
SKIP_TOKEN = "picoquic_cnx_t"
PLACEHOLDER = "\x00CNX_T_PLACEHOLDER\x00"

# Match `cnx` as a whole identifier OR as a prefix/suffix in
# snake_case identifiers.  `\b` works because `_` is a word
# character in Python regex — but we need the body of the
# identifier to count as a word boundary.  Use a manual lookahead
# / lookbehind to identify identifier boundaries.

# Identifier characters: [A-Za-z0-9_].  We want to match `cnx`
# only when surrounded by non-identifier characters OR by `_`
# (which is part of an identifier but is the canonical separator).
#
# Approach:
#   - bare `cnx`: `(?<![A-Za-z0-9_])cnx(?![A-Za-z0-9_])`
#   - `cnx_FOO` (prefix):
#     `(?<![A-Za-z0-9_])cnx(?=_[A-Za-z])`
#   - `FOO_cnx` (suffix):
#     `(?<=_)cnx(?![A-Za-z0-9_])`

PATTERNS = [
    # cnx_FOO -> connection_FOO  (prefix in snake_case)
    (re.compile(r"(?<![A-Za-z0-9_])cnx(?=_[A-Za-z])"), "connection"),
    # FOO_cnx -> FOO_connection  (suffix in snake_case)
    (re.compile(r"(?<=_)cnx(?![A-Za-z0-9_])"), "connection"),
    # bare cnx -> connection
    (re.compile(r"(?<![A-Za-z0-9_])cnx(?![A-Za-z0-9_])"), "connection"),
]


def rewrite(text: str) -> tuple[str, int]:
    text = text.replace(SKIP_TOKEN, PLACEHOLDER)
    total = 0
    for pat, repl in PATTERNS:
        text, n = pat.subn(repl, text)
        total += n
    text = text.replace(PLACEHOLDER, SKIP_TOKEN)
    return text, total


def main() -> int:
    grand = 0
    for path in sorted(SRC.rglob("*.rs")):
        original = path.read_text()
        new, n = rewrite(original)
        if new != original:
            path.write_text(new)
            print(f"{path.relative_to(ROOT)}: {n} replacement(s)")
            grand += n
    print(f"\nTotal: {grand} replacement(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())


# `cnxid` (no underscore separator) -> `connection_id`.  Same
# logic but the substring is `cnxid`.  Ran as a separate pass
# after the main `cnx` rename above.
def rewrite_cnxid(text: str) -> tuple[str, int]:
    text = text.replace(SKIP_TOKEN, PLACEHOLDER)
    pat = re.compile(r"(?<![A-Za-z0-9])cnxid(?![A-Za-z0-9])")
    text, n = pat.subn("connection_id", text)
    text = text.replace(PLACEHOLDER, SKIP_TOKEN)
    return text, n


if __name__ == "__main__":
    grand = 0
    for path in sorted(SRC.rglob("*.rs")):
        original = path.read_text()
        new, n = rewrite_cnxid(original)
        if new != original:
            path.write_text(new)
            print(f"cnxid pass {path.relative_to(ROOT)}: {n} replacement(s)")
            grand += n
    print(f"\nTotal cnxid replacements: {grand}.")
