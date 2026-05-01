//! Translation of `picoquic/picoquic_crypto_provider_api.h`.
//!
//! Plug-in interface between picoquic-core and the swappable TLS /
//! crypto providers (OpenSSL, minicrypto, fusion, mbedtls).  Each
//! provider calls `picoquic_register_*` at startup to install its
//! ciphersuites, key-exchange algorithms, HPKE primitives, and the
//! handful of file-/key-handling callbacks that picoquic-core itself
//! does not implement.
//!
//! Phase 1: signatures only — every body is `todo!()`.  The registry
//! storage (the `extern` arrays and function-pointer slots from the C
//! header) is accessed through getter functions whose bodies land in
//! Phase 3; that defers the question of how the single-threaded
//! global state is represented (`OnceCell`, `RefCell`, plain
//! `static mut` behind a safety boundary, …) until we have the rest
//! of `tls_api.c` in front of us.
//!
//! Pointer-shape decisions follow the Phase 1 rules:
//!
//! * Function-pointer typedefs become traits (one trait per typedef).
//!   None of the typedefs carry a `void* ctx` — picoquic.h's pattern
//!   of folding a context pointer into the trait implementor still
//!   applies because every implementation is backed by some
//!   provider-specific state (e.g. an OpenSSL `EVP_PKEY*`).
//! * picotls types this header references but does not define
//!   (`ptls_cipher_suite_t`, `ptls_key_exchange_algorithm_t`, …) are
//!   forward-declared as zero-sized opaque structs.  picotls is an
//!   external C dependency that has not been translated; the structs
//!   are declared here rather than in `picoquic.rs` because that
//!   module already pulls in `ptls_iovec_t` and
//!   `ptls_verify_certificate_t` and we re-use those rather than
//!   duplicating.  Any future picotls Rust binding can replace these
//!   placeholders without touching call sites.
//! * Owning-out-pointer C idioms (`uint8_t** pubkey, size_t *
//!   pubkey_len`, `ptls_iovec_t* (*)(…, size_t* count)`, etc.)
//!   collapse to `Result<Vec<…>, ()>` — the C callee always
//!   `malloc`s the buffer and the caller `free`s it, so a `Vec`
//!   captures both ownership and length faithfully.
//! * The `extern` registry globals (`picoquic_cipher_suites`,
//!   `picoquic_key_exchanges`, `picoquic_*_fn`, …) are exposed as
//!   accessor functions returning borrowed slices / trait references.
//!   The companion `_NB_MAX` length constants are kept as `pub const`
//!   so other modules that index by name can compile against them,
//!   even though the slice carries its own length.
//! * `picoquic_tls_ctx_t` is an internal struct; it does not cross
//!   any FFI boundary (it is allocated and consumed entirely inside
//!   `tls_api.c`), so `repr(C)` is dropped.  Heap arrays
//!   (`alpn_vec` + `alpn_vec_size`/`alpn_count`, `ext_data` +
//!   `ext_data_size`) become owning `Vec<T>`s; the fixed `app_secret_*`
//!   buffers become `[u8; PTLS_MAX_DIGEST_SIZE]` arrays.
//! * The `picoquic_register_*_fn` setters take
//!   `Option<Box<dyn Trait>>` so the C `NULL` sentinel ("opt out")
//!   maps to `None` and the registered impl is owned by the registry.

#![allow(non_camel_case_types)]
#![allow(clippy::result_unit_err)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::{picoquic_cnx_t, ptls_iovec_t, ptls_verify_certificate_t};

// ---------------------------------------------------------------------------
// `picoquic_tls_api_init` flag bits.

/// Suppress the OpenSSL provider during `picoquic_tls_api_init`.
/// C: `TLS_API_INIT_FLAGS_NO_OPENSSL`.
pub const TLS_API_INIT_FLAGS_NO_OPENSSL: u64 = 1;

/// Suppress the minicrypto provider during `picoquic_tls_api_init`.
/// C: `TLS_API_INIT_FLAGS_NO_MINICRYPTO`.
pub const TLS_API_INIT_FLAGS_NO_MINICRYPTO: u64 = 2;

/// Suppress the fusion provider during `picoquic_tls_api_init`.
/// C: `TLS_API_INIT_FLAGS_NO_FUSION`.
pub const TLS_API_INIT_FLAGS_NO_FUSION: u64 = 4;

