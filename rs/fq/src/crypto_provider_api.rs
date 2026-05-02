//! Translation of `picoquic/picoquic_crypto_provider_api.h`.
//!
//! Plug-in interface between quic-core and the swappable TLS /
//! crypto providers (OpenSSL, minicrypto, fusion, mbedtls).  Each
//! provider calls `register_*` at startup to install its
//! ciphersuites, key-exchange algorithms, HPKE primitives, and the
//! handful of file-/key-handling callbacks that quic-core itself
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
//! * Function-pointer typedefs become traits (one trait per typedef)
//!   with `&self` methods.  None of the C typedefs carry a `void*
//!   ctx`, mirroring fn pointers with no closure state, so taking
//!   `&self` matches the C semantics — implementors that need
//!   mutable provider state (an OpenSSL handle, a thread-local error
//!   queue) reach for interior mutability.  This also lets the
//!   registry hand out plain shared references rather than
//!   `&'static mut` aliases.
//! * tls types this header references but does not define
//!   (`PtlsCipherSuite`, `PtlsKeyExchangeAlgorithm`, …) are
//!   forward-declared here as zero-sized opaque structs, since tls
//!   is an external C dependency that has not been translated.  Any
//!   future tls Rust binding can replace these placeholders without
//!   touching call sites.
//! * Owning-out-pointer C idioms (`uint8_t** pubkey, size_t *
//!   pubkey_len`, `PtlsIovec* (*)(…, size_t* count)`, etc.)
//!   collapse to `Result<Vec<…>, Error>` — the C callee always
//!   `malloc`s the buffer and the caller `free`s it, so a `Vec`
//!   captures both ownership and length faithfully.
//! * The `extern` registry globals (`picoquic_cipher_suites`,
//!   `picoquic_key_exchanges`, `picoquic_*_fn`, …) are exposed as
//!   accessor functions returning borrowed slices / trait references.
//!   The companion `_NB_MAX` length constants are kept as `pub const`
//!   so other modules that index by name can compile against them,
//!   even though the slice carries its own length.
//! * `TlsCtx` is an internal struct; it does not cross
//!   any FFI boundary (it is allocated and consumed entirely inside
//!   `tls_api.c`), so `repr(C)` is dropped.  Heap arrays
//!   (`alpn_vec` + `alpn_vec_size`/`alpn_count`, `ext_data` +
//!   `ext_data_size`) become owning `Vec<T>`s; the fixed `app_secret_*`
//!   buffers become `[u8; PTLS_MAX_DIGEST_SIZE]` arrays.
//! * The `register_*` setters take `Option<Box<dyn Trait>>` so the C
//!   `NULL` sentinel ("opt out") maps to `None` and the registered
//!   impl is owned by the registry.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::Error;
use crate::{Cnx, PtlsIovec, PtlsVerifyCertificate};

// ---------------------------------------------------------------------------
// `tls_api_init` flag bits.

/// Suppress the OpenSSL provider during `tls_api_init`.
/// C: `TLS_API_INIT_FLAGS_NO_OPENSSL`.
pub const TLS_API_INIT_FLAGS_NO_OPENSSL: u64 = 1;

/// Suppress the minicrypto provider during `tls_api_init`.
/// C: `TLS_API_INIT_FLAGS_NO_MINICRYPTO`.
pub const TLS_API_INIT_FLAGS_NO_MINICRYPTO: u64 = 2;

/// Suppress the fusion provider during `tls_api_init`.
/// C: `TLS_API_INIT_FLAGS_NO_FUSION`.
pub const TLS_API_INIT_FLAGS_NO_FUSION: u64 = 4;

/// Suppress the mbedtls provider during `tls_api_init`.
/// C: `TLS_API_INIT_FLAGS_NO_MBEDTLS`.
pub const TLS_API_INIT_FLAGS_NO_MBEDTLS: u64 = 8;

// ---------------------------------------------------------------------------
// Forward declarations of tls types referenced by this header.
//
// tls is an external C dependency that has not been ported.  The
// structs below are zero-sized opaque types so signatures in this
// module compile; their layout and methods land when (or if) the
// tls headers get a Rust counterpart.  Pointers across these
// types stay behind references (`&T` / `&mut T`) to keep `unsafe`
// out of Phase 1.

