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

### AI agent selection

AI-driven scripts invoke a shared runner at `scripts/agent_runner.py`.
Claude is the default provider for compatibility with the original
translation run, but each driver can switch to Codex with
`--agent codex` or `XLATE_AGENT=codex`.  Common model overrides are
`--model`, `XLATE_AGENT_MODEL`, `XLATE_CLAUDE_MODEL`, and
`XLATE_CODEX_MODEL`.

Claude receives the script's `--allowedTools` list directly.  Codex has
no equivalent per-run allowlist flag, so the runner appends the intended
tool scope to the prompt and invokes `codex exec` with
`--sandbox workspace-write --ask-for-approval never` by default
(overridable with `--codex-sandbox` / `--codex-approval`).
Provider transcripts are separated under `xlate/claude_logs/` and
`xlate/codex_logs/`.

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
* Per header: composes a review prompt, invokes the configured agent
  with `Read Edit Glob Grep Bash(cargo check) Bash(cargo clippy)` as
  the intended tool scope — no `Write`, this is refinement only.
* Tracks state in `xlate/phase1a_state.json` (status: `ok` /
  `fail` / `noop`).
* Resumable; continues on failure by default.
* Per-header transcripts at `xlate/<agent>_logs/phase1a/<path>.log`.

## Phase 1B — Cross-module consistency report

1A is per-header by design — one agent invocation sees one module
deeply but isn't well-suited to spot patterns that vary *between* modules
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

`scripts/phase1b.py` is a single-shot inspection — no agent,
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
* For each, invokes the configured agent with the same intended tool
  scope as 1A and a prompt naming the file plus the extracted REVIEW
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

Phase 2 splits into three sub-phases that mirror the 1A/1B/1C
shape: an AI-drafted plan (2A), a human-reviewed plan with
iterative revision (2B), and an AI-driven implementation that
follows the agreed plan (2C).  The plan is the artifact carried
between sub-phases — it lives at `xlate/phase2_plan.md` and is the
single source of truth for what gets abstracted, how, and behind
which Cargo features.

### Top-level rules (apply to all of 2A/2B/2C)

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

### Seed inventory (input to Phase 2A)

The list below is *not* the plan — it's the starting point Phase 2A
re-derives from the current Rust source.  These are the capability
boundaries already visible:

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

## Phase 2A — Draft the abstraction plan

Phase 2A is a single-shot AI pass whose only output is a written
plan at `xlate/phase2_plan.md`.  No source changes, no trait
definitions, no refactoring — just analysis and a plan document
the human can review.

### What the AI does

1. Read `TRANSLATE_PLAN.md` (this document) and `CLAUDE.md`.
2. Walk `rs/fq/src/` and catalog every place the current
   translation touches an external dependency: stub `extern`
   blocks, opaque types lifted from C dependency headers, free
   functions whose only purpose is to call into a dependency,
   Cargo `dependencies` entries.
3. Cross-reference the seed inventory above and confirm,
   refine, or extend each item based on what's actually in the
   Rust source.
4. Write `xlate/phase2_plan.md` covering, for each capability:
   * **Boundary description** — what the capability is, which C
     library currently provides it, which Rust modules consume
     it.
   * **Evidence** — the specific files / lines / symbols that
     pin the boundary.  This is what makes the plan auditable.
   * **Provider trait sketch** — name, location, methods (with
     signatures translated per Phase 1 pointer rules), and
     whether dispatch should be generic (`impl Trait`) or
     dynamic (`dyn Trait`) with a one-line reason.
   * **Callback trait sketch** — same shape, when applicable.
   * **Default implementation plan** — which sibling module
     (`tls_picotls.rs`, `crypto_openssl.rs`, …), which Cargo
     feature gates it, what crates.io / sys-crate dependencies
     it pulls in.
   * **Open questions** — anything the AI couldn't decide from
     the source alone, called out explicitly so the human can
     resolve in 2B.
5. Include a **Cargo feature layout** section at the end of the
   plan: the proposed `[features]` table, plus a note on which
   feature combinations Phase 2C must `cargo check` against
   (default, `--no-default-features --features alloc`, each
   single-backend permutation worth checking).
