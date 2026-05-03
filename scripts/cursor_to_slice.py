#!/usr/bin/env python3
"""Sweep raw byte-cursor pairs in internal.rs to slices.

Three function signatures recur across the frame encode/decode
helpers in `rs/fq/src/internal.rs`:

  pub fn foo(..., _bytes: *const u8, _bytes_max: *const u8, ...) -> *const u8
  pub fn bar(..., _bytes: *mut u8,   _bytes_max: *mut u8,   ...) -> *mut u8
  pub fn baz(..., _bytes_next: *mut u8, _bytes_max: *mut u8, ...) -> *mut u8

In every case the C contract is "walk a byte buffer between two
pointers, return the new cursor, or NULL/bytes_next on error."  The
Rust idiom is `&[u8]` / `&mut [u8]` plus `Option<&[u8]>` /
`Option<&mut [u8]>` for the return.

This script rewrites each of the three patterns mechanically.  It
also lifts a `<'a>` parameter onto the function so the returned
slice can borrow from the input slice via explicit lifetime.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET = ROOT / "rs" / "fq" / "src" / "internal.rs"

# Match a `pub fn NAME(` or `pub fn NAME<'a>(` block, all the way to
# the matching `)` followed by an optional `-> RET {`.  Use a balanced
# brace scan rather than a regex (regex isn't right for matching
# parens).  We re-scan parameter text inside the body of the match.

FN_HEADER = re.compile(
    r"""
    (?P<indent>^[ \t]*)                       # leading whitespace
    pub\ fn\ (?P<name>\w+)
    (?P<generics>(?:<[^<>]*>)?)               # optional <'a> etc.
    \(
    """,
    re.VERBOSE | re.MULTILINE,
)


def find_paren_close(text: str, open_idx: int) -> int:
    """Return the index of the `)` matching the `(` at open_idx."""
    depth = 0
    i = open_idx
    while i < len(text):
        c = text[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError("unbalanced parens")


def rewrite_signature(text: str, fn_match: re.Match) -> tuple[str, bool]:
    """Return (new_text_for_this_fn_header, changed?)."""
    open_paren_idx = fn_match.end() - 1  # match ends at the `(`
    close_paren_idx = find_paren_close(text, open_paren_idx)
    params_text = text[open_paren_idx + 1 : close_paren_idx]

    # Skip ahead past `)` to the optional `-> RET`.
    after_paren = close_paren_idx + 1
    # find the next `{` to know where the signature ends
    brace_idx = text.find("{", after_paren)
    if brace_idx < 0:
        return text[fn_match.start() : close_paren_idx + 1], False
    return_clause = text[after_paren:brace_idx]

    new_params, did_params = rewrite_params(params_text)
    new_return = return_clause
    did_return = False
    new_return, did_return = rewrite_return(return_clause, did_params)

    if not (did_params or did_return):
        return text[fn_match.start() : brace_idx], False

    # Re-emit the function header with possibly-added `<'a>`.
    name = fn_match.group("name")
    indent = fn_match.group("indent")
    generics = fn_match.group("generics")
    if did_params and not generics:
        generics = "<'a>"
    elif did_params and "'a" not in generics:
        # add 'a to existing generics, e.g. <T> -> <'a, T>
        generics = "<'a, " + generics[1:]

    rebuilt = f"{indent}pub fn {name}{generics}({new_params}){new_return}"
    return rebuilt, True


PARAM_PATTERNS = [
    # decode: _bytes: *const u8, _bytes_max: *const u8
    (
        re.compile(
            r"(?P<lead>[\s,])_bytes:\s*\*const\s+u8\s*,"
            r"\s*_bytes_max:\s*\*const\s+u8(?P<trail>\s*,?)"
        ),
        r"\g<lead>_bytes: &'a [u8]\g<trail>",
    ),
    # encode: _bytes: *mut u8, _bytes_max: *mut u8
    (
        re.compile(
            r"(?P<lead>[\s,])_bytes:\s*\*mut\s+u8\s*,"
            r"\s*_bytes_max:\s*\*mut\s+u8(?P<trail>\s*,?)"
        ),
        r"\g<lead>_bytes: &'a mut [u8]\g<trail>",
    ),
    # encode-cursor: _bytes_next: *mut u8, _bytes_max: *mut u8
    (
        re.compile(
            r"(?P<lead>[\s,])_bytes_next:\s*\*mut\s+u8\s*,"
            r"\s*_bytes_max:\s*\*mut\s+u8(?P<trail>\s*,?)"
        ),
        r"\g<lead>_bytes: &'a mut [u8]\g<trail>",
    ),
    # encode-cursor with const max: _bytes_next: *mut u8, _bytes_max: *const u8
    (
        re.compile(
            r"(?P<lead>[\s,])_bytes_next:\s*\*mut\s+u8\s*,"
            r"\s*_bytes_max:\s*\*const\s+u8(?P<trail>\s*,?)"
        ),
        r"\g<lead>_bytes: &'a mut [u8]\g<trail>",
    ),
]


def rewrite_params(params: str) -> tuple[str, bool]:
    out = params
    changed = False
    for pat, repl in PARAM_PATTERNS:
        new, n = pat.subn(repl, out)
        if n:
            changed = True
            out = new
    return out, changed


RETURN_PATTERNS = [
    (re.compile(r"->\s*\*const\s+u8\s*"), "-> Option<&'a [u8]> "),
    (re.compile(r"->\s*\*mut\s+u8\s*"), "-> Option<&'a mut [u8]> "),
]


def rewrite_return(clause: str, params_changed: bool) -> tuple[str, bool]:
    out = clause
    changed = False
    for pat, repl in RETURN_PATTERNS:
        new, n = pat.subn(repl, out)
        if n:
            changed = True
            out = new
    return out, changed


def main() -> int:
    text = TARGET.read_text()
    new_chunks: list[str] = []
    cursor = 0
    rewrites = 0

    for m in FN_HEADER.finditer(text):
        # everything up to this header
        new_chunks.append(text[cursor : m.start()])
        # find the matching close paren
        open_paren_idx = m.end() - 1
        close_paren_idx = find_paren_close(text, open_paren_idx)
        brace_idx = text.find("{", close_paren_idx)
        if brace_idx < 0:
            new_chunks.append(text[m.start() : close_paren_idx + 1])
            cursor = close_paren_idx + 1
            continue

        rebuilt, changed = rewrite_signature(text, m)
        if changed:
            rewrites += 1
            new_chunks.append(rebuilt)
        else:
            new_chunks.append(text[m.start() : brace_idx])
        cursor = brace_idx

    new_chunks.append(text[cursor:])
    new_text = "".join(new_chunks)

    if new_text != text:
        TARGET.write_text(new_text)
    print(f"rewrote {rewrites} function signature(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
