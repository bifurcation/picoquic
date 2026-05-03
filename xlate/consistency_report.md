# Cross-module consistency report

Generated: 2026-05-03T14:25:17
Files scanned: 22
Total lines: 13272

**How to use this report.**  Read each section.  Where you
see inconsistency that should be reconciled, decide a
policy, then sprinkle `// REVIEW: <instruction>` markers in
the offending files.  Run `scripts/phase1c.py` to apply.

## Trait names by case style

Total traits: 34.  PascalCase: 34

Rust convention says traits are `PascalCase`.  Anything
else is a refactor candidate.

### PascalCase (34)

| Trait | File | Line |
|---|---|---|
| `SetTlsKeyProvider` | `rs/fq/src/crypto_provider_api.rs` | 211 |
| `GetPrivateKeyFromFile` | `rs/fq/src/crypto_provider_api.rs` | 219 |
| `SetPrivateKeyFromFile` | `rs/fq/src/crypto_provider_api.rs` | 225 |
| `GetPublicKeyFromPrivate` | `rs/fq/src/crypto_provider_api.rs` | 233 |
| `DisposeSignCertificate` | `rs/fq/src/crypto_provider_api.rs` | 240 |
| `GetCertsFromFile` | `rs/fq/src/crypto_provider_api.rs` | 248 |
| `VerifyCertificate` | `rs/fq/src/crypto_provider_api.rs` | 257 |
| `VerifySignature` | `rs/fq/src/crypto_provider_api.rs` | 272 |
| `GetCertificateVerifier` | `rs/fq/src/crypto_provider_api.rs` | 298 |
| `SetTlsRootCertificates` | `rs/fq/src/crypto_provider_api.rs` | 305 |
| `ExplainCryptoError` | `rs/fq/src/crypto_provider_api.rs` | 313 |
| `ClearCryptoErrors` | `rs/fq/src/crypto_provider_api.rs` | 319 |
| `SetRandomProviderInCtx` | `rs/fq/src/crypto_provider_api.rs` | 326 |
| `CryptoRandomProvider` | `rs/fq/src/crypto_provider_api.rs` | 333 |
| `KeyexFromKeyFile` | `rs/fq/src/crypto_provider_api.rs` | 341 |
| `KeyexDispose` | `rs/fq/src/crypto_provider_api.rs` | 347 |
| `SpinBitPolicy` | `rs/fq/src/internal.rs` | 401 |
| `AutoQlog` | `rs/fq/src/internal.rs` | 814 |
| `PerformanceLog` | `rs/fq/src/internal.rs` | 821 |
| `MemLogHook` | `rs/fq/src/internal.rs` | 831 |
| `MaskOps` | `rs/fq/src/internal.rs` | 3818 |
| `StreamDataCb` | `rs/fq/src/lib.rs` | 658 |
| `AlpnSelect` | `rs/fq/src/lib.rs` | 674 |
| `ConnectionIdCb` | `rs/fq/src/lib.rs` | 681 |
| `Fuzz` | `rs/fq/src/lib.rs` | 693 |
| `StreamDirectReceive` | `rs/fq/src/lib.rs` | 707 |
| `CongestionControl` | `rs/fq/src/lib.rs` | 844 |
| `Logger` | `rs/fq/src/logger.rs` | 75 |
| `PacketLoopCbFn` | `rs/fq/src/packet_loop.rs` | 312 |
| `CustomThreadCreateFn` | `rs/fq/src/packet_loop.rs` | 391 |
| `CustomThreadSetnameFn` | `rs/fq/src/packet_loop.rs` | 410 |
| `CustomThreadDeleteFn` | `rs/fq/src/packet_loop.rs` | 418 |
| `ThreadFn` | `rs/fq/src/utils.rs` | 654 |
| `TestAqm` | `rs/fq/src/utils.rs` | 825 |

## Module-level lint allowances

Lints suppressed at module scope across 22 files:

| Lint | Modules using it | Modules NOT using it |
|---|---|---|
| `clippy::too_many_arguments` | 1: lib.rs | 21: arena.rs, binlog.rs, bytestream.rs, cc_common.rs, config.rs, crypto_provider_api.rs, hash.rs, internal.rs… |

