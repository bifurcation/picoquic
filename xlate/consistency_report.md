# Cross-module consistency report

Generated: 2026-05-02T19:26:34
Files scanned: 20
Total lines: 13100

**How to use this report.**  Read each section.  Where you
see inconsistency that should be reconciled, decide a
policy, then sprinkle `// REVIEW: <instruction>` markers in
the offending files.  Run `scripts/phase1c.py` to apply.

## Trait names by case style

Total traits: 39.  PascalCase: 39

Rust convention says traits are `PascalCase`.  Anything
else is a refactor candidate.

### PascalCase (39)

| Trait | File | Line |
|---|---|---|
| `SetTlsKeyProvider` | `rs/fq/src/crypto_provider_api.rs` | 211 |
| `GetPrivateKeyFromFile` | `rs/fq/src/crypto_provider_api.rs` | 219 |
| `SetPrivateKeyFromFile` | `rs/fq/src/crypto_provider_api.rs` | 225 |
| `GetPublicKeyFromPrivate` | `rs/fq/src/crypto_provider_api.rs` | 233 |
| `DisposeSignCertificate` | `rs/fq/src/crypto_provider_api.rs` | 240 |
| `GetCertsFromFile` | `rs/fq/src/crypto_provider_api.rs` | 248 |
| `DisposeCertificateVerifier` | `rs/fq/src/crypto_provider_api.rs` | 254 |
| `GetCertificateVerifier` | `rs/fq/src/crypto_provider_api.rs` | 280 |
| `SetTlsRootCertificates` | `rs/fq/src/crypto_provider_api.rs` | 287 |
| `ExplainCryptoError` | `rs/fq/src/crypto_provider_api.rs` | 295 |
| `ClearCryptoErrors` | `rs/fq/src/crypto_provider_api.rs` | 301 |
| `SetRandomProviderInCtx` | `rs/fq/src/crypto_provider_api.rs` | 308 |
| `CryptoRandomProvider` | `rs/fq/src/crypto_provider_api.rs` | 315 |
| `KeyexFromKeyFile` | `rs/fq/src/crypto_provider_api.rs` | 323 |
| `KeyexDispose` | `rs/fq/src/crypto_provider_api.rs` | 329 |
| `HashOps` | `rs/fq/src/hash.rs` | 53 |
| `SpinBitPolicy` | `rs/fq/src/internal.rs` | 369 |
| `AutoQlog` | `rs/fq/src/internal.rs` | 776 |
| `PerformanceLog` | `rs/fq/src/internal.rs` | 783 |
| `MemLogHook` | `rs/fq/src/internal.rs` | 793 |
| `MaskOps` | `rs/fq/src/internal.rs` | 3644 |
| `StreamDataCb` | `rs/fq/src/lib.rs` | 683 |
| `AlpnSelect` | `rs/fq/src/lib.rs` | 697 |
| `AlpnSelectV2` | `rs/fq/src/lib.rs` | 703 |
| `ConnectionIdCb` | `rs/fq/src/lib.rs` | 710 |
| `Fuzz` | `rs/fq/src/lib.rs` | 722 |
| `VerifySignCb` | `rs/fq/src/lib.rs` | 735 |
| `VerifyCertificateCb` | `rs/fq/src/lib.rs` | 743 |
| `FreeVerifyCertificateCtx` | `rs/fq/src/lib.rs` | 756 |
| `StreamDirectReceive` | `rs/fq/src/lib.rs` | 764 |
| `CongestionControl` | `rs/fq/src/lib.rs` | 901 |
| `PacketLoopCbFn` | `rs/fq/src/packet_loop.rs` | 312 |
| `CustomThreadCreateFn` | `rs/fq/src/packet_loop.rs` | 391 |
| `CustomThreadSetnameFn` | `rs/fq/src/packet_loop.rs` | 410 |
| `CustomThreadDeleteFn` | `rs/fq/src/packet_loop.rs` | 418 |
| `SplayOps` | `rs/fq/src/splay.rs` | 52 |
| `UnifiedLogging` | `rs/fq/src/unified_log.rs` | 74 |
| `ThreadFn` | `rs/fq/src/utils.rs` | 679 |
| `TestAqm` | `rs/fq/src/utils.rs` | 850 |

