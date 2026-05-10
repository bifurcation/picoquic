//! Translation of `quic/performance_log.h` and `quic/performance_log.c`.
//!
//! The performance log records a fixed-shape vector of metrics
//! (durations, byte counts, RTTs, congestion-control parameters)
//! per closed connection and appends them to a CSV file once the
//! server's connection list drains.  The header itself only exposes
//! the column-index enum and two entry points: attaching a perflog
//! to a QUIC context, and looking up the short CSV column name.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::internal::PerformanceLog;
use crate::utils::print_connection_id_hexa;
use crate::{Connection, Error, Quic};

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
        match self {
            PerflogColumn::IsClient => "is_client",
            PerflogColumn::NbPacketsReceived => "pkt_recv",
            PerflogColumn::NbTrainsSent => "trains_s",
            PerflogColumn::NbTrainsShort => "t_short",
            PerflogColumn::NbTrainsBlockedCwin => "tb_cwin",
            PerflogColumn::NbTrainsBlockedPacing => "tb_pacing",
            PerflogColumn::NbTrainsBlockedOthers => "tb_others",
            PerflogColumn::NbPacketsSent => "pkt_sent",
            PerflogColumn::NbRetransmissionTotal => "retrans.",
            PerflogColumn::NbSpurious => "spurious",
            PerflogColumn::DelayedAckOption => "delayed_ack_option",
            PerflogColumn::MinAckDelayRemote => "min_ack_delay_remote",
            PerflogColumn::MaxAckDelayRemote => "max_ack_delay_remote",
            PerflogColumn::MaxAckGapRemote => "max_ack_gap_remote",
            PerflogColumn::MinAckDelayLocal => "min_ack_delay_local",
            PerflogColumn::MaxAckDelayLocal => "max_ack_delay_local",
            PerflogColumn::MaxAckGapLocal => "max_ack_gap_local",
            PerflogColumn::MaxMtuSent => "max_mtu_sent",
            PerflogColumn::MaxMtuReceived => "max_mtu_received",
            PerflogColumn::ZeroRtt => "zero_rtt",
            PerflogColumn::Srtt => "srtt",
            PerflogColumn::Minrtt => "minrtt",
            PerflogColumn::Cwin => "cwin",
            PerflogColumn::Ccalgo => "ccalgo",
            PerflogColumn::BweMax => "bwe_max",
            PerflogColumn::PacingQuantumMax => "p_quantum",
            PerflogColumn::PacingRate => "p_rate",
        }
    }

    /// Columns in canonical CSV order; mirrors the C `for (i = 0; i <
    /// PICOQUIC_PERF_LOG_MAX_ITEMS; i++)` header-emission loop.
    fn all() -> [PerflogColumn; PERF_LOG_MAX_ITEMS] {
        [
            PerflogColumn::IsClient,
            PerflogColumn::NbPacketsReceived,
            PerflogColumn::NbTrainsSent,
            PerflogColumn::NbTrainsShort,
            PerflogColumn::NbTrainsBlockedCwin,
            PerflogColumn::NbTrainsBlockedPacing,
            PerflogColumn::NbTrainsBlockedOthers,
            PerflogColumn::NbPacketsSent,
            PerflogColumn::NbRetransmissionTotal,
            PerflogColumn::NbSpurious,
            PerflogColumn::DelayedAckOption,
            PerflogColumn::MinAckDelayRemote,
            PerflogColumn::MaxAckDelayRemote,
            PerflogColumn::MaxAckGapRemote,
            PerflogColumn::MinAckDelayLocal,
            PerflogColumn::MaxAckDelayLocal,
            PerflogColumn::MaxAckGapLocal,
            PerflogColumn::MaxMtuSent,
            PerflogColumn::MaxMtuReceived,
            PerflogColumn::ZeroRtt,
            PerflogColumn::Srtt,
            PerflogColumn::Minrtt,
            PerflogColumn::Cwin,
            PerflogColumn::Ccalgo,
            PerflogColumn::BweMax,
            PerflogColumn::PacingQuantumMax,
            PerflogColumn::PacingRate,
        ]
    }
}

/// One per-connection metric vector queued for later flushing.
/// C: `picoquic_performance_log_item_t`.
struct PerflogItem {
    duration_sec: f64,
    send_mbps: f64,
    recv_mbps: f64,
    data_sent: u64,
    data_received: u64,
    quic_version: u32,
    alpn: Option<String>,
    cnxid: crate::ConnectionId,
    cnx_time_64: u64,
    v: [u64; PERF_LOG_MAX_ITEMS],
}

