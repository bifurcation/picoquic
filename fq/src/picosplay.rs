//! Splay tree implementation for efficient lookups.
//!
//! Translated from picoquic/picosplay.c.
//!
//! A splay tree is a self-balancing binary search tree with the property that
//! recently accessed elements are quick to access again. This implementation
//! provides the same interface as the C version, using callback functions
//! for node operations.
//!
//! Based on the original implementation from https://github.com/lrem/splay
//! by Remigiusz Modrzejewski (MIT license).

use std::ptr;

// =============================================================================
// Node Structure
// =============================================================================

/// A splay tree node.
///
/// This node only contains the tree structure pointers. The actual data
/// is stored in a containing struct that embeds this node.
#[repr(C)]
#[derive(Debug)]
pub struct PicosplayNode {
    /// Parent node pointer (null for root).
    pub parent: *mut PicosplayNode,
    /// Left child pointer.
    pub left: *mut PicosplayNode,
    /// Right child pointer.
    pub right: *mut PicosplayNode,
}

impl Default for PicosplayNode {
    fn default() -> Self {
        Self::new()
    }
}

impl PicosplayNode {
    /// Create a new unlinked node.
    pub const fn new() -> Self {
        Self {
            parent: ptr::null_mut(),
            left: ptr::null_mut(),
            right: ptr::null_mut(),
        }
    }
}

// =============================================================================
// Callback Types
// =============================================================================

/// Comparator function type.
///
/// Returns:
/// - negative if left < right
/// - zero if left == right
/// - positive if left > right
pub type Comparator = unsafe extern "C" fn(left: *mut (), right: *mut ()) -> i64;

/// Node creation function type.
///
/// Creates a new node for the given value.
pub type CreateNode = unsafe extern "C" fn(value: *mut ()) -> *mut PicosplayNode;

/// Node deletion function type.
///
/// Deletes the node and frees associated memory.
pub type DeleteNode = unsafe extern "C" fn(tree: *mut (), node: *mut PicosplayNode);

/// Node value extraction function type.
///
/// Gets the value pointer from a node.
pub type NodeValue = unsafe extern "C" fn(node: *mut PicosplayNode) -> *mut ();

// =============================================================================
// Tree Structure
// =============================================================================

/// A splay tree.
#[repr(C)]
#[derive(Debug)]
pub struct PicosplayTree {
    /// Root node of the tree.
    pub root: *mut PicosplayNode,
    /// Comparator function.
    pub comp: Option<Comparator>,
    /// Node creation function.
    pub create: Option<CreateNode>,
    /// Node deletion function.
    pub delete_node: Option<DeleteNode>,
    /// Node value extraction function.
    pub node_value: Option<NodeValue>,
    /// Number of nodes in the tree.
    pub size: i32,
}

impl Default for PicosplayTree {
    fn default() -> Self {
        Self::new()
    }
}

impl PicosplayTree {
    /// Create an empty tree without callbacks.
    pub const fn new() -> Self {
        Self {
            root: ptr::null_mut(),
            comp: None,
            create: None,
            delete_node: None,
            node_value: None,
            size: 0,
        }
    }

    /// Initialize a tree with callbacks.
    pub fn init(
        &mut self,
        comp: Comparator,
        create: CreateNode,
        delete_node: DeleteNode,
        node_value: NodeValue,
    ) {
        self.comp = Some(comp);
        self.create = Some(create);
        self.delete_node = Some(delete_node);
        self.node_value = Some(node_value);
        self.root = ptr::null_mut();
        self.size = 0;
    }

    /// Check if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.root.is_null()
    }

    /// Get the number of nodes in the tree.
    pub fn len(&self) -> usize {
        self.size as usize
    }
}

// =============================================================================
// Internal Helper Functions
// =============================================================================

