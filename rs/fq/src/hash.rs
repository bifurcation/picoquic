//! Token-based hash table.
//!
//! Replaces the C `picohash_table` (an open-addressed table with
//! chained collision lists, intrusive `picohash_item`s embedded in
//! the caller's structs and a function-pointer trio for hashing /
//! comparing / projecting the embedded item).
//!
//! ## API shape
//!
//! `HashTable<K, V>` is a slotmap-backed hash map.  `K: Hash + Eq`
//! provides the key contract; `V` is whatever the caller stores
//! (usually a token into some other arena — see Phase-4 plan
//! below).  The trio of C function pointers is gone: hashing comes
//! from `K`'s `Hash` impl, comparison from `Eq`, and the table
//! never reaches into the caller's struct so no `key_to_item`
//! callback is needed.
//!
//! Operations return [`HashToken`]s — opaque (`idx, generation`)
//! handles into this table's slot vector.  Tokens are `Copy`,
//! survive insertion and removal of *other* keys, but become stale
//! the moment the keyed entry is removed (the slot's generation
//! bumps).  [`HashTable::get`] returns `None` for stale tokens —
//! the safe equivalent of the C dangling-`HashItem*` use-after-free.
//!
//! ## Phase 4 plan — implementation
//!
//! The bodies in this module are `todo!()`.  Phase 4 picks one of:
//!
//! 1. **Hand-roll the slotmap.**  Internal layout (illustrative):
//!
//!    ```ignore
//!    struct Slot<K, V> {
//!        generation: u32,
//!        state: SlotState<K, V>,
//!    }
//!    enum SlotState<K, V> {
//!        Free   { next_free: Option<u32> },
//!        Filled { hash: u64, key: K, value: V, next_in_bin: Option<u32> },
//!    }
//!    pub struct HashTable<K, V> {
//!        slots: Vec<Slot<K, V>>,
//!        free:  Option<u32>,
//!        bins:  Vec<Option<u32>>,
//!        seed:  [u8; 16],
//!        len:   usize,
//!    }
//!    ```
//!
//!    `next_in_bin` lives inside the slot, not the user struct, so
//!    no `unsafe`.  Lookup hashes `K` to a bin index, walks the
//!    chain by following `next_in_bin` slot indices, comparing each
//!    `slot.key` against the target.  Insert pushes a new slot
//!    (or pops one off the free list) and threads it onto the bin
//!    chain.  Remove unlinks, marks the slot `Free`, bumps
//!    generation, and adds it to the free list.
//!
//! 2. **Build on the `slotmap` crate.**  `slotmap::SlotMap` already
//!    provides the slot/generation/free-list machinery; this module
//!    layers the bin-array indexing on top.  The crate is
//!    `no_std`-friendly with the right features and would let us
//!    delete maybe 80% of the implementation.
//!
//! Either implementation should keep `HashToken` byte-compatible
//! with the public API below — the choice is local.
//!
//! ## Phase 4 plan — call sites
//!
//! In the C tree, every hash table is intrusive: the value type
//! embeds a `HashItem` field and the table's bins point straight
//! into the middle of those user-owned objects.  In the Rust port
//! the embedding turns into a *membership token*: the parent
//! struct holds `Option<HashToken>` for each table it can be in,
//! and the table's slot stores the parent's `ConnectionToken`
//! (or other token type) into the arena that owns the parent.
//!
//! For example, `quic.cnx_by_id: HashTable<ConnectionId,
//! ConnectionToken>` points at connections that live in
//! `quic.connections: Arena<Connection>`.  A `Connection` carries
//! `cnx_by_id_membership: Option<HashToken>`; on removal the
//! Connection hands that token back for an O(1) `cnx_by_id.remove`.

use core::hash::Hash;
use core::marker::PhantomData;

use crate::Error;

/// Opaque handle into a [`HashTable`]'s slot vector.
///
/// `idx` selects a slot; `generation` is incremented on every
/// removal so an old token comparing against a recycled slot
/// returns `None` from [`HashTable::get`].  The pair is `Copy` and
/// 64-bit total — call sites pass it by value.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct HashToken {
    idx: u32,
    generation: u32,
}

