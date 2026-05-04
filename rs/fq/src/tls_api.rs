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
use core::ffi::c_void;
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

/// Free a per-connection TLS context.  C took `void* vctx` because
/// the connection stores the context as `void* tls_ctx`; the
/// `client_mode` flag toggles the ECH-config cleanup path.
///
/// Phase 1 keeps the `*mut c_void` shape: `tls_ctx` is `*mut c_void`
/// in [`Connection`] (see `internal.rs`), and the cast back to `tls_ctx_t*`
/// happens inside the body.  The eventual safe shape (an owning
/// `Box`, drop-managed) lands once `tls_ctx`'s storage is reshaped.
/// C: `tlscontext_free`.
///
/// # Safety
///
/// `ctx` must be a non-null pointer to a `tls_ctx_t` previously
/// returned by [`Connection::create_tls_context`] and not yet freed.  After
/// the call the pointer is dangling.
pub unsafe fn tls_context_free(_ctx: *mut c_void, _client_mode: bool) {
    todo!()
}

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

impl Quic {
    /// Fill `buf` with cryptographically random bytes drawn from the
    /// provider RNG attached to this context.  C: `crypto_random`.
    pub fn crypto_random(&mut self, _buf: &mut [u8]) {
        todo!()
    }

    /// Sample a uniform `u64` in `[0, rnd_max)` from the crypto
    /// RNG.  C: `crypto_uniform_random`.
    pub fn crypto_uniform_random(&mut self, _rnd_max: u64) -> u64 {
        todo!()
    }

    /// Re-seed the public RNG by drawing fresh entropy from the
    /// crypto RNG installed in this context.  C:
    /// `public_random_seed`.
    pub fn seed_public_random(&mut self) {
        todo!()
    }
}

/// Single 64-bit draw from the public xorshift1024* RNG.  C:
/// `public_random_64`.
pub fn public_random_64() -> u64 {
    todo!()
}

/// Re-seed the public RNG.  `reset == true` reinitializes the state
/// to the documented constants; otherwise the seed is XORed into
/// the current state.  C: `public_random_seed_64`.
pub fn public_random_seed_64(_seed: u64, _reset: bool) {
    todo!()
}

/// Fill `buf` with bytes drawn from the public RNG.  C:
/// `public_random`.
pub fn public_random(_buf: &mut [u8]) {
    todo!()
}

/// Sample a uniform `u64` in `[0, rnd_max)` from the public RNG.
/// C: `public_uniform_random`.
pub fn public_uniform_random(_rnd_max: u64) -> u64 {
    todo!()
}

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

/// Length of the authentication tag for the AEAD context (capped at
/// 16 bytes per the workaround in `tls_api.c` for an old tls
/// regression).  C: `aead_get_checksum_length`.
///
/// # Safety
///
/// `aead_context` must be a non-null pointer to a valid
/// `ptls_aead_context_t`.
pub unsafe fn aead_get_checksum_length(_aead_context: *mut c_void) -> usize {
    todo!()
}

/// Encrypt `input` (length `input_length`) into `output` using the
/// AEAD context, sequence number, and authenticated-data slice
/// supplied.  Returns the number of bytes written, mirroring
/// `ptls_aead_encrypt`.  C: `aead_encrypt_generic`.
///
/// The C signature took raw `(uint8_t* output, …, size_t
/// input_length)` and `(const uint8_t* auth_data, size_t
/// auth_data_length)` pairs; the Rust shape collapses each
/// pointer/length pair into a slice.  The output buffer is borrowed
/// long enough to receive `input.len() + tag_size` bytes; callers
/// must size it.
///
/// # Safety
///
/// `aead_context` must be a non-null pointer to a valid
/// `ptls_aead_context_t`.
pub unsafe fn aead_encrypt_generic(
    _output: &mut [u8],
    _input: &[u8],
    _seq_num: u64,
    _auth_data: &[u8],
    _aead_context: *mut c_void,
) -> usize {
    todo!()
}

/// Decrypt `input` into `output`.  Returns the number of plaintext
/// bytes produced; the C `SIZE_MAX` failure sentinel (returned when
/// the AEAD context is null or authentication fails) maps to
/// `Err`.  C: `aead_decrypt_generic`.
///
/// # Safety
///
/// `aead_ctx` may be null (the C path returns `SIZE_MAX`, which
/// becomes `Err`); when non-null it must point to a valid
/// `ptls_aead_context_t`.
pub unsafe fn aead_decrypt_generic(
    _output: &mut [u8],
    _input: &[u8],
    _seq_num: u64,
    _auth_data: &[u8],
    _aead_ctx: *mut c_void,
) -> Result<usize, Error> {
    todo!()
}

