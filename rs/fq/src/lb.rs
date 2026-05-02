//! Translation of `quic/lb.h`.
//!
//! Server-side support for the QUIC load-balancer draft
//! (<https://datatracker.ietf.org/doc/draft-ietf-quic-load-balancers/>):
//! parse the LB-supplied configuration string into an [`LbConfig`],
//! then install a per-server CID-generation / -verification context
//! ([`LbCidContext`]) on a [`Quic`] via the connection-ID callback
//! hook.
//!
//! ## API shape
//!
//! * [`LbConfig::parse`] takes the LB string as `&str` and returns
//!   the populated config; the C `(char const* txt, size_t
//!   txt_length)` pair plus zeroed out-parameter collapse to a
//!   normal Rust constructor.
//! * [`Quic::set_lb_cid_config`] applies an `LbConfig` to the QUIC
//!   context, installing the per-server CID context.
//! * [`Quic::clear_lb_cid_config`] tears the installed context back
//!   down (no-op when none is installed).
//! * [`LbCidContext::generate`] / [`LbCidContext::verify`] are the
//!   per-CID callback bodies.  They are exposed as inherent methods
//!   so the eventual [`crate::ConnectionIdCb`] trait impl can
//!   delegate to them; the C `void* cnx_id_cb_data` parameter is
//!   recovered as `&mut self`.
//!
//! ## Field-shape decisions
//!
//! * `rotation_bits : 2` — multi-bit C bitfield, kept as `u8`
//!   (legal range `0..=3`) per the Phase 1 bitfield rule.
//! * `first_byte_encodes_length : 1` — single-bit C bitfield used
//!   as a Boolean flag throughout the C body, promoted to `bool`
//!   (matching the `config.rs` precedent).
//! * `cid_encryption_context` / `cid_decryption_context` were
//!   `void*` in C, holding AES-128-ECB contexts produced by
//!   `aes128_ecb_create` and freed by `aes128_ecb_free` (both
//!   declared in `tls_api.h`, which has not been translated yet).
//!   The fields own their contexts in C (the clear path explicitly
//!   calls the AES destructor), so Rust models them as
//!   `Option<Box<Aes128EcbContext>>`, with the AES-context type
//!   forward-declared here as an opaque struct.  `Box::drop` will
//!   subsume the explicit `aes128_ecb_free` call once Phase 3 gives
//!   the type a `Drop` impl.
//! * `server_id: [u8; 16]` and `cid_encryption_key: [u8; 16]` stay
//!   as fixed-capacity byte arrays — the C declarations are size-16
//!   arrays and the algorithms are AES-128, so the capacity *is*
//!   part of the contract.

extern crate alloc;

use alloc::boxed::Box;

use crate::Error;
use crate::{ConnectionId, Quic};

// ---------------------------------------------------------------------------
// CID-encoding methods.

/// CID-encoding method selected by the load-balancer config.
/// C: `load_balancer_cid_method_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum LbCidMethod {
    /// Server ID copied in clear after the first byte.
    /// C: `load_balancer_cid_clear`.
    #[default]
    Clear,
    /// Server ID encrypted with an AES-128-ECB stream cipher keyed
    /// off a per-CID nonce.
    /// C: `load_balancer_cid_stream_cipher`.
    StreamCipher,
    /// Server ID encrypted with a single AES-128-ECB block.
    /// C: `load_balancer_cid_block_cipher`.
    BlockCipher,
}

// ---------------------------------------------------------------------------
// Forward declarations.

/// Forward declaration of the AES-128-ECB context produced by the
/// (not-yet-translated) `tls_api` module — `void*` in C, a
/// crypto-provider-specific opaque type at runtime.  Phase 3
/// replaces this opaque struct with whatever shape `tls_api.rs`
/// settles on (typically a trait object) and gives it a `Drop` impl
/// so the explicit `aes128_ecb_free` in
/// [`Quic::clear_lb_cid_config`] collapses into RAII.
#[derive(Debug)]
pub struct Aes128EcbContext {
    _opaque: [u8; 0],
}

// ---------------------------------------------------------------------------
// Configuration record.

