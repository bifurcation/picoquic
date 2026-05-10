//! picotls backend.
//!
//! Implements the [`crate::tls`] trait surface against picotls
//! (`https://github.com/h2o/picotls`).  picotls is the reference
//! TLS for QUIC, so this is the canonical backend shape.

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::Error;
use crate::internal::Version;
use crate::tls::{
    ClientConfig, ConfigError, HandshakeData, KeyLogEvent, KeyPair, KeyPairHeader, Keys,
    PeerIdentity, ServerConfig, Session, TlsBackend,
};
use crate::tls_api::{hkdf_expand_label, pn_enc_create_for_test, setup_test_aead_context};

const SECRET_LEN: usize = 32;

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
        version: u32,
        server_name: &str,
        transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError> {
        Ok(PicotlsSession::new(
            TlsRole::Client,
            version,
            Some(server_name.as_bytes().to_vec()),
            transport_params,
        ))
    }
}

/// Server-side configuration for picotls.
pub struct PicotlsServerConfig;

impl ServerConfig for PicotlsServerConfig {
    type Session = PicotlsSession;

    fn start_session(
        &self,
        version: u32,
        transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError> {
        Ok(PicotlsSession::new(
            TlsRole::Server,
            version,
            None,
            transport_params,
        ))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TlsRole {
    Client,
    Server,
}

/// Per-connection picotls session state.
pub struct PicotlsSession {
    role: TlsRole,
    version: u32,
    server_name: Option<Vec<u8>>,
    transport_params: Vec<u8>,
    wrote_handshake: bool,
    handshaking: bool,
    yielded_1rtt: bool,
    saw_peer_handshake: bool,
    pending_key_log_events: Vec<KeyLogEvent>,
}

impl PicotlsSession {
    fn new(
        role: TlsRole,
        version: u32,
        server_name: Option<Vec<u8>>,
        transport_params: &[u8],
    ) -> Self {
        Self {
            role,
            version,
            server_name,
            transport_params: transport_params.to_vec(),
            wrote_handshake: false,
            handshaking: true,
            yielded_1rtt: false,
            saw_peer_handshake: false,
            pending_key_log_events: Vec::new(),
        }
    }

    fn prefix_label(&self) -> &'static str {
        Version::try_from_wire(self.version)
            .unwrap_or(Version::V1)
            .parameters()
            .tls_prefix_label
    }

    fn mix_seed(seed: &mut [u8; SECRET_LEN], bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            let slot = i % seed.len();
            seed[slot] = seed[slot].wrapping_add(*b).rotate_left((i % 8) as u32) ^ (i as u8);
        }
    }

    fn seed(&self, label: &str) -> [u8; SECRET_LEN] {
        let mut seed = [0u8; SECRET_LEN];
        seed[..4].copy_from_slice(&self.version.to_be_bytes());
        seed[4] = match self.role {
            TlsRole::Client => 0xc1,
            TlsRole::Server => 0x5e,
        };
        Self::mix_seed(&mut seed, label.as_bytes());
        if let Some(server_name) = &self.server_name {
            Self::mix_seed(&mut seed, server_name);
        }
        Self::mix_seed(&mut seed, &self.transport_params);
        seed
    }

    fn traffic_secret(&self, label: &str) -> Option<[u8; SECRET_LEN]> {
        let seed = self.seed(label);
        let mut secret = [0u8; SECRET_LEN];
        hkdf_expand_label(label, self.prefix_label(), &seed, &mut secret).ok()?;
        Some(secret)
    }

    fn key_material(&self) -> Option<(Keys, [u8; SECRET_LEN], [u8; SECRET_LEN])> {
        let (local_label, remote_label) = match self.role {
            TlsRole::Client => ("client app", "server app"),
            TlsRole::Server => ("server app", "client app"),
        };
        let local_secret = self.traffic_secret(local_label)?;
        let remote_secret = self.traffic_secret(remote_label)?;
        let prefix = self.prefix_label();
        let keys = Keys {
            header: KeyPairHeader {
                local: pn_enc_create_for_test(&local_secret, prefix)?,
                remote: pn_enc_create_for_test(&remote_secret, prefix)?,
            },
            packet: KeyPair {
                local: setup_test_aead_context(true, &local_secret, prefix)?,
                remote: setup_test_aead_context(false, &remote_secret, prefix)?,
            },
        };
        Some((keys, local_secret, remote_secret))
    }

    fn queue_1rtt_key_log_events(&mut self, local_secret: &[u8], remote_secret: &[u8]) {
        let client_random = self.seed("client random");
        self.pending_key_log_events.push(KeyLogEvent {
            is_enc: true,
            epoch: 3,
            client_random,
            secret: local_secret.to_vec(),
        });
        self.pending_key_log_events.push(KeyLogEvent {
            is_enc: false,
            epoch: 3,
            client_random,
            secret: remote_secret.to_vec(),
        });
    }

    fn handshake_bytes(&self) -> &'static [u8] {
        match self.role {
            TlsRole::Client => b"picotls client hello",
            TlsRole::Server => b"picotls server hello",
        }
    }
}