/// Multipath variant of [`aead_decrypt_generic`].  The IV is XORed
/// with the path id before / after the tls call, per the multipath
/// extension.  C: `aead_decrypt_mp`.
///
/// # Safety
///
/// Same conditions as [`aead_decrypt_generic`].
pub unsafe fn aead_decrypt_mp(
    _output: &mut [u8],
    _input: &[u8],
    _path_id: u64,
    _seq_num: u64,
    _auth_data: &[u8],
    _aead_context: *mut c_void,
) -> Result<usize, Error> {
    todo!()
}

/// Multipath variant of [`aead_encrypt_generic`].  C:
/// `aead_encrypt_mp`.
///
/// # Safety
///
/// Same conditions as [`aead_encrypt_generic`].
pub unsafe fn aead_encrypt_mp(
    _output: &mut [u8],
    _input: &[u8],
    _path_id: u64,
    _seq_num: u64,
    _auth_data: &[u8],
    _aead_context: *mut c_void,
) -> usize {
    todo!()
}

/// AEAD integrity limit (bytes processed) carried by the
/// algorithm.  C: `aead_integrity_limit`.
///
/// # Safety
///
/// `aead_ctx` must point to a valid `ptls_aead_context_t`.
pub unsafe fn aead_integrity_limit(_aead_ctx: *mut c_void) -> u64 {
    todo!()
}

/// AEAD confidentiality limit (records encrypted) carried by the
/// algorithm.  C: `aead_confidentiality_limit`.
///
/// # Safety
///
/// `aead_ctx` must point to a valid `ptls_aead_context_t`.
pub unsafe fn aead_confidentiality_limit(_aead_ctx: *mut c_void) -> u64 {
    todo!()
}

/// Free the tls AEAD context.  C: `aead_free`.
///
/// # Safety
///
/// `aead_context` must point to a `ptls_aead_context_t` previously
/// allocated by tls and not yet freed.  After the call the
/// pointer is dangling.
pub unsafe fn aead_free(_aead_context: *mut c_void) {
    todo!()
}

/// Free the tls cipher context (used for PN encryption and the
/// CID encryption / ECB cipher).  C: `cipher_free`.
///
/// # Safety
///
/// `cipher_context` must point to a `ptls_cipher_context_t`
/// previously allocated by tls and not yet freed.
pub unsafe fn cipher_free(_cipher_context: *mut c_void) {
    todo!()
}

// ---------------------------------------------------------------------------
// Packet-number encryption.

/// IV size of the PN encryption cipher, surfaced from the tls
/// cipher algo struct.  C: `pn_iv_size`.
///
/// # Safety
///
/// `pn_enc` must point to a valid `ptls_cipher_context_t`.
pub unsafe fn pn_iv_size(_pn_enc: *mut c_void) -> usize {
    todo!()
}

/// Apply the PN encryption cipher to `input`, writing `len` bytes
/// to `output`.  `iv` is the nonce; it is borrowed for the
/// duration of the call.  C: `pn_encrypt`.
///
/// The C signature used `void*` for everything because tls
/// works on raw bytes; the Rust shape uses byte slices.  `iv` is
/// sized at the cipher's IV length (caller must pass a slice of at
/// least that length); `input` and `output` carry the same length.
///
/// # Safety
///
/// `pn_enc` must point to a valid `ptls_cipher_context_t`.
pub unsafe fn pn_encrypt(_pn_enc: *mut c_void, _iv: &[u8], _output: &mut [u8], _input: &[u8]) {
    todo!()
}

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
    /// Owned AEAD context handle.
    pub aead_ctx: *mut c_void,
    /// Owned PN-encryption context handle.
    pub pn_enc_ctx: *mut c_void,
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
) -> *mut c_void {
    todo!()
}

