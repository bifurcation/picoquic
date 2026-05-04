//! picotls backend.
//!
//! Implements the [`crate::tls`] trait surface against picotls
//! (`https://github.com/h2o/picotls`).  Phase 1 carries skeleton
//! types; the body lands once Rust bindings exist.  picotls is the
//! reference TLS for QUIC, so this is the canonical backend.
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

/// Top-level picotls backend marker.
pub struct Picotls;

impl TlsBackend for Picotls {
    type Client = PicotlsClientConfig;
    type Server = PicotlsServerConfig;
}

/// Client-side configuration for picotls.  Holds the cert
/// store, ALPN list, key share preferences, ticket-store hooks,
/// etc.  Application-side construction goes through a builder
/// (Phase 4).
pub struct PicotlsClientConfig;

impl ClientConfig for PicotlsClientConfig {
    type Session = PicotlsSession;

    fn start_session(
        &self,
        _version: u32,
        _server_name: &str,
        _transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError> {
        todo!()
    }
}

/// Server-side configuration for picotls.
pub struct PicotlsServerConfig;

impl ServerConfig for PicotlsServerConfig {
    type Session = PicotlsSession;

    fn start_session(
        &self,
        _version: u32,
        _transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError> {
        todo!()
    }
}

/// Per-connection picotls session.  Wraps `ptls_t*`.
pub struct PicotlsSession;

impl Session for PicotlsSession {
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

// Safety: picotls is single-threaded per connection; the pointer
// inside doesn't alias.  Phase 4 wires the actual `ptls_t*` and
// re-evaluates.
unsafe impl Send for PicotlsSession {}
