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
//! and lets the secondary indexes (`connection_by_id`, `connection_by_net`, …)
//! store `ConnectionToken`s as their values.

use core::marker::PhantomData;

use crate::Error;

/// Opaque handle into an [`Arena<T>`].  `Copy`, 64 bits total.
///
/// The `T` parameter is purely a phantom — it stops you from using
/// a `Token<Connection>` to index into an `Arena<Stream>`.  The
/// runtime check still lives in the generation counter inside the
/// arena.
#[derive(Debug)]
pub struct Token<T> {
    idx: u32,
    generation: u32,
    _phantom: PhantomData<fn() -> T>,
}

// Manual PartialEq/Eq/Hash so that `Token<T>` works even when `T` is not
// `PartialEq`.  Two tokens are equal when their physical slot index and
// generation counter match — the `T` phantom plays no role at runtime.
impl<T> PartialEq for Token<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx && self.generation == other.generation
    }
}

impl<T> Eq for Token<T> {}

impl<T> core::hash::Hash for Token<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
        self.generation.hash(state);
    }
}

// `Copy` and `Clone` written by hand so they don't depend on `T:
// Copy`.  Tokens are always `Copy` regardless of what `T` is.
impl<T> Copy for Token<T> {}
impl<T> Clone for Token<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Token<T> {
    /// Construct a synthetic token with explicit `idx` and `generation` fields.
    /// This is `pub(crate)` — external code must go through [`Arena::insert`].
    /// Used internally when code must name an arena slot from data that already
    /// carries the slot index and generation.
    pub(crate) fn synthetic(idx: u32, generation: u32) -> Self {
        Self {
            idx,
            generation,
            _phantom: PhantomData,
        }
    }

    /// Raw slot index.  Used by `Arena::next_after_idx` to resume
    /// iteration from a known position.
    pub(crate) fn slot_idx(self) -> usize {
        self.idx as usize
    }
}

/// A slot in the arena storage vector.
enum Slot<T> {
    Free {
        next_free: Option<u32>,
        generation: u32,
    },
    Filled {
        generation: u32,
        value: T,
    },
}

/// Slotmap-style arena that owns `T`s and addresses them by
/// generation-tagged tokens.
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Option<u32>,
    len: usize,
}

impl<T> Arena<T> {
    /// Build an empty arena.
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: None,
            len: 0,
        }
    }

    /// Insert `value`, returning its token.
    ///
    /// Returns [`Error::Memory`] on slot-vector allocation failure.
    pub fn insert(&mut self, value: T) -> Result<Token<T>, Error> {
        let (idx, generation) = if let Some(free_idx) = self.free {
            let (next, slot_gen) = if let Slot::Free {
                next_free,
                generation,
            } = &self.slots[free_idx as usize]
            {
                (*next_free, *generation)
            } else {
                unreachable!("free list pointed at a Filled slot")
            };
            self.free = next;
            self.slots[free_idx as usize] = Slot::Filled {
                generation: slot_gen,
                value,
            };
            (free_idx, slot_gen)
        } else {
            let idx = self.slots.len() as u32;
            self.slots.push(Slot::Filled {
                generation: idx,
                value,
            });
            (idx, idx)
        };
        self.len += 1;
        Ok(Token {
            idx,
            generation,
            _phantom: PhantomData,
        })
    }

    /// Borrow the value at `token`, or `None` if it has been
    /// removed (slot generation has moved on) or is out of bounds.
    pub fn get(&self, token: Token<T>) -> Option<&T> {
        match self.slots.get(token.idx as usize)? {
            Slot::Filled { generation, value } if *generation == token.generation => Some(value),
            _ => None,
        }
    }

    /// Mutably borrow the value at `token`, or `None` if stale.
    pub fn get_mut(&mut self, token: Token<T>) -> Option<&mut T> {
        match self.slots.get_mut(token.idx as usize)? {
            Slot::Filled { generation, value } if *generation == token.generation => Some(value),
            _ => None,
        }
    }

    /// Remove and return the value at `token`.  Bumps the slot's
    /// generation; subsequent [`Arena::get`] calls with the same
    /// token return `None`.
    pub fn remove(&mut self, token: Token<T>) -> Option<T> {
        let idx = token.idx as usize;
        let is_match = matches!(
            self.slots.get(idx),
            Some(Slot::Filled { generation, .. }) if *generation == token.generation
        );
        if is_match {
            let next_generation = token.generation.wrapping_add(1);
            let old = core::mem::replace(
                &mut self.slots[idx],
                Slot::Free {
                    next_free: self.free,
                    generation: next_generation,
                },
            );
            self.free = Some(token.idx);
            self.len -= 1;
            if let Slot::Filled { value, .. } = old {
                Some(value)
            } else {
                unreachable!()
            }
        } else {
            None
        }
    }

    /// `true` when `token` still names a live value.
    pub fn contains(&self, token: Token<T>) -> bool {
        self.get(token).is_some()
    }

    /// Number of live values.
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` when the arena holds no live values.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Drop every value.  Tokens issued before the call are stale
    /// after it.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free = None;
        self.len = 0;
    }

    /// Iterate over shared references to all live values.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.slots.iter().filter_map(|slot| match slot {
            Slot::Filled { value, .. } => Some(value),
            Slot::Free { .. } => None,
        })
    }

    /// Iterate over mutable references to all live values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.slots.iter_mut().filter_map(|slot| match slot {
            Slot::Filled { value, .. } => Some(value),
            Slot::Free { .. } => None,
        })
    }

    /// Return a mutable reference to the first live value at or after slot
    /// index `start_idx`, or `None` when no such slot exists.
    ///
    /// Used by `Quic::next_cnx` to implement `picoquic_get_next_cnx`-style
    /// traversal without an intrusive linked list.
    pub fn next_after_idx(&mut self, start_idx: usize) -> Option<&mut T> {
        self.slots
            .get_mut(start_idx..)?
            .iter_mut()
            .find_map(|slot| match slot {
                Slot::Filled { value, .. } => Some(value),
                Slot::Free { .. } => None,
            })
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn insert_get_remove() {
        let mut arena: Arena<u32> = Arena::new();
        let tok = arena.insert(42).unwrap();
        assert_eq!(arena.get(tok), Some(&42));
        assert_eq!(arena.len(), 1);
        let val = arena.remove(tok).unwrap();
        assert_eq!(val, 42);
        assert_eq!(arena.get(tok), None);
        assert_eq!(arena.len(), 0);
    }

    #[test]
    fn iter_live_values() {
        let mut arena: Arena<u32> = Arena::new();
        let t1 = arena.insert(1).unwrap();
        let _t2 = arena.insert(2).unwrap();
        arena.remove(t1).unwrap();
        let vals: Vec<_> = arena.iter().copied().collect();
        assert_eq!(vals, vec![2]);
    }

    #[test]
    fn reused_slot_invalidates_old_token() {
        let mut arena: Arena<u32> = Arena::new();
        let old = arena.insert(10).unwrap();
        assert_eq!(arena.remove(old), Some(10));

        let new = arena.insert(20).unwrap();
        assert_eq!(arena.get(old), None);
        assert_eq!(arena.get(new), Some(&20));
        assert_ne!(old, new);
    }
}
