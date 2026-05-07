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
//!   so the eventual [`crate::ConnectionIdCallback`] trait impl can
//!   delegate to them; the C `void* connection_id_cb_data` parameter is
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
use crate::{CONNECTION_ID_MAX_SIZE, ConnectionId, Quic};

// ---------------------------------------------------------------------------
// Private hex-parsing helpers.

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Decode hex bytes from `hex` into `out`, returning bytes written.
/// C: `picoquic_parse_hexa`.
fn parse_hex_bytes(hex: &[u8], out: &mut [u8]) -> usize {
    let mut i = 0;
    let mut written = 0;
    while i + 1 < hex.len() && written < out.len() {
        match (hex_nibble(hex[i]), hex_nibble(hex[i + 1])) {
            (Some(hi), Some(lo)) => {
                out[written] = (hi << 4) | lo;
                written += 1;
                i += 2;
            }
            _ => break,
        }
    }
    written
}

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
    pub fn parse(txt: &str) -> Result<Self, Error> {
        let bytes = txt.as_bytes();
        if bytes.len() < 4 {
            return Err(Error::InvalidArgument);
        }

        let rotation_bits = match bytes[0] {
            b'0' => RotationBits::Zero,
            b'1' => RotationBits::One,
            b'2' => RotationBits::Two,
            b'3' => RotationBits::Three,
            _ => return Err(Error::InvalidArgument),
        };

        let first_byte_encodes_length = match bytes[1] {
            b'Y' | b'y' => true,
            b'N' | b'n' => false,
            _ => return Err(Error::InvalidArgument),
        };

        let mut config = Config {
            rotation_bits,
            first_byte_encodes_length,
            ..Config::default()
        };

        let mut parsed = 2usize;

        // Optional decimal connection_id_length.
        let mut cid_len: usize = 0;
        while parsed < bytes.len() && bytes[parsed].is_ascii_digit() {
            cid_len = cid_len * 10 + (bytes[parsed] - b'0') as usize;
            parsed += 1;
            if cid_len >= 256 {
                return Err(Error::InvalidArgument);
            }
            config.connection_id_length = cid_len;
        }

        if parsed >= bytes.len() {
            return Err(Error::InvalidArgument);
        }
        let method_char = bytes[parsed];
        parsed += 1;
        match method_char {
            b'c' | b'C' => {
                config.method = ConnectionIdMethod::Clear;
            }
            b's' | b'S' => {
                config.method = ConnectionIdMethod::StreamCipher;
                let mut nonce_len: usize = 0;
                while parsed < bytes.len() && bytes[parsed].is_ascii_digit() {
                    nonce_len = nonce_len * 10 + (bytes[parsed] - b'0') as usize;
                    parsed += 1;
                    if nonce_len >= 256 {
                        return Err(Error::InvalidArgument);
                    }
                    config.nonce_length = nonce_len;
                }
            }
            b'b' | b'B' => {
                config.method = ConnectionIdMethod::BlockCipher;
            }
            _ => return Err(Error::InvalidArgument),
        }

        // Hyphen before server ID.
        if parsed >= bytes.len() || bytes[parsed] != b'-' {
            return Err(Error::InvalidArgument);
        }
        parsed += 1;

        if parsed >= bytes.len() {
            return Err(Error::InvalidArgument);
        }

        // Server ID hex (scan to next '-' or end).
        let hex_len = bytes[parsed..]
            .iter()
            .position(|&b| b == b'-')
            .unwrap_or(bytes.len() - parsed);
        let hex_end = parsed + hex_len;
        let mut s_id_bin = [0u8; 8];
        let s_id_len = parse_hex_bytes(&bytes[parsed..hex_end], &mut s_id_bin);
        if s_id_len == 0 {
            return Err(Error::InvalidArgument);
        }
        config.server_id_length = s_id_len;
        for b in s_id_bin.iter().take(s_id_len) {
            config.server_id <<= 8;
            config.server_id |= *b as u64;
        }
        parsed += 2 * s_id_len;

        // Encryption key for stream/block cipher.
        if matches!(
            config.method,
            ConnectionIdMethod::StreamCipher | ConnectionIdMethod::BlockCipher
        ) {
            if parsed >= bytes.len() || bytes[parsed] != b'-' {
                return Err(Error::InvalidArgument);
            }
            parsed += 1;
            if bytes.len() < parsed + 32 {
                return Err(Error::InvalidArgument);
            }
            let key_len = parse_hex_bytes(&bytes[parsed..], &mut config.cid_encryption_key);
            if key_len != 16 {
                return Err(Error::InvalidArgument);
            }
            parsed += 32;
        }

        if parsed != bytes.len() {
            return Err(Error::InvalidArgument);
        }

        // Validate connection_id_length if explicitly set.
        if config.connection_id_length != 0 {
            let min_length = 1 + config.server_id_length + config.nonce_length;
            if config.connection_id_length < min_length {
                return Err(Error::InvalidArgument);
            }
            if matches!(config.method, ConnectionIdMethod::BlockCipher)
                && config.connection_id_length < 17
            {
                return Err(Error::InvalidArgument);
            }
        }

        Ok(config)
    }
}

