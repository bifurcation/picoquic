# scripts/

Tooling for the picoquic → fq Rust translation.  Mostly Python.

## Agent selection

AI-driven phases use `scripts/agent_runner.py`.  Claude remains the
default so existing commands keep working, but every driver that invokes
an AI agent now accepts the same switch:

```sh
python3 scripts/phase4.py --agent codex
XLATE_AGENT=codex python3 scripts/phase3a.py --limit 1
```

Common knobs:

| Setting | Effect |
|---------|--------|
| `--agent claude\|codex` / `XLATE_AGENT` | Select the provider. |
| `--model` / `XLATE_AGENT_MODEL` | Override the model for either provider. |
| `XLATE_CLAUDE_MODEL`, `XLATE_CODEX_MODEL` | Provider-specific model defaults. |
| `--codex-sandbox` / `XLATE_CODEX_SANDBOX` | Sandbox passed to `codex exec` (default `workspace-write`). |
| `--codex-approval` / `XLATE_CODEX_APPROVAL` | Approval policy passed to `codex exec` (default `never`). |
| `XLATE_CLAUDE_ARGS`, `XLATE_CODEX_ARGS` | Extra provider CLI args, shell-split. |

Transcripts go to `xlate/claude_logs/...` or `xlate/codex_logs/...`
depending on the selected provider.  Claude receives the script's
`--allowedTools` list directly.  Codex has no matching per-run
allowlist flag, so the runner appends that list to the prompt as the
intended action scope and runs `codex exec` in the selected sandbox.

## Phase 0: inventory & dashboard

One-shot pipeline that produces a snapshot of the C codebase in
`xlate/*.json` plus a status dashboard at `xlate/dashboard.html`.

```sh
# First-time setup (only when the C source has changed substantively):
mkdir -p build && cd build && \
  cmake -DPICOQUIC_FETCH_PTLS=Y -DCMAKE_EXPORT_COMPILE_COMMANDS=ON ..
cd ..

# Run the whole pipeline:
python3 scripts/phase0.py
open xlate/dashboard.html
```

Individual steps (in order; later scripts depend on earlier outputs):

| Script              | Produces                       | Notes |
|---------------------|--------------------------------|-------|
| `inventory.py`      | `xlate/inventory.json`         | parses every in-scope C TU with libclang; ~110s for picoquic. |
| `call_graph.py`     | `xlate/call_graph.json`        | SCCs, heights (BFS from leaves), mutually-recursive groups. |
| `ifdef_scan.py`     | `xlate/ifdef_manifest.json`    | classifies every `#if`/`#ifdef` symbol against `-D` flags + include guards. |
| `dashboard.py`      | `xlate/dashboard.html`         | static HTML page consuming all of the above. |
| `phase0.py`         | (driver)                       | runs all four in sequence. |

In-scope directory (v1): `picoquic/`.  `picoquictest/` will be added
in Phase 2.  `loglib/` and `picohttp/` are out of scope for v1.  Also
out of scope: executables (`picoquicfirst/`, `pqbench_app/`,
`picoquic_t/`, etc.) and fetched dependencies under `build/_deps/`
(picotls).

## Phase 1: per-header translation (driver)

`phase1.py` translates each in-scope header to a Rust module via the
selected agent, gated on `cargo fmt + clippy + check`.  Resumable via
`xlate/phase1_state.json` (`ok` / `fail` / `stub`).  See
`TRANSLATE_PLAN.md` Phase 1 for policy; the script's docstring
explains flags.

## Phase 1A: AI self-review

`phase1a.py` re-reads each Phase-1-`ok` header and applies quality
improvements (safety, consistency, idiomatic Rust) in place via
`Edit`.  No `Write` — refinement only.  State at
`xlate/phase1a_state.json`; per-header transcripts under
`xlate/<agent>_logs/phase1a/`.

```sh
python3 scripts/phase1a.py --status
python3 scripts/phase1a.py --limit 1     # smoke one header first
python3 scripts/phase1a.py               # full pass
```

## Phase 1B: cross-module consistency report

`phase1b.py` is pure inspection — no agent.  It scans
`rs/fq/src/picoquic/` and writes
`xlate/consistency_report.md` cataloguing patterns that vary
across modules: trait naming conventions, module-level lint
allowances, type definitions (with duplicates flagged), and
cross-module imports.  The human reviewer reads the report,
decides on consistency policies, and uses Phase 1C to apply.

