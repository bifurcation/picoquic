//! Translation of `quic/config.h`.
//!
//! Demo-application–facing configuration plumbing for quic-core:
//! the [`Config`] bag of CLI-derived options, the [`OptionId`]
//! tag identifying each option, and [`Config::create_and_configure`],
//! the one-shot constructor that turns a populated config into a
//! fully-wired [`Quic`].
//!
//! Phase 4: all function bodies are implemented.
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
use crate::internal::MICROSEC_HANDSHAKE_MAX;
use crate::{LossbitVersion, Quic, SpinbitVersion, StreamDataCallback};

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
// Option table (mirrors C static option_table[]).

struct OptionEntry {
    id: OptionId,
    letter: char,
    name: &'static str,
    nb_params: usize,
    param_sample: &'static str,
    help: &'static str,
}

static OPTION_TABLE: &[OptionEntry] = &[
    OptionEntry {
        id: OptionId::Cert,
        letter: 'c',
        name: "cert",
        nb_params: 1,
        param_sample: "file",
        help: "cert file",
    },
    OptionEntry {
        id: OptionId::Key,
        letter: 'k',
        name: "key",
        nb_params: 1,
        param_sample: "file",
        help: "key file",
    },
    OptionEntry {
        id: OptionId::ServerPort,
        letter: 'p',
        name: "port",
        nb_params: 1,
        param_sample: "number",
        help: "server port",
    },
    OptionEntry {
        id: OptionId::ProposedVersion,
        letter: 'v',
        name: "proposed_version",
        nb_params: 1,
        param_sample: "",
        help: "Version proposed by client, e.g. -v ff000012",
    },
    OptionEntry {
        id: OptionId::OutDir,
        letter: 'o',
        name: "outdir",
        nb_params: 1,
        param_sample: "folder",
        help: "Folder where client writes downloaded files, defaults to current directory.",
    },
    OptionEntry {
        id: OptionId::WwwDir,
        letter: 'w',
        name: "wwwdir",
        nb_params: 1,
        param_sample: "folder",
        help: "Folder containing web pages served by server",
    },
    OptionEntry {
        id: OptionId::MaxConnections,
        letter: 'x',
        name: "max_connections",
        nb_params: 1,
        param_sample: "number",
        help: "Maximum number of concurrent connections, default 256",
    },
    OptionEntry {
        id: OptionId::DoRetry,
        letter: 'r',
        name: "do_retry",
        nb_params: 0,
        param_sample: "",
        help: "Do Retry Request",
    },
    OptionEntry {
        id: OptionId::InitialRandom,
        letter: 'R',
        name: "initial_random",
        nb_params: 1,
        param_sample: "option",
        help: "Randomize packet number spaces: none(0), initial(1, default), all(2).",
    },
    OptionEntry {
        id: OptionId::ResetSeed,
        letter: 's',
        name: "reset_seed",
        nb_params: 1,
        param_sample: "<32 hex chars>",
        help: "Reset seed",
    },
    OptionEntry {
        id: OptionId::DisablePortBlocking,
        letter: 'X',
        name: "disable_block",
        nb_params: 0,
        param_sample: "",
        help: "Disable the check for blocked ports",
    },
    OptionEntry {
        id: OptionId::SolutionDir,
        letter: 'S',
        name: "solution_dir",
        nb_params: 1,
        param_sample: "folder",
        help: "Set the path to the source files to find the default files",
    },
    OptionEntry {
        id: OptionId::CcAlgo,
        letter: 'G',
        name: "cc_algo",
        nb_params: 1,
        param_sample: "cc_algorithm",
        help: "Use the specified congestion control algorithm. Defaults to bbr.",
    },
    OptionEntry {
        id: OptionId::CcOption,
        letter: 'H',
        name: "cco",
        nb_params: 1,
        param_sample: "option",
        help: "Set option string if required by congestion control algorithm.",
    },
    OptionEntry {
        id: OptionId::Spinbit,
        letter: 'P',
        name: "spinbit",
        nb_params: 1,
        param_sample: "number",
        help: "Set the default spinbit policy",
    },
    OptionEntry {
        id: OptionId::Lossbit,
        letter: 'O',
        name: "lossbit",
        nb_params: 1,
        param_sample: "number",
        help: "Set the default lossbit policy",
    },
    OptionEntry {
        id: OptionId::Multipath,
        letter: 'M',
        name: "multipath",
        nb_params: 0,
        param_sample: "",
        help: "Enable QUIC multipath extension",
    },
    OptionEntry {
        id: OptionId::DestIf,
        letter: 'e',
        name: "dest_if",
        nb_params: 1,
        param_sample: "if",
        help: "Send on interface (default: -1)",
    },
    OptionEntry {
        id: OptionId::CipherSuite,
        letter: 'C',
        name: "cipher_suite",
        nb_params: 1,
        param_sample: "cipher_suite_id",
        help: "specify cipher suite (e.g. -C 20 = chacha20)",
    },
    OptionEntry {
        id: OptionId::InitCnxId,
        letter: 'i',
        name: "cnxid_params",
        nb_params: 1,
        param_sample: "per-text-lb-spec",
        help: "See documentation for LB compatible CID configuration",
    },
    OptionEntry {
        id: OptionId::LogFile,
        letter: 'l',
        name: "text_log",
        nb_params: 1,
        param_sample: "file",
        help: "Log file, Log to stdout if file = \"-\". No text logging if absent.",
    },
    OptionEntry {
        id: OptionId::LongLog,
        letter: 'L',
        name: "long_log",
        nb_params: 0,
        param_sample: "",
        help: "Log all packets. If absent, log stops after 100 packets.",
    },
    OptionEntry {
        id: OptionId::BinlogDir,
        letter: 'b',
        name: "binlog_dir",
        nb_params: 1,
        param_sample: "folder",
        help: "Binary logging to this directory. No binary logging if absent.",
    },
    OptionEntry {
        id: OptionId::QlogDir,
        letter: 'q',
        name: "qlog_dir",
        nb_params: 1,
        param_sample: "folder",
        help: "Qlog logging to this directory.",
    },
    OptionEntry {
        id: OptionId::MtuMax,
        letter: 'm',
        name: "mtu_max",
        nb_params: 1,
        param_sample: "mtu_max",
        help: "Largest mtu value that can be tried for discovery.",
    },
    OptionEntry {
        id: OptionId::Sni,
        letter: 'n',
        name: "sni",
        nb_params: 1,
        param_sample: "sni",
        help: "sni (default: server name)",
    },
    OptionEntry {
        id: OptionId::Alpn,
        letter: 'a',
        name: "alpn",
        nb_params: 1,
        param_sample: "alpn",
        help: "alpn (default function of version)",
    },
    OptionEntry {
        id: OptionId::RootTrustFile,
        letter: 't',
        name: "root_trust_file",
        nb_params: 1,
        param_sample: "file",
        help: "root trust file",
    },
    OptionEntry {
        id: OptionId::ForceZeroShare,
        letter: 'z',
        name: "force_zero_share",
        nb_params: 0,
        param_sample: "",
        help: "Set TLS zero share behavior on client, to force HRR",
    },
    OptionEntry {
        id: OptionId::CnxIdLength,
        letter: 'I',
        name: "cnxid_length",
        nb_params: 1,
        param_sample: "length",
        help: "Length of CNX_ID used by the client, default=8",
    },
    OptionEntry {
        id: OptionId::IdleTimeout,
        letter: 'd',
        name: "idle_timeout",
        nb_params: 1,
        param_sample: "ms",
        help: "Duration of idle timeout in milliseconds (Default 30,000ms)",
    },
    OptionEntry {
        id: OptionId::NoDisk,
        letter: 'D',
        name: "no_disk",
        nb_params: 0,
        param_sample: "",
        help: "no disk: do not save received files on disk",
    },
    OptionEntry {
        id: OptionId::LargeClientHello,
        letter: 'Q',
        name: "large_client_hello",
        nb_params: 0,
        param_sample: "",
        help: "send a large client hello in order to test post quantum readiness",
    },
    OptionEntry {
        id: OptionId::TicketFileName,
        letter: 'T',
        name: "ticket_file",
        nb_params: 1,
        param_sample: "file",
        help: "File storing the session tickets",
    },
    OptionEntry {
        id: OptionId::TokenFileName,
        letter: 'N',
        name: "token_file",
        nb_params: 1,
        param_sample: "file",
        help: "File storing the new tokens",
    },
    OptionEntry {
        id: OptionId::SocketBufferSize,
        letter: 'B',
        name: "so_buf_size",
        nb_params: 1,
        param_sample: "number",
        help: "Set buffer size with SO_SNDBUF SO_RCVBUF",
    },
    OptionEntry {
        id: OptionId::PerformanceLog,
        letter: 'F',
        name: "log_file_name",
        nb_params: 1,
        param_sample: "file",
        help: "Append performance reports to performance log",
    },
    OptionEntry {
        id: OptionId::PreemptiveRepeat,
        letter: 'V',
        name: "preemptive_repeat",
        nb_params: 0,
        param_sample: "",
        help: "enable preemptive repeat",
    },
    OptionEntry {
        id: OptionId::VersionUpgrade,
        letter: 'U',
        name: "version_upgrade",
        nb_params: 1,
        param_sample: "",
        help: "Version upgrade if server agrees, e.g. -U 6b3343cf",
    },
    OptionEntry {
        id: OptionId::NoGso,
        letter: '0',
        name: "no_gso",
        nb_params: 0,
        param_sample: "",
        help: "Do not use UDP GSO or equivalent",
    },
    OptionEntry {
        id: OptionId::BdpFrame,
        letter: 'j',
        name: "bdp",
        nb_params: 1,
        param_sample: "number",
        help: "use bdp extension frame(1) or don't (0). Default=0",
    },
    OptionEntry {
        id: OptionId::CwinMax,
        letter: 'W',
        name: "cwin_max",
        nb_params: 1,
        param_sample: "bytes",
        help: "Max value for CWIN. Default=UINT64_MAX",
    },
    OptionEntry {
        id: OptionId::SslKeyLog,
        letter: '8',
        name: "sslkeylog",
        nb_params: 0,
        param_sample: "",
        help: "Enable SSLKEYLOG",
    },
    OptionEntry {
        id: OptionId::AddressDiscovery,
        letter: 'J',
        name: "addr_disc",
        nb_params: 1,
        param_sample: "mode",
        help: "provider (0), receiver (1) or both (2).",
    },
    OptionEntry {
        id: OptionId::EchServer,
        letter: 'E',
        name: "ech_s",
        nb_params: 2,
        param_sample: "key config",
        help: "ECH private key file, config file. Default= no ECH on server.",
    },
    OptionEntry {
        id: OptionId::EchInit,
        letter: 'y',
        name: "ech_init",
        nb_params: 1,
        param_sample: "public_name",
        help: "Create an ECH configuration before applying the `ech_s` parameter.",
    },
    OptionEntry {
        id: OptionId::EchClient,
        letter: 'K',
        name: "ech_c",
        nb_params: 1,
        param_sample: "base64",
        help: "ECH configuration for the client connection, base64 encoded.",
    },
    OptionEntry {
        id: OptionId::FlowControlMax,
        letter: 'Z',
        name: "flow_control_max",
        nb_params: 1,
        param_sample: "bytes",
        help: "Set the flow control's initial max data.",
    },
    OptionEntry {
        id: OptionId::PreferredV4,
        letter: '4',
        name: "preferred_v4",
        nb_params: 1,
        param_sample: "ip[:port]",
        help: "Preferred address for v4 connections.",
    },
    OptionEntry {
        id: OptionId::PreferredV6,
        letter: '6',
        name: "preferred_v6",
        nb_params: 1,
        param_sample: "ipv6[:port]",
        help: "Preferred address for v6 connections.",
    },
    OptionEntry {
        id: OptionId::Help,
        letter: 'h',
        name: "help",
        nb_params: 0,
        param_sample: "",
        help: "This help message",
    },
];