Lints used in only some modules are the interesting ones.
Either the lint is appropriate for those modules and not
the others (fine — but worth a line of comment), or the
application is inconsistent.

## Type definitions across modules

**1 type name(s) defined in more than one module.**
Usually this means an opaque stub somewhere should be
replaced by a `use crate::other_module::Type;`
import — Rust resolves the reference fine, but the
duplicate `pub struct X { _private: () }` is dead weight.

| Type | Definitions |
|---|---|
| `Config` | `struct` in `rs/fq/src/config.rs:136`<br>`struct` in `rs/fq/src/lb.rs:92` |

### All `pub struct` / `pub enum` / `pub type` declarations (130 total)

Single definitions are shown collapsed by source file.
Use this to see at a glance which module owns each type.

| Type | Kind | Source |
|---|---|---|
| `AckContext` | `struct` | `rs/fq/src/internal.rs:1236` |
| `AckContextTrack` | `struct` | `rs/fq/src/internal.rs:1225` |
| `Aes128EcbContext` | `struct` | `rs/fq/src/tls_api.rs:998` |
| `Alpn` | `enum` | `rs/fq/src/lib.rs:912` |
| `AlpnEntry` | `struct` | `rs/fq/src/lib.rs:926` |
| `Arena` | `struct` | `rs/fq/src/arena.rs:48` |
| `ByteStream` | `struct` | `rs/fq/src/bytestream.rs:77` |
| `ByteStreamBuf` | `struct` | `rs/fq/src/bytestream.rs:98` |
| `ByteStreamData` | `enum` | `rs/fq/src/bytestream.rs:55` |
| `CallbackEvent` | `enum` | `rs/fq/src/lib.rs:481` |
| `CertificateVerifier` | `struct` | `rs/fq/src/crypto_provider_api.rs:283` |
| `CipherSuiteEntry` | `struct` | `rs/fq/src/crypto_provider_api.rs:429` |
| `CmsgInfo` | `struct` | `rs/fq/src/socks.rs:335` |
| `Config` | `struct` | **2 definitions — see above** |
| `CongestionAlgorithm` | `struct` | `rs/fq/src/lib.rs:873` |
| `CongestionNotification` | `enum` | `rs/fq/src/lib.rs:787` |
| `Connection` | `struct` | `rs/fq/src/internal.rs:1488` |
| `ConnectionId` | `struct` | `rs/fq/src/lib.rs:444` |
| `ConnectionIdContext` | `struct` | `rs/fq/src/lb.rs:139` |
| `ConnectionIdMethod` | `enum` | `rs/fq/src/lb.rs:55` |
| `ConnectionToken` | `type` | `rs/fq/src/internal.rs:70` |
| `CryptoContext` | `struct` | `rs/fq/src/internal.rs:1475` |
| `DatagramActive` | `enum` | `rs/fq/src/lib.rs:760` |
| `DecryptedRetryToken` | `struct` | `rs/fq/src/tls_api.rs:751` |
| `Dualq` | `struct` | `rs/fq/src/tests/dualq.rs:106` |
| `DualqQueue` | `struct` | `rs/fq/src/tests/dualq.rs:63` |
| `EcnCodepoint` | `enum` | `rs/fq/src/socks.rs:355` |
| `Epoch` | `enum` | `rs/fq/src/internal.rs:331` |
| `Error` | `enum` | `rs/fq/src/lib.rs:98` |
| `Event` | `struct` | `rs/fq/src/utils.rs:647` |
| `FrameType` | `enum` | `rs/fq/src/internal.rs:218` |
| `HashTable` | `struct` | `rs/fq/src/hash.rs:104` |
| `HashToken` | `struct` | `rs/fq/src/hash.rs:94` |
| `InitialAeadContext` | `struct` | `rs/fq/src/tls_api.rs:563` |
| `IssuedTicket` | `struct` | `rs/fq/src/internal.rs:775` |
| `IssuedTicketToken` | `type` | `rs/fq/src/internal.rs:76` |
| `JitterMode` | `enum` | `rs/fq/src/utils.rs:848` |
| `LocalCnxid` | `struct` | `rs/fq/src/internal.rs:1251` |
| `LocalCnxidList` | `struct` | `rs/fq/src/internal.rs:1262` |
| `LocalCnxidToken` | `type` | `rs/fq/src/internal.rs:84` |
| `LogEventType` | `enum` | `rs/fq/src/binlog.rs:120` |
| `LoopEvent` | `enum` | `rs/fq/src/packet_loop.rs:232` |
| `LoopOptions` | `struct` | `rs/fq/src/packet_loop.rs:333` |
| `LoopParam` | `struct` | `rs/fq/src/packet_loop.rs:350` |
| `LossbitVersion` | `enum` | `rs/fq/src/lib.rs:408` |
| `MessageHeader` | `struct` | `rs/fq/src/socks.rs:94` |
| `MinMaxRtt` | `struct` | `rs/fq/src/cc_common.rs:60` |
| `MiscFrameHeader` | `struct` | `rs/fq/src/internal.rs:1186` |
| `Mutex` | `struct` | `rs/fq/src/utils.rs:639` |
| `NetworkThreadCtx` | `struct` | `rs/fq/src/packet_loop.rs:447` |
| `NewRenoAlgState` | `enum` | `rs/fq/src/cc_common.rs:233` |
| `NewRenoSimState` | `struct` | `rs/fq/src/cc_common.rs:242` |
| `OptionId` | `enum` | `rs/fq/src/config.rs:61` |
| `OsError` | `struct` | `rs/fq/src/socks.rs:104` |
| `Pacing` | `struct` | `rs/fq/src/internal.rs:1299` |
| `Packet` | `struct` | `rs/fq/src/internal.rs:491` |
| `PacketContext` | `enum` | `rs/fq/src/lib.rs:364` |
| `PacketContextState` | `struct` | `rs/fq/src/internal.rs:1198` |
| `PacketData` | `struct` | `rs/fq/src/internal.rs:1807` |
| `PacketDataPathAck` | `struct` | `rs/fq/src/internal.rs:1793` |
| `PacketHeader` | `struct` | `rs/fq/src/internal.rs:358` |
| `PacketToken` | `type` | `rs/fq/src/internal.rs:82` |
| `PacketType` | `enum` | `rs/fq/src/internal.rs:340` |
| `Path` | `struct` | `rs/fq/src/internal.rs:1336` |
| `PathQuality` | `struct` | `rs/fq/src/lib.rs:724` |
| `PathStatus` | `enum` | `rs/fq/src/lib.rs:422` |
| `PathToken` | `type` | `rs/fq/src/internal.rs:85` |
| `PerAckState` | `struct` | `rs/fq/src/lib.rs:818` |
| `PerflogColumn` | `enum` | `rs/fq/src/performance_log.rs:40` |
| `PmtuDiscoveryStatus` | `enum` | `rs/fq/src/internal.rs:270` |
| `PmtudPolicy` | `enum` | `rs/fq/src/lib.rs:377` |
| `PreparedCnxPacket` | `struct` | `rs/fq/src/lib.rs:1966` |
| `PreparedPacket` | `struct` | `rs/fq/src/lib.rs:1922` |
| `Ptls` | `struct` | `rs/fq/src/crypto_provider_api.rs:132` |
| `PtlsCipherSuite` | `struct` | `rs/fq/src/crypto_provider_api.rs:94` |
| `PtlsContext` | `struct` | `rs/fq/src/crypto_provider_api.rs:119` |
| `PtlsHandshakeProperties` | `struct` | `rs/fq/src/crypto_provider_api.rs:146` |
| `PtlsHpkeCipherSuite` | `struct` | `rs/fq/src/crypto_provider_api.rs:106` |
| `PtlsHpkeKem` | `struct` | `rs/fq/src/crypto_provider_api.rs:112` |
| `PtlsKeyExchangeAlgorithm` | `struct` | `rs/fq/src/crypto_provider_api.rs:100` |
| `PtlsKeyExchangeContext` | `struct` | `rs/fq/src/crypto_provider_api.rs:152` |
| `PtlsRawExtension` | `struct` | `rs/fq/src/crypto_provider_api.rs:140` |
| `PtlsSignCertificate` | `struct` | `rs/fq/src/crypto_provider_api.rs:125` |
| `Quic` | `struct` | `rs/fq/src/internal.rs:840` |
| `RecvInfo` | `struct` | `rs/fq/src/socks.rs:188` |
| `RegisteredToken` | `struct` | `rs/fq/src/internal.rs:560` |
| `RegisteredTokenToken` | `type` | `rs/fq/src/internal.rs:73` |
| `RemoteCnxid` | `struct` | `rs/fq/src/internal.rs:1276` |
| `RemoteCnxidStash` | `struct` | `rs/fq/src/internal.rs:1287` |
| `Result` | `type` | `rs/fq/src/lib.rs:134` |
| `RotationBits` | `enum` | `rs/fq/src/lb.rs:73` |
| `SackItem` | `struct` | `rs/fq/src/internal.rs:1040` |
| `SackItemToken` | `type` | `rs/fq/src/internal.rs:83` |
| `SackList` | `struct` | `rs/fq/src/internal.rs:1054` |
| `SackRangeCount` | `struct` | `rs/fq/src/internal.rs:1050` |
| `SelectInfo` | `struct` | `rs/fq/src/socks.rs:223` |
| `ServerAddress` | `struct` | `rs/fq/src/socks.rs:310` |
| `ServerSockets` | `struct` | `rs/fq/src/socks.rs:82` |
| `Socket` | `struct` | `rs/fq/src/socks.rs:74` |
| `SocketCtx` | `struct` | `rs/fq/src/packet_loop.rs:133` |
| `SpinbitDef` | `struct` | `rs/fq/src/internal.rs:411` |
| `SpinbitVersion` | `enum` | `rs/fq/src/lib.rs:392` |
| `SplayToken` | `struct` | `rs/fq/src/splay.rs:102` |
| `SplayTree` | `struct` | `rs/fq/src/splay.rs:113` |
| `State` | `enum` | `rs/fq/src/lib.rs:288` |
| `StatelessPacket` | `struct` | `rs/fq/src/internal.rs:425` |
| `StoredTicket` | `struct` | `rs/fq/src/internal.rs:587` |
| `StoredToken` | `struct` | `rs/fq/src/internal.rs:712` |
| `StreamDataBufferArgument` | `struct` | `rs/fq/src/internal.rs:3161` |
| `StreamDataNode` | `struct` | `rs/fq/src/internal.rs:467` |
| `StreamDataToken` | `type` | `rs/fq/src/internal.rs:81` |
| `StreamHead` | `struct` | `rs/fq/src/internal.rs:1069` |
| `StreamQueueNode` | `struct` | `rs/fq/src/internal.rs:481` |
| `StreamToken` | `type` | `rs/fq/src/internal.rs:80` |
| `SystemCallDuration` | `struct` | `rs/fq/src/packet_loop.rs:271` |
| `TestSimLink` | `struct` | `rs/fq/src/utils.rs:874` |
| `TestSimPacket` | `struct` | `rs/fq/src/utils.rs:797` |
| `Thread` | `struct` | `rs/fq/src/utils.rs:634` |
| `TimeCheckArg` | `struct` | `rs/fq/src/packet_loop.rs:289` |
| `TlsCtx` | `struct` | `rs/fq/src/crypto_provider_api.rs:555` |
| `Token` | `struct` | `rs/fq/src/arena.rs:29` |
| `Tp` | `enum` | `rs/fq/src/lib.rs:318` |
| `Tp0rttKind` | `enum` | `rs/fq/src/internal.rs:574` |
| `TpPreferredAddress` | `struct` | `rs/fq/src/lib.rs:520` |
| `TpVersionNegotiation` | `struct` | `rs/fq/src/lib.rs:536` |
| `TransportParameters` | `struct` | `rs/fq/src/lib.rs:557` |
| `Tuple` | `struct` | `rs/fq/src/internal.rs:1311` |
| `VerifiedRetryToken` | `struct` | `rs/fq/src/tls_api.rs:793` |
| `VersionParameters` | `struct` | `rs/fq/src/internal.rs:305` |