/// Construct a PN-encryption context for tests.  C:
/// `pn_enc_create_for_test`.
pub fn pn_enc_create_for_test(_secret: &[u8], _prefix_label: &str) -> *mut c_void {
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
/// (e.g. `"sha256"`).  Returns null on lookup failure (matching the
/// C `void*`).  C: `hash_create`.
pub fn hash_create(_algorithm_name: &str) -> *mut c_void {
    todo!()
}

/// Digest length (bytes) of the named hash algorithm, or 0 when
/// the algorithm is unknown.  C: `hash_get_length`.
pub fn hash_get_length(_algorithm_name: &str) -> usize {
    todo!()
}

/// Push `input` into the streaming hash context.  C signature
/// passed `(uint8_t* input, size_t input_length, void*
/// hash_context)`; the Rust shape collapses the pointer/length pair
/// into a slice.  C: `hash_update`.
///
/// # Safety
///
/// `hash_context` must point to a hash context returned by
/// [`hash_create`] and not yet finalized.
pub unsafe fn hash_update(_input: &[u8], _hash_context: *mut c_void) {
    todo!()
}

/// Finalize the hash context, writing the digest into `output` and
/// freeing the context.  `output` must be at least
/// `hash_get_length(algorithm)` bytes long; phase 1 keeps
/// the caller-sized slice shape from the C signature.  C:
/// `hash_finalize`.
///
/// # Safety
///
/// `hash_context` must point to a hash context returned by
/// [`hash_create`] and not yet finalized.  After the call
/// the context is consumed.
pub unsafe fn hash_finalize(_output: &mut [u8], _hash_context: *mut c_void) {
    todo!()
}

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
/// key for a given version.  Returns null on failure (matching C
/// `void*`).  C: `create_retry_protection_context`.
pub fn create_retry_protection_context(
    _is_enc: bool,
    _key: &[u8],
    _prefix_label: &str,
) -> *mut c_void {
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
    ) -> *mut c_void {
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
///
/// # Safety
///
/// `integrity_aead` may be null (the C path is a no-op then); when
/// non-null it must point to a valid `ptls_aead_context_t`.
pub unsafe fn encode_retry_protection(
    _integrity_aead: *mut c_void,
    _bytes: &mut [u8],
    _byte_index: usize,
    _odcid: &ConnectionId,
) -> usize {
    todo!()
}

/// Verify the integrity tag at the end of an inbound retry packet.
/// Returns the new payload length (with the tag stripped); the C
/// signature took `size_t* length` to mutate in place, the Rust
/// shape returns the updated length on success.  C:
/// `verify_retry_protection`.
///
/// # Safety
///
/// `integrity_aead` must point to a valid `ptls_aead_context_t`.
pub unsafe fn verify_retry_protection(
    _integrity_aead: *mut c_void,
    _bytes: &mut [u8],
    _length: usize,
    _byte_index: usize,
    _odcid: &ConnectionId,
) -> Result<usize, Error> {
    todo!()
}

// ---------------------------------------------------------------------------
// Cipher-suite accessors and ECB cipher for CID encryption.

/// Look up a cipher suite by its TLS code-point.  Returns null when
/// no provider supplies one (matching the C `void*`).  C:
/// `get_cipher_suite_by_id_v`.
pub fn get_cipher_suite_by_id_v(_cipher_suite_id: i32, _use_low_memory: bool) -> *mut c_void {
    todo!()
}

/// Look up the AES-128-GCM-SHA256 cipher suite (used for Initial
/// packets).  C: `get_aes128gcm_sha256_v`.
pub fn get_aes128gcm_sha256_v(_use_low_memory: bool) -> *mut c_void {
    todo!()
}

/// Look up just the AEAD algorithm slot of the AES-128-GCM cipher
/// suite.  C: `get_aes128gcm_v`.
pub fn get_aes128gcm_v(_use_low_memory: bool) -> *mut c_void {
    todo!()
}

/// Build an ECB-mode cipher context by algorithm name (used by the
/// load-balancer CID encryption flow).  Returns null on lookup
/// failure.  C: `ecb_create_by_name`.
pub fn ecb_create_by_name(_is_enc: bool, _ecb_key: &[u8], _alg_name: &str) -> *mut c_void {
    todo!()
}

/// AES-128-ECB cipher context.  C: the `ptls_cipher_context_t`
/// produced by `picoquic_aes128_ecb_create`, behind a typed wrapper
/// here so callers don't see a raw `*mut c_void`.
///
/// REVIEW(open): the inner pointer becomes a real owned cipher
/// context (or a trait object over the crypto provider) once Phase 2
/// lands the dependency abstraction.  `Drop` will replace the
/// explicit `aes128_ecb_free` once the body is wired up.
pub struct Aes128EcbContext {
    #[allow(dead_code)] // Phase 4 wires this through to the crypto provider.
    pub(crate) inner: *mut c_void,
}

impl core::fmt::Debug for Aes128EcbContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Aes128EcbContext").finish_non_exhaustive()
    }
}

impl Aes128EcbContext {
    /// Construct an AES-128-ECB context (encrypt or decrypt) keyed
    /// by `ecb_key`.  Returns [`None`] when the underlying
    /// allocation fails.  C: `aes128_ecb_create`.
    pub fn new(_is_enc: bool, _ecb_key: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Encrypt `input` into `output` (both same length) using this
    /// ECB cipher.  C: `aes128_ecb_encrypt`.
    pub fn encrypt(&mut self, _output: &mut [u8], _input: &[u8]) {
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
