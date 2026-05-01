//! Translation of `picoquic/tls_api.h`.
//!
//! `tls_api.h` is the boundary between picoquic-core's QUIC machinery
//! and the picotls TLS 1.3 stack underneath.  It exposes the master
//! TLS context lifecycle, per-connection TLS contexts, the AEAD /
//! cipher / hash primitives picoquic uses internally, the public and
//! crypto random generators, the retry-token / retry-protection
//! helpers, and a handful of provider-installation entry points kept
//! in this header so applications don't have to include `picotls.h`.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//! Phase 3 fills in the bodies.
//!
//! ## Pointer-shape decisions
//!
//! * picotls types this header references but does not define
//!   (`ptls_cipher_suite_t`, `ptls_verify_certificate_t`, …) are
//!   already forward-declared in
//!   [`crate::crypto_provider_api`] and
//!   [`crate`]; this module re-uses those
//!   declarations rather than duplicating them.
//! * The `void*` AEAD / PN-encryption / cipher / hash contexts on the
//!   C side are picotls handles whose Rust binding doesn't exist yet
//!   (picotls is an external dependency that has not been
//!   translated).  They stay as `*mut c_void` opaque handles in
//!   Phase 1; Phase 3 will replace them with proper trait objects or
//!   forward-declared structs once picotls is bound.
//! * `int` flag parameters that encode booleans (`is_enc`,
//!   `is_client`, `client_mode`, `use_low_memory`, `reset`,
//!   `check_reuse`, `sending`) become `bool`.  Likewise
//!   `unsigned int client_mode` in `picoquic_tlscontext_free`.
//! * C status-code returns (`int` 0/-1 / `PICOQUIC_ERROR_*`) become
//!   `Result<T, ()>` per the Phase 1 contract — the crate-level
//!   `Error` enum doesn't exist yet, so `()` is a placeholder.
//! * Owning `ptls_iovec_t* picoquic_get_certs_from_file(…, size_t*
//!   count)` collapses to `Option<Vec<ptls_iovec_t>>`: the C callee
//!   `malloc`s the slot array and writes its length through the
//!   out-pointer, and the caller `free`s.  A Rust `Vec` carries both
//!   ownership and length.
//! * Output parameters (`uint8_t reset_secret[16]`,
//!   `uint8_t* master_secret`, `void** aead_ctx`, `size_t*
//!   text_length`, `int* data_consumed`, `int* is_new_token`,
//!   `picoquic_connection_id_t* odcid`, `size_t* token_size`) become
//!   `&mut`-borrowed slots or are returned as part of the function's
//!   result tuple, depending on whether the caller already holds
//!   storage.
//! * `struct sockaddr*` arguments map to [`core::net::SocketAddr`]
//!   for parity with `picoquic.rs`.
//! * `FreeVerifyCertificateCtx` is a function-pointer
//!   typedef in picoquic.h; the trait counterpart is already in
//!   `picoquic.rs` and is reused here.
//! * Function-pointer parameters in callbacks
//!   (`picoquic_tls_set_verify_certificate_callback`) take
//!   `Option<Box<dyn …>>` — the C `NULL` sentinel maps to `None` and
//!   the registered callback is owned by the registry.
//!
//! ## Initial-secret label constants
//!
//! `PICOQUIC_LABEL_QUIC_BASE` is `#define`d as `NULL` in C — it's
//! the "no prefix label" sentinel passed to picotls.  In Rust the
//! prefix-label parameters that accept it become `Option<&str>`, so
//! the constant itself doesn't appear; the call sites pass `None`
//! when they would have passed `PICOQUIC_LABEL_QUIC_BASE`.
//!
//! ## Out of scope
//!
//! * The five `#if 0`-disabled `picoquic_cid_*_under_mask_ctx` /
//!   `picoquic_cid_free_encrypt_global_ctx` entries are dead code in
//!   the C source and are not translated.
//! * `picoquic_get_private_key_from_file` is also `#if 0` in the
//!   header (and the `_t` callback variant lives in
//!   `picoquic_crypto_provider_api.rs`); not translated.

#![allow(clippy::result_unit_err)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::net::SocketAddr;

use crate::internal::picoquic_crypto_context_t;
use crate::{
    FreeVerifyCertificateCtx, PICOQUIC_RESET_SECRET_SIZE, picoquic_cnx_t, picoquic_connection_id_t,
    picoquic_quic_t, ptls_iovec_t, ptls_verify_certificate_t,
};

// ---------------------------------------------------------------------------
// Label constants (C `#define` → `&str`).
//
// All labels are NUL-terminated bytes in the C header because picotls
// takes them as `char const*`.  The Rust shape is plain `&str`; the
// trailing NUL is reintroduced (if needed) at the picotls FFI
// boundary.

/// Initial secret label for the client direction.  C:
/// `PICOQUIC_LABEL_INITIAL_CLIENT`.
pub const PICOQUIC_LABEL_INITIAL_CLIENT: &str = "client in";

/// Initial secret label for the server direction.  C:
/// `PICOQUIC_LABEL_INITIAL_SERVER`.
pub const PICOQUIC_LABEL_INITIAL_SERVER: &str = "server in";

