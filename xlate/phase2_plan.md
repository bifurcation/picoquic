# Phase 2 plan — dependency abstraction

This document is the single source of truth for Phase 2 (per
`TRANSLATE_PLAN.md`).  It catalogues every external dependency
the Rust port currently touches, sketches the provider /
callback traits that will replace each, and orders the
implementation work for Phase 2C.

The plan is **draftable**: Phase 2B turns it into a working
artifact through `// REVIEW: <instruction>` markers.  Phase 2C
implements what the plan dictates.  Plan drift discovered
during 2C rewrites the relevant section in the same commit so
this document stays authoritative.

## Capability inventory

Six capability boundaries are visible in the current source.
For each: what it is, the C library that provides it today,
the Rust modules that consume it, and the trait surface that
will replace it.

| # | Capability | Today | Consumed in | Status |
|---|---|---|---|---|
| 1 | TLS state machine | picotls | `tls_api`, `internal`, `crypto_provider_api` | Partially scaffolded (opaque `Ptls*` structs) |
| 2 | AEAD / PN-encrypt / hash crypto | OpenSSL or mbedtls | `tls_api`, `internal::CryptoContext` | Free functions taking `*mut c_void` (~50 sites) |
| 3 | Random source | OpenSSL `RAND_bytes` (or `/dev/urandom`) | `tls_api::public_random*`, `crypto_provider_api::CryptoRandomProvider` | Trait already drafted (`CryptoRandomProvider`) |
| 4 | Clock | application-supplied `current_time: u64` | every `pub fn` and trait method that takes `current_time` | Not yet abstracted |
| 5 | Sockets / packet I/O | platform UDP (`socks.rs`) | `socks`, `packet_loop` | Concrete `Socket(i32)` newtype; `// REVIEW(open):` notes flag the trait |
| 6 | TLS-fixture / config files | `std::fs::File` already (per Phase 1B) | `binlog`, `textlog`, `qlog`, `config` | Done — no new trait needed |

Capability 6 is no longer a Phase 2 boundary — Phase 1B replaced
the C `FILE*` handles with `std::fs::File` and the path
parameters with `&impl AsRef<Path>`.  Listed here for
completeness; no work in 2C.

---

## 1. TLS state machine

### Boundary description

picoquic's connection logic drives a TLS 1.3 handshake through
picotls.  Every per-connection TLS operation today goes through
opaque `*mut c_void` handles (`tls_master_ctx`, `tls_ctx`,
`tls_sendbuf`) plus a family of free functions in `tls_api.rs`.
The picotls structures themselves are forward-declared in
`crypto_provider_api.rs` as zero-sized `Ptls*` opaque structs.

### Evidence

| File | Symbol | Phase-1 shape |
|---|---|---|
| `internal.rs:830` | `Quic.tls_master_ctx` | `*mut c_void` |
| `internal.rs:1619` | `Connection.tls_ctx` | `*mut c_void` |
| `internal.rs:1623` | `Connection.tls_sendbuf` | `*mut c_void` |
| `tls_api.rs:246` | `tls_context_free(ctx)` | `unsafe fn(*mut c_void, bool)` |
| `tls_api.rs:213` | `Connection::create_tls_context` | `&mut self -> Result<()>` (body opaque) |
| `tls_api.rs:260` | `Connection::process_tls_stream` | `&mut self, current_time -> Result<usize>` |
| `tls_api.rs:267` | `Connection::is_tls_complete` | `&self -> bool` |
| `crypto_provider_api.rs:99-160` | `Ptls{CipherSuite,KeyExchangeAlgorithm,HpkeCipherSuite,HpkeKem,Context,SignCertificate,Ptls,RawExtension,HandshakeProperties,KeyExchangeContext}` | Forward-declared opaque structs |
| `crypto_provider_api.rs:551-589` | `TlsCtx` struct | Owns `Option<Box<Ptls>>`, `[PtlsRawExtension; 2]`, etc. |

The TLS *callbacks* (operations picotls invokes on the picoquic
side) are partly in `crypto_provider_api.rs` already, as the
`Set*KeyProvider`, `Get*FromFile`, `VerifyCertificate`,
`VerifySignature`, `GetCertificateVerifier`, `KeyexFromKeyFile`,
`KeyexDispose` traits.  Those are picotls-callback-shaped
(provider supplies, picoquic configures) and need to fold into
the Phase-2 split too.

### Provider trait sketch — `TlsStack`

