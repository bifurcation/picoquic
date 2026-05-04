# Cross-module consistency report

Generated: 2026-05-03T21:27:43
Files scanned: 23
Total lines: 13321

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
| `SpinBitPolicy` | `rs/fq/src/internal.rs` | 403 |
| `AutoQlog` | `rs/fq/src/internal.rs` | 797 |
| `PerformanceLog` | `rs/fq/src/internal.rs` | 804 |
| `MemLogHook` | `rs/fq/src/internal.rs` | 814 |
| `MaskOps` | `rs/fq/src/internal.rs` | 3878 |
| `StreamDataCb` | `rs/fq/src/lib.rs` | 658 |
| `AlpnSelect` | `rs/fq/src/lib.rs` | 674 |
| `ConnectionIdCb` | `rs/fq/src/lib.rs` | 681 |
| `Fuzz` | `rs/fq/src/lib.rs` | 693 |
| `StreamDirectReceive` | `rs/fq/src/lib.rs` | 707 |
| `CongestionControl` | `rs/fq/src/lib.rs` | 844 |
| `Logger` | `rs/fq/src/logger.rs` | 75 |
| `PacketLoopCbFn` | `rs/fq/src/packet_loop.rs` | 298 |
| `CustomThreadCreateFn` | `rs/fq/src/packet_loop.rs` | 369 |
| `CustomThreadSetnameFn` | `rs/fq/src/packet_loop.rs` | 388 |
| `CustomThreadDeleteFn` | `rs/fq/src/packet_loop.rs` | 396 |
| `TestAqm` | `rs/fq/src/tests/util.rs` | 111 |
| `ThreadFn` | `rs/fq/src/utils.rs` | 654 |

## Module-level lint allowances

Lints suppressed at module scope across 23 files:

| Lint | Modules using it | Modules NOT using it |
|---|---|---|
| `clippy::too_many_arguments` | 1: lib.rs | 22: arena.rs, binlog.rs, bytestream.rs, cc_common.rs, config.rs, crypto_provider_api.rs, hash.rs, internal.rs… |

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

### All `pub struct` / `pub enum` / `pub type` declarations (132 total)

Single definitions are shown collapsed by source file.
Use this to see at a glance which module owns each type.

