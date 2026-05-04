# `REVIEW(open)` triage

Post-Phase-2 status of the six `REVIEW(open)` sites in `rs/fq/src`.

## 1. `cc_common.rs:23` — raw `u64` RTT → typed `Duration`

**Big.**  Phase 2 shipped `Duration`, so the abstraction the comment was waiting on
exists.  But the migration touches ~50 fields and parameters across `cc_common.rs`,
`internal.rs`, and `lib.rs` — every `*_rtt: u64`, plus several `*_time: u64`
fields (`internal.rs:1342, 1449, 1468–69, 1482–83, 1657, 1858`).  Real-deal
refactor; should be a focused pass, not piggy-backed on cleanup.

REVIEW: Do it.  Don't be afraid.

## 2. `bytestream.rs:144` — drop `tail()` if Phase 4 finds no users

**Defer.**  Conditioned on Phase 4 finding (or not finding) callers.

REVIEW: OK to wait.

## 3. `lb.rs:102` — `cid_encryption_key` belongs inside the AES context

**Resolved by Phase 2.**  The post-Phase-2 `Aes128EcbContext`
(`enum { Encrypt(aes::Aes128Enc), Decrypt(aes::Aes128Dec) }`) already encapsulates
the key.  `Config.cid_encryption_key` is just the parsed wire-format input that
gets consumed when the contexts are built — the field on `Config` is fine as-is.
The REVIEW comment can be dropped.

REVIEW: OK, drop it.

## 4. `lb.rs:133` — AES contexts as `Option<Box<…>>`

**Comment misdiagnoses; leave the code.**  The optionality isn't a parsing-init
artifact (as the comment claims); it's method-dependent: `Clear` needs neither
context, `StreamCipher` needs only the encrypt context, `BlockCipher` needs both.
Folding the contexts into `ConnectionIdMethod` variants would remove the
`Option`s but causes other problems, so leave as-is.

REVIEW: OK, drop it.

## 5. `binlog.rs:217` — `impl Connection` → `Binlog` trait (same for logger / qlog / cc_common)

**Big.**  Cross-cutting refactor of four modules' inherent impls.  Worth doing as
a focused pass when we're ready.

REVIEW: Fix this!  It is important to have this right before Phase 3 so that
we don't have to do massive refactors of using code later..

## 6. `crypto_provider_api.rs:55` — `Ptls*` opaque structs

**Defer.**  The `Ptls*` placeholders cover the provider-registry layer
(OpenSSL / minicrypto / fusion / mbedtls); Phase 2's `crate::tls` traits cover the
per-connection hot path — these are different layers.  The likely answer is that
the entire `crypto_provider_api` module dissolves once a real `TlsBackend` impl
lands, but that's Phase 4 territory.

REVIEW: WTF is "fusion"?  You just made that up?

REVIEW: This is a very important thing to get cleaned up before phase 3.  We
should have the following structure:

* In `src/crypto.rs` and `src/tls.rs` - TLS traits that capture the APIs that
  crypto / TLS providers need to conform to.  There should be no code in these
  modules that is specific to picotls or OpenSSL or anything other specific
  implementation.

* In a `src/sys` - Files that implement the traits using various libraries.  In
  the current effort, we should implement with picotls and OpenSSL (the latter
  via the `openssl` crate).


