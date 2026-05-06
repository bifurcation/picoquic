//! Translation of `quic/logger.h`.
//!
//! Public surface of the quic *text* logger backend.  Once a text
//! log file is installed on a [`Quic`] context, the context's
//! text-log slot points at the textlog implementation of
//! [`crate::logger::Logger`] and every per-event log
//! call fans out through that vtable.  The install/teardown entry
//! points hang as inherent methods on [`Quic`]
//! ([`Quic::set_textlog`] / [`Quic::textlog_close`]).
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
//! * `picoquic_textlog_picotls_ticket` was the textlog backend's
//!   TLS-ticket pretty-printer.  Phase 4 reintroduces it as a
//!   private helper inside the textlog `Logger` impl; no public
//!   API needs it.
//! * `picoquic_log_fin_or_event_name` is declared in the C header
//!   but never defined or referenced — dropped from the Rust API.

use std::fs::OpenOptions;
use std::path::Path;

use crate::Error;
use crate::Quic;

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
        textlog_file: Option<&(impl AsRef<Path> + ?Sized)>,
    ) -> Result<(), Error> {
        self.textlog_close();

        let Some(path) = textlog_file else {
            return Ok(());
        };
        let path = path.as_ref();

        if path == Path::new("-") {
            self.f_log = Some(Box::new(std::io::stdout()));
            self.should_close_log = false;
        } else {
            match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
            {
                Ok(f) => {
                    self.f_log = Some(Box::new(f));
                    self.should_close_log = true;
                }
                Err(_) => return Err(Error::NoSuchFile),
            }
        }

        Ok(())
    }

    /// Close the text log, e.g., when closing the QUIC context.
    /// Safe to call when no text log is currently installed — the C
    /// body guards on `quic->F_log != NULL` and on the
    /// `quic->should_close_log` flag (so an external `stdout`
    /// handle is left alone).
    ///
    /// C: `void picoquic_textlog_close(picoquic_quic_t*)`.
    pub fn textlog_close(&mut self) {
        self.f_log = None;
        self.should_close_log = false;
    }
}

#[cfg(test)]
mod test {}
