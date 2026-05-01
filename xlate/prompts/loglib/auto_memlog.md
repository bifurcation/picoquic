        # Phase 1 translation: `loglib/auto_memlog.h` → Rust

        You are running as part of a fully-automated translation pipeline.
        Your job is to translate **one C header** to a Rust module,
        following Phase 1 of the project's translation plan.

        ## Required reading (in this order)
        1. `TRANSLATE_PLAN.md` — the binding plan and rules.  Phase 1
           is the section that applies here.
        2. `CLAUDE.md` — project conventions, especially the rule that
           you may only edit files under `rs/`, `scripts/`, or `*.md`.
        3. The C header you are translating: `loglib/auto_memlog.h`.
        4. (No matching `.c` file — header-only module.)
5. (No in-scope dependencies.)
6. Reference scaffolding (do NOT ship; for diff comparison only):
   `xlate/bindgen_baseline/loglib/auto_memlog.rs`

## What to produce
Write a Rust module at:

    rs/fq/src/loglib/auto_memlog.rs

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
   `picoquic/`, `picohttp/`, `loglib/`, `picoquictest/` to see
   how callers pass it (NULL? `&local`? owning new
   allocation?  freed by callee?).  Use the evidence to pick
   the Rust shape.  Comment your choice briefly when
   non-obvious.
3. Write `rs/fq/src/loglib/auto_memlog.rs`.
4. From `rs/fq/`, run `cargo check`.  Iterate until it
   passes.  If a callee module doesn't exist yet, that's
   expected — add a `todo!()` stub for it under the right
   submodule, but only if it's part of the type signature
   you're writing.  Don't translate other headers.
5. Do NOT run `cargo test`, `cargo fmt`, or `cargo clippy` —
   the parent script handles those after you finish.

### Constraints
- You may only create / edit files under `rs/fq/src/`.  In
  particular, do NOT touch C sources, CMake, `xlate/`, or
  anything outside `rs/fq/`.
- Do NOT modify `rs/fq/Cargo.toml` unless the translation
  genuinely requires a new dependency.  If it does, add it
  with `default-features = false` and note why in your
  response.
- Do NOT modify `rs/fq/src/lib.rs` to add `pub mod` lines —
  the parent script regenerates module wiring.

Report when done with a one-line summary of any non-obvious
choices you made.
