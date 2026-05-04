//! QUIC frame types and (eventually) frame encode/decode logic.

/// QUIC frame-type tags.
///
/// Wire values exceed `u32` for some extension frame types, hence
/// the `#[repr(u64)]`.  `StreamRangeMin`/`StreamRangeMax` mark the
/// inclusive bounds of the eight-variant STREAM frame block
/// (0x08-0x0f); the bit-flag-encoded variants in between aren't
/// individually named.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u64)]
pub enum FrameType {
    Padding = 0,
    Ping = 1,
    Ack = 0x02,
    AckEcn = 0x03,
    ResetStream = 0x04,
    StopSending = 0x05,
    CryptoHs = 0x06,
    NewToken = 0x07,
    StreamRangeMin = 0x08,
    StreamRangeMax = 0x0f,
    MaxData = 0x10,
    MaxStreamData = 0x11,
    MaxStreamsBidir = 0x12,
    MaxStreamsUnidir = 0x13,
    DataBlocked = 0x14,
    StreamDataBlocked = 0x15,
    StreamsBlockedBidir = 0x16,
    StreamsBlockedUnidir = 0x17,
    NewConnectionId = 0x18,
    RetireConnectionId = 0x19,
    PathChallenge = 0x1a,
    PathResponse = 0x1b,
    ConnectionClose = 0x1c,
    ApplicationClose = 0x1d,
    HandshakeDone = 0x1e,
    ImmediateAck = 0x1F,
    ResetStreamAt = 0x24,
    Datagram = 0x30,
    DatagramL = 0x31,
    PathAck = 0x3e,
    PathAckEcn = 0x3f,
    AckFrequency = 0xAF,
    TimeStamp = 757,
    PathAbandon = 0x3e75,
    PathBackup = 0x3e76,
    PathAvailable = 0x3e77,
    PathNewConnectionId = 0x3e78,
    PathRetireConnectionId = 0x3e79,
    MaxPathId = 0x3e7a,
    PathsBlocked = 0x3e7b,
    PathCidBlocked = 0x3e7c,
    Bdp = 0xebd9,
    ObservedAddressV4 = 0x9f81a6,
    ObservedAddressV6 = 0x9f81a7,
}

impl FrameType {
    /// Textual name for `frame_type`, taking a raw `u64` so callers
    /// can pass extension or unknown wire values too.  C:
    /// `frame_name`.
    pub fn name(_frame_type: u64) -> Option<&'static str> {
        todo!()
    }
}

#[cfg(test)]
mod test {}
