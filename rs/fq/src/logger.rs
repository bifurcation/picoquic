//! Translation of `quic/logger.h`.
//!
//! Public-facing entry points to the quic *text* logger
//! backend.  The header lifts four symbols out of `logger.c` so
//! applications can install / tear down the textual log file
//! without ever touching the unified-log vtable directly.  Once a
//! text log file is installed, the QUIC context's
//! `text_log_fns` slot points at the textlog implementation of
//! [`crate::unified_log::UnifiedLogging`],
//! and every per-event log call fans out through that vtable.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//!
//! Pointer-shape and translation policy notes for this module:
//!
//! * `quic_t*` — every observed caller (`config.c::963`,
//!   the recursive call in `logger.c::set_textlog`) passes
//!   a non-NULL, mutable context handle.  Maps to `&mut
//!   quic_t`.
//! * `char const* textlog_file` — the C contract uses `NULL` as the
//!   "stop the text log" sentinel, so the parameter maps to
//!   `Option<&str>`.  `Some("-")` is the established
//!   redirect-to-stdout shortcut and is preserved as-is.
//! * The C return is `int` (0 on success, `-1` on file-open
//!   failure).  Mapped to `Result<(), Error>` per the project's
//!   error-handling convention.
//! * `FILE* F` → `&mut dyn core::fmt::Write`.  Matches the
//!   convention introduced in
//!   [`crate::config::config_usage_file`]
//!   and keeps the helper `no_std`-friendly.
//! * `connection_id_t cnx_id` is `Copy` and only read by
//!   the body (it is folded into a 64-bit value via
//!   `val64_connection_id`), so it stays a pass-by-value
//!   parameter, mirroring the C ABI.
//! * `uint8_t* ticket` + `uint16_t ticket_length` collapse to a
//!   single `&[u8]` — the body only reads the buffer and the slice
//!   length subsumes the explicit length argument.
//! * `log_fin_or_event_name` returns a static C string in
//!   the header but is not defined or referenced anywhere under
//!   `quic/`.  Translated as a function returning `&'static
//!   str`; Phase 3 either supplies the lookup table or removes the
//!   orphan declaration.

use crate::Error;
use crate::{call_back_event_t, connection_id_t, quic_t};

/// Set the text log file and start tracing into it.
///
/// Pass `None` to stop the text log (the C "set to NULL"
/// sentinel).  `Some("-")` redirects output to stdout without
/// taking ownership of the handle, matching the C body in
/// `logger.c::set_textlog`.
///
/// C: `int set_textlog(quic_t*, char const*)`.
// lands.  The C return is 0 / -1 (file-open failure).
pub fn set_textlog(_quic: &mut quic_t, _textlog_file: Option<&str>) -> Result<(), Error> {
    todo!()
}

/// Close the text log, e.g., when closing the QUIC context.  Safe
/// to call when no text log is currently installed — the C body
/// guards on `quic->F_log != NULL` and on the
/// `quic->should_close_log` flag (so an external `stdout` handle
/// is left alone).
///
/// C: `void textlog_close(quic_t*)`.
pub fn textlog_close(_quic: &mut quic_t) {
    todo!()
}

/// Pretty-print the contents of a tls session ticket to a
/// `core::fmt::Write` sink.
///
/// The C signature took a `FILE*`; the Rust equivalent accepts
/// any sink so the helper stays `no_std`-friendly.  Internal
/// callers (currently `logger.c::textlog_tls_ticket`) will pass
/// through whichever writer the QUIC context's text log slot is
/// holding.
///
/// C: `void textlog_tls_ticket(FILE*,
/// connection_id_t, uint8_t*, uint16_t)`.
pub fn textlog_tls_ticket(_f: &mut dyn core::fmt::Write, _cnx_id: connection_id_t, _ticket: &[u8]) {
    todo!()
}

/// Return a printable name for an application callback event.
///
/// The C function returns a pointer to a `static const char[]`
/// literal selected by a switch on the event tag; the Rust
/// equivalent returns the same lifetime-static `&str`.
///
/// The header declares this symbol but no `.c` file under
/// `quic/` defines or calls it — kept here for header parity.
/// Phase 3 either supplies the lookup table (most likely a `match`
/// on every variant of [`call_back_event_t`]) or removes
/// the orphan declaration upstream.
///
/// C: `const char* log_fin_or_event_name(call_back_event_t)`.
pub fn log_fin_or_event_name(_ev: call_back_event_t) -> &'static str {
    todo!()
}

#[cfg(test)]
mod test {}
