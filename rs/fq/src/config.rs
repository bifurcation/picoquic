//! Translation of `quic/config.h`.
//!
//! Demo-application–facing configuration plumbing for quic-core:
//! the `quic_config_t` bag of CLI-derived options, the
//! `option_enum_t` tag identifying each option, and the
//! `create_and_configure` one-shot constructor that turns
//! a populated config into a fully-wired `quic_t`.
//!
//! Phase 1: signatures only — every body is `todo!()`.
//!
//! Pointer-shape decisions for the config struct were guided by
//! reading `quic/config.c`:
//!
//! * Every `char const*` string field that
//!   `config_clear` `free`s is owned in C (allocated via
//!   `config_set_string_param`'s `malloc` + `memcpy`).  These map
//!   to `Option<String>`; `None` substitutes for the C `NULL`
//!   sentinel that all callers test before use.
//! * `multipath_alt_config: *mut char` is owned the same way and
//!   becomes `Option<String>`.
//! * `ech_target: uint8_t*` plus `ech_target_len` is owned by the
//!   config (`config_clear` frees it after the base64
//!   decode in `option_ECH_client`); it becomes
//!   `Option<Vec<u8>>` with the length implicit in the vector.
//! * `ticket_encryption_key: const uint8_t*` is *borrowed* in C —
//!   `config_clear` does not free it.  Phase 1 stores an
//!   owned `Option<Vec<u8>>` for safety; in v1 only the demo apps
//!   set this field, and they can supply an owned buffer.  Phase 3
//!   may revisit if a caller actually relies on aliasing.  The
//!   paired `ticket_encryption_key_length` field is dropped — its
//!   value is always implicit in the Vec length.
//! * Single-bit `unsigned int : 1` flag bitfields (`use_long_log`,
//!   `do_retry`, …) collapse to individual `bool` fields.  Every C
//!   call site reads/writes one bit at a time as a Boolean
//!   (`config->do_retry = 1`), so the
//!   integer-with-mask-and-shift translation buys nothing here and
//!   `bitflags!` is overkill for an unrelated set of flags.
//! * Bitfield `enable_sslkeylog` is gated by
//!   `#ifndef WITHOUT_SSLKEYLOG` in C.  v1 follows the
//!   canonical build (the macro is undefined), so the field is
//!   always present in Rust; a `cfg`-gated variant lands when the
//!   build options are translated.

#![allow(non_camel_case_types)]
// Variant names mirror the C enum tags one-to-one and all share
// the `option_` prefix.  Renaming would break source-level
// parity required by the translation plan.
#![allow(clippy::enum_variant_names)]

use crate::{StreamDataCb, lossbit_version_enum, quic_t, spinbit_version_enum};

// ---------------------------------------------------------------------------
// Option identifiers.

/// One identifier per CLI / API option understood by
/// `quic_config_t`.  Mirrors the C
/// `option_enum_t`; variant order is load-bearing — the
/// option dispatch table in `quic/config.c` indexes by
/// variant — so the enum does not get its discriminants reordered.
///
/// `option_SSLKEYLOG` is unconditional here even though
/// the C enum gates it on `#ifndef WITHOUT_SSLKEYLOG`,
/// matching the canonical-build behavior; see the module-level
/// docs for the rationale.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum option_enum_t {
    option_CERT,
    option_KEY,
    option_SERVER_PORT,
    option_PROPOSED_VERSION,
    option_OUTDIR,
    option_WWWDIR,
    option_MAX_CONNECTIONS,
    option_DO_RETRY,
    option_INITIAL_RANDOM,
    option_RESET_SEED,
    option_DisablePortBlocking,
    option_SOLUTION_DIR,
    option_CC_ALGO,
    option_CC_OPTION,
    option_SPINBIT,
    option_LOSSBIT,
    option_MULTIPATH,
    option_DEST_IF,
    option_CIPHER_SUITE,
    option_INIT_CNXID,
    option_LOG_FILE,
    option_LONG_LOG,
    option_BINLOG_DIR,
    option_QLOG_DIR,
    option_MTU_MAX,
    option_SNI,
    option_ALPN,
    option_ROOT_TRUST_FILE,
    option_FORCE_ZERO_SHARE,
    option_CNXID_LENGTH,
    option_NO_DISK,
    option_Idle_Timeout,
    option_LARGE_CLIENT_HELLO,
    option_Ticket_File_Name,
    option_Token_File_Name,
    option_Socket_buffer_size,
    option_Performance_Log,
    option_Preemptive_Repeat,
    option_Version_Upgrade,
    option_No_GSO,
    option_BDP_frame,
    option_CWIN_MAX,
    option_SSLKEYLOG,
    option_AddressDiscovery,
    option_ECH_server,
    option_ECH_client,
    option_ECH_init,
    option_FLOW_CONTROL_MAX,
    option_Preferred_V4,
    option_Preferred_V6,
    option_HELP,
}

// ---------------------------------------------------------------------------
// Configuration struct.