/// Forward declaration of `PtlsCipherSuite` from tls.
#[derive(Debug)]
pub struct PtlsCipherSuite {
    _opaque: [u8; 0],
}

/// Forward declaration of `PtlsKeyExchangeAlgorithm` from tls.
#[derive(Debug)]
pub struct PtlsKeyExchangeAlgorithm {
    _opaque: [u8; 0],
}

/// Forward declaration of `PtlsHpkeCipherSuite` from tls.
#[derive(Debug)]
pub struct PtlsHpkeCipherSuite {
    _opaque: [u8; 0],
}

/// Forward declaration of `PtlsHpkeKem` from tls.
#[derive(Debug)]
pub struct PtlsHpkeKem {
    _opaque: [u8; 0],
}

/// Forward declaration of `PtlsContext` from tls.  Setters in
/// this header mutate it in place.
#[derive(Debug)]
pub struct PtlsContext {
    _opaque: [u8; 0],
}

/// Forward declaration of `PtlsSignCertificate` from tls.
#[derive(Debug)]
pub struct PtlsSignCertificate {
    _opaque: [u8; 0],
}

/// Forward declaration of `ptls_t` from tls (the per-connection
/// TLS state owned by `TlsCtx`).
#[derive(Debug)]
pub struct Ptls {
    _opaque: [u8; 0],
}

/// Forward declaration of `PtlsRawExtension` from tls.  Used
/// as a fixed two-element array inside `TlsCtx`; phase 3
/// will replace this with the real definition or a dedicated wrapper.
#[derive(Debug, Default, Copy, Clone)]
pub struct PtlsRawExtension {
    _opaque: [u8; 0],
}

/// Forward declaration of `PtlsHandshakeProperties` from tls.
#[derive(Debug, Default)]
pub struct PtlsHandshakeProperties {
    _opaque: [u8; 0],
}

/// Forward declaration of `PtlsKeyExchangeContext` from tls.
#[derive(Debug)]
pub struct PtlsKeyExchangeContext {
    _opaque: [u8; 0],
}

// ---------------------------------------------------------------------------
// Constants borrowed from tls.h.
//
// `PTLS_MAX_DIGEST_SIZE` is defined in `tls.h`; it is replicated
// here as a `pub const` to size the per-context app-secret buffers
// without forcing every consumer of this module to depend on a
// (still-absent) tls binding.  The value mirrors tls 1.x.

/// Maximum digest size across the hash algorithms tls supports.
/// Mirrors `PTLS_MAX_DIGEST_SIZE` in `tls.h`.
pub const PTLS_MAX_DIGEST_SIZE: usize = 64;

// ---------------------------------------------------------------------------
// Provider registration: ciphersuites, key exchanges, HPKE.

/// Install a cipher suite in the global registry.  `is_low_memory`
/// distinguishes the reduced-memory variant of the suite from the
/// default; the C flag (0 / non-zero) is promoted to `bool`.  C:
/// `picoquic_register_ciphersuite`.
pub fn register_ciphersuite(_suite: &'static PtlsCipherSuite, _is_low_memory: bool) {
    todo!()
}

/// Install a key-exchange algorithm in the global registry.  C:
/// `picoquic_register_key_exchange_algorithm`.
pub fn register_key_exchange_algorithm(_key_exchange: &'static PtlsKeyExchangeAlgorithm) {
    todo!()
}

/// Install an HPKE cipher suite in the global registry.  C:
/// `picoquic_register_hpke_cipher_suite`.
pub fn register_hpke_cipher_suite(_hpke_cipher_suite: &'static PtlsHpkeCipherSuite) {
    todo!()
}

/// Install an HPKE KEM in the global registry.  C:
/// `picoquic_register_hpke_kem`.
pub fn register_hpke_kem(_hpke_kem: &'static PtlsHpkeKem) {
    todo!()
}