## Module-level lint allowances

Lints suppressed at module scope across 20 files:

| Lint | Modules using it | Modules NOT using it |
|---|---|---|
| `clippy::too_many_arguments` | 1: lib.rs | 19: binlog.rs, bytestream.rs, cc_common.rs, config.rs, crypto_provider_api.rs, hash.rs, internal.rs, lb.rs… |
| `non_camel_case_types` | 1: lib.rs | 19: binlog.rs, bytestream.rs, cc_common.rs, config.rs, crypto_provider_api.rs, hash.rs, internal.rs, lb.rs… |
| `non_upper_case_globals` | 1: lib.rs | 19: binlog.rs, bytestream.rs, cc_common.rs, config.rs, crypto_provider_api.rs, hash.rs, internal.rs, lb.rs… |

Lints used in only some modules are the interesting ones.
Either the lint is appropriate for those modules and not
the others (fine — but worth a line of comment), or the
application is inconsistent.

## Type definitions across modules

No name is defined in more than one module.  Good.

### All `pub struct` / `pub enum` / `pub type` declarations (120 total)

Single definitions are shown collapsed by source file.
Use this to see at a glance which module owns each type.

| Type | Kind | Source |
|---|---|---|
| `AckContext` | `struct` | `rs/fq/src/internal.rs:1134` |
| `AckContextTrack` | `struct` | `rs/fq/src/internal.rs:1123` |
| `Aes128EcbContext` | `struct` | `rs/fq/src/lb.rs:86` |
| `Alpn` | `enum` | `rs/fq/src/lib.rs:969` |
| `AlpnEntry` | `struct` | `rs/fq/src/lib.rs:983` |
| `ByteStream` | `struct` | `rs/fq/src/bytestream.rs:80` |
| `ByteStreamBuf` | `struct` | `rs/fq/src/bytestream.rs:101` |
| `ByteStreamData` | `enum` | `rs/fq/src/bytestream.rs:58` |
| `CallbackEvent` | `enum` | `rs/fq/src/lib.rs:506` |
| `CertificateVerifier` | `struct` | `rs/fq/src/crypto_provider_api.rs:262` |
| `CipherSuiteEntry` | `struct` | `rs/fq/src/crypto_provider_api.rs:410` |
| `CmsgInfo` | `struct` | `rs/fq/src/socks.rs:331` |
| `Cnx` | `struct` | `rs/fq/src/internal.rs:1382` |
| `Config` | `struct` | `rs/fq/src/config.rs:135` |
| `CongestionAlgorithm` | `struct` | `rs/fq/src/lib.rs:930` |
| `CongestionNotification` | `enum` | `rs/fq/src/lib.rs:844` |
| `ConnectionId` | `struct` | `rs/fq/src/lib.rs:444` |
| `CryptoContext` | `struct` | `rs/fq/src/internal.rs:1369` |
| `DatagramActive` | `enum` | `rs/fq/src/lib.rs:817` |
| `DecryptedRetryToken` | `struct` | `rs/fq/src/tls_api.rs:757` |
| `Dualq` | `struct` | `rs/fq/src/test_dualq.rs:103` |
| `DualqQueue` | `struct` | `rs/fq/src/test_dualq.rs:63` |
| `Epoch` | `enum` | `rs/fq/src/internal.rs:304` |
| `Error` | `enum` | `rs/fq/src/lib.rs:101` |
| `Event` | `struct` | `rs/fq/src/utils.rs:672` |
| `File` | `struct` | `rs/fq/src/utils.rs:465` |
| `FrameType` | `enum` | `rs/fq/src/internal.rs:191` |
| `HashItem` | `struct` | `rs/fq/src/hash.rs:82` |
| `HashTable` | `struct` | `rs/fq/src/hash.rs:116` |
| `InitialAeadContext` | `struct` | `rs/fq/src/tls_api.rs:565` |
| `Iovec` | `struct` | `rs/fq/src/lib.rs:478` |
| `IssuedTicket` | `struct` | `rs/fq/src/internal.rs:737` |
| `JitterMode` | `enum` | `rs/fq/src/utils.rs:873` |
| `LbCidContext` | `struct` | `rs/fq/src/lb.rs:139` |
| `LbCidMethod` | `enum` | `rs/fq/src/lb.rs:61` |
| `LbConfig` | `struct` | `rs/fq/src/lb.rs:100` |
| `LocalCnxid` | `struct` | `rs/fq/src/internal.rs:1149` |
| `LocalCnxidList` | `struct` | `rs/fq/src/internal.rs:1160` |
| `LogEventType` | `enum` | `rs/fq/src/binlog.rs:121` |
| `LoopEvent` | `enum` | `rs/fq/src/packet_loop.rs:232` |
| `LoopOptions` | `struct` | `rs/fq/src/packet_loop.rs:333` |
| `LoopParam` | `struct` | `rs/fq/src/packet_loop.rs:350` |
| `LossbitVersion` | `enum` | `rs/fq/src/lib.rs:408` |
| `MinMaxRtt` | `struct` | `rs/fq/src/cc_common.rs:54` |
| `MiscFrameHeader` | `struct` | `rs/fq/src/internal.rs:1093` |
| `Msghdr` | `struct` | `rs/fq/src/socks.rs:87` |
| `Mutex` | `struct` | `rs/fq/src/utils.rs:664` |
| `NetworkThreadCtx` | `struct` | `rs/fq/src/packet_loop.rs:447` |
| `NewRenoAlgState` | `enum` | `rs/fq/src/cc_common.rs:220` |
| `NewRenoSimState` | `struct` | `rs/fq/src/cc_common.rs:229` |
| `OptionId` | `enum` | `rs/fq/src/config.rs:61` |
| `OsError` | `struct` | `rs/fq/src/socks.rs:97` |
| `Pacing` | `struct` | `rs/fq/src/internal.rs:1196` |
| `Packet` | `struct` | `rs/fq/src/internal.rs:457` |
| `PacketContext` | `enum` | `rs/fq/src/lib.rs:364` |
| `PacketContextState` | `struct` | `rs/fq/src/internal.rs:1104` |
| `PacketData` | `struct` | `rs/fq/src/internal.rs:1674` |
| `PacketDataPathAck` | `struct` | `rs/fq/src/internal.rs:1660` |
| `PacketHeader` | `struct` | `rs/fq/src/internal.rs:331` |
| `PacketType` | `enum` | `rs/fq/src/internal.rs:313` |
| `Path` | `struct` | `rs/fq/src/internal.rs:1234` |
| `PathQuality` | `struct` | `rs/fq/src/lib.rs:781` |
| `PathStatus` | `enum` | `rs/fq/src/lib.rs:422` |
| `PerAckState` | `struct` | `rs/fq/src/lib.rs:875` |
| `PerflogColumn` | `enum` | `rs/fq/src/performance_log.rs:44` |
| `PmtuDiscoveryStatus` | `enum` | `rs/fq/src/internal.rs:243` |
| `PmtudPolicy` | `enum` | `rs/fq/src/lib.rs:377` |
| `PreparedCnxPacket` | `struct` | `rs/fq/src/lib.rs:2028` |
| `PreparedPacket` | `struct` | `rs/fq/src/lib.rs:1984` |
| `Ptls` | `struct` | `rs/fq/src/crypto_provider_api.rs:132` |
| `PtlsCipherSuite` | `struct` | `rs/fq/src/crypto_provider_api.rs:94` |
| `PtlsContext` | `struct` | `rs/fq/src/crypto_provider_api.rs:119` |
| `PtlsHandshakeProperties` | `struct` | `rs/fq/src/crypto_provider_api.rs:146` |
| `PtlsHpkeCipherSuite` | `struct` | `rs/fq/src/crypto_provider_api.rs:106` |
| `PtlsHpkeKem` | `struct` | `rs/fq/src/crypto_provider_api.rs:112` |
| `PtlsIovec` | `struct` | `rs/fq/src/lib.rs:463` |
| `PtlsKeyExchangeAlgorithm` | `struct` | `rs/fq/src/crypto_provider_api.rs:100` |
| `PtlsKeyExchangeContext` | `struct` | `rs/fq/src/crypto_provider_api.rs:152` |
| `PtlsRawExtension` | `struct` | `rs/fq/src/crypto_provider_api.rs:140` |
| `PtlsSignCertificate` | `struct` | `rs/fq/src/crypto_provider_api.rs:125` |
| `PtlsVerifyCertificate` | `struct` | `rs/fq/src/lib.rs:728` |
| `Quic` | `struct` | `rs/fq/src/internal.rs:802` |
| `RecvInfo` | `struct` | `rs/fq/src/socks.rs:184` |
| `RegisteredToken` | `struct` | `rs/fq/src/internal.rs:525` |
| `RemoteCnxid` | `struct` | `rs/fq/src/internal.rs:1173` |
| `RemoteCnxidStash` | `struct` | `rs/fq/src/internal.rs:1185` |
| `SackItem` | `struct` | `rs/fq/src/internal.rs:964` |
| `SackList` | `struct` | `rs/fq/src/internal.rs:976` |
| `SackRangeCount` | `struct` | `rs/fq/src/internal.rs:972` |
| `SelectInfo` | `struct` | `rs/fq/src/socks.rs:219` |
| `ServerAddress` | `struct` | `rs/fq/src/socks.rs:306` |
| `ServerSockets` | `struct` | `rs/fq/src/socks.rs:75` |
| `Socket` | `struct` | `rs/fq/src/socks.rs:67` |
| `SocketCtx` | `struct` | `rs/fq/src/packet_loop.rs:133` |
| `SpinbitDef` | `struct` | `rs/fq/src/internal.rs:379` |
| `SpinbitVersion` | `enum` | `rs/fq/src/lib.rs:392` |
| `SplayNode` | `struct` | `rs/fq/src/splay.rs:87` |
| `SplayTree` | `struct` | `rs/fq/src/splay.rs:138` |
| `State` | `enum` | `rs/fq/src/lib.rs:288` |
| `StatelessPacket` | `struct` | `rs/fq/src/internal.rs:393` |
| `StoredTicket` | `struct` | `rs/fq/src/internal.rs:550` |
| `StoredToken` | `struct` | `rs/fq/src/internal.rs:675` |
| `StreamDataBufferArgument` | `struct` | `rs/fq/src/internal.rs:2997` |
| `StreamDataNode` | `struct` | `rs/fq/src/internal.rs:436` |
| `StreamHead` | `struct` | `rs/fq/src/internal.rs:986` |
| `StreamQueueNode` | `struct` | `rs/fq/src/internal.rs:446` |
| `SystemCallDuration` | `struct` | `rs/fq/src/packet_loop.rs:271` |
| `TestSimLink` | `struct` | `rs/fq/src/utils.rs:899` |
| `TestSimPacket` | `struct` | `rs/fq/src/utils.rs:822` |
| `Thread` | `struct` | `rs/fq/src/utils.rs:659` |
| `TimeCheckArg` | `struct` | `rs/fq/src/packet_loop.rs:289` |
| `TlsCtx` | `struct` | `rs/fq/src/crypto_provider_api.rs:545` |
| `Tp` | `enum` | `rs/fq/src/lib.rs:318` |
| `Tp0rttKind` | `enum` | `rs/fq/src/internal.rs:537` |
| `TpPreferredAddress` | `struct` | `rs/fq/src/lib.rs:545` |
| `TpVersionNegotiation` | `struct` | `rs/fq/src/lib.rs:561` |
| `TransportParameters` | `struct` | `rs/fq/src/lib.rs:582` |
| `Tuple` | `struct` | `rs/fq/src/internal.rs:1208` |
| `VerifiedRetryToken` | `struct` | `rs/fq/src/tls_api.rs:799` |
| `VersionParameters` | `struct` | `rs/fq/src/internal.rs:278` |

