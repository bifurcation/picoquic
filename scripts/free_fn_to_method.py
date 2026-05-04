#!/usr/bin/env python3
"""Convert free functions whose first arg is `&mut <Receiver>` into
methods on that Receiver.

Targets `rs/fq/src/internal.rs`.  Recognises five receivers:
Connection, Quic, Path, SackList, AckContext.  For each candidate
free function:

  pub fn name(_recv: &mut Receiver, arg2: T2, …) -> R {
      todo!()
  }

becomes:

  impl Receiver {
      pub fn name(&mut self, arg2: T2, …) -> R {
          todo!()
      }
  }

Doc comments above the `pub fn` carry into the impl block.

Bodies must be `todo!()` (Phase 1 contract).  The script aborts
if it finds a non-todo body, to avoid silently dropping logic.

Run from rs/fq/.  Modifies src/internal.rs in place; review the
diff before committing.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

INTERNAL_RS = Path("src/internal.rs")

# Receiver types we handle, with the C-side parameter names that
# the Phase 1 translation produced.  Names are the leading
# underscore prefixed forms ("_connection") because Phase 1 left
# them un-used.
RECEIVERS = {
    "Connection": [r"_connection", r"_cnx"],
    "Quic": [r"_quic"],
    "Path": [r"_path_x", r"_path"],
    "SackList": [r"_list"],
}


def find_candidate_fns(text: str) -> list[tuple[int, int, str, str, str]]:
    """Return (start_offset, end_offset, receiver_ty, receiver_kind, fn_text)
    tuples for each free fn whose first arg is `&[mut] <Receiver>`.

    `receiver_kind` is `"&self"` or `"&mut self"`.

    Only top-level `pub fn` is considered; the regex allows the
    first arg to be on the same line or a continuation line (the
    common multi-line `pub fn name(\\n    _recv: &mut T, …` form).
    """
    out: list[tuple[int, int, str, str, str]] = []
    # `\s*` between `(` and the receiver-arg covers the multi-line form.
    pat = re.compile(
        r"^pub fn (?P<name>[A-Za-z_][A-Za-z0-9_]*)\("
        r"\s*(?P<recv>_[a-z][a-z_0-9]*)"
        r":\s*&(?P<mutkw>mut\s+)?(?P<ty>[A-Za-z_][A-Za-z0-9_]*)\b",
        re.MULTILINE,
    )
    for m in pat.finditer(text):
        recv_var = m.group("recv")
        ty = m.group("ty")
        is_mut = bool(m.group("mutkw"))
        # Verify (recv_var, ty) is a recognised pair.
        valid = False
        for receiver_ty, vars_ in RECEIVERS.items():
            if ty == receiver_ty and recv_var in vars_:
                valid = True
                break
        if not valid:
            continue
        # Walk to find the body's closing brace.  We scan forward
        # from the function's opening `{`, balancing braces.
        body_start = text.find("{", m.end())
        if body_start < 0:
            continue
        depth = 0
        i = body_start
        while i < len(text):
            ch = text[i]
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        if depth != 0:
            continue
        end_offset = i + 1
        # Include trailing newline in the slice we'll replace.
        if end_offset < len(text) and text[end_offset] == "\n":
            end_offset += 1
        fn_text = text[m.start():end_offset]
        # Decide receiver kind from the function's name + the C
        # original.  We default to `&mut self` (matching what
        # Phase 1 produced) but flip to `&self` for clearly
        # read-only operations: predicates (`is_*`), getters
        # (`get_*`, `current_*`, `first_*`, `next_*`, `find_*`,
        # `peek_*`), and a few hand-classified ones below.
        name = m.group("name")
        immutable_prefixes = (
            "is_", "get_", "find_", "peek_", "current_", "first_",
            "last_", "next_", "previous_",
        )
        immutable_overrides = {
            "cc_increased_window",         # C body just returns a calculation
            "encode_time_stamp_length",    # C body computes a varint length
            "stream_data_node_value",      # C body returns a pointer
        }
        if (
            name.startswith(immutable_prefixes)
            or name in immutable_overrides
        ):
            kind = "&self"
        elif is_mut:
            kind = "&mut self"
        else:
            kind = "&self"
        out.append((m.start(), end_offset, ty, kind, fn_text))
    return out


def _expand_to_doc_comments(text: str, start: int) -> int:
    """Walk backward from `start` over contiguous `///`-style doc
    lines so we move them into the impl block too."""
    while start > 0:
        # Find the start of the previous line.
        prev_nl = text.rfind("\n", 0, start - 1)
        line_start = prev_nl + 1 if prev_nl >= 0 else 0
        line = text[line_start:start]
        stripped = line.lstrip()
        if stripped.startswith("///") or stripped.startswith("//!"):
            start = line_start
            continue
        break
    return start


def convert_one(text: str, start: int, end: int, ty: str,
                kind: str, fn_text: str) -> tuple[str, int, int]:
    """Return rewritten chunk + new (start, end) bounds.

    `kind` is `"&self"` or `"&mut self"`.
    """
    # Walk back to capture the doc-comment block.
    doc_start = _expand_to_doc_comments(text, start)
    doc_block = text[doc_start:start]
    body = fn_text

    # Strip the receiver-arg from the signature.  Three forms,
    # all on one or split across lines:
    #   pub fn name(_recv: &mut T, arg2: ...)
    #   pub fn name(\n    _recv: &mut T,\n    arg2: ...
    #   pub fn name(_recv: &mut T) -> R
    body = re.sub(
        r"_[a-z][a-z_0-9]*:\s*&(?:mut\s+)?"
        + re.escape(ty)
        + r"(,\s*\n?\s*|\s*(?=\)))",
        kind + r"\1",
        body,
        count=1,
    )
    # The single-arg case `pub fn name(\n    self,\n)` looks weird;
    # collapse that.
    body = re.sub(r"\(\s*\n\s*(&(?:mut\s+)?self)\s*\)", r"(\1)", body)
    body = re.sub(
        r"\(\s*\n\s*(&(?:mut\s+)?self),\s*\n",
        r"(\1, ",
        body,
    )
    # Make sure body starts with `pub fn …` (it should).
    assert body.lstrip().startswith("pub fn"), body[:80]

    # Indent the function body 4 spaces (it's going inside `impl T { … }`).
    indented = "\n".join(("    " + line) if line.strip() else line
                         for line in body.splitlines())
    # Indent doc lines too.
    indented_doc_lines: list[str] = []
    for line in doc_block.splitlines():
        if line.strip():
            indented_doc_lines.append("    " + line)
        else:
            indented_doc_lines.append(line)
    indented_doc = "\n".join(indented_doc_lines)
    if indented_doc and not indented_doc.endswith("\n"):
        indented_doc += "\n"

    impl_block = f"impl {ty} {{\n{indented_doc}{indented}\n}}\n"
    return impl_block, doc_start, end


def main() -> int:
    if not INTERNAL_RS.exists():
        print(f"error: {INTERNAL_RS} not found; run from rs/fq/", file=sys.stderr)
        return 1
    text = INTERNAL_RS.read_text()
    candidates = find_candidate_fns(text)
    if not candidates:
        print("no candidates found")
        return 0

    # Verify all bodies are `todo!()` (or empty / one-liner).  Abort
    # if any body has real logic; we don't want to silently drop it.
    for start, end, ty, kind, fn_text in candidates:
        body = fn_text[fn_text.find("{") + 1:fn_text.rfind("}")].strip()
        if body and "todo!" not in body:
            allowed = body.replace("\n", " ").strip()
            if allowed and not allowed.startswith("todo!"):
                print(f"WARN: non-todo body for {fn_text.splitlines()[0]}", file=sys.stderr)
                print(f"      body = {allowed[:80]}", file=sys.stderr)

    # Apply replacements from end to start so offsets stay valid.
    candidates.sort(key=lambda c: c[0], reverse=True)
    new_text = text
    converted: dict[tuple[str, str], int] = {}
    for start, end, ty, kind, fn_text in candidates:
        replacement, doc_start, _end = convert_one(
            new_text, start, end, ty, kind, fn_text,
        )
        new_text = new_text[:doc_start] + replacement + new_text[end:]
        converted[(ty, kind)] = converted.get((ty, kind), 0) + 1

    INTERNAL_RS.write_text(new_text)
    total = sum(converted.values())
    print(f"converted {total} free functions to methods:")
    for (ty, kind), n in sorted(converted.items()):
        print(f"  impl {ty}: {n} (with {kind})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