/// Get the leftmost node starting from the given node.
fn leftmost(mut node: *mut PicosplayNode) -> *mut PicosplayNode {
    let mut parent = ptr::null_mut();
    while !node.is_null() {
        parent = node;
        unsafe {
            node = (*node).left;
        }
    }
    parent
}

/// Get the rightmost node starting from the given node.
fn rightmost(mut node: *mut PicosplayNode) -> *mut PicosplayNode {
    let mut parent = ptr::null_mut();
    while !node.is_null() {
        parent = node;
        unsafe {
            node = (*node).right;
        }
    }
    parent
}

/// Mark grandparent relationship during rotation.
///
/// # Safety
/// Caller must ensure child and its parent are valid pointers.
unsafe fn mark_gp(child: *mut PicosplayNode) {
    let parent = (*child).parent;
    let grand = (*parent).parent;
    (*child).parent = grand;
    (*parent).parent = child;
    if grand.is_null() {
        return;
    }
    if (*grand).left == parent {
        (*grand).left = child;
    } else {
        (*grand).right = child;
    }
}

/// Rotate to make the given child take its parent's place.
///
/// # Safety
/// Caller must ensure child has a valid parent pointer.
unsafe fn rotate(child: *mut PicosplayNode) {
    let parent = (*child).parent;
    debug_assert!(!parent.is_null());

    if (*parent).left == child {
        // Left child given
        mark_gp(child);
        (*parent).left = (*child).right;
        if !(*child).right.is_null() {
            (*(*child).right).parent = parent;
        }
        (*child).right = parent;
    } else {
        // Right child given
        mark_gp(child);
        (*parent).right = (*child).left;
        if !(*child).left.is_null() {
            (*(*child).left).parent = parent;
        }
        (*child).left = parent;
    }
}

/// Zig operation: when parent is root, rotate on edge between x and p.
///
/// # Safety
/// Caller must ensure x has a valid parent.
unsafe fn zig(x: *mut PicosplayNode) {
    rotate(x);
}

/// Zig-zig operation: when both x and p are left (or both right) children.
///
/// # Safety
/// Caller must ensure x and p are valid and p has a parent.
unsafe fn zigzig(x: *mut PicosplayNode, p: *mut PicosplayNode) {
    rotate(p);
    rotate(x);
}

/// Zig-zag operation: when one of x and p is left child and other is right.
///
/// # Safety
/// Caller must ensure x is valid with parent and grandparent.
unsafe fn zigzag(x: *mut PicosplayNode) {
    rotate(x);
    rotate(x);
}

/// Splay the node x to the root of the tree.
///
/// # Safety
/// Caller must ensure tree and x are valid.
unsafe fn splay(tree: *mut PicosplayTree, x: *mut PicosplayNode) {
    loop {
        let p = (*x).parent;
        if p.is_null() {
            (*tree).root = x;
            return;
        }
        let g = (*p).parent;
        if g.is_null() {
            zig(x);
        } else if (x == (*p).left && p == (*g).left) || (x == (*p).right && p == (*g).right) {
            zigzig(x, p);
        } else {
            zigzag(x);
        }
    }
}

// =============================================================================
// Public Tree Operations
// =============================================================================

/// Initialize an empty tree with the given callbacks.
///
/// # Safety
/// Caller must ensure tree pointer is valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_init_tree(
    tree: *mut PicosplayTree,
    comp: Comparator,
    create: CreateNode,
    delete_node: DeleteNode,
    node_value: NodeValue,
) {
    (*tree).comp = Some(comp);
    (*tree).create = Some(create);
    (*tree).delete_node = Some(delete_node);
    (*tree).node_value = Some(node_value);
    (*tree).root = ptr::null_mut();
    (*tree).size = 0;
}

/// Allocate and initialize a new tree.
///
/// Returns null on allocation failure.
#[no_mangle]
pub extern "C" fn picosplay_new_tree(
    comp: Comparator,
    create: CreateNode,
    delete_node: DeleteNode,
    node_value: NodeValue,
) -> *mut PicosplayTree {
    let tree = Box::new(PicosplayTree {
        root: ptr::null_mut(),
        comp: Some(comp),
        create: Some(create),
        delete_node: Some(delete_node),
        node_value: Some(node_value),
        size: 0,
    });
    Box::into_raw(tree)
}

