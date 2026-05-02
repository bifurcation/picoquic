//! Translation of `quic/performance_log.h`.
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

use crate::Error;
use crate::quic_t;

// ---------------------------------------------------------------------------
// Tunable constants (`#define`s in the header).

/// Schema version stamped at the start of every CSV row.  Bumped if
/// the column layout changes incompatibly.
pub const PER_LOG_VERSION: u32 = 1;

/// Number of metric slots in `performance_log_item_t::v`.
/// Equal to the count of variants in `perflog_column_enum`.
/// Used as an array dimension and as a loop bound, hence `usize`.
pub const PERF_LOG_MAX_ITEMS: usize = 27;

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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum perflog_column_enum {
    perflog_is_client = 0,
    perflog_nb_packets_received = 1,
    perflog_nb_trains_sent = 2,
    perflog_nb_trains_short = 3,
    perflog_nb_trains_blocked_cwin = 4,
    perflog_nb_trains_blocked_pacing = 5,
    perflog_nb_trains_blocked_others = 6,
    perflog_nb_packets_sent = 7,
    perflog_nb_retransmission_total = 8,
    perflog_nb_spurious = 9,
    perflog_delayed_ack_option = 10,
    perflog_min_ack_delay_remote = 11,
    perflog_max_ack_delay_remote = 12,
    perflog_max_ack_gap_remote = 13,
    perflog_min_ack_delay_local = 14,
    perflog_max_ack_delay_local = 15,
    perflog_max_ack_gap_local = 16,
    perflog_max_mtu_sent = 17,
    perflog_max_mtu_received = 18,
    perflog_zero_rtt = 19,
    perflog_srtt = 20,
    perflog_minrtt = 21,
    perflog_cwin = 22,
    perflog_ccalgo = 23,
    perflog_bwe_max = 24,
    perflog_pacing_quantum_max = 25,
    perflog_pacing_rate = 26,
}

// ---------------------------------------------------------------------------
// Public API.

/// Short column name used in the CSV header for `rank`.  The C
/// counterpart returns `NULL` for out-of-range inputs (the `default`
/// switch arm); Rust's exhaustive enum makes that case unreachable, so
/// this returns `&'static str` directly rather than `Option`.
/// Lifetime mirrors the C string-literal return.
pub fn perflog_param_name(_rank: perflog_column_enum) -> &'static str {
    todo!()
}

/// Attach a performance log to `quic`, writing CSV rows to
/// `perflog_file_name` whenever the connection list drains.  Mirrors
/// `int perflog_setup(quic_t*, char const*)`.
///
/// Pointer-shape choices, from the sole observed caller
/// (`quic/sockloop.c:1910`): both arguments are non-NULL — the
/// QUIC context is taken from `*qserver`, and the filename is gated
/// by `if (config->performance_log != NULL)` immediately above.  So
/// the QUIC context is `&mut` (the call mutates `quic->perflog_fn`
/// and `quic->v_perflog_ctx`), and the filename is a borrowed `&str`
/// (the C body deep-copies via `string_duplicate`).
///
/// The C `int` return is a 0/-1 status, mapped to `Result<(), Error>`.
pub fn perflog_setup(_quic: &mut quic_t, _perflog_file_name: &str) -> Result<(), Error> {
    todo!()
}

#[cfg(test)]
mod test {}
