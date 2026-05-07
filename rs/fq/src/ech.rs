//! Encrypted ClientHello (ECH) helpers.
//!
//! Pure-Rust portion of the ECH support layer, fully translated from
//! `picoquic/ech.c`.  HPKE key-exchange (DHKEM-X25519, DHKEM-P256,
//! DHKEM-P384) is implemented via the `hpke` crate; the remaining logic
//! — cipher-suite selection, config-ID derivation, base64 encoding, and
//! opener-state plumbing — is translated directly.

extern crate alloc;
use alloc::vec::Vec;

use crate::Error;

// ---------------------------------------------------------------------------
// HPKE constants (IANA registries, RFC 9180).

/// HPKE KEM algorithm identifiers (RFC 9180 §7.1).
pub mod kem_id {
    /// DHKEM(P-256, HKDF-SHA256)
    pub const P256_SHA256: u16 = 0x0010;
    /// DHKEM(P-384, HKDF-SHA384)
    pub const P384_SHA384: u16 = 0x0011;
    /// DHKEM(X25519, HKDF-SHA256)
    pub const X25519_SHA256: u16 = 0x0020;
}

/// HPKE KDF algorithm identifiers (RFC 9180 §7.2).
pub mod kdf_id {
    /// HKDF-SHA256
    pub const HKDF_SHA256: u16 = 0x0001;
    /// HKDF-SHA384
    pub const HKDF_SHA384: u16 = 0x0002;
    /// HKDF-SHA512
    pub const HKDF_SHA512: u16 = 0x0003;
}

/// HPKE AEAD algorithm identifiers (RFC 9180 §7.3).
pub mod aead_id {
    /// AES-128-GCM
    pub const AES_128_GCM: u16 = 0x0001;
    /// AES-256-GCM
    pub const AES_256_GCM: u16 = 0x0002;
    /// ChaCha20-Poly1305
    pub const CHACHA20_POLY1305: u16 = 0x0003;
}

// ---------------------------------------------------------------------------
// Cipher-suite identifier.

/// HPKE symmetric cipher-suite: (KDF id, AEAD id) pair.
///
/// C: `ptls_hpke_cipher_suite_id_t`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HpkeCipherSuiteId {
    pub kdf: u16,
    pub aead: u16,
}

/// The three cipher suites we advertise as supported, mirroring the
/// `picoquic_hpke_cipher_suites[]` array that the C picotls backend
/// populates at startup.
const SUPPORTED_CIPHER_SUITES: &[HpkeCipherSuiteId] = &[
    HpkeCipherSuiteId {
        kdf: kdf_id::HKDF_SHA256,
        aead: aead_id::AES_128_GCM,
    },
    HpkeCipherSuiteId {
        kdf: kdf_id::HKDF_SHA256,
        aead: aead_id::CHACHA20_POLY1305,
    },
    HpkeCipherSuiteId {
        kdf: kdf_id::HKDF_SHA384,
        aead: aead_id::AES_256_GCM,
    },
];

// ---------------------------------------------------------------------------
// Base64 encoding.

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const OID_ALGO_SECP: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
const OID_PR_SECP256R1: &[u8] = &[0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
const OID_PR_SECP384R1: &[u8] = &[0x06, 0x05, 0x2b, 0x81, 0x04, 0x00, 0x22];
const OID_X25519: &[u8] = &[0x2b, 0x65, 0x6e];

/// Return the number of base64 characters needed to encode `input_len` bytes,
/// not counting a NUL terminator.  Mirrors `ptls_base64_howlong`.
#[inline]
pub fn base64_encoded_len(input_len: usize) -> usize {
    input_len.div_ceil(3) * 4
}

/// Encode `input` as standard RFC 4648 base64 into `out`.
///
/// `out` must be at least `base64_encoded_len(input.len()) + 1` bytes long
/// (the extra byte is for the NUL terminator written to match picotls
/// convention).  Returns the number of base64 characters written (excluding
/// the NUL).  Returns `Err(Error::Generic)` when `out` is too small.
///
/// C: `picoquic_base64_encode`
pub fn base64_encode(input: &[u8], out: &mut [u8]) -> Result<usize, Error> {
    let len = base64_encoded_len(input.len());
    if len + 1 > out.len() {
        return Err(Error::Generic);
    }
    let mut pos = 0usize;
    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        let b = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out[pos] = BASE64_ALPHABET[((b >> 18) & 0x3f) as usize];
        out[pos + 1] = BASE64_ALPHABET[((b >> 12) & 0x3f) as usize];
        out[pos + 2] = BASE64_ALPHABET[((b >> 6) & 0x3f) as usize];
        out[pos + 3] = BASE64_ALPHABET[(b & 0x3f) as usize];
        pos += 4;
    }
    match chunks.remainder() {
        [a] => {
            let b = (*a as u32) << 16;
            out[pos] = BASE64_ALPHABET[((b >> 18) & 0x3f) as usize];
            out[pos + 1] = BASE64_ALPHABET[((b >> 12) & 0x3f) as usize];
            out[pos + 2] = b'=';
            out[pos + 3] = b'=';
            pos += 4;
        }
        [a, b] => {
            let v = ((*a as u32) << 16) | ((*b as u32) << 8);
            out[pos] = BASE64_ALPHABET[((v >> 18) & 0x3f) as usize];
            out[pos + 1] = BASE64_ALPHABET[((v >> 12) & 0x3f) as usize];
            out[pos + 2] = BASE64_ALPHABET[((v >> 6) & 0x3f) as usize];
            out[pos + 3] = b'=';
            pos += 4;
        }
        _ => {}
    }
    out[pos] = 0; // NUL terminator (mirrors ptls_base64_encode convention)
    Ok(len)
}

// ---------------------------------------------------------------------------
// Cipher-suite selector.

