# Plan: Translate a C library to safe, idiomatic Rust

## Goal

Produce a Rust library that is functionally equivalent to the C source —
the same observable behavior, exercised by the same test cases — written in
safe, idiomatic Rust.  The Rust source should mirror the C source's
structure (modules, functions, control flow, allocation patterns) closely
enough that a reviewer holding both side-by-side can match them
function-for-function.  Deviations from the C structure are allowed only
when (a) safety requires them, or (b) the C idiom has no Rust equivalent.

## Scope

In scope for v1:

* **Only the `picoquic/` directory** — the picoquic-core library.  This
  is CMake's `picoquic-core` target.  `picoquictest/` is translated in
  Phase 3 (just the tests for picoquic-core, i.e., the files in CMake's
  `PICOQUIC_TEST_LIBRARY_FILES` list).  Single target
  (`x86_64-unknown-linux-gnu`).
* `no_std` + `alloc`, with an `std` feature gate.

Out of scope for v1 (explicit follow-on work):

* `loglib/` (the `picoquic-log` target) and `picohttp/` (the
  `picohttp-core` HTTP/3 target).  These layer on top of picoquic-core
  and can be ported in a v2 expansion.

* Multi-threading.  v1 assumes single-threaded C; pthread / atomics /
  volatile usage in the C source becomes a translation note ("threading
  dropped, revisit in v2"), not an in-line decision.
* Multi-platform support.  Code that's purely `#ifdef _WIN32` (or any
  non-target branch) is not translated in v1; other platforms get added
  back later behind Rust `#[cfg(target_os = …)]`.
* Differential testing against the C library.
* Performance benchmarking, fuzzing, mutation testing, coverage gating.
* Stable ABI / FFI compatibility with the existing C library.  This is a
  hard fork — there is no incremental cutover, no shim, no mixed build.

## Guiding rules (apply throughout)

1. **Safety always wins.**  When safe-idiomatic Rust and bug-for-bug
   compatibility conflict (integer overflow, signed shifts, overlapping
   `memcpy`, uninitialized reads, pointer provenance), choose the safe
   idiom even if outputs diverge for pathological inputs.
2. **Mirror the C source line-by-line.**  Same control flow, same variable
   lifetimes, same allocation pattern.  The Rust function should look like
   a translation, not a reinterpretation.
3. **Bug-for-bug compatibility is the default behavior**, achieved by
   source-level mimicry.  Empirical equivalence with the C library will be
   verified later, in a follow-on phase, by a binary that links both
   libraries and diffs outputs on shared inputs.

## Cross-cutting policies

### Memory and ownership

* Default to `Box`, owned values, and borrowing.  Follow the C ownership
  model where it makes sense.
* `Rc<RefCell<T>>` is the last-resort escape hatch, used only when the
  ownership graph genuinely isn't a tree (mutual references, callbacks
  with shared mutable state).
* `Send` and `Sync` are not required in v1 (single-threaded scope).  v2
  multi-threading will revisit ownership comprehensively.

### Pointer translation (Phase 1 rules)

* Pointer fields and parameters get a per-case Rust shape decided by
  reading callers: `&T`, `&mut T`, `Box<T>`, `Rc<RefCell<T>>`, `Vec<T>`,
  `&[T]`, `Option<T>` for nullable, raw pointer (only inside `unsafe`).
* Function pointers map to traits.
* Bitfields → ordinary integer fields with mask/shift accessors (or
  `bitflags!` for flag bitsets).
* Packed structs → normal Rust structs plus serialization helpers for any
  wire-format use.
* Unions → Rust `enum`.
* Flexible array members → `Box<[T]>` or `Vec<T>`.
* Mutually recursive *types* get an indirection (`Box`, `Rc`) or are
  co-located in a shared parent module.
* Cross-module function references are fine — `super::` paths resolve
  mutual recursion at the function level without restructuring.
* `repr(C)` is dropped except where layout is part of an external
  contract (FFI, syscall, wire format).

### `unsafe`

* Avoid unless absolutely necessary.
* Before writing `unsafe`, the translator explicitly considers and
  documents safe alternatives that didn't work.