/// Suppress the mbedtls provider during `picoquic_tls_api_init`.
/// C: `TLS_API_INIT_FLAGS_NO_MBEDTLS`.
pub const TLS_API_INIT_FLAGS_NO_MBEDTLS: u64 = 8;

// ---------------------------------------------------------------------------
// Forward declarations of picotls types referenced by this header.
//
// picotls is an external C dependency that has not been ported.  The
// structs below are zero-sized opaque types so signatures in this
// module compile; their layout and methods land when (or if) the
// picotls headers get a Rust counterpart.  Pointers across these
// types stay behind references (`&T` / `&mut T`) to keep `unsafe`
// out of Phase 1.

/// Forward declaration of `ptls_cipher_suite_t` from picotls.
#[derive(Debug)]
pub struct ptls_cipher_suite_t {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_key_exchange_algorithm_t` from picotls.
#[derive(Debug)]
pub struct ptls_key_exchange_algorithm_t {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_hpke_cipher_suite_t` from picotls.
#[derive(Debug)]
pub struct ptls_hpke_cipher_suite_t {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_hpke_kem_t` from picotls.
#[derive(Debug)]
pub struct ptls_hpke_kem_t {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_context_t` from picotls.  Setters in
/// this header mutate it in place.
#[derive(Debug)]
pub struct ptls_context_t {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_sign_certificate_t` from picotls.
#[derive(Debug)]
pub struct ptls_sign_certificate_t {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_t` from picotls (the per-connection
/// TLS state owned by `picoquic_tls_ctx_t`).
#[derive(Debug)]
pub struct ptls_t {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_raw_extension_t` from picotls.  Used
/// as a fixed two-element array inside `picoquic_tls_ctx_t`; phase 3
/// will replace this with the real definition or a dedicated wrapper.
#[derive(Debug, Default, Copy, Clone)]
pub struct ptls_raw_extension_t {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_handshake_properties_t` from picotls.
#[derive(Debug, Default)]
pub struct ptls_handshake_properties_t {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_key_exchange_context_t` from picotls.
#[derive(Debug)]
pub struct ptls_key_exchange_context_t {
    _opaque: [u8; 0],
}

// ---------------------------------------------------------------------------
// Constants borrowed from picotls.h.
//
// `PTLS_MAX_DIGEST_SIZE` is defined in `picotls.h`; it is replicated
// here as a `pub const` to size the per-context app-secret buffers
// without forcing every consumer of this module to depend on a
// (still-absent) picotls binding.  The value mirrors picotls 1.x.

/// Maximum digest size across the hash algorithms picotls supports.
/// Mirrors `PTLS_MAX_DIGEST_SIZE` in `picotls.h`.
pub const PTLS_MAX_DIGEST_SIZE: usize = 64;

// ---------------------------------------------------------------------------
// Provider registration: ciphersuites, key exchanges, HPKE.

/// Install a cipher suite in the global registry.  `is_low_memory`
/// is a flag (0 / non-zero in C); promoted to `bool`.  C:
/// `picoquic_register_ciphersuite`.
pub fn picoquic_register_ciphersuite(_suite: &'static ptls_cipher_suite_t, _is_low_memory: bool) {
    todo!()
}

/// Install a key-exchange algorithm in the global registry.  C:
/// `picoquic_register_key_exchange_algorithm`.
pub fn picoquic_register_key_exchange_algorithm(
    _key_exchange: &'static ptls_key_exchange_algorithm_t,
) {
    todo!()
}

/// Install an HPKE cipher suite in the global registry.  C:
/// `picoquic_register_hpke_cipher_suite`.
pub fn picoquic_register_hpke_cipher_suite(_hpke_cipher_suite: &'static ptls_hpke_cipher_suite_t) {
    todo!()
}

/// Install an HPKE KEM in the global registry.  C:
/// `picoquic_register_hpke_kem`.
pub fn picoquic_register_hpke_kem(_hpke_kem: &'static ptls_hpke_kem_t) {
    todo!()
}

// ---------------------------------------------------------------------------
// Function-pointer typedefs → traits.
//
// The C side sets each slot to a single fn pointer at init time; the
// Rust side stores a trait object instead.  None of these typedefs
// carry a `void* ctx`, but provider implementations always have
// internal state (e.g. an OpenSSL handle), so the trait methods take
// `&mut self`.

/// C: `SetTlsKeyProvider`.  Installs a key provider in
/// a TLS context using a raw key blob.  Returns 0 on success in C;
/// modeled as `Result<(), ()>` until the crate-level `Error` enum
/// lands.
pub trait SetTlsKeyProvider {
    fn set(&mut self, ctx: &mut ptls_context_t, data: &[u8]) -> Result<(), ()>;
}

/// C: `GetPrivateKeyFromFile`.  Reads a private key
/// PEM file and returns its DER-encoded bytes.  The C callee
/// `malloc`s the buffer and writes its length through `int*
/// key_length`; the Rust shape returns an owning `Vec<u8>` that
/// captures both, with `None` standing in for the C `NULL` failure
/// sentinel.
pub trait GetPrivateKeyFromFile {
    fn get(&mut self, file_name: &str) -> Option<Vec<u8>>;
}

/// C: `SetPrivateKeyFromFile`.  Reads a PEM file and
/// installs the resulting key in `ctx`.  Returns 0 on success in C.
pub trait SetPrivateKeyFromFile {
    fn set(&mut self, keypem: &str, ctx: &mut ptls_context_t) -> Result<(), ()>;
}

/// C: `GetPublicKeyFromPrivate`.  Reads a private-key
/// PEM file and produces the matching public key DER.  C signature
/// returned the bytes through `uint8_t** pubkey` plus `size_t*
/// pubkey_len`; the owning out-pointer collapses to `Vec<u8>`.
pub trait GetPublicKeyFromPrivate {
    fn get(&mut self, keypem: &str) -> Result<Vec<u8>, ()>;
}

/// C: `DisposeSignCertificate`.  Provider-specific
/// teardown for a sign-certificate vtable.  Phase 3 may collapse
/// into `Drop` once the `ptls_sign_certificate_t` binding is real.
pub trait DisposeSignCertificate {
    fn dispose(&mut self, cert: &mut ptls_sign_certificate_t);
}

/// C: `GetCertsFromFile`.  Reads a PEM bundle and
/// returns the certificate chain as a sequence of iovecs.  The C
/// out-pointer pair (`ptls_iovec_t*` + `size_t* count`) collapses
/// to `Vec<ptls_iovec_t>`.
pub trait GetCertsFromFile {
    fn get(&mut self, file_name: &str) -> Option<Vec<ptls_iovec_t>>;
}

/// C: `DisposeCertificateVerifier`.  Provider-specific
/// teardown for a verifier vtable.
pub trait DisposeCertificateVerifier {
    fn dispose(&mut self, verifier: &mut ptls_verify_certificate_t);
}

/// Output bundle from
/// [`GetCertificateVerifier::get`].  `is_cert_store_not_empty`
/// and the disposer were separate out-parameters in C; grouping them
/// here keeps the trait method signature one-out, one-return.
pub struct PicoquicCertificateVerifier {
    /// Owned verifier vtable handed back to the caller.  C: the
    /// `ptls_verify_certificate_t*` returned by value.
    pub verifier: Box<ptls_verify_certificate_t>,
    /// Mirrors the C `unsigned int* is_cert_store_not_empty`
    /// out-parameter.  Promoted to `bool`.
    pub is_cert_store_not_empty: bool,
    /// Mirrors the C `DisposeCertificateVerifier*`
    /// out-parameter.  `None` matches the C sentinel where the
    /// provider has no teardown hook to register.
    pub free_certificate_verifier_fn: Option<Box<dyn DisposeCertificateVerifier>>,
}

/// C: `GetCertificateVerifier`.  Returns a freshly
/// allocated verifier plus its sidekick metadata.
pub trait GetCertificateVerifier {
    fn get(&mut self, cert_root_file_name: &str) -> Option<PicoquicCertificateVerifier>;
}

/// C: `SetTlsRootCertificates`.  Installs a root-CA
/// bundle in `ctx`.  The C `(ptls_iovec_t* certs, size_t count)`
/// pair collapses to a borrowed slice.
pub trait SetTlsRootCertificates {
    fn set(&mut self, ctx: &mut ptls_context_t, certs: &[ptls_iovec_t]) -> Result<(), ()>;
}

/// C: `ExplainCryptoError`.  Reports the most recent
/// crypto error's source location.  The C signature filled
/// `*err_file` (a borrowed pointer into static storage) and
/// `*err_line`; the Rust shape returns them as a tuple, with
/// `None` for "no current error".  Returns 0/-1 in C; the
/// presence of the tuple captures success.
pub trait ExplainCryptoError {
    fn explain(&mut self) -> Option<(&'static str, i32)>;
}

/// C: `ClearCryptoErrors`.  Drains the provider's
/// thread-local error queue.
pub trait ClearCryptoErrors {
    fn clear(&mut self);
}

/// C: `SetRandomProviderInCtx`.  Installs a
/// provider-specific RNG into a TLS context.  Unused in the current
/// C tree but kept on the API surface.
pub trait SetRandomProviderInCtx {
    fn install(&mut self, ctx: &mut ptls_context_t);
}

/// C: `CryptoRandomProvider`.  Fills `buf` with random
/// bytes.  The C `(void* buf, size_t len)` pair collapses to
/// `&mut [u8]`.
pub trait CryptoRandomProvider {
    fn random(&mut self, buf: &mut [u8]);
}

/// C: `KeyexFromKeyFile`.  Reads a private-key PEM
/// file and constructs a key-exchange context.  C returned the
/// allocated context through `ptls_key_exchange_context_t**`; the
/// Rust shape returns an owning `Box`.
pub trait KeyexFromKeyFile {
    fn create(&mut self, keypem: &str) -> Result<Box<ptls_key_exchange_context_t>, ()>;
}

/// C: `KeyexDispose`.  Provider-specific teardown for a
/// key-exchange context.
pub trait KeyexDispose {
    fn dispose(&mut self, keyex: &mut ptls_key_exchange_context_t);
}

// ---------------------------------------------------------------------------
// Provider registration: callbacks (function-pointer slots).
//
// The C `register_*_fn` calls install a set of fn pointers in the
// global registry.  The Rust shape takes `Option<Box<dyn Trait>>`
// per slot — `None` is the C `NULL` sentinel for "opt out", and the
// `Box` owns the implementor's state.

/// C: `picoquic_register_tls_key_provider_fn`.  Installs the
/// four-function key-provider bundle (private-key import, sign-cert
/// disposer, cert-chain reader, public-key derivation).
pub fn picoquic_register_tls_key_provider_fn(
    _set_private_key_from_file_fn: Option<Box<dyn SetPrivateKeyFromFile>>,
    _dispose_sign_certificate_fn: Option<Box<dyn DisposeSignCertificate>>,
    _get_certs_from_file_fn: Option<Box<dyn GetCertsFromFile>>,
    _get_public_key_from_private_fn: Option<Box<dyn GetPublicKeyFromPrivate>>,
) {
    todo!()
}

/// C: `picoquic_register_verify_certificate_fn`.  Installs the
/// three-function certificate-verifier bundle.
pub fn picoquic_register_verify_certificate_fn(
    _certificate_verifier_fn: Option<Box<dyn GetCertificateVerifier>>,
    _dispose_certificate_verifier_fn: Option<Box<dyn DisposeCertificateVerifier>>,
    _set_tls_root_certificates_fn: Option<Box<dyn SetTlsRootCertificates>>,
) {
    todo!()
}

/// C: `picoquic_register_explain_crypto_error_fn`.  Installs the
/// error-explanation hook pair.
pub fn picoquic_register_explain_crypto_error_fn(
    _explain_crypto_error_fn: Option<Box<dyn ExplainCryptoError>>,
    _clear_crypto_errors_fn: Option<Box<dyn ClearCryptoErrors>>,
) {
    todo!()
}

/// C: `picoquic_register_crypto_random_provider_fn`.  Installs the
/// global RNG provider.
pub fn picoquic_register_crypto_random_provider_fn(
    _random_provider: Option<Box<dyn CryptoRandomProvider>>,
) {
    todo!()
}

/// C: `picoquic_register_keyex_from_key_file_fn`.  Installs the
/// key-exchange constructor / disposer pair.
pub fn picoquic_register_keyex_from_key_file_fn(
    _keyex_from_key_file_fn: Option<Box<dyn KeyexFromKeyFile>>,
    _keyex_dispose_fn: Option<Box<dyn KeyexDispose>>,
) {
    todo!()
}

// ---------------------------------------------------------------------------
// Registry size limits (kept as `pub const` per the comment block at
// the top of this module — slices carry their length, but other
// modules index into the registry by name).

/// C: `PICOQUIC_CIPHER_SUITES_NB_MAX`.
pub const PICOQUIC_CIPHER_SUITES_NB_MAX: usize = 8;

/// C: `PICOQUIC_KEY_EXCHANGES_NB_MAX`.
pub const PICOQUIC_KEY_EXCHANGES_NB_MAX: usize = 4;

/// C: `PICOQUIC_HPKE_CIPHER_SUITE_NB_MAX`.
pub const PICOQUIC_HPKE_CIPHER_SUITE_NB_MAX: usize = 4;

/// C: `PICOQUIC_HPKE_KEM_NB_MAX`.
pub const PICOQUIC_HPKE_KEM_NB_MAX: usize = 3;

/// One entry in the cipher-suite registry: a high-memory and an
/// optional low-memory implementation of the same suite, paired so
/// the stack can downshift on memory-constrained connections.  C:
/// `struct st_picoquic_cipher_suites_t`.  Both slots are nullable in
/// C — promoted to `Option<&'static …>`.
#[derive(Debug)]
pub struct picoquic_cipher_suites_t {
    pub high_memory_suite: Option<&'static ptls_cipher_suite_t>,
    pub low_memory_suite: Option<&'static ptls_cipher_suite_t>,
}

// ---------------------------------------------------------------------------
// Registry accessors (replace the `extern` globals from the header).
//
// Phase 3 will pick the storage; Phase 1 just exposes the read-side
// API surface.  The `_NB_MAX + 1` C arrays kept a NULL sentinel at
// the end; the Rust slices return populated entries only — the
// `Option` inside `picoquic_cipher_suites_t` already encodes
// "missing".

/// Read access to `picoquic_cipher_suites`.
pub fn picoquic_cipher_suites() -> &'static [picoquic_cipher_suites_t] {
    todo!()
}

/// Read access to `picoquic_key_exchanges`.
pub fn picoquic_key_exchanges() -> &'static [&'static ptls_key_exchange_algorithm_t] {
    todo!()
}

/// Read access to `picoquic_key_exchange_secp256r1`.  C declared a
/// fixed two-element array (slot + NULL terminator); the Rust
/// accessor exposes only the populated entry, or `None` when the
/// secp256r1 implementation has not been registered.
pub fn picoquic_key_exchange_secp256r1() -> Option<&'static ptls_key_exchange_algorithm_t> {
    todo!()
}

/// Read access to `picoquic_hpke_cipher_suites`.
pub fn picoquic_hpke_cipher_suites() -> &'static [&'static ptls_hpke_cipher_suite_t] {
    todo!()
}

/// Read access to `picoquic_hpke_kems`.
pub fn picoquic_hpke_kems() -> &'static [&'static ptls_hpke_kem_t] {
    todo!()
}

/// Read access to the registered `picoquic_set_private_key_from_file_fn`
/// callback.
pub fn picoquic_set_private_key_from_file_fn() -> Option<&'static mut dyn SetPrivateKeyFromFile> {
    todo!()
}

/// Read access to the registered `picoquic_dispose_sign_certificate_fn`
/// callback.
pub fn picoquic_dispose_sign_certificate_fn() -> Option<&'static mut dyn DisposeSignCertificate> {
    todo!()
}