/// C: `config_parse_target_version` (picoquic/config.c:116)
///
/// Parses a hex-encoded QUIC version number.  Returns 0 on any invalid input,
/// matching the C behaviour of breaking on the first unrecognised character.
fn parse_hex_version(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap_or(0)
}

/// C: `config_optval_string` (picoquic/config.c:181)
///
/// Copies at most `buffer.len() - 1` bytes from `p` into `buffer`,
/// null-terminates, and returns the filled prefix as a `str`.
/// Matches the C truncation-and-NUL-terminate contract exactly.
fn config_optval_string<'a>(buffer: &'a mut [u8], p: &[u8]) -> &'a str {
    let len = p.len().min(buffer.len().saturating_sub(1));
    buffer[..len].copy_from_slice(&p[..len]);
    if !buffer.is_empty() {
        buffer[len] = 0;
    }
    core::str::from_utf8(&buffer[..len]).unwrap_or("")
}

/// C: `config_optval_param_string` (picoquic/config.c:191)
///
/// Bounds-checked wrapper around [`config_optval_string`]: copies
/// `params[x]` into `buffer` (NUL-terminated, truncated to fit) and
/// returns the filled prefix.  Writes a single NUL and returns `""` when
/// `x` is out of bounds — matching the C behaviour for `params == NULL`
/// or `x < 0 || x >= nb_param`.
///
/// In the C implementation this helper exists because `option_param_t`
/// carries a raw pointer and length that need explicit bounds checking.
/// The Rust `&[&str]` slice already encodes the length, so the only
/// real work is the `x >= params.len()` guard.
#[allow(dead_code)]
fn config_optval_param_string<'a>(buffer: &'a mut [u8], params: &[&str], x: usize) -> &'a str {
    if x >= params.len() {
        if !buffer.is_empty() {
            buffer[0] = 0;
        }
        return "";
    }
    config_optval_string(buffer, params[x].as_bytes())
}

