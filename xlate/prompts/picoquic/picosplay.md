# Phase 1 translation: `picoquic/picosplay.h` → Rust

You are running as part of a fully-automated translation pipeline.
Your job is to translate **one C header** to a Rust module,
following Phase 1 of the project's translation plan.

## Required reading (in this order)
1. `TRANSLATE_PLAN.md` — the binding plan and rules.  Phase 1
   is the section that applies here.
2. `CLAUDE.md` — project conventions, especially the rule that
   you may only edit files under `rs/`, `scripts/`, or `*.md`.
3. The C header you are translating: `picoquic/picosplay.h`.
4. The matching `.c` file: `picoquic/picosplay.c`.
5. (No in-scope dependencies.)
6. Reference scaffolding (do NOT ship; for diff comparison only):
   `xlate/bindgen_baseline/picoquic/picosplay.rs`

## What to produce
Write a Rust module at:

    rs/fq/src/picoquic/picosplay.rs

A placeholder file already exists at that path and is wired
into the crate via `lib.rs` and parent `mod` files — your
edits will be picked up by `cargo check` immediately.

**Note on pre-existing stubs.**  Earlier headers in this run
may have left minimal `todo!()` stubs at sibling paths under
`rs/fq/src/picoquic/` so they could compile.  If the file at
your target path is one of those stubs (rather than the
untouched placeholder), treat it as throw-away scaffolding:
replace it with the full translation of your header.  The
earlier dependents may need updates after that — fix any
compile breakage their files now show during your `cargo
check` loop, but only when the fix is mechanical (e.g.
filling in a now-defined field, adjusting an import path).
Do not redesign other modules.

### Phase 1 contract (binding)
- All function bodies are `todo!()`.  Phase 3 fills them in.
- Add an empty `#[cfg(test)] mod test {}` at the end of the
  module (Phase 2 fills it).
- Drop `#[repr(C)]` *except* on types that cross an external
  boundary (FFI, syscall, wire format).
- Pointer fields and parameters get a Rust shape decided by
  reading callers: `&T`, `&mut T`, `Box<T>`, `Vec<T>`, `&[T]`,
  `Option<T>` for nullable.  `Rc<RefCell<T>>` is the
  last-resort escape hatch.  Raw pointers only inside `unsafe`,
  and only with a `// SAFETY:` comment.
- Function pointers map to traits.
- Bitfields → integer field with mask/shift accessors (or
  `bitflags!` for flag bitsets).
- Unions → Rust `enum`.
- Flexible array members → `Box<[T]>` or `Vec<T>`.
- Macros: `const` for value `#define`s, `fn` for function-like
  macros, `macro_rules!` only when there's no alternative.
- The crate is `#![no_std] + alloc` with an `std` Cargo
  feature.  Use `core::` / `alloc::` paths; do not introduce
  `std::` references except behind a feature gate.
- One top-level `crate::Error` enum; functions return
  `Result<T, Error>`.  v1 may not have one yet — if so, do
  not invent it; use `Result<T, ()>` or skip the error type
  and leave a TODO comment naming the gap.
- `Send`/`Sync` are NOT required (single-threaded scope).

### Procedure
1. Read `TRANSLATE_PLAN.md` and the materials above.
2. For each pointer field/parameter, grep the C source under
   `picoquic/` to see how callers pass it (NULL? `&local`?
   owning new allocation?  freed by callee?).  Use the
   evidence to pick the Rust shape.  Comment your choice
   briefly when non-obvious.
3. Replace the placeholder content of `rs/fq/src/picoquic/picosplay.rs`.
4. From `rs/fq/`, run **both** `cargo check` and
   `cargo clippy -- -D warnings`.  Iterate until **both**
   pass cleanly.  These match the parent script's gate, so
   converging here means you're done.  If a callee module
   doesn't exist yet, create a minimal stub at the right
   path with `todo!()` placeholders for the types you need.
   Don't translate other headers.
5. Do NOT run `cargo test` or `cargo fmt` — the parent
   script runs `cargo fmt` after you finish.

### Constraints
- You may only create / edit files under `rs/fq/src/`.  In
  particular, do NOT touch C sources, CMake, `xlate/`, or
  anything outside `rs/fq/`.
- Do NOT modify `rs/fq/Cargo.toml` unless the translation
  genuinely requires a new dependency.  If it does, add it
  with `default-features = false` and note why in your
  response.
- The parent script regenerates `pub mod` lines in `lib.rs`
  and parent `mod` files.  Don't waste turns editing them; the
  pre-wired layout is enough for `cargo check`.

Report when done with a one-line summary of any non-obvious
choices you made.