## Cross-module imports

Items pulled in via `use crate::…`, grouped by
source module.  A type imported by many modules but defined
in one place is the healthy pattern; a type imported via
two different source paths is a smell.

| Source module | Items imported | Importers |
|---|---|---|
| `ConnectionId` | `*` | 1: bytestream.rs |
| `Error` | `*` | 12: binlog.rs, bytestream.rs, config.rs, crypto_provider_api.rs, hash.rs, … (7 more) |
| `Quic` | `*` | 1: performance_log.rs |
| `config::Config` | `*` | 1: packet_loop.rs |
| `crypto_provider_api::PtlsCipherSuite` | `*` | 1: tls_api.rs |
| `hash` | `HashItem`, `HashTable` | 1: internal.rs |
| `internal` | `Cnx`, `PacketHeader`, `PacketType`, `Path`, `Quic` | 4: binlog.rs, cc_common.rs, lib.rs, unified_log.rs |
| `internal::CryptoContext` | `*` | 1: tls_api.rs |
| `socks::OsError` | `*` | 1: packet_loop.rs |
| `splay` | `SplayNode`, `SplayTree` | 1: internal.rs |
| `unified_log::UnifiedLogging` | `*` | 1: internal.rs |
| `utils` | `TestAqm`, `TestSimLink`, `TestSimPacket`, `Thread`, `ThreadFn` | 2: packet_loop.rs, test_dualq.rs |
| `utils::File` | `*` | 1: binlog.rs |