// ---------------------------------------------------------------------------
// Function-pointer typedefs → traits.
//
// The C side stores each slot as a single fn pointer set at init
// time; the Rust side stores a trait object instead.  None of these
// typedefs carry a `void* ctx`, mirroring fn pointers with no
// closure state.  Methods take `&self` to match: implementors that
// need mutable provider state (an OpenSSL handle, a thread-local
// error queue) use interior mutability.  This also keeps the
// registry accessors as plain shared references, avoiding the
// `&'static mut` aliasing smell.

/// Installs a private key, supplied as a raw blob, into a TLS
/// context.  C: `picoquic_set_tls_key_provider_t`.
pub trait SetTlsKeyProvider {
    fn set(&self, ctx: &mut PtlsContext, data: &[u8]) -> Result<(), Error>;
}

/// Reads a private key PEM file and returns its DER-encoded bytes.
/// `None` stands in for the C `NULL` failure sentinel; the C callee's
/// `malloc`'d buffer and `int* key_length` out-parameter collapse
/// into the owning `Vec<u8>`.  C: `picoquic_get_private_key_from_file_t`.
pub trait GetPrivateKeyFromFile {
    fn get(&self, file_name: &str) -> Option<Vec<u8>>;
}

/// Reads a PEM file and installs the resulting private key in `ctx`.
/// C: `picoquic_set_private_key_from_file_t`.
pub trait SetPrivateKeyFromFile {
    fn set(&self, keypem: &str, ctx: &mut PtlsContext) -> Result<(), Error>;
}

/// Reads a private-key PEM file and derives the matching public key
/// in DER form.  The C `uint8_t** pubkey` + `size_t* pubkey_len`
/// owning out-pair collapses to `Vec<u8>`.  C:
/// `picoquic_get_public_key_from_private_t`.
pub trait GetPublicKeyFromPrivate {
    fn get(&self, keypem: &str) -> Result<Vec<u8>, Error>;
}

/// Provider-specific teardown for a sign-certificate vtable.  Phase
/// 3 may collapse this into `Drop` once the `PtlsSignCertificate`
/// binding is real.  C: `picoquic_dispose_sign_certificate_t`.
pub trait DisposeSignCertificate {
    fn dispose(&self, cert: &mut PtlsSignCertificate);
}

/// Reads a PEM bundle and returns the certificate chain as a
/// sequence of iovecs.  The C `(PtlsIovec*, size_t* count)` owning
/// out-pair collapses to `Vec<PtlsIovec>`.  C:
/// `picoquic_get_certs_from_file_t`.
pub trait GetCertsFromFile {
    fn get(&self, file_name: &str) -> Option<Vec<PtlsIovec>>;
}

/// Provider-specific teardown for a certificate-verifier vtable.
/// C: `picoquic_dispose_certificate_verifier_t`.
pub trait DisposeCertificateVerifier {
    fn dispose(&self, verifier: &mut PtlsVerifyCertificate);
}

/// Output bundle from [`GetCertificateVerifier::get`].  The C
/// signature returned a verifier and filled two separate
/// out-parameters; grouping them in one struct keeps the trait
/// method's signature single-output.
pub struct CertificateVerifier {
    /// Owned verifier vtable.  Replaces the C
    /// `PtlsVerifyCertificate*` return value.
    pub verifier: Box<PtlsVerifyCertificate>,
    /// `true` when the underlying certificate store has at least one
    /// trust anchor loaded.  Mirrors the C
    /// `unsigned int* is_cert_store_not_empty` out-parameter.
    pub is_cert_store_not_empty: bool,
    /// Optional disposer to call when the verifier is no longer
    /// needed.  `None` matches the C sentinel where the provider has
    /// no teardown hook to register.
    pub dispose_verifier: Option<Box<dyn DisposeCertificateVerifier>>,
}

/// Constructs a verifier from a CA-bundle file path.  Returns the
/// verifier together with its companion metadata; `None` indicates
/// the provider could not produce a verifier.  C:
/// `picoquic_get_certificate_verifier_t`.
pub trait GetCertificateVerifier {
    fn get(&self, cert_root_file_name: &str) -> Option<CertificateVerifier>;
}

/// Installs a root-CA bundle in `ctx`.  The C
/// `(PtlsIovec* certs, size_t count)` pair collapses to a borrowed
/// slice.  C: `picoquic_set_tls_root_certificates_t`.
pub trait SetTlsRootCertificates {
    fn set(&self, ctx: &mut PtlsContext, certs: &[PtlsIovec]) -> Result<(), Error>;
}