/// Fill `out` with HPKE cipher suites appropriate for `kem_id`, returning
/// the number of entries written.
///
/// The first entry is the preferred suite for the given KEM; when there is
/// also a suitable default suite (AES-128-GCM + HKDF-SHA256) that differs
/// from the preferred one and `out.len() > 2`, it is appended as the second
/// entry.
///
/// Returns `Err(Error::Generic)` if `out.len() < 2` (mirrors the C guard
/// `cipher_vec_nb_max < 2`) or if no suite can be found.
///
/// C: `picoquic_ech_get_ciphers_from_kem`
pub fn ech_get_ciphers_from_kem(
    kem_id: u16,
    out: &mut [HpkeCipherSuiteId],
) -> Result<usize, Error> {
    if out.len() < 2 {
        return Err(Error::Generic);
    }

    let (target_kdf, target_aead) = match kem_id {
        kem_id::P256_SHA256 => (kdf_id::HKDF_SHA256, aead_id::AES_128_GCM),
        kem_id::P384_SHA384 => (kdf_id::HKDF_SHA384, aead_id::AES_256_GCM),
        kem_id::X25519_SHA256 => (kdf_id::HKDF_SHA256, aead_id::CHACHA20_POLY1305),
        _ => (kdf_id::HKDF_SHA256, aead_id::AES_128_GCM),
    };
    let target = HpkeCipherSuiteId {
        kdf: target_kdf,
        aead: target_aead,
    };
    let default_suite = HpkeCipherSuiteId {
        kdf: kdf_id::HKDF_SHA256,
        aead: aead_id::AES_128_GCM,
    };

    let mut target_cipher: Option<HpkeCipherSuiteId> = None;
    let mut default_cipher: Option<HpkeCipherSuiteId> = None;

    for &suite in SUPPORTED_CIPHER_SUITES {
        if target_cipher.is_none() && suite == target {
            target_cipher = Some(suite);
        }
        if default_cipher.is_none() && suite == default_suite {
            default_cipher = Some(suite);
        }
    }

    // Mirrors the C branching: target first; fall back to default; error if neither.
    // C: `cipher_vec_nb_max > 2` guards adding the default as a second entry.
    let nb_ciphers = match (target_cipher, default_cipher) {
        (Some(t), def) => {
            out[0] = t;
            if let Some(d) = def {
                if d != t && out.len() > 2 {
                    out[1] = d;
                    2
                } else {
                    1
                }
            } else {
                1
            }
        }
        (None, Some(d)) => {
            out[0] = d;
            1
        }
        (None, None) => return Err(Error::Generic),
    };
    Ok(nb_ciphers)
}

// ---------------------------------------------------------------------------
// Config-ID derivation (private helper).

/// Compute the one-byte ECH config-ID from the public key material, KEM id,
/// cipher suites, and the server public name.
///
/// The algorithm accumulates all bytes into a `u64` then XOR-folds the
/// result down to a single byte.
///
/// C: `ech_config_id_from_config` (file-static)
fn ech_config_id_from_config(
    public_key_bits: &[u8],
    kem_id: u16,
    cipher_suites: &[HpkeCipherSuiteId],
    public_name: &str,
) -> u8 {
    let mut config_sum: u64 = 0;
    for &b in public_key_bits {
        config_sum = config_sum.wrapping_add(b as u64);
    }
    config_sum = config_sum.wrapping_add(kem_id as u64);
    for cs in cipher_suites {
        config_sum = config_sum.wrapping_add(cs.aead as u64);
        config_sum = config_sum.wrapping_add(cs.kdf as u64);
    }
    for b in public_name.bytes() {
        config_sum = config_sum.wrapping_add(b as u64);
    }
    let mut config_id: u8 = 0;
    while config_sum > 0 {
        config_id ^= (config_sum & 0xff) as u8;
        config_sum >>= 8;
    }
    config_id
}

// ---------------------------------------------------------------------------
// Server-side ECH opener state.

/// Result returned by a successful [`EchOpenerState::open`] call.
///
/// `key_material` holds the HPKE-derived key material exported from the
/// receiver context; callers build the concrete AEAD context from it
/// together with `cipher`.
pub struct EchOpenResult {
    /// The negotiated cipher suite.
    pub cipher: HpkeCipherSuiteId,
    /// HPKE-exported key material (32 bytes) derived via RFC 9180
    /// `Context.Export`.  Callers use this to construct the AEAD context for
    /// decrypting the ECH inner ClientHello.
    pub key_material: Vec<u8>,
}

/// Server-side ECH opener callback state.
///
/// Holds the KEM id, the private key bytes used for HPKE decapsulation,
/// and the serialised ECH config bytes (used to reconstruct the HPKE `info`
/// string during opening).
///
/// C: `ech_opener_callback_t`
pub struct EchOpenerState {
    /// HPKE KEM identifier, e.g. `kem_id::X25519_SHA256`.
    /// C: `kem` (`ptls_hpke_kem_t*`; we store only the numeric id).
    pub kem_id: u16,
    /// Raw private key bytes for HPKE decapsulation.
    /// C: `keyex` (`ptls_key_exchange_context_t*` loaded from a PEM file).
    pub private_key: Vec<u8>,
    /// Serialised ECH config bytes including the two-byte outer length prefix.
    /// C: `config` (`ptls_buffer_t`).
    pub config: Vec<u8>,
}

impl EchOpenerState {
    /// Attempt to open (decapsulate) an incoming ECH extension.
    ///
    /// Finds the cipher suite matching `cipher_id` in the supported list,
    /// builds the HPKE `info` string as `info_prefix || config[2..]`, then
    /// performs HPKE base-mode receiver setup (RFC 9180 §5.1.1) to derive
    /// the AEAD key material.
    ///
    /// Returns `Some(EchOpenResult)` on success, `None` when `cipher_id` is
    /// not in the supported list or the key bytes cannot be deserialized.
    ///
    /// C: `ech_opener_callback`
    pub fn open(
        &self,
        cipher_id: HpkeCipherSuiteId,
        enc: &[u8],
        info_prefix: &[u8],
    ) -> Option<EchOpenResult> {
        // Find the matching cipher suite in the supported list.
        let cipher = SUPPORTED_CIPHER_SUITES
            .iter()
            .copied()
            .find(|c| c.kdf == cipher_id.kdf && c.aead == cipher_id.aead)?;

        // Build info = info_prefix || config[2..]
        // (config[0..2] is the two-byte ECHConfigList outer length prefix,
        // skipped to match the `ptls_buffer_pushv(&infobuf, config.base+2, …)`
        // call in the C implementation.)
        let config_payload = self.config.get(2..).unwrap_or(&[]);
        let mut info = Vec::with_capacity(info_prefix.len() + config_payload.len());
        info.extend_from_slice(info_prefix);
        info.extend_from_slice(config_payload);

        // HPKE base-mode receiver setup (equivalent to ptls_hpke_setup_base_r).
        let key_material = hpke_setup_receiver(self.kem_id, cipher, &self.private_key, enc, &info)?;
        Some(EchOpenResult {
            cipher,
            key_material,
        })
    }
}

