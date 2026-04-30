# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Active branch context

The current branch is `c2rust`, which exists to translate the C library
to safe, idiomatic Rust.  Before making any change on this branch, read:

- `TRANSLATE_PLAN.md` — the agreed plan, scope, and policies (single
  target, single-threaded scope, no_std + alloc + std feature, hard
  fork, "safety wins" tie-breaker, etc.).
- `TRANSLATE.md` — the annotated discussion that produced the plan.
  Useful for *why* a decision was made.

The plan is binding: don't introduce C-side changes on this branch
unless they're explicitly part of the translation work, and don't
deviate from the policies in `TRANSLATE_PLAN.md` (memory model, error
handling, `no_std` constraints, tooling stack) without raising it
first.

The Rust translation lives in `rs/fq/` (currently a stub crate).  All
translation work goes there — don't create sibling crates, a
workspace, or parallel trees without discussing first.  Any scripts
written to drive the translation (Phase 0 inventory, dashboards,
`next_todo.py`, etc.) go in `./scripts/`.

Don't run inline Python (`python3 -c '…'`).  Write the script as a
file under `./scripts/`, then invoke it.  Even one-shot diagnostic
queries against `inventory.json` get a script — they are reusable
later and they make the work visible.

Translation artifacts (inventory, call graph, ifdef manifest,
dashboard) live in `./xlate/`.  The cmake-produced
`compile_commands.json` lives in `./build/` (a regular cmake build
dir).  Don't conflate the two — `build/` is for cmake, `xlate/` is
for the translation pipeline.

Log every shell command run during the translation work to
`./COMMANDS.log` (append-only, with a one-line description).

**Never edit files outside of `rs/`, `scripts/`, or `*.md`.**  The C
sources, CMake files, and everything else in the tree are read-only
on this branch.  If a translation task seems to require a C-side
change, surface it instead of making the change.

Inner loop, run from `rs/fq/`:

```sh
cargo check                 # per-function gate during Phase 3
cargo test                  # end-of-sub-tree gate (see TRANSLATE_PLAN.md)
cargo fmt
cargo clippy -- -D warnings
```

## Build and test (C, on `master`)

The project uses CMake.  Picotls is a required dependency; the simplest
path is to let CMake fetch it:

```sh
cmake -DPICOQUIC_FETCH_PTLS=Y .
make
```

Otherwise clone and build picotls separately at the same directory
level as picoquic, then `cmake . && make`.

OpenSSL is required (1.1.1 or 3.0).  See `doc/building_picoquic.md`
for platform-specific details (Windows uses the `picoquic.sln` Visual
Studio solution).

### Running tests

After building, two test binaries exist:

```sh
./picoquic_ct -S . -n -r        # core library tests
./picohttp_ct -S . -n -r        # HTTP/3 + WebTransport tests
```

`-S <dir>` points at the source root (for fixtures), `-n` disables
network access, `-r` randomizes order.  Both binaries also accept a
single test name as a positional argument:

```sh
./picoquic_ct <test_name>       # run one named test
./picoquic_ct -x <test_name>    # exclude one test
./picoquic_ct -h                # list available tests
```

`ctest` also works (runs both suites with the flags above):

```sh
ctest --output-on-failure
```

### Useful CMake options

- `-DPICOQUIC_FETCH_PTLS=Y` — fetch picotls during configure.
- `-DENABLE_ASAN=ON` / `-DENABLE_UBSAN=ON` — sanitizer builds.
- `-DDISABLE_DEBUG_PRINTF=ON` — silence debug output.
- `-DWITH_MBEDTLS=ON` — enable mbedtls crypto provider.
- `-Dpicoquic_BUILD_TESTS=OFF` — skip building tests (useful when
  consumed as a CMake subproject).

### Formatting

```sh
make clangformat        # WebKit style, applied to all *.c and *.h
```

## Architecture

Authoritative reference: `doc/architecture.md`.  Quick orientation:

### Library layering

1. **`picoquic-core`** (`picoquic/`) — the QUIC transport library.
   Public API in `picoquic/picoquic.h`; internals in
   `picoquic/picoquic_internal.h` (subject to change).  Depends on
   picotls + OpenSSL (or mbedtls) for TLS 1.3.
2. **`picoquic-log`** (`loglib/`) — optional text/binary/qlog/perf
   logging.  Linked only when the application enables it.
3. **`picohttp-core`** (`picohttp/`) — minimal HTTP/3 + QPACK +
   WebTransport on top of picoquic-core.
4. Executables: `picoquicdemo` (`picoquicfirst/`), `pqbench`
   (`pqbench_app/`), `picolog_t` (`picolog/`), `pico_sim` (`p_sim/`),
   `picoquic_sample` (`sample/`).

### Object model (transport library)

- **QUIC context** — top-level object holding routing tables,
  configuration, and shared state.  An application typically creates
  one QUIC context per protocol/listener.  Multiple contexts can
  coexist in one process.
- **Connection** — created within a QUIC context, represents one
  QUIC connection (client- or server-side).
- **Path** — within a connection, represents one network path.
  Multiple paths support migration and the multipath extensions.
- **Stream** — within a connection, application data flow.

### Two design invariants worth knowing

1. **Single-threaded library.**  picoquic-core is *not* thread-safe.
   The application must serialize calls into a given QUIC context.
   Multiple QUIC contexts can run on different threads.
2. **Virtual time.**  The library never reads the system clock.
   Every API call that needs "now" takes the time as a parameter.
   This is what makes the network-simulator-driven test suite
   possible.

### Test architecture

Tests are compiled into a static library `picoquic-test`
(`picoquictest/*.c`), then dispatched by name via `picoquic_ct` /
`picohttp_ct`.  Many tests use a built-in network simulator
(`sim_link.c`) and virtual time, so they run deterministically without
real sockets.

### Application/Network API split

- **Application API** (`picoquic.h`) — create contexts and
  connections, register callbacks for events (new connection, data
  received, stream events), open streams, push data, close.
- **Network API** — feed received packets in, poll for packets to
  send, ask "how long until I need to act again."  The default event
  loop is `sockloop.c`; applications can write their own (e.g., to
  integrate with `libevent` or to share sockets with other protocols).

A typical application loop: get next-action-deadline → wait on
sockets up to that deadline → submit any received packets → poll for
packets to send → repeat.
