//! Translation of `quic/qlog.h`.
//!
//! Header-only module exposing a single entry point that turns
//! on per-connection qlog tracing for a QUIC context.  The
//! implementation lives in `loglib/autoqlog.c` (out of scope for
//! v1, since `loglib/` is a separate target — this header is the
//! quic-core stub that links against it on demand).

use crate::Error;
use crate::quic_t;

/// Set the qlog directory and start streaming qlog traces for
/// each connection.  C: `int set_qlog(quic_t*
/// quic, char const* qlog_dir)`.
///
/// Pointer-shape choices:
///
/// * `quic` → `&mut quic_t`.  Mutates the unified-logging
///   function-pointer slots on the QUIC context; no in-tree caller
///   passes NULL.
/// * `qlog_dir` → `&str`.  Every caller in `quic-core` and the
///   sample apps passes a non-NULL path string (often `"."` or a
///   `config->qlog_dir` field that is populated before the call).
///   Translation note: the C parameter is `char const*` so NULL is
///   technically representable, but no caller does so — `&str`
///   matches the actual contract.  Swap to `Option<&str>` if a
///   future caller needs to clear the directory.
///
/// Returns `Result<(), Error>` to mirror the C status code (`0` on
/// success, non-zero on failure).  Error type is `()` because the
/// crate-level `Error` enum does not yet exist.
pub fn set_qlog(_quic: &mut quic_t, _qlog_dir: &str) -> Result<(), Error> {
    todo!()
}

#[cfg(test)]
mod test {}
