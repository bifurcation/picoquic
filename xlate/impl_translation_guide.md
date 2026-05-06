# Implementation translation guide (Phase 4)

A focused reference for filling in `todo!()` bodies in
`rs/fq/src/`.  Companion to `xlate/test_translation_guide.md`
(which covers the test side); they share the API surface
overview and naming conventions, but this guide focuses on
*body* translation.

**Read this first.**

---

## NO SHORTCUTS — the prime directive

Phase 4's job is to **translate the entire library**, not to fill
the easy bodies and stub the hard ones.  Past sweeps have produced
hundreds of "completed" stubs that returned `None` / `Err(Generic)`
/ `Ok(())` plus a `// SKIP:` comment.  **That is failure, not
completion.**  The gate now flags `todo!()`, `unimplemented!()`,
and `// SKIP:` markers all the same.

If a body is hard — borrow checker, lifetimes, missing API
surface, missing struct fields — **do the hard work**:

* If the borrow checker says `&mut` from `&self`, use the actual
  Rust idiom (`&mut self`, `RefCell`, restructure the call site).
  Don't return `None` and call it done.
* If a method needs a static table the C side has, **port the
  table** to a Rust `static` / `const`.  Don't fabricate a
  placeholder.
* If the Rust API is missing a field a C body uses, **add the
  field** to the Rust struct.  Don't skip the function.
* If the Rust API is missing a helper a C body needs, **translate
  the helper first**, then come back.

The only legitimate way to leave a body unfinished is `todo!()`
with a clear blocker note above it explaining what it depends on.
**Never `unimplemented!()`, never `// SKIP:`, never a fake stub
return** — those hide the work behind the gate.

---

## Operating mandate

* **Translate every C body faithfully.**  Walk `picoquic/<src>.c`
  for each function whose Rust counterpart is incomplete and write
  the equivalent Rust body.  Don't sidestep with a simpler-looking
  algorithm — the C body is the spec.
* **Tests are the gate.**  After each meaningful chunk, run
  `cd rs/fq && cargo test` and watch the panic count drop.  A
  function is "done" when the tests that reach it stop panicking.
* **Idiomatic Rust over C-mirroring.**  Phase 1 already shaped
  signatures.  Phase 4 is where the shape pays off — use Rust
  idioms in the body even when they don't match the C control
  flow.
* **No `unsafe`.**  No edits outside `rs/fq/src/`.

---

## C body patterns → Rust idioms

### Memory and ownership

| C pattern | Rust translation |
|---|---|
| `T* p = malloc(sizeof(T)); ...; free(p);` | `let p = Box::new(T { ... });` (auto `Drop`) |
| `T arr[N]` on stack, returns by ptr | `[T; N]` or `&[T; N]`, never `*const T` |
| `(uint8_t* buf, size_t len)` aliasing call | `&[u8]` (read) / `&mut [u8]` (write); slice carries len |
| Linked list: `T* head; T* cur->next` | `Vec<T>` or `VecDeque<T>` if FIFO; iterate with `.iter()` |
| Intrusive node: `struct T { …; next *T; }` | Token-based: parent owns `Vec<T>`, callers hold an [`Arena`]/[`SplayTree`]/[`HashTable`] token |
| `void* opaque_handle` | `Box<dyn Trait>` (one trait per shape) |
| `static T global` | `static T` if `T: Sync` else `pub fn name() -> T` accessor |

### Control flow

| C pattern | Rust translation |
|---|---|
| `if (ret == 0) { ... } else { ret = -1; }` | `if x { … }`; propagate fallibility with `?` |
| `goto cleanup; cleanup: free(p); return ret;` | Rust `Drop` runs automatically; `?` for early-return |
| `int ret = 0; ... ret \|= f(...); ... return ret;` | Compose `Result` with `?`; never use bitwise-OR for status |
| `for (size_t i = 0; i < n; i++) { ... }` | `for i in 0..n { ... }` or `.iter().enumerate()` |
| `while (cur != NULL) { ... cur = cur->next; }` | `while let Some(node) = ...` or `.iter()` |
| `switch (e) { case X: ...; case Y: ...; default: ...; }` | `match e { X => ..., Y => ..., _ => ... }`, exhaustive |
| `assert(cond)` | `assert!(cond)` or `debug_assert!(cond)` |

