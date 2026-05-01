//! Translation of `picoquic/picoquic_qlog.h`.
//!
//! Header-only module exposing a single entry point that turns
//! on per-connection qlog tracing for a QUIC context.  The
//! implementation lives in `loglib/autoqlog.c` (out of scope for
//! v1, since `loglib/` is a separate target — this header is the
//! picoquic-core stub that links against it on demand).

use crate::picoquic_quic_t;

/// Set the qlog directory and start streaming qlog traces for
/// each connection.  C: `int picoquic_set_qlog(picoquic_quic_t*
/// quic, char const* qlog_dir)`.
///
/// Pointer-shape choices:
///
/// * `quic` → `&mut picoquic_quic_t`.  Mutates the unified-logging
///   function-pointer slots on the QUIC context; no in-tree caller
///   passes NULL.
/// * `qlog_dir` → `&str`.  Every caller in `picoquic-core` and the
///   sample apps passes a non-NULL path string (often `"."` or a
///   `config->qlog_dir` field that is populated before the call).
///   Translation note: the C parameter is `char const*` so NULL is
///   technically representable, but no caller does so — `&str`
///   matches the actual contract.  Swap to `Option<&str>` if a
///   future caller needs to clear the directory.
///
/// Returns `Result<(), ()>` to mirror the C status code (`0` on
/// success, non-zero on failure).  Error type is `()` because the
/// crate-level `Error` enum does not yet exist.
// TODO(error-enum): swap `()` for the crate's `Error` once it lands.
#[allow(clippy::result_unit_err)]
pub fn picoquic_set_qlog(_quic: &mut picoquic_quic_t, _qlog_dir: &str) -> Result<(), ()> {
    todo!()
}

#[cfg(test)]
mod test {}
