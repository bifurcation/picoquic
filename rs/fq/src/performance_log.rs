//! Translation of `picoquic/performance_log.h`.
//!
//! The performance log records a fixed-shape vector of metrics
//! (durations, byte counts, RTTs, congestion-control parameters)
//! per closed connection and appends them to a CSV file once the
//! server's connection list drains.  The header itself only exposes
//! the column-name enum and two entry points: `setup` (attach the
//! perflog to a QUIC context) and `param_name` (column name lookup
//! for header generation).
//!
//! Phase 1: signatures only — both function bodies are `todo!()`.
//! The internal item/context types and the bookkeeping helpers
//! defined in `performance_log.c` are private to that translation
//! unit and will land alongside their bodies in Phase 3.

use crate::picoquic_quic_t;

// ---------------------------------------------------------------------------
// Tunable constants (`#define`s in the header).

/// Schema version stamped at the start of every CSV row.  Bumped if
/// the column layout changes incompatibly.
pub const PICOQUIC_PER_LOG_VERSION: u32 = 1;

/// Number of metric slots in `picoquic_performance_log_item_t::v`.
/// Equal to the count of variants in `picoquic_perflog_column_enum`.
/// Used as an array dimension and as a loop bound, hence `usize`.
pub const PICOQUIC_PERF_LOG_MAX_ITEMS: usize = 27;

// ---------------------------------------------------------------------------
// Column index for the metric vector.
//
// Each variant names one slot in the per-connection metric vector and
// its position in the CSV row.  Discriminants are explicit so that
// reordering would be a visible breaking change — the C code uses
// these values both as array indices and as the column ordering.

/// CSV column identifiers for the per-connection metric vector.
/// Mirrors the C enum of the same name; discriminants match the C
/// values 0..=26.
// Variant names mirror the C enum tags one-to-one and all share the
// `picoquic_perflog_` prefix; renaming would break source-level parity.
#[allow(non_camel_case_types, clippy::enum_variant_names)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum picoquic_perflog_column_enum {
    picoquic_perflog_is_client = 0,
    picoquic_perflog_nb_packets_received = 1,
    picoquic_perflog_nb_trains_sent = 2,
    picoquic_perflog_nb_trains_short = 3,
    picoquic_perflog_nb_trains_blocked_cwin = 4,
    picoquic_perflog_nb_trains_blocked_pacing = 5,
    picoquic_perflog_nb_trains_blocked_others = 6,
    picoquic_perflog_nb_packets_sent = 7,
    picoquic_perflog_nb_retransmission_total = 8,
    picoquic_perflog_nb_spurious = 9,
    picoquic_perflog_delayed_ack_option = 10,
    picoquic_perflog_min_ack_delay_remote = 11,
    picoquic_perflog_max_ack_delay_remote = 12,
    picoquic_perflog_max_ack_gap_remote = 13,
    picoquic_perflog_min_ack_delay_local = 14,
    picoquic_perflog_max_ack_delay_local = 15,
    picoquic_perflog_max_ack_gap_local = 16,
    picoquic_perflog_max_mtu_sent = 17,
    picoquic_perflog_max_mtu_received = 18,
    picoquic_perflog_zero_rtt = 19,
    picoquic_perflog_srtt = 20,
    picoquic_perflog_minrtt = 21,
    picoquic_perflog_cwin = 22,
    picoquic_perflog_ccalgo = 23,
    picoquic_perflog_bwe_max = 24,
    picoquic_perflog_pacing_quantum_max = 25,
    picoquic_perflog_pacing_rate = 26,
}

// ---------------------------------------------------------------------------
// Public API.

/// Short column name used in the CSV header for `rank`.  The C
/// counterpart returns `NULL` for out-of-range inputs (the `default`
/// switch arm); Rust's exhaustive enum makes that case unreachable, so
/// this returns `&'static str` directly rather than `Option`.
/// Lifetime mirrors the C string-literal return.
pub fn picoquic_perflog_param_name(_rank: picoquic_perflog_column_enum) -> &'static str {
    todo!()
}

/// Attach a performance log to `quic`, writing CSV rows to
/// `perflog_file_name` whenever the connection list drains.  Mirrors
/// `int picoquic_perflog_setup(picoquic_quic_t*, char const*)`.
///
/// Pointer-shape choices, from the sole observed caller
/// (`picoquic/sockloop.c:1910`): both arguments are non-NULL — the
/// QUIC context is taken from `*qserver`, and the filename is gated
/// by `if (config->performance_log != NULL)` immediately above.  So
/// the QUIC context is `&mut` (the call mutates `quic->perflog_fn`
/// and `quic->v_perflog_ctx`), and the filename is a borrowed `&str`
/// (the C body deep-copies via `picoquic_string_duplicate`).
///
/// The C `int` return is a 0/-1 status.  No crate-level `Error`
/// enum exists yet, so this returns `Result<(), ()>`; revisit when
/// the top-level error type is introduced.
// TODO(error-enum): replace `()` with the crate-level `Error` once it
// lands; clippy's `result_unit_err` is silenced in the meantime.
#[allow(clippy::result_unit_err)]
pub fn picoquic_perflog_setup(
    _quic: &mut picoquic_quic_t,
    _perflog_file_name: &str,
) -> Result<(), ()> {
    todo!()
}

#[cfg(test)]
mod test {}
