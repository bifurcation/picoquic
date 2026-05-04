//! OpenSSL backend (via the `openssl` crate).
//!
//! OpenSSL's QUIC API (introduced in 3.2) provides the building
//! blocks for a TLS-for-QUIC backend.  This module wires the
//! [`crate::tls`] traits to the `openssl` crate's bindings; the
//! Cargo feature `sys-openssl` gates the dependency.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::Error;
use crate::tls::{
    ClientConfig, ConfigError, HandshakeData, KeyPair, PeerIdentity, ServerConfig, Session,
    TlsBackend,
};

/// Top-level OpenSSL backend marker.
pub struct OpenSsl;

impl TlsBackend for OpenSsl {
    type Client = OpenSslClientConfig;
    type Server = OpenSslServerConfig;
}

/// Client-side configuration for OpenSSL.
pub struct OpenSslClientConfig;

impl ClientConfig for OpenSslClientConfig {
    type Session = OpenSslSession;

    fn start_session(
        &self,
        _version: u32,
        _server_name: &str,
        _transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError> {
        todo!()
    }
}

/// Server-side configuration for OpenSSL.
pub struct OpenSslServerConfig;

impl ServerConfig for OpenSslServerConfig {
    type Session = OpenSslSession;

    fn start_session(
        &self,
        _version: u32,
        _transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError> {
        todo!()
    }
}

/// Per-connection OpenSSL session.  Wraps `openssl::ssl::Ssl`
/// configured for QUIC.
pub struct OpenSslSession;

impl Session for OpenSslSession {
    fn read_handshake(&mut self, _plaintext: &[u8]) -> Result<bool, Error> {
        todo!()
    }

    fn write_handshake(&mut self, _buf: &mut Vec<u8>) -> Option<crate::tls::Keys> {
        todo!()
    }

    fn is_handshaking(&self) -> bool {
        todo!()
    }

    fn next_1rtt_keys(&mut self) -> Option<KeyPair> {
        todo!()
    }

    fn handshake_data(&self) -> Option<HandshakeData> {
        todo!()
    }

    fn peer_identity(&self) -> Option<PeerIdentity> {
        todo!()
    }

    fn early_keys(
        &self,
    ) -> Option<(
        Box<dyn crate::tls::HeaderKey>,
        Box<dyn crate::tls::PacketKey>,
    )> {
        todo!()
    }

    fn early_data_accepted(&self) -> Option<bool> {
        todo!()
    }

    fn transport_parameters(&self) -> Result<Option<Vec<u8>>, Error> {
        todo!()
    }

    fn export_keying_material(
        &self,
        _label: &[u8],
        _context: &[u8],
        _output: &mut [u8],
    ) -> Result<(), Error> {
        todo!()
    }
}

// Safety: the underlying `openssl::ssl::Ssl` is `Send` when
// configured for QUIC; the wrapper inherits that.  Once Phase 4
// wires the real type, this becomes derivable.
unsafe impl Send for OpenSslSession {}
