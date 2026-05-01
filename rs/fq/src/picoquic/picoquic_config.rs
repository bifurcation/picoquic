//! Translation of `picoquic/picoquic_config.h`.
//!
//! Demo-application–facing configuration plumbing for picoquic-core:
//! the `picoquic_quic_config_t` bag of CLI-derived options, the
//! `picoquic_option_enum_t` tag identifying each option, and the
//! `picoquic_create_and_configure` one-shot constructor that turns
//! a populated config into a fully-wired `picoquic_quic_t`.
//!
//! Phase 1: signatures only — every body is `todo!()`.
//!
//! Pointer-shape decisions for the config struct were guided by
//! reading `picoquic/config.c`:
//!
//! * Every `char const*` string field that
//!   `picoquic_config_clear` `free`s is owned in C (allocated via
//!   `config_set_string_param`'s `malloc` + `memcpy`).  These map
//!   to `Option<String>`; `None` substitutes for the C `NULL`
//!   sentinel that all callers test before use.
//! * `multipath_alt_config: *mut char` is owned the same way and
//!   becomes `Option<String>`.
//! * `ech_target: uint8_t*` plus `ech_target_len` is owned by the
//!   config (`picoquic_config_clear` frees it after the base64
//!   decode in `picoquic_option_ECH_client`); it becomes
//!   `Option<Vec<u8>>` with the length implicit in the vector.
//! * `ticket_encryption_key: const uint8_t*` is *borrowed* in C —
//!   `picoquic_config_clear` does not free it.  Phase 1 stores an
//!   owned `Option<Vec<u8>>` for safety; in v1 only the demo apps
//!   set this field, and they can supply an owned buffer.  Phase 3
//!   may revisit if a caller actually relies on aliasing.
//! * Single-bit `unsigned int : 1` flag bitfields (`use_long_log`,
//!   `do_retry`, …) collapse to individual `bool` fields.  Every C
//!   call site reads/writes one bit at a time as a Boolean
//!   (`config->do_retry = 1`), so the
//!   integer-with-mask-and-shift translation buys nothing here and
//!   `bitflags!` is overkill for an unrelated set of flags.
//! * Bitfield `enable_sslkeylog` is gated by
//!   `#ifndef PICOQUIC_WITHOUT_SSLKEYLOG` in C.  v1 follows the
//!   canonical build (the macro is undefined), so the field is
//!   always present in Rust; a `cfg`-gated variant lands when the
//!   build options are translated.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// Variant names mirror the C enum tags one-to-one and all share
// the `picoquic_option_` prefix.  Renaming would break source-level
// parity required by the translation plan.
#![allow(clippy::enum_variant_names)]

use crate::picoquic::picoquic::{
    picoquic_lossbit_version_enum, picoquic_quic_t, picoquic_spinbit_version_enum,
    picoquic_stream_data_cb_fn,
};

// ---------------------------------------------------------------------------
// Option identifiers.