6. Include an **implementation order** section: the order Phase
   2C should attack capabilities in (typically leaf
   dependencies first — RNG, clock — then crypto primitives,
   then TLS state machine, then I/O), with a one-line rationale
   per capability.

After 2A, `xlate/phase2_plan.md` exists; nothing under `rs/fq/`
has changed.

### Cargo feature layout (template for the plan)

The plan should converge on something like:

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

`scripts/phase2a.py` drives the pass:

* Single-shot; no per-file iteration.
* Composes a prompt that points the AI at `TRANSLATE_PLAN.md`,
  `CLAUDE.md`, the seed inventory, and `rs/fq/src/`, and asks
  for the plan document at `xlate/phase2_plan.md`.
* Invokes the configured agent with `Read Glob Grep Write` as the
  intended tool scope — `Write` is allowed because the plan document
  itself is the output.  `Edit` is not needed.  No cargo build/check
  tools — 2A doesn't change the source, so there's nothing to gate.
* Transcript at `xlate/<agent>_logs/phase2a.log`.

## Phase 2B — Human review of the plan

Phase 2B is *pure human* — no AI, no edits to `rs/fq/`.  The
reviewer reads `xlate/phase2_plan.md`, marks places they
disagree with or want clarified using `// REVIEW: <instruction>`
comments inline in the plan document, and runs `scripts/phase2b.py`
to have the AI revise the plan in place.

This is the same REVIEW-marker mechanic Phase 1C uses, but
applied to a markdown plan instead of Rust source.  The plan is
revised, not the implementation — 2B and 2C stay separate.

### How `// REVIEW` comments work in the plan

Same conventions as 1C:

* Plain `// REVIEW: <instruction>` — the AI revises the
  surrounding plan section to address the request and removes
  the comment.
* `// REVIEW(open): <reason>` — the AI tried and could not
  resolve automatically; the comment stays for further human
  attention.

Markdown doesn't have a native comment syntax, so the
`// REVIEW: ` token is used verbatim as inline text within the
plan.  It stays greppable across the project.

### Per-iteration procedure

1. Human reads `xlate/phase2_plan.md` and adds `// REVIEW: ` markers
   wherever a section needs revision.  Markers can name a
   specific trait, ask "why dynamic dispatch here?", request a
   different feature-gate split, etc.
2. Run `scripts/phase2b.py`.  The AI reads the plan, addresses
   each marker, and rewrites the affected sections.
3. Human re-reads the revised plan.  If more revision is
   needed, add fresh markers and rerun.  Iterate until the plan
   reads cleanly.

### Scripting

`scripts/phase2b.py` drives the iteration:

* Scans `xlate/phase2_plan.md` for `// REVIEW: ` markers (the
  open form is excluded).
* If any markers exist, invokes the configured agent with
  `Read Edit Glob Grep` as the intended tool scope — Edit on the
  plan document only.  No source changes; no cargo gates.
* Each invocation passes the file plus the extracted REVIEW
  comments (line numbers + text) in the prompt.
* State at `xlate/phase2b_state.json` records each iteration's
  resolved-vs-open counts.
* Resumable; re-running with no fresh markers is a no-op.
* Transcript at `xlate/<agent>_logs/phase2b/<iteration>.log`.

### Phase 2B acceptance gate

Phase 2B is complete when the human signs off on the plan and
no plain `// REVIEW: ` markers remain in `xlate/phase2_plan.md`
(only `// REVIEW(open):` entries, each with a recorded reason
the human has chosen to defer).

## Phase 2C — Implement the agreed plan

Phase 2C executes the plan from 2A/2B.  The plan dictates which
capabilities to abstract, which traits to define, which default
implementations to wire, and in what order.  The AI follows the
plan; it does not re-derive it.

### Per-capability procedure

Iterate the capabilities in the order the plan specifies.  For each:

1. **Locate the boundary.**  Use the evidence the plan recorded
   to find the exact files / symbols to refactor.
2. **Define the provider trait** in the module the plan names,
   with the methods the plan sketched (signatures may be
   refined when reality bites — record any divergence in the
   commit message).