/// Traffic-update label for QUIC v1.  C:
/// `PICOQUIC_LABEL_V1_TRAFFIC_UPDATE`.
pub const PICOQUIC_LABEL_V1_TRAFFIC_UPDATE: &str = "quic ku";

/// Traffic-update label for QUIC v2.  C:
/// `PICOQUIC_LABEL_V2_TRAFFIC_UPDATE`.
pub const PICOQUIC_LABEL_V2_TRAFFIC_UPDATE: &str = "quicv2 ku";

/// HKDF "key" component label.  C: `PICOQUIC_LABEL_KEY`.
pub const PICOQUIC_LABEL_KEY: &str = "key";

/// HKDF "iv" component label.  C: `PICOQUIC_LABEL_IV`.
pub const PICOQUIC_LABEL_IV: &str = "iv";

/// HKDF "hp" (header protection) component label.  C:
/// `PICOQUIC_LABEL_HP`.
pub const PICOQUIC_LABEL_HP: &str = "hp";

/// HKDF "cid" component label.  C: `PICOQUIC_LABEL_CID`.
pub const PICOQUIC_LABEL_CID: &str = "cid";

/// HKDF label for the global CID encryption secret.  C:
/// `PICOQUIC_LABEL_CID_GLOBAL`.
pub const PICOQUIC_LABEL_CID_GLOBAL: &str = "cid global";

/// Number of HKDF rounds run when deriving the global CID secret.
/// C: `PICOQUIC_LABEL_CID_GLOBAL_ROUNDS`.
pub const PICOQUIC_LABEL_CID_GLOBAL_ROUNDS: u32 = 4;

/// QUIC v1 TLS key-derivation label prefix.  C:
/// `PICOQUIC_LABEL_QUIC_V1_KEY_BASE`.
pub const PICOQUIC_LABEL_QUIC_V1_KEY_BASE: &str = "tls13 quic ";

/// QUIC v2 TLS key-derivation label prefix.  C:
/// `PICOQUIC_LABEL_QUIC_V2_KEY_BASE`.
pub const PICOQUIC_LABEL_QUIC_V2_KEY_BASE: &str = "tls13 quicv2 ";

// ---------------------------------------------------------------------------
// Forward declaration of `ptls_cipher_suite_t`.
//
// The C header re-typedef-s `ptls_cipher_suite_t` as
// `const struct st_ptls_cipher_suite_t` so consumers don't need
// `picotls.h`.  picotls itself has not been translated; this opaque
// stand-in keeps signatures compiling.  Phase 3 will swap in the
// real picotls binding once it exists.
//
// Note: `crypto_provider_api` already declares an identical
// placeholder.  Pull it in for use within this module's signatures
// (no re-export).
use crate::crypto_provider_api::ptls_cipher_suite_t;

// ---------------------------------------------------------------------------
// Master TLS context.
//
// The master context lives in `quic->tls_master_ctx` (a `void*` on
// the C side, a `ptls_context_t*` once cast).  All three functions
// take `&mut picoquic_quic_t` because they install or release state
// on the QUIC context itself.

/// Initialize the per-`picoquic_quic_t` master TLS context.  Loads
/// the certificate chain, the private key, the trusted-root bundle
/// and the ticket-encryption key.  Any of the file-name / key
/// parameters may be `NULL` in C (server-only, no roots, no ticket
/// key); Rust models that with `Option<&str>` / `Option<&[u8]>`.
/// C: `picoquic_master_tlscontext`.
pub fn picoquic_master_tlscontext(
    _quic: &mut picoquic_quic_t,
    _cert_file_name: Option<&str>,
    _key_file_name: Option<&str>,
    _cert_root_file_name: Option<&str>,
    _ticket_key: Option<&[u8]>,
) -> Result<(), ()> {
    todo!()
}

/// Tear down the master TLS context allocated by
/// [`picoquic_master_tlscontext`].  C:
/// `picoquic_master_tlscontext_free`.
pub fn picoquic_master_tlscontext_free(_quic: &mut picoquic_quic_t) {
    todo!()
}

// ---------------------------------------------------------------------------
// Per-connection TLS context.
//
// `picoquic_tls_ctx_t` itself lives in
// `picoquic_crypto_provider_api.rs`; this module just exposes the
// API surface that picoquic-core uses to drive it.

/// Allocate a per-connection TLS context, attach it to `cnx`, and
/// initialize the picotls handshake-property slots.  C side returns
/// 0 on success, `PICOQUIC_ERROR_TLS_SERVER_CON_WITHOUT_CERT` /
/// `PICOQUIC_ERROR_MEMORY` / -1 on failure.  C:
/// `picoquic_tlscontext_create`.
pub fn picoquic_tlscontext_create(
    _quic: &mut picoquic_quic_t,
    _cnx: &mut picoquic_cnx_t,
) -> Result<(), ()> {
    todo!()
}

