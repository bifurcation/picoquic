# Test translation guide (Phase 3A)

A focused reference for the C-test → Rust-test translation pass.
**Read this first.**  It substitutes for grepping `lib.rs` /
`internal.rs`; you only need to open them when you can't find an
answer here.

---

## Operating mandate

* **Translate every C test body faithfully.**  No `todo!("entry_fn")`
  placeholders.  The body should reproduce what the C body does.
* **Compile-not-pass is the gate.**  Tests will panic on the first
  `todo!()` they hit inside a Phase-1-stubbed API.  That's expected
  and the user explicitly approved it.
* **Use the Rust public API as designed.**  If the canonical method
  doesn't exist yet, call the *intended* method anyway — the
  resulting compile error documents the API Phase 4 must supply.
* **No `unsafe`.**  No edits outside `rs/fq/src/tests/` and
  `rs/fq/tests/fixtures/`.
* The build gate is `cargo fmt && cargo test --no-run && cargo
  clippy --tests --all-features -- -D warnings`.  Iterate until all
  three are clean.

---

## Crate map (where APIs live)

| Module | Surface |
|---|---|
| `crate::lib` (re-exported at root) | public API: `Quic`, `Connection`, `ConnectionId`, `Error`, `Result`, `Instant`, `Duration`, `VERSION`, `MAX_PACKET_SIZE`, `RESET_SECRET_SIZE`, `Alpn`, `State`, `PacketContext`, `PmtudPolicy`, `SpinbitVersion`, `LossbitVersion`, `PathStatus`, `CloseReason`, `CongestionAlgorithm`, callbacks (`StreamDataCallback`, `AlpnSelect`, `ConnectionIdCallback`, `Fuzz`, `StreamDirectReceive`, `CongestionControl`) |
| `crate::internal` | internal types touched by tests: `Path`, `PacketHeader`, `PacketType`, `Epoch`, `SackList`, `SackItem`, `Pacing`, `MiscFrameHeader`, `Tuple`, `LocalConnectionId`, `RemoteConnectionId`, `StreamDataNode`, `Packet`, `RegisteredToken`, `StoredTicket`, `StoredToken`, `IssuedTicket`, `MinMaxRtt`, `NewRenoSimState`; helpers like `parse_16/24/32/64`, `format_16/24/32/64`, `varint_encode/decode`, `get_packet_number64`, `create_long_header`, `create_packet_header`, `seed_bandwidth` |
| `crate::tp` | `TransportParameter`, `TransportParameters`, `PreferredAddress`, `VersionNegotiation`, `TransportParameter0RttKind`, `NB_TP_0RTT` |
| `crate::frames` | `FrameType`, `FrameType::name(u64) -> Option<&'static str>` |
| `crate::stream` | `StreamId(pub u64)`, `Direction`, `Role` + `is_client`, `is_bidir`, `is_local`, `from_parts`, `rank`, `kind`, `next_with_same_kind` |
| `crate::errors` | `InternalError` (picoquic-internal codes), `TransportError` (RFC 9000 §20), `transport_crypto_error(alert: u8)`, `InternalError::name(u64)` |
| `crate::bytestream` | `ByteStream<'a>`, `ByteStreamBuf`, `BYTESTREAM_MAX_BUFFER_SIZE` |
| `crate::splay` | `SplayTree<K, V>`, `SplayToken` |
| `crate::hash` | `HashTable`, `HashToken`, `HashItem` |
| `crate::arena` | `Arena<T>`, `Token<T>` |
| `crate::siphash` | `siphash(input: &[u8], key: &[u8; 16]) -> u64` |
| `crate::utils` | `uint8_to_str`, `constant_time_memcmp`, `set_preferred_address` |
| `crate::tls` | `Session`, `ClientConfig`, `ServerConfig`, `TlsBackend`, `PacketKey`, `HeaderKey`, `Keys`, `KeyPair`, `TlsCallbacks` |
| `crate::tls_api` | TLS handshake glue: `Aes128EcbContext`, `setup_initial_master_secret`, `setup_initial_secrets`, `rotate_app_secret`, `setup_test_aead_context`, `pn_enc_create_for_test`, `create_retry_protection_context`, `encode_retry_protection`, `verify_retry_protection`, `hash_create`, `hash_get_length`, `QUIC_AEAD_TAG_LEN` |
| `crate::lb` | LB CID config: `Config`, `ConnectionIdContext`, `ConnectionIdMethod`, `RotationBits` |
| `crate::config` | demo-app `Config`, `OptionId`, `Config::option_letters() -> String`, `Config::parse_command_line`, `Config::set_option` |
| `crate::logger` | `Logger` (backend trait), `Log` (per-Connection dispatch trait) |
| `crate::binlog` | `Binlog` (per-Connection trait), free fns `pdu`, `packet`, `tls_ticket` (file-only writers), `LogEventType`, `Quic::set_binlog`, `Quic::enable_binlog` |
| `crate::qlog` | `Quic::set_qlog` |
| `crate::header_protection` | `AesHeaderProtector<C>`, `AesHeaderKey<C>`, `ChaCha20HeaderProtector`, `ChaCha20HeaderKey` |
| `crate::cc_common` | `MIN_MAX_RTT_SCOPE`, `HYSTART_PP_*`, `MinMaxRtt`, `NewRenoSimState`, `ConnectionCc` (trait on Connection), `PathCc` (trait on Path) |
| `crate::socks` / `crate::socks_socket2` | `Socket` trait, `ServerSockets<S>`, `Socket2Udp` |
| `crate::packet_loop` | `LoopParam`, `LoopEvent`, `PacketLoopCbFn`, `NetworkThreadCtx`, `SocketCtx<S>` |
| `crate::sys::picotls` / `crate::sys::openssl` | TLS backends |
| `crate::tests::util` | test infrastructure (see below) |
| `crate::tests::dualq` | DualQ AQM `TestAqm` impl |