### Errors and returns

| C pattern | Rust translation |
|---|---|
| `int` 0/-1 status | `Result<(), Error>`; map `0 → Ok(())`, `-1 → Err(Error::…)` |
| `int` 0/PICOQUIC_ERROR_X | `Result<T, Error>`; map error codes to specific `Error::…` variants |
| Out-parameter `T* result` | `-> Result<T, Error>` returning the value by-move |
| `size_t` returning bytes-written, with `SIZE_MAX` on failure | `Result<usize, Error>`, never `usize::MAX` as a sentinel |
| `bool` returning 0/1 | `bool` |

### Strings and bytes

| C pattern | Rust translation |
|---|---|
| `char const*` (read-only string) | `&str` |
| `char*` (owned) | `String` |
| `(char* str, size_t cap)` (caller-allocated) | `&mut [u8]` or `&mut String`; document encoding |
| `memcpy(dst, src, n)` | `dst.copy_from_slice(src)` (if same len) |
| `memset(buf, c, n)` | `buf.fill(c)` |
| `memcmp(a, b, n)` | `a == b` (slice-equality) or `a.cmp(b)` |
| `picoquic_constant_time_memcmp` | [`crate::utils::constant_time_memcmp`] |

### Time

* `uint64_t current_time` (microseconds) → [`crate::Instant`] (`fugit::Instant<u64, 1, 1_000_000>`).
* Duration / RTT / interval → [`crate::Duration`].
* Construct with `Instant::from_ticks(usec)` / `Duration::from_ticks(usec)`.
* Subtract instants: `now - earlier` returns `Duration`.

### Network

* `struct sockaddr*` → `&core::net::SocketAddr`.
* IP address stored as bytes → `core::net::IpAddr`.
* Port byte-order: keep host-order in Rust; do `.to_be_bytes()` /
  `.from_be_bytes` only at the wire boundary.

### Logging / formatting

* `printf` / `fprintf` / `picoquic_log_xxx` → `log::{trace, debug,
  info, warn, error}` macros where appropriate; or write into a
  `&mut impl core::fmt::Write` buffer for the existing logger
  callbacks.
* `va_list` / variadic → `core::fmt::Arguments<'_>`; the call site
  uses `format_args!`.
* `snprintf` / `sprintf` → `format!` / `write!` / `core::fmt::Write`.
* Hex / binary formatting: use `{:02x}`, `{:08b}`, etc.

### Crypto

* AES / ChaCha20: use the Rust Crypto trait crates (`aead`,
  `cipher`, `digest`) and concrete impls (`aes`, `chacha20`).
* Header protection lives in [`crate::header_protection`]; AEAD
  packet-key plumbing in [`crate::tls::PacketKey`] (impls in
  [`crate::sys::picotls`] / [`crate::sys::openssl`] backends).
* HKDF with SHA-256 for QUIC initial secrets (RFC 9001 fixes
  this); use the `hkdf` crate.

---

## Anti-patterns — these are forbidden, period

The gate detects all of these.  None of them is "Phase 4 done":

* **`unimplemented!()`** — same runtime effect as `todo!()` but
  often used by agents to feel productive while skipping work.
  Treat it as identical to `todo!()`.
* **`// SKIP:` markers with stub returns** — `None` / `Err(Generic)`
  / `Ok(())` plus a comment explaining why you couldn't.  This is
  the worst pattern: it compiles, it doesn't panic obnoxiously,
  it looks like a translation, and it silently breaks every
  caller.  **Do not write `// SKIP:` comments.**
* **`Err(Error::Generic)` stubs** — same as `// SKIP:` without the
  honesty.  Forbidden.
* **`Ok(())` no-ops for functions with real C-side side effects** —
  running scenarios, building state, walking qlog files, mutating
  shared state.  Forbidden.
* **Fabricated return values** — returning `0` / `false` / `None`
  / a placeholder default when the C body computes something real.
  Forbidden.
* **"Half-translated" bodies** that skip the C function's harder
  branches (e.g. translating the success path and writing
  `todo!()` for the error path).  Forbidden.

### Nothing is fundamentally untranslatable

