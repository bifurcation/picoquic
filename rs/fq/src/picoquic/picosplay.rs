//! Translation of `picoquic/picosplay.h`.
//!
//! An intrusive top-down splay tree.  `picosplay_node_t` is a
//! three-pointer header that callers embed inside their own value
//! struct (see `picoquic_sack_item_t.node`,
//! `picoquic_stream_data_node_t.stream_data_node`, etc.).  The four
//! function-pointer fields on the tree (comparator, create,
//! delete_node, node_value) tell the tree how to compare keys,
//! allocate fresh nodes, free them, and project a node pointer
//! back to its key.
//!
//! Translation choices:
//!
//! * The four function pointers fold into one [`PicoSplayOps`]
//!   trait per the "function pointers map to traits" rule.
//! * `picosplay_node_t`'s `parent`/`left`/`right` and
//!   `picosplay_tree_t.root` stay as raw `*mut picosplay_node_t`.
//!   The chain is intrusive — the tree does not own the nodes, the
//!   user struct that wraps each node does — and no safe Rust
//!   container expresses that ownership pattern.  Phase 3 will
//!   dereference these inside `unsafe { … }` blocks with
//!   `// SAFETY:` notes.
//! * `value`/`key` parameters stay as `*mut c_void` (type-erased
//!   key pointers, mirroring the C signature).  The Phase 3 caller
//!   knows the real type via its `PicoSplayOps` impl.
//! * The new-tree allocator returns `Option<Box<picosplay_tree_t>>`
//!   so allocation failure surfaces in the type rather than as a
//!   null pointer.
//! * `picosplay_init_tree` is kept (rather than collapsed into
//!   [`picosplay_tree_t::empty`]) because every in-tree caller
//!   embeds a `picosplay_tree_t` inside a larger struct that gets
//!   zeroed and then run through `picosplay_init_tree` — see
//!   `picoquic/sacks.c:435`, `picoquic/quicctx.c:1501`, etc.  The
//!   `Option<Box<dyn PicoSplayOps>>` field expresses that
//!   "uninitialised intermediate" state.
//!
//! Phase 1 contract: signatures only; every body is `todo!()`.
//!
//! Adapted from <https://github.com/lrem/splay> (MIT, © 2014
//! Remigiusz Modrzejewski).

use core::ffi::c_void;
use core::ptr::{self, NonNull};

// ---------------------------------------------------------------------------
// Operations vtable.

/// Bundle of the four function pointers attached to a
/// `picosplay_tree_t` in C.  Per the translation rules,
/// function-pointer typedefs that travel together collapse to one
/// trait; the four splay callbacks are always installed as a unit
/// by `picosplay_init_tree`, so they live in one trait here.
pub trait PicoSplayOps {
    /// Compare two keys: negative when `left < right`, zero on
    /// equal, positive when `left > right`.  Mirrors
    /// `int64_t (*picosplay_comparator)(void*, void*)`.
    fn compare(&self, left: *mut c_void, right: *mut c_void) -> i64;

    /// Allocate a fresh node carrying `value` and return a pointer
    /// to its embedded `picosplay_node_t`.  `None` mirrors a `NULL`
    /// return from the C callback (allocation failure).  Mirrors
    /// `picosplay_node_t* (*picosplay_create)(void*)`.
    fn create(&self, value: *mut c_void) -> Option<NonNull<picosplay_node_t>>;

    /// Free the node previously produced by [`create`].  In C the
    /// first argument is `void* tree`, opaque to the splay code and
    /// passed straight through from [`picosplay_delete_hint`];
    /// keep the same type-erased pointer here so Phase 3 can wire
    /// it through unchanged.
    ///
    /// [`create`]: PicoSplayOps::create
    fn delete_node(&self, tree: *mut c_void, node: NonNull<picosplay_node_t>);

