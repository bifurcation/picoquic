# Translating a C project to Rust

This is a general methodology for translating a C library or binary to Rust.
The Rust binary should provide the same functionality as the C code (including
bug-for-bug compatibility), and its structure and performance should be as
similar as possible without sacrificing safety.

The overall approach is to move from the "outside in".  Start by translating any
interfaces that are exposed either publicly or to tests.  Then translate the
implementation of the interface methods, the methods that these methods call,
and so on down the call tree.

> **Review note (overall):**
> The "outside-in" framing is sound for *interface shape*, but for the
> *implementation* it inverts the natural data flow.  In Phase 3 the plan
> already pivots to a bottom-up order (lowest-height first), which is the
> right call — leaf functions have no `todo!()` dependencies, can be unit-
> tested in isolation, and don't churn when callers are rewritten.  Worth
> stating explicitly up front that "outside-in" governs *API design* while
> "bottom-up" governs *implementation order*.
>
> RLB: This makes sense.  Feel free to amend the above to reflect that
> combination.
>
> CLAUDE: Will do — I'll fold "outside-in for API, bottom-up for
> implementation" into the intro on the next edit pass.
>
> Major risks not addressed by the plan as written:
>   - **Shared mutable state.**  Idiomatic C uses pointers into a global or
>     parent struct; idiomatic Rust uses ownership trees + borrows.  A purely
>     mechanical port will fight the borrow checker constantly.  Plan needs
>     an escape hatch (e.g., "fall back to `Rc<RefCell<…>>` or arena indices
>     when ownership is genuinely a graph") and a policy for when to refactor
>     vs. when to preserve C structure.
>
>    RLB: Yeah, we can have Rc<RefCell<_>> as an escape hatch.  Feel free to
>    indicate that in the appropriate places.
>
>    CLAUDE: Acknowledged.  I'll add it to the Phase 1 pointer-mapping rules
>    and to the cross-cutting policy section as the last-resort option.
>
>   - **Bug-for-bug compatibility vs. safety.**  These goals conflict at the
>     edges: integer overflow, signed/unsigned conversion, UB on aliasing,
>     uninitialized reads.  Pick a tie-breaker rule.
>
>     RLB: The tie-breaker is safety.  The safe option always wins.
>
>     CLAUDE: Got it.  Practical implication: when the C code relies on
>     wrap-on-overflow, signed-shift-as-arithmetic, or memcpy of overlapping
>     buffers, we use the safe Rust idiom (`wrapping_add` only if the
>     wraparound is intended; `Result` on out-of-range cast; `copy_from_slice`
>     with non-overlapping slices) even if outputs diverge for pathological
>     inputs.  I'll capture that as an explicit rule.
>
>   - **Differential testing.**  The strongest correctness signal for a port
>     is running C and Rust side-by-side on the same inputs and diffing
>     outputs.  The plan should reserve a phase for this; unit tests alone
>     will miss endianness, padding, and FP-rounding divergences.
>
>     RLB: A separate phase isn't needed.  The translated tests suffice.
>
>     CLAUDE: Fine.  Tradeoff worth noting: ported tests inherit the C tests'
>     blind spots — anything the C suite never exercised (e.g., a specific
>     overflow path) we won't catch either.  If a downstream user reports
>     "tests pass but bytes differ from the C version," that's the signal to
>     add differential testing as a follow-on phase.
>
>   - **Cutover strategy.**  Nothing in the plan describes how the Rust port
>     replaces the C build, nor whether intermediate states ship.  Decide
>     early: hard fork, or incremental FFI shim where Rust modules are
>     callable from the C build (and vice versa) until parity.
>
>     RLB: There is no cut-over strategy, this is a hard fork.
>
>     CLAUDE: Clean.  That eliminates a lot of complexity (no FFI shim, no
>     mixed builds, no ABI-stability requirement on the Rust side).

## Phase 0: Inventory

* Analyze the build system to extract information needed by later phases

> **Review note (Phase 0):**
> *Feasibility:* High, but this phase is dramatically under-specified given
> how much of the plan depends on it.  Concretely, Phase 0 should produce:
>   - the full source file list (with public vs. internal classification),
>   - per-file compile flags and `-D` defines (these change the AST),
>   - the include graph,
>   - the link graph (which symbols are public ABI vs. internal),
>   - the test inventory (binaries, fixtures, expected outputs),
>   - the third-party dependency list with version pins,
>   - platform conditionals (`#ifdef _WIN32`, etc.) — decide which targets
>     the Rust port supports on day one.
>
> *Pitfalls:*
>   - Build systems often have multiple entry points (CMake + Makefile +
>     IDE projects); pick one as canonical and ignore the others, or you
>     will translate dead code paths.
>
>   RLB: I'm willing to assume that this is a CMake project and focus on the
>   CMake build.
>
>   CLAUDE: Good simplification — CMake is the canonical build, everything
>   else (Makefile, IDE projects, appveyor.yml, etc.) is ignored.
>
>   - Generated source (lex/yacc, protobuf, codegen scripts) is easy to
>     miss and breaks downstream phases.
>   - Conditional compilation: `#ifdef` branches that are dead on your
>     target platform should be pruned before parsing, or stubs will
>     proliferate.
>
>   RLB: I am worried about this.  Any thoughts on how to address?
>
>   CLAUDE: A few options, in increasing rigor:
>     1. **Run the preprocessor first.** CMake's `compile_commands.json`
>        already pins the right `-D` flags for the chosen target.  Pipe each
>        translation unit through `clang -E -P` (or `unifdef -D… -U…`) to
>        materialize one fully-expanded source per file with all `#ifdef`s
>        resolved.  libclang then parses an unambiguous AST.  This is the
>        default I'd reach for.
>     2. **Pin a single target triple.** Pick one (e.g.,
>        `x86_64-unknown-linux-gnu`) as the source of truth.  Code that's
>        purely `#ifdef _WIN32` simply isn't translated in v1; other
>        platforms get added back later behind Rust `#[cfg(target_os = …)]`.
>     3. **Inventory pruning.** Before parsing, run a script that lists
>        every `#ifdef` symbol used in the codebase and classify each as
>        always-on, always-off, or per-target.  Produces a written manifest
>        for review.
>   I'd combine (1) and (2): preprocess for the chosen target, accept that
>   we lose the other-platform branches, port them in a follow-on if needed.
>
> *Scripting:*
>   - Drive a *real* build with `bear` (or `compiledb`) to capture
>     `compile_commands.json` — this is the ground truth for files,
>     flags, and defines.  Don't reverse-engineer CMake by hand.
>
>   RLB: I'm willing to assume that the C project has a CMake build and focus on
>   that.  So we would use `cmake` to get the `compile_commands.json`.  That
>   should be useful.  My only concern is that that only addresses the host
>   platform.  How can we address other platforms?  Maybe with
>   cross-compilation?
>
>   CLAUDE: Yes, cross-compilation is the standard answer.  `cmake
>   -DCMAKE_TOOLCHAIN_FILE=…` per target produces one `compile_commands.json`
>   per platform; you get an inventory per target and can union the symbol
>   set or diff between them.  But I'd defer this.  Pick one target now, get
>   the single-target port working, then add others as a follow-on.
>   Multi-platform parsing roughly doubles the inventory complexity for a
>   benefit you only collect at the end of v1.  Concretely: target
>   `x86_64-unknown-linux-gnu` first; everything else gets a v2 task.
>
>   - Use `clang -MM` (or `clang-scan-deps`) for the include graph and
>     `nm`/`objdump` on built `.o` files for the symbol/link graph.
>   - Parse with libclang (Python bindings) to dump function signatures,
>     struct layouts, and call edges to JSON.  Every later phase consumes
>     this JSON.
>   - Treat the Phase 0 output as a versioned artifact (`inventory.json`)
>     checked into the repo so re-runs are diffable.
>
>   RLB: I would add to this list making a script we can use to visualize
>   progress along the dependency graph, possibly at the level of files.
>
>   CLAUDE: Yes — build it as part of Phase 0's deliverable.  Inputs: the
>   libclang call/include graph (already produced) plus a scan of the Rust
>   crate for `todo!()` markers.  Output: a single static HTML page
>   (Graphviz SVG, or interactive via pyvis/d3) with nodes colored
>   C-only / stubbed / translated / translated-and-tested, plus a per-file
>   rollup percentage.  Default to file-granularity rendering with
>   click-to-expand to function granularity — the full function graph is
>   unreadable as one image.  Per-test sub-graphs (the call tree below one
>   test) are also useful since that's the unit Phase 3 operates on.

## Phase 1: Create a stub implementation

For each header:
* Translate to safe, idiomatic Rust module.  Add an empty `mod test` at the end.
* todo!() implementation
* Primitive, enum, struct values should translate directly
* For pointer fields
    * read C functions that reference the field to understand life cycle
    * Map to appropriate Rust semantic — slice, vector, linked list, borrow, str, etc.  
* For pointer args
    * Read the function
    * Map to appropriate Rust semantic — ref, mut ref, out param, etc. 
* Do not translate anything external.  These should be stdlib or crates, or worst case sys 
* Function pointers should be mapped to traits
* Function pointers should be mapped to traits

After translating each file, format the crate and verify that crate builds and
passes `cargo clippy`.  If there are any errors or warnings, fix them.

> **Review note (Phase 1):**
> *Feasibility:* This is the highest-judgment phase and the *least*
> automatable part of the plan.  Expect it to dominate the schedule.
> Mapping a C pointer to the right Rust shape (`&T`, `&mut T`, `Box<T>`,
> `Rc<RefCell<T>>`, `Vec<T>`, `&[T]`, arena index, raw pointer) requires
> reading every caller and understanding lifetimes — that is exactly the
> kind of cross-cutting reasoning that resists scripting.
>
> *Pitfalls:*
>   - **Stub signatures lock in API decisions.**  A wrong choice here
>     (e.g., `&T` where the real semantic is shared mutation) cascades
>     into Phase 3 rewrites.  Treat early signatures as drafts and
>     budget revisits.
>   - **Dependency cycles between modules.**  C headers freely include
>     each other; Rust modules form a DAG.  Mutual recursion across
>     module boundaries forces either merging modules or hoisting types
>     to a common parent.  Decide the policy before you start.
>
>   RLB: This is not a problem.  Modules can cross-depend on each other. For
>   example, the following code builds just fine.
> 
>   ```
>   mod a {
>       pub fn x() { super::b::y(); }
>       pub fn z() {}
>   }
>   
>   mod b {
>       pub fn y() { super::a::z(); }
>   }
>   ```
>
>   CLAUDE: You're right at the function level — `super::` paths resolve
>   fine and `rustc` happily handles mutual function references across
>   modules.  The remaining wrinkle is *type* recursion: `struct A { b: B }`
>   and `struct B { a: A }` won't compile without an indirection (`Box<B>`,
>   `&'a B`, or hoisting both into a shared parent module).  Worth a
>   one-liner in the policy: types that mutually reference get an indirection
>   (`Box`/`Rc`) or are co-located.  Function-level cycles are a non-issue.
>
>   - **Macros and X-macros.**  Constants behind `#define`, function-like
>     macros, and table-generation X-macros need a per-case decision:
>     `const`, `fn`, `macro_rules!`, or `build.rs` codegen.
>
>   RLB: Agreed.  The tool can use judgement here.
>
>   CLAUDE: Acknowledged.
>
>   - **Bitfields, packed structs, unions, flexible array members.**
>     These do not have direct safe-Rust equivalents.  Either accept
>     `repr(C)` + unsafe accessors, or translate to a different shape
>     and accept layout divergence.  Pick a rule.
>
>   RLB: I don't think this is quite accurate.  Maintaining memory
>   representation isn't required, just semantic equivalence.  Bitfields and
>   packed structs can be semantically represented in safe Rust.  Unions should
>   likewise map to Rust enums.  Flexible array members should map to boxed
>   slices.
>
>   CLAUDE: You're right, I overstated the constraint.  Concrete mappings:
>   bitfields → ordinary integer fields with mask/shift accessor methods (or
>   `bitflags!` for flag-style bitsets); packed structs → normal structs
>   plus serialization helpers for any wire-format use; unions → `enum`;
>   flexible array members → `Box<[T]>` or `Vec<T>`.  The only exception is
>   structs that cross an FFI/syscall boundary where layout is part of the
>   contract — those keep `#[repr(C)]`.  I'll fold this into the Phase 1
>   rules.
>
>   - **`todo!()` everywhere makes the crate technically compile but
>     hides type errors that only surface when bodies are filled in.**
>     Force at least the *signatures* to typecheck against each other
>     by writing a couple of internal call sites that won't be `todo!()`.
>
>   RLB: We will surface these mismatches in Phase 3.  This is a first guess.
>
>   CLAUDE: Acknowledged — first-guess signatures, refine in Phase 3 when
>   real call sites prove them wrong.
>
>   - "Safe, idiomatic Rust" and "bug-for-bug compatible" are in tension
>     for anything dealing with integer wraparound, undefined-on-overflow,
>     or pointer provenance.  State the tie-breaker.
>
>   RLB: Safety always wins.
>
>   CLAUDE: Same rule as the overall section — noted.
>
> *Scripting:*
>
>   RLB: Important note, this phase should explicitly operate **file by file**.
>   Each header file should be processed independently, with other headers and C
>   files available only for context in understanding the header.
>
>   CLAUDE: Important constraint — thanks for being explicit.  This bounds
>   the per-file work and makes the scripting straightforward (a per-header
>   driver script).  It also bounds the context window for any AI-assisted
>   translation.  I'll word the Phase 1 instructions around: "process header
>   H given (H itself, related .c files, headers H depends on) — nothing
>   else.  Output one Rust module per header."
>
>   - Use `bindgen` to generate a *baseline* FFI-shaped Rust translation
>     of every header.  Don't ship it — diff against it.  It gives you
>     authoritative struct layouts, enum values, and function signatures
>     to compare your idiomatic translation against.
>
>   RLB: I agree that `bindgen` is a good first pass.  But it should be run with
>   a strict allowlist so that it only generates bindings for the things
>   **defined in the file**.  And the instructions should be clear that the
>   bindgen output is a starting point -- all the `repr(C)` parts can be
>   abandoned, the structs should be refactored to be more safe and idiomatic,
>   the methods should become struct or trait methods as appropriate, etc.
>
>   CLAUDE: Right — `--allowlist-file '.*<filename>\.h$'` (and/or
>   `--allowlist-function`/`--allowlist-type` derived from the header's own
>   declarations) keeps bindgen from pulling in transitive `<stdio.h>` etc.
>   The driver script reads each header, derives the allowlist regex from
>   the header path, runs bindgen, and stores the output as
>   `bindgen_baseline/<file>.rs` for diff reference only.  I'll make
>   explicit that the bindgen output is reference scaffolding — never
>   committed as the production module — and that `repr(C)` is dropped,
>   structs refactored to safe shapes, free functions promoted to inherent
>   or trait methods.
>
>   - Build a small Python tool over the libclang AST that, for each
>     pointer parameter, prints: callers, whether they pass `NULL`,
>     whether the callee writes through it, whether ownership transfers
>     (look for `free` on the same pointer downstream).  This is the
>     evidence you need to choose the Rust shape — automate gathering
>     it, not the choice itself.
>   - `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo check` in a
>     pre-commit hook per translated file.  Cheap, catches drift.
>   - Generate the empty `mod test { }` block and module skeleton with a
>     script driven by the Phase 0 inventory; humans should never type
>     boilerplate.
>   - For function-pointer-to-trait conversions, scripting can list
>     every function-pointer typedef and its call sites, but the trait
>     boundary (one trait per typedef? grouped by struct?) is a judgment
>     call.