/// Insert a new node with the given value.
///
/// Returns the new node, or null on failure.
///
/// # Safety
/// Caller must ensure tree is valid and value is appropriate for the callbacks.
#[no_mangle]
pub unsafe extern "C" fn picosplay_insert(
    tree: *mut PicosplayTree,
    value: *mut (),
) -> *mut PicosplayNode {
    let create_fn = match (*tree).create {
        Some(f) => f,
        None => return ptr::null_mut(),
    };

    let new_node = create_fn(value);
    if new_node.is_null() {
        return ptr::null_mut();
    }

    (*new_node).left = ptr::null_mut();
    (*new_node).right = ptr::null_mut();

    if (*tree).root.is_null() {
        (*tree).root = new_node;
        (*new_node).parent = ptr::null_mut();
    } else {
        let comp_fn = (*tree).comp.unwrap();
        let value_fn = (*tree).node_value.unwrap();

        let mut curr = (*tree).root;
        let mut parent = ptr::null_mut();
        let mut go_left = false;

        while !curr.is_null() {
            parent = curr;
            let cmp = comp_fn(value_fn(new_node), value_fn(curr));
            if cmp < 0 {
                go_left = true;
                curr = (*curr).left;
            } else {
                go_left = false;
                curr = (*curr).right;
            }
        }

        (*new_node).parent = parent;
        if go_left {
            (*parent).left = new_node;
        } else {
            (*parent).right = new_node;
        }
    }

    splay(tree, new_node);
    (*tree).size += 1;

    new_node
}

/// Find a node with the given value.
///
/// Returns null if not found.
///
/// # Safety
/// Caller must ensure tree and value are valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_find(
    tree: *mut PicosplayTree,
    value: *mut (),
) -> *mut PicosplayNode {
    let comp_fn = match (*tree).comp {
        Some(f) => f,
        None => return ptr::null_mut(),
    };
    let value_fn = match (*tree).node_value {
        Some(f) => f,
        None => return ptr::null_mut(),
    };

    let mut curr = (*tree).root;
    let mut found = false;

    while !curr.is_null() && !found {
        let relation = comp_fn(value, value_fn(curr));
        if relation == 0 {
            found = true;
        } else if relation < 0 {
            curr = (*curr).left;
        } else {
            curr = (*curr).right;
        }
    }

    if !curr.is_null() {
        splay(tree, curr);
    }

    curr
}

/// Find the node with the largest value less than or equal to the given value.
///
/// # Safety
/// Caller must ensure tree and value are valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_find_previous(
    tree: *mut PicosplayTree,
    value: *mut (),
) -> *mut PicosplayNode {
    let comp_fn = match (*tree).comp {
        Some(f) => f,
        None => return ptr::null_mut(),
    };
    let value_fn = match (*tree).node_value {
        Some(f) => f,
        None => return ptr::null_mut(),
    };

    let mut curr = (*tree).root;
    let mut previous = ptr::null_mut();
    let mut found = false;

    while !curr.is_null() && !found {
        let relation = comp_fn(value, value_fn(curr));
        if relation == 0 {
            found = true;
            previous = curr;
        } else if relation < 0 {
            curr = (*curr).left;
        } else {
            previous = curr;
            curr = (*curr).right;
        }
    }

    previous
}

/// Delete a node with the given value.
///
/// # Safety
/// Caller must ensure tree and value are valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_delete(tree: *mut PicosplayTree, value: *mut ()) {
    let node = picosplay_find(tree, value);
    picosplay_delete_hint(tree, node);
}

