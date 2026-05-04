//! Token-based splay tree.
//!
//! Replaces the C `picosplay_tree_t` (top-down splay with intrusive
//! `picosplay_node_t`s embedded in the caller's structs and four
//! function pointers — `comparator`, `create`, `delete_node`,
//! `node_value` — plumbed through the API).
//!
//! ## API shape
//!
//! `SplayTree<K, V>` is a slotmap-backed ordered map.  `K: Ord`
//! provides comparison; `V` is the stored value (typically a
//! token into another arena).  Callers handle [`SplayToken`]s
//! returned by `insert`/`find`; the parent/left/right linkage
//! lives inside the slot, never inside the caller's struct.
//!
//! Tokens are stable across operations on *other* keys.  A splay
//! rotation that lifts node A to the root rewrites the linkage of
//! the rotation path, but slot indices don't move and tokens for
//! those nodes stay valid.  Removal bumps the slot's generation
//! and frees the slot for reuse — old tokens then return `None`
//! from [`SplayTree::get`].
//!
//! ## Phase 4 plan — implementation
//!
//! Bodies are `todo!()`.  Phase 4 picks one of:
//!
//! 1. **Hand-roll the slotmap.**  Internal layout:
//!
//!    ```ignore
//!    struct Slot<K, V> {
//!        generation: u32,
//!        state: SlotState<K, V>,
//!    }
//!    enum SlotState<K, V> {
//!        Free   { next_free: Option<u32> },
//!        Filled {
//!            key:    K,
//!            value:  V,
//!            parent: Option<u32>,
//!            left:   Option<u32>,
//!            right:  Option<u32>,
//!        },
//!    }
//!    pub struct SplayTree<K, V> {
//!        slots: Vec<Slot<K, V>>,
//!        free:  Option<u32>,
//!        root:  Option<u32>,
//!        len:   usize,
//!    }
//!    ```
//!
//!    Standard top-down splay over `u32` indices.  Every "follow a
//!    pointer" in the C body becomes "index into `self.slots`."
//!
//! 2. **Replace with a third-party crate.**  `splay_tree`,
//!    `splay-tree`, or BTreeMap (if log-N is fine and we don't
//!    actually need access locality).  picoquic uses splay
//!    specifically for the access-locality property in
//!    `connection_wake_tree` (the next-to-fire connection is usually the
//!    one we just touched).  `BTreeMap` is the safe-default
//!    fallback; a real splay crate is the performance-preserving
//!    choice.
//!
//! ## Phase 4 plan — call sites
//!
//! Same pattern as the hash table.  Every C site of the form
//!
//! ```c
//! struct Foo { …; picosplay_node_t node; …; };
//! picosplay_init_tree(&tree, foo_compare, foo_create, foo_delete, foo_value);
//! picosplay_insert(&tree, &foo);     // value = pointer to parent
//! ```
//!
//! becomes
//!
//! ```ignore
//! struct Foo {
//!     // … real fields …
//!     wake_tree_membership: Option<SplayToken>,   // for fast O(1) removal
//! }
//! let tok = tree.insert(key, foo_arena_token);
//! foo.wake_tree_membership = Some(tok);
//! // …
//! tree.remove(foo.wake_tree_membership.take().unwrap());
//! ```
//!
//! In particular, every C `_create` callback (which just projected
//! the embedded `SplayNode` out of a parent pointer) disappears —
//! the parent already lives in its own arena, the splay tree only
//! stores a token to it.

use core::marker::PhantomData;

use crate::Error;

/// Opaque handle into a [`SplayTree`]'s slot vector.
///
/// `idx` selects a slot; `generation` is incremented on every
/// removal so an old token comparing against a recycled slot
/// returns `None` from [`SplayTree::get`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SplayToken {
    idx: u32,
    generation: u32,
}

/// Splay tree mapping `K` to `V`, addressable by [`SplayToken`].
///
/// Operations rotate the touched node to the root for access
/// locality — that's the whole reason picoquic uses splay rather
/// than red-black or AVL.  `K: Ord` provides comparison; `V` is
/// typically a token into some other arena.
pub struct SplayTree<K, V> {
    /// Phase 4 fills the body — see module docs.
    _slots: PhantomData<(K, V)>,
}

impl<K: Ord, V> SplayTree<K, V> {
    /// Build an empty tree.
    pub const fn new() -> Self {
        Self {
            _slots: PhantomData,
        }
    }

    /// Insert `(key, value)`, splay it to the root, and return its
    /// token.  If `key` was already present, the previous value is
    /// replaced and returned in the `Ok` payload.
    ///
    /// Returns [`Error::Memory`] on slot-vector allocation failure.
    pub fn insert(&mut self, _key: K, _value: V) -> Result<(SplayToken, Option<V>), Error> {
        todo!()
    }

    /// Look up `key`, splaying the matching node to the root.
    /// Returns its token, or `None` on miss.  C: `picosplay_find`.
    pub fn find(&mut self, _key: &K) -> Option<SplayToken> {
        todo!()
    }

    /// Locate the largest node whose key is `<= key`, without
    /// splaying.  C: `picosplay_find_previous`.  Returns `None`
    /// when no node is small enough.
    pub fn find_previous(&self, _key: &K) -> Option<SplayToken> {
        todo!()
    }

    /// Smallest (left-most) node, or `None` for an empty tree.
    /// C: `picosplay_first`.
    pub fn first(&self) -> Option<SplayToken> {
        todo!()
    }

    /// Largest (right-most) node, or `None` for an empty tree.
    /// C: `picosplay_last`.
    pub fn last(&self) -> Option<SplayToken> {
        todo!()
    }

    /// In-order predecessor of `token`, or `None` if it is the
    /// minimum.  C: `picosplay_previous`.
    pub fn previous(&self, _token: SplayToken) -> Option<SplayToken> {
        todo!()
    }

    /// In-order successor of `token`, or `None` if it is the
    /// maximum.  C: `picosplay_next`.
    pub fn next(&self, _token: SplayToken) -> Option<SplayToken> {
        todo!()
    }

    /// Borrow the value at `token`, or `None` if stale or out of
    /// bounds.
    pub fn get(&self, _token: SplayToken) -> Option<&V> {
        todo!()
    }

    /// Mutably borrow the value at `token`, or `None` if stale.
    pub fn get_mut(&mut self, _token: SplayToken) -> Option<&mut V> {
        todo!()
    }

    /// Borrow the `(key, value)` pair at `token`.
    pub fn get_key_value(&self, _token: SplayToken) -> Option<(&K, &V)> {
        todo!()
    }

    /// Remove the entry at `token` (O(1) once located via the
    /// stored membership token).  Bumps the slot's generation.
    /// Returns `None` for stale tokens.  C: `picosplay_delete_hint`.
    pub fn remove(&mut self, _token: SplayToken) -> Option<(K, V)> {
        todo!()
    }

    /// Remove the entry matching `key` (locating it via the tree
    /// first — splay walk, then unlink).  Prefer [`SplayTree::remove`]
    /// when a token is on hand.  C: `picosplay_delete`.
    pub fn remove_by_key(&mut self, _key: &K) -> Option<V> {
        todo!()
    }

    /// Number of live entries.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drop every entry.  Tokens issued before the call are stale
    /// after it.  C: `picosplay_empty_tree`.
    pub fn clear(&mut self) {
        todo!()
    }
}

impl<K: Ord, V> Default for SplayTree<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {}