## Phase 2: Translate tests

For each test in the C project:

* Identify the methods that it calls in the library
* Identify an appropriate test module to put the test in 
    * If all of the tested methods are in one module, add it to the `mod test`
      in that file
    * Otherwise, add it to a top-level `tests.rs`
* Translate the test to the stub API generated in Phase 1

After verifying each file, format the crate and verify that the crate builds and
pass `cargo clippy`, **including the tests**.  If there are any errors or
warnings, fix them.  Verify that the tests build and cleanly fail (e.g., no
segfaults).

> **Review note (Phase 2):**
> *Feasibility:* Medium.  Translating tests early is the right instinct —
> tests pin the API and provide the acceptance gate for Phase 3.  But
> "translate the test" is itself a non-trivial port: tests touch test
> harnesses, fixtures, file paths, timing assumptions, and helper macros
> that aren't part of the library.
>
> *Pitfalls:*
>   - **Test harness translation is a project of its own.**  C tests
>     often use an ad-hoc framework (custom `assert` macros, a `main()`
>     that runs a table of cases).  Decide whether to port the harness
>     or use `#[test]` + `cargo test`.  The latter is cleaner but
>     reorganizes how failures are reported.
>
>   RLB: We should *NOT* translate the test harness, just the semantics.  We
>   should use the cargo-native test framework.
>
>   CLAUDE: Agreed — `#[test] fn …` blocks plus `assert_eq!` / `assert!`.
>   Any custom C macros (`TEST_ASSERT_EQUAL_INT`, etc.) translate to the
>   matching cargo-native call.  The C `main()` that runs a table of cases
>   is dropped entirely; cargo discovers tests automatically.
>
>   - **Fixture data and golden files** must be reachable at the new
>     paths Rust tests expect (`CARGO_MANIFEST_DIR`).  Plan for a
>     fixtures directory and a path-resolution helper.
>
>   RLB: Agreed that we should do that.
>
>   CLAUDE: Will do — `tests/fixtures/...` (or `src/fixtures/...` if we
>   want them included with the crate), with a small helper
>   `fn fixture_path(rel: &str) -> PathBuf` anchored to
>   `env!("CARGO_MANIFEST_DIR")`.
>
>   - **Cleanly fail** is under-defined.  `todo!()` panics, which is
>     "clean" in the no-segfault sense but pollutes test output and
>     can mask real compile errors.  Consider `unimplemented!()` or
>     `#[ignore]` with a tracking attribute so you can distinguish
>     "not yet implemented" from "broken."
>   - **Tests that depend on undefined or platform-specific C behavior**
>     (signed overflow, struct padding visible via `memcmp`, specific
>     `errno` values) will need rewriting, not translation.  Flag and
>     quarantine these.
>   - **Integration tests vs. unit tests.**  The plan's rule ("one
>     module → `mod test`, otherwise `tests.rs`") collapses Rust's
>     `tests/` directory (integration, black-box) with `#[cfg(test)]
>     mod test` (unit, white-box).  Be explicit about which a given
>     test should be — they have different visibility rules.
>
>   RLB: You're confusing `src/tests.rs` with `tests`.  I haven't specified any
>   completely external integration tests.
>
>   CLAUDE: You're right, my mistake.  `src/tests.rs` (or any
>   `mod tests;` file under `src/`) is module-as-file with `#[cfg(test)]`
>   — it has full crate-internals visibility, same as an inline
>   `mod tests {}`.  The `tests/` directory at the crate root is the
>   integration-test convention and was never in the plan.  Strike that
>   bullet.
>
>   - **Determinism.**  Tests using RNGs, timing, or thread scheduling
>     may pass in C and flake in Rust (or vice versa) for unrelated
>     reasons.  Seed everything, mock clocks.
>
> *Scripting:*
>   - From Phase 0 inventory, auto-generate one stub `#[test] fn name()
>     { todo!() }` per C test case, placed by the rule above.  Humans
>     fill in bodies.
>   - A script that reads each C test file, extracts the API symbols it
>     calls, and looks them up in the Phase-1 module map will pick the
>     right destination module mechanically.
>   - Run `cargo test --no-run` in CI for this phase — that's the
>     "builds + cleanly fails" gate without actually executing
>     `todo!()` panics.
>
>   RLB: No, I actually mean `cargo test`.  It should fail cleanly in the sense
>   that all tests run and fail, and there's not a segfault or something.
>
>   CLAUDE: Got it.  The Phase 2 acceptance criterion is: `cargo test` runs
>   to completion, every test reports failure with a `todo!()` panic (or
>   matching panic message), no segfault, no abort, no compile error.  The
>   panic *is* the clean fail.
>
>   - Add a custom lint or a CI grep that fails the build if any test
>     `assert`s on a `todo!()` return value (a sign the test was copied
>     without thinking).
>
>   RLB: That doesn't make sense.  All the return values will be todo!(), but
>   the tests should be written as if the return values are real.
>
>   CLAUDE: Right, I was overthinking it.  The test asserts on the eventual
>   real return value; the panic from `todo!()` just happens earlier today
>   than it will tomorrow.  Strike that bullet.
>
>   - **Strong addition:** for every C test, also build a small
>     "differential harness" stub — same inputs, run both C and Rust,
>     diff outputs.  Wire it now even if it's empty; you'll want it in
>     Phase 3.
>
>   RLB: I don't know what you mean by this, but I think we should not do this.
>   If you mean that we should build a test binary that links against both the
>   Rust and C code and verifies their empirical equivalence -- maybe as a
>   follow-on phase. 
>
>   CLAUDE: Yes, that's what I meant — a binary that links both libraries,
>   drives them with the same inputs, and diffs outputs.  Agreed it's a
>   follow-on, not necessary for v1.  I'll move it to a "follow-on / risks"
>   appendix.