/// Delete the node given by pointer.
///
/// # Safety
/// Caller must ensure tree and node are valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_delete_hint(tree: *mut PicosplayTree, node: *mut PicosplayNode) {
    if node.is_null() {
        return;
    }

    splay(tree, node); // Now node is tree's root

    if (*node).left.is_null() {
        (*tree).root = (*node).right;
        if !(*tree).root.is_null() {
            (*(*tree).root).parent = ptr::null_mut();
        }
    } else if (*node).right.is_null() {
        (*tree).root = (*node).left;
        (*(*tree).root).parent = ptr::null_mut();
    } else {
        let x = leftmost((*node).right);
        if (*x).parent != node {
            (*(*x).parent).left = (*x).right;
            if !(*x).right.is_null() {
                (*(*x).right).parent = (*x).parent;
            }
            (*x).right = (*node).right;
            (*(*x).right).parent = x;
        }
        (*tree).root = x;
        (*x).parent = ptr::null_mut();
        (*x).left = (*node).left;
        (*(*x).left).parent = x;
    }

    if let Some(delete_fn) = (*tree).delete_node {
        delete_fn(tree as *mut (), node);
    }
    (*tree).size -= 1;
}

/// Empty the tree by deleting all nodes.
///
/// # Safety
/// Caller must ensure tree is valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_empty_tree(tree: *mut PicosplayTree) {
    if tree.is_null() {
        return;
    }
    while !(*tree).root.is_null() {
        picosplay_delete_hint(tree, (*tree).root);
    }
}

/// Get the first (leftmost) node in the tree.
///
/// # Safety
/// Caller must ensure tree is valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_first(tree: *mut PicosplayTree) -> *mut PicosplayNode {
    leftmost((*tree).root)
}

/// Get the previous node (in-order predecessor).
///
/// # Safety
/// Caller must ensure node is valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_previous(mut node: *mut PicosplayNode) -> *mut PicosplayNode {
    if !(*node).left.is_null() {
        return rightmost((*node).left);
    }
    while !(*node).parent.is_null() && node == (*(*node).parent).left {
        node = (*node).parent;
    }
    (*node).parent
}

/// Get the next node (in-order successor).
///
/// # Safety
/// Caller must ensure node is valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_next(mut node: *mut PicosplayNode) -> *mut PicosplayNode {
    if !(*node).right.is_null() {
        return leftmost((*node).right);
    }
    while !(*node).parent.is_null() && node == (*(*node).parent).right {
        node = (*node).parent;
    }
    (*node).parent
}