Lives in `tls_api.rs`.  One trait covers the per-context
operations the TLS stack performs on behalf of picoquic.

```rust
pub trait TlsStack {
    type Connection: TlsConnection;

    /// Build a fresh per-`Quic` master context.  C:
    /// `init_master_tls_context`.
    fn create_master(&mut self, config: &MasterTlsConfig)
        -> Result<(), Error>;

    /// Open a per-connection TLS state.  C:
    /// `Connection::create_tls_context`.
    fn create_connection(&mut self, params: &ConnectionTlsParams)
        -> Result<Self::Connection, Error>;

    /// Look up a registered cipher suite.  Replaces the C
    /// `get_cipher_suite_by_id_v` / `get_aes128gcm_*_v` family.
    fn cipher_suite(&self, id: CipherSuiteId) -> Option<CipherSuiteHandle>;
}

/// Per-connection TLS state.  Replaces `Connection.tls_ctx`.
pub trait TlsConnection {
    /// Drive the handshake forward; returns bytes consumed.
    fn process_stream(&mut self, current_time: u64) -> Result<usize, Error>;

    fn is_complete(&self) -> bool;

    fn time(&self) -> u64;

    fn rotate_keys(&mut self, is_enc: bool) -> Result<(), Error>;

    /// Returns the AEAD / PN-encrypt / PN-decrypt cipher
    /// objects for the current epoch (see capability #2).
    fn current_keys(&mut self, is_enc: bool) -> KeyMaterial<'_>;

    // … etc.  Full method list in 2C; this is the surface
    // shape, not the complete API.
}
```

Dispatch: **dynamic** (`Box<dyn TlsStack>` on `Quic`).  Reason:
the v1 backend is picotls; the moment v2 wants rustls we want
to swap without recompiling, and per-`Quic` overhead is a
single vtable pointer.

### Callback trait sketch — `TlsCallbacks`

Lives in `tls_api.rs` (or `tls_callbacks.rs` if we want
separation).  picoquic implements this; the TLS backend
invokes it during the handshake.

```rust
pub trait TlsCallbacks {
    /// Negotiate ALPN from the proposed list.
    fn select_alpn(&mut self, list: &[&[u8]]) -> Option<usize>;

    /// Resumption-ticket store hook.
    fn lookup_ticket(&mut self, sni: &str, alpn: &str)
        -> Option<&[u8]>;

    /// Transport-parameters exchange (encode / decode).
    fn local_transport_parameters(&self) -> &[u8];
    fn remote_transport_parameters(&mut self, bytes: &[u8])
        -> Result<(), Error>;

    /// Certificate-verification hook (folds the existing
    /// `VerifyCertificate` and `VerifySignature` traits in).
    fn verify_certificate(&mut self, certs: &[&[u8]])
        -> Result<Box<dyn VerifySignature>, Error>;
}
```

The existing `VerifyCertificate` / `VerifySignature` /
`GetCertificateVerifier` traits in `crypto_provider_api.rs`
fold into `TlsCallbacks` (the shape was already right; just
needs co-location).  The `register_*` free functions become
methods on a `TlsBackend` builder.

### Default implementation

`tls_picotls.rs`, gated on `feature = "tls-picotls"`.  Wraps
`picotls-sys` (or hand-written FFI).  All `unsafe` blocks
that currently live in `tls_api.rs` move here; the public
`tls_api` becomes safe.

### Open questions

- Is `MasterTlsConfig` rich enough as a single struct, or do we
  need a builder?  (Decide in 2B based on call-site count.)
- `KeyMaterial<'_>`: borrowed view of the AEAD + PN-encrypt
  contexts, or owned `Box<dyn AeadCipher>`?  Borrowed avoids
  per-packet allocation; owned gives the implementor freedom to
  rotate underneath.  Probably borrowed-with-lifetime.
- The `register_*` free-function registry on `crypto_provider_api`
  predates the Phase-2 design.  Drop them in favour of the
  builder, or keep for backwards-compatible-shape with C?
  Recommendation: drop — the builder is cleaner.

---

## 2. AEAD / PN-encrypt / hash crypto

### Boundary description

picoquic uses three crypto primitives directly: AEAD ciphers
(`aead_encrypt_ticket_ctx`, `CryptoContext.aead_encrypt`/`_decrypt`),
packet-number encryption (`pn_enc`, `pn_dec`), and content
hashing (`hash_create`/`update`/`finalize`).  All three are
opaque `*mut c_void` today, with `unsafe fn` wrappers in
`tls_api.rs`.

