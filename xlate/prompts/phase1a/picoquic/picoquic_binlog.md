# Phase 1A self-review: `picoquic/picoquic_binlog.h`

You're doing a self-review pass on a Phase 1 translation.
Re-read the existing translation and apply quality
improvements in place.  The existing module is gate-clean
and represents real work — refine via `Edit`, never rewrite
from scratch.

## Required reading
1. `TRANSLATE_PLAN.md` — Phase 1A section especially.
2. `CLAUDE.md` — project conventions, edit-scope rules.
3. The existing translation: `rs/fq/src/picoquic/picoquic_binlog.rs`.
4. (No matching `.c` file — header-only module.)
5. Headers this one directly depends on:
   - `picoquic/picoquic_internal.h` (translation: `rs/fq/src/picoquic/picoquic_internal.rs`)

## What to look for
- **Safety holes** — raw pointers used without a clear
  `// SAFETY:` story, `unsafe` blocks that have a safe
  equivalent, ownership patterns that smell wrong.
- **Type-shape consistency** — same C type translated
  differently in different functions of the same module;
  signed/unsigned that disagrees with caller arithmetic;
  integer widths that drift from the C source.
- **Idiom** — `&[T]` vs raw pointer + length, `Option<&T>`
  for nullable, owned `String` vs borrowed `&str`,
  function-pointer typedef → trait, doc-comment placement.
- **Naming** — Rust traits are `PascalCase` even when the
  C origin is `snake_case`.  Preserve the C name where
  callers reference the typedef identifier; let Rust
  convention win for purely internal traits.
- **Lint allowances** — tighten any `#![allow(…)]` that's
  wider than necessary.
- **Documentation** — every public item should have a doc
  comment citing the C source and explaining non-obvious
  shape decisions.

## How to work
1. Read the materials above.
2. Identify improvements you'd apply.
3. Apply them via `Edit` to `rs/fq/src/picoquic/picoquic_binlog.rs` (and sibling
   modules under `rs/fq/src/picoquic/` if a coordinated
   change is needed).
4. Validate with **both** of these (Bash tool's cwd is the
   repo root, so prefix with `cd rs/fq && `):
     cd rs/fq && cargo check
     cd rs/fq && cargo clippy -- -D warnings
   Iterate until both pass cleanly.
5. Report on stdout: a one-paragraph summary of what you
   changed and why.

If nothing needs improvement, say so on stdout and exit
without editing — that is a valid outcome.

## Constraints
- You may only edit files under `rs/fq/src/picoquic/`.
  Do NOT touch `lib.rs`, `Cargo.toml`, parent `mod` files,
  or anything outside `rs/fq/`.
- Don't create new files (`Write` is not in your allowlist).
- Don't run `cargo test` or `cargo fmt` — the parent
  script handles formatting after you finish.
- This pass should not introduce `// REVIEW` comments;
  those are the human reviewer's tool in Phase 1B.
