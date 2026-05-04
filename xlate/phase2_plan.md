# Phase 2 plan — dependency abstraction (revised after 2B)

This document is the single source of truth for Phase 2 (per
`TRANSLATE_PLAN.md`).  It catalogues every external dependency
the Rust port currently touches, names the trait surface that
will replace each, and orders the implementation work for
Phase 2C.

## Phase 2B revision log

The 2A draft was rewritten end-to-end based on the human
reviewer's `// REVIEW:` markers.  Headline changes:

* **Reuse Rust Crypto traits** (`aead`, `cipher`, `digest`)
  inside backend implementations.  Don't invent parallel
  primitives.
* **Reuse the `rand` crate** (`CryptoRng`, `Rng`,
  `rand::rng()` for the default thread-local generator).
  No invented `SecureRandom` / `PublicRandom` traits.
* **`fugit::Instant<u64, 1, 1_000_000>` replaces `u64`** at
  every `current_time` site (sweeping rename across ~100
  call-sites).
* **No `Clock` trait.**  Keep per-call `current_time` parameters
  exactly as the C source has them.  An optional
  `Clock`-based wrapper that holds a clock and injects
  the time per call is fine, but it's user code, not part of
  the library API.
* **TLS-for-QUIC trait modeled on `quinn-proto::crypto::Session`**
  — same shape, but lifted into our own module (no quinn
  dependency).  The Session trait is the QUIC-facing surface;
  backend implementations wrap picotls / rustls / OpenSSL.
* **Header protection (PN encryption) is QUIC logic** — implement
  it ourselves as a struct generic over a `cipher` trait, not as
  a `PnEncrypt` trait the backend supplies.
* **Generic dispatch** (`Quic<T: TlsBackend>`) instead of `Box<dyn>`.
  Compile-time backend selection is fine.
* **One `register_callbacks(impl TlsCallbacks)` entry point**, not
  the C-style global registry of function pointers.  Trait with
  default impls.
* **`socket2::Socket` as the concrete socket impl**; the trait is
  shaped to accept it naturally.  `MessageHeader` borrowed from
  `socket2`.
* **Bonus**: ship rustls and OpenSSL backend implementations
  alongside picotls so we exercise the abstraction immediately.

## Decisions reference

For quick lookup — every Phase 2A open question now has an
answer, recorded here so 2C doesn't re-litigate them:

| #  | Question                   | Decision                                                                                                |
|----|----------------------------|---------------------------------------------------------------------------------------------------------|
| 1  | Time type                  | `fugit::Instant<u64, 1, 1_000_000>` (microsecond ticks).  Type alias `crate::Instant`.                  |
| 2  | Clock                      | No trait.  Keep per-call `current_time: Instant` parameters.                                            |
| 3  | RNG                        | Use `rand` crate directly (`CryptoRng`, `Rng`, `rand::rng()` default).  No new traits.                  |
| 4  | AEAD / hash / block cipher | Use Rust Crypto traits in backend impls; expose QUIC-specific traits (`PacketKey`, `HeaderKey`) on top. |
| 5  | Header protection          | Implemented in this crate as `HeaderProtector` over a `cipher` trait; not delegated to TLS backend.     |
| 6  | TLS-for-QUIC trait shape   | Model on `quinn-proto::crypto::{Session, ClientConfig, ServerConfig, PacketKey, HeaderKey, HmacKey}`.   |
| 7  | TLS backend dispatch       | Generic (`Quic<T: TlsBackend>`).  No `Box<dyn>` at the QUIC-context level.                              |
| 8  | TLS backends to ship       | picotls (v1 fidelity) + rustls (validates abstraction) + OpenSSL (validates abstraction).               |
| 9  | `register_*` C registry    | Replace with a single `register_callbacks(impl TlsCallbacks)` call.  Trait has default impls.           |
| 10 | `Aes128EcbContext`         | Its own type, parameterised by a `cipher::BlockEncrypt` impl from the `cipher` crate.                   |
| 11 | `MessageHeader`            | Concrete struct borrowed from `socket2`.  Not generic.                                                  |
| 12 | Socket trait               | Custom trait shaped to fit `socket2::Socket`; default impl wraps `socket2`.                             |
| 13 | `KeyMaterial`              | Owned `Vec<u8>` exported from TLS once per epoch; AEAD/HP keys instantiated separately.                 |
| 14 | `quinn-proto` dependency   | NO.  We lift the trait shape; we do not link.                                                           |

