//! Translation of `quic/hash.h`.
//!
//! A small open hash table with chained collision lists.  Two
//! distinct allocation modes coexist in the C source:
//!
//! * **Owning mode** — `hash_key_to_item` is `NULL`; the table
//!   `malloc`s a fresh `hash_item` per insert and `free`s it
//!   on delete (`hash_create` / `hash_create_ex` with a
//!   `NULL` last-but-one argument).
//! * **Intrusive mode** — the caller embeds a `hash_item` in
//!   its own struct and supplies a `key_to_item` callback that
//!   returns a pointer to that field.  The table never frees those
//!   items.
//!
//! Both modes are exercised by the rest of `quic-core`
//! (`quicctx.c` uses intrusive mode for all five tables it owns),
//! so the Rust translation has to keep both.  That forces a few
//! data-shape choices:
//!
//! * `hash_item.next_in_bin` stays `*mut hash_item` — no
//!   safe Rust container expresses an intrusive linked list whose
//!   nodes might or might not be owned by the list.  Reads and
//!   writes through that pointer go in `unsafe` blocks with
//!   `// SAFETY:` comments in Phase 3.
//! * `hash_item.key` stays `*const c_void`.  The C side stores
//!   arbitrary key types behind a single hash-table type; the
//!   caller is responsible for knowing the real type.  Phase 3 may
//!   revisit by pushing a key type parameter up onto
//!   `hash_table<K>`, but that requires auditing every caller
//!   and is out of scope for the Phase 1 stub.
//! * The function-pointer trio (`hash_hash`,
//!   `hash_compare`, `hash_key_to_item`) is folded into a
//!   single `HashOps` trait per the "function pointers map to
//!   traits" rule.  `key_to_item` has a default-`None` impl so
//!   owning-mode tables don't have to implement it.
//!
//! Phase 1 contract: signatures only; every body is `todo!()`.

// `Box` comes from the prelude.  Once the crate flips to
// `#![no_std]` (per the translation plan) this will become an
// explicit `use alloc::boxed::Box;` at the crate root.
use core::ffi::c_void;
use core::ptr::NonNull;

// ---------------------------------------------------------------------------
// Operations vtable.

/// The trio of function pointers that travel together in C
/// (`hash_hash`, `hash_compare`, `hash_key_to_item`)
/// describes how a particular table understands its keys.  The
/// translation maps them to one trait per table.
///
/// `key` parameters are `*const c_void` because the C side stores
/// arbitrary key types behind a single hash-table type — only the
/// caller's `HashOps` impl knows the real type.
pub trait HashOps {
    /// Hash one key with the table's 16-byte seed.  Mirrors
    /// `uint64_t (*hash_hash)(const void*, const uint8_t*)`.
    fn hash(&self, key: *const c_void, hash_seed: &[u8; 16]) -> u64;

    /// Compare two keys for equality.  C: `int (*hash_compare)
    /// (const void*, const void*)` returning `0` for equal, anything
    /// else for unequal — only `== 0` is ever checked at the call
    /// sites in `hash.c`, so this trait method returns a plain
    /// `bool` (`true` ↔ equal).
    fn compare(&self, key: *const c_void, item_key: *const c_void) -> bool;

    /// Intrusive-mode hook: return a pointer to a `hash_item`
    /// embedded inside the structure pointed to by `key`.  Default
    /// `None` means "owning mode" — `hash_insert` will allocate
    /// a fresh item itself.  Mirrors the optional `hash_item*
    /// (*hash_key_to_item)(const void*)` field, where `NULL`
    /// signaled owning mode.
    fn key_to_item(&self, _key: *const c_void) -> Option<NonNull<hash_item>> {
        None
    }
}

// ---------------------------------------------------------------------------
// Hash item: one slot in a collision chain.