/// Read access to the registered `picoquic_get_certs_from_file_fn`
/// callback.
pub fn picoquic_get_certs_from_file_fn() -> Option<&'static mut dyn GetCertsFromFile> {
    todo!()
}

/// Read access to the registered `picoquic_get_public_key_from_private_fn`
/// callback.
pub fn picoquic_get_public_key_from_private_fn() -> Option<&'static mut dyn GetPublicKeyFromPrivate>
{
    todo!()
}

/// Read access to the registered `picoquic_get_certificate_verifier_fn`
/// callback.
pub fn picoquic_get_certificate_verifier_fn() -> Option<&'static mut dyn GetCertificateVerifier> {
    todo!()
}

/// Read access to the registered `picoquic_dispose_certificate_verifier_fn`
/// callback.
pub fn picoquic_dispose_certificate_verifier_fn()
-> Option<&'static mut dyn DisposeCertificateVerifier> {
    todo!()
}

/// Read access to the registered `picoquic_set_tls_root_certificates_fn`
/// callback.
pub fn picoquic_set_tls_root_certificates_fn() -> Option<&'static mut dyn SetTlsRootCertificates> {
    todo!()
}

/// Read access to the registered `picoquic_explain_crypto_error_fn`
/// callback.
pub fn picoquic_explain_crypto_error_fn() -> Option<&'static mut dyn ExplainCryptoError> {
    todo!()
}