/// Configuration bag for a QUIC context, populated from CLI flags
/// (via [`quic_config_t::command_line`]) or directly
/// (via [`quic_config_t::set_option`]) and consumed by
/// [`create_and_configure`].
///
/// Mirrors the C `quic_config_t` field-for-field;
/// `repr(C)` is dropped because the struct never crosses an
/// external boundary.  See the module-level docs for the
/// pointer-shape rationale.
///
/// `Default` produces the all-zero / `None` shape that the C side
/// reaches via `memset(config, 0, sizeof(...))` at the top of
/// `config_init`.  Callers should follow that with
/// [`quic_config_t::init`] to pick up the documented
/// non-zero defaults (`nb_connections = 256`, `cnx_id_length =
/// -1`, `cwin_max = u64::MAX`, …).
#[derive(Debug, Default)]
pub struct quic_config_t {
    pub nb_connections: u32,
    pub solution_dir: Option<String>,
    pub server_cert_file: Option<String>,
    pub server_key_file: Option<String>,
    pub log_file: Option<String>,
    pub bin_dir: Option<String>,
    pub qlog_dir: Option<String>,
    pub performance_log: Option<String>,
    pub server_port: u16,
    pub local_port: u16,
    pub is_port_shared: i32,
    pub nb_threads: i32,
    pub dest_if: i32,
    pub mtu_max: i32,
    /// `-1` is the C "unset" sentinel applied by `init`; values
    /// `>= 0` set the connection-ID length explicitly.  Kept as
    /// `i32` rather than `Option<u8>` for source-level parity with
    /// the C body.
    pub cnx_id_length: i32,
    pub idle_timeout: i32,
    pub socket_buffer_size: i32,
    pub cc_algo_id: Option<String>,
    pub cc_algo_option_string: Option<String>,
    pub cnx_id_cbdata: Option<String>,
    /// C: `spinbit_version_enum spinbit_policy`.
    pub spinbit_policy: spinbit_version_enum,
    /// C: `lossbit_version_enum lossbit_policy`.
    pub lossbit_policy: lossbit_version_enum,
    pub multipath_option: i32,
    pub multipath_alt_config: Option<String>,
    pub bdp_frame_option: i32,
    pub cwin_max: u64,
    pub address_discovery_mode: i32,

    // Common flags.
    pub initial_random: u32,
    pub use_long_log: bool,
    pub do_preemptive_repeat: bool,
    pub do_not_use_gso: bool,
    pub disable_port_blocking: bool,
    /// Originally bitfield-gated by `#ifndef
    /// WITHOUT_SSLKEYLOG`; v1 follows the canonical build
    /// (always defined), so the field is unconditional here.
    pub enable_sslkeylog: bool,

    // Server only.
    pub www_dir: Option<String>,
    pub reset_seed: [u8; 16],
    /// Borrowed in C (`config_clear` does not free it);
    /// owned `Vec<u8>` here for safety.  The C `ticket_encryption_key_length`
    /// field is dropped — its value is always `ticket_encryption_key.as_ref().map_or(0, Vec::len)`.
    pub ticket_encryption_key: Option<Vec<u8>>,

    // Server flags.
    pub do_retry: bool,
    pub has_reset_seed: bool,

    // Client only.
    pub ticket_file_name: Option<String>,
    pub token_file_name: Option<String>,
    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub out_dir: Option<String>,
    pub root_trust_file: Option<String>,
    pub cipher_suite_id: i32,
    pub proposed_version: u32,
    pub desired_version: u32,
    pub force_zero_share: bool,
    pub no_disk: bool,
    pub large_client_hello: bool,

    // ECH parameters for server.
    pub ech_key_file: Option<String>,
    pub ech_config_file: Option<String>,
    pub ech_public_name: Option<String>,
    /// ECH parameter for the client, base64-decoded.  Owned by
    /// the config — `config_clear` frees it in C.  The C
    /// `ech_target_len` field is dropped — its value is always
    /// `ech_target.as_ref().map_or(0, Vec::len)`.
    pub ech_target: Option<Vec<u8>>,

    pub flow_control_max: u64,

    // Preferred address, encoded as strings.
    pub preferred_address_v4: Option<String>,
    pub preferred_address_v6: Option<String>,
}

impl quic_config_t {
    /// C: `config_init`.  Initialise the struct to the
    /// documented defaults (256 connections, `cnx_id_length = -1`,
    /// `cwin_max = u64::MAX`, idle timeout from
    /// `MICROSEC_HANDSHAKE_MAX`, etc.).  The C body first
    /// `memset`s the struct to zero; in Rust we expect the caller
    /// to start from `Self::default()` so this method only needs
    /// to layer the non-zero defaults on top.
    pub fn init(&mut self) {
        todo!()
    }

    /// C: `config_clear`.  Release every owned
    /// allocation in the struct (C `free`s each `char const*`
    /// that was set via `config_set_string_param`) and then
    /// re-initialise via [`Self::init`].  In Rust the `Option`
    /// fields drop their backing buffers automatically when
    /// reassigned to `None`, so the body collapses to `*self =
    /// Self::default()` followed by `self.init()`.
    pub fn clear(&mut self) {
        todo!()
    }