/// One entry in a hash bin.  C: `hash_item`.
///
/// `next_in_bin` is a raw pointer because the chain's nodes are
/// not necessarily owned by the table (intrusive mode).  `key` is
/// a raw pointer because the table is type-erased.  Both will be
/// dereferenced inside `unsafe` blocks in Phase 3.
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub struct hash_item {
    pub hash: u64,
    pub next_in_bin: *mut hash_item,
    pub key: *const c_void,
}

// ---------------------------------------------------------------------------
// Hash table.

/// Open hash table with chained collisions and a pluggable
/// operations vtable.  C: `hash_table`.
///
/// Field-shape choices:
///
/// * `hash_bin: Box<[*mut hash_item]>` — the bin count is
///   fixed at construction (no resize anywhere in `quic-core`),
///   so a boxed slice expresses the contract better than `Vec`.
///   The bin entries themselves are raw pointers because the chain
///   nodes are not owned by the table in intrusive mode.
/// * `nb_bin` mirrors the C field directly.  Invariant:
///   `nb_bin == hash_bin.len()` must hold at all times; Phase 3
///   code may use either to index bins.
/// * `count` mirrors the C field directly.
/// * `hash_seed: [u8; 16]` — the C field was `const uint8_t*` and
///   either aliased caller memory (the QUIC context's seed buffer,
///   `quicctx.c:706`) or pointed at a `static` zero buffer.  The
///   safe translation owns a copy; the seed is a 16-byte secret
///   set once at QUIC-context creation, so duplication is cheap.
/// * `ops: Box<dyn HashOps>` — folds the C function-pointer
///   trio into a single trait object per the translation rules.
///
/// Threading: the C header carries a `/* TODO: lock ! */` comment.
/// Multi-threading is out of scope for v1; revisit in v2 when
/// `Send`/`Sync` are added to the crate.
#[allow(non_camel_case_types)]
pub struct hash_table {
    pub hash_bin: Box<[*mut hash_item]>,
    pub nb_bin: usize,
    pub count: usize,
    pub hash_seed: [u8; 16],
    pub ops: Box<dyn HashOps>,
}

