//! Translation of `quic/hash.h`.
//!
//! A small open hash table with chained collision lists.  Two
//! distinct allocation modes coexist in the C source:
//!
//! * **Owning mode** — the caller leaves [`HashOps::key_to_item`]
//!   at its default; the table allocates a fresh [`HashItem`] per
//!   insert and frees it on delete.
//! * **Intrusive mode** — the caller embeds a [`HashItem`] in its
//!   own struct and overrides [`HashOps::key_to_item`] to return a
//!   pointer to that field.  The table never frees those items.
//!
//! Both modes are exercised by the rest of `quic-core` (the QUIC
//! context uses intrusive mode for all five tables it owns), so the
//! Rust translation has to keep both.  That forces a few data-shape
//! choices:
//!
//! * `HashItem::next_in_bin` stays `*mut HashItem` — no safe Rust
//!   container expresses an intrusive linked list whose nodes
//!   might or might not be owned by the list.  Reads and writes
//!   through that pointer go in `unsafe` blocks with `// SAFETY:`
//!   comments in Phase 3.
//! * `HashItem::key` stays `*const c_void`.  The C side stores
//!   arbitrary key types behind a single hash-table type; the
//!   caller is responsible for knowing the real type.  A future
//!   refactor could push a key type parameter onto `HashTable<K>`,
//!   but that requires auditing every caller and is out of scope
//!   for the Phase 1 stub.
//! * The function-pointer trio (`picohash_hash`, `picohash_compare`,
//!   `picohash_key_to_item`) is folded into a single [`HashOps`]
//!   trait per the "function pointers map to traits" rule.
//!   [`HashOps::key_to_item`] has a default `None` impl so
//!   owning-mode tables don't have to implement it.
//!
//! Phase 1 contract: signatures only; every body is `todo!()`.

use crate::Error;
use core::ffi::c_void;
use core::ptr::NonNull;

// ---------------------------------------------------------------------------
// Operations vtable.

/// Per-table operations for hashing, key comparison, and (in
/// intrusive mode) locating the embedded [`HashItem`] for a given
/// key.  The C source carries this as a trio of function pointers
/// stored on `picohash_table`; the translation maps them to one
/// trait so an implementor supplies them as a unit.
///
/// `key` parameters are `*const c_void` because the table is
/// type-erased over key types — only the implementor knows the
/// real key shape.
pub trait HashOps {
    /// Hash one key with the table's 16-byte seed.
    fn hash(&self, key: *const c_void, hash_seed: &[u8; 16]) -> u64;

    /// Compare two keys for equality.  Returns `true` if the keys
    /// are equal.  (The C callback returns `0` for equal and only
    /// `== 0` is checked at the call sites, so the Rust signature
    /// collapses to `bool` in the natural direction.)
    fn compare(&self, key: *const c_void, item_key: *const c_void) -> bool;

    /// Intrusive-mode hook: return a pointer to the [`HashItem`]
    /// embedded inside the structure pointed to by `key`.  The
    /// default `None` selects owning mode — [`HashTable::insert`]
    /// allocates a fresh item itself.
    fn key_to_item(&self, _key: *const c_void) -> Option<NonNull<HashItem>> {
        None
    }
}

// ---------------------------------------------------------------------------
// Hash item: one slot in a collision chain.

/// One entry in a hash bin.  C: `picohash_item`.
///
/// `next_in_bin` is a raw pointer because the chain's nodes are
/// not necessarily owned by the table (intrusive mode).  `key` is
/// a raw pointer because the table is type-erased.  Both will be
/// dereferenced inside `unsafe` blocks in Phase 3.
#[derive(Debug)]
pub struct HashItem {
    pub hash: u64,
    pub next_in_bin: *mut HashItem,
    pub key: *const c_void,
}

// ---------------------------------------------------------------------------
// Hash table.

/// Open hash table with chained collisions and a pluggable
/// operations vtable.  C: `picohash_table`.
///
/// Field-shape choices:
///
/// * `hash_bin: Box<[*mut HashItem]>` — the bin count is fixed at
///   construction (no resize anywhere in `quic-core`), so a boxed
///   slice expresses the contract better than `Vec`.  The bin
///   entries themselves are raw pointers because the chain nodes
///   are not owned by the table in intrusive mode.
/// * `nb_bin` mirrors the C field directly.  Invariant:
///   `nb_bin == hash_bin.len()` must hold at all times; Phase 3
///   code may use either to index bins.
/// * `count` mirrors the C field directly.
/// * `hash_seed: [u8; 16]` — the C field was `const uint8_t*` and
///   either aliased caller memory (the QUIC context's seed buffer)
///   or pointed at a `static` zero buffer.  The safe translation
///   owns a copy; the seed is a 16-byte secret set once at
///   QUIC-context creation, so duplication is cheap.
/// * `ops: Box<dyn HashOps>` — folds the C function-pointer trio
///   into a single trait object per the translation rules.
///
/// Threading: the C header carries a `/* TODO: lock ! */` comment.
/// Multi-threading is out of scope for v1; revisit in v2 when
/// `Send`/`Sync` are added to the crate.
pub struct HashTable {
    pub hash_bin: Box<[*mut HashItem]>,
    pub nb_bin: usize,
    pub count: usize,
    pub hash_seed: [u8; 16],
    pub ops: Box<dyn HashOps>,
}