### Evidence

| File | Symbol | Phase-1 shape |
|---|---|---|
| `internal.rs:1489-1492` | `CryptoContext.{aead_encrypt,aead_decrypt,pn_enc,pn_dec}` | `*mut c_void` |
| `internal.rs:980-981` | `Quic.aead_{encrypt,decrypt}_ticket_ctx` | `*mut c_void` |
| `internal.rs:982-983` | `Quic.retry_integrity_{sign,verify}_ctx` | `*mut *mut c_void` (one cipher per QUIC version) |
| `tls_api.rs:361` | `aead_get_checksum_length` | `unsafe fn(*mut c_void) -> usize` |
| `tls_api.rs:386` | `aead_encrypt_generic` | `unsafe fn(*mut c_void, ...)` |
| `tls_api.rs:406, 424` | `aead_decrypt_mp` / `aead_decrypt_generic` | `unsafe fn(*mut c_void, ...)` |
| `tls_api.rs:441` | `aead_encrypt_mp` | `unsafe fn(*mut c_void, ...)` |
| `tls_api.rs:452, 462` | `aead_integrity_limit` / `aead_confidentiality_limit` | `unsafe fn(*mut c_void) -> u64` |
| `tls_api.rs:473` | `aead_free` | `unsafe fn(*mut c_void)` (gone — Drop replaces) |
| `tls_api.rs:497, 513` | `pn_iv_size` / `pn_encrypt` | `unsafe fn(*mut c_void, ...)` |
| `tls_api.rs:484` | `cipher_free` | (Drop replaces) |
| `tls_api.rs:829, 848, 863` | `hash_create` / `hash_update` / `hash_finalize` | `*mut c_void` returns / params |
| `tls_api.rs:986` | `ecb_create_by_name` | returns `*mut c_void` |
| `tls_api.rs:1000` | `Aes128EcbContext.inner` | `*mut c_void` |

### Provider traits

Lives in a new `crypto.rs` module (split from `crypto_provider_api.rs`,
which keeps the picotls-specific registration glue).

```rust
pub trait AeadCipher {
    fn checksum_length(&self) -> usize;
    fn confidentiality_limit(&self) -> u64;
    fn integrity_limit(&self) -> u64;

    /// In-place encrypt: `data[..plaintext_len]` is the
    /// plaintext, `data[plaintext_len..plaintext_len + tag_len]`
    /// receives the auth tag.  `aad` is the additional data;
    /// `seq` is the packet sequence number used as nonce input.
    fn encrypt(&mut self, data: &mut [u8], plaintext_len: usize,
               aad: &[u8], seq: u64);

    /// In-place decrypt + tag check.  Returns the plaintext
    /// length on success.
    fn decrypt(&mut self, data: &mut [u8], aad: &[u8], seq: u64)
        -> Result<usize, Error>;
}

pub trait PnEncrypt {
    fn iv_size(&self) -> usize;
    /// Header-protection mask for `sample` written into `mask`.
    fn encrypt_mask(&mut self, sample: &[u8], mask: &mut [u8]);
}

pub trait Hasher {
    fn update(&mut self, input: &[u8]);
    fn finalize(self: Box<Self>, output: &mut [u8]);
    fn digest_size(&self) -> usize;
}

/// Constructor trait for the cipher / hash families.  Lives on
/// the [`TlsStack`] (the TLS layer typically owns the cipher
/// registry); a `TlsStack::cipher_suite()` returns one of these.
pub trait CipherSuite {
    fn create_aead(&self, key: &[u8], iv: &[u8]) -> Box<dyn AeadCipher>;
    fn create_pn_enc(&self, key: &[u8]) -> Box<dyn PnEncrypt>;
    fn create_hasher(&self) -> Box<dyn Hasher>;
    fn name(&self) -> &str;
    fn id(&self) -> CipherSuiteId;
}
```

Dispatch: **dynamic** (`Box<dyn AeadCipher>`).  Reason: keys
rotate, so per-epoch ciphers come and go; the per-cipher
overhead is amortized over thousands of packets.  Generic
dispatch would force `CryptoContext<A: AeadCipher, P: PnEncrypt>`
to thread two type parameters through the entire connection
state — not worth the inlining.

The `Aes128EcbContext` wrapper in `tls_api.rs` becomes a thin
`Box<dyn BlockCipher>` (where `BlockCipher` is a degenerate
`AeadCipher` without auth tags) or stays a concrete struct
backed by the AeadCipher impl — TBD in 2C based on what
`lb.rs::ConnectionIdContext` actually needs.