## Capability inventory

Six capability boundaries are visible in the current source.
Capability 6 (logging output) is closed by Phase 1B.

| # | Capability                      | Today                                    | Consumed in                                                            | Status                                                                  |
|---|---------------------------------|------------------------------------------|------------------------------------------------------------------------|-------------------------------------------------------------------------|
| 1 | TLS state machine               | picotls                                  | `tls_api`, `internal`, `crypto_provider_api`                           | Partially scaffolded (opaque `Ptls*` structs)                           |
| 2 | AEAD / PN-encrypt / hash crypto | OpenSSL or mbedtls                       | `tls_api`, `internal::CryptoContext`                                   | Free functions taking `*mut c_void` (~50 sites)                         |
| 3 | Random source                   | OpenSSL `RAND_bytes` (or `/dev/urandom`) | `tls_api::public_random*`, `crypto_provider_api::CryptoRandomProvider` | Trait already drafted (`CryptoRandomProvider`)                          |
| 4 | Time type                       | `u64` microseconds                       | every `pub fn` and trait method that takes `current_time`              | Not yet typed                                                           |
| 5 | Sockets / packet I/O            | platform UDP (`socks.rs`)                | `socks`, `packet_loop`                                                 | Concrete `Socket(i32)` newtype; `// REVIEW(open):` notes flag the trait |
| 6 | Logging output                  | `std::fs::File` + `core::fmt::Write`     | `binlog`, `textlog`, `qlog`, `config`                                  | Done in Phase 1B — no Phase 2 work                                      |

---

## 1. TLS state machine

### Boundary description