/// Hash table mapping `K` to `V`, addressable by [`HashToken`].
///
/// The value `V` is typically a token into some other arena (see
/// the Phase-4 call-site plan in the module docs) but any owned
/// value works.  `K: Hash + Eq` is the only contract on the key.
pub struct HashTable<K, V> {
    /// Phase 4 fills the body — see module docs for the slot /
    /// bin / free-list shape.
    _slots: PhantomData<(K, V)>,
}

impl<K: Hash + Eq, V> HashTable<K, V> {
    /// Build an empty table with `nb_bin` collision bins, using a
    /// zero hash seed.
    ///
    /// `nb_bin` is fixed at construction (the C source never
    /// resized any of its tables); pick a value comfortably larger
    /// than the expected entry count.
    ///
    /// C: `picohash_create` (with the per-table `key_to_item`
    /// callback dropped — Rust never reaches into the user struct).
    pub fn new(_nb_bin: usize) -> Result<Self, Error> {
        todo!()
    }

    /// Like [`HashTable::new`] but with a caller-supplied 16-byte
    /// hash seed.  picoquic uses one shared seed across every
    /// per-`Quic` table (see `Quic::hash_seed`) so independent
    /// `Quic` instances don't share collision patterns.
    ///
    /// C: `picohash_create_ex`.
    pub fn with_seed(_nb_bin: usize, _hash_seed: &[u8; 16]) -> Result<Self, Error> {
        todo!()
    }

    /// Insert `(key, value)` and return a [`HashToken`] that can be
    /// used to retrieve or remove the entry without re-hashing.
    /// If `key` was already present, the previous value is replaced
    /// and returned in the `Ok` payload.
    ///
    /// Returns [`Error::Memory`] on slot-vector allocation failure.
    pub fn insert(&mut self, _key: K, _value: V) -> Result<(HashToken, Option<V>), Error> {
        todo!()
    }

    /// Look up `key`, returning the matching token or `None`.
    /// Equivalent to `picohash_retrieve` but typed.
    pub fn lookup(&self, _key: &K) -> Option<HashToken> {
        todo!()
    }

    /// Borrow the value at `token`, or `None` if the token is stale
    /// (the slot has been recycled) or out of bounds.
    pub fn get(&self, _token: HashToken) -> Option<&V> {
        todo!()
    }

    /// Mutably borrow the value at `token`, or `None` if the token
    /// is stale.
    pub fn get_mut(&mut self, _token: HashToken) -> Option<&mut V> {
        todo!()
    }

    /// Borrow the `(key, value)` pair at `token`.  Useful when the
    /// value is a token into another arena and the caller wants
    /// the original key for re-derivation.
    pub fn get_key_value(&self, _token: HashToken) -> Option<(&K, &V)> {
        todo!()
    }

    /// Remove and return the entry at `token`.  Bumps the slot's
    /// generation; subsequent [`HashTable::get`] calls with the
    /// same token return `None`.  Returns `None` if the token was
    /// already stale.
    ///
    /// O(1) — no key re-hashing.  This is the path Phase-4 call
    /// sites should prefer over [`HashTable::remove_by_key`] when
    /// they hold a stored membership token.
    pub fn remove(&mut self, _token: HashToken) -> Option<(K, V)> {
        todo!()
    }

    /// Remove the entry matching `key`, hashing and walking the bin
    /// chain to find it.  O(1) amortized but pays a hash + chain
    /// walk; prefer [`HashTable::remove`] when a token is on hand.
    pub fn remove_by_key(&mut self, _key: &K) -> Option<V> {
        todo!()
    }

    /// `true` if `key` is currently in the table.
    pub fn contains_key(&self, _key: &K) -> bool {
        todo!()
    }

    /// Number of live entries.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drop every entry.  Tokens issued before the call are stale
    /// after it.
    pub fn clear(&mut self) {
        todo!()
    }
}

/// picoquic's bespoke byte-string hash, used by
/// `connection_id_hash` for short connection IDs where SipHash
/// would be overkill.  Kept as a free function (rather than a
/// `Hasher` impl) for source-level parity with the C body — Phase
/// 4 may fold it into a `Hasher` if a `Hash` impl on
/// `ConnectionId` ends up wanting it.
///
/// C: `picohash_bytes`.
pub fn hash_bytes(_bytes: &[u8], _hash_seed: &[u8; 16]) -> u64 {
    todo!()
}

#[cfg(test)]
mod test {}