3. **Define the callback trait** if the plan calls for one.
   Same module.
4. **Refactor library code** to take the provider through the
   dispatch shape (`impl Trait` / `dyn Trait`) the plan chose.
   Concrete dependency types disappear from the library's
   surface.
5. **Provide the default implementation** in the sibling module
   the plan names, behind the Cargo feature the plan specifies.
6. **Run the gate.**  `cargo check` for every feature
   combination the plan's Cargo-features section enumerates
   (typically: default features, `--no-default-features
   --features alloc`, each single-backend permutation).
   `cargo clippy -- -D warnings` for the default build.

### Plan drift

When implementation reveals that a plan decision was wrong (a
trait method needs a different signature, dynamic dispatch is
required where generic dispatch was planned, an unforeseen
callback edge), the implementation wins, but the plan document
gets updated in the same commit so it stays the authoritative
record.  Don't silently diverge.

### Scripting

`scripts/phase2c.py` drives the per-capability pass:

* Reads `xlate/phase2_plan.md` and iterates the capabilities in
  the order the plan specifies.
* Per capability: composes a prompt that names the capability,
  quotes the relevant plan section, and lists the consuming
  Rust modules.
* Invokes the configured agent with `Read Edit Write Glob Grep
  Bash(cargo check) Bash(cargo clippy)` as the intended tool scope —
  `Write` is allowed because new sibling modules (`tls_picotls.rs`,
  etc.) need to be created.
* State at `xlate/phase2c_state.json`, keyed by capability name
  (`status: ok / fail / skipped`).
* Resumable; continues on failure by default.
* Per-capability transcripts at
  `xlate/<agent>_logs/phase2c/<capability>.log`.

### Phase 2 acceptance gate

Phase 2 is complete when:

1. `xlate/phase2_plan.md` exists, has been human-reviewed
   through 2B, and matches the implementation (no silent drift).
2. No public function or type in `rs/fq/src/` mentions a concrete
   external dependency.  (Greppable check: no `picotls::`,
   `openssl::`, `mbedtls::`, `getrandom::` outside the
   feature-gated default-implementation modules.)
3. Every capability the plan calls out has a provider trait and,
   where applicable, a callback trait, both documented.
4. A default implementation exists for each dependency the C
   library currently uses, behind the matching Cargo feature.
5. `cargo check --no-default-features --features alloc` passes
   (the library compiles with no backends selected — consumers
   wire their own).
6. `cargo check` and `cargo clippy -- -D warnings` pass with
   default features.

## Phase 3 — Translate tests

Phase 3 turns the C `picoquictest/` test suite into Rust `#[test]`
bodies under `rs/fq/src/tests/`.  Bodies are written **against the
Rust API as designed in Phases 1 / 2** — i.e. as if the methods the
test calls already work — even when those methods are still
`todo!()` stubs.  A test that compiles and panics inside the API on
its first call is the expected outcome until Phase 4 fills bodies.

### Per-source layout

* One Rust file per `picoquictest/<src>.c`, at
  `rs/fq/src/tests/<rust>.rs` where `<rust>` is `<src>` with any
  trailing `_test` / `_tests` stripped (e.g. `bytestream_test.c`
  → `tests/bytestream.rs`).  Collisions with existing
  test-infrastructure module names (`util.rs`, `dualq.rs`) keep
  the suffix (`util_test.rs`).
* Per-test-name sanitization: lowercase, non-`[A-Za-z0-9_]`
  collapsed to `_`, leading-digit prefixed with `_`, Rust-keyword
  collisions get `r#`.
* Cross-source helpers live in `rs/fq/src/tests/util.rs` (the
  test-infrastructure module).  Common patterns to port:
  `picoquic_test_set_minimal_cnx*`, `tls_api_init_ctx*`, the
  deterministic test RNG, certificate-fixture path constants, the
  `TestSimLink` / `TestAqm` simulator, etc.
* Test fixtures (binlog references, qlog templates, certificate
  PEMs, etc.) live under `rs/fq/tests/fixtures/`, reached via
  `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/<name>")`.