// ---------------------------------------------------------------------------
// HPKE receiver setup dispatcher.

/// Perform HPKE base-mode receiver setup (RFC 9180 §5.1.1) for the given
/// `(kem, cipher)` combination and return 32 bytes of exported key material
/// via `Context.Export`.  Returns `None` on key-format mismatch or an
/// unknown algorithm combination.
///
/// Dispatches over the three KEM/KDF/AEAD triples that picoquic supports.
/// The C equivalent is the `ptls_hpke_setup_base_r` call inside
/// `ech_opener_callback`.
fn hpke_setup_receiver(
    kem: u16,
    cipher: HpkeCipherSuiteId,
    private_key: &[u8],
    enc: &[u8],
    info: &[u8],
) -> Option<Vec<u8>> {
    use hpke::{Deserializable, Kem as HpkeKem, OpModeR};

    macro_rules! run_hpke {
        ($KemType:ty, $KdfType:ty, $AeadType:ty) => {{
            let sk =
                <$KemType as HpkeKem>::PrivateKey::from_bytes(private_key).ok()?;
            let ek =
                <$KemType as HpkeKem>::EncappedKey::from_bytes(enc).ok()?;
            let ctx = hpke::setup_receiver::<$AeadType, $KdfType, $KemType>(
                &OpModeR::Base,
                &sk,
                &ek,
                info,
            )
            .ok()?;
            let mut out = alloc::vec![0u8; 32];
            ctx.export(b"picoquic ech key material", &mut out).ok()?;
            Some(out)
        }};
    }

    match (kem, cipher.kdf, cipher.aead) {
        (kem_id::X25519_SHA256, kdf_id::HKDF_SHA256, aead_id::AES_128_GCM) => {
            run_hpke!(
                hpke::kem::X25519HkdfSha256,
                hpke::kdf::HkdfSha256,
                hpke::aead::AesGcm128
            )
        }
        (kem_id::X25519_SHA256, kdf_id::HKDF_SHA256, aead_id::CHACHA20_POLY1305) => {
            run_hpke!(
                hpke::kem::X25519HkdfSha256,
                hpke::kdf::HkdfSha256,
                hpke::aead::ChaCha20Poly1305
            )
        }
        (kem_id::P256_SHA256, kdf_id::HKDF_SHA256, aead_id::AES_128_GCM) => {
            run_hpke!(
                hpke::kem::DhP256HkdfSha256,
                hpke::kdf::HkdfSha256,
                hpke::aead::AesGcm128
            )
        }
        (kem_id::P384_SHA384, kdf_id::HKDF_SHA384, aead_id::AES_256_GCM) => {
            run_hpke!(
                hpke::kem::DhP384HkdfSha384,
                hpke::kdf::HkdfSha384,
                hpke::aead::AesGcm256
            )
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// TLS group-id → HPKE KEM-id mapping.

/// TLS named-group identifiers (RFC 8446 §4.2.7, as used by picotls).
pub mod group_id {
    /// secp256r1 / P-256  (`PTLS_GROUP_SECP256R1`)
    pub const SECP256R1: u16 = 23;
    /// secp384r1 / P-384  (`PTLS_GROUP_SECP384R1`)
    pub const SECP384R1: u16 = 24;
    /// X25519             (`PTLS_GROUP_X25519`)
    pub const X25519: u16 = 29;
}

/// Find the HPKE KEM identifier for the given TLS named-group id.
///
/// Mirrors the C loop over `picoquic_hpke_kems[]` that matches on
/// `picoquic_hpke_kems[i]->keyex->id == group_id`.
///
/// Returns `Some(kem_id)` on success, `None` when the group is not
/// one of the three supported curves.
///
/// C: `picoquic_ech_get_kem_from_curve`
pub fn ech_get_kem_from_curve(group_id: u16) -> Option<u16> {
    match group_id {
        group_id::SECP256R1 => Some(kem_id::P256_SHA256),
        group_id::SECP384R1 => Some(kem_id::P384_SHA384),
        group_id::X25519 => Some(kem_id::X25519_SHA256),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ASN.1 DER SubjectPublicKeyInfo parser.

/// Parsed fields extracted from a DER-encoded SubjectPublicKeyInfo blob.
///
/// All slices borrow from the original input passed to
/// [`parse_public_key_asn1`], so their lifetime is tied to that input.
///
/// C: the four output parameters of `picoquic_parse_public_key_asn1`.
pub struct PublicKeyAsn1<'a> {
    /// Algorithm OID value bytes (tag + length stripped).
    ///
    /// C: `public_key_algo.{base,len}`
    pub algo: &'a [u8],
    /// Parameters field: raw bytes from after the algorithm OID to the
    /// end of the inner AlgorithmIdentifier SEQUENCE.  Includes the
    /// tag + length of any nested TLV; empty when there are no parameters
    /// (e.g. X25519 keys).
    ///
    /// C: `public_key_param.{base,len}`
    pub param: &'a [u8],
    /// BIT STRING value bytes.  The first byte is the padding-count
    /// (always `0x00` for the curves picoquic supports); the key bytes
    /// follow immediately.
    ///
    /// C: `public_key_bit_string.{base,len}`
    pub bit_string: &'a [u8],
    /// Total number of bytes consumed from the input.
    pub consumed: usize,
}

/// Parse a DER TLV header at `pos` within `bytes[..bytes_max]`.
///
/// `expected_type` must match the tag byte.  Returns `(value_start, value_end)`
/// on success, `Err` on type mismatch, truncation, or overflow.
fn asn1_tlv(
    bytes: &[u8],
    bytes_max: usize,
    pos: usize,
    expected_type: u8,
) -> Result<(usize, usize), Error> {
    if pos >= bytes_max || bytes[pos] != expected_type {
        return Err(Error::Generic);
    }
    let pos = pos + 1;
    if pos >= bytes_max {
        return Err(Error::Generic);
    }
    let first = bytes[pos];
    let (length, val_start) = if first < 0x80 {
        (first as usize, pos + 1)
    } else {
        let n = (first & 0x7F) as usize;
        if n == 0 || pos + 1 + n > bytes_max {
            return Err(Error::Generic);
        }
        let mut len = 0usize;
        for i in 0..n {
            len = (len << 8) | (bytes[pos + 1 + i] as usize);
        }
        (len, pos + 1 + n)
    };
    let val_end = val_start + length;
    if val_end > bytes_max {
        return Err(Error::Generic);
    }
    Ok((val_start, val_end))
}

/// Parse a DER-encoded SubjectPublicKeyInfo structure.
///
/// Walks the outer SEQUENCE → AlgorithmIdentifier SEQUENCE → OID +
/// optional parameters → BIT STRING layout described in RFC 5480 and
/// RFC 8410, returning zero-copy sub-slices into `input`.
///
/// Returns `Err(Error::Generic)` for any structural violation (wrong tag,
/// truncated length field, outer-sequence length mismatch, or trailing
/// bytes after the BIT STRING).
///
/// C: `picoquic_parse_public_key_asn1`
pub fn parse_public_key_asn1(input: &[u8]) -> Result<PublicKeyAsn1<'_>, Error> {
    let bytes_max = input.len();

    // Outer SEQUENCE must span the entire buffer.
    let (outer_val, outer_end) = asn1_tlv(input, bytes_max, 0, 0x30)?;
    if outer_end != bytes_max {
        return Err(Error::Generic); // PTLS_ERROR_BER_EXCESSIVE_LENGTH
    }

    // AlgorithmIdentifier SEQUENCE.
    let (inner_val, inner_end) = asn1_tlv(input, outer_end, outer_val, 0x30)?;

    // Algorithm OID — value bytes only (tag + length consumed by asn1_tlv).
    let (oid_val, oid_end) = asn1_tlv(input, inner_end, inner_val, 0x06)?;
    let algo = &input[oid_val..oid_end];

    // Parameters: everything remaining in the inner SEQUENCE after the OID.
    let param = if inner_end <= oid_end {
        &input[0..0]
    } else {
        &input[oid_end..inner_end]
    };

    // BIT STRING (public key), must consume the rest of the outer SEQUENCE.
    let (bs_val, bs_end) = asn1_tlv(input, outer_end, inner_end, 0x03)?;
    if bs_end != outer_end {
        return Err(Error::Generic); // PTLS_ERROR_BER_ELEMENT_TOO_SHORT
    }
    let bit_string = &input[bs_val..bs_end];

    Ok(PublicKeyAsn1 {
        algo,
        param,
        bit_string,
        consumed: bs_end,
    })
}

/// Extract the TLS named-group id and HPKE public key bytes from a
/// DER-encoded SubjectPublicKeyInfo blob.
///
/// C: `picoquic_ech_parse_public_key`
pub fn ech_parse_public_key(public_key_asn1: &[u8]) -> Result<(u16, &[u8]), Error> {
    let parsed = parse_public_key_asn1(public_key_asn1)?;
    let (group_id, expected_key_length) = if parsed.algo == OID_ALGO_SECP {
        if parsed.param == OID_PR_SECP256R1 {
            (group_id::SECP256R1, 0x41usize)
        } else if parsed.param == OID_PR_SECP384R1 {
            (group_id::SECP384R1, 0x61usize)
        } else {
            return Err(Error::Generic);
        }
    } else if parsed.algo == OID_X25519 {
        (group_id::X25519, 0x20usize)
    } else {
        return Err(Error::Generic);
    };

    if parsed.bit_string.len() <= 1 || parsed.bit_string[0] != 0 {
        return Err(Error::Generic);
    }
    let public_key_bits = &parsed.bit_string[1..];
    if public_key_bits.len() != expected_key_length {
        return Err(Error::Generic);
    }
    Ok((group_id, public_key_bits))
}

// ---------------------------------------------------------------------------
// Base64 decoder (companion to `base64_encode`).

/// Decode standard base64 (RFC 4648) from `input` into `out`.
///
/// Ignores whitespace and `=` padding characters.  Returns
/// `Err(Error::Generic)` for any non-base64, non-whitespace, non-padding
/// character.
///
/// C: `picoquic_base64_decode` (picoquic/ech.c:50)
fn base64_decode(input: &[u8], out: &mut Vec<u8>) -> Result<(), Error> {
    let decode_char = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };

    let mut accum: u32 = 0;
    let mut nbits: u32 = 0;

    for &c in input {
        if c == b'=' || c == b'\n' || c == b'\r' || c == b' ' || c == b'\t' {
            continue;
        }
        let v = decode_char(c).ok_or(Error::Generic)?;
        accum = (accum << 6) | v as u32;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            out.push((accum >> nbits) as u8);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// PEM / DER helpers for private key loading.

/// Strip PEM header/footer lines and base64-decode the body.
///
/// Handles both bare base64 (no headers) and PEM-wrapped base64
/// (`-----BEGIN ...-----` / `-----END ...-----`).  Lines that start
/// with `-----` are silently skipped; all other content is fed to
/// [`base64_decode`].
///
/// Returns the decoded bytes.
fn pem_base64_decode(pem: &str) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    for line in pem.lines() {
        let line = line.trim();
        if line.starts_with("-----") {
            continue;
        }
        base64_decode(line.as_bytes(), &mut out)?;
    }
    if out.is_empty() {
        return Err(Error::Generic);
    }
    Ok(out)
}

/// Extract the raw scalar bytes of a private key from DER-encoded data.
///
/// Handles two wire formats:
///
/// * **RFC 5915** (`EC PRIVATE KEY`): `SEQUENCE { INTEGER 1, OCTET STRING
///   { key }, … }` — the private key scalar is the first OCTET STRING.
///
/// * **PKCS#8 / OneAsymmetricKey** (`PRIVATE KEY`): `SEQUENCE { INTEGER
///   0, AlgorithmIdentifier SEQUENCE, OCTET STRING { payload } }`.
///   The `payload` is either:
///   - A nested OCTET STRING (for OKP keys: X25519, etc.) whose
///     content is the raw scalar, or
///   - An ECPrivateKey SEQUENCE (for EC keys) — parsed as RFC 5915.
fn extract_private_key_der(der: &[u8], label: &str) -> Result<Vec<u8>, Error> {
    let bytes_max = der.len();

    // Outer SEQUENCE.
    let (outer_val, outer_end) = asn1_tlv(der, bytes_max, 0, 0x30)?;

    if label.contains("EC PRIVATE KEY") {
        // RFC 5915: SEQUENCE { INTEGER version, OCTET STRING private_key, … }
        let (_, after_ver) = asn1_tlv(der, outer_end, outer_val, 0x02)?;
        let (pk_val, pk_end) = asn1_tlv(der, outer_end, after_ver, 0x04)?;
        Ok(der[pk_val..pk_end].to_vec())
    } else {
        // PKCS#8: SEQUENCE { INTEGER version, AlgorithmId SEQUENCE, OCTET STRING privateKey }
        let (_, after_ver) = asn1_tlv(der, outer_end, outer_val, 0x02)?;
        let (_, after_alg) = asn1_tlv(der, outer_end, after_ver, 0x30)?;
        let (pk_outer_val, pk_outer_end) = asn1_tlv(der, outer_end, after_alg, 0x04)?;

        if pk_outer_val < pk_outer_end && der[pk_outer_val] == 0x04 {
            // Nested OCTET STRING: OKP / X25519 CurvePrivateKey.
            let (inner_val, inner_end) = asn1_tlv(der, pk_outer_end, pk_outer_val, 0x04)?;
            Ok(der[inner_val..inner_end].to_vec())
        } else if pk_outer_val < pk_outer_end && der[pk_outer_val] == 0x30 {
            // ECPrivateKey SEQUENCE inside the privateKey OCTET STRING.
            let (ec_val, ec_end) = asn1_tlv(der, pk_outer_end, pk_outer_val, 0x30)?;
            let (_, after_ec_ver) = asn1_tlv(der, ec_end, ec_val, 0x02)?;
            let (ec_pk_val, ec_pk_end) = asn1_tlv(der, ec_end, after_ec_ver, 0x04)?;
            Ok(der[ec_pk_val..ec_pk_end].to_vec())
        } else {
            Err(Error::Generic)
        }
    }
}

fn group_from_private_key_len(private_key: &[u8]) -> Result<u16, Error> {
    match private_key.len() {
        32 => Ok(group_id::SECP256R1),
        48 => Ok(group_id::SECP384R1),
        _ => Err(Error::Generic),
    }
}

fn group_from_pkcs8_algorithm(
    der: &[u8],
    outer_end: usize,
    after_ver: usize,
) -> Result<u16, Error> {
    let (alg_val, alg_end) = asn1_tlv(der, outer_end, after_ver, 0x30)?;
    let (oid_val, oid_end) = asn1_tlv(der, alg_end, alg_val, 0x06)?;
    let algo = &der[oid_val..oid_end];
    let param = &der[oid_end..alg_end];
    if algo == OID_ALGO_SECP {
        if param == OID_PR_SECP256R1 {
            Ok(group_id::SECP256R1)
        } else if param == OID_PR_SECP384R1 {
            Ok(group_id::SECP384R1)
        } else {
            Err(Error::Generic)
        }
    } else if algo == OID_X25519 {
        Ok(group_id::X25519)
    } else {
        Err(Error::Generic)
    }
}

fn extract_private_key_der_with_group(der: &[u8], label: &str) -> Result<(u16, Vec<u8>), Error> {
    let bytes_max = der.len();
    let (outer_val, outer_end) = asn1_tlv(der, bytes_max, 0, 0x30)?;
    let (_, after_ver) = asn1_tlv(der, outer_end, outer_val, 0x02)?;

    let private_key = extract_private_key_der(der, label)?;
    if label.contains("EC PRIVATE KEY") {
        let mut group_id = group_from_private_key_len(&private_key)?;
        let after_pk = asn1_tlv(der, outer_end, after_ver, 0x04)?.1;
        if after_pk < outer_end && der[after_pk] == 0xa0 {
            let (param_val, param_end) = asn1_tlv(der, outer_end, after_pk, 0xa0)?;
            let param = &der[param_val..param_end];
            if param == OID_PR_SECP256R1 {
                group_id = group_id::SECP256R1;
            } else if param == OID_PR_SECP384R1 {
                group_id = group_id::SECP384R1;
            }
        }
        Ok((group_id, private_key))
    } else {
        let group_id = group_from_pkcs8_algorithm(der, outer_end, after_ver)?;
        Ok((group_id, private_key))
    }
}

fn derive_public_key_from_private(group_id: u16, private_key: &[u8]) -> Result<Vec<u8>, Error> {
    use hpke::{Deserializable, Kem as HpkeKem, Serializable};

    macro_rules! derive_for_kem {
        ($KemType:ty) => {{
            let sk = <$KemType as HpkeKem>::PrivateKey::from_bytes(private_key)
                .map_err(|_| Error::Generic)?;
            let pk = <$KemType as HpkeKem>::sk_to_pk(&sk);
            Ok(pk.to_bytes().to_vec())
        }};
    }

    match group_id {
        group_id::SECP256R1 => derive_for_kem!(hpke::kem::DhP256HkdfSha256),
        group_id::SECP384R1 => derive_for_kem!(hpke::kem::DhP384HkdfSha384),
        group_id::X25519 => derive_for_kem!(hpke::kem::X25519HkdfSha256),
        _ => Err(Error::Generic),
    }
}

// ---------------------------------------------------------------------------
// ECH config file reader.

/// Read a base64-encoded ECHConfigList from a text file and return the
/// decoded binary bytes.
///
/// The file may contain PEM headers (`-----BEGIN ...-----` /
/// `-----END ...-----`) or bare base64.  The decoded bytes are the
/// ECHConfigList wire format: a 2-byte big-endian total-length prefix
/// followed by one or more ECHConfig entries.
///
/// C: `picoquic_ech_read_config` (picoquic/ech.c:98)
pub fn ech_read_config_file(config_file_name: &str) -> Result<Vec<u8>, Error> {
    let text = std::fs::read_to_string(config_file_name).map_err(|_| Error::Generic)?;
    pem_base64_decode(&text)
}

/// Save a binary ECHConfigList to a base64-encoded text file.
///
/// C: `picoquic_ech_save_config`
pub fn ech_save_config_file(config: &[u8], config_file_name: &str) -> Result<(), Error> {
    let mut text = alloc::vec![0u8; base64_encoded_len(config.len()) + 1];
    let n = base64_encode(config, &mut text)?;
    text.truncate(n);
    text.push(b'\n');
    std::fs::write(config_file_name, text).map_err(|_| Error::Generic)
}

// ---------------------------------------------------------------------------
// ECH config encoder.

/// Encode a single ECHConfig into wire bytes (version + length +
/// ECHConfigContents).
///
/// The result is a self-contained ECHConfig entry suitable for inclusion
/// in an ECHConfigList.  The caller must prepend the 2-byte outer
/// ECHConfigList length when building the list.
///
/// C: `ptls_ech_encode_config` (picotls, called from
/// `picoquic_ech_create_config_from_pk`).
fn ech_encode_config(
    config_id: u8,
    kem_id: u16,
    public_key: &[u8],
    cipher_suites: &[HpkeCipherSuiteId],
    maximum_name_length: u8,
    public_name: &str,
) -> Result<Vec<u8>, Error> {
    let name_bytes = public_name.as_bytes();
    if name_bytes.len() > 255 {
        return Err(Error::Generic);
    }

    // Compute the length of ECHConfigContents:
    //   config_id (1) + kem_id (2) + pk_len (2) + pk (N)
    //   + cs_len (2) + cs entries (4 * n)
    //   + max_name_len (1) + name_len (1) + name (L)
    //   + extensions_len (2)
    let pk_len = public_key.len();
    let cs_bytes = cipher_suites.len() * 4;
    let name_len = name_bytes.len();
    let contents_len = 1 + 2 + 2 + pk_len + 2 + cs_bytes + 1 + 1 + name_len + 2;

    // ECHConfig wire: version (2) + contents_length (2) + contents.
    let mut buf = Vec::with_capacity(4 + contents_len);

    // version = 0xFE0D (ECH draft-13+)
    buf.extend_from_slice(&0xFE0Du16.to_be_bytes());
    // ECHConfigContents length
    buf.extend_from_slice(&(contents_len as u16).to_be_bytes());
    // config_id
    buf.push(config_id);
    // kem_id
    buf.extend_from_slice(&kem_id.to_be_bytes());
    // public_key (2-byte length prefix)
    buf.extend_from_slice(&(pk_len as u16).to_be_bytes());
    buf.extend_from_slice(public_key);
    // cipher_suites (2-byte length prefix; each suite is kdf_id:2 + aead_id:2)
    buf.extend_from_slice(&(cs_bytes as u16).to_be_bytes());
    for cs in cipher_suites {
        buf.extend_from_slice(&cs.kdf.to_be_bytes());
        buf.extend_from_slice(&cs.aead.to_be_bytes());
    }
    // maximum_name_length
    buf.push(maximum_name_length);
    // public_name (1-byte length prefix)
    buf.push(name_len as u8);
    buf.extend_from_slice(name_bytes);
    // extensions (empty list, 2-byte length = 0)
    buf.extend_from_slice(&0u16.to_be_bytes());

    Ok(buf)
}

// ---------------------------------------------------------------------------
// ECH opener initialiser.

/// Initialise a server-side ECH opener from on-disk key material.
///
/// Reads the binary ECH config from `config_file_name` (base64-encoded
/// ECHConfigList) and the EC/OKP private key from `private_key_file`
/// (PEM-encoded), then constructs and returns an [`EchOpenerState`].
///
/// The `config` field of the returned state is the decoded binary
/// ECHConfigList (2-byte outer length + ECHConfig).  The `private_key`
/// field is the raw scalar bytes required by the HPKE `PrivateKey::from_bytes`
/// call in [`EchOpenerState::open`].
///
/// C: `ech_init_opener_callback` (picoquic/ech.c:280)
pub fn ech_init_opener(
    private_key_file: &str,
    config_file_name: &str,
) -> Result<EchOpenerState, Error> {
    // Read and decode the ECHConfigList from the base64-encoded config file.
    let config = ech_read_config_file(config_file_name)?;

    // Extract kem_id from ECHConfigList bytes[7:9]:
    //   bytes[0:2] = ECHConfigList outer length
    //   bytes[2:4] = ECHConfig version (0xFE0D)
    //   bytes[4:6] = ECHConfig contents length
    //   bytes[6]   = config_id
    //   bytes[7:9] = kem_id
    if config.len() < 9 {
        return Err(Error::Generic);
    }
    let kem_id = (config[7] as u16) << 8 | config[8] as u16;

    // Validate the kem_id against the supported set.
    if !matches!(
        kem_id,
        kem_id::P256_SHA256 | kem_id::P384_SHA384 | kem_id::X25519_SHA256
    ) {
        return Err(Error::Generic);
    }

    // Read and parse the PEM private key file.
    let pem_text = std::fs::read_to_string(private_key_file).map_err(|_| Error::Generic)?;
    // Determine the key format from the PEM label.
    let label_start = pem_text.find("-----BEGIN ").ok_or(Error::Generic)? + "-----BEGIN ".len();
    let label_end = pem_text[label_start..]
        .find("-----")
        .ok_or(Error::Generic)?;
    let label = &pem_text[label_start..label_start + label_end];

    let der = pem_base64_decode(&pem_text)?;
    let private_key = extract_private_key_der(&der, label)?;

    Ok(EchOpenerState {
        kem_id,
        private_key,
        config,
    })
}

// ---------------------------------------------------------------------------
// ECH config builder from a public key.

/// Build an ECHConfig entry from an ASN.1 public key and a server public
/// name, returning the raw ECHConfig wire bytes.
///
/// 1. Derives the HPKE KEM from `group_id` (one of the TLS named-group
///    identifiers in [`group_id`]).
/// 2. Selects the preferred cipher suites for that KEM via
///    [`ech_get_ciphers_from_kem`].
/// 3. Computes a one-byte config-ID by XOR-folding the key material,
///    KEM id, cipher-suite ids, and the public name.
/// 4. Encodes the full ECHConfig wire structure.
///
/// The returned `Vec<u8>` is one ECHConfig (without the outer
/// ECHConfigList 2-byte total-length prefix).  Wrap it with that prefix
/// to obtain a complete ECHConfigList.
///
/// C: `picoquic_ech_create_config_from_pk` (picoquic/ech.c:802)
pub fn ech_create_config_from_pk(
    group_id: u16,
    public_key_bits: &[u8],
    public_name: &str,
) -> Result<Vec<u8>, Error> {
    // Derive KEM id from the TLS named-group id.
    let kem_id = ech_get_kem_from_curve(group_id).ok_or(Error::Generic)?;

    // Collect the preferred cipher suites for this KEM.
    let mut cipher_buf = [HpkeCipherSuiteId { kdf: 0, aead: 0 }; 5];
    let n_ciphers = ech_get_ciphers_from_kem(kem_id, &mut cipher_buf)?;
    let cipher_suites = &cipher_buf[..n_ciphers];

    // Compute a one-byte config-ID from all the key material.
    let config_id = ech_config_id_from_config(public_key_bits, kem_id, cipher_suites, public_name);

    // Encode and return the ECHConfig wire bytes.
    ech_encode_config(
        config_id,
        kem_id,
        public_key_bits,
        cipher_suites,
        255,
        public_name,
    )
}

/// Wrap a single ECHConfig entry in an ECHConfigList length prefix.
///
/// C: `picoquic_ech_create_config_list_from_config`
pub fn ech_create_config_list_from_config(config: &[u8]) -> Result<Vec<u8>, Error> {
    let config_len = u16::try_from(config.len()).map_err(|_| Error::Generic)?;
    let mut out = Vec::with_capacity(config.len() + 2);
    out.extend_from_slice(&config_len.to_be_bytes());
    out.extend_from_slice(config);
    Ok(out)
}

/// Create a single ECHConfig entry from a DER SubjectPublicKeyInfo blob.
///
/// C: `picoquic_ech_create_rr_from_binary`
pub fn ech_create_rr_from_binary(
    public_key_asn1: &[u8],
    public_name: &str,
) -> Result<Vec<u8>, Error> {
    let (group_id, public_key_bits) = ech_parse_public_key(public_key_asn1)?;
    ech_create_config_from_pk(group_id, public_key_bits, public_name)
}

/// Create an ECHConfigList from a DER SubjectPublicKeyInfo blob.
///
/// C: `picoquic_ech_create_config_from_binary`
pub fn ech_create_config_from_binary(
    public_key_asn1: &[u8],
    public_name: &str,
) -> Result<Vec<u8>, Error> {
    let config = ech_create_rr_from_binary(public_key_asn1, public_name)?;
    ech_create_config_list_from_config(&config)
}

/// Load a PEM `PUBLIC KEY` file and create an ECHConfigList.
///
/// C: `picoquic_ech_create_config_from_public_key`
pub fn ech_create_config_from_public_key_file(
    public_key_file: &str,
    public_name: &str,
) -> Result<Vec<u8>, Error> {
    let text = std::fs::read_to_string(public_key_file).map_err(|_| Error::Generic)?;
    let public_key_asn1 = pem_base64_decode(&text)?;
    ech_create_config_from_binary(&public_key_asn1, public_name)
}

/// Load a PEM private key, derive its public key, and create an ECHConfigList.
///
/// C: `picoquic_ech_create_config_from_private_key`
pub fn ech_create_config_from_private_key_file(
    private_key_file: &str,
    public_name: &str,
) -> Result<Vec<u8>, Error> {
    let text = std::fs::read_to_string(private_key_file).map_err(|_| Error::Generic)?;
    let label_start = text.find("-----BEGIN ").ok_or(Error::Generic)? + "-----BEGIN ".len();
    let label_end = text[label_start..].find("-----").ok_or(Error::Generic)?;
    let label = &text[label_start..label_start + label_end];
    let der = pem_base64_decode(&text)?;
    let (group_id, private_key) = extract_private_key_der_with_group(&der, label)?;
    let public_key = derive_public_key_from_private(group_id, &private_key)?;
    let config = ech_create_config_from_pk(group_id, &public_key, public_name)?;
    ech_create_config_list_from_config(&config)
}

// ---------------------------------------------------------------------------
// Tests.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_empty() {
        let mut buf = [0u8; 1];
        let n = base64_encode(&[], &mut buf).unwrap();
        assert_eq!(n, 0);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn base64_encode_three_bytes() {
        // "Man" → "TWFu"
        let mut buf = [0u8; 5];
        let n = base64_encode(b"Man", &mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(&buf[..4], b"TWFu");
        assert_eq!(buf[4], 0);
    }

    #[test]
    fn base64_encode_padding_one() {
        // "M" → "TQ=="
        let mut buf = [0u8; 5];
        let n = base64_encode(b"M", &mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(&buf[..4], b"TQ==");
    }

    #[test]
    fn base64_encode_padding_two() {
        // "Ma" → "TWE="
        let mut buf = [0u8; 5];
        let n = base64_encode(b"Ma", &mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(&buf[..4], b"TWE=");
    }

    #[test]
    fn base64_encode_buffer_too_small() {
        let mut buf = [0u8; 4]; // needs 5 (4 chars + NUL)
        assert!(base64_encode(b"Man", &mut buf).is_err());
    }

    #[test]
    fn base64_encoded_len_values() {
        assert_eq!(base64_encoded_len(0), 0);
        assert_eq!(base64_encoded_len(1), 4);
        assert_eq!(base64_encoded_len(2), 4);
        assert_eq!(base64_encoded_len(3), 4);
        assert_eq!(base64_encoded_len(4), 8);
        assert_eq!(base64_encoded_len(6), 8);
    }

    #[test]
    fn ech_get_ciphers_p256() {
        let mut out = [HpkeCipherSuiteId { kdf: 0, aead: 0 }; 4];
        let n = ech_get_ciphers_from_kem(kem_id::P256_SHA256, &mut out).unwrap();
        assert!(n >= 1);
        assert_eq!(out[0].kdf, kdf_id::HKDF_SHA256);
        assert_eq!(out[0].aead, aead_id::AES_128_GCM);
    }

    #[test]
    fn ech_get_ciphers_p384() {
        let mut out = [HpkeCipherSuiteId { kdf: 0, aead: 0 }; 4];
        let n = ech_get_ciphers_from_kem(kem_id::P384_SHA384, &mut out).unwrap();
        assert!(n >= 1);
        assert_eq!(out[0].kdf, kdf_id::HKDF_SHA384);
        assert_eq!(out[0].aead, aead_id::AES_256_GCM);
    }

    #[test]
    fn ech_get_ciphers_x25519() {
        let mut out = [HpkeCipherSuiteId { kdf: 0, aead: 0 }; 4];
        let n = ech_get_ciphers_from_kem(kem_id::X25519_SHA256, &mut out).unwrap();
        assert!(n >= 1);
        assert_eq!(out[0].kdf, kdf_id::HKDF_SHA256);
        assert_eq!(out[0].aead, aead_id::CHACHA20_POLY1305);
    }

    #[test]
    fn ech_get_ciphers_out_too_small() {
        let mut out = [HpkeCipherSuiteId { kdf: 0, aead: 0 }; 1];
        assert!(ech_get_ciphers_from_kem(kem_id::P256_SHA256, &mut out).is_err());
    }

    #[test]
    fn config_id_is_deterministic() {
        let key = [1u8, 2, 3, 4];
        let suites = [HpkeCipherSuiteId {
            kdf: kdf_id::HKDF_SHA256,
            aead: aead_id::AES_128_GCM,
        }];
        let id1 = ech_config_id_from_config(&key, kem_id::X25519_SHA256, &suites, "example.com");
        let id2 = ech_config_id_from_config(&key, kem_id::X25519_SHA256, &suites, "example.com");
        assert_eq!(id1, id2);
    }

    #[test]
    fn config_id_changes_with_key() {
        let suites = [HpkeCipherSuiteId {
            kdf: kdf_id::HKDF_SHA256,
            aead: aead_id::AES_128_GCM,
        }];
        let id1 =
            ech_config_id_from_config(&[1, 2, 3], kem_id::X25519_SHA256, &suites, "example.com");
        let id2 =
            ech_config_id_from_config(&[4, 5, 6], kem_id::X25519_SHA256, &suites, "example.com");
        assert_ne!(id1, id2);
    }

    #[test]
    fn kem_from_curve_known_groups() {
        assert_eq!(
            ech_get_kem_from_curve(group_id::SECP256R1),
            Some(kem_id::P256_SHA256)
        );
        assert_eq!(
            ech_get_kem_from_curve(group_id::SECP384R1),
            Some(kem_id::P384_SHA384)
        );
        assert_eq!(
            ech_get_kem_from_curve(group_id::X25519),
            Some(kem_id::X25519_SHA256)
        );
    }

    #[test]
    fn kem_from_curve_unknown() {
        assert_eq!(ech_get_kem_from_curve(0xFFFF), None);
    }

    // Minimal DER-encoded SubjectPublicKeyInfo for X25519 (RFC 8410):
    // SEQUENCE {
    //   SEQUENCE { OID 1.3.101.110 }        -- id-X25519
    //   BIT STRING { 0x00 <32 zero bytes> }  -- fake key, padding=0
    // }
    fn x25519_spki() -> Vec<u8> {
        let oid: &[u8] = &[0x2b, 0x65, 0x6e]; // 1.3.101.110
        // inner SEQUENCE: OID TLV
        let mut inner = Vec::new();
        inner.push(0x06);
        inner.push(oid.len() as u8);
        inner.extend_from_slice(oid);
        // BIT STRING: padding byte + 32-byte key
        let mut bs_val = vec![0x00u8]; // padding count
        bs_val.extend_from_slice(&[0xABu8; 32]);
        let mut outer = Vec::new();
        // inner SEQUENCE TLV
        outer.push(0x30);
        outer.push(inner.len() as u8);
        outer.extend_from_slice(&inner);
        // BIT STRING TLV
        outer.push(0x03);
        outer.push(bs_val.len() as u8);
        outer.extend_from_slice(&bs_val);
        // wrap in outer SEQUENCE
        let mut spki = Vec::new();
        spki.push(0x30);
        spki.push(outer.len() as u8);
        spki.extend_from_slice(&outer);
        spki
    }

    #[test]
    fn parse_public_key_asn1_x25519() {
        let spki = x25519_spki();
        let r = parse_public_key_asn1(&spki).unwrap();
        assert_eq!(r.algo, &[0x2b, 0x65, 0x6e]);
        assert_eq!(r.param, &[] as &[u8]); // X25519 has no parameters
        assert_eq!(r.bit_string[0], 0x00); // padding byte
        assert_eq!(r.bit_string.len(), 33); // 1 + 32 key bytes
        assert_eq!(r.consumed, spki.len());
    }

    #[test]
    fn parse_public_key_asn1_truncated() {
        let spki = x25519_spki();
        assert!(parse_public_key_asn1(&spki[..4]).is_err());
    }

    #[test]
    fn parse_public_key_asn1_wrong_outer_tag() {
        let mut spki = x25519_spki();
        spki[0] = 0x31; // SET instead of SEQUENCE
        assert!(parse_public_key_asn1(&spki).is_err());
    }
}