* Every `unsafe` block carries a `// SAFETY:` comment stating the
  invariants the caller must uphold.
* The codebase stays greppable for `unsafe` so occurrences can be counted
  and reviewed.

### Error handling

* One top-level `Error` enum at the crate root.
* Functions return `Result<T, Error>`.  C status-code returns become
  `Result<(), Error>`; C output parameters become the `T` in `Result<T,
  Error>`.
* Module-level sub-error types are allowed and convert into the top-level
  enum via `From` impls.
* `errno`-style global state is dropped; the error rides in the `Result`.

### `no_std` + `alloc` + `std` feature

* `#![no_std]` at the crate root, `extern crate alloc;` for `Vec`, `Box`,
  `String`.
* `std` is a Cargo feature, on by default.
* `core::error::Error` (stable since 1.81) replaces `std::error::Error`.
* `core::time::Duration` is fine; `Instant` is std-only — any timing API
  takes a clock trait or is `std`-gated.
* `HashMap` is std-only — use `hashbrown` (or `BTreeMap`) for maps.
* I/O traits are std-only — gate any I/O behind the `std` feature, or use
  `core2`/`embedded-io`.
* Every dependency is added with `default-features = false` and vetted
  for `no_std` compatibility.

### Logging, errors, asserts (concrete crates)

```toml
log       = { version = "0.4", default-features = false }
thiserror = { version = "2",   default-features = false }
```

* Logging: `log` (the de-facto standard, no_std-friendly).  `tracing` is
  not used in v1.
* Errors: `thiserror` v2 for the top-level enum.  Or hand-rolled
  `impl core::error::Error` if we want zero error-handling dependencies.
* Asserts and panics: built-ins (`assert!`, `debug_assert!`, `panic!`).
  C `abort()` maps to `core::process::abort` or `panic!`.

## Phase 0 — Inventory

A one-shot artifact, regenerated only when the C source changes
substantively.

### Inputs

* The C project's CMake build (assumed canonical — Makefiles and IDE
  projects are ignored).
* A chosen target triple: `x86_64-unknown-linux-gnu`.

### Steps

1. Run `cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON …` to produce
   `build/compile_commands.json`.  This is the source of truth for
   files, compile flags, and `-D` defines.  It lives in the cmake
   build dir; everything else the translation pipeline produces
   lives in `xlate/`.
2. For each translation unit, run `clang -E -P` (or `unifdef -D… -U…`)
   using the flags from `compile_commands.json` to materialize one
   fully-expanded `.c` file with all `#ifdef` branches resolved for the
   chosen target.  This eliminates conditional-compilation branches from
   the AST that downstream phases see.
3. Parse each preprocessed file with libclang (Python bindings) and emit
   `xlate/inventory.json` containing:
   * the source-file list, classified public vs. internal,
   * function signatures (name, return type, parameters),
   * struct, enum, and union definitions,
   * the include graph,
   * the call graph,
   * the test inventory (test source files, fixture paths, expected
     outputs),
   * the third-party-dependency list (versions if pinnable).
4. Build the call graph, including indirect-call edges through function
   pointers where statically resolvable.  Compute height per function
   (BFS from leaves: leaves are height 0; every other node's height is
   the shortest path to a leaf).  Identify mutually recursive function
   pairs — these will be translated as a unit in Phase 4.  Output:
   `xlate/call_graph.json`.
5. Build a progress dashboard at `xlate/dashboard.html`.  Inputs: the
   call/include graph from step 3 plus a scan of the (initially empty)
   Rust crate for `todo!()` markers.  Output: a single static HTML
   page rendering the graph at file granularity by default, with
   click-to-expand to function granularity, with nodes colored:
   * **C-only** — no Rust counterpart yet,
   * **stubbed** — Rust signature exists but body is `todo!()`,
   * **translated** — body implemented but not yet exercised by a passing
     test,
   * **translated-and-tested** — exercised by at least one passing test.
   Per-file rollup percentages on the side.

### Pitfall: dropped `#ifdef` branches