/// Per-context perflog state.  Owns the queued items and the file
/// path; flushed when the server's connection list drains.
/// C: `picoquic_performance_log_ctx_t`.  In Rust the function
/// pointer + `void*` pair from C collapses into this single trait
/// object: the boxed `PerflogCtx` holds the state and provides the
/// `PerformanceLog::emit` callback in one piece.
struct PerflogCtx {
    items: Vec<PerflogItem>,
    perflog_file_name: PathBuf,
}

impl PerflogCtx {
    /// Append a CSV row per queued item to the perflog file, then
    /// drain the queue.  C: `picoquic_perflog_save`.
    fn save(&mut self) -> Result<(), Error> {
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.perflog_file_name)
            .map_err(|_| Error::Generic)?;
        for item in self.items.drain(..) {
            let mut cnxid_str = String::new();
            if print_connection_id_hexa(&mut cnxid_str, &item.cnxid).is_err() {
                cnxid_str.clear();
            }
            write!(f, "{}, {}, ", PER_LOG_VERSION, crate::VERSION).map_err(|_| Error::Generic)?;
            write!(
                f,
                "{:.6}, {}, {}, {:.6}, {:.6}",
                item.duration_sec,
                item.data_sent,
                item.data_received,
                item.send_mbps,
                item.recv_mbps,
            )
            .map_err(|_| Error::Generic)?;
            write!(
                f,
                ", 0x{:x}, {}, 0x{}, {}",
                item.quic_version,
                item.alpn.as_deref().unwrap_or(""),
                cnxid_str,
                item.cnx_time_64,
            )
            .map_err(|_| Error::Generic)?;
            for v in item.v.iter() {
                write!(f, ", {}", v).map_err(|_| Error::Generic)?;
            }
            writeln!(f).map_err(|_| Error::Generic)?;
        }
        Ok(())
    }

    /// Snapshot the connection's metrics into a queued item.
    /// C: `picoquic_perflog_record`.
    fn record(&mut self, connection: &Connection) {
        let start_time = connection.start_time.ticks();
        let close_time = connection.quic_time().ticks();
        let duration_usec = close_time.saturating_sub(start_time);
        let duration_sec = (duration_usec as f64) / 1_000_000.0;

        let mut item = PerflogItem {
            duration_sec,
            send_mbps: 0.0,
            recv_mbps: 0.0,
            data_sent: 0,
            data_received: 0,
            quic_version: 0,
            alpn: None,
            cnxid: connection.logging_connection_id(),
            cnx_time_64: start_time,
            v: [0; PERF_LOG_MAX_ITEMS],
        };

        if duration_usec > 0 {
            item.data_sent = connection.data_sent;
            item.data_received = connection.data_received;
            item.send_mbps = (connection.data_sent as f64) * 8.0 / (duration_usec as f64);
            item.recv_mbps = (connection.data_received as f64) * 8.0 / (duration_usec as f64);
        }

        item.alpn = connection.alpn.clone();
        item.quic_version = if connection.version_index >= 0 {
            connection.version_number()
        } else {
            0
        };

        let v = &mut item.v;
        v[PerflogColumn::IsClient as usize] = connection.client_mode as u64;
        v[PerflogColumn::NbPacketsReceived as usize] = connection.nb_packets_received;
        v[PerflogColumn::NbTrainsSent as usize] = connection.nb_trains_sent;
        v[PerflogColumn::NbTrainsShort as usize] = connection.nb_trains_short;
        v[PerflogColumn::NbTrainsBlockedCwin as usize] = connection.nb_trains_blocked_cwin;
        v[PerflogColumn::NbTrainsBlockedPacing as usize] = connection.nb_trains_blocked_pacing;
        v[PerflogColumn::NbTrainsBlockedOthers as usize] = connection.nb_trains_blocked_others;
        v[PerflogColumn::NbPacketsSent as usize] = connection.nb_packets_sent;
        v[PerflogColumn::NbRetransmissionTotal as usize] = connection.nb_retransmission_total;
        v[PerflogColumn::NbSpurious as usize] = connection.nb_spurious;
        v[PerflogColumn::DelayedAckOption as usize] = connection.is_ack_frequency_negotiated as u64;
        v[PerflogColumn::MinAckDelayRemote as usize] = connection.min_ack_delay_remote.ticks();
        v[PerflogColumn::MaxAckDelayRemote as usize] = connection.max_ack_delay_remote.ticks();
        v[PerflogColumn::MaxAckGapRemote as usize] = connection.max_ack_gap_remote;
        v[PerflogColumn::MinAckDelayLocal as usize] = connection.min_ack_delay_local.ticks();
        v[PerflogColumn::MaxAckDelayLocal as usize] = connection.max_ack_delay_local.ticks();
        v[PerflogColumn::MaxAckGapLocal as usize] = connection.max_ack_gap_local;
        v[PerflogColumn::MaxMtuSent as usize] = connection.max_mtu_sent as u64;
        v[PerflogColumn::MaxMtuReceived as usize] = connection.max_mtu_received as u64;
        v[PerflogColumn::ZeroRtt as usize] =
            (connection.nb_zero_rtt_received > 0 || connection.nb_zero_rtt_acked > 0) as u64;

        if let Some(path) = connection.paths.first() {
            v[PerflogColumn::Srtt as usize] = path.smoothed_rtt.ticks();
            v[PerflogColumn::Minrtt as usize] = path.rtt_min.ticks();
            v[PerflogColumn::Cwin as usize] = path.cwin;
            v[PerflogColumn::BweMax as usize] = path.bandwidth_estimate_max;
            v[PerflogColumn::PacingQuantumMax as usize] = path.pacing.quantum_max;
            v[PerflogColumn::PacingRate as usize] = path.pacing.rate_max;
        }

        if let Some(alg) = connection.congestion_alg {
            v[PerflogColumn::Ccalgo as usize] = alg.congestion_algorithm_number as u64;
        }

        self.items.push(item);
    }
}

