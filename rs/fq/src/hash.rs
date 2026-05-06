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
//! Phase 4: hand-rolled slotmap implementation.  One of:
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
//! For example, `quic.connection_by_id: HashTable<ConnectionId,
//! ConnectionToken>` points at connections that live in
//! `quic.connections: Arena<Connection>`.  A `Connection` carries
//! `connection_by_id_membership: Option<HashToken>`; on removal the
//! Connection hands that token back for an O(1) `connection_by_id.remove`.

extern crate alloc;
use alloc::vec::Vec;

use core::hash::{Hash, Hasher};

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
        hash: u64,
        key: K,
        value: V,
        next_in_bin: Option<u32>,
    },
}

// ---------------------------------------------------------------------------
// Seeded hasher — streams bytes through the picohash_bytes mixing function.

struct SeedHasher {
    hash: u64,
    rotate: u32,
    seed: [u8; 16],
    byte_idx: u32,
}

impl SeedHasher {
    fn new(seed: &[u8; 16]) -> Self {
        let hash = u64::from_le_bytes(seed[8..16].try_into().unwrap());
        SeedHasher {
            hash,
            rotate: 11,
            seed: *seed,
            byte_idx: 0,
        }
    }
}

impl Hasher for SeedHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.hash ^= b as u64;
            self.hash ^= self.seed[(self.byte_idx & 15) as usize] as u64;
            self.hash ^= self.hash << 8;
            self.hash = self.hash.wrapping_add(self.hash >> self.rotate);
            self.rotate = (self.hash & 31) as u32 + 11;
            self.byte_idx = self.byte_idx.wrapping_add(1);
        }
    }

    fn finish(&self) -> u64 {
        let mut h = self.hash;
        h ^= h >> self.rotate;
        h
    }
}

// ---------------------------------------------------------------------------

/// Hash table mapping `K` to `V`, addressable by [`HashToken`].
///
/// The value `V` is typically a token into some other arena (see
/// the Phase-4 call-site plan in the module docs) but any owned
/// value works.  `K: Hash + Eq` is the only contract on the key.
pub struct HashTable<K, V> {
    slots: Vec<Slot<K, V>>,
    free: Option<u32>,
    bins: Vec<Option<u32>>,
    seed: [u8; 16],
    len: usize,
}

impl<K: Hash + Eq, V> HashTable<K, V> {
    // -----------------------------------------------------------------------
    // Constructors

    /// Build an empty table with `nb_bin` collision bins, using a
    /// zero hash seed.
    ///
    /// C: `picohash_create`.
    pub fn new(nb_bin: usize) -> Result<Self, Error> {
        Self::with_seed(nb_bin, &[0u8; 16])
    }

    /// Like [`HashTable::new`] but with a caller-supplied 16-byte
    /// hash seed.
    ///
    /// C: `picohash_create_ex`.
    pub fn with_seed(nb_bin: usize, hash_seed: &[u8; 16]) -> Result<Self, Error> {
        let bins = alloc::vec![None; nb_bin];
        Ok(Self {
            slots: Vec::new(),
            free: None,
            bins,
            seed: *hash_seed,
            len: 0,
        })
    }

    // -----------------------------------------------------------------------
    // Internal helpers

    fn hash_key(&self, key: &K) -> u64 {
        let mut h = SeedHasher::new(&self.seed);
        key.hash(&mut h);
        h.finish()
    }

    fn bin_of(&self, hash: u64) -> usize {
        (hash % self.bins.len() as u64) as usize
    }