    /// C: `config_set_option`.  Apply one option,
    /// selected by `option_num`, to the config.  `opt_val` is
    /// optional — flag-style options (`option_DO_RETRY`,
    /// `option_LONG_LOG`, …) ignore it; value-style
    /// options require it and return `Err(())` when it is
    /// missing or malformed.
    ///
    /// `Result<(), ()>` is a placeholder until the crate-level
    /// `Error` enum lands; the C signature returned `int`
    /// (`0` ↔ `Ok`, `-1` ↔ `Err`).
    // TODO(error-enum): swap `()` for the crate's `Error` once it lands.
    #[allow(clippy::result_unit_err)]
    pub fn set_option(
        &mut self,
        _option_num: option_enum_t,
        _opt_val: Option<&str>,
    ) -> Result<(), ()> {
        todo!()
    }

    /// C: `config_command_line`.  Dispatch one option
    /// from a single-character flag (`-x`).  `p_optind` advances
    /// past any extra arguments consumed beyond the inline
    /// `optarg`, mirroring the C in/out parameter so that an
    /// outer getopt-style loop stays in sync.
    ///
    /// `argv` is a borrowed slice of borrowed strings — the C
    /// caller (`first` etc.) owns the argument vector for
    /// the program lifetime, so a borrow is safe.  `argc` is
    /// implicit in `argv.len()` and the `int argc` parameter is
    /// dropped.
    // TODO(error-enum): swap `()` for the crate's `Error` once it lands.
    #[allow(clippy::result_unit_err)]
    pub fn command_line(
        &mut self,
        _opt: i32,
        _p_optind: &mut usize,
        _argv: &[&str],
        _optarg: Option<&str>,
    ) -> Result<(), ()> {
        todo!()
    }

    /// C: `config_command_line_ex`.  Like
    /// [`Self::command_line`] but accepts both single-character
    /// (`-x`) and long-form (`--name`) option strings; the leading
    /// dashes are part of `opt_string`, matching the C contract.
    // TODO(error-enum): swap `()` for the crate's `Error` once it lands.
    #[allow(clippy::result_unit_err)]
    pub fn command_line_ex(
        &mut self,
        _opt_string: &str,
        _p_optind: &mut usize,
        _argv: &[&str],
        _optarg: Option<&str>,
    ) -> Result<(), ()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Free functions.

/// C: `config_option_letters`.  Build the getopt-style
/// option string from the dispatch table in `config.c` — one
/// letter per option, with `:` after each option that takes an
/// argument.
///
/// The C signature wrote into a caller-supplied buffer
/// (`option_string`, `string_max`) and reported the populated
/// length via `*string_length`; the Rust wrapper owns the buffer
/// and returns it directly, so the buffer-too-small failure mode
/// disappears.  The `Result<String, ()>` shape is kept for
/// signature stability with the rest of the API.
// TODO(error-enum): swap `()` for the crate's `Error` once it lands.
#[allow(clippy::result_unit_err)]
pub fn config_option_letters() -> Result<String, ()> {
    todo!()
}

/// C: `config_usage_file` — write the option help to a
/// `core::fmt::Write` sink.  The C parameter was `FILE*`; the
/// Rust translation accepts any sink (a `String` buffer, the
/// stdout/stderr handles under the `std` feature, or a custom
/// writer) so the function stays `no_std`-friendly.
pub fn config_usage_file(_w: &mut dyn core::fmt::Write) {
    todo!()
}

/// C: `config_usage` — convenience wrapper that prints
/// the option help to stderr.  Phase 3 will route this through
/// `eprintln!` (std-only); for now it is just a `todo!()`.
pub fn config_usage() {
    todo!()
}

/// C: `create_and_configure`.  Build a fully-configured
/// QUIC context from `config` plus an application-supplied
/// callback.  Returns `None` when context creation fails (the C
/// side returned `NULL`).
///
/// Pointer-shape choices, derived from the C signature and the
/// body in `config.c`:
///
/// * `config: *mut quic_config_t` is consumed-borrowed
///   (`&mut`): the body reads every field and may set up
///   downstream owned state, but it does not free the struct
///   itself — ownership stays with the caller.
/// * `default_callback_fn` + `default_callback_ctx` collapse to
///   one `Option<Box<dyn StreamDataCb>>` per the
///   function-pointers-map-to-traits rule, with the `void*`
///   context folded into the trait implementor's state.  `None`
///   matches the C "no default callback" case where the function
///   pointer was `NULL`.
/// * `p_simulated_time: *mut u64` carries simulated wall time for
///   tests; the C QUIC context retains the pointer across calls,
///   so an `&'a mut u64` borrow expresses the contract directly.
///   Phase 3 may revisit (e.g. with a clock trait) once the QUIC
///   context type is real.
pub fn create_and_configure(
    _config: &mut quic_config_t,
    _default_callback: Option<Box<dyn StreamDataCb>>,
    _current_time: u64,
    _p_simulated_time: Option<&mut u64>,
) -> Option<Box<quic_t>> {
    todo!()
}

#[cfg(test)]
mod test {}