impl PerformanceLog for PerflogCtx {
    /// C: `picoquic_perflog`.  Records the connection's metrics, and
    /// flushes to disk when this is the last connection on the
    /// context (or on context teardown).
    fn emit(&mut self, quic: &Quic, connection: &Connection, should_delete: bool) -> i32 {
        self.record(connection);
        // C check: cnx_list == cnx && cnx_last == cnx — i.e. the
        // connection being closed is the only one on this context.
        // In Rust we count entries in the connection arena; teardown
        // (`should_delete`) is also a flush trigger.
        let last_one = quic.connections.len() <= 1;
        if (last_one || should_delete) && self.save().is_err() {
            return -1;
        }
        0
    }

    fn close(&mut self, _quic: &Quic) -> i32 {
        if self.save().is_err() { -1 } else { 0 }
    }
}

/// Probe whether the perflog file is missing or zero-length.
/// C: `picoquic_perflog_file_is_empty`.
fn file_is_empty(perflog_file_name: &Path) -> bool {
    match std::fs::metadata(perflog_file_name) {
        Ok(md) => md.len() == 0,
        Err(_) => true,
    }
}

/// Truncate and write the CSV header row.
/// C: `picoquic_perflog_file_set_header`.
fn file_set_header(perflog_file_name: &Path) {
    let Ok(mut f) = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(perflog_file_name)
    else {
        return;
    };
    let _ = write!(f, "Log_v, PQ_v, Duration, Sent, Received, Mpbs_S, Mbps_R");
    let _ = write!(f, ", QUIC_v, ALPN, CNX_ID, T64");
    for col in PerflogColumn::all() {
        let _ = write!(f, ", {}", col.param_name());
    }
    let _ = writeln!(f);
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
    /// `self.perflog_fn`), and the filename is borrowed (the C body
    /// deep-copies via `picoquic_string_duplicate`; we keep an owned
    /// `PathBuf` inside the trait object's state).
    ///
    /// The C `int` return is a 0/-1 status, mapped to
    /// `Result<(), Error>`.
    /// C: `picoquic_perflog_setup`.
    pub fn perflog_setup(
        &mut self,
        perflog_file_name: &(impl AsRef<Path> + ?Sized),
    ) -> Result<(), Error> {
        let path = perflog_file_name.as_ref().to_path_buf();
        if file_is_empty(&path) {
            file_set_header(&path);
        }
        let ctx = PerflogCtx {
            items: Vec::new(),
            perflog_file_name: path,
        };
        self.perflog_fn = Some(Box::new(ctx));
        Ok(())
    }
}

#[cfg(test)]
mod test {}
