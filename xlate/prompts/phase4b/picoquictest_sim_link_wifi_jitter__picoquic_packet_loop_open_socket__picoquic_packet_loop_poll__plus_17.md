# Phase 4B missing implementation batch

## NO SHORTCUTS — do the hard work

Phase 4B implements approved required_missing items.  Past sweeps
produced hundreds of stubs that compile but don't translate the
C body — that is failure, not completion.  The gate flags all
of these patterns identically:

* `todo!()`
* `unimplemented!()`
* `// SKIP:` or any placeholder marker (None / Err(Generic) /
  Ok(()) returns plus an excuse comment) — forbidden
* Fabricated default returns (0, false, None, empty Vec) when
  the C body computes a real value — forbidden
* Half-translated bodies that punt error paths to `todo!()` —
  forbidden

**Nothing in picoquic is fundamentally untranslatable.**  Every
function has a body — translate it.  Concrete mappings:
  * `pthread_create` → `std::thread::spawn`
  * `pipe()` wake-up → `std::sync::mpsc` or `Condvar`
  * `select`/`poll`/`io_uring` → `mio` crate
  * loglib helpers → `log` crate + standard file I/O
  * borrow-checker issues → `&mut self`, `RefCell`, restructure

**Forbidden 'blocker' excuses** (every one is a shortcut):
'out of scope', 'deferred to a later pass', 'multi-threading
not in scope', 'loglib not in scope', 'sub-system not yet
translated', 'needs design thought', 'borrow-checker issue'.