---

## Constructors & key signatures

```rust
// Top-level QUIC context.
Quic::new(
    max_nb_connections: u32,
    cert_file_name: Option<&str>,
    key_file_name: Option<&str>,
    cert_root_file_name: Option<&str>,
    default_alpn: Option<&str>,
    default_callback: Option<Box<dyn StreamDataCallback>>,
    cnx_id_callback: Option<Box<dyn ConnectionIdCallback>>,
    reset_seed: [u8; RESET_SECRET_SIZE],
    current_time: Instant,
    ticket_file_name: Option<&str>,
    ticket_encryption_key: Option<&[u8]>,
) -> Option<Box<Quic>>;

// Per-connection.
Quic::create_connection(
    &mut self,
    initial_cnx_id: ConnectionId,
    remote_cnx_id: ConnectionId,
    addr_to: Option<&SocketAddr>,
    start_time: Instant,
    preferred_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
    client_mode: bool,
) -> Option<&mut Connection>;

Quic::create_client_connection(
    &mut self,
    addr: &SocketAddr,
    start_time: Instant,
    preferred_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
    callback: Option<Box<dyn StreamDataCallback>>,
) -> Option<&mut Connection>;

Connection::start_client(&mut self) -> Result<(), Error>;
Connection::close(&mut self, application_reason_code: u64) -> Result<(), Error>;
Connection::tls_negotiated_alpn(&self) -> Alpn;
Connection::tls_sni(&self) -> Option<&str>;
Connection::close_reason(&self) -> Option<CloseReason>;

// ConnectionId.
ConnectionId::clone_from_slice(bytes: &[u8]) -> Option<Self>;
ConnectionId::with_size(len: usize) -> Option<Self>;
ConnectionId::as_bytes(&self) -> &[u8];
ConnectionId::as_bytes_mut(&mut self) -> &mut [u8];
ConnectionId::len(&self) -> usize;
ConnectionId::is_empty(&self) -> bool;
```

Time and duration are `fugit::Instant<u64, 1, 1_000_000>` and
`fugit::Duration<u64, 1, 1_000_000>` — microseconds.  Construct
with `Instant::from_ticks(usec)` / `Duration::from_ticks(usec)`.

---

## Translation conventions (apply uniformly)

| C pattern | Rust translation |
|---|---|
| `picoquic_quic_t* quic` | `&mut Quic` (or `Box<Quic>` when owning) |
| `picoquic_cnx_t* cnx` | `&mut Connection` (no `Cnx`) |
| `picoquic_path_t* path_x` | `&mut Path` |
| `picoquic_connection_id_t cid` | `ConnectionId` (Copy) |
| `int` return code (0 / -1 / errno) | `Result<T, crate::Error>` |
| `int` boolean flag | `bool` |
| `void* callback_ctx` | folds into trait implementor's state |
| `void (*fn_ptr)(...)` | `Box<dyn Trait>` |
| `(uint8_t* buf, size_t len)` pair | `&[u8]` (read) or `&mut [u8]` (write) |
| `(NULL, 0)` slice sentinel | empty Rust slice |
| `(struct sockaddr*)` | `&core::net::SocketAddr` |
| `uint64_t` microsecond timestamp | `Instant` |
| `uint64_t` microsecond interval | `Duration` |
| `picoquic_xxx_yyy_zzz` | `xxx_yyy_zzz` (drop prefix) |
| `cnx_id` / `cid` | `connection_id` |
| `cnx` (in identifiers) | `connection` |