/// Read access to the registered `picoquic_clear_crypto_errors_fn`
/// callback.
pub fn picoquic_clear_crypto_errors_fn() -> Option<&'static mut dyn ClearCryptoErrors> {
    todo!()
}

/// Read access to the registered `picoquic_crypto_random_provider_fn`
/// callback.
pub fn picoquic_crypto_random_provider_fn() -> Option<&'static mut dyn CryptoRandomProvider> {
    todo!()
}

/// Read access to the registered `picoquic_keyex_from_key_file_fn`
/// callback.
pub fn picoquic_keyex_from_key_file_fn() -> Option<&'static mut dyn KeyexFromKeyFile> {
    todo!()
}

/// Read access to the registered `picoquic_keyex_dispose_fn` callback.
pub fn picoquic_keyex_dispose_fn() -> Option<&'static mut dyn KeyexDispose> {
    todo!()
}

// ---------------------------------------------------------------------------
// Per-connection TLS context.

/// Per-connection TLS state.  C: `struct st_picoquic_tls_ctx_t`.
///
/// `repr(C)` is dropped: the struct is allocated, populated, and
/// freed entirely within `tls_api.c` — no caller in the C tree
/// inspects it across an FFI boundary.  Heap-arrays from the C
/// shape (`alpn_vec` + `alpn_vec_size` / `alpn_count`, `ext_data` +
/// `ext_data_size`) become owning `Vec<T>`s; capacity-vs-length
/// tracking moves into the vector.  `tls`, `cnx`, and the picotls
/// extension array stay as their forward-declared shapes; phase 3
/// will pick `Box` vs. `&mut` once the picotls bindings settle.
///
/// `Debug` is intentionally not derived: `ptls_iovec_t`
/// (forward-declared in `picoquic.rs`) is opaque and does not
/// implement `Debug`, and adding it there is out of scope for this
/// header's translation.
pub struct picoquic_tls_ctx_t {
    /// Owned picotls connection state.  C allocated this with
    /// `ptls_new`; phase 3 will model the ownership transfer.
    pub tls: Option<Box<ptls_t>>,
    /// Back-pointer to the connection that owns this context.  Not
    /// owned — the connection outlives the TLS context.  Borrows
    /// will be sorted out in phase 3 when the surrounding
    /// connection lifetime is mapped.
    pub cnx: Option<*mut picoquic_cnx_t>,
    /// `int client_mode` in C is a 0/1 flag — promoted to `bool`.
    pub client_mode: bool,
    /// QUIC-transport-parameter raw extensions buffer.  C declared
    /// a fixed two-element array; preserved verbatim.
    pub ext: [ptls_raw_extension_t; 2],
    /// Retry-config opaque blob handed to picotls.
    pub retry_configs: ptls_iovec_t,
    /// picotls handshake-properties slot.  Embedded by value as in C.
    pub handshake_properties: ptls_handshake_properties_t,
    /// ALPN list passed to picotls during handshake.  C kept the
    /// owned buffer pointer plus a max size and a current count;
    /// the Rust `Vec` collapses both length tracks into its `len()`,
    /// while `capacity()` covers the C `alpn_vec_size` invariant.
    pub alpn_vec: Vec<ptls_iovec_t>,
    /// QUIC transport-parameter encode/decode scratch buffer.  C
    /// allocated `ext_data_size` bytes and stored that size
    /// alongside; the Rust shape uses the vector's capacity.
    pub ext_data: Vec<u8>,
    /// Forward-secret key for application-data encryption.
    pub app_secret_enc: [u8; PTLS_MAX_DIGEST_SIZE],
    /// Forward-secret key for application-data decryption.
    pub app_secret_dec: [u8; PTLS_MAX_DIGEST_SIZE],
}

// `picoquic_mbedtls_get_certificate_verifier` is gated on the C
// `#ifdef PICOQUIC_WITH_MBEDTLS` branch.  v1 targets the canonical
// build with mbedtls disabled, so the function is dropped here per
// the Phase 0 ifdef policy ("non-target branches are not
// translated in v1; other platforms get added back later behind
// `#[cfg(…)]`").

#[cfg(test)]
mod test {}