## Cross-module imports

Items pulled in via `use crate::…`, grouped by
source module.  A type imported by many modules but defined
in one place is the healthy pattern; a type imported via
two different source paths is a smell.

| Source module | Items imported | Importers |
|---|---|---|
| `Connection` | `*` | 1: crypto_provider_api.rs |
| `ConnectionId` | `*` | 1: bytestream.rs |
| `Error` | `*` | 14: arena.rs, binlog.rs, bytestream.rs, config.rs, crypto_provider_api.rs, … (9 more) |
| `Quic` | `*` | 2: performance_log.rs, textlog.rs |
| `arena` | `Arena`, `Token` | 1: internal.rs |
| `config::Config` | `*` | 1: packet_loop.rs |
| `crypto_provider_api::PtlsCipherSuite` | `*` | 1: tls_api.rs |
| `crypto_provider_api::VerifyCertificate` | `*` | 2: internal.rs, tls_api.rs |
| `hash` | `HashTable`, `HashToken` | 1: internal.rs |
| `internal` | `Connection`, `PacketHeader`, `PacketType`, `Path`, `Quic` | 4: binlog.rs, cc_common.rs, lib.rs, logger.rs |
| `internal::CryptoContext` | `*` | 1: tls_api.rs |
| `logger::Logger` | `*` | 1: internal.rs |
| `socks::OsError` | `*` | 1: packet_loop.rs |
| `splay` | `SplayToken`, `SplayTree` | 1: internal.rs |
| `tls_api::Aes128EcbContext` | `*` | 1: lb.rs |
| `utils` | `TestAqm`, `TestSimLink`, `TestSimPacket`, `Thread`, `ThreadFn` | 2: packet_loop.rs, dualq.rs |

