//! Translation of `quic/qlog.h`.
//!
//! Header-only module exposing a single entry point that turns
//! on per-connection qlog tracing for a QUIC context.  The body
//! lives in `loglib/autoqlog.c` (a separate target, out of scope
//! for v1); this module is the picoquic-core stub that links
//! against it on demand.

use std::path::Path;

use crate::{Quic, Result};

impl Quic {
    /// Enable qlog tracing on this context, writing one qlog file per
    /// connection into `qlog_dir`.  Installs the unified-logging
    /// vtable; subsequent connections on this context will stream
    /// qlog records until the context is dropped.
    ///
    /// Both observed call sites guard on the directory being set
    /// before calling, so the parameter is borrowed (no
    /// `Option<…>`); the C `int` status (0 / -1) becomes
    /// `Result<()>`.
    ///
    /// C: `int set_qlog(Quic*, char const*)`.
    pub fn set_qlog(&mut self, _qlog_dir: &(impl AsRef<Path> + ?Sized)) -> Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod test {}
