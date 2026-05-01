//! Translation of `quic/lb.h`.
//!
//! Server-side support for the QUIC load-balancer draft
//! (<https://datatracker.ietf.org/doc/draft-ietf-quic-load-balancers/>):
//! the configuration record carved out of an LB-supplied string,
//! and the per-server CID-generation / -verification context that
//! gets wired into a `quic_t` via the connection-ID
//! callback hook.
//!
//! Phase 1: signatures only — every body is `todo!()`.  Pointer-
//! shape decisions for the public surface come from reading the
//! C side under `quic/lb.c`:
//!
//! * `lb_compat_cid_config_parse` zeroes its `lb_config`
//!   out-parameter and fills it from a hex/ASCII config string.
//!   In Rust the out-parameter becomes the `T` of `Result<T, _>`;
//!   the `(char const* txt, size_t txt_length)` pair collapses to
//!   `&str`.  The function returns `Result<…, ()>` until the
//!   crate-level `Error` enum lands (see TODO below).
//! * `lb_compat_cid_config(quic, lb_config)` reads
//!   `lb_config` and writes back into `quic` (sets
//!   `local_cnxid_length`, installs the callback function and its
//!   context).  Borrows are `&mut quic` and `&lb_config`.
//! * `lb_compat_cid_config_free(quic)` looks up the
//!   previously installed context inside `quic`, frees its
//!   AES-ECB encryption contexts, and clears the callback slot —
//!   `&mut quic`.
//! * `lb_compat_cid_generate` and
//!   `lb_compat_cid_verify` are invoked through the
//!   `cnx_id_callback_fn` / `cnx_id_cb_data` pair on the QUIC
//!   context.  The `void* cnx_id_cb_data` argument *is* the
//!   `load_balancer_cid_context_t`, so they become
//!   inherent methods on that struct (`generate` / `verify`)
//!   with the `void*` recovered as `&mut self` / `&self`.  The
//!   `cnx_id_returned` out-parameter is documented in C as
//!   read-then-write (the caller pre-fills nonce / for-server-use
//!   bits) so it stays `&mut`, not a return value.  The C
//!   `cnx_id_local` / `cnx_id_remote` parameters are unused by
//!   the body but kept on the signature to match the
//!   [`ConnectionIdCb`](crate::ConnectionIdCb)
//!   shape; a future Phase 3 refactor can hook the trait up to
//!   the context.
//!
//! Field-shape decisions inside the structs:
//!
//! * `rotation_bits : 2` — multi-bit C bitfield, kept as `u8`
//!   (legal range `0..=3`) per the Phase 1 bitfield rule.
//! * `first_byte_encodes_length : 1` — single-bit C bitfield
//!   used as a Boolean flag throughout the C body, promoted to
//!   `bool` (matching the `config.rs` precedent).
//! * `cid_encryption_context` / `cid_decryption_context` were
//!   `void*` in C, holding AES-128-ECB contexts produced by
//!   `aes128_ecb_create` and freed by
//!   `aes128_ecb_free` (both declared in `tls_api.h`,
//!   which has not been translated yet).  The fields own their
//!   contexts in C (the `_free` path explicitly calls the AES
//!   destructor), so Rust models them as
//!   `Option<Box<aes128_ecb_context_t>>`, with the
//!   AES-context type forward-declared here as an opaque struct.
//!   `Box::drop` will subsume the explicit `_free` once Phase 3
//!   gives the type a `Drop` impl.
//! * `server_id: [u8; 16]` and `cid_encryption_key: [u8; 16]`
//!   stay as fixed-capacity byte arrays — the C declarations
//!   are size-16 arrays and the algorithms are AES-128, so the
//!   capacity *is* part of the contract.

#![allow(non_camel_case_types)]
// Enum variants share the `load_balancer_cid_` prefix to mirror
// the C enum tag names verbatim — renaming would break source-level parity.
#![allow(clippy::enum_variant_names)]
#![allow(clippy::result_unit_err)]

extern crate alloc;

use alloc::boxed::Box;

use crate::{connection_id_t, quic_t};

// ---------------------------------------------------------------------------
// CID-encoding methods.

/// CID-encoding method selected by the load-balancer config.
/// C: `load_balancer_cid_method_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum load_balancer_cid_method_enum {
    /// Server ID copied in clear after the first byte.
    #[default]
    load_balancer_cid_clear,
    /// Server ID encrypted with an AES-128-ECB stream cipher
    /// keyed off a per-CID nonce.
    load_balancer_cid_stream_cipher,
    /// Server ID encrypted with a single AES-128-ECB block.
    load_balancer_cid_block_cipher,
}

// ---------------------------------------------------------------------------
// Forward declarations.

/// Forward declaration of the AES-128-ECB context produced by
/// the (not-yet-translated) `tls_api` module — `void*` in C, a
/// crypto-provider-specific opaque type at runtime.  Phase 3
/// replaces this opaque struct with whatever shape `tls_api.rs`
/// settles on (typically a trait object) and gives it a `Drop`
/// impl so the explicit `aes128_ecb_free` in
/// [`lb_compat_cid_config_free`] can collapse into RAII.
#[derive(Debug)]
pub struct aes128_ecb_context_t {
    _opaque: [u8; 0],
}