/// Get the last (rightmost) node in the tree.
///
/// # Safety
/// Caller must ensure tree is valid.
#[no_mangle]
pub unsafe extern "C" fn picosplay_last(tree: *mut PicosplayTree) -> *mut PicosplayNode {
    rightmost((*tree).root)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{alloc, dealloc, Layout};

    // Test node that contains an i64 value
    #[repr(C)]
    struct TestNode {
        node: PicosplayNode,
        value: i64,
    }

    // Comparator for i64 values
    unsafe extern "C" fn test_comp(left: *mut (), right: *mut ()) -> i64 {
        let l = *(left as *const i64);
        let r = *(right as *const i64);
        l - r
    }

    // Create a test node
    unsafe extern "C" fn test_create(value: *mut ()) -> *mut PicosplayNode {
        let layout = Layout::new::<TestNode>();
        let ptr = alloc(layout) as *mut TestNode;
        if ptr.is_null() {
            return ptr::null_mut();
        }
        (*ptr).node = PicosplayNode::new();
        (*ptr).value = *(value as *const i64);
        &mut (*ptr).node as *mut PicosplayNode
    }

    // Delete a test node
    unsafe extern "C" fn test_delete_cb(_tree: *mut (), node: *mut PicosplayNode) {
        let layout = Layout::new::<TestNode>();
        dealloc(node as *mut u8, layout);
    }

    // Get value from test node
    unsafe extern "C" fn test_value(node: *mut PicosplayNode) -> *mut () {
        let test_node = node as *mut TestNode;
        &mut (*test_node).value as *mut i64 as *mut ()
    }

    #[test]
    fn test_node_default() {
        let node = PicosplayNode::new();
        assert!(node.parent.is_null());
        assert!(node.left.is_null());
        assert!(node.right.is_null());
    }

    #[test]
    fn test_tree_default() {
        let tree = PicosplayTree::new();
        assert!(tree.root.is_null());
        assert!(tree.comp.is_none());
        assert_eq!(tree.size, 0);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_insert_single() {
        unsafe {
            let tree = picosplay_new_tree(test_comp, test_create, test_delete_cb, test_value);
            assert!(!tree.is_null());

            let mut val: i64 = 42;
            let node = picosplay_insert(tree, &mut val as *mut i64 as *mut ());
            assert!(!node.is_null());
            assert_eq!((*tree).size, 1);

            // Clean up
            picosplay_empty_tree(tree);
            assert_eq!((*tree).size, 0);
            let _ = Box::from_raw(tree);
        }
    }

    #[test]
    fn test_insert_multiple() {
        unsafe {
            let tree = picosplay_new_tree(test_comp, test_create, test_delete_cb, test_value);

            let values: [i64; 5] = [10, 5, 15, 3, 7];
            for mut val in values {
                let node = picosplay_insert(tree, &mut val as *mut i64 as *mut ());
                assert!(!node.is_null());
            }
            assert_eq!((*tree).size, 5);

            // Clean up
            picosplay_empty_tree(tree);
            let _ = Box::from_raw(tree);
        }
    }

    #[test]
    fn test_find() {
        unsafe {
            let tree = picosplay_new_tree(test_comp, test_create, test_delete_cb, test_value);

            let values: [i64; 5] = [10, 5, 15, 3, 7];
            for mut val in values {
                picosplay_insert(tree, &mut val as *mut i64 as *mut ());
            }

            // Find existing value
            let mut search_val: i64 = 7;
            let found = picosplay_find(tree, &mut search_val as *mut i64 as *mut ());
            assert!(!found.is_null());
            let found_val = *(test_value(found) as *const i64);
            assert_eq!(found_val, 7);

            // Find non-existing value
            let mut search_val: i64 = 100;
            let not_found = picosplay_find(tree, &mut search_val as *mut i64 as *mut ());
            assert!(not_found.is_null());

            picosplay_empty_tree(tree);
            let _ = Box::from_raw(tree);
        }
    }

    #[test]
    fn test_find_previous() {
        unsafe {
            let tree = picosplay_new_tree(test_comp, test_create, test_delete_cb, test_value);

            let values: [i64; 5] = [10, 5, 15, 3, 7];
            for mut val in values {
                picosplay_insert(tree, &mut val as *mut i64 as *mut ());
            }

            // Find previous for existing value
            let mut search_val: i64 = 7;
            let prev = picosplay_find_previous(tree, &mut search_val as *mut i64 as *mut ());
            assert!(!prev.is_null());
            let prev_val = *(test_value(prev) as *const i64);
            assert_eq!(prev_val, 7);

            // Find previous for non-existing value between others
            let mut search_val: i64 = 6;
            let prev = picosplay_find_previous(tree, &mut search_val as *mut i64 as *mut ());
            assert!(!prev.is_null());
            let prev_val = *(test_value(prev) as *const i64);
            assert_eq!(prev_val, 5);

            picosplay_empty_tree(tree);
            let _ = Box::from_raw(tree);
        }
    }

    #[test]
    fn test_delete_node() {
        unsafe {
            let tree = picosplay_new_tree(test_comp, test_create, test_delete_cb, test_value);

            let values: [i64; 5] = [10, 5, 15, 3, 7];
            for mut val in values {
                picosplay_insert(tree, &mut val as *mut i64 as *mut ());
            }
            assert_eq!((*tree).size, 5);

            // Delete a value
            let mut del_val: i64 = 5;
            picosplay_delete(tree, &mut del_val as *mut i64 as *mut ());
            assert_eq!((*tree).size, 4);

            // Verify it's gone
            let not_found = picosplay_find(tree, &mut del_val as *mut i64 as *mut ());
            assert!(not_found.is_null());

            picosplay_empty_tree(tree);
            let _ = Box::from_raw(tree);
        }
    }

    #[test]
    fn test_first_last() {
        unsafe {
            let tree = picosplay_new_tree(test_comp, test_create, test_delete_cb, test_value);

            let values: [i64; 5] = [10, 5, 15, 3, 7];
            for mut val in values {
                picosplay_insert(tree, &mut val as *mut i64 as *mut ());
            }

            let first = picosplay_first(tree);
            assert!(!first.is_null());
            let first_val = *(test_value(first) as *const i64);
            assert_eq!(first_val, 3);

            let last = picosplay_last(tree);
            assert!(!last.is_null());
            let last_val = *(test_value(last) as *const i64);
            assert_eq!(last_val, 15);

            picosplay_empty_tree(tree);
            let _ = Box::from_raw(tree);
        }
    }

    #[test]
    fn test_iteration() {
        unsafe {
            let tree = picosplay_new_tree(test_comp, test_create, test_delete_cb, test_value);

            let values: [i64; 5] = [10, 5, 15, 3, 7];
            for mut val in values {
                picosplay_insert(tree, &mut val as *mut i64 as *mut ());
            }

            // Forward iteration
            let mut collected = Vec::new();
            let mut node = picosplay_first(tree);
            while !node.is_null() {
                collected.push(*(test_value(node) as *const i64));
                node = picosplay_next(node);
            }
            assert_eq!(collected, vec![3, 5, 7, 10, 15]);

            // Backward iteration
            let mut collected = Vec::new();
            let mut node = picosplay_last(tree);
            while !node.is_null() {
                collected.push(*(test_value(node) as *const i64));
                node = picosplay_previous(node);
            }
            assert_eq!(collected, vec![15, 10, 7, 5, 3]);

            picosplay_empty_tree(tree);
            let _ = Box::from_raw(tree);
        }
    }

    #[test]
    fn test_empty_tree_operations() {
        unsafe {
            let tree = picosplay_new_tree(test_comp, test_create, test_delete_cb, test_value);

            // Operations on empty tree
            let first = picosplay_first(tree);
            assert!(first.is_null());

            let last = picosplay_last(tree);
            assert!(last.is_null());

            let mut val: i64 = 5;
            let not_found = picosplay_find(tree, &mut val as *mut i64 as *mut ());
            assert!(not_found.is_null());

            // Delete from empty tree should be safe
            picosplay_delete(tree, &mut val as *mut i64 as *mut ());
            assert_eq!((*tree).size, 0);

            let _ = Box::from_raw(tree);
        }
    }

    #[test]
    fn test_delete_cb_root() {
        unsafe {
            let tree = picosplay_new_tree(test_comp, test_create, test_delete_cb, test_value);

            let mut val: i64 = 42;
            picosplay_insert(tree, &mut val as *mut i64 as *mut ());
            assert_eq!((*tree).size, 1);

            picosplay_delete(tree, &mut val as *mut i64 as *mut ());
            assert_eq!((*tree).size, 0);
            assert!((*tree).root.is_null());

            let _ = Box::from_raw(tree);
        }
    }

    #[test]
    fn test_leftmost_rightmost() {
        // Test the internal helper functions
        let node = PicosplayNode::new();
        assert!(leftmost(ptr::null_mut()).is_null());
        assert!(rightmost(ptr::null_mut()).is_null());

        let node_ptr = &node as *const PicosplayNode as *mut PicosplayNode;
        assert_eq!(leftmost(node_ptr), node_ptr);
        assert_eq!(rightmost(node_ptr), node_ptr);
    }
}