## Phase 3 Translate implementation

First decide on an implementation order:
* Make a call graph
* For each function, note its height in the graph
* For each test, note the max height of a function that it calls
* Order tests from lowest height to highest

Then, get each test to pass:
* While there are any `todo!()` methods in the call tree below the test...
* Choose one such method and translate it.
* If it calls a function that has not yet been defined, create a stub `todo!()` implementation.
* Verify that `cargo check` passes after translating each function
* Once there are no more functions to translate, verify that all tests converted
  so far pass, and all unconverted tests still fail cleanly

> **Review note (Phase 3):**
> *Feasibility:* Conceptually clean; operationally the riskiest phase
> because every function-level decision has cross-cutting consequences.
> "Height in the call graph" is a good first-cut ordering metric, but
> real call graphs have cycles (mutual recursion, callbacks via
> function pointers / traits, dispatch through tables) — height is
> undefined on a cyclic graph.  Pre-process: collapse strongly-connected
> components into single nodes, then topo-sort the DAG of SCCs.  An SCC
> with >1 node has to be translated as a unit.
>
>   RLB: I think we can make a working definition of "height" here: (a) Nodes
>   with no dependencies have height 0 (b) the height of any other node is the
>   length of the shortest path to a node of height 0.
>
>   CLAUDE: That definition works — BFS from leaves, height = shortest path
>   to a leaf.  Well-defined for any node that can reach a leaf at all.
>   Useful enough as an ordering heuristic.  One caveat: mutually recursive
>   pairs (functions that only ever call each other and shared leaves) end
>   up with the same height and have to be translated together regardless
>   — worth a one-line note that mutually recursive pairs are ported as a
>   unit.
>
> *Pitfalls:*
>   - **Call graphs through function pointers / traits are
>     under-approximated by static analysis.**  You'll miss edges and
>     surprise yourself when a "leaf" function turns out to be invoked
>     via a dispatch table.  Cross-check the static graph against a
>     dynamic call graph from a coverage-instrumented C run on the test
>     suite.
>
>   RLB: I'm OK with this.  We can discover this by running the tests.  In other
>   words, most call edges will be discovered at static analysis time or compile
>   time, and runtime might reveal more edges.  If that happens, we go implement
>   the required trait methods.
>
>   CLAUDE: Acknowledged — static catches most edges, runtime fills in the
>   rest, and the trait method we land on is the natural place to put the
>   missing implementation.
>
>   - **`cargo check` ≠ tests pass.**  The plan says "verify `cargo
>     check` passes after each function" — that's necessary but the
>     real gate is "the test that motivated this function still passes
>     and no previously passing test regressed."  Run the full
>     converted test set on every translation, not just `check`.
>
>   RLB: We should keep the per-function effort light.  The test will fail until
>   all dependencies are implemented, so `cargo test` won't add much over `cargo
>   check`.  We should run `cargo check` until we think we are done, then `cargo
>   test` + fix errors until we are actually done.
>
>   CLAUDE: Reasoning checks out — until the full sub-tree below a target
>   test is implemented, `cargo test` reports the same panic regardless of
>   which leaf you just translated, so it adds no signal over `cargo check`.
>   `cargo check` per function; `cargo test` only when the call tree below
>   the target test looks complete.
>
>   - **`todo!()` panics propagate at runtime, not at compile time.**
>     A function you thought was a leaf may transitively call a
>     `todo!()` because the call graph was wrong.  Treat each `todo!()`
>     hit during testing as a graph correction.
>
>   RLB: This is deliberate.
>
>   CLAUDE: OK.
>
>   - **Refactoring drift.**  When translating function `f`, you'll
>     often realize the Phase-1 signature for `g` (a callee) is wrong.
>     Allow signature corrections, but log them — late-phase signature
>     churn is a strong signal Phase 1 didn't read enough callers.
>
>   RLB: This is fine, we can refine as we go.
>
>   CLAUDE: OK.
>
>   - **Performance regressions are invisible to correctness tests.**
>     Bookmark hot functions (from a C profile) and microbenchmark
>     them as you port.  An idiomatic Rust translation that does an
>     extra allocation in a per-packet path is a real defect.
>
>   RLB: We can do benchmarking as a follow-on phase.  But you're correct that
>   the Rust logic should mirror the C logic as closely as possible, including
>   in when/how allocations are made.  The only deviations that should be made
>   are for safety reasons.
>
>   CLAUDE: Strong rule, easy to enforce in review.  I'll word it as: "the
>   Rust function should be a line-by-line analogue of the C function, with
>   deviations only where (a) safety requires it or (b) the C idiom has no
>   Rust equivalent."  Allocation patterns, control flow, and variable
>   lifetimes mirror the original.  Benchmarking moves to follow-on.
>
>   - **Bug-for-bug compatibility audits are rarely automatic.**  When
>     a Rust test passes but produces different bytes than C did,
>     decide which is correct.  Differential testing (Phase 2 hook)
>     catches this; unit assertions usually don't.
>
>   RLB: As stated above, we can do differential testing once we have a working
>   library.  Until then, we will aim for bug-for-bug compatibility by mimicking
>   at the source lvel.
>
>   CLAUDE: Noted — source-level mimicry is the v1 strategy, differential
>   testing is the v2 verification.
>
>   - **Unsafe and FFI escape hatches.**  Some functions cannot be
>     translated to safe Rust without a redesign (e.g., custom
>     allocators, intrusive lists, type punning via unions).  Permit
>     and isolate `unsafe` in named modules; require a comment
>     justifying each block.  Don't pretend it won't happen.
>
>   RLB: I will allow that we can allow unsafe, but it should be minimized, and
>   we should consider how to engineer around.
>
>   CLAUDE: Agreed.  Concrete rules: every `unsafe` block needs a
>   `// SAFETY:` comment justifying the invariants; the codebase should be
>   greppable so we can count occurrences and review them; before writing
>   `unsafe`, the translator should explicitly consider safe alternatives
>   (Box, slice, enum-as-union, arena+index) and write down why they don't
>   work.
>
>   - **Concurrency model translation.**  pthread / atomics / volatile
>     in C don't map 1:1 to Rust's `Send`/`Sync`/`Arc`/`Mutex`.  This
>     deserves its own sub-phase, not an in-line decision per function.
>
>   RLB: Let's limit our scope for now to single-threaded C code.
>
>   CLAUDE: Good scoping.  Worth a paragraph in the README that v1 is
>   single-threaded; multi-threading is explicit future work.  Any pthread
>   / atomic / volatile usage in the C source becomes a translation note
>   ("threading dropped; revisit in v2") rather than an in-line decision.
>
> *Scripting:*
>   - Build the call graph from Phase 0's libclang dump.  Collapse
>     SCCs, topo-sort, emit a JSON ordering.  Re-run on every commit;
>     diff to detect when new edges (e.g., a callback registration)
>     change the order.
>
>   RLB: No need to re-run on every commit.  If we discover new edges while
>   developing, we'll just use them within a run.  No need to update the call
>   graph.
>
>   CLAUDE: Agreed — Phase 0 is a one-shot artifact.  Regenerate only when
>   the C source changes substantively (e.g., a new vendored upstream).
>
>   - A `cargo todo` helper that lists every remaining `todo!()` with
>     its file/line and (from the call graph) which still-failing
>     tests need it.  Surface "next function to translate" as a single
>     command.
>
>   RLB: Don't make this `cargo todo`, just maek a script.
>
>   CLAUDE: Will be a standalone Python script, e.g.,
>   `scripts/next_todo.py`.  No cargo subcommand integration.
>
>   - Run `cargo test` after every translated function via a
>     file-watcher (`cargo watch -x test`).  Combined with the
>     differential harness, this is the tight inner loop.
>
>   RLB: No, just `cargo check` and `cargo test` manually.
>
>   CLAUDE: Noted — no file watcher, no `cargo-watch`.  Manual invocation.
>
>   - Track translation progress as a metric: count of `todo!()`
>     remaining, count of tests passing, lines of C with no Rust
>     equivalent yet.  Plot it; it's the only honest schedule.
>   - For each translated function, auto-generate a side-by-side diff
>     view (C source, Rust source) and store it as a review artifact —
>     reviewers should not have to grep for the original.
>   - Use `cargo-mutants` or similar mutation testing on translated
>     leaves to confirm tests actually exercise the new code.
>   - Coverage-gate the merge: `cargo llvm-cov` should show the
>     translated function is hit by at least one passing test before
>     it counts as "done."

