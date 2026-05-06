# Phase 4 implementation translation: `rs/fq/src/lb.rs`

Translate `todo!()` function bodies in this Rust source
file by mirroring the C bodies in `picoquic/`.  The Phase 3
test suite is the gate: a function is done when tests that
exercise it stop panicking.

This file currently has **5** `todo!()` bodies to fill.

## What this is

Phase 1 / 2 designed the Rust API surface.  Phase 3 translated
the C tests against that surface (they currently panic on
`todo!()`).  **Phase 4 fills the bodies** so those tests
pass.  The C source is the spec; produce idiomatic Rust
that satisfies the same invariants.

## Required reading (in order)

1. **`xlate/impl_translation_guide.md`** — Rust idioms for
   common C body patterns (memory, control flow, errors,
   strings, time, network, logging, crypto).  Read this once.
2. The Rust target: `rs/fq/src/lb.rs` — every `todo!()` body in here
   is your work.
3. The matching C source.  Most modules under `rs/fq/src/`
   correspond to one or more files under `picoquic/`; the
   guide names the mapping.  Use `Glob` / `Grep` against
   `picoquic/*.c` to find the C bodies for each function.
4. The tests that exercise this module: `rs/fq/src/tests/`.

## Translation rules

- **Faithful, idiomatic.**  Walk the C body and produce
  the equivalent Rust.  Don't reshape the algorithm.
- **Don't change signatures.**  The Phase 1 / 2 / 3 design
  is settled.  If a body genuinely cannot be expressed under
  the existing signature, surface it on stdout and skip the
  function — that is a human design call, not yours.
- **No `unsafe`.**  No edits outside `rs/fq/src/`.
- **Idiomatic Rust over C-mirroring.**  See the guide for
  the canonical mappings (linked list -> Vec/VecDeque,
  malloc -> Box, void* -> Box<dyn Trait>, int 0/-1 -> Result).
- **Don't introduce new public items.**  Helpers are fine
  but should be private (`fn` without `pub`).
- **Tests are the gate.**  After each meaningful chunk, run
     cd rs/fq && cargo test --no-run
     python3 scripts/phase4_check.py rs/fq/src/lb.rs
  to confirm the file still compiles and how many `todo!()`s
  remain.  Iterate until zero.

## Process

1. Read the impl translation guide (once).
2. Read the Rust target `rs/fq/src/lb.rs` end-to-end.
3. Identify the corresponding C source(s) under `picoquic/`.
   The first line of each `todo!()` function's doc comment
   typically names the C function (`/// C: \`picoquic_xxx\``).
4. For each `todo!()`:
   a. Find the C body.
   b. Translate idiomatically per the guide.
   c. Run `cargo test --no-run` to confirm it still compiles.
5. After every `todo!()` in this file is replaced, run:
     cd rs/fq && cargo fmt
     cd rs/fq && cargo test --no-run
     cd rs/fq && cargo clippy --tests --all-features -- -D warnings
     python3 scripts/phase4_check.py rs/fq/src/lb.rs
   All four must pass.
6. Optionally run `cargo test` (full) and report which
   previously-panicking tests now pass.
7. Report on stdout: a one-paragraph summary.

## Stop spinning

If 5 grep/read tool calls into the same file haven't found
what you need, **stop searching**.  Open the C source and
the Rust target side by side; the C function name is in
the doc comment.