/// Reports the source location of the most recent crypto error.  The
/// returned `&'static str` borrows into the provider's static error
/// table; `None` means there is no current error.  C:
/// `picoquic_explain_crypto_error_t`.
pub trait ExplainCryptoError {
    fn explain(&self) -> Option<(&'static str, i32)>;
}

/// Drains the provider's thread-local error queue.  C:
/// `picoquic_clear_crypto_errors_t`.
pub trait ClearCryptoErrors {
    fn clear(&self);
}

/// Installs a provider-specific RNG into a TLS context.  Unused in
/// the current C tree but kept on the API surface.  C:
/// `picoquic_set_random_provider_in_ctx_t`.
pub trait SetRandomProviderInCtx {
    fn install(&self, ctx: &mut PtlsContext);
}

/// Fills `buf` with cryptographically secure random bytes.  The C
/// `(void* buf, size_t len)` pair collapses to `&mut [u8]`.  C:
/// `picoquic_crypto_random_provider_t`.
pub trait CryptoRandomProvider {
    fn random(&self, buf: &mut [u8]);
}

/// Reads a private-key PEM file and constructs a key-exchange
/// context.  The C `PtlsKeyExchangeContext**` owning out-parameter
/// collapses to an owning `Box`.  C:
/// `picoquic_keyex_from_key_file_t`.
pub trait KeyexFromKeyFile {
    fn create(&self, keypem: &str) -> Result<Box<PtlsKeyExchangeContext>, Error>;
}

/// Provider-specific teardown for a key-exchange context.  C:
/// `picoquic_keyex_dispose_t`.
pub trait KeyexDispose {
    fn dispose(&self, keyex: &mut PtlsKeyExchangeContext);
}

// ---------------------------------------------------------------------------
// Provider registration: callbacks (function-pointer slots).
//
// Each of these installs a set of trait objects in the global
// registry.  Per-slot `None` is the C `NULL` sentinel for "opt
// out", and the `Box` transfers ownership of the implementor's
// state to the registry.

/// Installs the four-function key-provider bundle: private-key
/// import, sign-cert disposer, cert-chain reader, and public-key
/// derivation.  C: `picoquic_register_tls_key_provider_fn`.
pub fn register_tls_key_provider(
    _set_private_key_from_file: Option<Box<dyn SetPrivateKeyFromFile>>,
    _dispose_sign_certificate: Option<Box<dyn DisposeSignCertificate>>,
    _get_certs_from_file: Option<Box<dyn GetCertsFromFile>>,
    _get_public_key_from_private: Option<Box<dyn GetPublicKeyFromPrivate>>,
) {
    todo!()
}

/// Installs the three-function certificate-verifier bundle.  C:
/// `picoquic_register_verify_certificate_fn`.
pub fn register_verify_certificate(
    _certificate_verifier: Option<Box<dyn GetCertificateVerifier>>,
    _dispose_certificate_verifier: Option<Box<dyn DisposeCertificateVerifier>>,
    _set_tls_root_certificates: Option<Box<dyn SetTlsRootCertificates>>,
) {
    todo!()
}

/// Installs the error-explanation hook pair.  C:
/// `picoquic_register_explain_crypto_error_fn`.
pub fn register_explain_crypto_error(
    _explain_crypto_error: Option<Box<dyn ExplainCryptoError>>,
    _clear_crypto_errors: Option<Box<dyn ClearCryptoErrors>>,
) {
    todo!()
}

/// Installs the global RNG provider.  C:
/// `picoquic_register_crypto_random_provider_fn`.
pub fn register_crypto_random_provider(_random_provider: Option<Box<dyn CryptoRandomProvider>>) {
    todo!()
}

/// Installs the key-exchange constructor / disposer pair.  C:
/// `picoquic_register_keyex_from_key_file_fn`.
pub fn register_keyex_from_key_file(
    _keyex_from_key_file: Option<Box<dyn KeyexFromKeyFile>>,
    _keyex_dispose: Option<Box<dyn KeyexDispose>>,
) {
    todo!()
}

// ---------------------------------------------------------------------------
// Registry size limits (kept as `pub const` per the comment block at
// the top of this module — slices carry their length, but other
// modules index into the registry by name).

/// C: `CIPHER_SUITES_NB_MAX`.
pub const CIPHER_SUITES_NB_MAX: usize = 8;

/// C: `KEY_EXCHANGES_NB_MAX`.
pub const KEY_EXCHANGES_NB_MAX: usize = 4;

/// C: `HPKE_CIPHER_SUITE_NB_MAX`.
pub const HPKE_CIPHER_SUITE_NB_MAX: usize = 4;

/// C: `HPKE_KEM_NB_MAX`.
pub const HPKE_KEM_NB_MAX: usize = 3;

/// One entry in the cipher-suite registry: a high-memory and an
/// optional low-memory implementation of the same suite, paired so
/// the stack can downshift on memory-constrained connections.  Both
/// slots are nullable in C — promoted to `Option<&'static …>`.  C:
/// `struct st_picoquic_cipher_suites_t`.
#[derive(Debug)]
pub struct CipherSuiteEntry {
    pub high_memory_suite: Option<&'static PtlsCipherSuite>,
    pub low_memory_suite: Option<&'static PtlsCipherSuite>,
}

// ---------------------------------------------------------------------------
// Registry accessors (replace the `extern` globals from the header).
//
// Phase 3 will pick the storage; Phase 1 just exposes the read-side
// API surface.  The `_NB_MAX + 1` C arrays kept a NULL sentinel at
// the end; the Rust slices return populated entries only — the
// `Option` inside `CipherSuiteEntry` already encodes "missing".

/// Borrowed view of the registered cipher-suite table.  C:
/// `picoquic_cipher_suites`.
pub fn cipher_suites() -> &'static [CipherSuiteEntry] {
    todo!()
}

