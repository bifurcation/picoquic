//! Translation of `quic/splay.h`.
//!
//! An intrusive top-down splay tree.  [`SplayNode`] is a
//! three-pointer header that callers embed inside their own value
//! struct (see `sack_item_t.node`,
//! `stream_data_node_t.stream_data_node`, etc.).  The four
//! function-pointer fields on the tree (comparator, create,
//! delete_node, node_value) tell the tree how to compare keys,
//! allocate fresh nodes, free them, and project a node pointer
//! back to its key.
//!
//! Translation choices:
//!
//! * The four function pointers fold into one [`SplayOps`]
//!   trait per the "function pointers map to traits" rule.
//! * [`SplayNode`]'s `parent`/`left`/`right` and
//!   [`SplayTree::root`] stay as raw `*mut SplayNode`.
//!   The chain is intrusive — the tree does not own the nodes, the
//!   user struct that wraps each node does — and no safe Rust
//!   container expresses that ownership pattern.  Phase 3 will
//!   dereference these inside `unsafe { … }` blocks with
//!   `// SAFETY:` notes.
//! * `value`/`key` parameters stay as `*mut c_void` (type-erased
//!   key pointers, mirroring the C signature).  The Phase 3 caller
//!   knows the real type via its [`SplayOps`] impl.
//! * The boxed constructor returns `Option<Box<SplayTree>>`
//!   so allocation failure surfaces in the type rather than as a
//!   null pointer.
//! * [`SplayTree::init`] is kept (rather than collapsed into
//!   [`SplayTree::empty`]) because every in-tree caller embeds a
//!   `SplayTree` inside a larger struct that gets zeroed and then
//!   run through `init` — see `quic/sacks.c:435`,
//!   `quic/quicctx.c:1501`, etc.  The `Option<Box<dyn SplayOps>>`
//!   field expresses that "uninitialised intermediate" state.
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
/// by [`SplayTree::init`], so they live in one trait here.
pub trait SplayOps {
    /// Compare two keys: negative when `left < right`, zero on
    /// equal, positive when `left > right`.  Mirrors
    /// `int64_t (*picosplay_comparator)(void*, void*)`.
    fn compare(&self, left: *mut c_void, right: *mut c_void) -> i64;

    /// Allocate a fresh node carrying `value` and return a pointer
    /// to its embedded [`SplayNode`].  `None` mirrors a `NULL`
    /// return from the C callback (allocation failure).  Mirrors
    /// `picosplay_node_t* (*picosplay_create)(void*)`.
    fn create(&self, value: *mut c_void) -> Option<NonNull<SplayNode>>;

    /// Free the node previously produced by [`create`].  In C the
    /// first argument is `void* tree`, opaque to the splay code and
    /// passed straight through from [`SplayTree::delete_hint`];
    /// keep the same type-erased pointer here so Phase 3 can wire
    /// it through unchanged.
    ///
    /// [`create`]: SplayOps::create
    fn delete_node(&self, tree: *mut c_void, node: NonNull<SplayNode>);

    /// Project an embedded node pointer back to its key.  Mirrors
    /// `void* (*picosplay_node_value)(picosplay_node_t*)`.
    fn node_value(&self, node: NonNull<SplayNode>) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// Node: the intrusive header embedded in each user value struct.

/// One node header in the splay tree.  C: `picosplay_node_t`.
///
/// All three pointers stay raw because the nodes form an intrusive
/// in-tree chain whose endpoints are owned by the surrounding user
/// struct, not by this module.
#[derive(Debug)]
pub struct SplayNode {
    pub parent: *mut SplayNode,
    pub left: *mut SplayNode,
    pub right: *mut SplayNode,
}

impl SplayNode {
    /// Build a fresh, unlinked node header.  Useful for embedding
    /// in larger user structs at construction time.
    pub const fn new() -> Self {
        Self {
            parent: ptr::null_mut(),
            left: ptr::null_mut(),
            right: ptr::null_mut(),
        }
    }

    /// In-order predecessor of `node`, or `None` if `node` is the
    /// minimum.  Walks the intrusive parent/child links; takes the
    /// node by `NonNull` because the chain is not owned by this
    /// module.  C: `picosplay_previous`.
    pub fn previous(_node: NonNull<SplayNode>) -> Option<NonNull<SplayNode>> {
        todo!()
    }