### Translation rules

* **Faithful, idiomatic.**  Walk the C body and write the
  equivalent Rust.  C `int` 0/-1 → `Result<…, Error>`.  C `int`
  flag → `bool`.  `(uint8_t* buf, size_t len)` → `&[u8]` /
  `&mut [u8]`.  `cnx`/`cid` → `connection`/`connection_id`
  uniformly.  `assert_eq!` / `assert!` over the C `if (…) ret = -1`
  pattern.  Use `?` for fallibility; don't transcribe goto-style ret
  tracking.
* **Use the Rust API as designed, not as currently implemented.**
  Most `Quic` / `Connection` / `Path` methods are still `todo!()`;
  call them anyway.  The first `todo!()` panic is the test's
  fail signal.  This is **test-driven development**: the tests
  document what Phase 4 must satisfy.
* **API gaps are filled by `todo!()` stubs in non-test source.**
  When a test body needs a method that doesn't yet exist on a
  public type, add it to the appropriate source file
  (`rs/fq/src/internal.rs` / `lib.rs`) as a fresh
  `pub fn name(...) { todo!() }` stub with a `/// C: `picoquic_xxx``
  doc comment.  This is the only authorized non-test edit during
  Phase 3.  Never rename / reshape existing items.
* **Platform-specific or UB-dependent C tests** (signed overflow,
  `memcmp` on padded structs, specific `errno` values) are flagged
  for rewriting.

### Pipeline (Phase 3A)

The bulk of the work is automated by a per-batch agent-driven
pipeline (`scripts/phase3a.py`).  Stages:

1. **Stub generation** (`scripts/phase3.py`): parse the
   `picoquic_t/picoquic_t.c` `test_table[]`, group entries by C
   source file, emit one `#[test] fn <name>() { todo!("<entry_fn>") }`
   per row to `rs/fq/src/tests/<rust>.rs`, register the modules in
   `rs/fq/src/tests/mod.rs`.  Run once.
2. **Translation guide** (`xlate/test_translation_guide.md`):
   handwritten reference summarising the Rust API surface, naming
   conventions, helper inventory, and translation patterns.  Each
   agent reads this once instead of re-grepping `lib.rs` /
   `internal.rs` from scratch.
3. **Body translation** (`scripts/phase3a.py`):
     * Per-source mode (`--batch 1`): one agent invocation
       per `picoquictest/<src>.c`.  Higher fidelity, but each
       invocation pays the full guide / `util.rs` re-read cost.
     * Batched mode (`--batch N`, default 1): one invocation per
       group of N sources.  Amortises guide / `util.rs` reads
       across the batch and lets the agent re-use helpers it
       added earlier in the same session.
   The script invokes the configured agent with streaming output where
   the provider supports it, surfaces each tool call to stdout, runs the
   build gate (`cargo fmt` +
   `cargo test --no-run` + `cargo clippy --tests --all-features --
   -D warnings`), and records ok/fail per source in
   `xlate/phase3a_state.json`.
4. **Completion check** (`scripts/phase3_check.py`): for every
   `#[test] fn` listed in `test_table[]`, verifies the Rust body
   is anything other than the auto-stub `todo!("<entry_fn>")`.
   Both the agent and the parent script call this; the parent
   re-runs sources whose checks fail.
5. **Rate-limit handling**: HTTP 429 from the provider API surfaces
   as a distinct exit code; the sweep aborts cleanly
   without polluting the state file.  Re-run after the quota
   resets.

State, logs, and artifacts:
* `xlate/phase3a_state.json` — per-source ok/fail.
* `xlate/phase3a_runs/<timestamp>.log` — run-level stdout.
* `xlate/<agent>_logs/phase3a/<src>.log` — per-source agent
  transcript.
* `xlate/prompts/phase3a/<src>.md` — composed prompts.

### Phase 3 acceptance gate

* `cargo test --no-run` compiles cleanly.
* `cargo test` runs to completion.  Every test that hasn't been
  validated against a real implementation panics on a `todo!()`
  (or matching panic message).  No segfault, no abort, no compile
  error.
