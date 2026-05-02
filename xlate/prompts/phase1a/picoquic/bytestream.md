# Phase 1A self-review: `picoquic/bytestream.h`

You're doing a self-review pass on a Phase 1 translation.
Re-read the existing translation and apply quality
improvements in place.  The existing module is gate-clean
but is too deferential to the C API — your job is to make
it idiomatic Rust.  Refine via `Edit`, never rewrite from
scratch.

## Required reading
1. `TRANSLATE_PLAN.md` — Phase 1A section especially.
2. `CLAUDE.md` — project conventions, edit-scope rules.
3. The existing translation: `rs/fq/src/bytestream.rs`.
4. The matching `.c` file: `picoquic/bytestream.c`.
5. Headers this one directly depends on:
   - `picoquic/picoquic_internal.h` (translation: `rs/fq/src/picoquic/picoquic_internal.rs`)

## Top-level rule

**The translation should not look obviously C-derived.**
Would a Rust programmer who hadn't seen the C source write
this?  If not, change it.  Specifically:

- **Naming convention is universal.**  `quic_t` → `Quic`.
  `cnx_t` → `Cnx`.  `state_enum` → `State`.
  Drop `_t` and `_enum` suffixes.  PascalCase types,
  traits, and enum variants.  snake_case fields and
  methods.  `SCREAMING_SNAKE` constants.  Rename **every**
  identifier that doesn't match — types, traits, fields,
  enum variants, free functions.
- **Free functions on a primary `&T` / `&mut T` argument
  are methods on `T`.**  `set_low_memory_mode(quic: &mut
  Quic, …)` → `impl Quic { fn set_low_memory_mode(&mut
  self, …) }`.  Same for getters, builders, and any
  function whose first argument is the type's "self".
- **Use Rust's memory model.**  `Drop` replaces explicit
  `free`.  `Vec` / owning `String` replace `malloc`/`free`
  pairs.  `Box<T>` parameters when the function doesn't
  need heap-stable storage are wrong — fix them (clippy
  flags this as `boxed_local`).
- **Use Rust's error model.**  `Result<T, Error>`, not
  `Result<T, ()>` or `i32` status codes.  Sentinel return
  values like `-1` map to `Err`, not `Ok(-1)`.

## Other things to look for

- **Safety holes** — raw pointers used without a clear
  `// SAFETY:` story, `unsafe` blocks that have a safe
  equivalent, ownership patterns that smell wrong.
- **Type-shape consistency** — same C type translated
  differently in different functions of the same module;
  signed/unsigned that disagrees with caller arithmetic;
  integer widths that drift from the C source.
- **Idiom plumbing** — `&[T]` vs raw pointer + length,
  `Option<&T>` for nullable, owned `String` vs borrowed
  `&str`, function-pointer typedef → trait, doc-comment
  placement.
- **Lint allowances** — `#![allow(…)]` blocks are usually
  a smell.  Fix the underlying code instead of suppressing.
- **Documentation** — every public item needs a doc comment
  explaining what the Rust function does.  A note like
  `C: \`name\`` is fine for traceability but isn't a
  substitute for explaining the Rust API.

## How to work
1. Read the materials above.
2. Identify improvements you'd apply.
3. Apply them via `Edit` to `rs/fq/src/bytestream.rs` (and sibling
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
- You may edit any file under `rs/fq/src/`.  Renaming a
  type or converting a free function to a method may force
  callers in sibling modules to change — that's expected.
- `lib.rs` is the kitchen-sink module (the crate's public
  API); editing it for a coordinated rename or method
  conversion is fine.  Do NOT touch `Cargo.toml` or
  anything outside `rs/fq/`.
- Don't create new files (`Write` is not in your allowlist).
- Don't run `cargo test` or `cargo fmt` — the parent
  script handles formatting after you finish.
- This pass should not introduce `// REVIEW` comments;
  those are the human reviewer's tool in Phase 1C.