## Per-file summary

| File | LOC | Traits | Structs | Enums | Type aliases | Fns | Inner #![allow] |
|---|---:|---:|---:|---:|---:|---:|---:|
| `rs/fq/src/binlog.rs` | 370 | 0 | 0 | 1 | 0 | 14 | 0 |
| `rs/fq/src/bytestream.rs` | 369 | 0 | 2 | 1 | 0 | 37 | 0 |
| `rs/fq/src/cc_common.rs` | 264 | 0 | 2 | 1 | 0 | 15 | 0 |
| `rs/fq/src/config.rs` | 368 | 0 | 1 | 1 | 0 | 9 | 0 |
| `rs/fq/src/crypto_provider_api.rs` | 586 | 15 | 13 | 0 | 0 | 26 | 0 |
| `rs/fq/src/hash.rs` | 256 | 1 | 2 | 0 | 0 | 9 | 0 |
| `rs/fq/src/internal.rs` | 3671 | 5 | 32 | 5 | 0 | 275 | 0 |
| `rs/fq/src/lb.rs` | 218 | 0 | 3 | 1 | 0 | 5 | 0 |
| `rs/fq/src/lib.rs` | 2566 | 10 | 13 | 12 | 0 | 207 | 3 |
| `rs/fq/src/logger.rs` | 108 | 0 | 0 | 0 | 0 | 4 | 0 |
| `rs/fq/src/packet_loop.rs` | 751 | 4 | 6 | 1 | 0 | 14 | 0 |
| `rs/fq/src/performance_log.rs` | 113 | 0 | 0 | 1 | 0 | 2 | 0 |
| `rs/fq/src/qlog.rs` | 29 | 0 | 0 | 0 | 0 | 1 | 0 |
| `rs/fq/src/siphash.rs` | 34 | 0 | 0 | 0 | 0 | 2 | 0 |
| `rs/fq/src/socks.rs` | 372 | 0 | 8 | 0 | 0 | 18 | 0 |
| `rs/fq/src/splay.rs` | 241 | 1 | 2 | 0 | 0 | 12 | 0 |
| `rs/fq/src/test_dualq.rs` | 240 | 0 | 2 | 0 | 0 | 3 | 0 |
| `rs/fq/src/tls_api.rs` | 1063 | 0 | 3 | 0 | 0 | 53 | 0 |
| `rs/fq/src/unified_log.rs` | 431 | 1 | 0 | 0 | 0 | 16 | 0 |
| `rs/fq/src/utils.rs` | 1050 | 2 | 6 | 1 | 0 | 88 | 0 |

