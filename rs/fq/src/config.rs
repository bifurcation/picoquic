//! Translation of `quic/config.h`.
//!
//! Demo-application–facing configuration plumbing for quic-core:
//! the [`Config`] bag of CLI-derived options, the [`OptionId`]
//! tag identifying each option, and [`Config::create_and_configure`],
//! the one-shot constructor that turns a populated config into a
//! fully-wired [`Quic`].
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

use crate::Error;
use crate::Instant;
use crate::{LossbitVersion, Quic, SpinbitVersion, StreamDataCb};

// ---------------------------------------------------------------------------
// Option identifiers.

/// One identifier per CLI / API option understood by [`Config`].
///
/// Mirrors the C `picoquic_option_enum_t`; variant order is
/// load-bearing — the option dispatch table in `quic/config.c`
/// indexes by variant — so the discriminants do not get reordered.
///
/// [`OptionId::SslKeyLog`] is unconditional here even though the C
/// enum gates it on `#ifndef WITHOUT_SSLKEYLOG`, matching the
/// canonical-build behavior; see the module-level docs for the
/// rationale.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OptionId {
    Cert,
    Key,
    ServerPort,
    ProposedVersion,
    OutDir,
    WwwDir,
    MaxConnections,
    DoRetry,
    InitialRandom,
    ResetSeed,
    DisablePortBlocking,
    SolutionDir,
    CcAlgo,
    CcOption,
    Spinbit,
    Lossbit,
    Multipath,
    DestIf,
    CipherSuite,
    InitCnxId,
    LogFile,
    LongLog,
    BinlogDir,
    QlogDir,
    MtuMax,
    Sni,
    Alpn,
    RootTrustFile,
    ForceZeroShare,
    CnxIdLength,
    NoDisk,
    IdleTimeout,
    LargeClientHello,
    TicketFileName,
    TokenFileName,
    SocketBufferSize,
    PerformanceLog,
    PreemptiveRepeat,
    VersionUpgrade,
    NoGso,
    BdpFrame,
    CwinMax,
    SslKeyLog,
    AddressDiscovery,
    EchServer,
    EchClient,
    EchInit,
    FlowControlMax,
    PreferredV4,
    PreferredV6,
    Help,
}

// ---------------------------------------------------------------------------
// Configuration struct.

/// Configuration bag for a QUIC context, populated from CLI flags
/// (via [`Config::command_line`]) or directly (via
/// [`Config::set_option`]) and consumed by
/// [`Config::create_and_configure`].
///
/// Mirrors the C `picoquic_quic_config_t` field-for-field;
/// `repr(C)` is dropped because the struct never crosses an
/// external boundary.  See the module-level docs for the
/// pointer-shape rationale.
///
/// `Default` produces the documented C defaults:
/// `nb_connections = 256`, `connection_id_length = -1`, `cwin_max =
/// u64::MAX`, idle timeout from `MICROSEC_HANDSHAKE_MAX`, etc.  This
/// folds the C `picoquic_config_init` step into the constructor —
/// the C `config_clear` (which `free`s every owned string) collapses
/// to `Drop`, and resetting a config to defaults is just
/// `*config = Config::default()`.
#[derive(Debug)]
pub struct Config {
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
    /// Whether the public port is shared with sibling threads
    /// (`SO_REUSEPORT`).  C: `int is_port_shared`, used as a
    /// Boolean flag.
    pub is_port_shared: bool,
    pub nb_threads: i32,
    pub dest_if: i32,
    pub mtu_max: i32,
    /// `-1` is the C "unset" sentinel applied by [`Self::init`];
    /// values `>= 0` set the connection-ID length explicitly.  Kept
    /// as `i32` rather than `Option<u8>` for source-level parity
    /// with the C body.
    pub connection_id_length: i32,
    pub idle_timeout: i32,
    pub socket_buffer_size: i32,
    pub cc_algo_id: Option<String>,
    pub cc_algo_option_string: Option<String>,
    pub connection_id_cbdata: Option<String>,
    pub spinbit_policy: SpinbitVersion,
    pub lossbit_policy: LossbitVersion,
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
    /// Borrowed in C (`config_clear` does not free it); owned
    /// `Vec<u8>` here for safety.  The C
    /// `ticket_encryption_key_length` field is dropped — its value
    /// is always `ticket_encryption_key.as_ref().map_or(0, Vec::len)`.
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

impl Default for Config {
    /// Build a fresh config with the documented C defaults.  C:
    /// `picoquic_config_init` (which the C body invoked after a
    /// zero-initialising `memset`).
    fn default() -> Self {
        todo!()
    }
}

impl Config {
    /// Apply one option, selected by `option`, to the config.
    /// C: `picoquic_config_set_option`.
    ///
    /// `value` is optional — flag-style options ([`OptionId::DoRetry`],
    /// [`OptionId::LongLog`], …) ignore it; value-style options
    /// require it and return `Err` when it is missing or malformed.
    ///
    /// The C signature returned `int` (`0` ↔ `Ok`, `-1` ↔ `Err`);
    /// mapped to `Result<(), Error>`.
    pub fn set_option(&mut self, _option: OptionId, _value: Option<&str>) -> Result<(), Error> {
        todo!()
    }