The ONLY acceptable bare `todo!()` is a concrete external
dependency outside our reach (e.g. 'blocked: needs picotls API
not yet exposed in the Rust binding') — and even then, prefer
translating the dependency first.

## Implementation rules

Implement the approved missing Rust functions/items listed below.
Follow `CLAUDE.md`, `TRANSLATE_PLAN.md`, and
`xlate/impl_translation_guide.md`.

* Translate the C behavior faithfully into safe, idiomatic Rust.
* Keep the approved module structure unless implementation proves
  it wrong; if it is wrong, stop and report the needed Phase 4A
  plan amendment.
* Add `/// C: ` references for implemented functions so Phase 4A
  can map them on the next refresh.
* Edit only `rs/fq/`, `scripts/`, `xlate/`, or markdown files
  allowed by the repository instructions.

## Verification — run ONLY these cargo commands

* **`cargo test --no-run`** — compile-only.  Use this as your
  build gate.  Do NOT run `cargo test` (full suite) — it will
  run all 505 tests and take 2+ hours.
* **`cargo test <specific_test_name>`** — running a single
  named test for narrow verification is fine.
* **`cargo fmt`** and **`cargo clippy --tests --all-features
  -- -D warnings`** — final polish.
* **Do NOT prefix with `CARGO_INCREMENTAL=0`** — the allowlist
  blocks env-var prefixes; just run `cargo ...` directly.

Run these after implementing:

```sh
cargo fmt
cargo test --no-run
cargo clippy --tests --all-features -- -D warnings
```

## `picoquic/sim_link.c:picoquictest_sim_link_wifi_jitter`
* C source: `picoquic/sim_link.c:201-228`
* C signature: `uint64_t picoquictest_sim_link_wifi_jitter(picoquictest_sim_link_t *)`
* Approved Rust destination: `rs/fq/src/tests/harness.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sockloop.c:picoquic_packet_loop_open_socket`
* C source: `picoquic/sockloop.c:363-462`
* C signature: `int picoquic_packet_loop_open_socket(int, int, picoquic_socket_ctx_t *, uint8_t)`
* Approved Rust destination: `rs/fq/src/packet_loop.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/sockloop.c:picoquic_packet_loop_poll`
* C source: `picoquic/sockloop.c:907-992`
* C signature: `int picoquic_packet_loop_poll(picoquic_socket_ctx_t *, int, struct pollfd *, struct sockaddr_storage *, struct sockaddr_storage *, int *, unsigned char *, uint8_t *, int, int64_t, int *, picoquic_network_thread_ctx_t *, int *)`
* Approved Rust destination: `rs/fq/src/packet_loop.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/ticket_store.c:picoquic_update_stored_ticket`
* C source: `picoquic/ticket_store.c:529-571`
* C signature: `void picoquic_update_stored_ticket(picoquic_cnx_t *, picoquic_path_t *)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/timing.c:picoquic_validate_bdp_seed`
* C source: `picoquic/timing.c:90-114`
* C signature: `void picoquic_validate_bdp_seed(picoquic_cnx_t *, picoquic_path_t *, uint64_t, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_compute_initial_secrets`
* C source: `picoquic/tls_api.c:1478-1498`
* C signature: `int picoquic_compute_initial_secrets(picoquic_quic_t *, int, picoquic_connection_id_t *, ptls_cipher_suite_t **, uint8_t *, uint8_t *)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_crypto_uniform_random`
* C source: `picoquic/tls_api.c:778-788`
* C signature: `uint64_t picoquic_crypto_uniform_random(picoquic_quic_t *, uint64_t)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_get_cipher_suite_by_id`
* C source: `picoquic/tls_api.c:435-449`
* C signature: `ptls_cipher_suite_t * picoquic_get_cipher_suite_by_id(int, int)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_public_random_seed`
* C source: `picoquic/tls_api.c:859-866`
* C signature: `void picoquic_public_random_seed(picoquic_quic_t *)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_server_encrypt_retry_token`
* C source: `picoquic/tls_api.c:2849-2886`
* C signature: `int picoquic_server_encrypt_retry_token(picoquic_quic_t *, const struct sockaddr *, int, uint8_t *, size_t *, size_t, const uint8_t *, size_t)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_server_setup_ticket_aead_contexts`
* C source: `picoquic/tls_api.c:2414-2442`
* C signature: `int picoquic_server_setup_ticket_aead_contexts(picoquic_quic_t *, ptls_context_t *, const uint8_t *, size_t)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/tls_api.c:picoquic_set_key_from_secret`
* C source: `picoquic/tls_api.c:1342-1361`
* C signature: `int picoquic_set_key_from_secret(ptls_cipher_suite_t *, int, int, picoquic_crypto_context_t *, const void *, const char *)`
* Approved Rust destination: `rs/fq/src/tls_api.rs`
* Item shape: private helper
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/transport.c:picoquic_decode_transport_preferred_address_address`
* C source: `picoquic/transport.c:130-162`
* C signature: `size_t picoquic_decode_transport_preferred_address_address(uint8_t *, size_t, picoquic_tp_preferred_address_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/transport.c:picoquic_encode_transport_param_version_negotiation`
* C source: `picoquic/transport.c:173-224`
* C signature: `uint8_t * picoquic_encode_transport_param_version_negotiation(uint8_t *, uint8_t *, int, picoquic_cnx_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/transport.c:picoquic_encode_transport_preferred_address_address`
* C source: `picoquic/transport.c:98-128`
* C signature: `uint8_t * picoquic_encode_transport_preferred_address_address(uint8_t *, uint8_t *, picoquic_tp_preferred_address_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/transport.c:picoquic_transport_param_cid_decode`
* C source: `picoquic/transport.c:87-96`
* C signature: `int picoquic_transport_param_cid_decode(picoquic_cnx_t *, uint8_t *, uint64_t, picoquic_connection_id_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/transport.c:picoquic_transport_param_cid_encode`
* C source: `picoquic/transport.c:77-85`
* C signature: `uint8_t * picoquic_transport_param_cid_encode(uint8_t *, const uint8_t *, picoquic_tp_enum, picoquic_connection_id_t *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/transport.c:picoquic_transport_param_type_flag_encode`
* C source: `picoquic/transport.c:68-75`
* C signature: `uint8_t * picoquic_transport_param_type_flag_encode(uint8_t *, const uint8_t *, picoquic_tp_enum)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/transport.c:picoquic_transport_param_type_varint_encode`
* C source: `picoquic/transport.c:59-66`
* C signature: `uint8_t * picoquic_transport_param_type_varint_encode(uint8_t *, const uint8_t *, picoquic_tp_enum, uint64_t)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching

## `picoquic/transport.c:picoquic_transport_param_varint_decode`
* C source: `picoquic/transport.c:31-41`
* C signature: `uint64_t picoquic_transport_param_varint_decode(picoquic_cnx_t *, uint8_t *, uint64_t, int *)`
* Approved Rust destination: `rs/fq/src/internal.rs`
* Item shape: public API function or method
* Verification: targeted cargo test if known; otherwise Phase 4 cargo gates
* Reason: no Rust counterpart found by C doc reference or conservative name matching