The preprocess-then-parse approach means we lose visibility into every
non-target branch.  That's fine for v1 — those branches are explicitly
out of scope — but the inventory should also produce a list of every
`#ifdef` symbol seen in the original source, classified as always-on,
always-off, or per-target, so we know what was dropped and can add it
back later.  Output: `xlate/ifdef_manifest.json`.

## Phase 1 — Stub the API, file by file

**Operate file by file.**  For each header, the translator works with
that header, the matching `.c` file, and any headers it directly depends
on, and nothing else.  This bounds the per-file context and keeps the
work parallelizable across translators (human or AI).

### Per-header procedure

1. **Generate a bindgen baseline.**  Run `bindgen` with a strict allowlist
   restricted to symbols defined *in this header*:
   ```
   bindgen --allowlist-file '.*<filename>\.h$' --no-layout-tests …
   ```
   Store the output as `bindgen_baseline/<file>.rs`.  This is reference
   scaffolding for diff comparisons — never committed as the production
   module.
2. **Translate the header to a Rust module** (`src/<file>.rs`):
   * Drop `repr(C)` (except where the type crosses an external boundary).
   * Refactor structs to safe, idiomatic shapes.
   * Promote free functions to inherent or trait methods on the relevant
     struct.
   * Map function pointers to traits (one trait per typedef by default;
     trait grouping is a judgment call).
   * Map pointer parameters and fields per the rules above (read callers
     to decide).
   * Map macros per case: `const` for value `#define`s, `fn` for
     function-like macros, `macro_rules!` only when there's no other way,
     `build.rs` codegen for X-macro tables.
   * Map external dependencies to crates.io / `core` / `alloc` / sys
     crates — never reimplement.
3. **Bodies are `todo!()`.**  Signatures may be wrong; we'll find out in
   Phase 4 and refine.
4. **Add an empty `#[cfg(test)] mod test {}`** at the end of the module.
5. **Per-file gate:** `cargo fmt`, `cargo clippy -- -D warnings`,
   `cargo check` all pass.

### Scripting

* Driver script (Python) that, for each header, runs bindgen with the
  derived allowlist, generates the empty module skeleton, and stages the
  baseline file for human review.  The translator never writes
  boilerplate.
* AST analysis tool that, for each pointer parameter, prints: callers,
  whether they pass `NULL`, whether the callee writes through it, whether
  ownership transfers (look for `free` on the same pointer downstream).
  This is *evidence* for the translator's pointer-shape decision — the
  decision itself stays human.

## Phase 1A — AI self-review of translated headers

After Phase 1 lands a Rust stub for each header, run a self-review
pass: an AI re-reads each translated module against the project's
quality bar (safety, consistency, idiomatic Rust) and applies
improvements in place.

This is *not* a regeneration.  The existing translation is the
starting point and represents real work that already passes the
gate.  The reviewer Edits, never Writes from scratch.

### Top-level rule: idiomatic Rust, not C with Rust syntax

**The translation should not look obviously C-derived.**  The
guiding question is: would a Rust programmer who hadn't seen the
C source write this?  If not, change it.  Specifically:

* **Naming convention is universal.**  `quic_t` → `Quic`.
  `cnx_t` → `Cnx`.  `state_enum` → `State`.
  `pmtud_policy_enum` → `PmtudPolicy`.  Drop the `_t` and `_enum`
  suffixes; PascalCase types and traits; PascalCase enum variants;
  snake_case fields and methods; `SCREAMING_SNAKE` constants.
  This applies to **every** name — types, traits, fields, enum
  variants, free functions — not just traits.
* **Free functions on a primary `&T` / `&mut T` argument are
  methods on `T`.**  `set_low_memory_mode(quic: &mut Quic, …)`
  → `impl Quic { fn set_low_memory_mode(&mut self, …) }`.  Same
  for getters, builders, and any function whose first argument
  is the type's "self".