## Per-file summary

| File | LOC | Traits | Structs | Enums | Type aliases | Fns | Inner #![allow] |
|---|---:|---:|---:|---:|---:|---:|---:|
| `rs/fq/src/arena.rs` | 128 | 0 | 2 | 0 | 0 | 8 | 0 |
| `rs/fq/src/binlog.rs` | 378 | 0 | 0 | 1 | 0 | 14 | 0 |
| `rs/fq/src/bytestream.rs` | 376 | 0 | 2 | 1 | 0 | 38 | 0 |
| `rs/fq/src/cc_common.rs` | 277 | 0 | 2 | 1 | 0 | 15 | 0 |
| `rs/fq/src/config.rs` | 354 | 0 | 1 | 1 | 0 | 7 | 0 |
| `rs/fq/src/crypto_provider_api.rs` | 596 | 16 | 13 | 0 | 0 | 25 | 0 |
| `rs/fq/src/hash.rs` | 223 | 0 | 2 | 0 | 0 | 14 | 0 |
| `rs/fq/src/internal.rs` | 3845 | 5 | 32 | 5 | 9 | 274 | 0 |
| `rs/fq/src/lb.rs` | 215 | 0 | 2 | 2 | 0 | 5 | 0 |
| `rs/fq/src/lib.rs` | 2504 | 6 | 10 | 12 | 1 | 206 | 1 |
| `rs/fq/src/logger.rs` | 432 | 1 | 0 | 0 | 0 | 16 | 0 |
| `rs/fq/src/packet_loop.rs` | 751 | 4 | 6 | 1 | 0 | 14 | 0 |
| `rs/fq/src/performance_log.rs` | 109 | 0 | 0 | 1 | 0 | 2 | 0 |
| `rs/fq/src/qlog.rs` | 31 | 0 | 0 | 0 | 0 | 1 | 0 |
| `rs/fq/src/siphash.rs` | 27 | 0 | 0 | 0 | 0 | 1 | 0 |
| `rs/fq/src/socks.rs` | 391 | 0 | 8 | 1 | 0 | 18 | 0 |
| `rs/fq/src/splay.rs` | 226 | 0 | 2 | 0 | 0 | 15 | 0 |
| `rs/fq/src/tests/dualq.rs` | 243 | 0 | 2 | 0 | 0 | 3 | 0 |
| `rs/fq/src/tests/mod.rs` | 14 | 0 | 0 | 0 | 0 | 0 | 0 |
| `rs/fq/src/textlog.rs` | 67 | 0 | 0 | 0 | 0 | 2 | 0 |
| `rs/fq/src/tls_api.rs` | 1060 | 0 | 4 | 0 | 0 | 54 | 0 |
| `rs/fq/src/utils.rs` | 1025 | 2 | 5 | 1 | 0 | 86 | 0 |