/// C: `config_atoi` (picoquic/config.c:202)
///
/// Parse a decimal (ASCII digit only) integer from `params[x]`.
///
/// Returns `Ok(value)` on success.  Returns `Err(InvalidArgument)` when `x`
/// is out of bounds or any character in `params[x]` is not an ASCII digit —
/// mirroring the C behaviour of setting `*ret = -1` and returning `-1` on error.
pub fn config_atoi(params: &[&str], x: usize) -> Result<i32, Error> {
    if x >= params.len() {
        return Err(Error::InvalidArgument);
    }
    let mut v: i32 = 0;
    for b in params[x].bytes() {
        let digit = b.wrapping_sub(b'0');
        if digit > 9 {
            return Err(Error::InvalidArgument);
        }
        v = v * 10 + digit as i32;
    }
    Ok(v)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, Error> {
    let mut table = [0xFFu8; 256];
    for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .iter()
        .enumerate()
    {
        table[c as usize] = i as u8;
    }
    let bytes: Vec<u8> = s.bytes().filter(|&c| c != b'=').collect();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let mut v = [0u8; 4];
        let n = chunk.len();
        for (i, &b) in chunk.iter().enumerate() {
            let x = table[b as usize];
            if x == 0xFF {
                return Err(Error::InvalidArgument);
            }
            v[i] = x;
        }
        out.push((v[0] << 2) | (v[1] >> 4));
        if n >= 3 {
            out.push((v[1] << 4) | (v[2] >> 2));
        }
        if n == 4 {
            out.push((v[2] << 6) | v[3]);
        }
    }
    Ok(out)
}