    /// In-order successor of `node`, or `None` if `node` is the
    /// maximum.  C: `picosplay_next`.
    pub fn next(_node: NonNull<SplayNode>) -> Option<NonNull<SplayNode>> {
        todo!()
    }
}

impl Default for SplayNode {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tree.

/// Splay-tree handle.  C: `picosplay_tree_t`.
///
/// * `root` stays raw for the same reason as the node pointers.
/// * `ops` is `Option<Box<dyn SplayOps>>` so the type has a
///   sensible default — every in-tree caller embeds the tree inside
///   a larger struct that gets zeroed and then run through
///   [`SplayTree::init`], so the "uninitialised intermediate"
///   state has to be expressible.  After [`SplayTree::init`] the
///   field is always `Some`.
/// * `size` mirrors the C field directly (kept signed).
pub struct SplayTree {
    pub root: *mut SplayNode,
    pub ops: Option<Box<dyn SplayOps>>,
    pub size: i32,
}

impl SplayTree {
    /// Build an empty, uninitialised tree.  Equivalent to
    /// `memset(&tree, 0, sizeof tree)` in C; the caller follows up
    /// with [`SplayTree::init`] before use.
    pub const fn empty() -> Self {
        Self {
            root: ptr::null_mut(),
            ops: None,
            size: 0,
        }
    }

    /// Initialise an empty tree in place, attaching its
    /// [`SplayOps`] vtable.  C: `picosplay_init_tree`.
    pub fn init(&mut self, _ops: Box<dyn SplayOps>) {
        todo!()
    }

    /// Allocate a new tree on the heap and initialise it.  `None`
    /// mirrors a `malloc` failure (the C version returns `NULL`).
    /// C: `picosplay_new_tree`.
    pub fn new_boxed(_ops: Box<dyn SplayOps>) -> Option<Box<Self>> {
        todo!()
    }

    /// Insert a new node carrying `value`, then splay it to the
    /// root.  Returns the freshly inserted node, or `None` if the
    /// create callback returned `NULL` (allocation failure on the
    /// user side).  C: `picosplay_insert`.
    pub fn insert(&mut self, _value: *mut c_void) -> Option<NonNull<SplayNode>> {
        todo!()
    }

    /// Locate the node whose key compares equal to `value` and
    /// splay it to the root.  Returns `None` on miss.
    /// C: `picosplay_find`.
    pub fn find(&mut self, _value: *mut c_void) -> Option<NonNull<SplayNode>> {
        todo!()
    }

    /// Locate the largest node whose key is less than or equal to
    /// `value`.  Unlike [`SplayTree::find`] this does *not* splay
    /// the tree.  C: `picosplay_find_previous`.
    pub fn find_previous(&self, _value: *mut c_void) -> Option<NonNull<SplayNode>> {
        todo!()
    }

    /// Return the smallest (left-most) node, or `None` for an
    /// empty tree.  C: `picosplay_first`.
    pub fn first(&self) -> Option<NonNull<SplayNode>> {
        todo!()
    }

    /// Return the largest (right-most) node, or `None` for an
    /// empty tree.  C: `picosplay_last`.
    pub fn last(&self) -> Option<NonNull<SplayNode>> {
        todo!()
    }

    /// Locate `value` and remove the matching node.  No-op when
    /// `value` is absent.  C: `picosplay_delete`.
    pub fn delete(&mut self, _value: *mut c_void) {
        todo!()
    }

    /// Remove a previously-located `node`.  `None` mirrors the C
    /// `if (node == NULL) return` early exit; the caller commonly
    /// passes the result of [`SplayTree::find`] (which itself may
    /// be `None`) straight into this function.
    /// C: `picosplay_delete_hint`.
    pub fn delete_hint(&mut self, _node: Option<NonNull<SplayNode>>) {
        todo!()
    }

    /// Drop every node, leaving the tree empty.
    /// C: `picosplay_empty_tree`.
    pub fn clear(&mut self) {
        todo!()
    }
}

impl Default for SplayTree {
    fn default() -> Self {
        Self::empty()
    }
}

impl core::fmt::Debug for SplayTree {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SplayTree")
            .field("root", &self.root)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod test {}