Every function in picoquic has a body.  Translate it.  The C
body is the spec; produce equivalent Rust.  Phase 4 IS the
translation — it covers the whole library, not "the easy parts."

* **`pthread_create` / thread-spawning** → `std::thread::spawn`.
  Rust has full threading support.  picoquic spawns network
  threads internally; translate that with `std::thread` and
  `std::sync::mpsc`.  The fact that the public `Quic` type is
  not `Send`/`Sync` (caller serializes access) is a Rust-API
  property; it does **not** excuse skipping threading code in
  the C body.
* **Cross-thread wake-up** (C uses `pipe()` + `write(1)` to wake
  a `select(2)`-blocked loop) → `std::sync::mpsc`,
  `std::sync::Condvar`, or a self-pipe via `nix`.  Real bodies.
* **Event loops (`picoquic_packet_loop_v3` etc.)** are the heart
  of the library.  Translate the receive-process-send loop
  directly.
* **`select(2)` / `poll(2)` / `io_uring`** → use the `mio` crate
  (or `tokio` lower-level bits) for portable poll, or
  platform-specific `nix` bindings.  Real Rust code.
* **Loglib functions (binlog / qlog / textlog / performance_log
  / logger).**  Tests call them; translate them.  Use the
  `log` crate plus standard file I/O.
* **Sub-system not yet translated** is never a valid blocker.
  Translate the sub-system **first**, then come back.  The whole
  library has to be translated; do it in dependency order.

### What to do instead when a body is hard

1. **Translate the helpers it needs first.**  A function that
   "needs `picoquic_get_stored_ticket`" really means "translate
   `picoquic_get_stored_ticket` first."  That helper is the
   blocker; resolve it.
2. **Add the missing fields / types.**  If a C body reads
   `cnx->nb_zero_rtt_sent` and the Rust struct doesn't have it,
   **add it** (it's already in `internal.rs` as
   `Connection::nb_zero_rtt_sent` — verify before assuming
   absence).
3. **Use Rust ownership idioms.**  "Returns `&mut` from `&self`"
   means the signature wants `&mut self` or `RefCell`, not
   "return `None`."
4. **Port C static tables to Rust `static`/`const`.**  If a
   version-table or cipher-suite array is "not yet wired up,"
   wire it up.
5. **Extend Phase 1/2 traits when a body needs it.**  If `Socket`
   doesn't expose `open_with_options` and a body needs it, add
   the trait method.  The shape is mutable until tests pass.

### When `todo!()` is the only option

Almost never.  The only acceptable unfinished state is a single
bare `todo!()` with a one-line **real-blocker** note above it.
"Real blocker" means a concrete external dependency you cannot
resolve in this pass, e.g.:

* "blocked: needs picotls API not yet exposed in the picotls
  Rust binding"
* "blocked: needs OS-level kernel-tls socket option that doesn't
  exist on this platform"

These are **NOT** valid blockers — every one is a shortcut:

* "deferred to a future pass" / "follow-on work"
* "out of scope"
* "multi-threading not in scope" (use `std::thread`)
* "loglib not in scope" (translate the helpers)
* "sub-system not yet translated" (translate the sub-system FIRST)
* "needs more design thought"
* "borrow-checker issue" (use `&mut self` / `RefCell` /
  restructure)
* "lifetime issue" (clone, restructure, or change the signature
  if Phase 1 got it wrong)

Translate the function.

---

## The "tests are the gate" workflow

For each translated function:

1. **Find the tests that exercise it.**  `grep -lE "<fn name>" rs/fq/src/tests/`
   often points right at them.  The Phase 3 known-answer tests
   (intformat, varint, siphash, splay, hashtest, code_version)
   exercise data-structure leaves first.
2. **Translate the body.**  Use Edit; preserve the existing
   signature and doc comment.  Don't rename / reshape.
3. **Run `cargo test <test_name>`.**  If it still panics, the
   panic backtrace tells you which other `todo!()` is in the
   way.  Translate that next, then rerun.
4. **Repeat until the test passes.**  Then move to the next test.

Bottom-up order is the natural one.  In the Phase 0 call graph,
function-level "max height" gives the depth.  Leaves first means
data-structure tests light up first; connection-lifecycle tests
last.

---