* `cargo fmt --check` clean.
* `cargo clippy --tests --all-features -- -D warnings` clean.
* `python3 scripts/phase3_check.py` reports no remaining stubs.

## Phase 4 — Translate implementations

After Phase 3 lands, every public test is expressed as Rust against the
designed API surface, and the body of every API method is `todo!()`.
Phase 4 fills those bodies by translating the matching C body from
`picoquic/<src>.c`.  The Phase 3 test suite is the gate: a function is
"done" when the tests that exercise it stop panicking on `todo!()`.

### Order (default: file size; opt-in: call-graph height)

Targets are the `.rs` files under `rs/fq/src/` that still contain
`todo!()` bodies (test files, sys/loglib stubs, and out-of-v1-scope
modules are skipped — see `SKIP_FILES` in `scripts/phase4.py`).

Default ordering is by file size (smallest first) — a crude but
effective leaf-first proxy.  Leaf modules (`siphash`, `splay`, `hash`,
`bytestream`) are short; `Connection` / `Quic` core in `internal.rs` is
huge.

Opt-in ordering with `--order callgraph` reads `xlate/call_graph.json`
(`height_of`) and sorts by the *minimum* C call-graph height of any
function whose name appears in `/// C: \`name\`` doc comments inside the
file.  Files with no matched names fall through to size order.  Use
this when the size proxy gets fooled by a big file full of
independent leaves (e.g. `utils.rs`).

### Pipeline (`scripts/phase4.py`)

For each Rust source file:

1. **Compose a prompt** that:
   * names the Rust target and its `todo!()` count;
   * points at `xlate/impl_translation_guide.md` for C → Rust idiom
     mappings (memory, control flow, errors, strings, time, network,
     logging, crypto);
   * names the matching C source(s) — the doc comments in the Rust
     file already encode this as `/// C: \`picoquic_xxx\``;
   * spells out the translation contract (faithful, idiomatic, no
     signature changes, no `unsafe`, no edits outside `rs/fq/src/`).
2. **Invoke the configured agent** with a tight intended scope
   (Read / Edit / Glob / Grep / Bash for `cargo` and the Phase 4
   completion check).
3. **Verify** with `scripts/phase4_check.py <rs file>` (zero
   `todo!()`s remain in the named files).
4. **Build gate** — `cargo fmt` + `cargo test --no-run` + `cargo
   clippy --tests --all-features -- -D warnings`.  Failure penalises
   the run.

A `--batch N` mode packs N source files per agent invocation,
amortising the guide read and any cross-module API lookups.  The
build gate runs once at the end of each batch.

State / log artifacts:

* `xlate/phase4_state.json` — per-file `{status, at, …}`.  `ok` /
  `partial` / `fail`; `partial` records `remaining` count.
* `xlate/phase4_runs/<timestamp>.log` — per-sweep tee of stdout.
* `xlate/<agent>_logs/phase4/<basename>.log` — per-run agent
  transcript.
* `xlate/prompts/phase4/<basename>.md` — the composed prompt
  (regenerated on each run).

Re-running with no flags resumes from `phase4_state.json` (already-`ok`
files are skipped).  `--force` redoes everything; `--src <file>`
targets one file; `--limit N` caps the sweep.  HTTP 429 is detected
from structured provider events where available and aborts cleanly
without polluting state.

### Signature drift

When translating function `f`, the translator may discover that the
Phase 1 signature for some callee `g` is wrong.  Surface it on
stdout and stop — that's a human design call, not Phase 4 work.
Phase 1 / 2 / 3 settled the surface; Phase 4 only fills bodies.

### Discovered call edges

Static analysis under-approximates call edges through function
pointers and trait objects.  When `cargo test` reveals a `todo!()`
panic in a function the call graph thought was a leaf, the
translator implements the required trait method and moves on — no
need to regenerate the call graph.

### Acceptance gate

* Every `todo!()` body in non-skipped Rust modules has been replaced
  with a translation.  Verified by
  `python3 scripts/phase4_check.py` (exit 0).