## Cross-cutting concerns the plan doesn't address

These belong somewhere in the methodology — possibly a Phase 4, possibly
threaded through Phases 1–3 as gates.

* **Memory and concurrency model.**  Decide globally: arena vs. `Box`,
  `Rc<RefCell<…>>` vs. ownership refactor, `unsafe` policy, `Send`/`Sync`
  rules.  Without a written policy each function gets a fresh ad-hoc answer.

    RLB: Here are the policies:
    * Memory: Prefer simple, safe patterns like `Box` and borrowing.  Try to
      follow the original ownership model as much as makes sense.  Use
      Rc<RefCell<_>>, but only as a last resort.
    * `unsafe`: Avoid unless absolutely necessary.  When used, make sure there
      is a `# Safety` comment that accurately explains the conditions.
    * `Send`/`Sync` rules: Types should generally be both `Send` and `Sync`.
      Alert the user if this is not feasible.

    CLAUDE: Confirmed: v1 relaxes the `Send + Sync` requirement.  The
    last-resort escape hatch is `Rc<RefCell<_>>` (cheaper than
    `Arc<Mutex<_>>` for single-threaded code).  Types do not need to be
    `Send` or `Sync` in v1; we'll revisit ownership when v2 adds
    multi-threading, since the threading pass would force a re-audit
    regardless.  Rationale captured: keep v1 simple, defer the cost to
    when it's actually needed.

