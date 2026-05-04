//! Translation of `quic/tls_api.h`.
//!
//! `tls_api.h` is the boundary between quic-core's QUIC machinery
//! and the tls TLS 1.3 stack underneath.  It exposes the master
//! TLS context lifecycle, per-connection TLS contexts, the AEAD /
//! cipher / hash primitives quic uses internally, the public and
//! crypto random generators, the retry-token / retry-protection
//! helpers, and a handful of provider-installation entry points kept
//! in this header so applications don't have to include `tls.h`.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//! Phase 3 fills in the bodies.
//!
//! ## Shape conventions
//!
//! * Free C functions whose first argument is a `Quic*` or `Connection*`
//!   become inherent methods on [`Quic`] / [`Connection`].  The remaining
//!   free functions either operate on opaque tls handles
//!   (`*mut c_void` — see below) or have no obvious receiver
//!   (global RNG, hash factories, cipher-suite lookups, the
//!   `tls_api_init` lifecycle).
//! * Tls types this header references but does not define
//!   (`PtlsCipherSuite`, …) are
//!   already forward-declared in [`crate::crypto_provider_api`] and
//!   [`crate`]; this module re-uses those declarations rather than
//!   duplicating them.
//! * The `void*` AEAD / PN-encryption / cipher / hash contexts on the
//!   C side are tls handles whose Rust binding doesn't exist yet
//!   (tls is an external dependency that has not been translated).
//!   They stay as `*mut c_void` opaque handles in Phase 1; Phase 3
//!   will replace them with proper trait objects or forward-declared
//!   structs once tls is bound.  Functions that dereference such a
//!   handle are `unsafe` with a `# Safety` doc note describing the
//!   caller's obligations.
//! * `int` flag parameters that encode booleans (`is_enc`,
//!   `is_client`, `client_mode`, `use_low_memory`, `reset`,
//!   `check_reuse`, `sending`) become `bool`.
//! * C status-code returns (`int` 0/-1 / `ERROR_*`) become
//!   `Result<T, Error>`.  The `SIZE_MAX` failure sentinel returned
//!   by `aead_decrypt_*` becomes `Err(Error::AeadCheck)` instead of
//!   leaking the sentinel through the success arm.
//! * Owning `PtlsIovec* get_certs_from_file(…, size_t* count)`
//!   collapses to `Option<Vec<Vec<u8>>>`: the C callee `malloc`s
//!   the slot array and writes its length through the out-pointer,
//!   and the caller `free`s.  A Rust `Vec` carries both ownership
//!   and length, with `Drop` taking the place of the manual free.
//! * Output parameters (`uint8_t reset_secret[16]`,
//!   `uint8_t* master_secret`, `void** aead_ctx`, `size_t*
//!   text_length`, `int* data_consumed`, `int* is_new_token`,
//!   `ConnectionId* odcid`, `size_t* token_size`, `size_t* length`)
//!   become `&mut`-borrowed slots or are returned as part of the
//!   function's result tuple, depending on whether the caller
//!   already holds storage.
//! * `struct sockaddr*` arguments map to [`core::net::SocketAddr`]
//!   for parity with `quic.rs`.
//! * `FreeVerifyCertificateCtx` is a function-pointer typedef in
//!   `quic.h`; the trait counterpart is already in `quic.rs` and is
//!   reused here.
//! * Function-pointer parameters in callbacks
//!   ([`Quic::tls_set_verify_certificate_callback`]) take
//!   `Option<Box<dyn …>>` — the C `NULL` sentinel maps to `None`
//!   and the registered callback is owned by the registry.
//!
//! ## Initial-secret label constants
//!
//! `LABEL_QUIC_BASE` is `#define`d as `NULL` in C — it's the "no
//! prefix label" sentinel passed to tls.  In Rust the prefix-label
//! parameters that accept it become `Option<&str>`, so the constant
//! itself doesn't appear; the call sites pass `None` when they would
//! have passed `LABEL_QUIC_BASE`.
//!
//! ## Public vs. internal `set_*` methods
//!
//! `picoquic.h` exposes `picoquic_set_verify_certificate_callback`,
//! `picoquic_set_client_authentication`, and
//! `picoquic_set_use_exporter` as thin wrappers around the
//! `picoquic_tls_set_*` functions declared here.  The wrappers live
//! on [`Quic`] without the `tls_` prefix (see `lib.rs`); the
//! TLS-side methods in this module keep the `tls_` prefix so both
//! pairs can coexist.
//!
//! ## Out of scope
//!
//! * The five `#if 0`-disabled `cid_*_under_mask_ctx` /
//!   `cid_free_encrypt_global_ctx` entries are dead code in the C
//!   source and are not translated.
//! * `get_private_key_from_file` is also `#if 0` in the header (and
//!   the `_t` callback variant lives in `crypto_provider_api.rs`);
//!   not translated.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::net::SocketAddr;