| Type | Kind | Source |
|---|---|---|
| `AckContext` | `struct` | `rs/fq/src/internal.rs:1243` |
| `AckContextTrack` | `struct` | `rs/fq/src/internal.rs:1232` |
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
| `Connection` | `struct` | `rs/fq/src/internal.rs:1501` |
| `ConnectionId` | `struct` | `rs/fq/src/lib.rs:444` |
| `ConnectionIdContext` | `struct` | `rs/fq/src/lb.rs:139` |
| `ConnectionIdMethod` | `enum` | `rs/fq/src/lb.rs:55` |
| `ConnectionToken` | `type` | `rs/fq/src/internal.rs:72` |
| `CryptoContext` | `struct` | `rs/fq/src/internal.rs:1488` |
| `DatagramActive` | `enum` | `rs/fq/src/lib.rs:760` |
| `DecryptedRetryToken` | `struct` | `rs/fq/src/tls_api.rs:751` |
| `Dualq` | `struct` | `rs/fq/src/tests/dualq.rs:97` |
| `DualqQueue` | `struct` | `rs/fq/src/tests/dualq.rs:63` |
| `EcnCodepoint` | `enum` | `rs/fq/src/socks.rs:355` |
| `Epoch` | `enum` | `rs/fq/src/internal.rs:333` |
| `Error` | `enum` | `rs/fq/src/lib.rs:98` |
| `Event` | `struct` | `rs/fq/src/utils.rs:647` |
| `FrameType` | `enum` | `rs/fq/src/internal.rs:220` |
| `HashTable` | `struct` | `rs/fq/src/hash.rs:104` |
| `HashToken` | `struct` | `rs/fq/src/hash.rs:94` |
| `InitialAeadContext` | `struct` | `rs/fq/src/tls_api.rs:563` |
| `IssuedTicket` | `struct` | `rs/fq/src/internal.rs:759` |
| `IssuedTicketToken` | `type` | `rs/fq/src/internal.rs:78` |
| `JitterMode` | `enum` | `rs/fq/src/tests/util.rs:134` |
| `LocalCnxid` | `struct` | `rs/fq/src/internal.rs:1258` |
| `LocalCnxidList` | `struct` | `rs/fq/src/internal.rs:1269` |
| `LocalCnxidToken` | `type` | `rs/fq/src/internal.rs:86` |
| `LogEventType` | `enum` | `rs/fq/src/binlog.rs:120` |
| `LoopEvent` | `enum` | `rs/fq/src/packet_loop.rs:229` |
| `LoopOptions` | `struct` | `rs/fq/src/packet_loop.rs:311` |
| `LoopParam` | `struct` | `rs/fq/src/packet_loop.rs:328` |
| `LossbitVersion` | `enum` | `rs/fq/src/lib.rs:408` |
| `MessageHeader` | `struct` | `rs/fq/src/socks.rs:94` |
| `MinMaxRtt` | `struct` | `rs/fq/src/cc_common.rs:60` |
| `MiscFrameHeader` | `struct` | `rs/fq/src/internal.rs:1193` |
| `Mutex` | `struct` | `rs/fq/src/utils.rs:639` |
| `NetworkThreadCtx` | `struct` | `rs/fq/src/packet_loop.rs:425` |
| `NewRenoAlgState` | `enum` | `rs/fq/src/cc_common.rs:233` |
| `NewRenoSimState` | `struct` | `rs/fq/src/cc_common.rs:242` |
| `ObservedAddress` | `struct` | `rs/fq/src/internal.rs:3728` |
| `OptionId` | `enum` | `rs/fq/src/config.rs:61` |
| `OsError` | `struct` | `rs/fq/src/socks.rs:104` |
| `Pacing` | `struct` | `rs/fq/src/internal.rs:1306` |
| `Packet` | `struct` | `rs/fq/src/internal.rs:492` |
| `PacketContext` | `enum` | `rs/fq/src/lib.rs:364` |
| `PacketContextState` | `struct` | `rs/fq/src/internal.rs:1205` |
| `PacketData` | `struct` | `rs/fq/src/internal.rs:1825` |
| `PacketDataPathAck` | `struct` | `rs/fq/src/internal.rs:1811` |
| `PacketHeader` | `struct` | `rs/fq/src/internal.rs:360` |
| `PacketToken` | `type` | `rs/fq/src/internal.rs:84` |
| `PacketType` | `enum` | `rs/fq/src/internal.rs:342` |
| `Path` | `struct` | `rs/fq/src/internal.rs:1343` |
| `PathQuality` | `struct` | `rs/fq/src/lib.rs:724` |
| `PathStatus` | `enum` | `rs/fq/src/lib.rs:422` |
| `PathToken` | `type` | `rs/fq/src/internal.rs:87` |
| `PerAckState` | `struct` | `rs/fq/src/lib.rs:818` |
| `PerflogColumn` | `enum` | `rs/fq/src/performance_log.rs:40` |
| `PmtuDiscoveryStatus` | `enum` | `rs/fq/src/internal.rs:272` |
| `PmtudPolicy` | `enum` | `rs/fq/src/lib.rs:377` |
| `PreparedCnxPacket` | `struct` | `rs/fq/src/lib.rs:1967` |
| `PreparedPacket` | `struct` | `rs/fq/src/lib.rs:1923` |
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
| `Quic` | `struct` | `rs/fq/src/internal.rs:829` |
| `RecvInfo` | `struct` | `rs/fq/src/socks.rs:188` |
| `RegisteredToken` | `struct` | `rs/fq/src/internal.rs:557` |
| `RegisteredTokenToken` | `type` | `rs/fq/src/internal.rs:75` |
| `RemoteCnxid` | `struct` | `rs/fq/src/internal.rs:1283` |
| `RemoteCnxidStash` | `struct` | `rs/fq/src/internal.rs:1294` |
| `Result` | `type` | `rs/fq/src/lib.rs:134` |
| `RotationBits` | `enum` | `rs/fq/src/lb.rs:73` |
| `SackItem` | `struct` | `rs/fq/src/internal.rs:1045` |
| `SackItemToken` | `type` | `rs/fq/src/internal.rs:85` |
| `SackList` | `struct` | `rs/fq/src/internal.rs:1059` |
| `SackRangeCount` | `struct` | `rs/fq/src/internal.rs:1055` |
| `SelectInfo` | `struct` | `rs/fq/src/socks.rs:223` |
| `ServerAddress` | `struct` | `rs/fq/src/socks.rs:310` |
| `ServerSockets` | `struct` | `rs/fq/src/socks.rs:82` |
| `Socket` | `struct` | `rs/fq/src/socks.rs:74` |
| `SocketCtx` | `struct` | `rs/fq/src/packet_loop.rs:132` |
| `SpinbitDef` | `struct` | `rs/fq/src/internal.rs:413` |
| `SpinbitVersion` | `enum` | `rs/fq/src/lib.rs:392` |
| `SplayToken` | `struct` | `rs/fq/src/splay.rs:102` |
| `SplayTree` | `struct` | `rs/fq/src/splay.rs:113` |
| `StashResult` | `struct` | `rs/fq/src/internal.rs:2108` |
| `State` | `enum` | `rs/fq/src/lib.rs:288` |
| `StatelessPacket` | `struct` | `rs/fq/src/internal.rs:427` |
| `StoredTicket` | `struct` | `rs/fq/src/internal.rs:584` |
| `StoredToken` | `struct` | `rs/fq/src/internal.rs:699` |
| `StreamDataBufferArgument` | `struct` | `rs/fq/src/internal.rs:3234` |
| `StreamDataNode` | `struct` | `rs/fq/src/internal.rs:468` |
| `StreamDataToken` | `type` | `rs/fq/src/internal.rs:83` |
| `StreamHead` | `struct` | `rs/fq/src/internal.rs:1074` |
| `StreamQueueNode` | `struct` | `rs/fq/src/internal.rs:482` |
| `StreamToken` | `type` | `rs/fq/src/internal.rs:82` |
| `SystemCallDuration` | `struct` | `rs/fq/src/packet_loop.rs:263` |
| `TestSimLink` | `struct` | `rs/fq/src/tests/util.rs:160` |
| `TestSimPacket` | `struct` | `rs/fq/src/tests/util.rs:85` |
| `Thread` | `struct` | `rs/fq/src/utils.rs:634` |
| `TimeCheckArg` | `struct` | `rs/fq/src/packet_loop.rs:281` |
| `TlsCtx` | `struct` | `rs/fq/src/crypto_provider_api.rs:555` |
| `Token` | `struct` | `rs/fq/src/arena.rs:29` |
| `Tp` | `enum` | `rs/fq/src/lib.rs:318` |
| `Tp0rttKind` | `enum` | `rs/fq/src/internal.rs:571` |
| `TpPreferredAddress` | `struct` | `rs/fq/src/lib.rs:520` |
| `TpVersionNegotiation` | `struct` | `rs/fq/src/lib.rs:536` |
| `TransportParameters` | `struct` | `rs/fq/src/lib.rs:557` |
| `Tuple` | `struct` | `rs/fq/src/internal.rs:1318` |
| `VerifiedRetryToken` | `struct` | `rs/fq/src/tls_api.rs:793` |
| `VersionParameters` | `struct` | `rs/fq/src/internal.rs:307` |

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
| `MAX_PACKET_SIZE` | `*` | 1: util.rs |
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
| `tests::util` | `TestAqm`, `TestSimLink`, `TestSimPacket` | 1: dualq.rs |
| `tls_api::Aes128EcbContext` | `*` | 1: lb.rs |
| `utils` | `Thread`, `ThreadFn` | 1: packet_loop.rs |

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
| `rs/fq/src/internal.rs` | 3905 | 5 | 34 | 5 | 9 | 270 | 0 |
| `rs/fq/src/lb.rs` | 215 | 0 | 2 | 2 | 0 | 5 | 0 |
| `rs/fq/src/lib.rs` | 2505 | 6 | 10 | 12 | 1 | 206 | 1 |
| `rs/fq/src/logger.rs` | 432 | 1 | 0 | 0 | 0 | 16 | 0 |
| `rs/fq/src/packet_loop.rs` | 717 | 4 | 6 | 1 | 0 | 14 | 0 |
| `rs/fq/src/performance_log.rs` | 109 | 0 | 0 | 1 | 0 | 2 | 0 |
| `rs/fq/src/qlog.rs` | 31 | 0 | 0 | 0 | 0 | 1 | 0 |
| `rs/fq/src/siphash.rs` | 27 | 0 | 0 | 0 | 0 | 1 | 0 |
| `rs/fq/src/socks.rs` | 391 | 0 | 8 | 1 | 0 | 18 | 0 |
| `rs/fq/src/splay.rs` | 226 | 0 | 2 | 0 | 0 | 15 | 0 |
| `rs/fq/src/tests/dualq.rs` | 234 | 0 | 2 | 0 | 0 | 3 | 0 |
| `rs/fq/src/tests/mod.rs` | 10 | 0 | 0 | 0 | 0 | 0 | 0 |
| `rs/fq/src/tests/util.rs` | 306 | 1 | 2 | 1 | 0 | 16 | 0 |
| `rs/fq/src/textlog.rs` | 67 | 0 | 0 | 0 | 0 | 2 | 0 |
| `rs/fq/src/tls_api.rs` | 1060 | 0 | 4 | 0 | 0 | 54 | 0 |
| `rs/fq/src/utils.rs` | 754 | 1 | 3 | 0 | 0 | 70 | 0 |