/// Borrowed view of the registered key-exchange algorithms.  C:
/// `picoquic_key_exchanges`.
pub fn key_exchanges() -> &'static [&'static PtlsKeyExchangeAlgorithm] {
    todo!()
}

/// The registered secp256r1 key-exchange algorithm, if any.  C
/// declared a fixed two-element array (slot + `NULL` terminator);
/// the Rust accessor exposes only the populated entry.  C:
/// `picoquic_key_exchange_secp256r1`.
pub fn key_exchange_secp256r1() -> Option<&'static PtlsKeyExchangeAlgorithm> {
    todo!()
}

/// Borrowed view of the registered HPKE cipher suites.  C:
/// `picoquic_hpke_cipher_suites`.
pub fn hpke_cipher_suites() -> &'static [&'static PtlsHpkeCipherSuite] {
    todo!()
}

/// Borrowed view of the registered HPKE KEMs.  C:
/// `picoquic_hpke_kems`.
pub fn hpke_kems() -> &'static [&'static PtlsHpkeKem] {
    todo!()
}

/// The currently registered private-key-import handler, if any.  C:
/// `picoquic_set_private_key_from_file_fn`.
pub fn set_private_key_from_file() -> Option<&'static dyn SetPrivateKeyFromFile> {
    todo!()
}

/// The currently registered sign-certificate disposer, if any.  C:
/// `picoquic_dispose_sign_certificate_fn`.
pub fn dispose_sign_certificate() -> Option<&'static dyn DisposeSignCertificate> {
    todo!()
}

/// The currently registered cert-chain reader, if any.  C:
/// `picoquic_get_certs_from_file_fn`.
pub fn get_certs_from_file() -> Option<&'static dyn GetCertsFromFile> {
    todo!()
}

/// The currently registered public-key derivation handler, if any.
/// C: `picoquic_get_public_key_from_private_fn`.
pub fn get_public_key_from_private() -> Option<&'static dyn GetPublicKeyFromPrivate> {
    todo!()
}

/// The currently registered certificate-verifier factory, if any.
/// C: `picoquic_get_certificate_verifier_fn`.
pub fn get_certificate_verifier() -> Option<&'static dyn GetCertificateVerifier> {
    todo!()
}