### Callback trait

None.  AEAD / PN-encrypt / hash are pure providers — the
crypto primitives never call back into picoquic.

### Default implementation

`crypto_openssl.rs`, gated on `feature = "crypto-openssl"`.
Wraps the `openssl` crate.  An `mbedtls` backend
(`crypto_mbedtls.rs`, `feature = "crypto-mbedtls"`) is a
secondary target; the C tree supports both.

### Open questions

- Should `AeadCipher::decrypt` take `data: &mut [u8]` and return
  the plaintext slice, or split input/output?  C does in-place;
  the Rust version probably should too (avoids a copy in the
  hot path).
- `PnEncrypt::encrypt_mask` always produces a 16-byte mask in
  picoquic; should we hard-code `[u8; 16]`?  Trade flexibility
  for a const-sized return.

---

## 3. Random source

### Boundary description

Two independent random streams: a cryptographically secure
source (used inside the TLS handshake and for connection IDs)
and a public PRNG (used for spin-bit randomization, jitter).
The C library has separate calls for each
(`uniform_random` / `public_random*`).

### Evidence

| File | Symbol | Phase-1 shape |
|---|---|---|
| `utils.rs:728` | `uniform_random(rnd_max: u64) -> u64` | secure RNG; reads `/dev/urandom` in C |
| `tls_api.rs:298` | `Connection::crypto_random(buf: &mut [u8])` | per-TLS-context CSPRNG |
| `tls_api.rs:304` | `Connection::crypto_uniform_random(rnd_max: u64) -> u64` | same family |
| `tls_api.rs:311` | `Connection::seed_public_random()` | seeds the public PRNG |
| `tls_api.rs:318-340` | `public_random_64`, `public_random_seed_64`, `public_random`, `public_uniform_random` | non-secure PRNG |
| `crypto_provider_api.rs:333` | `CryptoRandomProvider::random(&self, buf: &mut [u8])` | already a trait |

### Provider traits

```rust
/// Cryptographically secure random source.  Lives in
/// `crypto.rs` next to the cipher traits.
pub trait SecureRandom {
    fn fill(&mut self, buf: &mut [u8]);

    /// Default: `fill` 16 bytes and reduce mod `rnd_max`.
    fn uniform(&mut self, rnd_max: u64) -> u64 {
        let mut bytes = [0u8; 16];
        self.fill(&mut bytes);
        u128::from_le_bytes(bytes).rem_euclid(rnd_max as u128) as u64
    }
}

/// Non-secure deterministic-or-fast random source.  Lives in
/// `utils.rs`.
pub trait PublicRandom {
    fn fill(&mut self, buf: &mut [u8]);
    fn next_u64(&mut self) -> u64;
    fn uniform(&mut self, rnd_max: u64) -> u64;
    fn seed(&mut self, seed: u64);
}
```

Dispatch: **dynamic** (`Box<dyn SecureRandom>`).  Reason: same
as crypto — implementor freedom matters more than per-call
cost, and the call frequency is low (once per handshake, once
per CID).

### Default implementations

- `SecureRandom` → wraps the active `TlsStack`'s built-in
  CSPRNG (picotls exposes one).  Optionally a `getrandom`-backed
  default behind `feature = "std"`.
- `PublicRandom` → a small SplitMix64 implementation in `utils.rs`
  (no extra dependency).

### Callback trait

None.

### Open questions