* **Error handling translation.**  C uses return codes + `errno` + global
  state; Rust uses `Result<T, E>`.  Define the error type(s) once, in
  Phase 1, and don't let each translator invent a new one.

    RLB: Yes, we should be idiomatic, returning Result<T, E>.  There should be
    one error type for the whole library, into which more precise error types
    might be convertible.

    CLAUDE: Standard pattern — top-level `Error` enum at the crate root,
    `From` impls (via `thiserror` or hand-rolled) from any module-level
    sub-error types.  C functions returning a status code become
    `Result<(), Error>`; functions with output parameters become
    `Result<T, Error>`.  `errno`-style globals are dropped — the error
    travels in the `Result`.

* **Logging, tracing, panics.**  C `assert` is `debug_assert!`; C `abort`
  is `process::abort`; C `fprintf(stderr, …)` is `tracing` or `log`.
  Pick the crates up front.

    RLB: What crates would you suggest?

    CLAUDE: Given the no_std + alloc + std-feature constraint below:
    * **Logging:** `log` (the `log` crate, v0.4) with
      `default-features = false`.  Works in no_std, very small, the
      de-facto standard.  `tracing` is more powerful but its no_std
      story is awkward — not worth it for this port.
    * **Errors:** `thiserror` v2 with `default-features = false` —
      v2 added no_std support in late 2024.  If you'd rather avoid the
      dependency entirely, hand-roll `impl core::error::Error` for the
      top-level enum (stable since Rust 1.81; ~30 lines for a typical
      enum).
    * **Asserts / panics:** Built-ins (`assert!`, `debug_assert!`,
      `panic!`) are no_std-native; no crate needed.  C `abort()` maps
      to `core::process::abort` or `panic!` (with `panic = "abort"` if
      the binary needs the abort-on-panic semantic).
    Concretely in `Cargo.toml`:
    ```
    log       = { version = "0.4", default-features = false }
    thiserror = { version = "2",   default-features = false }
    ```

