//! Translation of `quic/performance_log.h`.
//!
//! The performance log records a fixed-shape vector of metrics
//! (durations, byte counts, RTTs, congestion-control parameters)
//! per closed connection and appends them to a CSV file once the
//! server's connection list drains.  The header itself only exposes
//! the column-index enum and two entry points: attaching a perflog
//! to a QUIC context, and looking up the short CSV column name.
//!
//! Phase 1: signatures only — both function bodies are `todo!()`.
//! The internal item/context types and the bookkeeping helpers
//! defined in `performance_log.c` are private to that translation
//! unit and will land alongside their bodies in Phase 3.

use std::path::Path;

use crate::Error;
use crate::Quic;

/// Schema version stamped at the start of every CSV row.  Bumped if
/// the column layout changes incompatibly.
/// C: `PICOQUIC_PER_LOG_VERSION`.
pub const PER_LOG_VERSION: u32 = 1;

/// Number of metric slots in the per-connection metric vector — i.e.
/// the count of variants in [`PerflogColumn`].  Used as an array
/// dimension and as a loop bound, hence `usize`.
/// C: `PICOQUIC_PERF_LOG_MAX_ITEMS`.
pub const PERF_LOG_MAX_ITEMS: usize = 27;

/// CSV column identifiers for the per-connection metric vector.
/// Each variant names one slot in the metric vector and its position
/// in the CSV row.  Discriminants are explicit so that reordering
/// would be a visible breaking change — the C code uses these values
/// both as array indices and as the column ordering.
/// C: `picoquic_perflog_column_enum`; discriminants match the C
/// values `0..=26`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum PerflogColumn {
    IsClient = 0,
    NbPacketsReceived = 1,
    NbTrainsSent = 2,
    NbTrainsShort = 3,
    NbTrainsBlockedCwin = 4,
    NbTrainsBlockedPacing = 5,
    NbTrainsBlockedOthers = 6,
    NbPacketsSent = 7,
    NbRetransmissionTotal = 8,
    NbSpurious = 9,
    DelayedAckOption = 10,
    MinAckDelayRemote = 11,
    MaxAckDelayRemote = 12,
    MaxAckGapRemote = 13,
    MinAckDelayLocal = 14,
    MaxAckDelayLocal = 15,
    MaxAckGapLocal = 16,
    MaxMtuSent = 17,
    MaxMtuReceived = 18,
    ZeroRtt = 19,
    Srtt = 20,
    Minrtt = 21,
    Cwin = 22,
    Ccalgo = 23,
    BweMax = 24,
    PacingQuantumMax = 25,
    PacingRate = 26,
}

impl PerflogColumn {
    /// Short column name used in the CSV header for this column.
    /// The C counterpart returns `NULL` for out-of-range inputs (the
    /// `default` switch arm); Rust's exhaustive enum makes that case
    /// unreachable, so this returns `&'static str` directly rather
    /// than `Option`.  Lifetime mirrors the C string-literal return.
    /// C: `picoquic_perflog_param_name`.
    pub fn param_name(self) -> &'static str {
        todo!()
    }
}

impl Quic {
    /// Attach a performance log to this QUIC context, writing CSV
    /// rows to `perflog_file_name` whenever the connection list
    /// drains.  If the file is empty (or missing), a CSV header row
    /// is written first.
    ///
    /// Pointer-shape choices, from the sole observed caller
    /// (`quic/sockloop.c:1910`): both C arguments are non-NULL — the
    /// QUIC context is taken from `*qserver`, and the filename is
    /// gated by `if (config->performance_log != NULL)` immediately
    /// above.  So the receiver is `&mut self` (the call mutates
    /// `self.perflog_fn` and `self.v_perflog_ctx`), and the filename
    /// is a borrowed `&str` (the C body deep-copies via
    /// `picoquic_string_duplicate`).
    ///
    /// The C `int` return is a 0/-1 status, mapped to
    /// `Result<(), Error>`.
    /// C: `picoquic_perflog_setup`.
    pub fn perflog_setup(
        &mut self,
        _perflog_file_name: &(impl AsRef<Path> + ?Sized),
    ) -> Result<(), Error> {
        todo!()
    }
}

#[cfg(test)]
mod test {}