impl core::fmt::Debug for hash_table {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("hash_table")
            .field("nb_bin", &self.nb_bin)
            .field("count", &self.count)
            .field("hash_seed", &self.hash_seed)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Public API.

/// Allocate an empty hash table with `nb_bin` bins.  The C version
/// passes `NULL` for both `hash_key_to_item` and `hash_seed`,
/// so this wrapper supplies the default zero seed and lets the
/// supplied `ops` decide whether `key_to_item` returns `None`
/// (owning mode) or `Some` (intrusive mode).  Returns `None` if
/// the underlying allocation fails — C returns `NULL` in the same
/// case (see `hash.c:44`).
pub fn hash_create(_nb_bin: usize, _ops: Box<dyn HashOps>) -> Option<Box<hash_table>> {
    todo!()
}

/// Like [`hash_create`] but lets the caller supply the hash
/// seed.  `None` means "use the default zero seed", matching the C
/// behavior where a `NULL` seed substituted the static `null_seed`
/// buffer (`hash.c:54`).
///
/// Returns `None` on allocation failure.
pub fn hash_create_ex(
    _nb_bin: usize,
    _ops: Box<dyn HashOps>,
    _hash_seed: Option<&[u8; 16]>,
) -> Option<Box<hash_table>> {
    todo!()
}

/// Look up a key in the table.  Returns `Some(item)` if a chain
/// node compares equal to `key`, else `None`.  C:
/// `hash_retrieve` returning `NULL` on miss.
///
/// Pointer-shape choice: every caller in `quicctx.c` immediately
/// dereferences the return value or compares it to `NULL`; the
/// returned item is borrowed from the table's bin chain, so
/// `Option<&'_ mut hash_item>` borrow-checks the lifetime.
/// `key` stays `*const c_void` (type-erased).
pub fn hash_retrieve(_hash_table: &mut hash_table, _key: *const c_void) -> Option<&mut hash_item> {
    todo!()
}

/// Insert a new key.  In owning mode the table allocates a fresh
/// `hash_item`; in intrusive mode it calls `ops.key_to_item`
/// to obtain a pointer to one already embedded inside the caller's
/// structure.  Returns `Err(())` if allocation fails or the
/// `key_to_item` callback yields `None` (matching the C `-1`
/// return).
///
/// `Result<(), ()>` is a placeholder — the crate-level `Error`
/// enum doesn't exist yet.  Replace once it lands.
// TODO(error-enum): swap `()` for the crate's `Error` once it lands.
#[allow(clippy::result_unit_err)]
pub fn hash_insert(_hash_table: &mut hash_table, _key: *const c_void) -> Result<(), ()> {
    todo!()
}

/// Remove `item` from its bin chain in `hash_table`.  When the
/// table is in owning mode (`ops.key_to_item` returns `None`) the
/// item itself is freed; in intrusive mode the caller retains
/// ownership.  When `delete_key_too` is `true` the key buffer is
/// also freed.
///
/// Pointer-shape choice: `item` is `*mut hash_item` rather
/// than `&mut hash_item` because the call-sites in
/// `quicctx.c` (e.g. `quicctx.c:1359` taking
/// `&cnx->registered_icid_item`) reach the item through the same
/// `cnx -> quic` chain that aliases `hash_table`.  The borrow
/// checker can't see those splits, so the raw pointer matches the
/// C contract; Phase 3 will dereference inside `unsafe { … }`.
///
/// `delete_key_too` was a C `int` flag; promoted to `bool`.
pub fn hash_delete_item(
    _hash_table: &mut hash_table,
    _item: *mut hash_item,
    _delete_key_too: bool,
) {
    todo!()
}

/// Find `key` in the table and remove its item.  When the lookup
/// misses and `delete_key_too` is `true` the key buffer is still
/// freed (matching `hash.c:150`).
///
/// `key` is `*mut c_void` (not `*const`) because the C signature
/// is `void*` — when `delete_key_too` is set the function takes
/// ownership and frees it.
pub fn hash_delete_key(_hash_table: &mut hash_table, _key: *mut c_void, _delete_key_too: bool) {
    todo!()
}

/// Free the entire table.  Walks every bin chain and (in owning
/// mode) frees each item; if `delete_key_too` is set, each key
/// buffer is also freed.  Then the bin array and the table itself
/// are freed.
///
/// Pointer-shape choice: takes `Box<hash_table>` so the table
/// is consumed and dropped at end of scope, mirroring the C
/// `free(hash_table)` at the end of the function.  Clippy's
/// `boxed_local` lint flags the parameter as unnecessary because
/// the `todo!()` body never moves it, but the Box is intentional
/// — Phase 3 will use it to free the table.
#[allow(clippy::boxed_local)]
pub fn hash_delete(_hash_table: Box<hash_table>, _delete_key_too: bool) {
    todo!()
}

/// quic's bespoke byte-string hash.  Used by
/// `connection_id_hash` for short connection IDs where
/// SipHash would be overkill.  C: `hash_bytes`.
///
/// Pointer-shape choice: `bytes` becomes `&[u8]` (length implicit
/// in the slice); `hash_seed` is `&[u8; 16]` because the C body
/// indexes `hash_seed[0..=15]` unconditionally.
pub fn hash_bytes(_bytes: &[u8], _hash_seed: &[u8; 16]) -> u64 {
    todo!()
}

/// SipHash-2-4 wrapper that returns the 8-byte digest as a `u64`.
/// Calls into the in-tree `siphash.c` (out of scope here — its
/// translation lives elsewhere).  C: `hash_siphash`.
pub fn hash_siphash(_bytes: &[u8], _hash_seed: &[u8; 16]) -> u64 {
    todo!()
}

#[cfg(test)]
mod test {}