* `cargo test --no-run` succeeds.
* `cargo clippy --tests --all-features -- -D warnings` succeeds.
* `cargo test` runs, panics only in not-yet-replaced `todo!()`s
  (zero by acceptance) — a green or near-green run is the goal.

The remaining red tests (if any) are real failures, not stub panics
— they get a separate triage pass.

## Phase 4A — Function map and implementation plan

Phase 4A is a planning pass.  It builds the first complete map from
in-scope C functions to their translated Rust counterparts, then
decides which unmapped C functions require Rust implementations and
where those implementations should live.  It does not implement missing
functions.  Its output is a concrete implementation plan for human
approval before Phase 4B starts.

### Function map

`scripts/phase4a.py` writes `xlate/function_translation_map.json`.
The map records, for every in-scope C function:

* C function name, source file, and start/end line numbers.
* Rust counterpart name, source file, and start/end line numbers, when
  one is found.
* Mapping evidence: `/// C: ...` comments, Phase 4 state, inventory
  metadata, direct name correspondence, or a manual override.
* Current implementation status.
* Required-action classification.

The first map is built from Phase 0 inventory data, Phase 4 state,
Rust `/// C: ...` references, direct source searches, and any manual
overrides needed to resolve naming or module-shape differences.

### Required-action classification

Every C function without a Rust counterpart is reviewed and classified:

* `implemented` — a completed Rust counterpart exists.
* `expected_omission` — no direct Rust function is needed.  Examples:
  allocation/free helpers absorbed by ownership, `Drop`, `Vec`, or
  `Box`; thin C wrappers folded into methods; helper functions made
  unnecessary by a safer Rust shape.
* `required_missing` — observable behavior is missing and needs a Rust
  implementation.
* `blocked` — a concrete design question or dependency prevents a
  faithful classification.

### Implementation plan

For each `required_missing` item, Phase 4A proposes:

* Rust destination module and item shape: free function, method, trait
  implementation, static descriptor, table entry, or test helper.
* Any module/export changes needed to make the item reachable.
* Any ordering constraints with other missing functions.
* The expected verification target: specific tests when known, or the
  standard Phase 4 cargo gates when no narrower test is available.

If the proposed module structure would differ from the current Rust
layout, the plan explains why the new shape is needed.  If a missing
function might be an `expected_omission` but the evidence is weak, it
stays `blocked` rather than being silently dropped.

### Approval artifact

Phase 4A writes:

* `xlate/function_translation_map.json` — the current map.
* `xlate/phase4a_plan.md` — the proposed implementation plan.
* `xlate/phase4a_plan.html` — the same plan in a browsable form.

Phase 4B starts only after the plan is approved.  Approval can be
recorded by updating the plan status in `xlate/phase4a_plan.md` or by
committing the reviewed plan with a note that it is approved.

### Acceptance gate

* Every in-scope C function appears in
  `xlate/function_translation_map.json`.
* Every unmapped C function is classified as `expected_omission`,
  `required_missing`, or `blocked`.
* Every `required_missing` item has a proposed Rust destination and
  verification target.
* The Phase 4A plan has been presented for human approval.

## Phase 4B — Implement missing required functions

Phase 4B implements the Rust functions and related items approved in
Phase 4A.  It uses `xlate/function_translation_map.json` as the
authoritative worklist and updates that map as each item moves from
`required_missing` to `implemented`.

This is not a relaxation of Phase 4.  The same translation standards
apply: faithful and idiomatic Rust, safe ownership, no placeholder
implementations, no stubs, no silent signature changes, and no edits
outside the allowed Rust/module support files unless the plan is updated
first.

### Procedure

For each approved `required_missing` item:

1. Read the C source, directly related headers, the approved Phase 4A
   destination, and nearby Rust code.
2. Implement the missing Rust item in the approved module structure.
   This may include descriptors, registration tables, aliases, module
   exports, or tests when those are part of making the function
   reachable.
3. Update `xlate/function_translation_map.json` with the Rust file,
   line span, implementation status, and any changed mapping evidence.