/// The currently registered certificate-verifier disposer, if any.
/// C: `picoquic_dispose_certificate_verifier_fn`.
pub fn dispose_certificate_verifier() -> Option<&'static dyn DisposeCertificateVerifier> {
    todo!()
}

/// The currently registered root-CA installer, if any.  C:
/// `picoquic_set_tls_root_certificates_fn`.
pub fn set_tls_root_certificates() -> Option<&'static dyn SetTlsRootCertificates> {
    todo!()
}

/// The currently registered crypto-error explainer, if any.  C:
/// `picoquic_explain_crypto_error_fn`.
pub fn explain_crypto_error() -> Option<&'static dyn ExplainCryptoError> {
    todo!()
}

/// The currently registered crypto-error queue drain, if any.  C:
/// `picoquic_clear_crypto_errors_fn`.
pub fn clear_crypto_errors() -> Option<&'static dyn ClearCryptoErrors> {
    todo!()
}

/// The currently registered RNG, if any.  C:
/// `picoquic_crypto_random_provider_fn`.
pub fn crypto_random_provider() -> Option<&'static dyn CryptoRandomProvider> {
    todo!()
}

/// The currently registered key-exchange constructor, if any.  C:
/// `picoquic_keyex_from_key_file_fn`.
pub fn keyex_from_key_file() -> Option<&'static dyn KeyexFromKeyFile> {
    todo!()
}

/// The currently registered key-exchange disposer, if any.  C:
/// `picoquic_keyex_dispose_fn`.
pub fn keyex_dispose() -> Option<&'static dyn KeyexDispose> {
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
/// tracking moves into the vector.  `tls`, `cnx`, and the tls
/// extension array stay as their forward-declared shapes; phase 3
/// will pick `Box` vs. `&mut` once the tls bindings settle.
///
/// `Debug` is intentionally not derived: `PtlsIovec`
/// (forward-declared in `quic.rs`) is opaque and does not
/// implement `Debug`, and adding it there is out of scope for this
/// header's translation.
pub struct TlsCtx {
    /// Owned tls connection state.  C allocated this with
    /// `ptls_new`; phase 3 will model the ownership transfer.
    pub tls: Option<Box<Ptls>>,
    /// Back-pointer to the connection that owns this context.  Not
    /// owned — the connection outlives the TLS context.  Borrows
    /// will be sorted out in phase 3 when the surrounding
    /// connection lifetime is mapped.
    pub cnx: Option<*mut Cnx>,
    /// `int client_mode` in C is a 0/1 flag — promoted to `bool`.
    pub client_mode: bool,
    /// QUIC-transport-parameter raw extensions buffer.  C declared
    /// a fixed two-element array; preserved verbatim.
    pub ext: [PtlsRawExtension; 2],
    /// Retry-config opaque blob handed to tls.
    pub retry_configs: PtlsIovec,
    /// tls handshake-properties slot.  Embedded by value as in C.
    pub handshake_properties: PtlsHandshakeProperties,
    /// ALPN list passed to tls during handshake.  C kept the
    /// owned buffer pointer plus a max size and a current count;
    /// the Rust `Vec` collapses both length tracks into its `len()`,
    /// while `capacity()` covers the C `alpn_vec_size` invariant.
    pub alpn_vec: Vec<PtlsIovec>,
    /// QUIC transport-parameter encode/decode scratch buffer.  C
    /// allocated `ext_data_size` bytes and stored that size
    /// alongside; the Rust shape uses the vector's capacity.
    pub ext_data: Vec<u8>,
    /// Forward-secret key for application-data encryption.
    pub app_secret_enc: [u8; PTLS_MAX_DIGEST_SIZE],
    /// Forward-secret key for application-data decryption.
    pub app_secret_dec: [u8; PTLS_MAX_DIGEST_SIZE],
}

// `mbedtls_get_certificate_verifier` is gated on the C
// `#ifdef WITH_MBEDTLS` branch.  v1 targets the canonical
// build with mbedtls disabled, so the function is dropped here per
// the Phase 0 ifdef policy ("non-target branches are not
// translated in v1; other platforms get added back later behind
// `#[cfg(…)]`").

#[cfg(test)]
mod test {}
