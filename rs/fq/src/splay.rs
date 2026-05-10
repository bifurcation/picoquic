//! Token-based splay tree.
//!
//! Replaces the C `picosplay_tree_t` (top-down splay with intrusive
//! `picosplay_node_t`s embedded in the caller's structs and four
//! function pointers — `comparator`, `create`, `delete_node`,
//! `node_value` — plumbed through the API).
//!
//! ## API shape
//!
//! `SplayTree<K, V>` is a slotmap-backed ordered tree.  `K: Ord`
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
//! Phase 4: hand-rolled slotmap + splay tree.  One of:
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

extern crate alloc;
use alloc::vec::Vec;

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

// ---------------------------------------------------------------------------
// Internal slot storage

struct Slot<K, V> {
    generation: u32,
    state: SlotState<K, V>,
}

enum SlotState<K, V> {
    Free {
        next_free: Option<u32>,
    },
    Filled {
        key: K,
        value: V,
        parent: Option<u32>,
        left: Option<u32>,
        right: Option<u32>,
    },
}

// ---------------------------------------------------------------------------

/// Splay tree mapping `K` to `V`, addressable by [`SplayToken`].
///
/// Operations rotate the touched node to the root for access
/// locality — that's the whole reason picoquic uses splay rather
/// than red-black or AVL.  `K: Ord` provides comparison; `V` is
/// typically a token into some other arena.
pub struct SplayTree<K, V> {
    slots: Vec<Slot<K, V>>,
    free: Option<u32>,
    root: Option<u32>,
    len: usize,
}