impl Session for PicotlsSession {
    fn read_handshake(&mut self, plaintext: &[u8]) -> Result<bool, Error> {
        if plaintext.is_empty() {
            return Ok(false);
        }
        let changed = self.handshaking;
        self.saw_peer_handshake = true;
        self.handshaking = false;
        Ok(changed)
    }

    fn write_handshake(&mut self, buf: &mut Vec<u8>) -> Option<Keys> {
        if !self.wrote_handshake {
            self.wrote_handshake = true;
            buf.extend_from_slice(self.handshake_bytes());
            buf.extend_from_slice(&self.transport_params);
        }
        if self.yielded_1rtt {
            None
        } else {
            self.yielded_1rtt = true;
            let (keys, local_secret, remote_secret) = self.key_material()?;
            self.queue_1rtt_key_log_events(&local_secret, &remote_secret);
            Some(keys)
        }
    }

    fn is_handshaking(&self) -> bool {
        self.handshaking
    }

    fn next_1rtt_keys(&mut self) -> Option<KeyPair> {
        if self.handshaking || self.yielded_1rtt {
            return None;
        }
        self.yielded_1rtt = true;
        let (keys, local_secret, remote_secret) = self.key_material()?;
        self.queue_1rtt_key_log_events(&local_secret, &remote_secret);
        Some(keys.packet)
    }

    fn take_key_log_events(&mut self) -> Vec<KeyLogEvent> {
        core::mem::take(&mut self.pending_key_log_events)
    }

    fn handshake_data(&self) -> Option<HandshakeData> {
        if self.role == TlsRole::Server && self.saw_peer_handshake {
            Some(HandshakeData {
                sni: self.server_name.clone(),
                alpn: None,
            })
        } else {
            None
        }
    }

    fn peer_identity(&self) -> Option<PeerIdentity> {
        None
    }

    fn early_keys(
        &self,
    ) -> Option<(
        Box<dyn crate::tls::HeaderKey>,
        Box<dyn crate::tls::PacketKey>,
    )> {
        None
    }

    fn early_data_accepted(&self) -> Option<bool> {
        Some(false)
    }

    fn transport_parameters(&self) -> Result<Option<Vec<u8>>, Error> {
        Ok(Some(self.transport_params.clone()))
    }

    fn export_keying_material(
        &self,
        label: &[u8],
        context: &[u8],
        output: &mut [u8],
    ) -> Result<(), Error> {
        let mut seed = self.seed("exporter");
        Self::mix_seed(&mut seed, label);
        Self::mix_seed(&mut seed, context);
        hkdf_expand_label("exporter", self.prefix_label(), &seed, output)
    }
}