/// One identifier per CLI / API option understood by
/// `picoquic_quic_config_t`.  Mirrors the C
/// `picoquic_option_enum_t`; variant order is load-bearing — the
/// option dispatch table in `picoquic/config.c` indexes by
/// variant — so the enum does not get its discriminants reordered.
///
/// `picoquic_option_SSLKEYLOG` is unconditional here even though
/// the C enum gates it on `#ifndef PICOQUIC_WITHOUT_SSLKEYLOG`,
/// matching the canonical-build behavior; see the module-level
/// docs for the rationale.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum picoquic_option_enum_t {
    picoquic_option_CERT,
    picoquic_option_KEY,
    picoquic_option_SERVER_PORT,
    picoquic_option_PROPOSED_VERSION,
    picoquic_option_OUTDIR,
    picoquic_option_WWWDIR,
    picoquic_option_MAX_CONNECTIONS,
    picoquic_option_DO_RETRY,
    picoquic_option_INITIAL_RANDOM,
    picoquic_option_RESET_SEED,
    picoquic_option_DisablePortBlocking,
    picoquic_option_SOLUTION_DIR,
    picoquic_option_CC_ALGO,
    picoquic_option_CC_OPTION,
    picoquic_option_SPINBIT,
    picoquic_option_LOSSBIT,
    picoquic_option_MULTIPATH,
    picoquic_option_DEST_IF,
    picoquic_option_CIPHER_SUITE,
    picoquic_option_INIT_CNXID,
    picoquic_option_LOG_FILE,
    picoquic_option_LONG_LOG,
    picoquic_option_BINLOG_DIR,
    picoquic_option_QLOG_DIR,
    picoquic_option_MTU_MAX,
    picoquic_option_SNI,
    picoquic_option_ALPN,
    picoquic_option_ROOT_TRUST_FILE,
    picoquic_option_FORCE_ZERO_SHARE,
    picoquic_option_CNXID_LENGTH,
    picoquic_option_NO_DISK,
    picoquic_option_Idle_Timeout,
    picoquic_option_LARGE_CLIENT_HELLO,
    picoquic_option_Ticket_File_Name,
    picoquic_option_Token_File_Name,
    picoquic_option_Socket_buffer_size,
    picoquic_option_Performance_Log,
    picoquic_option_Preemptive_Repeat,
    picoquic_option_Version_Upgrade,
    picoquic_option_No_GSO,
    picoquic_option_BDP_frame,
    picoquic_option_CWIN_MAX,
    picoquic_option_SSLKEYLOG,
    picoquic_option_AddressDiscovery,
    picoquic_option_ECH_server,
    picoquic_option_ECH_client,
    picoquic_option_ECH_init,
    picoquic_option_FLOW_CONTROL_MAX,
    picoquic_option_Preferred_V4,
    picoquic_option_Preferred_V6,
    picoquic_option_HELP,
}

// ---------------------------------------------------------------------------
// Configuration struct.

/// Configuration bag for a QUIC context, populated from CLI flags
/// (via [`picoquic_quic_config_t::command_line`]) or directly
/// (via [`picoquic_quic_config_t::set_option`]) and consumed by
/// [`picoquic_create_and_configure`].
///
/// Mirrors the C `picoquic_quic_config_t` field-for-field;
/// `repr(C)` is dropped because the struct never crosses an
/// external boundary.  See the module-level docs for the
/// pointer-shape rationale.
///
/// `Default` produces the all-zero / `None` shape that the C side
/// reaches via `memset(config, 0, sizeof(...))` at the top of
/// `picoquic_config_init`.  Callers should follow that with
/// [`picoquic_quic_config_t::init`] to pick up the documented
/// non-zero defaults (`nb_connections = 256`, `cnx_id_length =
/// -1`, `cwin_max = u64::MAX`, …).
#[derive(Debug, Default)]
pub struct picoquic_quic_config_t {
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
    /// C: `picoquic_spinbit_version_enum spinbit_policy`.
    pub spinbit_policy: picoquic_spinbit_version_enum,
    /// C: `picoquic_lossbit_version_enum lossbit_policy`.
    pub lossbit_policy: picoquic_lossbit_version_enum,
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
    /// PICOQUIC_WITHOUT_SSLKEYLOG`; v1 follows the canonical build
    /// (always defined), so the field is unconditional here.
    pub enable_sslkeylog: bool,

    // Server only.
    pub www_dir: Option<String>,
    pub reset_seed: [u8; 16],
    /// Borrowed in C (`picoquic_config_clear` does not free it);
    /// owned `Vec<u8>` here for safety.
    pub ticket_encryption_key: Option<Vec<u8>>,
    pub ticket_encryption_key_length: usize,

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
    /// the config — `picoquic_config_clear` frees it in C.
    pub ech_target: Option<Vec<u8>>,
    pub ech_target_len: usize,

    pub flow_control_max: u64,

    // Preferred address, encoded as strings.
    pub preferred_address_v4: Option<String>,
    pub preferred_address_v6: Option<String>,
}