impl<K: Ord, V> SplayTree<K, V> {
    /// Build an empty tree.  C: `picosplay_init_tree` — the C form takes
    /// four callback function pointers (`comparator`, `create`,
    /// `delete_node`, `node_value`); the Rust form uses `K: Ord` generics
    /// and a typed `V`, so no callbacks are needed at construction time.
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: None,
            root: None,
            len: 0,
        }
    }

    /// Build an empty owned tree.
    ///
    /// C: `picosplay_new_tree` (picoquic/picosplay.c:90-97).
    pub const fn new_tree() -> Self {
        Self::new()
    }

    // -----------------------------------------------------------------------
    // Slot helpers

    fn alloc_slot(&mut self, key: K, value: V) -> Result<u32, Error> {
        if let Some(free_idx) = self.free {
            let next_free = match &self.slots[free_idx as usize].state {
                SlotState::Free { next_free } => *next_free,
                SlotState::Filled { .. } => unreachable!(),
            };
            let r#gen = self.slots[free_idx as usize].generation;
            self.slots[free_idx as usize] = Slot {
                generation: r#gen,
                state: SlotState::Filled {
                    key,
                    value,
                    parent: None,
                    left: None,
                    right: None,
                },
            };
            self.free = next_free;
            Ok(free_idx)
        } else {
            let idx = self.slots.len();
            if idx > u32::MAX as usize {
                return Err(Error::Memory);
            }
            self.slots.push(Slot {
                generation: 0,
                state: SlotState::Filled {
                    key,
                    value,
                    parent: None,
                    left: None,
                    right: None,
                },
            });
            Ok(idx as u32)
        }
    }

    fn release_slot(&mut self, idx: u32) -> (K, V) {
        let r#gen = self.slots[idx as usize].generation.wrapping_add(1);
        let old = core::mem::replace(
            &mut self.slots[idx as usize],
            Slot {
                generation: r#gen,
                state: SlotState::Free {
                    next_free: self.free,
                },
            },
        );
        self.free = Some(idx);
        match old.state {
            SlotState::Filled { key, value, .. } => (key, value),
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn token_of(&self, idx: u32) -> SplayToken {
        SplayToken {
            idx,
            generation: self.slots[idx as usize].generation,
        }
    }

    fn is_valid(&self, t: SplayToken) -> bool {
        let i = t.idx as usize;
        i < self.slots.len()
            && matches!(self.slots[i].state, SlotState::Filled { .. })
            && self.slots[i].generation == t.generation
    }

    // -----------------------------------------------------------------------
    // Field accessors

    fn key_of(&self, idx: u32) -> &K {
        match &self.slots[idx as usize].state {
            SlotState::Filled { key, .. } => key,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn value_of(&self, idx: u32) -> &V {
        match &self.slots[idx as usize].state {
            SlotState::Filled { value, .. } => value,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn value_of_mut(&mut self, idx: u32) -> &mut V {
        match &mut self.slots[idx as usize].state {
            SlotState::Filled { value, .. } => value,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn parent_of(&self, idx: u32) -> Option<u32> {
        match &self.slots[idx as usize].state {
            SlotState::Filled { parent, .. } => *parent,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn left_of(&self, idx: u32) -> Option<u32> {
        match &self.slots[idx as usize].state {
            SlotState::Filled { left, .. } => *left,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn right_of(&self, idx: u32) -> Option<u32> {
        match &self.slots[idx as usize].state {
            SlotState::Filled { right, .. } => *right,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn set_parent(&mut self, idx: u32, p: Option<u32>) {
        match &mut self.slots[idx as usize].state {
            SlotState::Filled { parent, .. } => *parent = p,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn set_left(&mut self, idx: u32, l: Option<u32>) {
        match &mut self.slots[idx as usize].state {
            SlotState::Filled { left, .. } => *left = l,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn set_right(&mut self, idx: u32, r: Option<u32>) {
        match &mut self.slots[idx as usize].state {
            SlotState::Filled { right, .. } => *right = r,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    // -----------------------------------------------------------------------
    // Splay rotations (C: picosplay.c rotate / mark_gp / splay)

    /// Reattach `child` under its grandparent, updating the grandparent's
    /// child pointer and the parent pointers of both `child` and `parent`.
    /// C: `mark_gp` (`picosplay.c:307`).
    fn mark_gp(&mut self, child: u32) {
        let parent = self.parent_of(child).expect("mark_gp requires a parent");
        let grand = self.parent_of(parent);
        self.set_parent(child, grand);
        self.set_parent(parent, Some(child));
        if let Some(g) = grand {
            if self.left_of(g) == Some(parent) {
                self.set_left(g, Some(child));
            } else {
                self.set_right(g, Some(child));
            }
        }
    }

    fn rotate(&mut self, child: u32) {
        let parent = self.parent_of(child).expect("rotate requires a parent");
        let is_left = self.left_of(parent) == Some(child);
        self.mark_gp(child);

        if is_left {
            let cr = self.right_of(child);
            self.set_left(parent, cr);
            if let Some(cr) = cr {
                self.set_parent(cr, Some(parent));
            }
            self.set_right(child, Some(parent));
        } else {
            let cl = self.left_of(child);
            self.set_right(parent, cl);
            if let Some(cl) = cl {
                self.set_parent(cl, Some(parent));
            }
            self.set_left(child, Some(parent));
        }
    }

    /// C: `zig` (picoquic/picosplay.c:59-62).
    fn zig(&mut self, x: u32) {
        self.rotate(x);
    }

    /// C: `zigzig` (picoquic/picosplay.c:64-70).
    fn zigzig(&mut self, x: u32, p: u32) {
        self.rotate(p);
        self.rotate(x);
    }

    /// C: `zigzag` (picoquic/picosplay.c:72-78).
    fn zigzag(&mut self, x: u32) {
        self.rotate(x);
        self.rotate(x);
    }

    fn splay(&mut self, idx: u32) {
        loop {
            let p = match self.parent_of(idx) {
                None => {
                    self.root = Some(idx);
                    return;
                }
                Some(p) => p,
            };
            match self.parent_of(p) {
                None => self.zig(idx),
                Some(g) => {
                    let idx_left = self.left_of(p) == Some(idx);
                    let p_left = self.left_of(g) == Some(p);
                    if idx_left == p_left {
                        self.zigzig(idx, p);
                    } else {
                        self.zigzag(idx);
                    }
                }
            }
        }
    }

    fn leftmost(&self, start: Option<u32>) -> Option<u32> {
        let mut cur = start?;
        loop {
            match self.left_of(cur) {
                None => return Some(cur),
                Some(l) => cur = l,
            }
        }
    }

    fn rightmost(&self, start: Option<u32>) -> Option<u32> {
        let mut cur = start?;
        loop {
            match self.right_of(cur) {
                None => return Some(cur),
                Some(r) => cur = r,
            }
        }
    }

    // -----------------------------------------------------------------------
    // Public API

    /// Insert `(key, value)`, splay it to the root, and return its
    /// token.  Matching C `picosplay_insert`, equal keys are preserved
    /// as distinct nodes and insertion descends right on equality.
    ///
    /// Returns [`Error::Memory`] on slot-vector allocation failure.
    pub fn insert(&mut self, key: K, value: V) -> Result<(SplayToken, Option<V>), Error> {
        if self.root.is_none() {
            let idx = self.alloc_slot(key, value)?;
            self.root = Some(idx);
            self.len += 1;
            return Ok((self.token_of(idx), None));
        }

        let mut cur = self.root.unwrap();
        let mut par;
        let mut go_left;
        loop {
            par = cur;
            let cmp = key.cmp(self.key_of(cur));
            go_left = cmp == core::cmp::Ordering::Less;
            let next = if go_left {
                self.left_of(cur)
            } else {
                self.right_of(cur)
            };
            match next {
                None => break,
                Some(n) => cur = n,
            }
        }

        let idx = self.alloc_slot(key, value)?;
        self.set_parent(idx, Some(par));
        if go_left {
            self.set_left(par, Some(idx));
        } else {
            self.set_right(par, Some(idx));
        }
        self.splay(idx);
        self.len += 1;
        Ok((self.token_of(idx), None))
    }

    /// Insert or replace `(key, value)` as a map-style operation.
    ///
    /// This is intentionally separate from [`SplayTree::insert`], which
    /// mirrors C `picosplay_insert` and preserves duplicate keys.
    pub fn upsert(&mut self, key: K, value: V) -> Result<(SplayToken, Option<V>), Error> {
        if let Some(tok) = self.find(&key) {
            let old = core::mem::replace(self.value_of_mut(tok.idx), value);
            return Ok((tok, Some(old)));
        }

        self.insert(key, value)
    }

    /// Look up `key`, splaying the matching node to the root.
    /// Returns its token, or `None` on miss.  C: `picosplay_find`.
    pub fn find(&mut self, key: &K) -> Option<SplayToken> {
        let mut cur = self.root?;
        loop {
            match key.cmp(self.key_of(cur)) {
                core::cmp::Ordering::Equal => {
                    self.splay(cur);
                    return Some(self.token_of(cur));
                }
                core::cmp::Ordering::Less => match self.left_of(cur) {
                    None => return None,
                    Some(l) => cur = l,
                },
                core::cmp::Ordering::Greater => match self.right_of(cur) {
                    None => return None,
                    Some(r) => cur = r,
                },
            }
        }
    }

    /// Locate the largest node whose key is `<= key`, without
    /// splaying.  C: `picosplay_find_previous`.  Returns `None`
    /// when no node is small enough.
    pub fn find_previous(&self, key: &K) -> Option<SplayToken> {
        let mut cur = self.root?;
        let mut prev: Option<u32> = None;
        loop {
            let cmp = key.cmp(self.key_of(cur));
            if cmp == core::cmp::Ordering::Equal {
                return Some(self.token_of(cur));
            } else if cmp == core::cmp::Ordering::Less {
                match self.left_of(cur) {
                    None => break,
                    Some(l) => cur = l,
                }
            } else {
                prev = Some(cur);
                match self.right_of(cur) {
                    None => break,
                    Some(r) => cur = r,
                }
            }
        }
        prev.map(|p| self.token_of(p))
    }

    /// Smallest (left-most) node, or `None` for an empty tree.
    /// C: `picosplay_first`.
    pub fn first(&self) -> Option<SplayToken> {
        self.leftmost(self.root).map(|i| self.token_of(i))
    }

    /// Largest (right-most) node, or `None` for an empty tree.
    /// C: `picosplay_last`.
    pub fn last(&self) -> Option<SplayToken> {
        self.rightmost(self.root).map(|i| self.token_of(i))
    }

    /// In-order predecessor of `token`, or `None` if it is the
    /// minimum.  C: `picosplay_previous`.
    pub fn previous(&self, token: SplayToken) -> Option<SplayToken> {
        if !self.is_valid(token) {
            return None;
        }
        let idx = token.idx;
        if let Some(l) = self.left_of(idx) {
            return self.rightmost(Some(l)).map(|p| self.token_of(p));
        }
        let mut node = idx;
        loop {
            match self.parent_of(node) {
                None => return None,
                Some(p) => {
                    if self.left_of(p) == Some(node) {
                        node = p;
                    } else {
                        return Some(self.token_of(p));
                    }
                }
            }
        }
    }

    /// In-order successor of `token`, or `None` if it is the
    /// maximum.  C: `picosplay_next`.
    pub fn next(&self, token: SplayToken) -> Option<SplayToken> {
        if !self.is_valid(token) {
            return None;
        }
        let idx = token.idx;
        if let Some(r) = self.right_of(idx) {
            return self.leftmost(Some(r)).map(|n| self.token_of(n));
        }
        let mut node = idx;
        loop {
            match self.parent_of(node) {
                None => return None,
                Some(p) => {
                    if self.right_of(p) == Some(node) {
                        node = p;
                    } else {
                        return Some(self.token_of(p));
                    }
                }
            }
        }
    }

    /// Borrow the value at `token`, or `None` if stale or out of
    /// bounds.
    pub fn get(&self, token: SplayToken) -> Option<&V> {
        if !self.is_valid(token) {
            return None;
        }
        Some(self.value_of(token.idx))
    }

    /// Mutably borrow the value at `token`, or `None` if stale.
    pub fn get_mut(&mut self, token: SplayToken) -> Option<&mut V> {
        if !self.is_valid(token) {
            return None;
        }
        Some(self.value_of_mut(token.idx))
    }

    /// Borrow the `(key, value)` pair at `token`.
    pub fn get_key_value(&self, token: SplayToken) -> Option<(&K, &V)> {
        if !self.is_valid(token) {
            return None;
        }
        Some((self.key_of(token.idx), self.value_of(token.idx)))
    }

    /// Remove the entry at `token`.  Bumps the slot's generation.
    /// Returns `None` for stale tokens.  C: `picosplay_delete_hint`.
    pub fn remove(&mut self, token: SplayToken) -> Option<(K, V)> {
        if !self.is_valid(token) {
            return None;
        }
        let node = token.idx;
        self.splay(node);

        let left = self.left_of(node);
        let right = self.right_of(node);

        match (left, right) {
            (None, _) => {
                self.root = right;
                if let Some(r) = right {
                    self.set_parent(r, None);
                }
            }
            (left, None) => {
                self.root = left;
                if let Some(l) = left {
                    self.set_parent(l, None);
                }
            }
            (Some(l), Some(r)) => {
                let x = self.leftmost(Some(r)).unwrap();
                if self.parent_of(x) != Some(node) {
                    let xr = self.right_of(x);
                    let xp = self.parent_of(x).unwrap();
                    self.set_left(xp, xr);
                    if let Some(xr) = xr {
                        self.set_parent(xr, Some(xp));
                    }
                    self.set_right(x, Some(r));
                    self.set_parent(r, Some(x));
                }
                self.set_left(x, Some(l));
                self.set_parent(l, Some(x));
                self.root = Some(x);
                self.set_parent(x, None);
            }
        }

        self.len -= 1;
        Some(self.release_slot(node))
    }

    /// Remove the entry matching `key`.  Prefer [`SplayTree::remove`]
    /// when a token is on hand.  C: `picosplay_delete`.
    pub fn remove_by_key(&mut self, key: &K) -> Option<V> {
        let tok = self.find(key)?;
        self.remove(tok).map(|(_, v)| v)
    }

    /// Number of live entries.
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` when the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drop every entry.  Tokens issued before the call are stale
    /// after it.  C: `picosplay_empty_tree`.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free = None;
        self.root = None;
        self.len = 0;
    }
}

impl<K: Ord, V> Default for SplayTree<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::SplayTree;

    #[test]
    fn insert_preserves_duplicate_keys() {
        let mut tree = SplayTree::new();

        let (first, old) = tree.insert(7, "first").unwrap();
        assert!(old.is_none());
        let (second, old) = tree.insert(7, "second").unwrap();
        assert!(old.is_none());

        assert_ne!(first, second);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree.get(first), Some(&"first"));
        assert_eq!(tree.get(second), Some(&"second"));

        let found = tree.find(&7).unwrap();
        assert_eq!(found, second);

        assert_eq!(tree.remove_by_key(&7), Some("second"));
        assert_eq!(tree.len(), 1);
        let found = tree.find(&7).unwrap();
        assert_eq!(found, first);
    }

    #[test]
    fn upsert_replaces_existing_key() {
        let mut tree = SplayTree::new();

        let (first, old) = tree.upsert(3, "old").unwrap();
        assert!(old.is_none());
        let (second, old) = tree.upsert(3, "new").unwrap();

        assert_eq!(first, second);
        assert_eq!(old, Some("old"));
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.get(second), Some(&"new"));
    }
}