picoquic's connection logic drives a TLS 1.3 handshake through
picotls.  In Phase 2 we lift the entire TLS surface into a
trait family **modeled directly on
[`quinn-proto::crypto`](https://docs.rs/quinn-proto/latest/quinn_proto/crypto/index.html)**.
The shape is proven (quinn ships the rustls backend; the
community has shipped boring, ring, and aws-lc-rs backends);
we copy the trait surface into our own `tls.rs` and write
backend implementations for picotls (v1 baseline), rustls
(v1 cross-check), and OpenSSL (v1 cross-check).

Quinn dependency: **none**.  We're translating picoquic, not
linking quinn.  We borrow the trait surface only.

### Evidence

| File                             | Symbol                                                                                                                                            | Phase-1 shape                                           |
|----------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------|
| `internal.rs:830`                | `Quic.tls_master_ctx`                                                                                                                             | `*mut c_void`                                           |
| `internal.rs:1619`               | `Connection.tls_ctx`                                                                                                                              | `*mut c_void`                                           |
| `internal.rs:1623`               | `Connection.tls_sendbuf`                                                                                                                          | `*mut c_void`                                           |
| `tls_api.rs:246`                 | `tls_context_free(ctx)`                                                                                                                           | `unsafe fn(*mut c_void, bool)`                          |
| `tls_api.rs:213`                 | `Connection::create_tls_context`                                                                                                                  | `&mut self -> Result<()>`                               |
| `tls_api.rs:260`                 | `Connection::process_tls_stream`                                                                                                                  | `&mut self, current_time -> Result<usize>`              |
| `tls_api.rs:267`                 | `Connection::is_tls_complete`                                                                                                                     | `&self -> bool`                                         |
| `crypto_provider_api.rs:99-160`  | `Ptls{CipherSuite,KeyExchangeAlgorithm,HpkeCipherSuite,HpkeKem,Context,SignCertificate,Ptls,RawExtension,HandshakeProperties,KeyExchangeContext}` | Forward-declared opaque structs                         |
| `crypto_provider_api.rs:551-589` | `TlsCtx` struct                                                                                                                                   | Owns `Option<Box<Ptls>>`, `[PtlsRawExtension; 2]`, etc. |
| `crypto_provider_api.rs:362-401` | `register_*` free functions                                                                                                                       | Install callbacks into a static C-style registry        |

### Trait surface (in `crate::tls`)

Lifted from quinn-proto, simplified for picoquic's idioms.
QUIC role distinction (client vs server) is captured by the
two builder traits — a single `Session` impl handles both
roles, but the constructor differs.

```rust
//! Trait shape modeled on quinn-proto::crypto::* (no link).

/// Per-connection TLS state.  One impl per backend (picotls,
/// rustls, openssl); each backend wraps its native session
/// type.  Replaces Connection.tls_ctx and Connection.tls_sendbuf.
pub trait Session: Send {
    /// Read handshake bytes from the peer; returns `true` when
    /// the handshake transitions to a new epoch.
    fn read_handshake(&mut self, plaintext: &[u8]) -> Result<bool, Error>;

    /// Write handshake bytes for the peer; returns the new
    /// keys when an epoch transition occurs.
    fn write_handshake(&mut self, buf: &mut Vec<u8>) -> Option<Keys>;

    /// `true` until the handshake completes.
    fn is_handshaking(&self) -> bool;

    /// Negotiated 1-RTT keys after the handshake completes.
    fn next_1rtt_keys(&mut self) -> Option<KeyPair>;

    /// Server-side: the SNI / ALPN / etc. picked during the
    /// handshake.  Available after the first ClientHello.
    fn handshake_data(&self) -> Option<HandshakeData>;

    /// Peer's certificate chain (or other identity), available
    /// after the handshake completes.
    fn peer_identity(&self) -> Option<PeerIdentity>;

    /// Early-data (0-RTT) keys, if the session offers them.
    fn early_keys(&self) -> Option<(HeaderKey, PacketKey)>;

    /// Whether the server accepted the client's 0-RTT data.
    fn early_data_accepted(&self) -> Option<bool>;

    /// Decode the peer's QUIC transport parameters (TLS
    /// extension 57; RFC 9001 §8.2).
    fn transport_parameters(&self) -> Result<Option<TransportParameters>, Error>;

    /// HKDF-Expand-Label using the session's exporter master
    /// secret.  Used by QUIC for stateless-reset tokens etc.
    fn export_keying_material(
        &self,
        label: &[u8],
        context: &[u8],
        output: &mut [u8],
    ) -> Result<(), Error>;
}

/// Builder for client-side sessions.  One impl per backend.
pub trait ClientConfig {
    type Session: Session;

    fn start_session(
        &self,
        version: u32,
        server_name: &str,
        params: &TransportParameters,
    ) -> Result<Self::Session, ConfigError>;
}

/// Builder for server-side sessions.
pub trait ServerConfig {
    type Session: Session;

    fn start_session(
        &self,
        version: u32,
        params: &TransportParameters,
    ) -> Result<Self::Session, ConfigError>;
}
```

### Header protection — implemented here, not in the backend

QUIC header protection is *not* part of the TLS handshake
(it's a packet-encoding concern).  Per the reviewer's note,
we implement it ourselves over a `cipher` crate primitive:

```rust
//! In `crate::header_protection`.

pub struct HeaderProtector<C> {
    cipher: C,
}

impl<C: cipher::BlockEncryptMut + cipher::KeyInit> HeaderProtector<C>
where
    C: cipher::BlockSizeUser<BlockSize = cipher::consts::U16>,
{
    pub fn new(key: &[u8]) -> Self { … }

    /// Compute the 16-byte mask from a 16-byte sample.  QUIC
    /// uses bytes 0..5 of the result; the rest are wasted but
    /// the AES API gives us a full block.
    pub fn mask(&mut self, sample: &[u8; 16]) -> [u8; 16] { … }
}
```

For ChaCha20-based suites the type parameter is a different
concrete cipher (ChaCha20 with the sample bytes as nonce).
The `HeaderKey` struct holds an enum of the two variants:

```rust
pub enum HeaderKey {
    Aes128(HeaderProtector<aes::Aes128>),
    Aes256(HeaderProtector<aes::Aes256>),
    ChaCha20(ChaCha20HeaderProtector),
}
```

### Packet protection (AEAD) trait

Same shape as quinn's `PacketKey`: a QUIC-specific trait that
hides the underlying AEAD primitive.  Backend implementations
hold an `aead::AeadInPlace` impl from a Rust Crypto crate.

```rust
//! In `crate::tls`.

pub trait PacketKey: Send {
    /// In-place encrypt: `payload[..]` is the plaintext on
    /// entry; on success it is the ciphertext + 16-byte tag.
    fn encrypt(&self, packet: u64, header: &[u8], payload: &mut Vec<u8>);

    /// In-place decrypt + tag verify.  `payload` is shrunk by
    /// the tag length on success.
    fn decrypt(&self, packet: u64, header: &[u8], payload: &mut Vec<u8>)
        -> Result<(), Error>;

    fn tag_len(&self) -> usize;
    fn integrity_limit(&self) -> u64;
    fn confidentiality_limit(&self) -> u64;
}

/// Pair of keys for one direction (client-to-server, or vice versa).
pub struct KeyPair {
    pub local: PacketKey,
    pub remote: PacketKey,
}

pub struct Keys {
    pub header: HeaderKey,
    pub packet: KeyPair,
}
```

### TLS backends

```rust
//! Trait every backend implements at the *crate* level (not
//! per-Session).  Provides the entry points to the backend's
//! configs and any ambient state.
pub trait TlsBackend {
    type Client: ClientConfig;
    type Server: ServerConfig;

    fn client_config(&self, …) -> Result<Self::Client, ConfigError>;
    fn server_config(&self, …) -> Result<Self::Server, ConfigError>;
}
```

`Quic<T: TlsBackend>` is generic over the backend, picked at
construction time.

### Concrete implementations

| Module           | Feature gate  | Crates pulled in                                 | Status         |
|------------------|---------------|--------------------------------------------------|----------------|
| `tls_picotls.rs` | `tls-picotls` | `picotls-sys` (hand-written FFI; no crate today) | v1 baseline    |
| `tls_rustls.rs`  | `tls-rustls`  | `rustls = "0.23"`                                | v1 cross-check |
| `tls_openssl.rs` | `tls-openssl` | `openssl = "0.10"`                               | v1 cross-check |

### Application-side callbacks (`TlsCallbacks`)

Quinn's pattern for the few hooks the TLS layer needs from the
application (ALPN selection, ticket store, transport params
hand-off): a single trait the application implements.  picoquic
takes an `impl TlsCallbacks` at QUIC-context construction time;
the trait carries default implementations for everything except
the genuinely application-specific bits.

```rust
//! In `crate::tls`.

pub trait TlsCallbacks {
    /// Negotiate ALPN.  Default: pick the first proposal that
    /// matches the configured ALPN list (set on the backend's
    /// config).  Override to do custom negotiation.
    fn select_alpn(&mut self, _list: &[&[u8]]) -> Option<usize> {
        Some(0)
    }

    /// Server-side: look up a previously-issued session ticket
    /// keyed by the SNI / ALPN combination.  Default: return
    /// `None` (no resumption).
    fn lookup_ticket(&mut self, _sni: &str, _alpn: &str) -> Option<Vec<u8>> {
        None
    }

    /// Server-side: persist a freshly-issued session ticket.
    /// Default: discard (no persistence).
    fn store_ticket(&mut self, _sni: &str, _alpn: &str, _ticket: &[u8]) {}

    /// Verify the peer certificate chain.  Default: trust any
    /// valid chain rooted in the system store.  Backends that
    /// don't have a system store provide their own default.
    fn verify_certificate(&mut self, _certs: &[&[u8]]) -> Result<(), Error> {
        Ok(())
    }
}
```

The implementer is a separate adapter struct
(`TlsCallbacksImpl` or named for the application) that the
QUIC context owns.  The Connection state machine doesn't
implement it directly — keeps the responsibilities apart and
matches quinn's layering.

`register_callbacks(impl TlsCallbacks)` is the single
installation point.  No `register_tls_key_provider`,
`register_verify_certificate`, etc.  The trait's default impls
collapse the C-style multi-function registry.

### Open / deferred items

(none — all 2A questions for this section answered in 2B)

---

## 2. Crypto primitives (AEAD / block cipher / hash)

### Boundary description

picoquic uses three crypto primitives directly:

* **AEAD** (AES-128-GCM, AES-256-GCM, ChaCha20-Poly1305) —
  packet protection.
* **Block cipher** (AES-128-ECB) — header protection sample
  step, plus the load-balancer CID encryption in `lb.rs`.
* **Hash / HKDF** (SHA-256, SHA-384) — initial-secret
  derivation, key updates.

Phase 2 replaces every `*mut c_void` cipher handle with the
Rust Crypto trait family from the `aead`, `cipher`, and
`digest` crates.

### Evidence

| File                    | Symbol                                                    | Phase-1 shape                                    |
|-------------------------|-----------------------------------------------------------|--------------------------------------------------|
| `internal.rs:1489-1492` | `CryptoContext.{aead_encrypt,aead_decrypt,pn_enc,pn_dec}` | `*mut c_void`                                    |
| `internal.rs:980-981`   | `Quic.aead_{encrypt,decrypt}_ticket_ctx`                  | `*mut c_void`                                    |
| `internal.rs:982-983`   | `Quic.retry_integrity_{sign,verify}_ctx`                  | `*mut *mut c_void` (one cipher per QUIC version) |
| `tls_api.rs:361-484`    | `aead_*` family                                           | `unsafe fn(*mut c_void, ...)` (~10 functions)    |
| `tls_api.rs:497-513`    | `pn_iv_size` / `pn_encrypt`                               | `unsafe fn(*mut c_void, ...)`                    |
| `tls_api.rs:829-863`    | `hash_create` / `hash_update` / `hash_finalize`           | `*mut c_void` returns / params                   |
| `tls_api.rs:986-1000`   | `Aes128EcbContext` family                                 | `*mut c_void` inside an opaque newtype           |

### How the Rust Crypto traits map

QUIC requires AEAD with a 12-byte nonce and 16-byte tag.
The relevant Rust Crypto traits:

```rust
use aead::{AeadInPlace, AeadCore, KeyInit, KeySizeUser};
use cipher::{BlockEncryptMut, BlockSizeUser, KeyInit as CipherKeyInit};
use digest::{Digest, FixedOutput};
```

* **AEAD**: a backend's `PacketKey` impl holds an
  `impl AeadInPlace<NonceSize = U12, TagSize = U16>` — a
  concrete cipher like `aes_gcm::Aes128Gcm`,
  `aes_gcm::Aes256Gcm`, or `chacha20poly1305::ChaCha20Poly1305`.
  These all already implement the AEAD traits.
* **Block cipher** (header protection, `lb.rs` AES-ECB): a
  backend's `HeaderProtector` holds an
  `impl BlockEncryptMut<BlockSize = U16> + KeyInit` — a
  concrete cipher like `aes::Aes128`.
* **Hash**: `digest::Digest` is used directly inside HKDF
  routines; `hkdf::Hkdf<H>` from the `hkdf` crate handles
  the actual derivation.

### `Aes128EcbContext` (used by `lb.rs`)

Promoted to a thin wrapper over the `cipher` crate, replacing
the C `void*`:

```rust
//! In `crate::tls_api` (or moved to `crate::crypto::aes_ecb`).

pub struct Aes128EcbContext {
    cipher: aes::Aes128Enc,
}

impl Aes128EcbContext {
    pub fn new(key: &[u8; 16]) -> Self { … }

    pub fn encrypt(&self, output: &mut [u8; 16], input: &[u8; 16]) { … }
}
```

For the load-balancer CID-encryption path the constructor
also accepts a decrypt-mode AES (or just exposes a separate
`Aes128EcbDecryptContext`).  Both are concrete, no trait.

### Default crypto backend

The Rust Crypto family is already pure-Rust and `no_std`-clean.
No need for an OpenSSL or mbedtls backend in v1 — the picotls
backend (capability 1) supplies its own AEAD implementations
via picotls's built-ins, and the rustls backend uses
`*ring*`/`aws-lc-rs` which already wrap the AEAD traits.

So for capability 2 the work is purely in the
`tls_picotls.rs` / `tls_rustls.rs` / `tls_openssl.rs` modules
(adapting their cipher contexts to the `PacketKey` trait
defined in capability 1).  No standalone "crypto backend"
module is needed.

### Open / deferred items

(none)

---

## 3. Random source

### Boundary description

picoquic uses two random streams: secure (handshake nonces,
connection IDs) and non-secure (spin-bit, jitter).  Phase 2
replaces both with the `rand` crate's trait family; no
invented traits.

### Evidence

| File                         | Symbol                                                                                | Phase-1 shape                         |
|------------------------------|---------------------------------------------------------------------------------------|---------------------------------------|
| `utils.rs:728`               | `uniform_random(rnd_max: u64) -> u64`                                                 | secure RNG; reads `/dev/urandom` in C |
| `tls_api.rs:298`             | `Connection::crypto_random(buf: &mut [u8])`                                           | per-TLS-context CSPRNG                |
| `tls_api.rs:304`             | `Connection::crypto_uniform_random(rnd_max: u64) -> u64`                              | same family                           |
| `tls_api.rs:311`             | `Connection::seed_public_random()`                                                    | seeds the public PRNG                 |
| `tls_api.rs:318-340`         | `public_random_64`, `public_random_seed_64`, `public_random`, `public_uniform_random` | non-secure PRNG                       |
| `crypto_provider_api.rs:333` | `CryptoRandomProvider::random(&self, buf: &mut [u8])`                                 | will be deleted (replaced by `rand`)  |

### `rand` usage

```rust
use rand::{CryptoRng, Rng, RngCore};

// Secure RNG: anywhere we need cryptographic randomness.
fn fill_random(rng: &mut impl CryptoRng) {
    let mut nonce = [0u8; 12];
    rng.fill_bytes(&mut nonce);
}

// Non-secure RNG: spin-bit, jitter, etc.
fn pick_jitter(rng: &mut impl Rng) -> u32 {
    rng.gen_range(0..1000)
}

// Default secure RNG: the OS thread-local generator.
fn make_default_rng() -> rand::rngs::ThreadRng {
    rand::rng()
}
```

### Plumbing

* The QUIC `Quic` context holds an `impl CryptoRng + RngCore`
  injected at construction time (default: `rand::rng()`).
* The non-secure stream uses `rand::rng()` too — the
  thread-local generator is fast and correctly seeded.
* The deterministic-test RNG (already moved to
  `tests/util.rs`) keeps its bespoke 64-bit-context API
  (`test_random` etc.) because tests want exact reproducibility,
  not the thread-local.

### Existing `CryptoRandomProvider` trait

Delete it.  The `rand` ecosystem covers everything it offered.

### Deferred / open items

(none)

---

## 4. Time type

### Boundary description

picoquic threads `current_time: u64` (microseconds since some
application-defined epoch) through every call that needs
"now".  Phase 2 replaces the raw `u64` with a typed
`fugit::Instant`.

There is **no `Clock` trait**.  The C source's design — the
caller decides what time it is — is preserved exactly.  An
optional Clock-based wrapper (user code, not library code)
can read a clock and inject the time per call.

### Evidence

| File                                        | Symbol                                                                              | Phase-1 shape                               |
|---------------------------------------------|-------------------------------------------------------------------------------------|---------------------------------------------|
| `internal.rs:845`                           | `Quic.p_simulated_time`                                                             | `*mut u64`                                  |
| `lib.rs:629`                                | `pub fn current_time() -> u64`                                                      | wall-clock helper                           |
| `lib.rs:1156-7`                             | `Config::create_and_configure(_, current_time, p_simulated_time: Option<&mut u64>)` | bootstraps clock                            |
| every method that takes `current_time: u64` | (~100 sites)                                                                        | parameter remains for source-level fidelity |

### Type alias

```rust
//! In `crate` (lib.rs).

pub type Instant = fugit::Instant<u64, 1, 1_000_000>;
pub type Duration = fugit::Duration<u64, 1, 1_000_000>;
```

Microsecond ticks (numerator 1, denominator 1_000_000), `u64`
storage.  `fugit::Instant` is `Copy`, supports arithmetic with
`fugit::Duration`, and serialises to/from `u64` cheaply when
crossing FFI / wire boundaries.

### Sweep

Every `current_time: u64` parameter (~100 sites) becomes
`current_time: crate::Instant`.  `Quic.p_simulated_time:
*mut u64` becomes — wait, this is gone.  The simulator
mutates the application's clock (whatever it is); the QUIC
context doesn't need to hold a pointer at all.  The
`Config::create_and_configure(_, current_time,
p_simulated_time: Option<&mut u64>)` constructor signature
collapses to just `(_, current_time)` — the simulator's
clock-mutation happens entirely on the application side, and
the test simulator's `current_time` flows in via the per-call
parameters that already exist.

### `pub fn current_time() -> u64` helper

Stays, retyped: `pub fn current_time() -> Instant`.  Reads
`std::time::Instant::now()` internally and converts to
microseconds.  `std`-feature-gated.

### Cargo dependency

```toml
fugit = { version = "0.3", default-features = false }
```

`no_std`-clean.

### Deferred / open items

(none)

---

## 5. Sockets / packet I/O

### Boundary description

picoquic's `socks.rs` carries a concrete `Socket(i32)`
newtype.  Phase 2 introduces a `Socket` trait shaped to fit
[`socket2::Socket`](https://docs.rs/socket2/latest/socket2/struct.Socket.html);
the default implementation wraps `socket2`.  `MessageHeader`
becomes the concrete struct from `socket2` (or a thin newtype
around it).

### Evidence

| File               | Symbol                                    | Phase-1 shape              |
|--------------------|-------------------------------------------|----------------------------|
| `socks.rs:74`      | `pub struct Socket { fd: i32 }`           | concrete fd                |
| `socks.rs:82`      | `pub struct ServerSockets`                | `[Option<Socket>; 2]`      |
| `socks.rs:94`      | `pub struct MessageHeader(())`            | opaque platform wrapper    |
| `socks.rs:120-167` | `Socket::open_client`/`bind_to_port`/etc. | `unsafe`-adjacent OS calls |

### Socket trait (shaped to fit `socket2::Socket`)

```rust
//! In `crate::socks`.

pub trait Socket {
    fn local_address(&self) -> Result<SocketAddr, Error>;

    fn recv(&mut self, buffer: &mut [u8]) -> Result<RecvInfo, Error>;

    fn send(
        &mut self,
        dest: &SocketAddr,
        src: Option<&SocketAddr>,
        dest_if: i32,
        bytes: &[u8],
        gso_size: i32,
    ) -> Result<usize, OsError>;

    /// Configure ECN, PMTUD, packet-info options.  Defaulted
    /// to no-op for backends that don't support them
    /// (matches the trait's reception over diverse
    /// platforms).
    fn set_pkt_info(&mut self) -> Result<(), Error> { Ok(()) }
    fn set_ecn_options(&mut self) -> Result<(bool, bool), Error> { Ok((false, false)) }
    fn set_pmtud_options(&mut self) -> Result<(), Error> { Ok(()) }
}

/// Concrete struct.  Borrowed from socket2.
pub type MessageHeader = socket2::MsgHdr<'static, 'static, 'static>;

/// Pair of UDP sockets owned by a server (typically v4 + v6).
pub struct ServerSockets<S: Socket> {
    pub sockets: [Option<S>; 2],
}
```

### Default implementation

```rust
//! In `crate::socks_socket2` (gated on `feature = "std"`).

impl Socket for socket2::Socket { … }
```

Generic dispatch (`S: Socket` parameter on `ServerSockets`,
`Quic`'s loop, etc.).  The type parameter is confined to the
loop module (most of the library doesn't see sockets at all).

### Cargo dependency

```toml
socket2 = { version = "0.5", optional = true, features = ["all"] }
```

Only pulled in when the `std` feature is on.

### Deferred / open items

(none)

---

## 6. Logging output (closed)

Phase 1B already replaced the C `FILE*` log handles with
`Option<std::fs::File>` (text/binary log sinks on `Quic` /
`Connection`) and the path parameters with
`&impl AsRef<Path>`.  No further trait abstraction needed.

The `log` crate's facade (`log::info!`, etc.) is the right
choice for level-filtered events when Phase 4 lands the
bodies — already a de-facto standard, no_std-friendly, zero
contract surface.

```toml
log = { version = "0.4", default-features = false }
```

---

## Cargo feature layout

```toml
[dependencies]

# --- Always on ---
fugit       = { version = "0.3",  default-features = false }
log         = { version = "0.4",  default-features = false }

# Rust Crypto trait crates (no implementations — those come from backends).
aead        = { version = "0.5",  default-features = false }
cipher      = { version = "0.4",  default-features = false }
digest      = { version = "0.10", default-features = false }
hkdf        = { version = "0.12", default-features = false }

# Random.
rand        = { version = "0.9",  default-features = false }
rand_core   = { version = "0.9",  default-features = false }

# Concrete cipher impls used by header protection.
aes         = { version = "0.8",  default-features = false }
chacha20    = { version = "0.9",  default-features = false }

# --- Optional, std-only ---
socket2     = { version = "0.5",  default-features = false, optional = true }
getrandom   = { version = "0.2",  default-features = false, optional = true }

# --- TLS backends (one of) ---
# picotls-sys is hand-written FFI; lives in this repo, not crates.io.
rustls      = { version = "0.23", default-features = false, optional = true }
openssl     = { version = "0.10",                            optional = true }

[features]
default        = ["std", "tls-picotls"]

# std gates std::fs / std::net / std::time / socket2 / getrandom.
std            = ["dep:socket2", "dep:getrandom"]

# TLS backends.  Pick one (or none, supplying your own).
tls-picotls    = []                       # uses in-repo FFI module
tls-rustls     = ["dep:rustls", "std"]    # rustls is std-only
tls-openssl    = ["dep:openssl", "std"]
```

The library compiles with **any one** TLS backend, with the
`std` feature on or off.  AEAD/block-cipher/hash impls come
from each TLS backend's own crypto layer (picotls's built-ins,
rustls via aws-lc-rs / ring, OpenSSL via openssl-sys).

### Feature combinations Phase 2C must check

* `cargo check` (default features)
* `cargo check --no-default-features` — pure-no_std, consumer
  brings TLS and sockets
* `cargo check --no-default-features --features std` — std
  available but no TLS backend
* `cargo check --features tls-rustls --no-default-features --features std,tls-rustls`
* `cargo check --features tls-openssl --no-default-features --features std,tls-openssl`
* `cargo check` (all default features incl. tls-picotls)
* `cargo clippy -- -D warnings` for default features
* `cargo test --no-run` for default features

---

## Implementation order

Land capabilities leaf-first so each Phase 2C step can build
on the previous.  Each step runs the relevant `cargo check`
combinations before moving on.

1. **Time type** (capability 4).  Single-day sweep:
   `pub type Instant = fugit::Instant<u64, 1, 1_000_000>;` plus
   ~100 `u64` → `Instant` parameter renames.  Drops
   `Quic.p_simulated_time` and the `Option<&mut u64>`
   constructor parameter.  Smallest payload, biggest
   call-site count.
2. **Random** (capability 3).  Add `rand` to `Cargo.toml`,
   delete `CryptoRandomProvider`, retype every
   `*_random(...)` site to take `&mut impl CryptoRng` or
   `&mut impl Rng`.  ~30 sites.
3. **Crypto primitives** (capability 2).  Define `PacketKey`,
   `HeaderKey`, `KeyPair`, `Keys` traits/structs in
   `crate::tls`.  Define `HeaderProtector<C>` in
   `crate::header_protection`.  Promote `Aes128EcbContext`
   to a real `aes::Aes128Enc` wrapper.  Replaces 30 `*mut
   c_void` fields and 17 `unsafe` blocks in `tls_api.rs`.
4. **TLS state machine** (capability 1).  The big payload.
   Define the `Session`, `ClientConfig`, `ServerConfig`,
   `TlsBackend` trait family in `crate::tls`.  Write
   `tls_picotls.rs` (the v1 baseline backend; FFI to picotls).
   Then `tls_rustls.rs` and `tls_openssl.rs` to validate the
   abstraction.  Replaces `tls_master_ctx`, `tls_ctx`,
   `tls_sendbuf`, all `Ptls*` opaque structs, and the
   `register_*` registry.
5. **Sockets** (capability 5).  Independent of the TLS work
   — could land in parallel with #4.  Define `Socket` trait
   shaped for `socket2::Socket`; default impl in
   `socks_socket2.rs` (gated on `std`).  Replaces the
   concrete `Socket(i32)` newtype.

Total expected duration: ~5-7 phase-2c iterations, with #1
and #2 done in a single iteration each, and #4 spread across
a few iterations (one per TLS backend).