* **Build, CI, release engineering.**  Workspace layout, MSRV,
  `cargo deny`, sanitizers (`-Zsanitizer=address` on nightly), fuzzing
  harness, no-std support if relevant.  Decide before Phase 1 because
  these constrain dependency choices.

    RLB: Don't worry about these for the most part.  The one thing I will insist
    on is that we should target no_std + alloc + an `std` feature gate.

    CLAUDE: Acknowledged.  Implications I'll bake into the plan:
    * `#![no_std]` at the crate root, `extern crate alloc;`, `std` is a
      feature in `Cargo.toml` (gated on by default for now, opt-out for
      embedded users).
    * `std::collections::HashMap` is std-only — replace with `hashbrown`
      (or `BTreeMap` if hashing isn't on the hot path).
    * `std::io::Read` / `Write` aren't available in core — gate any I/O
      behind the `std` feature, or use `core2`/`embedded-io` traits.
    * Time: `core::time::Duration` exists, but `std::time::Instant`
      doesn't — any timing API takes a clock trait or is `std`-gated.
    * Errors: `core::error::Error` (stable 1.81+), no `std::error::Error`
      import needed.
    * Every dependency must be checked for `default-features = false`
      compatibility and a no_std feature flag.

* **Tooling stack to settle on early:**
    - libclang + Python for AST analysis (Phase 0, 3 graph build)
    - `bindgen` for the FFI baseline (Phase 1 reference)
    - `c2rust` if you want a transpiled starting point — but treat its
      output as scaffolding to delete, not code to ship; it produces
      heavily-`unsafe` Rust that defeats the safety goal
    - `bear`/`compiledb` for `compile_commands.json`
    - `cargo-watch`, `cargo-llvm-cov`, `cargo-mutants`, `cargo-deny`,
      `cargo-fuzz`, `cargo-criterion`
    - a differential-testing harness (custom; small)
    - a progress dashboard (a Python script that reads the inventory
      and the current crate and renders a single-page HTML status)

    RLB: Changes:
    * Do not use `c2rust`.  We will translate manually to safe, idiomatic Rust
    * Use `cmake` instead of `bear`/`compiledb`.
    * We shouldn't need most of those `cargo` tools
    * Please do make a progress dashboard.

    CLAUDE: Agreed across the board.  Final tooling list:
    * `cmake` (for `compile_commands.json`)
    * libclang + Python (Phase 0 inventory; call graph; dashboard)
    * `bindgen` (per-file allowlisted reference output, never shipped)
    * Python progress dashboard (HTML, color-coded call/include graph)
    * `cargo check` and `cargo test` for the inner loop (manual)
    No `c2rust`, no `bear`, no `cargo-watch`/`-mutants`/`-llvm-cov`/
    `-deny`/`-fuzz`/`-criterion`, no differential harness in v1.

* **Definition of done.**  State the exit criteria for the whole port:
  e.g., all C tests pass in Rust, differential harness clean on the
  fuzz corpus, microbenchmarks within X% of C, zero `unsafe` outside
  declared modules, `cargo deny` clean.  Without this, the project
  can't end.

    RLB: The current project is complete when all tests and functions have been
    translated and all of the translated  tests pass.

    CLAUDE: Clean and consistent with the rest of the plan — the test
    suite is the acceptance gate, period.  No microbenchmark target, no
    fuzz corpus, no `cargo deny` clean, no `unsafe` count threshold —
    those move to follow-on phases.  Concretely the v1 exit gate is:
    `cargo build` clean, `cargo test` green, zero `todo!()` remaining
    in the crate (greppable check).