## When to add new types

* The C type catalog is in [`crate::internal`] (private structs
  internal to the library) and [`crate::lib`] (public types
  re-exported at the crate root).  These are stable; don't add
  fields unless absolutely necessary.
* If the C body needs a `static` table (e.g. cipher-suite
  registries, frame-name tables, per-version parameter blocks),
  put it in the same module as the function that consumes it,
  as a `pub(crate) const` or `static`.
* Helpers that are only used by one function: nest with `fn`
  inside that function or as a private `fn` immediately above.
  Keep the public surface lean.

---

## Conservative refactors allowed

The Phase 4 prompt forbids "rename / reshape" of existing
items.  But these refactors are explicitly allowed:

* **Splitting a long fn into helpers** when the C body has a
  natural segmentation (e.g. parse → validate → record).  Keep
  the public surface — the helpers are private.
* **Replacing a manual loop with an iterator chain** when the
  semantics are exactly preserved.
* **Replacing a manual `for` with `.copy_from_slice`,
  `.iter().sum()`, `.windows(N)`, etc.**
* **Changing local variable types** (e.g. `usize` → `u32` if the
  range fits) when the body's invariants justify it.

If a refactor crosses a public-API boundary (changing a
parameter type, adding a return value, splitting one fn into
two public ones), surface it on stdout and stop — that's a
human design call.

---

## Module map (where to translate what)

| Rust module | C source | Notes |
|---|---|---|
| `crate::siphash` | `picoquic/picohash.c` (siphash bit) | Known-answer tested.  Pure leaf. |
| `crate::splay` | `picoquic/picosplay.c` | Token-based splay tree; tests cover insert/find/delete/iter. |
| `crate::hash` | `picoquic/picohash.c` | Hash table over arena tokens. |
| `crate::arena` | (Phase 2 design — no C counterpart) | Slotmap. |
| `crate::bytestream` | `picoquic/bytestream.c` | Cursor over `&mut [u8]` plus owned-buffer helper. |
| `crate::utils` | `picoquic/picoquic_utils.c` | uint8_to_str, constant_time_memcmp, addr helpers. |
| `crate::errors` | `picoquic/picoquic_internal.h` consts | `name()` lookup tables for the two enum sets. |
| `crate::frames::FrameType::name` | `picoquic/frames.c::picoquic_frame_name` | Lookup table. |
| `crate::tp::TransportParameter::name` | `picoquic/transport.c::picoquic_tp_name` | Lookup table. |
| `crate::internal::{parse,format}_*` | `picoquic/picoquic_utils.c` | One-line bodies; mostly already translated. |
| `crate::internal::varint_*` | `picoquic/frames.c` | RFC 9000 §16. |
| `crate::internal::get_packet_number64` | `picoquic/packet.c` | RFC 9000 §A.3 reconstruction. |
| `crate::header_protection` | `picoquic/protoop_aead.c` (header bits) | RFC 9001 §5.4. |
| `crate::sys::picotls` / `::openssl` | picotls / openssl bindings | Phase 4 wires real crypto. |
| `crate::tls_api::*` | `picoquic/tls_api.c` | Master TLS context, retry tokens, app-secret rotation. |
| `crate::lb::*` | `picoquic/cid_for_lb.c` | LB CID encryption. |
| `crate::config::*` | `picoquic/quic_config.c` (sample-app uses) | Command-line option parsing. |
| `crate::binlog`, `::logger`, `::qlog`, `::textlog` | `loglib/*` | (out of v1 scope per plan; if any todo!() left, leave them) |
| `crate::packet_loop` | `picoquic/sockloop.c` | Network event loop. |
| `crate::socks*` | `picoquic/picosocks.c` | UDP socket abstraction. |
| `crate::internal::Quic` / `Connection` / `Path` impls | `picoquic/quicctx.c`, `cnx_*.c`, `paths.c`, … | The bulk: ~200 fns each on Quic / Connection. |

Recommended order (bottom-up): siphash, splay, hash, arena (no
todo!()s left there?), bytestream, utils, errors, frames, tp,
varint helpers, get_packet_number64, header_protection, then up
to lb, config, tls_api, then Connection / Path / Quic core, then
sys backends, then network event loop / socks.
