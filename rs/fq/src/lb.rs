//! Translation of `quic/lb.h`.
//!
//! Server-side support for the QUIC load-balancer draft
//! (<https://datatracker.ietf.org/doc/draft-ietf-quic-load-balancers/>):
//! parse the LB-supplied configuration string into a [`Config`],
//! then install a per-server CID-generation / -verification context
//! ([`ConnectionIdContext`]) on a [`Quic`] via the connection-ID
//! callback hook.
//!
//! ## API shape
//!
//! * [`Config::parse`] takes the LB string as `&str` and returns
//!   the populated config; the C `(char const* txt, size_t
//!   txt_length)` pair plus zeroed out-parameter collapse to a
//!   normal Rust constructor.
//! * [`Quic::set_lb_cid_config`] applies a `Config` to the QUIC
//!   context, installing the per-server CID context.
//! * [`Quic::clear_lb_cid_config`] tears the installed context back
//!   down (no-op when none is installed).
//! * [`ConnectionIdContext::generate`] / [`ConnectionIdContext::verify`]
//!   are the per-CID callback bodies.  They are exposed as inherent methods
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

use crate::Error;
use crate::tls_api::Aes128EcbContext;
use crate::{ConnectionId, Quic};

/// CID-encoding method selected by the load-balancer config.
/// C: `load_balancer_cid_method_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ConnectionIdMethod {
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

/// Number of low bits of the first CID byte that select between
/// short-lived configurations.  C: `unsigned int rotation_bits : 2`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum RotationBits {
    #[default]
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
}

/// Configuration parsed from an LB-supplied string and applied to
/// a [`Quic`] via [`Quic::set_lb_cid_config`].
/// C: `load_balancer_config_t`.
///
/// `repr(C)` is dropped — the struct is purely an internal shape,
/// never inspected by an LB or another process.
///
/// Lives in `crate::lb`; consumers that also import the demo-app
/// [`crate::config::Config`] should rename one at the import
/// site (`use crate::lb::Config as LbConfig;`).
#[derive(Debug, Default)]
pub struct Config {
    pub method: ConnectionIdMethod,
    pub rotation_bits: RotationBits,
    /// 1-bit field in C; promoted to `bool` to match call-site usage.
    pub first_byte_encodes_length: bool,
    pub server_id_length: usize,
    /// Used in stream-cipher mode.
    pub nonce_length: usize,
    pub connection_id_length: usize,
    pub server_id: u64,
    // REVIEW(open): the encryption key naturally belongs inside the AES context (newtype around
    // `[u8; 16]` exposed as an associated type of `Aes128EcbContext`).  Land that when the
    // crypto provider abstraction (Phase 2) defines the AES wrapper for real.
    pub cid_encryption_key: [u8; 16],
}

impl Config {
    /// Parse an LB-format configuration string into a fresh
    /// [`Config`].  Returns `Err` when the string is malformed,
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
/// config has been applied.  Built from a [`Config`] by
/// [`Quic::set_lb_cid_config`] and consumed by the
/// [`crate::ConnectionIdCb`] hook.
/// C: `load_balancer_cid_context_t`.
///
/// REVIEW(open): the AES contexts below are `Option` to match the C
/// "uninitialised" intermediate state during parsing.  Once the
/// crypto provider abstraction (Phase 2) lands, switch construction
/// to a builder so the contexts are always set on a finished
/// `ConnectionIdContext` and drop the `Option` wrapper.
#[derive(Debug, Default)]
pub struct ConnectionIdContext {
    pub method: ConnectionIdMethod,
    pub rotation_bits: RotationBits,
    /// 1-bit field in C; promoted to `bool`.
    pub first_byte_encodes_length: bool,
    pub server_id_length: usize,
    /// Used in stream-cipher mode.
    pub nonce_length: usize,
    pub connection_id_length: usize,
    pub server_id: u64,
    /// Big-endian encoding of `server_id`, padded to
    /// `server_id_length` bytes.
    pub server_id_encoded: [u8; 16],
    /// Used in stream- and block-cipher modes.  Owned by this
    /// struct: the C clear path calls `aes128_ecb_free` on it.
    pub cid_encryption_context: Option<Box<Aes128EcbContext>>,
    /// Used in block-cipher mode for the verify path.  Owned the
    /// same way.
    pub cid_decryption_context: Option<Box<Aes128EcbContext>>,
}

impl ConnectionIdContext {
    /// Encode a CID from `nonce` per `self.method` and return it.
    /// C: `lb_compat_cid_generate`.
    ///
    /// The C signature took a `cnx_id_returned` out parameter that
    /// the body both read (for the nonce / "for-server use" bytes)
    /// and wrote (for the encoded CID); the Rust signature splits
    /// those two roles — the input nonce comes in by reference and
    /// the encoded CID is the return value.  `&mut self` because the
    /// underlying AES contexts mutate cipher state in place.  `quic`
    /// is read for `local_cnxid_length`; the unused `cnx_id_local` /
    /// `cnx_id_remote` parameters of the C signature are dropped
    /// here and reintroduced (if needed) by the
    /// [`crate::ConnectionIdCb`] adapter.
    pub fn generate(&mut self, _quic: &Quic, _nonce: &ConnectionId) -> ConnectionId {
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

impl Quic {
    /// Apply `lb_config` to `self`, installing a [`ConnectionIdContext`]
    /// as the connection-ID callback context.
    /// C: `lb_compat_cid_config`.
    ///
    /// Returns `Err` when `self` already has a different CID
    /// callback configured, when the requested CID length doesn't
    /// fit the chosen method, or when the AES context allocation
    /// fails.
    pub fn set_lb_cid_config(&mut self, _lb_config: &Config) -> Result<(), Error> {
        todo!()
    }

    /// Tear down the [`ConnectionIdContext`] previously installed by
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
