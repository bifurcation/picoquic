//! Numeric error codes used over the QUIC wire and inside picoquic.
//!
//! Two distinct ranges:
//!
//! * [`InternalError`] — picoquic-private codes allocated in the
//!   `0x400`+ range so they never collide with QUIC transport or
//!   TLS alert codes.  Surface inside the library; never put on the
//!   wire.
//! * [`TransportError`] — QUIC transport / TLS-alert codes from
//!   RFC 9000 §20 (and various drafts).  These cross the wire in
//!   CONNECTION_CLOSE frames.
//!
//! The `crate::Error` enum (defined in `lib.rs`) is the richer Rust
//! error surface used by fallible APIs; these enums provide the
//! numeric on-wire / on-log codes those errors map to.

// ---------------------------------------------------------------------------
// Picoquic-internal errors.

/// Base offset for picoquic's internal error codes.  Allocated in
/// the `0x400`+ range so they never collide with QUIC transport or
/// TLS alert codes.
pub const ERROR_CLASS: u64 = 0x400;

/// Picoquic-internal error code.  Numeric values follow C source.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u64)]
pub enum InternalError {
    Duplicate = ERROR_CLASS + 1,
    AeadCheck = ERROR_CLASS + 3,
    UnexpectedPacket = ERROR_CLASS + 4,
    Memory = ERROR_CLASS + 5,
    CnxidCheck = ERROR_CLASS + 7,
    InitialTooShort = ERROR_CLASS + 8,
    VersionNegotiationSpoofed = ERROR_CLASS + 9,
    MalformedTransportExtension = ERROR_CLASS + 10,
    ExtensionBufferTooSmall = ERROR_CLASS + 11,
    IllegalTransportExtension = ERROR_CLASS + 12,
    CannotResetStreamZero = ERROR_CLASS + 13,
    InvalidStreamId = ERROR_CLASS + 14,
    StreamAlreadyClosed = ERROR_CLASS + 15,
    FrameBufferTooSmall = ERROR_CLASS + 16,
    InvalidFrame = ERROR_CLASS + 17,
    CannotControlStreamZero = ERROR_CLASS + 18,
    Retry = ERROR_CLASS + 19,
    Disconnected = ERROR_CLASS + 20,
    Detected = ERROR_CLASS + 21,
    InvalidTicket = ERROR_CLASS + 23,
    InvalidFile = ERROR_CLASS + 24,
    SendBufferTooSmall = ERROR_CLASS + 25,
    UnexpectedState = ERROR_CLASS + 26,
    UnexpectedError = ERROR_CLASS + 27,
    TlsServerConWithoutCert = ERROR_CLASS + 28,
    NoSuchFile = ERROR_CLASS + 29,
    StatelessReset = ERROR_CLASS + 30,
    ConnectionDeleted = ERROR_CLASS + 31,
    CnxidSegment = ERROR_CLASS + 32,
    CnxidNotAvailable = ERROR_CLASS + 33,
    MigrationDisabled = ERROR_CLASS + 34,
    CannotComputeKey = ERROR_CLASS + 35,
    CannotSetActiveStream = ERROR_CLASS + 36,
    CannotChangeActiveContext = ERROR_CLASS + 37,
    InvalidToken = ERROR_CLASS + 38,
    InitialCidTooShort = ERROR_CLASS + 39,
    KeyRotationNotReady = ERROR_CLASS + 40,
    AeadNotReady = ERROR_CLASS + 41,
    NoAlpnProvided = ERROR_CLASS + 42,
    NoCallbackProvided = ERROR_CLASS + 43,
    /// Not actually an error: signals that the stream-receive
    /// callback consumed all available bytes.
    StreamReceiveComplete = ERROR_CLASS + 44,
    PacketHeaderParsing = ERROR_CLASS + 45,
    QuicBitMissing = ERROR_CLASS + 46,
    /// Not an error: terminates the packet loop cleanly.
    NoErrorTerminatePacketLoop = ERROR_CLASS + 47,
    /// Not an error: simulated NAT for tests.
    NoErrorSimulateNat = ERROR_CLASS + 48,
    /// Not an error: simulated migration for tests.
    NoErrorSimulateMigration = ERROR_CLASS + 49,
    VersionNotSupported = ERROR_CLASS + 50,
    IdleTimeout = ERROR_CLASS + 51,
    RepeatTimeout = ERROR_CLASS + 52,
    HandshakeTimeout = ERROR_CLASS + 53,
    SocketError = ERROR_CLASS + 54,
    VersionNegotiation = ERROR_CLASS + 55,
    PacketTooLong = ERROR_CLASS + 56,
    PacketWrongVersion = ERROR_CLASS + 57,
    PortBlocked = ERROR_CLASS + 58,
    DatagramTooLong = ERROR_CLASS + 59,
    PathIdInvalid = ERROR_CLASS + 60,
    RetryNeeded = ERROR_CLASS + 61,
    ServerBusy = ERROR_CLASS + 62,
    PathDuplicate = ERROR_CLASS + 63,
    PathIdBlocked = ERROR_CLASS + 64,
    PathCidBlocked = ERROR_CLASS + 65,
    PathAddressFamily = ERROR_CLASS + 66,
    PathNotReady = ERROR_CLASS + 67,
    PathLimitExceeded = ERROR_CLASS + 68,
    /// Not an error: the packet was captured by a proxy and needs
    /// no further processing.
    Redirected = ERROR_CLASS + 69,
    PaddingPacket = ERROR_CLASS + 70,
}

impl InternalError {
    /// Short textual name for `error_code`, taking a raw `u64` so
    /// callers can also name unknown / extension codes.  C:
    /// `error_name`.
    pub fn name(_error_code: u64) -> Option<&'static str> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// QUIC transport / TLS-alert protocol errors.

/// QUIC transport-layer error code (RFC 9000 §20.1) or TLS-alert
/// code routed through the QUIC `CRYPTO_ERROR(0x0100..=0x01ff)`
/// range.  Wire values fit in `u62` but extension drafts go above
/// `u32`, so the discriminant is `u64`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u64)]
pub enum TransportError {
    InternalError = 0x1,
    ServerBusy = 0x2,
    FlowControlError = 0x3,
    StreamLimitError = 0x4,
    StreamStateError = 0x5,
    FinalOffsetError = 0x6,
    FrameFormatError = 0x7,
    ParameterError = 0x8,
    ConnectionIdLimitError = 0x9,
    ProtocolViolation = 0xA,
    InvalidToken = 0xB,
    ApplicationError = 0xC,
    CryptoBufferExceeded = 0xD,
    KeyUpdateError = 0xE,
    AeadLimitReached = 0xF,

    VersionNegotiationError = 0x11,

    TlsAlertWrongAlpn = 0x178,
    TlsHandshakeFailed = 0x201,

    /// Per draft quic-multipath 20.
    ApplicationAbandon = 0x3e,
    ResourceLimitReached = 0x3e75,
    UnstableInterface = 0x3e76,
    NoCidAvailable = 0x3e77,
}

/// Build a `CRYPTO_ERROR` transport code from a TLS alert byte.
/// C macro: `TRANSPORT_CRYPTO_ERROR(alert)`.
pub const fn transport_crypto_error(alert: u8) -> u16 {
    0x100 | (alert as u16)
}

#[cfg(test)]
mod test {}
