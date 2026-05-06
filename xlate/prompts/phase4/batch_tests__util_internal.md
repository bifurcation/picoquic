# Phase 4 batch translation: 2 source files

## NO SHORTCUTS — do the hard work

Phase 4 translates the **whole library**.  Past sweeps
produced hundreds of stubs that compile but don't translate
the C body — this is failure, not completion.  The gate
flags all three forms of incompleteness identically:

* `todo!()`
* `unimplemented!()` ← forbidden
* `// SKIP:` markers with stub returns (`None`,
  `Err(Generic)`, `Ok(())`, fabricated defaults) ← forbidden

If a body is hard: translate the helpers it needs first;
add missing struct fields; port C static tables; extend
the Phase 1/2 traits as needed; use Rust ownership idioms
instead of `None` shortcuts.

**Nothing is fundamentally untranslatable.**  Every function
in picoquic has a body — translate it.  Phase 4 IS the
translation: it covers the whole library, not 'the easy
parts.'  Concrete mappings:
  * `pthread_create` → `std::thread::spawn`
  * `pipe()` wake-up → `std::sync::mpsc` or `Condvar`
  * `select`/`poll`/`io_uring` → `mio` crate
  * loglib helpers → `log` crate + standard file I/O

**Forbidden 'blocker' excuses** (every one is a shortcut):
'out of scope', 'deferred to a later pass', 'multi-threading
not in scope', 'loglib not in scope', 'sub-system not yet
translated', 'needs design thought', 'borrow-checker issue'.

Do not leave blocker `todo!()`s behind.  If a dependency is
missing, expose or implement the dependency first, then return
to the original body.

Fill in incomplete function bodies in the Rust files listed
below by translating the matching C bodies in `picoquic/`.
This is a **batch** invocation — handle every file in one
session, amortising the read of the translation guide and
any cross-module API lookups.

Total incomplete markers in this batch: **17** (counted across `todo!()`, `unimplemented!()`, `// SKIP:`,
and placeholder/stub/TODO wording).

## What this is

Phase 1 / 2 designed the Rust API surface.  Phase 3
translated the C tests against it (currently panic on
`todo!()`).  **Phase 4 fills the bodies** so those tests
pass.  The C source is the spec.

## Required reading (once for the whole batch)

1. **`xlate/impl_translation_guide.md`** — Rust idioms for
   common C body patterns (memory, control flow, errors,
   strings, time, network, logging, crypto).  Read once.
2. The shared module map at the bottom of that guide tells
   you which C source(s) match each Rust file in this batch.
3. Skim `rs/fq/src/lib.rs` re-exports + `rs/fq/src/internal.rs`
   only as needed; the guide should cover most signatures.

Do **not** re-read the guide once per file — read it once
and use what you remember across the whole batch.

## Sources to translate (this batch)

### `rs/fq/src/tests/util.rs` (5 incomplete markers)

### `rs/fq/src/internal.rs` (12 incomplete markers)

## Translation rules

- **Faithful, idiomatic.**  Walk the C body and produce the
  equivalent Rust.  Don't reshape the algorithm.
- **Don't change signatures.**  Phase 1 / 2 / 3 settled them.
  If a body genuinely cannot be expressed under the existing
  signature, surface it on stdout and skip the function.
- **No `unsafe`.**  No edits outside `rs/fq/src/`.  Test files
  under `rs/fq/src/tests/` are in scope when they are listed
  in this batch.
- **Idiomatic Rust over C-mirroring.**  See the guide.
- **No new public items.**  Helpers are fine but private.
- **Tests are the gate.**  Run `cargo test --no-run` after
  each meaningful chunk to verify the file still compiles.

## Stop spinning

If 5 grep/read calls into the same file haven't found what
you need, **stop searching**.  Open the C source and the
Rust target side by side; the C function name is in the
doc comment.  Aim to start writing edits within the first
10 tool calls of EACH file.

## Verification

After translating each file, run:
    python3 scripts/phase4_check.py <rs file>
to confirm 0 incomplete markers remain.  If any do, fix them
before moving on.

After the whole batch, run:
    python3 scripts/phase4_check.py rs/fq/src/tests/util.rs rs/fq/src/internal.rs
    cd rs/fq && cargo fmt
    cd rs/fq && cargo test --no-run
    cd rs/fq && cargo clippy --tests --all-features -- -D warnings
All four must succeed.  Iterate until they do.

## Process

1. Read the impl translation guide once.
2. For each file in this batch:
   a. Read the Rust target end-to-end.
   b. Identify matching C source(s) (doc comments name them).
   c. For each placeholder marker: find the C body or stale
      comment it refers to, then implement or correct it.
   d. Run `python3 scripts/phase4_check.py <file>`; iterate.
3. Run the gate (fmt + cargo test --no-run + clippy) once at
   the end of the batch.
4. Report on stdout: a one-paragraph summary per file.