- Should the secure RNG live on the `TlsStack` (picotls already
  exposes one and we'd avoid a second backend) or as a
  standalone trait the application can override?  Both, probably:
  `TlsStack: SecureRandom` (every TLS backend already provides
  one) and applications inject something stronger if they care.

---

## 4. Clock

### Boundary description

picoquic is virtual-time by design — every `pub fn` and trait
method that needs "now" takes a `current_time: u64`
(microseconds since process start) parameter.  This is what
makes the test simulator deterministic.  Phase 2 adds a `Clock`
trait so applications can inject a clock once at the
QUIC-context level instead of threading the parameter through
every call.

The C source threads it through every call too; the trait is a
Rust convenience, not a behavioral change.  The
`Quic.p_simulated_time: *mut u64` field (last raw pointer in
`internal.rs` outside the crypto opaque handles) is the test
simulator's way of mutating the clock; the trait makes it safe.

### Evidence

| File | Symbol | Phase-1 shape |
|---|---|---|
| `internal.rs:845` | `Quic.p_simulated_time` | `*mut u64` |
| `lib.rs:629` | `pub fn current_time() -> u64` | wall-clock helper |
| `lib.rs:1156-7` | `Config::create_and_configure(_, current_time, p_simulated_time: Option<&mut u64>)` | bootstraps clock |
| every method that takes `current_time: u64` | (~100 sites) | parameter remains for source-level fidelity |

### Provider trait

```rust
/// Application clock.  Lives in `lib.rs` (top-level capability).
pub trait Clock {
    /// Microseconds since some application-defined epoch.
    fn now(&self) -> u64;
}
```

Dispatch: **generic** at the `Quic` level (`Quic<C: Clock>`)
when feasible, **dynamic** otherwise.  A clock call costs one
indirection either way; generic gives inlining for the
production case, dynamic gives the simulator a cheap swap.
Likely outcome: `Box<dyn Clock>` on `Quic`.

### Default implementation

`std`-feature: `StdClock` returning microseconds since
`UNIX_EPOCH` (or process start).  `no_std` users supply their
own.

### Callback trait

None.

### Open questions

- Keep the per-call `current_time: u64` parameters, or pull
  them all from the clock?  Recommendation: **keep them**.
  picoquic's design is "the caller decides what time it is";
  the clock trait is for one specific path (the simulator's
  mutating clock + the production wall clock).  Removing the
  parameters would diverge from the C source unnecessarily.

---

## 5. Sockets / packet I/O

### Boundary description

picoquic is already abstract over packet I/O at the C level —
the application feeds incoming packets in via
`incoming_packet_ex` and pulls outgoing packets out via
`prepare_next_packet_ex`.  The default event loop
(`packet_loop.rs`) wraps platform UDP for the convenience of
demo apps; library consumers can supply their own loop.

What's not abstract today is `socks.rs`: it carries a concrete
`Socket(i32)` newtype and OS-specific `MessageHeader` wrapping.
Phase 2 introduces a `Socket` trait so the OS-specific bits
move into a default implementation.

### Evidence

| File | Symbol | Phase-1 shape |
|---|---|---|
| `socks.rs:74` | `pub struct Socket { fd: i32 }` | concrete fd |
| `socks.rs:82` | `pub struct ServerSockets` | `[Option<Socket>; 2]` |
| `socks.rs:94` | `pub struct MessageHeader(())` | opaque platform wrapper |
| `socks.rs:120-167` | `Socket::open_client`/`bind_to_port`/etc. | `unsafe`-adjacent OS calls |
| `// REVIEW(open):` line 67 | flags this as the Phase 2 boundary | — |

### Provider trait

```rust
/// One UDP socket on the application's network stack.  Lives in
/// `socks.rs`.
pub trait UdpSocket {
    type MessageHeader;

    fn local_address(&self) -> Result<SocketAddr, Error>;

    fn recv(&mut self, buffer: &mut [u8]) -> Result<RecvInfo, Error>;

    fn send(&mut self, dest: &SocketAddr, src: Option<&SocketAddr>,
            dest_if: i32, bytes: &[u8], gso_size: i32)
            -> Result<usize, OsError>;

    /// Configure ECN, PMTUD, packet-info options (one method per
    /// concept; defaulted to no-op for backends that don't
    /// support them).
    fn set_pkt_info(&mut self) -> Result<(), Error> { Ok(()) }
    fn set_ecn_options(&mut self) -> Result<(bool, bool), Error> { Ok((false, false)) }
    fn set_pmtud_options(&mut self) -> Result<(), Error> { Ok(()) }
}

/// Pair of UDP sockets owned by a server (typically v4 + v6).
pub struct ServerSockets<S: UdpSocket> {
    pub sockets: [Option<S>; 2],
}
```

Dispatch: **generic** (`S: UdpSocket`).  Reason: `Socket` types
are typically zero-cost wrappers around an fd; generic dispatch
inlines the I/O calls, and the type parameter doesn't infect the
core library because socket use is confined to the loop module.

### Callback trait

None.  The "callback" direction is already handled by
`PacketLoopCbFn` in `packet_loop.rs`.

### Default implementation

`socks_unix.rs`, gated on `feature = "std"` and
`#[cfg(unix)]`.  Wraps libc.  `socks_windows.rs` is v2 work
(per the v1 single-platform scope in `TRANSLATE_PLAN.md`).

### Open questions

- Should `MessageHeader` be a *separate* trait, an associated
  type on `UdpSocket`, or just a concrete struct backed by the
  active platform?  Phase 1B suggested associated type;
  reaffirm in 2B.
- Does the `select` free function fold into the trait (as
  `UdpSocket::select(sockets: &[&mut Self], ...)`) or stay
  separate?  The C version takes a slice of sockets — generic
  dispatch makes that awkward.  Probably stays free, parameterized
  by the socket type.

---

## 6. Logging output (closed)

Phase 1B already replaced the C `FILE*` log handles with
`Option<std::fs::File>` (text/binary log sinks on `Quic` /
`Connection`) and the path parameters with
`&impl AsRef<Path>`.  No further trait abstraction needed.

The `log` crate's facade (`log::info!`, etc.) is the right
choice for level-filtered events when Phase 4 lands the
bodies — it's already a de-facto standard, no_std-friendly,
and adds zero contract surface to picoquic.

---

## Cargo feature layout

```toml
[features]
default      = ["std", "tls-picotls", "crypto-openssl"]

# std feature gates std::fs / std::net / std::time::Instant.
std          = []

# TLS backends — pick one (or none, supplying your own).
tls-picotls  = ["dep:picotls-sys"]
tls-rustls   = ["dep:rustls"]                     # v2

# Crypto backends — pick one (or none).  AEAD / PN-encrypt /
# hash all come from the same backend in v1.
crypto-openssl = ["dep:openssl"]
crypto-mbedtls = ["dep:mbedtls"]                  # v1 alternate
crypto-ring  = ["dep:ring"]                       # v2

# Std-RNG default (uses `getrandom`); off implies the app
# supplies its own SecureRandom impl.
std-rng      = ["std", "dep:getrandom"]
```

The library compiles with **any one** TLS backend and **any
one** crypto backend, with the std and std-rng features
on, off, or any combination thereof.  The traits are the
contract.

### Feature combinations Phase 2C must check

- `cargo check` (default features)
- `cargo check --no-default-features --features alloc` — the
  pure-no_std build, consumers wire their own backends
- `cargo check --no-default-features --features std` — std
  available but no built-in backends
- `cargo check --no-default-features --features "std tls-picotls crypto-openssl"` — explicit single-backend permutation
- `cargo clippy -- -D warnings` (default features)

---

## Implementation order

Land capabilities leaf-first so each Phase 2C step can build on
the previous.  All steps run their relevant `cargo check`
combinations before moving on.

1. **Clock** (capability 4).  Smallest surface, no new
   dependencies, unblocks the `*mut u64 p_simulated_time`
   cleanup.  ~50 LOC of trait + std impl.
2. **PublicRandom** (part of capability 3).  Self-contained
   SplitMix64 in `utils.rs`, no external deps.
3. **AEAD / PN-encrypt / hash crypto** (capability 2).  The
   biggest single payload — replaces 35 raw pointers + 17
   `unsafe` blocks in `tls_api.rs`.  Land before TLS because
   the TLS impl will use these traits.
4. **SecureRandom** (rest of capability 3).  Folded into the
   crypto backend's interface; sits on `TlsStack`.
5. **TLS state machine** (capability 1).  Largest surface
   redesign.  Replaces `tls_master_ctx`, `tls_ctx`,
   `tls_sendbuf`, all `Ptls*` opaque structs.  Folds in the
   existing `crypto_provider_api` trait family.
6. **Sockets** (capability 5).  Independent of the TLS work;
   could land in parallel.  Replaces the concrete `Socket(i32)`
   newtype with a `UdpSocket` trait + `socks_unix.rs` default.

Total expected duration if executed sequentially: ~3-5
phase-2c iterations.

---

## Open Phase-2 questions (collected)

These bubble up from the per-capability sections and need
human resolution in 2B before 2C starts:

1. `Aes128EcbContext` — keep as a wrapper around the AEAD
   trait, or expose ECB as a separate trait?  (lb.rs needs
   one; tls_api.rs needs the other.)
2. `register_*` free-function registry on
   `crypto_provider_api.rs` — drop in favour of a builder
   pattern, or keep for source-level fidelity with the C API?
3. Per-call `current_time: u64` parameters — keep (recommended)
   or pull all of them from the clock trait?
4. `MessageHeader` shape — associated type on `UdpSocket`,
   separate trait, or concrete struct?
5. `KeyMaterial<'_>` — borrowed view of AEAD/PN contexts (lower
   per-packet cost) or owned `Box<dyn AeadCipher>` (more
   flexibility for the implementor)?