4. Run the relevant narrow test when one is known, then the Phase 4
   gates: `cargo fmt`, `cargo test --no-run`, and
   `cargo clippy --tests --all-features -- -D warnings`.

If implementation reveals that the Phase 4A module structure or item
shape is wrong, update the map entry to `blocked` with the reason and
return the proposed plan amendment for approval before continuing that
item.

### Acceptance gate

* `xlate/function_translation_map.json` has no approved
  `required_missing` entries.
* Every required C function is either `implemented` or `blocked` with a
  concrete human-actionable reason.
* Every implemented item has a Rust file and line span recorded in the
  map.
* The Phase 4 build and lint gates still pass.
* `cargo test` failures, if any remain, are runtime behavior failures
  rather than missing-translation failures.

## Phase 4C — Function correspondence audit

Phase 4C is a lightweight audit of the final C/Rust function map.  For
each mapped C/Rust pair, it asks whether the Rust function appears, from
the bodies alone, to be a reasonable translation of the C function.  The
goal is to catch obvious inconsistencies cheaply before deeper
debugging.

This phase does not prove semantic equivalence.  It deliberately avoids
full-program context and treats the result as triage data: `ok`,
`suspect`, or `definitely_not_ok`.

### Body-only comparison

For each mapped C/Rust pair, the driver asks an agent for a superficial
translation review using only:

* the C function body;
* the Rust function body;
* the two function names and source spans.

The prompt must not include dependencies, type definitions, callers,
callee definitions, module context, or build errors.  The reviewer is
only looking for obvious inconsistencies visible from the bodies:
missing branches, inverted conditions, dropped state updates, mismatched
constants, omitted calls, loop-shape changes, suspicious error handling,
or placeholder-like code.  Idiomatic Rust differences are fine when the
body still appears to preserve the C control flow and effects.

Each review returns:

* `ok` — no obvious body-level concern.
* `suspect` — possible mismatch; needs human or deeper agent review.
* `definitely_not_ok` — clear body-level mismatch or placeholder.
* A short rationale, limited to body-visible evidence.

This should be fast and low-cost.  Batch multiple small functions per
agent call when possible, and do not ask the reviewer to inspect or
modify source files.

### Report

`scripts/phase4c.py` writes:

* `xlate/phase4c_reviews.json` — body-only comparison results.
* `xlate/phase4c_report.html` — human-readable report.

The HTML report includes:

* A summary table for mapped function pairs: counts and percentages for
  `ok`, `suspect`, and `definitely_not_ok`.
* A list of C functions that still have no Rust equivalent, by name,
  with their Phase 4A/4B classification.
* For every `suspect` or `definitely_not_ok` pair, a side-by-side view
  of the C and Rust function bodies with file/line spans and the short
  body-only rationale.

### Acceptance gate

* Every mapped pair in `xlate/function_translation_map.json` has a
  Phase 4C review result.
* `xlate/phase4c_report.html` exists and includes the comparison
  summary, remaining no-equivalent list, and side-by-side views for all
  non-`ok` pairs.
* Any `definitely_not_ok` item is routed back to Phase 4B or marked
  `blocked` with a concrete human-actionable reason.

## Phase 4D — Deep review and repair

Phase 4D resolves the Phase 4C triage output.  It consumes every
`suspect` and `definitely_not_ok` review result and performs a deeper
context-aware analysis.  Unlike Phase 4C, this phase may inspect
directly relevant definitions, constants, helper callees, callers,
tests, and surrounding module context.  It may also edit Rust source
when the deeper analysis confirms a real translation mismatch.

Phase 4D does not re-review Phase 4C `ok` entries unless a later fix
changes the relevant Rust function enough to invalidate the old review.

### Deep analysis

For each Phase 4C non-`ok` entry:

1. Reconstruct the C function's intended observable behavior from the
   C body plus directly relevant C context.
2. Compare the mapped Rust function in its module context, including
   translated helper functions, local types, and tests when useful.
3. Classify the Phase 4C finding as either:
   * `ok` — the body-only concern was a false positive after deeper
     inspection; no source edit is needed.
   * `fixed` — the concern was real and the Rust implementation has
     been corrected.
   * `blocked` — a real mismatch remains, but the fix requires a
     concrete design decision or dependency outside the current batch.
     The blocker must be specific and human-actionable.