use crate::Instant;
use crate::crypto_provider_api::VerifyCertificate;
use crate::internal::CryptoContext;
use crate::{Connection, ConnectionId, Error, Quic, RESET_SECRET_SIZE};

// ---------------------------------------------------------------------------
// Label constants (C `#define` → `&str`).
//
// All labels are NUL-terminated bytes in the C header because tls
// takes them as `char const*`.  The Rust shape is plain `&str`; the
// trailing NUL is reintroduced (if needed) at the tls FFI
// boundary.

/// Initial secret label for the client direction.  C:
/// `LABEL_INITIAL_CLIENT`.
pub const LABEL_INITIAL_CLIENT: &str = "client in";

/// Initial secret label for the server direction.  C:
/// `LABEL_INITIAL_SERVER`.
pub const LABEL_INITIAL_SERVER: &str = "server in";

/// Traffic-update label for QUIC v1.  C:
/// `LABEL_V1_TRAFFIC_UPDATE`.
pub const LABEL_V1_TRAFFIC_UPDATE: &str = "quic ku";

/// Traffic-update label for QUIC v2.  C:
/// `LABEL_V2_TRAFFIC_UPDATE`.
pub const LABEL_V2_TRAFFIC_UPDATE: &str = "quicv2 ku";

/// HKDF "key" component label.  C: `LABEL_KEY`.
pub const LABEL_KEY: &str = "key";

/// HKDF "iv" component label.  C: `LABEL_IV`.
pub const LABEL_IV: &str = "iv";

/// HKDF "hp" (header protection) component label.  C:
/// `LABEL_HP`.
pub const LABEL_HP: &str = "hp";

/// HKDF "cid" component label.  C: `LABEL_CID`.
pub const LABEL_CID: &str = "cid";

/// HKDF label for the global CID encryption secret.  C:
/// `LABEL_CID_GLOBAL`.
pub const LABEL_CID_GLOBAL: &str = "cid global";

/// Number of HKDF rounds run when deriving the global CID secret.
/// C: `LABEL_CID_GLOBAL_ROUNDS`.
pub const LABEL_CID_GLOBAL_ROUNDS: u32 = 4;

/// QUIC v1 TLS key-derivation label prefix.  C:
/// `LABEL_QUIC_V1_KEY_BASE`.
pub const LABEL_QUIC_V1_KEY_BASE: &str = "tls13 quic ";

/// QUIC v2 TLS key-derivation label prefix.  C:
/// `LABEL_QUIC_V2_KEY_BASE`.
pub const LABEL_QUIC_V2_KEY_BASE: &str = "tls13 quicv2 ";

// ---------------------------------------------------------------------------
// Forward declaration of `PtlsCipherSuite` (C: `ptls_cipher_suite_t`).
//
// The C header re-typedef-s `ptls_cipher_suite_t` as
// `const struct st_ptls_cipher_suite_t` so consumers don't need
// `tls.h`.  tls itself has not been translated; this opaque
// stand-in keeps signatures compiling.  Phase 3 will swap in the
// real tls binding once it exists.
//
// Note: `crypto_provider_api` already declares an identical
// placeholder.  Pull it in for use within this module's signatures
// (no re-export).
use crate::crypto_provider_api::PtlsCipherSuite;