/// Configuration parsed from an LB-supplied string and applied to
/// a [`Quic`] via [`Quic::set_lb_cid_config`].
/// C: `load_balancer_config_t`.
///
/// `repr(C)` is dropped — the struct is purely an internal shape,
/// never inspected by an LB or another process.
#[derive(Debug, Default)]
pub struct LbConfig {
    pub method: LbCidMethod,
    /// 2-bit field in C; legal values `0..=3`.
    pub rotation_bits: u8,
    /// 1-bit field in C; promoted to `bool` to match call-site usage.
    pub first_byte_encodes_length: bool,
    pub server_id_length: u8,
    /// Used in stream-cipher mode.
    pub nonce_length: u8,
    pub connection_id_length: u8,
    pub server_id64: u64,
    pub cid_encryption_key: [u8; 16],
}

impl LbConfig {
    /// Parse an LB-format configuration string into a fresh
    /// [`LbConfig`].  Returns `Err` when the string is malformed,
    /// the embedded server ID overflows 8 bytes, or the encoded CID
    /// length is incompatible with the chosen method.
    /// C: `lb_compat_cid_config_parse`.
    ///
    /// The C signature took an out-parameter and a `(char const*
    /// txt, size_t txt_length)` pair returning an `int`; the Rust
    /// signature returns the populated struct in the success arm
    /// and folds the explicit length into the `&str` slice.
    pub fn parse(_txt: &str) -> Result<Self, Error> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Per-server CID context.

/// Per-server context attached to a [`Quic`] once a load-balancer
/// config has been applied.  Built from an [`LbConfig`] by
/// [`Quic::set_lb_cid_config`] and consumed by the
/// [`crate::ConnectionIdCb`] hook.
/// C: `load_balancer_cid_context_t`.
#[derive(Debug, Default)]
pub struct LbCidContext {
    pub method: LbCidMethod,
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
    /// struct: the C clear path calls `aes128_ecb_free` on it.
    pub cid_encryption_context: Option<Box<Aes128EcbContext>>,
    /// Used in block-cipher mode for the verify path.  Owned the
    /// same way.
    pub cid_decryption_context: Option<Box<Aes128EcbContext>>,
}

impl LbCidContext {
    /// Fill `cnx_id_returned` with a CID encoded per `self.method`.
    /// C: `lb_compat_cid_generate`.
    ///
    /// The body assumes the caller has pre-filled `cnx_id_returned`
    /// with the expected nonce / "for-server use" bytes — the
    /// parameter is read AND written, so it stays `&mut` rather
    /// than collapsing to a return value.  `quic` is read for
    /// `local_cnxid_length`; the unused `cnx_id_local` /
    /// `cnx_id_remote` parameters of the C signature are dropped
    /// here and reintroduced (if needed) by the
    /// [`crate::ConnectionIdCb`] adapter.  `&mut self` because the
    /// underlying AES contexts mutate cipher state in place.
    pub fn generate(&mut self, _quic: &Quic, _cnx_id_returned: &mut ConnectionId) {
        todo!()
    }

    /// Decode the server ID embedded in `cnx_id`.  Returns `None`
    /// when the CID length doesn't match `self.connection_id_length`
    /// or when `self.method` is unrecognised; the C sentinel
    /// `UINT64_MAX` is folded into `Option`.
    /// C: `lb_compat_cid_verify`.
    ///
    /// `&mut self` because the underlying AES contexts mutate
    /// cipher state in place.
    pub fn verify(&mut self, _cnx_id: &ConnectionId) -> Option<u64> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Quic-context glue.

impl Quic {
    /// Apply `lb_config` to `self`, installing an [`LbCidContext`]
    /// as the connection-ID callback context.
    /// C: `lb_compat_cid_config`.
    ///
    /// Returns `Err` when `self` already has a different CID
    /// callback configured, when the requested CID length doesn't
    /// fit the chosen method, or when the AES context allocation
    /// fails.
    pub fn set_lb_cid_config(&mut self, _lb_config: &LbConfig) -> Result<(), Error> {
        todo!()
    }

    /// Tear down the [`LbCidContext`] previously installed by
    /// [`Quic::set_lb_cid_config`], releasing the AES-ECB
    /// encryption contexts and clearing the callback slot on
    /// `self`.  No-op when no LB CID context is installed.
    /// C: `lb_compat_cid_config_free`.
    pub fn clear_lb_cid_config(&mut self) {
        todo!()
    }
}

#[cfg(test)]
mod test {}