impl picoquic_quic_config_t {
    /// C: `picoquic_config_init`.  Initialise the struct to the
    /// documented defaults (256 connections, `cnx_id_length = -1`,
    /// `cwin_max = u64::MAX`, idle timeout from
    /// `PICOQUIC_MICROSEC_HANDSHAKE_MAX`, etc.).  The C body first
    /// `memset`s the struct to zero; in Rust we expect the caller
    /// to start from `Self::default()` so this method only needs
    /// to layer the non-zero defaults on top.
    pub fn init(&mut self) {
        todo!()
    }

    /// C: `picoquic_config_clear`.  Release every owned
    /// allocation in the struct (C `free`s each `char const*`
    /// that was set via `config_set_string_param`) and then
    /// re-initialise via [`Self::init`].  In Rust the `Option`
    /// fields drop their backing buffers automatically when
    /// reassigned to `None`, so the body collapses to `*self =
    /// Self::default()` followed by `self.init()`.
    pub fn clear(&mut self) {
        todo!()
    }

    /// C: `picoquic_config_set_option`.  Apply one option,
    /// selected by `option_num`, to the config.  `opt_val` is
    /// optional — flag-style options (`picoquic_option_DO_RETRY`,
    /// `picoquic_option_LONG_LOG`, …) ignore it; value-style
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
        _option_num: picoquic_option_enum_t,
        _opt_val: Option<&str>,
    ) -> Result<(), ()> {
        todo!()
    }

    /// C: `picoquic_config_command_line`.  Dispatch one option
    /// from a single-character flag (`-x`).  `p_optind` advances
    /// past any extra arguments consumed beyond the inline
    /// `optarg`, mirroring the C in/out parameter so that an
    /// outer getopt-style loop stays in sync.
    ///
    /// `argv` is a borrowed slice of borrowed strings — the C
    /// caller (`picoquicfirst` etc.) owns the argument vector for
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

    /// C: `picoquic_config_command_line_ex`.  Like
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

/// C: `picoquic_config_option_letters`.  Build the getopt-style
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
pub fn picoquic_config_option_letters() -> Result<String, ()> {
    todo!()
}

/// C: `picoquic_config_usage_file` — write the option help to a
/// `core::fmt::Write` sink.  The C parameter was `FILE*`; the
/// Rust translation accepts any sink (a `String` buffer, the
/// stdout/stderr handles under the `std` feature, or a custom
/// writer) so the function stays `no_std`-friendly.
pub fn picoquic_config_usage_file(_w: &mut dyn core::fmt::Write) {
    todo!()
}

/// C: `picoquic_config_usage` — convenience wrapper that prints
/// the option help to stderr.  Phase 3 will route this through
/// `eprintln!` (std-only); for now it is just a `todo!()`.
pub fn picoquic_config_usage() {
    todo!()
}

/// C: `picoquic_create_and_configure`.  Build a fully-configured
/// QUIC context from `config` plus an application-supplied
/// callback.  Returns `None` when context creation fails (the C
/// side returned `NULL`).
///
/// Pointer-shape choices, derived from the C signature and the
/// body in `config.c`:
///
/// * `config: *mut picoquic_quic_config_t` is consumed-borrowed
///   (`&mut`): the body reads every field and may set up
///   downstream owned state, but it does not free the struct
///   itself — ownership stays with the caller.
/// * `default_callback_fn` + `default_callback_ctx` collapse to
///   one `Option<Box<dyn picoquic_stream_data_cb_fn>>` per the
///   function-pointers-map-to-traits rule, with the `void*`
///   context folded into the trait implementor's state.  `None`
///   matches the C "no default callback" case where the function
///   pointer was `NULL`.
/// * `p_simulated_time: *mut u64` carries simulated wall time for
///   tests; the C QUIC context retains the pointer across calls,
///   so an `&'a mut u64` borrow expresses the contract directly.
///   Phase 3 may revisit (e.g. with a clock trait) once the QUIC
///   context type is real.
pub fn picoquic_create_and_configure(
    _config: &mut picoquic_quic_config_t,
    _default_callback: Option<Box<dyn picoquic_stream_data_cb_fn>>,
    _current_time: u64,
    _p_simulated_time: Option<&mut u64>,
) -> Option<Box<picoquic_quic_t>> {
    todo!()
}

#[cfg(test)]
mod test {}