`assert_eq!` / `assert!` over the C `if (...) ret = -1` pattern.
Use `?` propagation on `Result` returns.  Don't transcribe goto-style
ret tracking — that's a C quirk.

---

## Test-helper surface (`crate::tests::util`)

Available test-only utilities (most are still `todo!()` until Phase 4):

```rust
pub fn test_random(random_context: &mut u64) -> u64;
pub fn test_random_bytes(random_context: &mut u64, bytes: &mut [u8]);
pub fn test_uniform_random(random_context: &mut u64, rnd_max: u64) -> u64;
pub fn test_gauss_random(random_context: &mut u64) -> f64;
pub fn test_poisson_random(random_context: &mut u64, exp_minus_lambda_2_30: u64) -> u64;

pub struct TestSimPacket { /* simulator packet */ }
pub struct TestSimLink { /* in-process sim link */ }
pub trait TestAqm;            // active queue management vtable
pub enum JitterMode;
```

Plus certificate-fixture path constants (`TEST_FILE_SERVER_CERT`,
`TEST_FILE_SERVER_KEY`, `TEST_FILE_CERT_STORE`, `TEST_SNI`, etc.)
that translate the picoquictest globals.

**When adding helpers:** if more than one test file needs the
helper, put it in `tests/util.rs`.  Heavy connection-setup helpers
(`tls_api_init_ctx`, `picoquic_test_set_minimal_cnx`) go here.
Keep one canonical name per helper — if you find yourself defining
`default_quic` or similar in multiple test files, refactor to
`tests/util.rs`.

---

## Common patterns

### Minimal Quic context

```rust
fn default_quic() -> Option<Box<Quic>> {
    Quic::new(
        8, None, None, None, None, None, None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        None, None,
    )
}
```

### Known-answer table test

```rust
#[test]
fn varint() {
    let cases = [
        Case { encoding: [...], length: 1, decoded: 0 },
        // ...
    ];
    for case in &cases {
        // exercise encoder, decoder; assert results
    }
}
```

### Connection-lifecycle test

The C `tls_api_init_ctx*` family creates a server `Quic`, a
client `Quic`, plumbs them via the simulator, and runs a
handshake loop.  Until those helpers land in `tests/util.rs`
(Phase 3A may add them), translate as:

```rust
#[test]
fn xxx() {
    let mut server_quic = default_quic().expect("server quic");
    let mut client_quic = default_quic().expect("client quic");
    // ... the C body ...
    todo!("tls_api connection-loop helper not yet ported")  // only as a last resort
}
```

Prefer translating the body in full — the panic happens
naturally inside `Quic::new` or whatever else is `todo!()`.

### Frame parse / skip table

`crate::internal::skip_frame(buffer: &[u8]) -> Option<(usize, bool)>`
(or whatever the analogous Rust signature ends up being — call
the intended method and let the compile-error / panic document).

---

## Idioms to avoid

* **Don't reach into private fields.**  C tests routinely
  do `cnx->ack_ctx[pc].sack_list`; the Rust `Connection`
  struct's fields aren't `pub` from outside `crate::internal`.
  Use the public method API or surface a comment that the
  translation needs an accessor.
* **Don't use `unsafe`.**
* **Don't use `panic!` where `assert_eq!` / `assert!` work.**
* **Don't transcribe C goto-ret tracking.**  Use early returns or
  `?` propagation.
* **Don't add `#[cfg(target_os = "linux")]` etc.** unless the C
  body itself is OS-specific.
* **Don't translate `picoquic_test_compare_text_files` golden-file
  tests** without copying the reference file to `rs/fq/tests/fixtures/`
  first.  Reach the path via
  `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/<name>")`.

---

## Known stubbed surface

Most `Quic` / `Connection` / `Path` methods have signatures only —
their bodies are `todo!()`.  Tests will panic on the first call.
Don't try to verify behaviour against a working impl; the test is
a forward-looking spec for Phase 4.

Functions with **real bodies** today (will pass tests against
them):

* `crate::VERSION` (constant)
* `crate::internal::parse_16/24/32/64`, `format_16/24/32/64`
* `crate::ConnectionId::{as_bytes, len, is_empty}`
* `crate::stream::StreamId::{is_client, is_bidir, from_parts,
  rank, kind, next_with_same_kind}`
* `crate::frames::FrameType` discriminants (the variants exist,
  even if `name()` is `todo!()`)
* `crate::errors::{InternalError, TransportError}` discriminants
* `crate::tests::util::TestSimPacket` / `TestSimLink` types
  (signatures only)