impl core::fmt::Debug for HashTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HashTable")
            .field("nb_bin", &self.nb_bin)
            .field("count", &self.count)
            .field("hash_seed", &self.hash_seed)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Public API.

impl HashTable {
    /// Allocate an empty hash table with `nb_bin` bins, using the
    /// default zero seed.  The supplied `ops` selects owning vs.
    /// intrusive mode through its [`HashOps::key_to_item`] impl.
    /// Returns [`Error::Memory`] if the underlying allocation
    /// fails (the C `picohash_create` returns `NULL` in the same
    /// case).
    ///
    /// C: `picohash_create`.
    pub fn new(_nb_bin: usize, _ops: Box<dyn HashOps>) -> Result<Box<Self>, Error> {
        todo!()
    }

    /// Like [`HashTable::new`] but lets the caller supply the hash
    /// seed.  `None` means "use the default zero seed", matching
    /// the C behavior where a `NULL` seed substituted a static
    /// zero buffer.
    ///
    /// Returns [`Error::Memory`] on allocation failure.
    ///
    /// C: `picohash_create_ex`.
    pub fn with_seed(
        _nb_bin: usize,
        _ops: Box<dyn HashOps>,
        _hash_seed: Option<&[u8; 16]>,
    ) -> Result<Box<Self>, Error> {
        todo!()
    }

    /// Look up a key in the table.  Returns `Some(item)` if a
    /// chain node compares equal to `key`, else `None`.  The
    /// returned item is borrowed from the table's bin chain.
    /// `key` stays `*const c_void` because the table is type-erased.
    ///
    /// C: `picohash_retrieve` (returning `NULL` on miss).
    pub fn retrieve(&mut self, _key: *const c_void) -> Option<&mut HashItem> {
        todo!()
    }

    /// Insert a new key.  In owning mode the table allocates a
    /// fresh [`HashItem`]; in intrusive mode it calls
    /// [`HashOps::key_to_item`] to obtain a pointer to one already
    /// embedded inside the caller's structure.  Returns
    /// [`Error::Memory`] if allocation fails or the `key_to_item`
    /// callback yields `None` (matching the C `-1` return).
    ///
    /// C: `picohash_insert`.
    pub fn insert(&mut self, _key: *const c_void) -> Result<(), Error> {
        todo!()
    }

    /// Remove `item` from its bin chain.  In owning mode
    /// ([`HashOps::key_to_item`] returns `None`) the item itself
    /// is freed; in intrusive mode the caller retains ownership.
    /// When `delete_key_too` is `true` the key buffer is also
    /// freed.
    ///
    /// Pointer-shape choice: `item` is `*mut HashItem` rather than
    /// `&mut HashItem` because the call-sites reach the item
    /// through the same `cnx -> quic` chain that aliases the
    /// table.  The borrow checker can't see those splits, so the
    /// raw pointer matches the C contract; Phase 3 will
    /// dereference inside `unsafe { … }`.
    ///
    /// C: `picohash_delete_item` (`delete_key_too` promoted from
    /// `int` to `bool`).
    pub fn delete_item(&mut self, _item: *mut HashItem, _delete_key_too: bool) {
        todo!()
    }

    /// Find `key` in the table and remove its item.  When the
    /// lookup misses and `delete_key_too` is `true` the key
    /// buffer is still freed.
    ///
    /// `key` is `*mut c_void` (not `*const`) because the C
    /// signature is `void*` — when `delete_key_too` is set the
    /// function takes ownership of the key buffer and frees it.
    ///
    /// C: `picohash_delete_key`.
    pub fn delete_key(&mut self, _key: *mut c_void, _delete_key_too: bool) {
        todo!()
    }

    /// Walks every bin chain and (in owning mode) clears each
    /// item; if `delete_key_too` is set, each key buffer is also
    /// dropped.  Replaces the C `picohash_delete(table)` plus
    /// `free(table)` pattern: the caller transfers the table by
    /// value, the method consumes it, and the destructor at end
    /// of scope handles deallocation.
    ///
    /// C: `picohash_delete`.
    pub fn delete(self, _delete_key_too: bool) {
        todo!()
    }
}

/// picoquic's bespoke byte-string hash.  Used by
/// `connection_id_hash` for short connection IDs where SipHash
/// would be overkill.
///
/// Pointer-shape choice: `bytes` becomes `&[u8]` (length implicit
/// in the slice); `hash_seed` is `&[u8; 16]` because the C body
/// indexes `hash_seed[0..=15]` unconditionally.
///
/// C: `picohash_bytes`.
pub fn hash_bytes(_bytes: &[u8], _hash_seed: &[u8; 16]) -> u64 {
    todo!()
}

/// SipHash-2-4 wrapper that returns the 8-byte digest as a `u64`.
/// Calls into the in-tree `siphash.c` (its translation lives in
/// [`crate::siphash`]).
///
/// C: `picohash_siphash`.
pub fn hash_siphash(_bytes: &[u8], _hash_seed: &[u8; 16]) -> u64 {
    todo!()
}

#[cfg(test)]
mod test {}