fn option_entry(id: OptionId) -> Option<&'static OptionEntry> {
    OPTION_TABLE.iter().find(|e| e.id == id)
}

fn option_entry_by_letter(letter: char) -> Option<(usize, &'static OptionEntry)> {
    OPTION_TABLE
        .iter()
        .enumerate()
        .find(|(_, e)| e.letter == letter)
}

fn option_entry_by_name(name: &str) -> Option<(usize, &'static OptionEntry)> {
    OPTION_TABLE
        .iter()
        .enumerate()
        .find(|(_, e)| e.name == name)
}

/// C: `config_set_string_param` (picoquic/config.c:148)
///
/// Replace `*v` with an owned copy of `params[x]`.  Clears `*v` first
/// (mirrors the `free(*v)` in C), then sets it if the selected param
/// is non-empty.  Returns `Err(InvalidArgument)` when `x` is out of
/// bounds or the param is empty (the C path where `length == 0` causes
/// `malloc` to be skipped and `-1` is returned).
#[allow(dead_code)]
fn config_set_string_param(v: &mut Option<String>, params: &[&str], x: usize) -> Result<(), Error> {
    *v = None;
    if x < params.len() && !params[x].is_empty() {
        *v = Some(params[x].to_string());
        Ok(())
    } else {
        Err(Error::InvalidArgument)
    }
}

/// C: `picoquic_config_get_option_char_index` (picoquic/config.c:641)
///
/// Return the index in `OPTION_TABLE` of the entry whose single-character
/// flag equals `opt`, or `-1` if not found.
pub fn picoquic_config_get_option_char_index(opt: char) -> i32 {
    OPTION_TABLE
        .iter()
        .position(|e| e.letter == opt)
        .map_or(-1, |i| i as i32)
}

/// C: `picoquic_config_get_option_name_index` (picoquic/config.c:654)
///
/// Return the index in `OPTION_TABLE` of the first entry whose long name
/// matches the first `l` bytes of `s` (mirrors `strncmp(s, name, l) == 0`),
/// or `-1` if not found.
pub fn picoquic_config_get_option_name_index(s: &str, l: usize) -> i32 {
    let l = l.min(s.len());
    let prefix = &s[..l];
    OPTION_TABLE
        .iter()
        .position(|e| e.name.len() >= l && &e.name[..l] == prefix)
        .map_or(-1, |i| i as i32)
}

/// C: `picoquic_config_get_command_line_option_index` (picoquic/config.c:667)
///
/// Inspect `opt_string` and return the `OPTION_TABLE` index:
/// * `-x` (exactly two bytes, second is the flag letter) → char-index lookup.
/// * `--name` (starts with `--`, at least one char after) → name-index lookup.
/// * Anything else → `-1`.
pub fn picoquic_config_get_command_line_option_index(opt_string: &str) -> i32 {
    parse_option_string(opt_string).map_or(-1, |(i, _)| i as i32)
}

/// C: `picoquic_get_command_line_option_value` (picoquic/config.c:683)
///
/// Collect the value(s) required by `OPTION_TABLE[option_index]`, advancing
/// `p_optind` for extra argv entries exactly like the C helper, then apply the
/// option to `config`.
pub fn picoquic_get_command_line_option_value(
    option_index: i32,
    _opt_string: &str,
    p_optind: &mut usize,
    argv: &[&str],
    argc: usize,
    optarg: Option<&str>,
    config: &mut Config,
) -> Result<(), Error> {
    let option_index = usize::try_from(option_index).map_err(|_| Error::InvalidArgument)?;
    let entry = OPTION_TABLE
        .get(option_index)
        .ok_or(Error::InvalidArgument)?;
    let argc = argc.min(argv.len());
    let mut params = Vec::new();

    if entry.nb_params > 0 {
        params.push(optarg.ok_or(Error::InvalidArgument)?);
        while params.len() < entry.nb_params {
            if *p_optind >= argc {
                return Err(Error::InvalidArgument);
            }
            params.push(argv[*p_optind]);
            *p_optind += 1;
        }
    }

    apply_option(config, entry, &params)
}