* **Use Rust's memory model.**  `Drop` replaces explicit `free`
  (`fn free(self: Box<Quic>)` → `impl Drop for Quic`); `Vec` /
  `Box` / owning `String` replace `malloc`/`free` pairs;
  `Rc<RefCell<_>>` only when ownership genuinely is a graph;
  no `Box<T>` parameters when the function doesn't need
  heap-stable storage (clippy's `boxed_local` flags these).
* **Use Rust's error model.**  `Result<T, Error>`, not
  `Result<T, ()>` or `i32` status codes.  Sentinel return values
  like `-1` map to `Err`, not `Ok(-1)`.

### What else to look for

* **Safety holes** — raw pointers used without a clear `// SAFETY:`
  story, `unsafe` blocks that have a safe equivalent, ownership
  patterns that smell wrong.
* **Type-shape consistency** — the same C type translated
  differently in different functions of the same module;
  signed/unsigned choices that disagree with caller arithmetic;
  integer widths that drift from the C source.
* **Idiom plumbing** — `&[T]` vs raw pointer + length,
  `Option<&T>` for nullable, owned `String` vs borrowed `&str`
  for caller-supplied text, function-pointer typedef → trait,
  doc-comment placement.
* **Lint allowances** — `#![allow(…)]` blocks added to silence
  the initial gate are usually a smell.  Tighten to the minimum
  scope, or fix the underlying code instead of suppressing.
* **Documentation** — every public item should have a doc comment
  explaining non-obvious shape decisions.  A note like
  `C: \`picoquic_create\`` is fine for traceability but is not
  a substitute for explaining what the Rust function does.

### Per-header procedure

1. Read `TRANSLATE_PLAN.md` (this document) and `CLAUDE.md`.
2. Read the C header, the matching `.c` file, and the existing
   Rust translation at `rs/fq/src/<path>.rs`.
3. Identify improvements.
4. Apply them via `Edit` operations.  No full rewrites.
5. Run `cargo check` and `cargo clippy -- -D warnings` from
   `rs/fq/`.  Iterate until both pass.
6. Report the changes in a one-paragraph summary on stdout.

If nothing needs changing, say so on stdout and exit without
editing — that's a valid outcome.

### Scripting

`scripts/phase1a.py` drives the pass:

* Iterates Phase 1 `ok` headers in topological order.
* Per header: composes a review prompt, invokes `claude -p` with
  `--allowedTools "Read Edit Glob Grep Bash(cargo check)
  Bash(cargo clippy)"` — no `Write`, this is refinement only.
* Tracks state in `xlate/phase1a_state.json` (status: `ok` /
  `fail` / `noop`).
* Resumable; continues on failure by default.
* Per-header transcripts at `xlate/claude_logs/phase1a/<path>.log`.

## Phase 1B — Cross-module consistency report

1A is per-header by design — claude sees one module deeply but
isn't well-suited to spot patterns that vary *between* modules
(trait naming conventions, lint allowances, type definitions
that drift, the same C type translated differently in different
places).  A whole-crate AI pass would burn context and turns
without a clear consistency oracle to compare against.

Phase 1B is **pure inspection — no AI, no edits**.  A script
scans `rs/fq/src/` and emits a markdown report cataloging the
patterns that vary across modules.  The human reviewer reads
the report, decides on consistency policies, and then sprinkles
`// REVIEW: <instruction>` markers in the source for Phase 1C
to address.

### What the report contains

* **Trait names by case style** — snake_case (mirroring C
  typedefs) vs. PascalCase (Rust idiom), with module locations
  for each.
* **Module-level lint allowances still present** — which `#![allow]`
  attributes survive in which modules.  Diversity here suggests
  inconsistent reasoning during translation.
* **Type definitions across modules** — every `pub struct`,
  `pub enum`, `pub type` declaration, with duplicates flagged
  (same name defined in multiple modules: usually means an
  opaque stub somewhere needs to be replaced by the real
  definition's import).
* **Cross-module imports** — `use crate::picoquic::…` lines
  with the imported items grouped by source module.

### Scripting

`scripts/phase1b.py` is a single-shot inspection — no claude,
no state machine.  It writes `xlate/consistency_report.md` and
exits.

```sh
python3 scripts/phase1b.py             # write the report
python3 scripts/phase1b.py --json      # also dump raw data
```

### Output goes back into 1C

The human reviewer reads `xlate/consistency_report.md`, decides
the policy ("all traits PascalCase except those whose typedef
name is part of a callback ABI" or whatever), and adds
`// REVIEW: <instruction>` markers where the source disagrees
with the policy.  Phase 1C then implements the changes.

## Phase 1C — Human review with AI execution

After 1A and 1B, the human reviewer reads each module and the
consistency report, then annotates the code with
`// REVIEW: <instruction>` comments wherever they want changes.
Comments can name a target type, a desired pattern, a question
to investigate — anything actionable.  Running
`scripts/phase1c.py` then asks the AI to address them.

### How `// REVIEW` comments work

* Plain `// REVIEW: <instruction>` — the AI addresses the request
  and removes the comment.
* `// REVIEW(open): <reason>` — the AI tried and could not resolve
  automatically; the comment stays for further human attention.

The plain form is greppable as `// REVIEW: ` (with the colon and
space) so the open form doesn't trigger another pass.

### Per-file procedure

For each file containing `// REVIEW: ` markers:

1. Read the file and the inline REVIEW comments.
2. For each comment, attempt the requested change.  When done,
   remove the `// REVIEW: ` line.  When stuck — the request needs
   more context than the file gives, requires breaking the gate,
   or the human's intent is unclear — rewrite as
   `// REVIEW(open): <reason>` instead.
3. Run `cargo check` and `cargo clippy -- -D warnings`.  Iterate.
4. Report a one-line summary: how many resolved, how many left
   open.

### Scripting

`scripts/phase1c.py` drives the pass:

* Scans `rs/fq/src/` for files containing `// REVIEW: ` (the
  open form is excluded).
* For each, invokes `claude -p` with the same tool allowlist as
  1A and a prompt naming the file plus the extracted REVIEW
  comments (line numbers + text).
* Tracks state in `xlate/phase1c_state.json`, keyed by Rust
  file path.
* Supports `--file`, `--list`, `--limit`, `--dry-run`,
  `--stop-on-failure`, `--force`.

### Iteration

1B and 1C can interleave: the human reads the report, adds
`// REVIEW:` markers, runs 1C, regenerates the report, repeats.
Each 1C iteration is cheap because `phase1c.py` only touches
files with fresh REVIEW markers — already-resolved files are
skipped.

### Phase 1 acceptance gate (revised)

Phase 1 is complete when:

1. Every in-scope header has a Rust module under `rs/fq/src/`.
2. `cargo fmt` + `cargo clippy -- -D warnings` + `cargo check`
   all pass.
3. Phase 1A self-review has run on every module.
4. The Phase 1B consistency report has been generated and
   reviewed; any policy decisions have been applied via 1C.
5. No plain `// REVIEW: ` comments remain (only `// REVIEW(open):`,
   each with a human-actionable reason recorded).

## Phase 2 — Dependency abstraction

Phase 1 stubs the *library's* surface; Phase 2 stubs the surface
between the library and everything outside it.  The Rust port has
**no fixed external dependencies** — every capability the library
borrows from the outside world is reached through a trait.  A
concrete implementation (picotls, OpenSSL, the OS sockets layer)
satisfies that trait, but is interchangeable.  This is what lets v2
swap picotls for rustls, swap OpenSSL for `ring`, or run on an
embedded target with no allocator changes.

This phase runs after the per-module Phase 1 work has settled
(1A → 1B → 1C) and before Phase 3 brings tests into the picture —
tests need a concrete dependency wiring to link, and that wiring
should be the abstraction layer's first consumer, not an ad-hoc
shim.

### Top-level rules

* **Two-direction traits.**  For each capability boundary, define
  *both* directions explicitly:
  * **Provider trait** — calls that flow *from* the library *into*
    the dependency.  The dependency implements this trait.
    Naming convention: capability noun, no suffix (`TlsStack`,
    `AeadCipher`, `RandomSource`, `Clock`).  These traits live
    next to the module that consumes them.
  * **Callback trait** — calls that flow *from* the dependency
    back *into* the library.  The library implements this trait
    (or supplies a struct that does).  Naming convention:
    capability noun + role (`TlsCallbacks`, `PacketSink`).  These
    traits also live next to the consuming module, even though
    the *implementation* is on a library type.
* **No concrete dependency types in the library's public API.**
  No `picotls::ptls_t` in a function signature, no `openssl::EVP_*`
  in a struct field.  If the C source exposed one, the Rust port
  hides it behind the relevant trait and the trait's associated
  types.
* **Prefer `core` / `alloc` / `std` over external dependencies.**
  Where the C source borrows from a library only because C lacks
  the facility (file I/O, string formatting, integer parsing, byte
  buffer manipulation, sorted maps), the Rust port uses the
  standard library directly:
  * `FILE*` + `fprintf` → `core::fmt::Write` for formatting,
    `std::io::Write` (gated on the `std` feature) for byte sinks.
  * `printf`-style formatting → `write!` / `format!`.
  * `qsort` → `slice::sort_by`.
  * `memcpy` / `memmove` → slice assignment / `copy_from_slice`.
  * `strtoul` / `atoi` → `str::parse`.
  * Sorted maps / sets → `BTreeMap` / `BTreeSet` from `alloc`.

  The trait abstraction is reserved for genuinely external
  capabilities (TLS, crypto, sockets, RNG, system clock), not
  for facilities the language already provides.
* **`std`-only capabilities are gated.**  Anything that pulls in
  `std::io`, `std::net`, `std::time::Instant`, etc., lives behind
  the `std` Cargo feature.  The trait itself is `no_std`-clean;
  the *default implementation* using `std` is feature-gated.

### What gets abstracted (initial inventory)

The list below is the starting point — Phase 2 begins by
re-deriving it from the current Rust source, but these are the
capability boundaries already visible:

* **TLS stack** — currently picotls.  Provider trait covers the
  TLS state machine, key schedule output, and certificate
  verification hooks.  Callback trait covers picoquic's
  ticket store, ALPN negotiation, and transport-parameter
  exchange.
* **Crypto provider** — currently OpenSSL or mbedtls.  Provider
  traits per primitive: `AeadCipher`, `Hash`, `Hkdf`, `Signer`,
  `Verifier`.  Already partially scaffolded as
  `crypto_provider_api.rs` from the C header.
* **Random source** — currently OpenSSL's `RAND_bytes`.
  `RandomSource` provider trait (`fn fill(&mut self, buf: &mut [u8])`).
  A `std`-feature default backed by `getrandom` is acceptable.
* **Clock** — the C library already takes "now" as a parameter,
  so this is purely the *application's* clock.  Define a `Clock`
  trait so applications can inject a virtual clock for the test
  simulator without going through the public `current_time`
  parameter for every call.
* **Sockets / packet I/O** — already abstract in the C library
  (the application feeds packets in and polls them out).  The
  Rust equivalent is a `PacketIo` provider trait plus a
  `PacketSink` callback trait, both `std`-gated for the default
  UDP-socket implementation.
* **Logging output** — `FILE*` in the C source becomes
  `core::fmt::Write` for the formatted-text path and the `log`
  crate's facade for level-filtered events.  No new trait
  needed — these *are* the standard abstractions.

### Per-capability procedure

For each capability:

1. **Locate the boundary.**  Find every place in `rs/fq/src/`
   where the current translation references a concrete external
   type (a stub `extern crate` symbol, an opaque type name lifted
   from a C dependency header, a free function whose only purpose
   is to call into the dependency).
2. **Define the provider trait** in the module that owns the
   capability.  Methods follow the C call patterns observed in
   step 1, with C signatures translated per the Phase 1 pointer
   rules.
3. **Define the callback trait** if the C dependency calls back
   into the library.  Same module.
4. **Refactor library code** to take a `&mut impl Provider` (or
   a generic type parameter, or a `&mut dyn Provider` if dynamic
   dispatch is preferable for object-safety reasons).  Concrete
   dependency types disappear from the library's surface.
5. **Provide a default implementation** for the dependency the
   C library currently uses.  Default implementations live in a
   sibling module (`tls_picotls.rs`, `crypto_openssl.rs`,
   `clock_std.rs`) and are feature-gated where appropriate
   (`#[cfg(feature = "std")]` for OS-backed implementations,
   per-backend Cargo features for swappable backends).
6. **Run the gate.**  `cargo check` for every meaningful feature
   combination: default features, `--no-default-features`,
   `--no-default-features --features alloc`.  `cargo clippy
   -- -D warnings` for the default build.

### Cargo feature layout

After Phase 2, `Cargo.toml` looks roughly like:

```toml
[features]
default      = ["std", "tls-picotls", "crypto-openssl"]
std          = []
tls-picotls  = ["dep:picotls-sys"]
tls-rustls   = ["dep:rustls"]                      # v2 backend
crypto-openssl = ["dep:openssl"]
crypto-mbedtls = ["dep:mbedtls"]                   # v1 alternate
crypto-ring  = ["dep:ring"]                        # v2 backend
```

The library compiles with **any one** TLS backend and **any one**
crypto backend selected, or with neither (consumers supply their
own implementations).  The traits are the contract; the backends
are interchangeable.

### Scripting

`scripts/phase2.py` (to be written) drives the pass per
capability rather than per file:

* Iterates a manually-curated capability list (initially the
  inventory above, refined as the work progresses).
* Per capability: composes a prompt naming the trait to define,
  the C reference(s), and the Rust modules that should consume
  the trait.
* Same `claude -p` allowlist as 1A — Read/Edit/Glob/Grep plus
  cargo gates.
* State at `xlate/phase2_state.json`, keyed by capability name.

### Phase 2 acceptance gate

Phase 2 is complete when:

1. No public function or type in `rs/fq/src/` mentions a concrete
   external dependency.  (Greppable check: no `picotls::`,
   `openssl::`, `mbedtls::`, `getrandom::` outside the
   feature-gated default-implementation modules.)
2. Every capability has a provider trait and, where applicable,
   a callback trait, both documented.
3. A default implementation exists for each dependency the C
   library currently uses, behind the matching Cargo feature.
4. `cargo check --no-default-features --features alloc` passes
   (the library compiles with no backends selected — consumers
   wire their own).
5. `cargo check` and `cargo clippy -- -D warnings` pass with
   default features.

## Phase 3 — Translate tests

For each C test:

1. Identify the API symbols it calls.
2. Look those symbols up in the Phase 1 module map.  If they all live in
   one module, the test goes into that module's `#[cfg(test)] mod test`.
   Otherwise, it goes into a top-level `src/tests.rs` (a sibling module
   file under `src/`, not the `tests/` integration-test directory — that
   directory isn't used in this plan).
3. Translate the *semantics* of the test, not the harness.  C custom
   asserts (`TEST_ASSERT_EQUAL_INT`, etc.) become `assert_eq!` /
   `assert!`.  Tests are written as if the API returned real values —
   the fact that those calls currently `todo!()`-panic is incidental.
4. Tests that depend on undefined or platform-specific C behavior
   (signed overflow, `memcmp` on padded structs, specific `errno`
   values) are flagged for rewriting.
5. Test fixtures (golden files, sample inputs) live under
   `tests/fixtures/` (or `src/test_fixtures/` if we want everything
   under `src/`), reached via a small helper anchored to
   `env!("CARGO_MANIFEST_DIR")`.

### Scripting

* From the Phase 0 inventory, generate one stub `#[test] fn name() {
  todo!() }` per C test case, placed by the rule above.
* A script reads each C test file, extracts API symbols, looks them up
  in the Phase 1 module map, and picks the destination module.

### Phase 3 acceptance gate

`cargo test` runs to completion.  Every test fails by panicking on a
`todo!()` (or matching panic message).  No segfault, no abort, no
compile error.  The panic *is* the clean fail.

## Phase 4 — Translate implementations

### Order

Tests are ordered by the maximum height (per Phase 0) of any function
they reach.  Lowest height first.  Mutually recursive function pairs are
translated together regardless of nominal heights.

### Inner loop

For each test, in order:

1. Pick a `todo!()` function in the call tree below the test.  Prefer
   the one with the lowest height (closest to a leaf).
2. Translate it.  If it calls a function not yet defined, create a
   `todo!()` stub for that function.
3. Run `cargo check`.  Fix until it passes.
4. Repeat until no `todo!()`s remain in the test's call tree.
5. Run `cargo test`.  The current test should pass; previously passing
   tests should still pass; not-yet-translated tests should still fail
   cleanly.

`cargo test` is run only at the *end* of a test's call tree, not after
every function — until the sub-tree is complete, every `cargo test` run
reports the same `todo!()` panic regardless of which leaf was last
translated, so it adds no signal over `cargo check`.

### Signature drift

When translating function `f`, the translator may discover that the
Phase 1 signature for some callee `g` is wrong.  Refining signatures
mid-Phase-3 is allowed — they were always first-guesses.

### Discovered call edges

Static analysis under-approximates call edges through function pointers
and trait objects.  When `cargo test` reveals a `todo!()` panic in a
function the call graph called a leaf, the translator implements the
required trait method and moves on — no need to regenerate the call
graph.

### Tooling for Phase 4

* `scripts/next_todo.py` — prints remaining `todo!()`s with file/line and
  which still-failing tests need them, surfaces the next function to
  translate.
* The progress dashboard (built in Phase 0) is regenerated as a static
  artifact whenever it's useful — no continuous CI integration, no file
  watcher, no `cargo-watch`.

## Tooling stack

* `cmake` — produces `compile_commands.json`.
* `clang -E -P` — preprocesses translation units against the chosen
  target's flags.
* libclang + Python — Phase 0 AST analysis, call graph, dashboard.
* `bindgen` — per-file allowlisted reference output (never shipped).
* Python scripts — driver for Phase 1 module skeletons; `next_todo.py`
  for Phase 4.
* `cargo check` and `cargo test` — inner loop, manually invoked.
* `cargo fmt` and `cargo clippy` — style and lint gates.

Explicitly *not* used in v1: `c2rust`, `bear` / `compiledb` (CMake covers
this), `cargo-watch`, `cargo-llvm-cov`, `cargo-mutants`, `cargo-deny`,
`cargo-fuzz`, `cargo-criterion`, any differential-testing harness.

## Definition of done

The v1 port is complete when:

1. Every C function is translated (no `todo!()` remains in the crate —
   greppable check).
2. Every C test is translated.
3. `cargo build` is clean (no warnings under `clippy -- -D warnings`).
4. `cargo test` is green.

That's it.  No fuzz target, no benchmark threshold, no `unsafe` count
limit, no audit checklist beyond the `// SAFETY:` comments themselves.

## Follow-on work (out of scope for v1)

* Multi-threading: revisit `Send`/`Sync`, audit ownership for
  thread-safety, replace `Rc<RefCell<_>>` with `Arc<Mutex<_>>` where
  needed.
* Multi-platform: cross-compile via `CMAKE_TOOLCHAIN_FILE`, generate a
  per-target inventory, port `#[cfg]`-gated branches.
* Differential testing: build a binary that links both libraries and
  diffs outputs on a shared input corpus.
* Performance: profile hot paths, microbenchmark with `criterion`,
  ensure the Rust port is within X% of C.
* Hardening: fuzzing (`cargo-fuzz`), mutation testing
  (`cargo-mutants`), supply-chain review (`cargo-deny`), sanitizers.

## Open questions / known risks

* **Generated source.**  If the C build invokes lex/yacc/protobuf or
  any other code generator, we need to decide before Phase 0 whether to
  port the generator or check in the generated output and translate it
  as ordinary C.  Phase 0 should surface this when reading the CMake
  build.
* **Allocation-pattern fidelity vs. safety.**  The "mirror C allocation
  patterns" rule and the "minimize `unsafe`" rule can collide on custom
  allocators or arena reuse.  The "safety wins" tie-breaker resolves
  it, but expect a few judgment calls.
* **`#ifdef` blast radius.**  If the C source is heavily conditionalized
  on the target platform, the preprocess-then-parse step may drop more
  code than expected.  The Phase 0 `#ifdef` manifest is the early-
  warning signal — review it before Phase 1 begins.