    /// Project an embedded node pointer back to its key.  Mirrors
    /// `void* (*picosplay_node_value)(picosplay_node_t*)`.
    fn node_value(&self, node: NonNull<picosplay_node_t>) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// Node: the intrusive header embedded in each user value struct.

/// One node header in the splay tree.  C: `picosplay_node_t`.
///
/// All three pointers stay raw because the nodes form an intrusive
/// in-tree chain whose endpoints are owned by the surrounding user
/// struct, not by this module.
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub struct picosplay_node_t {
    pub parent: *mut picosplay_node_t,
    pub left: *mut picosplay_node_t,
    pub right: *mut picosplay_node_t,
}

impl picosplay_node_t {
    /// Build a fresh, unlinked node header.  Useful for embedding
    /// in larger user structs at construction time.
    pub const fn new() -> Self {
        Self {
            parent: ptr::null_mut(),
            left: ptr::null_mut(),
            right: ptr::null_mut(),
        }
    }
}

impl Default for picosplay_node_t {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tree.

/// Splay-tree handle.  C: `picosplay_tree_t`.
///
/// * `root` stays raw for the same reason as the node pointers.
/// * `ops` is `Option<Box<dyn PicoSplayOps>>` so the type has a
///   sensible default — every in-tree caller embeds the tree inside
///   a larger struct that gets zeroed and then run through
///   [`picosplay_init_tree`], so the "uninitialised intermediate"
///   state has to be expressible.  After `picosplay_init_tree` the
///   field is always `Some`.
/// * `size` mirrors the C field directly (kept signed).
#[allow(non_camel_case_types)]
pub struct picosplay_tree_t {
    pub root: *mut picosplay_node_t,
    pub ops: Option<Box<dyn PicoSplayOps>>,
    pub size: i32,
}

impl picosplay_tree_t {
    /// Build an empty, uninitialised tree.  Equivalent to
    /// `memset(&tree, 0, sizeof tree)` in C; the caller follows up
    /// with [`picosplay_init_tree`] before use.
    pub const fn empty() -> Self {
        Self {
            root: ptr::null_mut(),
            ops: None,
            size: 0,
        }
    }
}

impl Default for picosplay_tree_t {
    fn default() -> Self {
        Self::empty()
    }
}

impl core::fmt::Debug for picosplay_tree_t {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("picosplay_tree_t")
            .field("root", &self.root)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Public API.

/// Initialise an empty tree in place, attaching its
/// [`PicoSplayOps`] vtable.  C: `picosplay_init_tree`.
pub fn picosplay_init_tree(_tree: &mut picosplay_tree_t, _ops: Box<dyn PicoSplayOps>) {
    todo!()
}

/// Allocate a new tree on the heap and initialise it.  `None`
/// mirrors a `malloc` failure (the C version returns `NULL`).
/// C: `picosplay_new_tree`.
pub fn picosplay_new_tree(_ops: Box<dyn PicoSplayOps>) -> Option<Box<picosplay_tree_t>> {
    todo!()
}

/// Insert a new node carrying `value`, then splay it to the root.
/// Returns the freshly inserted node, or `None` if the create
/// callback returned `NULL` (allocation failure on the user side).
/// C: `picosplay_insert`.
pub fn picosplay_insert(
    _tree: &mut picosplay_tree_t,
    _value: *mut c_void,
) -> Option<NonNull<picosplay_node_t>> {
    todo!()
}

/// Locate the node whose key compares equal to `value` and splay
/// it to the root.  Returns `None` on miss.  C: `picosplay_find`.
pub fn picosplay_find(
    _tree: &mut picosplay_tree_t,
    _value: *mut c_void,
) -> Option<NonNull<picosplay_node_t>> {
    todo!()
}

/// Locate the largest node whose key is less than or equal to
/// `value`.  Unlike [`picosplay_find`] this does *not* splay the
/// tree.  C: `picosplay_find_previous`.
pub fn picosplay_find_previous(
    _tree: &picosplay_tree_t,
    _value: *mut c_void,
) -> Option<NonNull<picosplay_node_t>> {
    todo!()
}

/// Return the smallest (left-most) node, or `None` for an empty
/// tree.  C: `picosplay_first`.
pub fn picosplay_first(_tree: &picosplay_tree_t) -> Option<NonNull<picosplay_node_t>> {
    todo!()
}

/// In-order predecessor of `node`, or `None` if `node` is the
/// minimum.  C: `picosplay_previous`.
pub fn picosplay_previous(_node: NonNull<picosplay_node_t>) -> Option<NonNull<picosplay_node_t>> {
    todo!()
}

/// In-order successor of `node`, or `None` if `node` is the
/// maximum.  C: `picosplay_next`.
pub fn picosplay_next(_node: NonNull<picosplay_node_t>) -> Option<NonNull<picosplay_node_t>> {
    todo!()
}

/// Return the largest (right-most) node, or `None` for an empty
/// tree.  C: `picosplay_last`.
pub fn picosplay_last(_tree: &picosplay_tree_t) -> Option<NonNull<picosplay_node_t>> {
    todo!()
}

/// Locate `value` in `tree` and remove the matching node.  No-op
/// when `value` is absent.  C: `picosplay_delete`.
pub fn picosplay_delete(_tree: &mut picosplay_tree_t, _value: *mut c_void) {
    todo!()
}

/// Remove a previously-located `node` from `tree`.  `None`
/// mirrors the C `if (node == NULL) return` early exit; the
/// caller commonly passes the result of [`picosplay_find`] (which
/// itself may be `None`) straight into this function.
/// C: `picosplay_delete_hint`.
pub fn picosplay_delete_hint(
    _tree: &mut picosplay_tree_t,
    _node: Option<NonNull<picosplay_node_t>>,
) {
    todo!()
}

/// Drop every node from `tree`, leaving it empty.  C:
/// `picosplay_empty_tree`.
pub fn picosplay_empty_tree(_tree: &mut picosplay_tree_t) {
    todo!()
}

#[cfg(test)]
mod test {}