/// Free a per-connection TLS context.  C took `void* vctx` because
/// the connection stores the context as `void* tls_ctx`; the
/// `client_mode` flag toggled the ECH-config cleanup path.
///
/// Phase 1 keeps the `*mut c_void` shape: `tls_ctx` is `*mut
/// c_void` in `picoquic_cnx_t` (see `picoquic_internal.rs`), and
/// the cast back to `picoquic_tls_ctx_t*` happens inside the body.
/// The eventual safe shape (an owning `Box`, drop-managed) lands
/// once `tls_ctx`'s storage is reshaped.  C:
/// `picoquic_tlscontext_free`.
///
/// # Safety
///
/// `ctx` must be a non-null pointer to a `picoquic_tls_ctx_t`
/// previously returned by [`picoquic_tlscontext_create`] and not
/// yet freed.
pub unsafe fn picoquic_tlscontext_free(_ctx: *mut c_void, _client_mode: bool) {
    todo!()
}

/// Drop transient buffers (ALPN list, transport-parameter encode
/// scratch) once the handshake is done.  C:
/// `picoquic_tlscontext_trim_after_handshake`.
pub fn picoquic_tlscontext_trim_after_handshake(_cnx: &mut picoquic_cnx_t) {
    todo!()
}

/// Forget the session ticket installed for a 0-RTT attempt.  C:
/// `picoquic_tlscontext_remove_ticket`.
pub fn picoquic_tlscontext_remove_ticket(_cnx: &mut picoquic_cnx_t) {
    todo!()
}

// ---------------------------------------------------------------------------
// TLS stream processing.

/// Drive the TLS handshake by feeding in any data buffered on the
/// crypto streams and pushing produced data back out.  C signature
/// returned the consumed-byte count through `int* data_consumed`;
/// the Rust shape returns it inside the success arm of `Result`.
/// C: `picoquic_tls_stream_process`.
pub fn picoquic_tls_stream_process(
    _cnx: &mut picoquic_cnx_t,
    _current_time: u64,
) -> Result<i32, ()> {
    todo!()
}

/// Report whether the TLS handshake has completed.  C signature
/// returned `int` (0/1); promoted to `bool`.  C:
/// `picoquic_is_tls_complete`.
pub fn picoquic_is_tls_complete(_cnx: &picoquic_cnx_t) -> bool {
    todo!()
}

/// Send the initial `ClientHello` (or the response to a `HelloRetry`)
/// on the TLS stream.  C: `picoquic_initialize_tls_stream`.
pub fn picoquic_initialize_tls_stream(
    _cnx: &mut picoquic_cnx_t,
    _current_time: u64,
) -> Result<(), ()> {
    todo!()
}

/// Read the virtual time picotls sees through its `get_time`
/// callback (microseconds).  C: `picoquic_get_tls_time`.
pub fn picoquic_get_tls_time(_quic: &picoquic_quic_t) -> u64 {
    todo!()
}

// ---------------------------------------------------------------------------
// Random number generation.
//
// The crypto-grade RNG is the picotls / OpenSSL provider; the
// public RNG is xorshift1024* seeded from the crypto RNG.  Both
// take a destination slice; the C `(void* buf, size_t len)` pair
// collapses to `&mut [u8]`.

/// Fill `buf` with cryptographically random bytes drawn from the
/// provider RNG attached to `quic`.  C: `picoquic_crypto_random`.
pub fn picoquic_crypto_random(_quic: &mut picoquic_quic_t, _buf: &mut [u8]) {
    todo!()
}

/// Sample a uniform `u64` in `[0, rnd_max)` from the crypto RNG.
/// C: `picoquic_crypto_uniform_random`.
pub fn picoquic_crypto_uniform_random(_quic: &mut picoquic_quic_t, _rnd_max: u64) -> u64 {
    todo!()
}

/// Single 64-bit draw from the public xorshift1024* RNG.  C:
/// `picoquic_public_random_64`.
pub fn picoquic_public_random_64() -> u64 {
    todo!()
}

/// Re-seed the public RNG.  `reset == true` reinitializes the state
/// to the documented constants; otherwise the seed is XORed into
/// the current state.  C: `picoquic_public_random_seed_64`.
pub fn picoquic_public_random_seed_64(_seed: u64, _reset: bool) {
    todo!()
}

/// Re-seed the public RNG by drawing fresh entropy from the crypto
/// RNG installed in `quic`.  C: `picoquic_public_random_seed`.
pub fn picoquic_public_random_seed(_quic: &mut picoquic_quic_t) {
    todo!()
}

/// Fill `buf` with bytes drawn from the public RNG.  C:
/// `picoquic_public_random`.
pub fn picoquic_public_random(_buf: &mut [u8]) {
    todo!()
}

/// Sample a uniform `u64` in `[0, rnd_max)` from the public RNG.
/// C: `picoquic_public_uniform_random`.
pub fn picoquic_public_uniform_random(_rnd_max: u64) -> u64 {
    todo!()
}

// ---------------------------------------------------------------------------
// AEAD primitives.
//
// All AEAD entry points take an opaque `void*` AEAD handle in C
// (really a `ptls_aead_context_t*`).  Phase 1 keeps the opaque
// shape as `*mut c_void`; Phase 3 will swap in a forward-declared
// picotls struct or trait object once that binding lands.  Each
// `picoquic_aead_*` body is a thin wrapper around the corresponding
// `ptls_aead_*` call, so the callee mutates state through the
// pointer (e.g. AEAD nonce) — the Rust shape uses `&mut` to capture
// that.

