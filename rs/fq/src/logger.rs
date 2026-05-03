//! Translation of `quic/logger.h`.
//!
//! Public surface of the quic *text* logger backend.  Once a text
//! log file is installed on a [`Quic`] context, the context's
//! text-log slot points at the textlog implementation of
//! [`crate::unified_log::Logger`] and every per-event log
//! call fans out through that vtable.  The install/teardown entry
//! points hang as inherent methods on [`Quic`]
//! ([`Quic::set_textlog`] / [`Quic::textlog_close`]); the textlog's
//! TLS-ticket pretty printer is the free function
//! [`write_tls_ticket`].
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//!
//! Pointer-shape and translation policy notes for this module:
//!
//! * `Quic*` — every observed caller (`config.c::963`,
//!   the recursive call in `logger.c::set_textlog`) passes
//!   a non-NULL, mutable context handle.  Maps to `&mut
//!   Quic`.
//! * `char const* textlog_file` — the C contract uses `NULL` as the
//!   "stop the text log" sentinel, so the parameter maps to
//!   `Option<&str>`.  `Some("-")` is the established
//!   redirect-to-stdout shortcut and is preserved as-is.
//! * The C return is `int` (0 on success, `-1` on file-open
//!   failure).  Mapped to `Result<(), Error>` per the project's
//!   error-handling convention.
//! * `FILE* F` → `&mut dyn core::fmt::Write`.  Matches the
//!   convention introduced in
//!   [`crate::config::Config::write_usage`]
//!   and keeps the helper `no_std`-friendly.
//! * `ConnectionId cnx_id` is `Copy` and only read by
//!   the body (it is folded into a 64-bit value via
//!   `val64_connection_id`), so it stays a pass-by-value
//!   parameter, mirroring the C ABI.
//! * `uint8_t* ticket` + `uint16_t ticket_length` collapse to a
//!   single `&[u8]` — the body only reads the buffer and the slice
//!   length subsumes the explicit length argument.
//! * `picoquic_log_fin_or_event_name` is declared in the C header
//!   but never defined or referenced — dropped from the Rust API.

use std::path::Path;

use crate::Error;
use crate::{ConnectionId, Quic};

impl Quic {
    /// Set the text log file and start tracing into it.
    ///
    /// Pass `None` to stop the text log (the C "set to NULL"
    /// sentinel).  `Some("-")` redirects output to stdout without
    /// taking ownership of the handle, matching the C body in
    /// `logger.c::set_textlog`.
    ///
    /// C: `int picoquic_set_textlog(picoquic_quic_t*, char const*)`.
    pub fn set_textlog(
        &mut self,
        _textlog_file: Option<&(impl AsRef<Path> + ?Sized)>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Close the text log, e.g., when closing the QUIC context.
    /// Safe to call when no text log is currently installed — the C
    /// body guards on `quic->F_log != NULL` and on the
    /// `quic->should_close_log` flag (so an external `stdout`
    /// handle is left alone).
    ///
    /// C: `void picoquic_textlog_close(picoquic_quic_t*)`.
    pub fn textlog_close(&mut self) {
        todo!()
    }
}

/// Pretty-print the contents of a TLS session ticket to a
/// `core::fmt::Write` sink, prefixed with a `cnx_id` label.
///
/// The C signature took a `FILE*`; the Rust equivalent accepts
/// any [`core::fmt::Write`] sink so the helper stays
/// `no_std`-friendly.  Free function rather than a method on
/// [`ConnectionId`] because the connection id is just a label —
/// the function's job is writing the ticket, not anything
/// `ConnectionId`-specific.
///
/// REVIEW(open): only one in-tree caller (the textlog backend);
/// likely fold into that backend's body in Phase 4 and delete from
/// the public API.
///
/// C: `void picoquic_textlog_picotls_ticket(FILE*,
/// picoquic_connection_id_t, uint8_t*, uint16_t)`.
pub fn write_tls_ticket(_f: &mut impl core::fmt::Write, _cnx_id: ConnectionId, _ticket: &[u8]) {
    todo!()
}

#[cfg(test)]
mod test {}