```sh
python3 scripts/phase1b.py            # write the report
python3 scripts/phase1b.py --json     # additionally emit raw data
```

## Phase 1C: address `// REVIEW:` comments

`phase1c.py` scans `rs/fq/src/` for `// REVIEW: <instruction>`
markers a human reviewer left in the code, and asks the selected agent to
address each one.  Successfully-addressed lines are removed;
unresolved ones are rewritten as `// REVIEW(open): <reason>` so
they don't get re-asked on the next run.  State at
`xlate/phase1c_state.json`, keyed by Rust file path.

```sh
python3 scripts/phase1c.py --list        # files with // REVIEW: now
python3 scripts/phase1c.py --status
python3 scripts/phase1c.py --dry-run --limit 1
python3 scripts/phase1c.py               # process all
```

1B and 1C can interleave: human reads the report, adds REVIEW
markers, runs 1C, regenerates report, repeats.

## Phase 4A/4B/4C/4D/4E: function map, completion, audit, and repair

After `phase4.py` has filled existing Rust incomplete markers, the
follow-up drivers close gaps that a marker-driven pass can miss:

```sh
python3 scripts/phase4a.py              # write map + approval plan
python3 scripts/phase4b.py --status     # list approved missing work
python3 scripts/phase4b.py --approved   # run implementation agent
python3 scripts/phase4c.py --status     # summarize body-only reviews
python3 scripts/phase4c.py              # run comparison agent + HTML report
python3 scripts/phase4d.py --status     # summarize deep-review repairs
python3 scripts/phase4d.py --classify-only --shard-count 8 --shard-index 0
python3 scripts/phase4d.py              # run repair agent + HTML report
python3 scripts/phase4e.py --status     # summarize confirmed repairs
python3 scripts/phase4e.py              # repair Phase 4D needs_fix entries
```

Artifacts:

| Script       | Produces |
|--------------|----------|
| `phase4a.py` | `xlate/function_translation_map.json`, `xlate/phase4a_plan.md`, `xlate/phase4a_plan.html` |
| `phase4b.py` | updates `xlate/function_translation_map.json` as required functions are implemented |
| `phase4c.py` | `xlate/phase4c_reviews.json`, `xlate/phase4c_report.html` |
| `phase4d.py` | `xlate/phase4d_results.json`, `xlate/phase4d_report.html`; updates Rust and refreshes `xlate/function_translation_map.json` when repairs change spans |
| `phase4e.py` | `xlate/phase4e_repairs.json`, `xlate/phase4e_report.html`; repairs `needs_fix` items and updates `xlate/phase4d_results.json` |

`phase4b.py` refuses to run unless `xlate/phase4a_plan.md` is marked
`Status: approved`, unless `--approved` is passed explicitly.

## Helpers / diagnostics

| Script              | Purpose |
|---------------------|---------|
| `_clang_setup.py`   | imported by other scripts; configures libclang.dylib path. |
| `cc_bucket.py`      | one-shot summary of `compile_commands.json` by source dir. |
| `inv_query.py`      | ad-hoc queries against `inventory.json` (callee histogram, top-callers, summary). |
| `_dump_flags.py`    | print the libclang flag list for one TU (for debugging parse failures). |
| `_callee_check.py`  | sanity-check the callee walk against raw CALL_EXPR cursor counts. |
| `_inv_inspect.py`   | find malformed entries in an inventory.json. |

The leading underscore marks scripts that are diagnostics / one-shot;
they were useful to build, are kept around for debugging, but aren't
part of the routine pipeline.

## Library-finding

libclang (Homebrew LLVM) lives at `/opt/homebrew/opt/llvm/lib/libclang.dylib`.
`_clang_setup.py` resolves this and ignores `LIBCLANG_PATH` if it points
to anything other than a libclang.dylib file.

The macOS SDK path (for `<stdint.h>` etc.) and clang's own builtin
include directory (for `<stdarg.h>`) are added automatically by
`inventory.py:args_for()`.

## Pre-existing scripts (not part of the translation work)

`coverage.sh` and `getlog.pl` were here before the translation effort.
Leave them alone.