Every confirmed mismatch should be fixed in this phase whenever
possible.  `blocked` is reserved for concrete cases where a correct fix
cannot be made safely without changing an approved design boundary.

### Repair procedure

For each entry that is actually not OK:

1. Edit the Rust implementation, preserving safe/idiomatic Rust and the
   existing API shape unless that shape cannot express the C behavior.
2. Keep or add `/// C: ...` mapping references where needed so the
   function map can refresh correctly.
3. Run the relevant narrow test when one is obvious.
4. Run the Phase 4 gates after any Rust edit:
   `cargo fmt`, `cargo test --no-run`, and
   `cargo clippy --tests --all-features -- -D warnings`.
5. Refresh `xlate/function_translation_map.json` if Rust line spans
   changed.

### Artifacts

`scripts/phase4d.py` writes:

* `xlate/phase4d_results.json` — deep-review outcomes for Phase 4C
  non-`ok` entries.
* `xlate/phase4d_report.html` — human-readable summary of confirmed OK
  entries, fixed entries, blocked entries, and remaining pending work.

### Acceptance gate

* Every Phase 4C `suspect` and `definitely_not_ok` entry has a Phase 4D
  result.
* Every result is either `ok`, `fixed`, or `blocked` with a concrete
  reason.
* Every `fixed` entry passes the Phase 4 build and lint gates after the
  relevant Rust edits.
* `xlate/function_translation_map.json` is refreshed after any fix that
  changes Rust function spans.

## Phase 4E — Repair confirmed mismatches

Phase 4E is the repair pass for Phase 4D entries classified as
`needs_fix`.  Phase 4D may be run widely in parallel in
classification-only mode; Phase 4E then performs the actual Rust edits
for the smaller set of confirmed mismatches.

Phase 4E uses file-owned work buckets.  All `needs_fix` entries mapped
to the same Rust file are assigned to the same bucket, so multiple
repair agents can run in parallel without editing the same primary file.
If a repair requires touching a helper in another Rust file, the agent
must keep the change directly related and report the cross-file edit in
the repair result.

### Procedure

For each `needs_fix` entry:

1. Read the Phase 4C rationale and Phase 4D deeper analysis.
2. Read the C and Rust implementation context needed for a faithful
   repair.
3. Fix the Rust implementation, or mark the item:
   * `fixed` — Rust was corrected.
   * `ok` — Phase 4D's `needs_fix` classification was too conservative
     after repair-level inspection.
   * `blocked` — a real mismatch remains but needs a concrete external
     design decision or dependency.
4. Run the relevant narrow test when one is obvious.
5. Run the Phase 4 gates after any Rust edit:
   `cargo fmt`, `cargo test --no-run`, and
   `cargo clippy --tests --all-features -- -D warnings`.
6. Refresh `xlate/function_translation_map.json` if Rust line spans
   changed.

### Artifacts

`scripts/phase4e.py` writes:

* `xlate/phase4e_repairs.json` — repair outcomes for Phase 4D
  `needs_fix` entries.
* `xlate/phase4e_report.html` — human-readable repair summary.

It also updates `xlate/phase4d_results.json` so repaired entries move
from `needs_fix` to `fixed`, `ok`, or `blocked`.

### Acceptance gate

* No Phase 4D entries remain in `needs_fix`.
* Every confirmed mismatch is either fixed or blocked with a concrete
  human-actionable reason.
* The Phase 4 build and lint gates pass after every Rust repair batch.

## Tooling stack

* `cmake` — produces `compile_commands.json`.
* `clang -E -P` — preprocesses translation units against the chosen
  target's flags.
* libclang + Python — Phase 0 AST analysis, call graph, dashboard.
* `bindgen` — per-file allowlisted reference output (never shipped).
* Python scripts — driver for Phase 1 module skeletons; `phase3a.py`,
  `phase4.py`, and the Phase 4A/4B/4C/4D/4E map, completion, audit,
  classification, and repair drivers.
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