/// Length of the authentication tag for the AEAD context (capped at
/// 16 bytes per the workaround in `tls_api.c` for an old picotls
/// regression).  C: `picoquic_aead_get_checksum_length`.
///
/// # Safety
///
/// `aead_context` must be a non-null pointer to a valid
/// `ptls_aead_context_t`.
pub unsafe fn picoquic_aead_get_checksum_length(_aead_context: *mut c_void) -> usize {
    todo!()
}

/// Encrypt `input` (length `input_length`) into `output` using the
/// AEAD context, sequence number, and authenticated-data slice
/// supplied.  Returns the number of bytes written, mirroring
/// `ptls_aead_encrypt`.  C: `picoquic_aead_encrypt_generic`.
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
pub unsafe fn picoquic_aead_encrypt_generic(
    _output: &mut [u8],
    _input: &[u8],
    _seq_num: u64,
    _auth_data: &[u8],
    _aead_context: *mut c_void,
) -> usize {
    todo!()
}

/// Decrypt `input` into `output`.  Returns the number of plaintext
/// bytes produced, or `usize::MAX` (mirroring the C `SIZE_MAX`
/// failure sentinel) when the AEAD context is null or
/// authentication fails.  C: `picoquic_aead_decrypt_generic`.
///
/// # Safety
///
/// `aead_ctx` may be null (the C path returns `SIZE_MAX`); when
/// non-null it must point to a valid `ptls_aead_context_t`.
pub unsafe fn picoquic_aead_decrypt_generic(
    _output: &mut [u8],
    _input: &[u8],
    _seq_num: u64,
    _auth_data: &[u8],
    _aead_ctx: *mut c_void,
) -> usize {
    todo!()
}

/// Multipath variant of [`picoquic_aead_decrypt_generic`].  The IV
/// is XORed with the path id before / after the picotls call,
/// per the multipath extension.  C: `picoquic_aead_decrypt_mp`.
///
/// # Safety
///
/// Same conditions as [`picoquic_aead_decrypt_generic`].
pub unsafe fn picoquic_aead_decrypt_mp(
    _output: &mut [u8],
    _input: &[u8],
    _path_id: u64,
    _seq_num: u64,
    _auth_data: &[u8],
    _aead_context: *mut c_void,
) -> usize {
    todo!()
}