// ---------------------------------------------------------------------------
// Per-server CID context.

/// Per-server context attached to a [`Quic`] once a load-balancer
/// config has been applied.  Built from a [`Config`] by
/// [`Quic::set_lb_cid_config`] and consumed by the
/// [`crate::ConnectionIdCallback`] hook.
/// C: `load_balancer_cid_context_t`.
///
/// The two AES contexts are method-dependent: [`ConnectionIdMethod::Clear`]
/// uses neither, [`ConnectionIdMethod::StreamCipher`] uses only the
/// encryption context (the verify path re-encrypts the nonce and XORs),
/// and [`ConnectionIdMethod::BlockCipher`] uses both.  Hence the
/// `Option<Box<…>>` wrappers — they encode method-dependence rather
/// than an init-time gap.
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
    /// Encode the first byte of a generated CID from the LB config.
    /// C: `picoquic_lb_compat_cid_generate_first_byte`.
    fn set_first_byte(&self, quic: &Quic, bytes: &mut [u8]) {
        if self.first_byte_encodes_length {
            bytes[0] = (self.rotation_bits as u8) << 6 | (quic.local_connection_id_length - 1);
        } else {
            bytes[0] &= 0x3F;
            bytes[0] |= (self.rotation_bits as u8) << 6;
        }
    }

    /// Generate a clear-text LB-compatible CID by writing the first byte
    /// and copying the configured server ID after it.
    /// C: `picoquic_lb_compat_cid_generate_clear`.
    fn generate_clear(&self, quic: &Quic, bytes: &mut [u8]) {
        self.set_first_byte(quic, bytes);
        bytes[1..1 + self.server_id_length]
            .copy_from_slice(&self.server_id_encoded[..self.server_id_length]);
    }

    /// Generate a stream-cipher LB-compatible CID.  The input CID bytes are
    /// assumed to already contain the nonce and server-use bytes, matching
    /// the C out-parameter prefill contract.
    /// C: `picoquic_lb_compat_cid_generate_stream_cipher`.
    fn generate_stream_cipher(&self, quic: &Quic, bytes: &mut [u8]) {
        let id_offset = 1 + self.nonce_length;
        self.set_first_byte(quic, bytes);
        bytes[id_offset..id_offset + self.server_id_length]
            .copy_from_slice(&self.server_id_encoded[..self.server_id_length]);

        let enc = self.cid_encryption_context.as_ref().unwrap();
        Self::one_pass_stream(
            enc,
            bytes,
            1,
            self.nonce_length,
            id_offset,
            self.server_id_length,
        );
        Self::one_pass_stream(
            enc,
            bytes,
            id_offset,
            self.server_id_length,
            1,
            self.nonce_length,
        );
        Self::one_pass_stream(
            enc,
            bytes,
            1,
            self.nonce_length,
            id_offset,
            self.server_id_length,
        );
    }

    /// Generate a block-cipher LB-compatible CID.  The first 16 bytes after
    /// the first octet are encrypted in place, preserving prefilled
    /// server-use bytes in that block.
    /// C: `picoquic_lb_compat_cid_generate_block_cipher`.
    fn generate_block_cipher(&self, quic: &Quic, bytes: &mut [u8]) {
        self.set_first_byte(quic, bytes);
        bytes[1..1 + self.server_id_length]
            .copy_from_slice(&self.server_id_encoded[..self.server_id_length]);
        let enc = self.cid_encryption_context.as_ref().unwrap();
        let block = <&mut [u8; 16]>::try_from(&mut bytes[1..17]).unwrap();
        enc.process(block);
    }

    /// Decode the server ID from a clear-text CID.
    /// C: `picoquic_lb_compat_cid_verify_clear`.
    fn verify_clear(&self, cnx_id: &ConnectionId) -> u64 {
        let bytes = cnx_id.as_bytes();
        let mut s_id64: u64 = 0;
        for i in 0..self.server_id_length {
            s_id64 <<= 8;
            s_id64 += bytes[i + 1] as u64;
        }
        s_id64
    }

    /// Decode the server ID from a stream-cipher LB-compatible CID.
    /// C: `picoquic_lb_compat_cid_verify_stream_cipher` (picoquic/picoquic_lb.c:163)
    fn verify_stream_cipher(&self, cnx_id: &ConnectionId) -> u64 {
        let id_offset = 1 + self.nonce_length;
        let mut target = [0u8; CONNECTION_ID_MAX_SIZE];
        let len = cnx_id.len();
        target[..len].copy_from_slice(cnx_id.as_bytes());
        let enc = self.cid_encryption_context.as_ref().unwrap();

        Self::one_pass_stream(
            enc,
            &mut target[..len],
            1,
            self.nonce_length,
            id_offset,
            self.server_id_length,
        );
        Self::one_pass_stream(
            enc,
            &mut target[..len],
            id_offset,
            self.server_id_length,
            1,
            self.nonce_length,
        );
        Self::one_pass_stream(
            enc,
            &mut target[..len],
            1,
            self.nonce_length,
            id_offset,
            self.server_id_length,
        );

        let mut s_id64: u64 = 0;
        for i in 0..self.server_id_length {
            s_id64 <<= 8;
            s_id64 += target[id_offset + i] as u64;
        }
        s_id64
    }

    /// Decode the server ID from a block-cipher LB-compatible CID.
    /// C: `picoquic_lb_compat_cid_verify_block_cipher`.
    fn verify_block_cipher(&self, cnx_id: &ConnectionId) -> u64 {
        let bytes = cnx_id.as_bytes();
        let mut decoded = [0u8; 16];
        decoded.copy_from_slice(&bytes[1..17]);
        let dec = self.cid_decryption_context.as_ref().unwrap();
        dec.process(&mut decoded);

        let mut s_id64: u64 = 0;
        for b in decoded.iter().take(self.server_id_length) {
            s_id64 <<= 8;
            s_id64 += *b as u64;
        }
        s_id64
    }

    /// One pass of the stream-cipher: fill `mask` from `bytes[nonce_start..]`,
    /// AES-encrypt it, then XOR `mask` into `bytes[target_start..]`.
    /// C: `picoquic_lb_compat_cid_one_pass_stream`.
    fn one_pass_stream(
        enc: &Aes128EcbContext,
        bytes: &mut [u8],
        nonce_start: usize,
        nonce_len: usize,
        target_start: usize,
        target_len: usize,
    ) {
        let mut mask = [0u8; 16];
        let copy_len = nonce_len.min(16);
        mask[..copy_len].copy_from_slice(&bytes[nonce_start..nonce_start + copy_len]);
        enc.process(&mut mask);
        for i in 0..target_len {
            bytes[target_start + i] ^= mask[i];
        }
    }

    /// Encode a CID from `nonce` per `self.method` and return it.
    /// C: `lb_compat_cid_generate`.
    ///
    /// The C signature took a `connection_id_returned` out parameter that
    /// the body both read (for the nonce / "for-server use" bytes)
    /// and wrote (for the encoded CID); the Rust signature splits
    /// those two roles — the input nonce comes in by reference and
    /// the encoded CID is the return value.  `&mut self` because the
    /// underlying AES contexts mutate cipher state in place.  `quic`
    /// is read for `local_connection_id_length`; the unused `connection_id_local` /
    /// `connection_id_remote` parameters of the C signature are dropped
    /// here and reintroduced (if needed) by the
    /// [`crate::ConnectionIdCallback`] adapter.
    pub fn generate(&mut self, quic: &Quic, nonce: &ConnectionId) -> ConnectionId {
        let mut cid = *nonce;
        {
            let bytes = cid.as_bytes_mut();
            match self.method {
                ConnectionIdMethod::Clear => self.generate_clear(quic, bytes),
                ConnectionIdMethod::StreamCipher => self.generate_stream_cipher(quic, bytes),
                ConnectionIdMethod::BlockCipher => self.generate_block_cipher(quic, bytes),
            }
        }
        cid
    }

    /// Decode the server ID embedded in `connection_id`.  Returns `None`
    /// when the CID length doesn't match `self.connection_id_length`
    /// or when `self.method` is unrecognised; the C sentinel
    /// `UINT64_MAX` is folded into `Option`.
    /// C: `lb_compat_cid_verify`.
    ///
    /// `&mut self` because the underlying AES contexts mutate
    /// cipher state in place.
    pub fn verify(&mut self, cnx_id: &ConnectionId) -> Option<u64> {
        if cnx_id.len() != self.connection_id_length {
            return None;
        }
        let s_id64 = match self.method {
            ConnectionIdMethod::Clear => self.verify_clear(cnx_id),
            ConnectionIdMethod::StreamCipher => self.verify_stream_cipher(cnx_id),
            ConnectionIdMethod::BlockCipher => self.verify_block_cipher(cnx_id),
        };
        Some(s_id64)
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
    pub fn set_lb_cid_config(&mut self, lb_config: &Config) -> Result<(), Error> {
        if !self.connections.is_empty()
            && self.local_connection_id_length as usize != lb_config.connection_id_length
        {
            return Err(Error::InvalidState);
        }
        if self.connection_id_callback_fn.is_some() && self.connection_id_callback_ctx.is_some() {
            return Err(Error::InvalidState);
        }
        if lb_config.connection_id_length > CONNECTION_ID_MAX_SIZE {
            return Err(Error::InvalidArgument);
        }
        match lb_config.method {
            ConnectionIdMethod::Clear => {
                if lb_config.server_id_length + 1 > lb_config.connection_id_length {
                    return Err(Error::InvalidArgument);
                }
            }
            ConnectionIdMethod::StreamCipher => {
                if lb_config.nonce_length < 8
                    || lb_config.nonce_length > 16
                    || lb_config.nonce_length + lb_config.server_id_length + 1
                        > lb_config.connection_id_length
                {
                    return Err(Error::InvalidArgument);
                }
            }
            ConnectionIdMethod::BlockCipher => {
                if lb_config.connection_id_length < 17 || lb_config.server_id_length > 15 {
                    return Err(Error::InvalidArgument);
                }
            }
        }

        // Encode server_id as big-endian bytes.
        let mut server_id_encoded = [0u8; 16];
        let mut s_id64 = lb_config.server_id;
        for i in 0..lb_config.server_id_length {
            let j = lb_config.server_id_length - i - 1;
            server_id_encoded[j] = s_id64 as u8;
            s_id64 >>= 8;
        }
        if s_id64 != 0 {
            return Err(Error::InvalidArgument);
        }

        let cid_encryption_context = match lb_config.method {
            ConnectionIdMethod::StreamCipher | ConnectionIdMethod::BlockCipher => Some(Box::new(
                Aes128EcbContext::new(true, &lb_config.cid_encryption_key),
            )),
            ConnectionIdMethod::Clear => None,
        };
        let cid_decryption_context = match lb_config.method {
            ConnectionIdMethod::BlockCipher => Some(Box::new(Aes128EcbContext::new(
                false,
                &lb_config.cid_encryption_key,
            ))),
            _ => None,
        };

        let context = ConnectionIdContext {
            method: lb_config.method,
            rotation_bits: lb_config.rotation_bits,
            first_byte_encodes_length: lb_config.first_byte_encodes_length,
            server_id_length: lb_config.server_id_length,
            nonce_length: lb_config.nonce_length,
            connection_id_length: lb_config.connection_id_length,
            server_id: lb_config.server_id,
            server_id_encoded,
            cid_encryption_context,
            cid_decryption_context,
        };

        self.local_connection_id_length = lb_config.connection_id_length as u8;
        self.connection_id_callback_ctx = Some(Box::new(context));
        Ok(())
    }

    /// Tear down the [`ConnectionIdContext`] previously installed by
    /// [`Quic::set_lb_cid_config`], releasing the AES-ECB
    /// encryption contexts and clearing the callback slot on
    /// `self`.  No-op when no LB CID context is installed.
    /// C: `lb_compat_cid_config_free`.
    pub fn clear_lb_cid_config(&mut self) {
        let is_lb = self
            .connection_id_callback_ctx
            .as_ref()
            .is_some_and(|c| c.is::<ConnectionIdContext>());
        if is_lb {
            self.connection_id_callback_ctx = None;
            self.connection_id_callback_fn = None;
        }
    }
}

#[cfg(test)]
mod test {}