/// C: `config_set_option` (picoquic/config.c:274)
///
/// Applies a single option — identified by `entry.id` — to `config`.  Each
/// arm mirrors the corresponding `case` in the C `switch (option_desc->option_num)`.
/// Error conditions that C prints to `stderr` via `config_optval_param_string`
/// are surfaced as `Err(Error::InvalidArgument)` instead; the C `int` return
/// (`0` / `-1`) maps to `Ok(())` / `Err`.
fn apply_option(config: &mut Config, entry: &OptionEntry, params: &[&str]) -> Result<(), Error> {
    let p0 = params.first().copied();
    let p1 = params.get(1).copied();
    match entry.id {
        OptionId::Cert => {
            config.server_cert_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::Key => {
            config.server_key_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::ServerPort => {
            config.set_port(p0.ok_or(Error::InvalidArgument)?)?;
        }
        OptionId::ProposedVersion => {
            let v = parse_hex_version(p0.ok_or(Error::InvalidArgument)?);
            if v == 0 {
                return Err(Error::InvalidArgument);
            }
            config.proposed_version = v;
        }
        OptionId::OutDir => {
            config.out_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::WwwDir => {
            config.www_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::MaxConnections => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v <= 0 {
                return Err(Error::InvalidArgument);
            }
            config.nb_connections = v as u32;
        }
        OptionId::DoRetry => {
            config.do_retry = true;
        }
        OptionId::InitialRandom => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if !(0..=2).contains(&v) {
                return Err(Error::InvalidArgument);
            }
            config.initial_random = v as u32;
        }
        OptionId::ResetSeed => {
            config.has_reset_seed = true;
            let n =
                crate::utils::parse_hexa(p0.ok_or(Error::InvalidArgument)?, &mut config.reset_seed);
            if n != config.reset_seed.len() {
                return Err(Error::InvalidArgument);
            }
        }
        OptionId::DisablePortBlocking => {
            config.disable_port_blocking = true;
        }
        OptionId::SslKeyLog => {
            config.enable_sslkeylog = true;
        }
        OptionId::SolutionDir => {
            config.solution_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::CcAlgo => {
            config.cc_algo_id = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::CcOption => {
            config.cc_algo_option_string = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::Spinbit => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            config.spinbit_policy = match v {
                0 => SpinbitVersion::Basic,
                1 => SpinbitVersion::Random,
                2 => SpinbitVersion::Null,
                3 => SpinbitVersion::On,
                _ => return Err(Error::InvalidArgument),
            };
        }
        OptionId::Lossbit => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            config.lossbit_policy = match v {
                0 => LossbitVersion::None,
                1 => LossbitVersion::SendOnly,
                2 => LossbitVersion::SendReceive,
                _ => return Err(Error::InvalidArgument),
            };
        }
        OptionId::Multipath => {
            config.multipath_option = 1;
        }
        OptionId::DestIf => {
            config.dest_if = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
        }
        OptionId::CipherSuite => {
            config.cipher_suite_id = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
        }
        OptionId::InitCnxId => {
            config.connection_id_cbdata = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::LogFile => {
            config.log_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::LongLog => {
            config.use_long_log = true;
        }
        OptionId::BinlogDir => {
            config.bin_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::QlogDir => {
            config.qlog_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::MtuMax => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v <= 0 || v > crate::internal::MAX_PACKET_SIZE as i32 {
                return Err(Error::InvalidArgument);
            }
            config.mtu_max = v;
        }
        OptionId::Sni => {
            config.sni = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::Alpn => {
            config.alpn = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::RootTrustFile => {
            config.root_trust_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::ForceZeroShare => {
            config.force_zero_share = true;
        }
        OptionId::CnxIdLength => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 || v > crate::CONNECTION_ID_MAX_SIZE as i32 {
                return Err(Error::InvalidArgument);
            }
            config.connection_id_length = v;
        }
        OptionId::IdleTimeout => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 {
                return Err(Error::InvalidArgument);
            }
            config.idle_timeout = v;
        }
        OptionId::NoDisk => {
            config.no_disk = true;
        }
        OptionId::LargeClientHello => {
            config.large_client_hello = true;
        }
        OptionId::TicketFileName => {
            config.ticket_file_name = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::TokenFileName => {
            config.token_file_name = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::SocketBufferSize => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 {
                return Err(Error::InvalidArgument);
            }
            config.socket_buffer_size = v;
        }
        OptionId::PerformanceLog => {
            config.performance_log = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::PreemptiveRepeat => {
            config.do_preemptive_repeat = true;
        }
        OptionId::VersionUpgrade => {
            let v = parse_hex_version(p0.ok_or(Error::InvalidArgument)?);
            if v == 0 {
                return Err(Error::InvalidArgument);
            }
            config.desired_version = v;
        }
        OptionId::NoGso => {
            config.do_not_use_gso = true;
        }
        OptionId::BdpFrame => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if !(0..=1).contains(&v) {
                return Err(Error::InvalidArgument);
            }
            config.bdp_frame_option = v;
        }
        OptionId::CwinMax => {
            let v: i64 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 {
                return Err(Error::InvalidArgument);
            }
            config.cwin_max = if v == 0 { u64::MAX } else { v as u64 };
        }
        OptionId::AddressDiscovery => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if !(0..=2).contains(&v) {
                return Err(Error::InvalidArgument);
            }
            config.address_discovery_mode = v + 1;
        }
        OptionId::EchServer => {
            config.ech_key_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
            if let Some(s) = p1 {
                config.ech_config_file = Some(s.to_string());
            }
        }
        OptionId::EchInit => {
            config.ech_public_name = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::EchClient => {
            let s = p0.ok_or(Error::InvalidArgument)?;
            if s == "-" {
                config.ech_target = None;
            } else {
                config.ech_target = Some(base64_decode(s).map_err(|_| Error::InvalidArgument)?);
            }
        }
        OptionId::FlowControlMax => {
            let v: i64 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 {
                return Err(Error::InvalidArgument);
            }
            config.flow_control_max = v as u64;
        }
        OptionId::PreferredV4 => {
            config.preferred_address_v4 = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::PreferredV6 => {
            config.preferred_address_v6 = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::Help => {
            return Err(Error::InvalidArgument);
        }
    }
    Ok(())
}

fn collect_params<'a>(
    entry: &OptionEntry,
    p_optind: &mut usize,
    argv: &[&'a str],
    optarg: Option<&'a str>,
) -> Result<Vec<&'a str>, Error> {
    let mut params = Vec::new();
    if entry.nb_params > 0 {
        params.push(optarg.ok_or(Error::InvalidArgument)?);
        let mut nb = 1;
        while nb < entry.nb_params {
            if *p_optind >= argv.len() {
                return Err(Error::InvalidArgument);
            }
            params.push(argv[*p_optind]);
            *p_optind += 1;
            nb += 1;
        }
    }
    Ok(params)
}

fn parse_option_string(opt_string: &str) -> Option<(usize, &'static OptionEntry)> {
    if opt_string.starts_with("--") && opt_string.len() > 2 {
        option_entry_by_name(&opt_string[2..])
    } else if opt_string.starts_with('-') && opt_string.len() == 2 {
        opt_string.chars().nth(1).and_then(option_entry_by_letter)
    } else {
        None
    }
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
        Config {
            nb_connections: 256,
            solution_dir: None,
            server_cert_file: None,
            server_key_file: None,
            log_file: None,
            bin_dir: None,
            qlog_dir: None,
            performance_log: None,
            server_port: 0,
            local_port: 0,
            is_port_shared: false,
            nb_threads: 0,
            dest_if: 0,
            mtu_max: 0,
            connection_id_length: -1,
            idle_timeout: (MICROSEC_HANDSHAKE_MAX.ticks() / 1000) as i32,
            socket_buffer_size: 0,
            cc_algo_id: None,
            cc_algo_option_string: None,
            connection_id_cbdata: None,
            spinbit_policy: SpinbitVersion::default(),
            lossbit_policy: LossbitVersion::default(),
            multipath_option: 0,
            multipath_alt_config: None,
            bdp_frame_option: 0,
            cwin_max: u64::MAX,
            address_discovery_mode: 0,
            initial_random: 3,
            use_long_log: false,
            do_preemptive_repeat: false,
            do_not_use_gso: false,
            disable_port_blocking: false,
            enable_sslkeylog: false,
            www_dir: None,
            reset_seed: [0; 16],
            ticket_encryption_key: None,
            do_retry: false,
            has_reset_seed: false,
            ticket_file_name: None,
            token_file_name: None,
            sni: None,
            alpn: None,
            out_dir: None,
            root_trust_file: None,
            cipher_suite_id: 0,
            proposed_version: 0,
            desired_version: 0,
            force_zero_share: false,
            no_disk: false,
            large_client_hello: false,
            ech_key_file: None,
            ech_config_file: None,
            ech_public_name: None,
            ech_target: None,
            flow_control_max: 0,
            preferred_address_v4: None,
            preferred_address_v6: None,
        }
    }
}

impl Config {
    /// C: `picoquic_config_clear` (picoquic/config.c:1046)
    ///
    /// Frees all owned fields and resets every member to the defaults that
    /// [`Config::default`] produces.  In the C version each `const char*`
    /// field is individually `free`d and then `picoquic_config_init` is
    /// called; in Rust the old values are dropped automatically when the
    /// struct is overwritten, so the entire operation collapses to a single
    /// assignment.
    pub fn clear(&mut self) {
        *self = Config::default();
    }

    /// Apply one option, selected by `option`, to the config.
    /// C: `picoquic_config_set_option`.
    ///
    /// `value` is optional — flag-style options ([`OptionId::DoRetry`],
    /// [`OptionId::LongLog`], …) ignore it; value-style options
    /// require it and return `Err` when it is missing or malformed.
    ///
    /// The C signature returned `int` (`0` ↔ `Ok`, `-1` ↔ `Err`);
    /// mapped to `Result<(), Error>`.
    pub fn set_option(&mut self, option: OptionId, value: Option<&str>) -> Result<(), Error> {
        let entry = option_entry(option).ok_or(Error::InvalidArgument)?;
        let params: &[&str] = match value {
            Some(v) => &[v],
            None => &[],
        };
        apply_option(self, entry, params)
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
        opt: char,
        p_optind: &mut usize,
        argv: &[&str],
        optarg: Option<&str>,
    ) -> Result<(), Error> {
        let (_, entry) = option_entry_by_letter(opt).ok_or(Error::InvalidArgument)?;
        let params = collect_params(entry, p_optind, argv, optarg)?;
        apply_option(self, entry, &params)
    }

    /// Like [`Self::command_line`] but accepts both single-character
    /// (`-x`) and long-form (`--name`) option strings; the leading
    /// dashes are part of `opt_string`, matching the C contract.
    /// C: `picoquic_config_command_line_ex`.
    pub fn command_line_ex(
        &mut self,
        opt_string: &str,
        p_optind: &mut usize,
        argv: &[&str],
        optarg: Option<&str>,
    ) -> Result<(), Error> {
        let (_, entry) = parse_option_string(opt_string).ok_or(Error::InvalidArgument)?;
        let params = collect_params(entry, p_optind, argv, optarg)?;
        apply_option(self, entry, &params)
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
        let mut s = String::new();
        for e in OPTION_TABLE {
            s.push(e.letter);
            if e.nb_params > 0 {
                s.push(':');
            }
        }
        s
    }

    /// Write the option help to a [`core::fmt::Write`] sink.
    /// C: `picoquic_config_usage_file`.
    ///
    /// The C parameter was `FILE*`; the Rust translation accepts
    /// any sink (a `String` buffer, the stdout/stderr handles under
    /// the `std` feature, or a custom writer) so the function
    /// stays `no_std`-friendly.
    pub fn write_usage(w: &mut dyn core::fmt::Write) {
        let _ = w.write_str("Picoquic options:\n");
        for e in OPTION_TABLE {
            let _ = write!(w, "  -{} {}", e.letter, e.param_sample);
            let pad = 12usize.saturating_sub(e.param_sample.len());
            for _ in 0..pad {
                let _ = w.write_char(' ');
            }
            let _ = writeln!(w, " {}", e.help);
        }
    }

    /// Print the option help to stderr.  C: `picoquic_config_usage`.
    ///
    /// Routes through an intermediate `String` buffer then `eprint!`.
    pub fn print_usage() {
        let mut buf = String::new();
        Self::write_usage(&mut buf);
        eprint!("{}", buf);
    }

    /// Parse a port-configuration string into `server_port`,
    /// `local_port`, `is_port_shared`, and `nb_threads`.
    /// C: `config_set_port` (internal, called from the `-p` option
    /// handler in `picoquic/config.c`).
    ///
    /// Accepted formats:
    /// - `""` / `"0"` — clear all (port 0, not shared)
    /// - `"4433"` — server port only
    /// - `"443:4434"` — server:local
    /// - `"S4433"` — shared server port
    /// - `"S443:4434*256"` — shared server:local, 256 threads
    /// - `"4433*7"` — server port, 7 threads
    ///
    /// Returns `Err` when any token cannot be parsed or a port value
    /// exceeds 65535.
    pub fn set_port(&mut self, port_string: &str) -> Result<(), Error> {
        let bytes = port_string.as_bytes();
        let mut i = 0;
        let mut is_port_shared = false;
        let mut p1: i32 = 0;
        let mut p2: i32 = 0;
        let mut nb_threads: i32 = 0;

        if i < bytes.len() && bytes[i] == b'S' {
            is_port_shared = true;
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            p1 = p1 * 10 + (bytes[i] - b'0') as i32;
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b':' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                p2 = p2 * 10 + (bytes[i] - b'0') as i32;
                i += 1;
            }
        }
        if i < bytes.len() && bytes[i] == b'*' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                nb_threads = nb_threads * 10 + (bytes[i] - b'0') as i32;
                i += 1;
            }
        }
        if i != bytes.len() || !(0..=65535).contains(&p1) || !(0..=65535).contains(&p2) {
            return Err(Error::InvalidArgument);
        }
        self.server_port = p1 as u16;
        self.local_port = p2 as u16;
        self.is_port_shared = is_port_shared;
        self.nb_threads = nb_threads;
        Ok(())
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
    ///   one `Option<Box<dyn StreamDataCallback>>` per the
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
        default_callback: Option<Box<dyn StreamDataCallback>>,
        current_time: Instant,
        _p_simulated_time: Option<&mut u64>,
    ) -> Option<Box<Quic>> {
        let reset_seed = if self.has_reset_seed {
            self.reset_seed
        } else {
            [0u8; 16]
        };

        let mut quic = Quic::new(
            self.nb_connections,
            self.server_cert_file.as_deref(),
            self.server_key_file.as_deref(),
            self.root_trust_file.as_deref(),
            self.alpn.as_deref(),
            default_callback,
            None,
            reset_seed,
            current_time,
            self.ticket_file_name.as_deref(),
            self.ticket_encryption_key.as_deref(),
        )?;

        if self.do_retry {
            quic.set_cookie_mode(1);
        } else {
            quic.set_cookie_mode(2);
        }

        if let Some(ref cc_id) = self.cc_algo_id {
            let _ = quic.set_default_congestion_algorithm_by_name(cc_id);
        }

        let _ = quic.set_default_spinbit_policy(self.spinbit_policy);
        quic.set_default_lossbit_policy(self.lossbit_policy);
        quic.set_default_multipath_option(self.multipath_option);
        quic.set_default_idle_timeout(crate::Duration::from_ticks(self.idle_timeout as u64 * 1000));
        quic.set_cwin_max(self.cwin_max);
        quic.set_default_address_discovery_mode(self.address_discovery_mode);

        if let Some(ref token_file) = self.token_file_name {
            let _ = quic.load_retry_tokens(token_file);
        }

        if self.force_zero_share {
            quic.client_zero_share = true;
        }

        if self.mtu_max > 0 {
            quic.set_mtu_max(self.mtu_max as u32);
        }

        if self.connection_id_length != -1 {
            let _ = quic.set_default_connection_id_length(self.connection_id_length as u8);
        }

        quic.set_padding_policy(39, 128);

        if let Some(ref bin_dir) = self.bin_dir {
            let _ = quic.set_binlog(Some(bin_dir.as_str()));
        }

        if let Some(ref qlog_dir) = self.qlog_dir {
            let _ = quic.set_qlog(qlog_dir.as_str());
        }

        if let Some(ref log_file) = self.log_file {
            let _ = quic.set_textlog(Some(log_file.as_str()));
        }

        quic.set_log_level(if self.use_long_log { 1 } else { 0 });
        quic.set_preemptive_repeat_policy(self.do_preemptive_repeat);
        quic.set_port_blocking_disabled(self.disable_port_blocking);
        quic.set_sslkeylog_enabled(self.enable_sslkeylog);

        if self.initial_random <= 2 {
            quic.set_random_initial(self.initial_random as i32);
        }

        if self.cipher_suite_id != 0 {
            let iana_code = match self.cipher_suite_id {
                20 => crate::CHACHA20_POLY1305_SHA256,
                128 => crate::AES_128_GCM_SHA256,
                256 => crate::AES_256_GCM_SHA384,
                v => v as u16,
            };
            let _ = quic.set_cipher_suite(iana_code);
        }

        if self.do_retry {
            quic.set_cookie_mode(1);
        } else {
            quic.set_cookie_mode(2);
        }

        quic.set_default_bdp_frame_option(self.bdp_frame_option != 0);

        let mut failed = false;

        if let Some(ref public_name) = self.ech_public_name {
            if self.ech_key_file.is_none() || self.ech_config_file.is_none() {
                // key file or config file not specified — cannot create ECH config
            } else {
                let key_file = self.ech_key_file.as_deref().unwrap();
                let cfg_file = self.ech_config_file.as_deref().unwrap();
                if crate::ech_create_config_file(public_name, key_file, cfg_file).is_err() {
                    failed = true;
                }
            }
        }

        if !failed
            && (self.ech_key_file.is_some() || self.ech_target.is_some())
            && quic
                .ech_configure(
                    self.ech_key_file.as_deref(),
                    self.ech_config_file.as_deref(),
                )
                .is_err()
        {
            failed = true;
        }

        if !failed && self.flow_control_max > 0 {
            quic.set_max_data_control(self.flow_control_max);
        }

        if failed { None } else { Some(quic) }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // --- config_atoi ---

    #[test]
    fn atoi_valid_integer() {
        assert_eq!(config_atoi(&["42"], 0), Ok(42));
    }

    #[test]
    fn atoi_zero() {
        assert_eq!(config_atoi(&["0"], 0), Ok(0));
    }

    #[test]
    fn atoi_multi_digit() {
        assert_eq!(config_atoi(&["12345"], 0), Ok(12345));
    }

    #[test]
    fn atoi_out_of_bounds_index() {
        assert_eq!(config_atoi(&["5"], 1), Err(Error::InvalidArgument));
    }

    #[test]
    fn atoi_empty_params() {
        assert_eq!(config_atoi(&[], 0), Err(Error::InvalidArgument));
    }

    #[test]
    fn atoi_non_digit_char() {
        assert_eq!(config_atoi(&["12a3"], 0), Err(Error::InvalidArgument));
    }

    #[test]
    fn atoi_negative_sign_rejected() {
        // C version rejects any non-digit byte; '-' is not a digit
        assert_eq!(config_atoi(&["-5"], 0), Err(Error::InvalidArgument));
    }

    // --- config_optval_string ---

    #[test]
    fn optval_string_copies_within_capacity() {
        let mut buf = [0u8; 16];
        let result = config_optval_string(&mut buf, b"hello");
        assert_eq!(result, "hello");
        assert_eq!(buf[5], 0); // null terminator
    }

    #[test]
    fn optval_string_truncates_to_buffer_minus_one() {
        let mut buf = [0u8; 4]; // capacity 4 → max 3 bytes + NUL
        let result = config_optval_string(&mut buf, b"abcdef");
        assert_eq!(result, "abc");
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn optval_string_empty_input() {
        let mut buf = [0xffu8; 8];
        let result = config_optval_string(&mut buf, b"");
        assert_eq!(result, "");
        assert_eq!(buf[0], 0);
    }
}