    /// Dispatch one option from a single-character flag (`-x`).
    /// C: `picoquic_config_command_line`.
    ///
    /// `p_optind` advances past any extra arguments consumed beyond
    /// the inline `optarg`, mirroring the C in/out parameter so that
    /// an outer getopt-style loop stays in sync.
    ///
    /// `argv` is a borrowed slice of borrowed strings — the C caller
    /// (`first` etc.) owns the argument vector for the program
    /// lifetime, so a borrow is safe.  The C `int argc` parameter is
    /// dropped (implicit in `argv.len()`).
    pub fn command_line(
        &mut self,
        _opt: char,
        _p_optind: &mut usize,
        _argv: &[&str],
        _optarg: Option<&str>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Like [`Self::command_line`] but accepts both single-character
    /// (`-x`) and long-form (`--name`) option strings; the leading
    /// dashes are part of `opt_string`, matching the C contract.
    /// C: `picoquic_config_command_line_ex`.
    pub fn command_line_ex(
        &mut self,
        _opt_string: &str,
        _p_optind: &mut usize,
        _argv: &[&str],
        _optarg: Option<&str>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Build the getopt-style option string from the dispatch table
    /// in `config.c` — one letter per option, with `:` after each
    /// option that takes an argument.  C:
    /// `picoquic_config_option_letters`.
    ///
    /// The C signature wrote into a caller-supplied buffer
    /// (`option_string`, `string_max`) and reported the populated
    /// length via `*string_length`.  The Rust wrapper owns the
    /// buffer and returns it directly, so the buffer-too-small
    /// failure mode disappears.
    pub fn option_letters() -> String {
        todo!()
    }

    /// Write the option help to a [`core::fmt::Write`] sink.
    /// C: `picoquic_config_usage_file`.
    ///
    /// The C parameter was `FILE*`; the Rust translation accepts
    /// any sink (a `String` buffer, the stdout/stderr handles under
    /// the `std` feature, or a custom writer) so the function
    /// stays `no_std`-friendly.
    pub fn write_usage(_w: &mut dyn core::fmt::Write) {
        todo!()
    }

    /// Print the option help to stderr.  C: `picoquic_config_usage`.
    ///
    /// Phase 3 will route this through `eprintln!` (std-only); for
    /// now it is just a `todo!()`.
    pub fn print_usage() {
        todo!()
    }

    /// Build a fully-configured QUIC context from this config plus
    /// an application-supplied callback.  C:
    /// `picoquic_create_and_configure`.
    ///
    /// Returns `None` when context creation fails (the C side
    /// returned `NULL`).
    ///
    /// Pointer-shape choices, derived from the C signature and the
    /// body in `config.c`:
    ///
    /// * `picoquic_quic_config_t* config` is consumed-borrowed
    ///   (`&mut self`): the body reads every field and may set up
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
    ///   so an `&mut u64` borrow expresses the contract directly.
    ///   Phase 3 may revisit (e.g. with a clock trait) once the QUIC
    ///   context type is real.
    pub fn create_and_configure(
        &mut self,
        _default_callback: Option<Box<dyn StreamDataCb>>,
        _current_time: Instant,
        _p_simulated_time: Option<&mut u64>,
    ) -> Option<Box<Quic>> {
        todo!()
    }
}

#[cfg(test)]
mod test {}