// ---------------------------------------------------------------------------
// Configuration record.

/// Configuration parsed from an LB-supplied string and applied
/// to a `quic_t` via [`lb_compat_cid_config`].
/// C: `load_balancer_config_t`.
///
/// `repr(C)` is dropped — the struct is purely an internal
/// shape, never inspected by an LB or another process.
#[derive(Debug, Default)]
pub struct load_balancer_config_t {
    pub method: load_balancer_cid_method_enum,
    /// 2-bit field in C; legal values `0..=3`.
    pub rotation_bits: u8,
    /// 1-bit field in C; promoted to `bool` to match call-site
    /// usage.
    pub first_byte_encodes_length: bool,
    pub server_id_length: u8,
    /// Used in stream-cipher mode.
    pub nonce_length: u8,
    pub connection_id_length: u8,
    pub server_id64: u64,
    pub cid_encryption_key: [u8; 16],
}

impl load_balancer_config_t {
    /// C: `lb_compat_cid_config_parse`.  Parse an
    /// LB-format configuration string into a fresh
    /// `load_balancer_config_t`.
    ///
    /// The C signature took an out-parameter and a
    /// `(char const* txt, size_t txt_length)` pair returning an
    /// `int`; the Rust signature returns the populated struct in
    /// the success arm and folds the explicit length into the
    /// `&str` slice.
    // TODO(error-enum): swap `()` for the crate's `Error` once it lands.
    pub fn parse(_txt: &str) -> Result<Self, ()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Per-server CID context.

/// Per-server context attached to a `quic_t` once a
/// load-balancer config has been applied.  Built from a
/// [`load_balancer_config_t`] by
/// [`lb_compat_cid_config`] and consumed by the
/// `cnx_id_callback_fn` hook.  C:
/// `load_balancer_cid_context_t`.
#[derive(Debug, Default)]
pub struct load_balancer_cid_context_t {
    pub method: load_balancer_cid_method_enum,
    /// 2-bit field in C; legal values `0..=3`.
    pub rotation_bits: u8,
    /// 1-bit field in C; promoted to `bool`.
    pub first_byte_encodes_length: bool,
    pub server_id_length: u8,
    /// Used in stream-cipher mode.
    pub nonce_length: u8,
    pub connection_id_length: u8,
    pub server_id64: u64,
    /// Big-endian encoding of `server_id64`, padded to
    /// `server_id_length` bytes.
    pub server_id: [u8; 16],
    /// Used in stream- and block-cipher modes.  Owned by this
    /// struct: the C `_config_free` path calls
    /// `aes128_ecb_free` on it.
    pub cid_encryption_context: Option<Box<aes128_ecb_context_t>>,
    /// Used in block-cipher mode for the verify path.  Owned
    /// the same way.
    pub cid_decryption_context: Option<Box<aes128_ecb_context_t>>,
}

impl load_balancer_cid_context_t {
    /// C: `lb_compat_cid_generate`.  Fill
    /// `cnx_id_returned` with a CID encoded per `self.method`.
    ///
    /// The C body assumes the caller has pre-filled
    /// `cnx_id_returned` with the expected nonce / "for-server
    /// use" bytes — the parameter is read AND written, so it
    /// stays `&mut` rather than collapsing to a return value.
    /// `cnx_id_local` and `cnx_id_remote` are accepted to match
    /// the [`ConnectionIdCb`](crate::ConnectionIdCb)
    /// shape even though the body discards them.
    pub fn generate(
        &mut self,
        _quic: &mut quic_t,
        _cnx_id_local: connection_id_t,
        _cnx_id_remote: connection_id_t,
        _cnx_id_returned: &mut connection_id_t,
    ) {
        todo!()
    }

    /// C: `lb_compat_cid_verify`.  Decode the server-ID
    /// embedded in `cnx_id`, returning `u64::MAX` when the CID
    /// length doesn't match `self.connection_id_length` or when
    /// `self.method` is unrecognised.
    pub fn verify(&self, _cnx_id: &connection_id_t) -> u64 {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// QUIC-context glue.

/// C: `lb_compat_cid_config`.  Apply `lb_config` to
/// `quic`, installing the LB CID context as the connection-ID
/// callback context.
///
/// Returns `Err(())` when `quic` already has a different CID
/// callback configured, when the requested CID length doesn't
/// fit the chosen method, or when the AES context allocation
/// fails.
// TODO(error-enum): swap `()` for the crate's `Error` once it lands.
pub fn lb_compat_cid_config(
    _quic: &mut quic_t,
    _lb_config: &load_balancer_config_t,
) -> Result<(), ()> {
    todo!()
}

/// C: `lb_compat_cid_config_free`.  Tear down the LB
/// CID context previously installed by
/// [`lb_compat_cid_config`], releasing the AES-ECB
/// encryption contexts and clearing the callback slot on
/// `quic`.  No-op when no LB CID context is installed.
pub fn lb_compat_cid_config_free(_quic: &mut quic_t) {
    todo!()
}

#[cfg(test)]
mod test {}