    fn alloc_slot(&mut self, hash: u64, key: K, value: V) -> Result<u32, Error> {
        if let Some(free_idx) = self.free {
            let next_free = match &self.slots[free_idx as usize].state {
                SlotState::Free { next_free } => *next_free,
                SlotState::Filled { .. } => unreachable!(),
            };
            let r#gen = self.slots[free_idx as usize].generation;
            self.slots[free_idx as usize] = Slot {
                generation: r#gen,
                state: SlotState::Filled {
                    hash,
                    key,
                    value,
                    next_in_bin: None,
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
                    hash,
                    key,
                    value,
                    next_in_bin: None,
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

    fn token_of(&self, idx: u32) -> HashToken {
        HashToken {
            idx,
            generation: self.slots[idx as usize].generation,
        }
    }

    fn is_valid(&self, t: HashToken) -> bool {
        let i = t.idx as usize;
        i < self.slots.len()
            && matches!(self.slots[i].state, SlotState::Filled { .. })
            && self.slots[i].generation == t.generation
    }

    fn slot_hash(&self, idx: u32) -> u64 {
        match &self.slots[idx as usize].state {
            SlotState::Filled { hash, .. } => *hash,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn slot_key(&self, idx: u32) -> &K {
        match &self.slots[idx as usize].state {
            SlotState::Filled { key, .. } => key,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn slot_value(&self, idx: u32) -> &V {
        match &self.slots[idx as usize].state {
            SlotState::Filled { value, .. } => value,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn slot_value_mut(&mut self, idx: u32) -> &mut V {
        match &mut self.slots[idx as usize].state {
            SlotState::Filled { value, .. } => value,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn slot_next(&self, idx: u32) -> Option<u32> {
        match &self.slots[idx as usize].state {
            SlotState::Filled { next_in_bin, .. } => *next_in_bin,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    fn set_next(&mut self, idx: u32, next: Option<u32>) {
        match &mut self.slots[idx as usize].state {
            SlotState::Filled { next_in_bin, .. } => *next_in_bin = next,
            SlotState::Free { .. } => unreachable!(),
        }
    }

    // -----------------------------------------------------------------------
    // Public API

    /// Insert `(key, value)` and return a [`HashToken`].
    /// If `key` was already present, the previous value is replaced.
    ///
    /// Returns [`Error::Memory`] on slot-vector allocation failure.
    pub fn insert(&mut self, key: K, value: V) -> Result<(HashToken, Option<V>), Error> {
        let hash = self.hash_key(&key);
        let bin = self.bin_of(hash);

        // Check for existing key.
        let mut cur = self.bins[bin];
        while let Some(idx) = cur {
            if self.slot_key(idx) == &key {
                let old = core::mem::replace(self.slot_value_mut(idx), value);
                return Ok((self.token_of(idx), Some(old)));
            }
            cur = self.slot_next(idx);
        }

        // New entry: prepend to bin chain.
        let idx = self.alloc_slot(hash, key, value)?;
        let old_head = self.bins[bin];
        self.set_next(idx, old_head);
        self.bins[bin] = Some(idx);
        self.len += 1;
        Ok((self.token_of(idx), None))
    }

    /// Look up `key`, returning the matching token or `None`.
    /// Equivalent to `picohash_retrieve` but typed.
    pub fn lookup(&self, key: &K) -> Option<HashToken> {
        let hash = self.hash_key(key);
        let bin = self.bin_of(hash);
        let mut cur = self.bins[bin];
        while let Some(idx) = cur {
            if self.slot_key(idx) == key {
                return Some(self.token_of(idx));
            }
            cur = self.slot_next(idx);
        }
        None
    }

    /// Borrow the value at `token`, or `None` if the token is stale.
    pub fn get(&self, token: HashToken) -> Option<&V> {
        if !self.is_valid(token) {
            return None;
        }
        Some(self.slot_value(token.idx))
    }

    /// Mutably borrow the value at `token`, or `None` if the token
    /// is stale.
    pub fn get_mut(&mut self, token: HashToken) -> Option<&mut V> {
        if !self.is_valid(token) {
            return None;
        }
        Some(self.slot_value_mut(token.idx))
    }

    /// Borrow the `(key, value)` pair at `token`.
    pub fn get_key_value(&self, token: HashToken) -> Option<(&K, &V)> {
        if !self.is_valid(token) {
            return None;
        }
        Some((self.slot_key(token.idx), self.slot_value(token.idx)))
    }

    /// Remove and return the entry at `token`.  Bumps the slot's
    /// generation.  Returns `None` if the token was already stale.
    pub fn remove(&mut self, token: HashToken) -> Option<(K, V)> {
        if !self.is_valid(token) {
            return None;
        }
        let idx = token.idx;
        let hash = self.slot_hash(idx);
        let bin = self.bin_of(hash);

        // Unlink from bin chain.
        let head = self.bins[bin];
        if head == Some(idx) {
            let next = self.slot_next(idx);
            self.bins[bin] = next;
        } else {
            let mut prev = head.unwrap();
            loop {
                let next = self.slot_next(prev);
                if next == Some(idx) {
                    let after = self.slot_next(idx);
                    self.set_next(prev, after);
                    break;
                }
                prev = next.unwrap();
            }
        }

        self.len -= 1;
        Some(self.release_slot(idx))
    }

    /// Remove the entry matching `key`.  Prefer [`HashTable::remove`]
    /// when a token is on hand.
    pub fn remove_by_key(&mut self, key: &K) -> Option<V> {
        let tok = self.lookup(key)?;
        self.remove(tok).map(|(_, v)| v)
    }

    /// `true` if `key` is currently in the table.
    pub fn contains_key(&self, key: &K) -> bool {
        self.lookup(key).is_some()
    }

    /// Number of live entries.
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` when `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drop every entry.  Tokens issued before the call are stale
    /// after it.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free = None;
        for b in &mut self.bins {
            *b = None;
        }
        self.len = 0;
    }
}

/// picoquic's bespoke byte-string hash, used by
/// `connection_id_hash` for short connection IDs where SipHash
/// would be overkill.  Kept as a free function (rather than a
/// `Hasher` impl) for source-level parity with the C body.
///
/// C: `picohash_bytes`.
pub fn hash_bytes(bytes: &[u8], hash_seed: &[u8; 16]) -> u64 {
    let mut hash = u64::from_le_bytes(hash_seed[8..16].try_into().unwrap());
    let mut rotate: u32 = 11;
    for (i, &b) in bytes.iter().enumerate() {
        hash ^= b as u64;
        hash ^= hash_seed[i & 15] as u64;
        hash ^= hash << 8;
        hash = hash.wrapping_add(hash >> rotate);
        rotate = (hash & 31) as u32 + 11;
    }
    hash ^= hash >> rotate;
    hash
}

#[cfg(test)]
mod test {}