// ---------------------------------------------------------------------------
// Master TLS context.
//
// The master context lives in `quic.tls_master_ctx` (a `*mut c_void`
// once cast from `ptls_context_t*`).  Both lifecycle entry points
// install or release state on the QUIC context itself.

impl Quic {
    /// Initialize this context's master TLS context.  Loads the
    /// certificate chain, the private key, the trusted-root bundle
    /// and the ticket-encryption key.  Any of the file-name / key
    /// parameters may be `NULL` in C (server-only, no roots, no
    /// ticket key); Rust models that with `Option<&str>` /
    /// `Option<&[u8]>`.  C: `master_tlscontext`.
    pub fn init_master_tls_context(
        &mut self,
        _cert_file_name: Option<&str>,
        _key_file_name: Option<&str>,
        _cert_root_file_name: Option<&str>,
        _ticket_key: Option<&[u8]>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Tear down the master TLS context installed by
    /// [`Quic::init_master_tls_context`].  C:
    /// `master_tlscontext_free`.
    pub fn free_master_tls_context(&mut self) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Per-connection TLS context.
//
// `tls_ctx_t` itself lives in `crypto_provider_api.rs`; this module
// just exposes the API surface that quic-core uses to drive it.

impl Connection {
    /// Allocate a per-connection TLS context, attach it to this
    /// connection, and initialize the tls handshake-property slots.
    /// C: `tlscontext_create`.  C side returns 0 on success,
    /// `ERROR_TLS_SERVER_CON_WITHOUT_CERT` / `ERROR_MEMORY` / -1 on
    /// failure.
    pub fn create_tls_context(&mut self, _quic: &mut Quic) -> Result<(), Error> {
        todo!()
    }

    /// Drop transient buffers (ALPN list, transport-parameter encode
    /// scratch) once the handshake is done.  C:
    /// `tlscontext_trim_after_handshake`.
    pub fn trim_tls_context_after_handshake(&mut self) {
        todo!()
    }

    /// Forget the session ticket installed for a 0-RTT attempt.  C:
    /// `tlscontext_remove_ticket`.
    pub fn remove_tls_ticket(&mut self) {
        todo!()
    }
}

// `tls_context_free` is gone -- `Connection.tls_ctx` is now an
// `Option<Box<dyn Session>>`, and `Drop` on the boxed Session
// runs the backend's teardown automatically.  The C
// `client_mode` flag is unnecessary too: the `Session` impl
// knows whether it's a client or server session internally.

// ---------------------------------------------------------------------------
// TLS stream processing.

impl Connection {
    /// Drive the TLS handshake by feeding in any data buffered on
    /// the crypto streams and pushing produced data back out.
    /// Returns the number of bytes consumed (the C `int*
    /// data_consumed` out-parameter; bytes consumed is non-negative,
    /// so the type widens to `usize` rather than tracking the C
    /// `int`).  C: `tls_stream_process`.
    pub fn process_tls_stream(&mut self, _current_time: Instant) -> Result<usize, Error> {
        todo!()
    }

    /// Report whether the TLS handshake has completed.  C signature
    /// returned `int` (0/1); promoted to `bool`.  C:
    /// `is_tls_complete`.
    pub fn is_tls_complete(&self) -> bool {
        todo!()
    }

    /// Send the initial `ClientHello` (or the response to a
    /// `HelloRetry`) on the TLS stream.  C:
    /// `initialize_tls_stream`.
    pub fn initialize_tls_stream(&mut self, _current_time: Instant) -> Result<(), Error> {
        todo!()
    }
}

impl Quic {
    /// Read the virtual time tls sees through its `get_time`
    /// callback (microseconds).  C: `get_tls_time`.
    pub fn tls_time(&self) -> u64 {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Random number generation.
//
// The crypto-grade RNG is the tls / OpenSSL provider; the public RNG
// is xorshift1024* seeded from the crypto RNG.  Both take a
// destination slice; the C `(void* buf, size_t len)` pair collapses
// to `&mut [u8]`.

// `crypto_random` / `crypto_uniform_random` / `seed_public_random`
// are gone -- callers reach `Quic.rng` (a `Box<dyn CryptoRng>`)
// directly and use the `rand::Rng` / `rand::RngCore` methods on
// it.  The public-RNG helpers (`public_random_64` etc.) likewise
// disappear: callers use `rand::rng()` for the thread-local
// non-secure stream.

// ---------------------------------------------------------------------------
// AEAD primitives.
//
// All AEAD entry points take an opaque `void*` AEAD handle in C
// (really a `ptls_aead_context_t*`).  Phase 1 keeps the opaque
// shape as `*mut c_void`; Phase 3 will swap in a forward-declared
// tls struct or trait object once that binding lands.  Each
// `aead_*` body is a thin wrapper around the corresponding
// `ptls_aead_*` call, so the callee mutates state through the
// pointer (e.g. AEAD nonce) — the Rust shape uses `&mut` to capture
// that.

/// Length of the QUIC AEAD authentication tag.  All TLS 1.3
/// cipher suites used by QUIC (RFC 9001 §5.3) produce a 16-byte
/// tag, so the value is fixed.  Replaces the C
/// `aead_get_checksum_length(void*)` accessor; once a
/// [`crate::tls::PacketKey`] exists, prefer `key.tag_len()`.
pub const QUIC_AEAD_TAG_LEN: usize = 16;

// The AEAD wrappers (`aead_encrypt_generic`, `aead_decrypt_generic`,
// `aead_encrypt_mp` / `aead_decrypt_mp`, `aead_integrity_limit`,
// `aead_confidentiality_limit`) are gone -- their behaviour is the
// `crate::tls::PacketKey` trait.  Backends supply concrete
// implementations.  Multipath variants fold into the same trait by
// XORing the path id into the nonce inside the implementor.
//
// `aead_free` and `cipher_free` are gone -- `Drop` on the boxed
// trait object replaces them.
//
// `pn_iv_size` and `pn_encrypt` are gone -- header protection
// (the only consumer) is implemented in `crate::header_protection`
// and exposed through `crate::tls::HeaderKey`.

// ---------------------------------------------------------------------------
// Initial-secret derivation.
//
// `setup_initial_master_secret` and
// `setup_initial_secrets` write into caller-provided
// buffers sized at `cipher->hash->digest_size`.  Phase 1 keeps the
// `&mut [u8]` shape for the buffers so the caller controls
// allocation; Phase 3 may switch to fixed-size arrays once the
// digest size is part of the cipher trait.

/// Derive the per-connection-ID initial master secret.  C:
/// `setup_initial_master_secret`.
pub fn setup_initial_master_secret(
    _cipher: &PtlsCipherSuite,
    _salt: &[u8],
    _initial_connection_id: ConnectionId,
    _master_secret: &mut [u8],
) -> Result<(), Error> {
    todo!()
}

/// Derive client/server initial secrets from the master secret.
/// `client_secret` and `server_secret` are filled in place.  C:
/// `setup_initial_secrets`.
pub fn setup_initial_secrets(
    _cipher: &PtlsCipherSuite,
    _master_secret: &[u8],
    _client_secret: &mut [u8],
    _server_secret: &mut [u8],
) -> Result<(), Error> {
    todo!()
}

impl Connection {
    /// Set up this connection's per-epoch initial AEAD / PN
    /// encryption contexts from the connection's initial CID.  C:
    /// `setup_initial_traffic_keys`.
    pub fn setup_initial_traffic_keys(&mut self) -> Result<(), Error> {
        todo!()
    }
}

/// Output bundle from [`Quic::initial_aead_context`].  C returned
/// the AEAD and PN contexts through `void**` out-pointers; the Rust
/// shape collapses both into a struct so the function signature is
/// one-out, one-return.
pub struct InitialAeadContext {
    /// Owned AEAD context for packet protection.
    pub aead_ctx: Box<dyn crate::tls::PacketKey>,
    /// Owned header-protection context.
    pub pn_enc_ctx: Box<dyn crate::tls::HeaderKey>,
}

impl Quic {
    /// Derive an AEAD + PN context for the initial encryption
    /// level.  `version_index` selects the QUIC version's salt and
    /// prefix label; `is_client` and `is_enc` pick the
    /// client/server and encrypt/decrypt directions.  C:
    /// `get_initial_aead_context`.
    pub fn initial_aead_context(
        &mut self,
        _version_index: i32,
        _initial_connection_id: &ConnectionId,
        _is_client: bool,
        _is_enc: bool,
    ) -> Result<InitialAeadContext, Error> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Application-secret access and rotation.
//
// `get_app_secret` returns one of the two
// `app_secret_*` buffers stashed inside `tls_ctx_t`.  The
// C signature returned a `uint8_t*` into that buffer; in Rust the
// natural shape is a borrowed slice carrying the digest length
// (callers consult `get_app_secret_size` for the length
// today, which is redundant once the borrow encodes it).  Phase 3
// may collapse the size accessor.

impl Connection {
    /// Return a borrow of the app-data traffic secret stored in
    /// this connection's TLS context for the chosen direction.  C:
    /// `get_app_secret`.
    pub fn app_secret(&mut self, _is_enc: bool) -> &mut [u8] {
        todo!()
    }

    /// Length (bytes) of the app-data traffic secret — the digest
    /// size of the negotiated cipher's hash.  C:
    /// `get_app_secret_size`.  Phase 3 may collapse this accessor
    /// since [`Connection::app_secret`] already returns a sized slice.
    pub fn app_secret_size(&self) -> usize {
        todo!()
    }

    /// Compute the post-rotation AEAD + PN contexts and stash them
    /// in `crypto_context_new`.  C: `compute_new_rotated_keys`.
    pub fn compute_new_rotated_keys(&mut self) -> Result<(), Error> {
        todo!()
    }

    /// Promote `crypto_context_new` to the active
    /// `crypto_context[3]` slot, demoting the previous keys.  C:
    /// `apply_rotated_keys`.
    pub fn apply_rotated_keys(&mut self, _is_enc: bool) {
        todo!()
    }
}

/// Rotate the application traffic secret in place using the
/// version-specific traffic-update label.  C:
/// `rotate_app_secret`.
pub fn rotate_app_secret(
    _cipher: &PtlsCipherSuite,
    _secret: &mut [u8],
    _traffic_update_label: &str,
) -> Result<(), Error> {
    todo!()
}

impl CryptoContext {
    /// Free every AEAD / PN-encryption slot held by this crypto
    /// context.  Called from key-rotation paths to recycle the
    /// underlying tls handles without dropping the parent
    /// [`Connection`], so this is *not* an `impl Drop` — Phase 3 may
    /// still install one for the parent-drop path.  C:
    /// `crypto_context_free`.
    pub fn free_handles(&mut self) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Test helpers (still part of the public API surface).
//
// These two helpers build standalone AEAD / PN-encryption contexts
// from a raw secret.  They are used by the test suite to mock
// crypto state.  The C return type is `void*` — Phase 1 keeps it
// as `*mut c_void` for parity with the rest of the AEAD plumbing.

/// Construct an AEAD context backed by AES128-GCM-SHA256 from a
/// raw secret + prefix label.  Returns null on allocation failure
/// (matching the C `void*`).  C: `setup_test_aead_context`.
pub fn setup_test_aead_context(
    _is_encrypt: bool,
    _secret: &[u8],
    _prefix_label: &str,
) -> Option<Box<dyn crate::tls::PacketKey>> {
    todo!()
}

/// Construct a PN-encryption context for tests.  C:
/// `pn_enc_create_for_test`.
pub fn pn_enc_create_for_test(
    _secret: &[u8],
    _prefix_label: &str,
) -> Option<Box<dyn crate::tls::HeaderKey>> {
    todo!()
}

// ---------------------------------------------------------------------------
// Reset secret and verify-certificate management.

impl Quic {
    /// Compute the 16-byte reset secret tied to `connection_id` using this
    /// context's reset seed.  C: `create_connection_id_reset_secret`.
    pub fn create_connection_id_reset_secret(
        &mut self,
        _cnx_id: &ConnectionId,
        _reset_secret: &mut [u8; RESET_SECRET_SIZE],
    ) -> Result<(), Error> {
        todo!()
    }

    /// Install a custom certificate-verification callback into the
    /// master TLS context, replacing any previously installed one.
    /// C: `tls_set_verify_certificate_callback`.
    ///
    /// `cb` is owned by the registry; take it by value.  `free_fn`
    /// runs when the verifier is replaced or the master context is
    /// freed; `None` matches the C `NULL` "no teardown hook"
    /// sentinel.  `Box<dyn …>` for `free_fn` is required (you can't
    /// own a `dyn Trait` any other way).
    ///
    /// The public-API wrapper [`Quic::set_verify_certificate_callback`]
    /// (declared in `picoquic.h`) calls
    /// [`Quic::dispose_verify_certificate_callback`] first; this
    /// internal entry point does not.
    pub fn tls_set_verify_certificate_callback(&mut self, _cb: Box<dyn VerifyCertificate>) {
        todo!()
    }

    /// Tear down whatever certificate-verifier callback is
    /// currently installed in this context's master TLS context.
    /// C: `dispose_verify_certificate_callback`.
    pub fn dispose_verify_certificate_callback(&mut self) {
        todo!()
    }

    /// Toggle whether the server requires client certificates.  C
    /// took an `int`; promoted to `bool`.  C:
    /// `tls_set_client_authentication`.
    pub fn tls_set_client_authentication(&mut self, _client_authentication: bool) {
        todo!()
    }

    /// Report whether client authentication is currently required.
    /// C: `tls_client_authentication_activated`.
    pub fn tls_client_authentication_activated(&self) -> bool {
        todo!()
    }

    /// Toggle whether tls exposes its exporter API on this master
    /// context.  C: `tls_set_use_exporter`.
    pub fn tls_set_use_exporter(&mut self, _use_exporter: bool) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Retry tokens.
//
// `server_decrypt_retry_token`,
// `prepare_retry_token`, and
// `verify_retry_token` were the three token entry points
// declared in this header.  All three take a `struct sockaddr*`
// peer address (mapped to [`SocketAddr`]) and produce / consume a
// caller-allocated `token` buffer.

/// Output bundle from [`server_decrypt_retry_token`].  C
/// returned the new-token flag through `int*` and the plaintext
/// length through `size_t*`; collapsing them into a struct keeps
/// the result one piece.  The plaintext is written into the
/// caller's `text` slice; `text_length` is how many bytes were
/// actually written.
pub struct DecryptedRetryToken {
    /// Mirrors the C `int* is_new_token` out-parameter; promoted
    /// to `bool`.
    pub is_new_token: bool,
    /// Mirrors the C `size_t* text_length` out-parameter.
    pub text_length: usize,
}

impl Quic {
    /// Decrypt a retry token and verify its peer-address binding.
    /// The plaintext is written into `text`; the result describes
    /// how many bytes were written and whether the token was a "new
    /// token" or a classic retry token.  C:
    /// `server_decrypt_retry_token`.
    pub fn server_decrypt_retry_token(
        &mut self,
        _addr_peer: &SocketAddr,
        _token: &[u8],
        _text: &mut [u8],
    ) -> Result<DecryptedRetryToken, Error> {
        todo!()
    }

    /// Construct a retry / new token signed for `addr_peer`.
    /// Returns the number of bytes written into `token`; `token_max`
    /// is `token.len()`.  C: `prepare_retry_token`.
    pub fn prepare_retry_token(
        &mut self,
        _addr_peer: &SocketAddr,
        _current_time: Instant,
        _odcid: &ConnectionId,
        _rcid: &ConnectionId,
        _initial_pn: u32,
        _token: &mut [u8],
    ) -> Result<usize, Error> {
        todo!()
    }
}

/// Output bundle from [`verify_retry_token`].  Mirrors the
/// `int* is_new_token` and `ConnectionId* odcid`
/// out-parameters of the C signature.
pub struct VerifiedRetryToken {
    /// Mirrors the C `int* is_new_token`; promoted to `bool`.
    pub is_new_token: bool,
    /// Original Destination Connection ID extracted from the
    /// token.  Empty (`id_len == 0`) for "new tokens".
    pub odcid: ConnectionId,
}

impl Quic {
    /// Verify a retry / new token presented by the peer.  Returns
    /// the extracted bundle when the token is fresh, address-bound,
    /// and (for retry tokens) RCID-matched.  C:
    /// `verify_retry_token`.
    pub fn verify_retry_token(
        &mut self,
        _addr_peer: &SocketAddr,
        _current_time: Instant,
        _rcid: &ConnectionId,
        _initial_pn: u32,
        _token: &[u8],
        _check_reuse: bool,
    ) -> Result<VerifiedRetryToken, Error> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Hash helpers exposed so applications don't need tls.h.

/// Maximum digest size across the hash algorithms tls supports.
/// C: `HASH_SIZE_MAX`.
pub const HASH_SIZE_MAX: usize = 64;

/// Construct a streaming hash context for the named algorithm
/// (e.g. `"sha256"`).  Returns a `digest::DynDigest` trait
/// object so callers feed bytes via `update` and read out via
/// `finalize` (the canonical Rust Crypto shape).
///
/// Backends supply concrete digest types (e.g. `sha2::Sha256`)
/// and lift them through this entry point.
pub fn hash_create(_algorithm_name: &str) -> Option<Box<dyn digest::DynDigest>> {
    todo!()
}

/// Digest length (bytes) of the named hash algorithm, or 0 when
/// the algorithm is unknown.  C: `hash_get_length`.
pub fn hash_get_length(_algorithm_name: &str) -> usize {
    todo!()
}

// `hash_update` and `hash_finalize` are gone -- callers use
// `digest::DynDigest`'s `update` and `finalize_into` methods on
// the boxed trait object returned by `hash_create`.

// ---------------------------------------------------------------------------
// Private-key / certificate file loaders.

impl Quic {
    /// Load a PEM-encoded private key from `file_name` and install
    /// it in this context's master TLS context.  C:
    /// `set_private_key_from_file`.
    pub fn set_private_key_from_file(&mut self, _file_name: &str) -> Result<(), Error> {
        todo!()
    }
}

/// Load a PEM-encoded certificate chain from `file_name` and
/// return it as an owned vector of DER-encoded certificate byte
/// strings.  C: `get_certs_from_file` (which allocated both the
/// outer iovec array and each `base` slot — the Rust shape owns
/// both via the nested `Vec<Vec<u8>>`).  Returns `None` when the
/// loader callback is unset or the file fails to parse.
pub fn get_certs_from_file(_file_name: &str) -> Option<Vec<Vec<u8>>> {
    todo!()
}

// ---------------------------------------------------------------------------
// Retry-packet integrity protection.
//
// These manage a small set of AEAD contexts (one per supported
// QUIC version) used to compute retry-packet integrity tags.

/// Build a retry-protection AEAD context from the retry integrity
/// key for a given version.  C: `create_retry_protection_context`.
pub fn create_retry_protection_context(
    _is_enc: bool,
    _key: &[u8],
    _prefix_label: &str,
) -> Option<Box<dyn crate::tls::PacketKey>> {
    todo!()
}

impl Quic {
    /// Look up (and lazily create) the retry-protection context for
    /// the chosen QUIC version, on the chosen direction.  C:
    /// `find_retry_protection_context`.
    pub fn find_retry_protection_context(
        &mut self,
        _version_index: i32,
        _sending: bool,
    ) -> Option<&mut dyn crate::tls::PacketKey> {
        todo!()
    }

    /// Tear down every retry-protection AEAD context held by this
    /// context.  C: `delete_retry_protection_contexts`.
    pub fn delete_retry_protection_contexts(&mut self) {
        todo!()
    }
}

/// Append the integrity tag to the bytes already written into the
/// retry packet buffer.  Returns the new write index.  C:
/// `encode_retry_protection`.
pub fn encode_retry_protection(
    _integrity_aead: &dyn crate::tls::PacketKey,
    _bytes: &mut [u8],
    _byte_index: usize,
    _odcid: &ConnectionId,
) -> usize {
    todo!()
}

/// Verify the integrity tag at the end of an inbound retry packet.
/// Returns the new payload length (with the tag stripped).
/// C: `verify_retry_protection`.
pub fn verify_retry_protection(
    _integrity_aead: &dyn crate::tls::PacketKey,
    _bytes: &mut [u8],
    _length: usize,
    _byte_index: usize,
    _odcid: &ConnectionId,
) -> Result<usize, Error> {
    todo!()
}

// ---------------------------------------------------------------------------
// Cipher-suite accessors and ECB cipher for CID encryption.

// The cipher-suite lookup functions (`get_cipher_suite_by_id_v`,
// `get_aes128gcm_sha256_v`, `get_aes128gcm_v`, `ecb_create_by_name`)
// returned opaque `*mut c_void` cipher-suite handles in C.  In
// Phase 2 the cipher-suite registry lives on the
// `crate::tls::TlsBackend` and is reached through
// `Session::next_1rtt_keys` etc., which return typed
// `KeyPair`s.  The free-function lookups disappear.

/// AES-128-ECB cipher context.  Phase 2: the C `*mut c_void`
/// wrapper is gone — this struct now owns a concrete
/// [`aes::Aes128`] (the Rust Crypto AES implementation).  Used
/// by the load-balancer CID-encryption flow in [`crate::lb`];
/// header protection has its own AES paths in
/// [`crate::header_protection`].
///
/// `Direction` distinguishes encrypt-mode from decrypt-mode (the
/// C side took an `is_enc` flag); each backing AES type
/// implements one direction only.
pub enum Aes128EcbContext {
    Encrypt(aes::Aes128Enc),
    Decrypt(aes::Aes128Dec),
}

impl core::fmt::Debug for Aes128EcbContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Aes128EcbContext").finish_non_exhaustive()
    }
}

impl Aes128EcbContext {
    /// Construct an AES-128-ECB context (encrypt or decrypt) keyed
    /// by `ecb_key`.  C: `aes128_ecb_create`.
    pub fn new(_is_enc: bool, _ecb_key: &[u8; 16]) -> Self {
        todo!()
    }

    /// Encrypt or decrypt `block` in place.  C:
    /// `aes128_ecb_encrypt` (encrypt-mode only — decrypt-mode
    /// is exposed through the same method here, since the
    /// direction is fixed at construction).
    pub fn process(&self, _block: &mut [u8; 16]) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// TLS API initialization.
//
// These four functions wrap the global crypto-provider registry's
// load / unload / reset cycle.  The flag word is the same
// `TLS_API_INIT_FLAGS_*` bit set declared in
// `crypto_provider_api.rs`.

/// Idempotent first-time initialization of the crypto provider
/// registry.  C: `tls_api_init`.
pub fn tls_api_init() {
    todo!()
}

/// Tear down the crypto provider registry.  C:
/// `tls_api_unload`.
pub fn tls_api_unload() {
    todo!()
}

/// Reset the crypto provider registry, applying a new
/// `TLS_API_INIT_FLAGS_*` mask.  Used by the test suite to swap
/// providers in/out.  C: `tls_api_reset`.
pub fn tls_api_reset(_init_flags: u64) {
    todo!()
}

impl Connection {
    /// Log the loaded provider versions to this connection's
    /// app-message stream.  C: `tls_api_log_versions`.
    pub fn log_tls_api_versions(&mut self) {
        todo!()
    }
}

#[cfg(test)]
mod test {}
