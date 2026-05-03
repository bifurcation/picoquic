//! Generic slotmap-style arena.
//!
//! A storage container that hands out [`Token`]s as opaque handles
//! to its values.  Tokens carry a generation counter so that
//! reusing a slot after removal invalidates any outstanding tokens
//! — the safe equivalent of the C `*mut T` use-after-free.
//!
//! Every Phase-4 collection that needs "address an object by
//! handle, not by raw pointer" sits on top of this — most notably
//! the `Quic` connection arena, which holds every live connection
//! and lets the secondary indexes (`cnx_by_id`, `cnx_by_net`, …)
//! store `ConnectionToken`s as their values.
//!
//! Bodies are `todo!()` — Phase 4 either hand-rolls the slotmap
//! (see the layout sketch in [`crate::hash`]) or wraps the
//! `slotmap` crate.  Either way the public API stays as below.

use core::marker::PhantomData;

use crate::Error;

/// Opaque handle into an [`Arena<T>`].  `Copy`, 64 bits total.
///
/// The `T` parameter is purely a phantom — it stops you from using
/// a `Token<Connection>` to index into an `Arena<Stream>`.  The
/// runtime check still lives in the generation counter inside the
/// arena.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Token<T> {
    idx: u32,
    generation: u32,
    _phantom: PhantomData<fn() -> T>,
}

// `Copy` and `Clone` written by hand so they don't depend on `T:
// Copy`.  Tokens are always `Copy` regardless of what `T` is.
impl<T> Copy for Token<T> {}
impl<T> Clone for Token<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Slotmap-style arena that owns `T`s and addresses them by
/// generation-tagged tokens.
///
/// Phase 4 picks an implementation; the API below stays the same.
pub struct Arena<T> {
    /// Phase 4 fills the body.  Sketch:
    ///
    /// ```ignore
    /// struct Slot<T> {
    ///     generation: u32,
    ///     state:      SlotState<T>,
    /// }
    /// enum SlotState<T> { Free { next_free: Option<u32> }, Filled(T) }
    /// pub struct Arena<T> {
    ///     slots: Vec<Slot<T>>,
    ///     free:  Option<u32>,
    ///     len:   usize,
    /// }
    /// ```
    _slots: PhantomData<T>,
}

impl<T> Arena<T> {
    /// Build an empty arena.
    pub const fn new() -> Self {
        Self {
            _slots: PhantomData,
        }
    }

    /// Insert `value`, returning its token.
    ///
    /// Returns [`Error::Memory`] on slot-vector allocation failure.
    pub fn insert(&mut self, _value: T) -> Result<Token<T>, Error> {
        todo!()
    }

    /// Borrow the value at `token`, or `None` if it has been
    /// removed (slot generation has moved on) or is out of bounds.
    pub fn get(&self, _token: Token<T>) -> Option<&T> {
        todo!()
    }

    /// Mutably borrow the value at `token`, or `None` if stale.
    pub fn get_mut(&mut self, _token: Token<T>) -> Option<&mut T> {
        todo!()
    }

    /// Remove and return the value at `token`.  Bumps the slot's
    /// generation; subsequent [`Arena::get`] calls with the same
    /// token return `None`.
    pub fn remove(&mut self, _token: Token<T>) -> Option<T> {
        todo!()
    }

    /// `true` when `token` still names a live value.
    pub fn contains(&self, _token: Token<T>) -> bool {
        todo!()
    }

    /// Number of live values.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when the arena holds no live values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drop every value.  Tokens issued before the call are stale
    /// after it.
    pub fn clear(&mut self) {
        todo!()
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {}