/// Multipath variant of [`picoquic_aead_encrypt_generic`].  C:
/// `picoquic_aead_encrypt_mp`.
///
/// # Safety
///
/// Same conditions as [`picoquic_aead_encrypt_generic`].
pub unsafe fn picoquic_aead_encrypt_mp(
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
/// algorithm.  C: `picoquic_aead_integrity_limit`.
///
/// # Safety
///
/// `aead_ctx` must point to a valid `ptls_aead_context_t`.
pub unsafe fn picoquic_aead_integrity_limit(_aead_ctx: *mut c_void) -> u64 {
    todo!()
}

/// AEAD confidentiality limit (records encrypted) carried by the
/// algorithm.  C: `picoquic_aead_confidentiality_limit`.
///
/// # Safety
///
/// `aead_ctx` must point to a valid `ptls_aead_context_t`.
pub unsafe fn picoquic_aead_confidentiality_limit(_aead_ctx: *mut c_void) -> u64 {
    todo!()
}

/// Free the picotls AEAD context.  C: `picoquic_aead_free`.
///
/// # Safety
///
/// `aead_context` must point to a `ptls_aead_context_t` previously
/// allocated by picotls and not yet freed.  After the call the
/// pointer is dangling.
pub unsafe fn picoquic_aead_free(_aead_context: *mut c_void) {
    todo!()
}

/// Free the picotls cipher context (used for PN encryption and the
/// CID encryption / ECB cipher).  C: `picoquic_cipher_free`.
///
/// # Safety
///
/// `cipher_context` must point to a `ptls_cipher_context_t`
/// previously allocated by picotls and not yet freed.
pub unsafe fn picoquic_cipher_free(_cipher_context: *mut c_void) {
    todo!()
}

// ---------------------------------------------------------------------------
// Packet-number encryption.

/// IV size of the PN encryption cipher, surfaced from the picotls
/// cipher algo struct.  C: `picoquic_pn_iv_size`.
///
/// # Safety
///
/// `pn_enc` must point to a valid `ptls_cipher_context_t`.
pub unsafe fn picoquic_pn_iv_size(_pn_enc: *mut c_void) -> usize {
    todo!()
}

/// Apply the PN encryption cipher to `input`, writing `len` bytes
/// to `output`.  `iv` is the nonce; it is borrowed for the
/// duration of the call.  C: `picoquic_pn_encrypt`.
///
/// The C signature used `void*` for everything because picotls
/// works on raw bytes; the Rust shape uses byte slices.  `iv` is
/// sized at the cipher's IV length (caller must pass a slice of at
/// least that length); `input` and `output` carry the same length.
///
/// # Safety
///
/// `pn_enc` must point to a valid `ptls_cipher_context_t`.
pub unsafe fn picoquic_pn_encrypt(
    _pn_enc: *mut c_void,
    _iv: &[u8],
    _output: &mut [u8],
    _input: &[u8],
) {
    todo!()
}

// ---------------------------------------------------------------------------
// Initial-secret derivation.
//
// `picoquic_setup_initial_master_secret` and
// `picoquic_setup_initial_secrets` write into caller-provided
// buffers sized at `cipher->hash->digest_size`.  Phase 1 keeps the
// `&mut [u8]` shape for the buffers so the caller controls
// allocation; Phase 3 may switch to fixed-size arrays once the
// digest size is part of the cipher trait.

/// Derive the per-connection-ID initial master secret.  C:
/// `picoquic_setup_initial_master_secret`.
pub fn picoquic_setup_initial_master_secret(
    _cipher: &ptls_cipher_suite_t,
    _salt: ptls_iovec_t,
    _initial_cnxid: picoquic_connection_id_t,
    _master_secret: &mut [u8],
) -> Result<(), ()> {
    todo!()
}

/// Derive client/server initial secrets from the master secret.
/// `client_secret` and `server_secret` are filled in place.  C:
/// `picoquic_setup_initial_secrets`.
pub fn picoquic_setup_initial_secrets(
    _cipher: &ptls_cipher_suite_t,
    _master_secret: &[u8],
    _client_secret: &mut [u8],
    _server_secret: &mut [u8],
) -> Result<(), ()> {
    todo!()
}

/// Set up `cnx`'s per-epoch initial AEAD / PN encryption contexts
/// from the connection's initial CID.  C:
/// `picoquic_setup_initial_traffic_keys`.
pub fn picoquic_setup_initial_traffic_keys(_cnx: &mut picoquic_cnx_t) -> Result<(), ()> {
    todo!()
}

/// Output bundle from [`picoquic_get_initial_aead_context`].  C
/// returned the AEAD and PN contexts through `void**` out-pointers;
/// the Rust shape collapses both into a struct so the function
/// signature is one-out, one-return.
pub struct PicoquicInitialAeadContext {
    /// Owned AEAD context handle.
    pub aead_ctx: *mut c_void,
    /// Owned PN-encryption context handle.
    pub pn_enc_ctx: *mut c_void,
}

/// Derive an AEAD + PN context for the initial encryption level.
/// `version_index` selects the QUIC version's salt and prefix
/// label; `is_client` and `is_enc` pick the client/server and
/// encrypt/decrypt directions.  C:
/// `picoquic_get_initial_aead_context`.
pub fn picoquic_get_initial_aead_context(
    _quic: &mut picoquic_quic_t,
    _version_index: i32,
    _initial_cnxid: &picoquic_connection_id_t,
    _is_client: bool,
    _is_enc: bool,
) -> Result<PicoquicInitialAeadContext, ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Application-secret access and rotation.
//
// `picoquic_get_app_secret` returns one of the two
// `app_secret_*` buffers stashed inside `picoquic_tls_ctx_t`.  The
// C signature returned a `uint8_t*` into that buffer; in Rust the
// natural shape is a borrowed slice carrying the digest length
// (callers consult `picoquic_get_app_secret_size` for the length
// today, which is redundant once the borrow encodes it).  Phase 3
// may collapse the size accessor.

/// Return a borrow of the app-data traffic secret stored in `cnx`'s
/// TLS context for the chosen direction.  C:
/// `picoquic_get_app_secret`.
pub fn picoquic_get_app_secret(_cnx: &mut picoquic_cnx_t, _is_enc: bool) -> &mut [u8] {
    todo!()
}

/// Length (bytes) of the app-data traffic secret — the digest size
/// of the negotiated cipher's hash.  C:
/// `picoquic_get_app_secret_size`.
pub fn picoquic_get_app_secret_size(_cnx: &picoquic_cnx_t) -> usize {
    todo!()
}

/// Compute the post-rotation AEAD + PN contexts and stash them in
/// `cnx->crypto_context_new`.  C:
/// `picoquic_compute_new_rotated_keys`.
pub fn picoquic_compute_new_rotated_keys(_cnx: &mut picoquic_cnx_t) -> Result<(), ()> {
    todo!()
}

/// Promote `cnx->crypto_context_new` to the active `crypto_context[3]`
/// slot, demoting the previous keys.  C:
/// `picoquic_apply_rotated_keys`.
pub fn picoquic_apply_rotated_keys(_cnx: &mut picoquic_cnx_t, _is_enc: bool) {
    todo!()
}

/// Rotate the application traffic secret in place using the
/// version-specific traffic-update label.  C:
/// `picoquic_rotate_app_secret`.
pub fn picoquic_rotate_app_secret(
    _cipher: &ptls_cipher_suite_t,
    _secret: &mut [u8],
    _traffic_update_label: &str,
) -> Result<(), ()> {
    todo!()
}

/// Free every AEAD / PN-encryption slot held by a crypto context.
/// C: `picoquic_crypto_context_free`.
pub fn picoquic_crypto_context_free(_ctx: &mut picoquic_crypto_context_t) {
    todo!()
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
/// (matching the C `void*`).  C: `picoquic_setup_test_aead_context`.
pub fn picoquic_setup_test_aead_context(
    _is_encrypt: bool,
    _secret: &[u8],
    _prefix_label: &str,
) -> *mut c_void {
    todo!()
}

/// Construct a PN-encryption context for tests.  C:
/// `picoquic_pn_enc_create_for_test`.
pub fn picoquic_pn_enc_create_for_test(_secret: &[u8], _prefix_label: &str) -> *mut c_void {
    todo!()
}

// ---------------------------------------------------------------------------
// Reset secret and verify-certificate management.

/// Compute the 16-byte reset secret tied to `cnx_id` using the
/// per-`quic` reset seed.  C:
/// `picoquic_create_cnxid_reset_secret`.
pub fn picoquic_create_cnxid_reset_secret(
    _quic: &mut picoquic_quic_t,
    _cnx_id: &picoquic_connection_id_t,
    _reset_secret: &mut [u8; PICOQUIC_RESET_SECRET_SIZE],
) -> Result<(), ()> {
    todo!()
}

/// Install a custom certificate-verification callback into the
/// master TLS context, replacing any previously installed one.  C:
/// `picoquic_tls_set_verify_certificate_callback`.
///
/// `cb` is owned by the registry; the C side stored the bare
/// pointer.  `free_fn` runs when the verifier is replaced or the
/// master context is freed; `None` matches the C `NULL` "no
/// teardown hook" sentinel.  The `Box` makes the ownership
/// transfer explicit; clippy's `boxed_local` is silenced because
/// the Box is the contract, not the implementation.
#[allow(clippy::boxed_local)]
pub fn picoquic_tls_set_verify_certificate_callback(
    _quic: &mut picoquic_quic_t,
    _cb: Box<ptls_verify_certificate_t>,
    _free_fn: Option<Box<dyn FreeVerifyCertificateCtx>>,
) {
    todo!()
}

/// Tear down whatever certificate-verifier callback is currently
/// installed in `quic`'s master TLS context.  C:
/// `picoquic_dispose_verify_certificate_callback`.
pub fn picoquic_dispose_verify_certificate_callback(_quic: &mut picoquic_quic_t) {
    todo!()
}

/// Toggle whether the server requires client certificates.  C
/// took an `int`; promoted to `bool`.  C:
/// `picoquic_tls_set_client_authentication`.
pub fn picoquic_tls_set_client_authentication(
    _quic: &mut picoquic_quic_t,
    _client_authentication: bool,
) {
    todo!()
}

/// Report whether client authentication is currently required.  C:
/// `picoquic_tls_client_authentication_activated`.
pub fn picoquic_tls_client_authentication_activated(_quic: &picoquic_quic_t) -> bool {
    todo!()
}

/// Toggle whether picotls exposes its exporter API on this master
/// context.  C: `picoquic_tls_set_use_exporter`.
pub fn picoquic_tls_set_use_exporter(_quic: &mut picoquic_quic_t, _use_exporter: bool) {
    todo!()
}

// ---------------------------------------------------------------------------
// Retry tokens.
//
// `picoquic_server_decrypt_retry_token`,
// `picoquic_prepare_retry_token`, and
// `picoquic_verify_retry_token` were the three token entry points
// declared in this header.  All three take a `struct sockaddr*`
// peer address (mapped to [`SocketAddr`]) and produce / consume a
// caller-allocated `token` buffer.

/// Output bundle from [`picoquic_server_decrypt_retry_token`].  C
/// returned the new-token flag through `int*` and the plaintext
/// length through `size_t*`; collapsing them into a struct keeps
/// the result one piece.  The plaintext is written into the
/// caller's `text` slice; `text_length` is how many bytes were
/// actually written.
pub struct PicoquicDecryptedRetryToken {
    /// Mirrors the C `int* is_new_token` out-parameter; promoted
    /// to `bool`.
    pub is_new_token: bool,
    /// Mirrors the C `size_t* text_length` out-parameter.
    pub text_length: usize,
}

/// Decrypt a retry token and verify its peer-address binding.  The
/// plaintext is written into `text`; the result describes how many
/// bytes were written and whether the token was a "new token" or a
/// classic retry token.  C: `picoquic_server_decrypt_retry_token`.
pub fn picoquic_server_decrypt_retry_token(
    _quic: &mut picoquic_quic_t,
    _addr_peer: &SocketAddr,
    _token: &[u8],
    _text: &mut [u8],
) -> Result<PicoquicDecryptedRetryToken, ()> {
    todo!()
}

/// Construct a retry / new token signed for `addr_peer`.  Returns
/// the number of bytes written into `token`; `token_max` is
/// `token.len()`.  C: `picoquic_prepare_retry_token`.
pub fn picoquic_prepare_retry_token(
    _quic: &mut picoquic_quic_t,
    _addr_peer: &SocketAddr,
    _current_time: u64,
    _odcid: &picoquic_connection_id_t,
    _rcid: &picoquic_connection_id_t,
    _initial_pn: u32,
    _token: &mut [u8],
) -> Result<usize, ()> {
    todo!()
}

/// Output bundle from [`picoquic_verify_retry_token`].  Mirrors the
/// `int* is_new_token` and `picoquic_connection_id_t* odcid`
/// out-parameters of the C signature.
pub struct PicoquicVerifiedRetryToken {
    /// Mirrors the C `int* is_new_token`; promoted to `bool`.
    pub is_new_token: bool,
    /// Original Destination Connection ID extracted from the
    /// token.  Empty (`id_len == 0`) for "new tokens".
    pub odcid: picoquic_connection_id_t,
}

/// Verify a retry / new token presented by the peer.  Returns the
/// extracted bundle when the token is fresh, address-bound, and
/// (for retry tokens) RCID-matched.  C:
/// `picoquic_verify_retry_token`.
pub fn picoquic_verify_retry_token(
    _quic: &mut picoquic_quic_t,
    _addr_peer: &SocketAddr,
    _current_time: u64,
    _rcid: &picoquic_connection_id_t,
    _initial_pn: u32,
    _token: &[u8],
    _check_reuse: bool,
) -> Result<PicoquicVerifiedRetryToken, ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Hash helpers exposed so applications don't need picotls.h.

/// Maximum digest size across the hash algorithms picotls supports.
/// C: `PICOQUIC_HASH_SIZE_MAX`.
pub const PICOQUIC_HASH_SIZE_MAX: usize = 64;

/// Construct a streaming hash context for the named algorithm
/// (e.g. `"sha256"`).  Returns null on lookup failure (matching the
/// C `void*`).  C: `picoquic_hash_create`.
pub fn picoquic_hash_create(_algorithm_name: &str) -> *mut c_void {
    todo!()
}

/// Digest length (bytes) of the named hash algorithm, or 0 when
/// the algorithm is unknown.  C: `picoquic_hash_get_length`.
pub fn picoquic_hash_get_length(_algorithm_name: &str) -> usize {
    todo!()
}

/// Push `input` into the streaming hash context.  C signature
/// passed `(uint8_t* input, size_t input_length, void*
/// hash_context)`; the Rust shape collapses the pointer/length pair
/// into a slice.  C: `picoquic_hash_update`.
///
/// # Safety
///
/// `hash_context` must point to a hash context returned by
/// [`picoquic_hash_create`] and not yet finalized.
pub unsafe fn picoquic_hash_update(_input: &[u8], _hash_context: *mut c_void) {
    todo!()
}

/// Finalize the hash context, writing the digest into `output` and
/// freeing the context.  `output` must be at least
/// `picoquic_hash_get_length(algorithm)` bytes long; phase 1 keeps
/// the caller-sized slice shape from the C signature.  C:
/// `picoquic_hash_finalize`.
///
/// # Safety
///
/// `hash_context` must point to a hash context returned by
/// [`picoquic_hash_create`] and not yet finalized.  After the call
/// the context is consumed.
pub unsafe fn picoquic_hash_finalize(_output: &mut [u8], _hash_context: *mut c_void) {
    todo!()
}

// ---------------------------------------------------------------------------
// Private-key / certificate file loaders.

/// Load a PEM-encoded private key from `file_name` and install it
/// in `quic`'s master TLS context.  C:
/// `picoquic_set_private_key_from_file`.
pub fn picoquic_set_private_key_from_file(
    _quic: &mut picoquic_quic_t,
    _file_name: &str,
) -> Result<(), ()> {
    todo!()
}

/// Load a PEM-encoded certificate chain from `file_name` and
/// return it as an owned vector.  C:
/// `picoquic_get_certs_from_file`.  The C side allocated both the
/// outer `ptls_iovec_t*` array and each `base` slot; the Rust
/// shape is `Option<Vec<ptls_iovec_t>>` because the iovec slot
/// type is still opaque (its `base`/`len` fields aren't surfaced
/// through `ptls_iovec_t` in `picoquic.rs`).  Callers free by
/// dropping the vector.  Returns `None` when the loader callback
/// is unset or the file fails to parse.
pub fn picoquic_get_certs_from_file(_file_name: &str) -> Option<Vec<ptls_iovec_t>> {
    todo!()
}

// ---------------------------------------------------------------------------
// Retry-packet integrity protection.
//
// These manage a small set of AEAD contexts (one per supported
// QUIC version) used to compute retry-packet integrity tags.

/// Build a retry-protection AEAD context from the retry integrity
/// key for a given version.  Returns null on failure (matching C
/// `void*`).  C: `picoquic_create_retry_protection_context`.
pub fn picoquic_create_retry_protection_context(
    _is_enc: bool,
    _key: &[u8],
    _prefix_label: &str,
) -> *mut c_void {
    todo!()
}

/// Look up (and lazily create) the retry-protection context for the
/// chosen QUIC version, on the chosen direction.  C:
/// `picoquic_find_retry_protection_context`.
pub fn picoquic_find_retry_protection_context(
    _quic: &mut picoquic_quic_t,
    _version_index: i32,
    _sending: bool,
) -> *mut c_void {
    todo!()
}

/// Tear down every retry-protection AEAD context held by `quic`.
/// C: `picoquic_delete_retry_protection_contexts`.
pub fn picoquic_delete_retry_protection_contexts(_quic: &mut picoquic_quic_t) {
    todo!()
}

/// Append the integrity tag to the bytes already written into the
/// retry packet buffer.  Returns the new write index.  C:
/// `picoquic_encode_retry_protection`.
///
/// # Safety
///
/// `integrity_aead` may be null (the C path is a no-op then); when
/// non-null it must point to a valid `ptls_aead_context_t`.
pub unsafe fn picoquic_encode_retry_protection(
    _integrity_aead: *mut c_void,
    _bytes: &mut [u8],
    _byte_index: usize,
    _odcid: &picoquic_connection_id_t,
) -> usize {
    todo!()
}

/// Verify the integrity tag at the end of an inbound retry packet.
/// `length` is updated in place to strip the tag on success.  C:
/// `picoquic_verify_retry_protection`.
///
/// # Safety
///
/// `integrity_aead` must point to a valid `ptls_aead_context_t`.
pub unsafe fn picoquic_verify_retry_protection(
    _integrity_aead: *mut c_void,
    _bytes: &mut [u8],
    _length: &mut usize,
    _byte_index: usize,
    _odcid: &picoquic_connection_id_t,
) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Cipher-suite accessors and ECB cipher for CID encryption.

/// Look up a cipher suite by its TLS code-point.  Returns null when
/// no provider supplies one (matching the C `void*`).  C:
/// `picoquic_get_cipher_suite_by_id_v`.
pub fn picoquic_get_cipher_suite_by_id_v(
    _cipher_suite_id: i32,
    _use_low_memory: bool,
) -> *mut c_void {
    todo!()
}

/// Look up the AES-128-GCM-SHA256 cipher suite (used for Initial
/// packets).  C: `picoquic_get_aes128gcm_sha256_v`.
pub fn picoquic_get_aes128gcm_sha256_v(_use_low_memory: bool) -> *mut c_void {
    todo!()
}

/// Look up just the AEAD algorithm slot of the AES-128-GCM cipher
/// suite.  C: `picoquic_get_aes128gcm_v`.
pub fn picoquic_get_aes128gcm_v(_use_low_memory: bool) -> *mut c_void {
    todo!()
}

/// Build an ECB-mode cipher context by algorithm name (used by the
/// load-balancer CID encryption flow).  Returns null on lookup
/// failure.  C: `picoquic_ecb_create_by_name`.
pub fn picoquic_ecb_create_by_name(_is_enc: bool, _ecb_key: &[u8], _alg_name: &str) -> *mut c_void {
    todo!()
}

/// Convenience wrapper for AES-128-ECB.  C:
/// `picoquic_aes128_ecb_create`.
pub fn picoquic_aes128_ecb_create(_is_enc: bool, _ecb_key: &[u8]) -> *mut c_void {
    todo!()
}

/// Free an ECB cipher context allocated by
/// [`picoquic_aes128_ecb_create`] / [`picoquic_ecb_create_by_name`].
/// C: `picoquic_aes128_ecb_free`.
///
/// # Safety
///
/// `v_aesecb` must point to a `ptls_cipher_context_t` previously
/// allocated by an ECB-cipher constructor and not yet freed.
pub unsafe fn picoquic_aes128_ecb_free(_v_aesecb: *mut c_void) {
    todo!()
}

/// Encrypt `input` (length `len`) into `output` using the ECB
/// cipher context.  C: `picoquic_aes128_ecb_encrypt`.
///
/// # Safety
///
/// `v_aesecb` must point to a valid encrypting `ptls_cipher_context_t`.
pub unsafe fn picoquic_aes128_ecb_encrypt(
    _v_aesecb: *mut c_void,
    _output: &mut [u8],
    _input: &[u8],
) {
    todo!()
}

// ---------------------------------------------------------------------------
// TLS API initialization.
//
// These four functions wrap the global crypto-provider registry's
// load / unload / reset cycle.  The flag word is the same
// `TLS_API_INIT_FLAGS_*` bit set declared in
// `picoquic_crypto_provider_api.rs`.

/// Idempotent first-time initialization of the crypto provider
/// registry.  C: `picoquic_tls_api_init`.
pub fn picoquic_tls_api_init() {
    todo!()
}

/// Tear down the crypto provider registry.  C:
/// `picoquic_tls_api_unload`.
pub fn picoquic_tls_api_unload() {
    todo!()
}

/// Reset the crypto provider registry, applying a new
/// `TLS_API_INIT_FLAGS_*` mask.  Used by the test suite to swap
/// providers in/out.  C: `picoquic_tls_api_reset`.
pub fn picoquic_tls_api_reset(_init_flags: u64) {
    todo!()
}

/// Log the loaded provider versions to the connection's app-message
/// stream.  C: `picoquic_tls_api_log_versions`.
pub fn picoquic_tls_api_log_versions(_cnx: &mut picoquic_cnx_t) {
    todo!()
}

#[cfg(test)]
mod test {}
