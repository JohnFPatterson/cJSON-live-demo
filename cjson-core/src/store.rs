//! The node-store abstraction every core algorithm is written against.
//!
//! A store holds nodes shaped exactly like the C `cJSON` struct
//! (cJSON.h:103-123). Algorithms manipulate them through handles so the same
//! safe code drives both the Rust [`Arena`] and the FFI crate's live C tree.
//!
//! String semantics follow C: a stored string is a NUL-terminated byte
//! buffer. Readers ([`NodeStore::valuestring`], [`NodeStore::string`]) return
//! the bytes *before the first NUL*, because every C consumer (`strlen`,
//! `strcmp`, `print_string_ptr`) stops there. Writers store the bytes they
//! are given verbatim (they may contain interior NULs, e.g. a parsed
//! `"\u0000"`), and the store appends the terminator.

use std::rc::Rc;

/// Allocation failed (the C code's `hooks->allocate` returned NULL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocError;

/// Access to a C-shaped `cJSON` node graph.
///
/// Ownership rules are the C ones: freeing a node does not free its strings
/// or children; callers decide based on `cJSON_IsReference` /
/// `cJSON_StringIsConst`, exactly as `cJSON_Delete` (cJSON.c:259-282) does.
pub trait NodeStore {
    /// Opaque node handle (a `cJSON *` in the FFI store).
    type Id: Copy + Eq + core::fmt::Debug;
    /// A caller-owned string that is stored without copying
    /// (`cJSON_AddItemToObjectCS` keys, `cJSON_CreateStringReference`).
    type Borrowed: Clone;

    // ----- allocation -------------------------------------------------------

    /// `cJSON_New_Item`: allocate a zeroed node (all links None, type 0,
    /// strings None, numbers 0).
    fn alloc_node(&mut self) -> Result<Self::Id, AllocError>;
    /// Free the node itself only (`global_hooks.deallocate(item)`).
    fn free_node(&mut self, id: Self::Id);

    // ----- links ------------------------------------------------------------

    fn next(&self, id: Self::Id) -> Option<Self::Id>;
    fn set_next(&mut self, id: Self::Id, next: Option<Self::Id>);
    fn prev(&self, id: Self::Id) -> Option<Self::Id>;
    fn set_prev(&mut self, id: Self::Id, prev: Option<Self::Id>);
    fn child(&self, id: Self::Id) -> Option<Self::Id>;
    fn set_child(&mut self, id: Self::Id, child: Option<Self::Id>);

    // ----- scalars ----------------------------------------------------------

    fn type_bits(&self, id: Self::Id) -> i32;
    fn set_type_bits(&mut self, id: Self::Id, type_bits: i32);
    fn valueint(&self, id: Self::Id) -> i32;
    fn set_valueint(&mut self, id: Self::Id, value: i32);
    fn valuedouble(&self, id: Self::Id) -> f64;
    fn set_valuedouble(&mut self, id: Self::Id, value: f64);

    // ----- valuestring ------------------------------------------------------

    /// Bytes before the first NUL, or None when the C pointer is NULL.
    fn valuestring(&self, id: Self::Id) -> Option<&[u8]>;
    /// Allocate a copy of `bytes` (+ terminator) and store it. Does NOT free
    /// the previous value; callers do that explicitly like the C code.
    fn set_valuestring_copy(&mut self, id: Self::Id, bytes: &[u8]) -> Result<(), AllocError>;
    /// Store a caller-owned string without copying.
    fn set_valuestring_borrowed(&mut self, id: Self::Id, s: Self::Borrowed);
    /// Free the stored valuestring (C `deallocate`) and set it to NULL.
    fn free_valuestring(&mut self, id: Self::Id);
    /// Set valuestring to NULL without freeing (ownership moved elsewhere).
    fn forget_valuestring(&mut self, id: Self::Id);
    /// C order `copy = strdup(bytes); if (free_old) free(old); v = copy;`.
    /// On allocation failure nothing changes. `bytes` may alias the old
    /// buffer in the FFI store, so the copy must be made before freeing.
    fn replace_valuestring_copy(
        &mut self,
        id: Self::Id,
        bytes: &[u8],
        free_old: bool,
    ) -> Result<(), AllocError>;

    // ----- string (object key) ----------------------------------------------

    fn string(&self, id: Self::Id) -> Option<&[u8]>;
    fn set_string_copy(&mut self, id: Self::Id, bytes: &[u8]) -> Result<(), AllocError>;
    fn set_string_borrowed(&mut self, id: Self::Id, s: Self::Borrowed);
    fn free_string(&mut self, id: Self::Id);
    fn forget_string(&mut self, id: Self::Id);
    /// Same contract as [`NodeStore::replace_valuestring_copy`] for the key
    /// (`add_item_to_object`, cJSON.c: strdup new key, then free old key
    /// unless `cJSON_StringIsConst`, then store).
    fn replace_string_copy(
        &mut self,
        id: Self::Id,
        bytes: &[u8],
        free_old: bool,
    ) -> Result<(), AllocError>;

    // ----- aliasing -----------------------------------------------------------

    /// `memcpy(dst, src, sizeof(cJSON))`: copy every field, including links
    /// and string pointers (aliasing them, not duplicating).
    fn copy_node_fields(&mut self, dst: Self::Id, src: Self::Id);
    /// `dst->string = src->string` (pointer alias, used for const keys in
    /// `cJSON_Duplicate`).
    fn alias_string(&mut self, dst: Self::Id, src: Self::Id);
}

/// Handle into an [`Arena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// Arena node: the Rust mirror of `struct cJSON`.
#[derive(Debug, Clone, Default)]
pub struct Node {
    pub next: Option<NodeId>,
    pub prev: Option<NodeId>,
    pub child: Option<NodeId>,
    pub type_bits: i32,
    /// Raw stored bytes (may contain interior NULs; readers truncate).
    pub valuestring: Option<Rc<[u8]>>,
    pub valueint: i32,
    pub valuedouble: f64,
    pub string: Option<Rc<[u8]>>,
}

/// Safe in-memory [`NodeStore`].
///
/// Strings are reference counted so aliasing (`cJSON_IsReference`,
/// const keys, `memcpy` in `create_reference`) is safe; "freeing" drops one
/// reference. Allocation failure can be injected to port the C tests that
/// install failing `cJSON_Hooks`.
#[derive(Debug, Default)]
pub struct Arena {
    nodes: Vec<Option<Node>>,
    free_slots: Vec<usize>,
    /// Remaining successful allocations (nodes and strings); None = unlimited.
    alloc_budget: Option<usize>,
    live_allocations: usize,
}

impl Arena {
    pub fn new() -> Self {
        Self::default()
    }

    /// Make every allocation after the next `n` fail (like a failing
    /// `malloc_fn` hook). `None` restores unlimited allocation.
    pub fn set_alloc_budget(&mut self, n: Option<usize>) {
        self.alloc_budget = n;
    }

    /// Number of nodes currently allocated (leak checks in tests).
    pub fn live_nodes(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }

    /// Number of node + string allocations not yet freed.
    pub fn live_allocations(&self) -> usize {
        self.live_allocations
    }

    /// Direct read access to a node (None if freed or out of range).
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0).and_then(Option::as_ref)
    }

    /// Direct write access, for tests that poke fields the way the C tests do.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.0).and_then(Option::as_mut)
    }

    fn take_budget(&mut self) -> Result<(), AllocError> {
        match self.alloc_budget {
            Some(0) => Err(AllocError),
            Some(n) => {
                self.alloc_budget = Some(n - 1);
                self.live_allocations += 1;
                Ok(())
            }
            None => {
                self.live_allocations += 1;
                Ok(())
            }
        }
    }

    fn release(&mut self) {
        self.live_allocations = self.live_allocations.saturating_sub(1);
    }

    /// Field read on a live node; a freed or unknown handle reads as a
    /// zeroed node (C would read freed memory; tests never rely on it).
    fn field<T: Default>(&self, id: NodeId, f: impl FnOnce(&Node) -> T) -> T {
        self.node(id).map(f).unwrap_or_default()
    }

    fn with_mut(&mut self, id: NodeId, f: impl FnOnce(&mut Node)) {
        if let Some(node) = self.node_mut(id) {
            f(node);
        }
    }
}

fn c_view(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(end) => &bytes[..end],
        None => bytes,
    }
}

impl NodeStore for Arena {
    type Id = NodeId;
    type Borrowed = Rc<[u8]>;

    fn alloc_node(&mut self) -> Result<NodeId, AllocError> {
        self.take_budget()?;
        let node = Some(Node::default());
        if let Some(slot) = self.free_slots.pop() {
            self.nodes[slot] = node;
            Ok(NodeId(slot))
        } else {
            self.nodes.push(node);
            Ok(NodeId(self.nodes.len() - 1))
        }
    }

    fn free_node(&mut self, id: NodeId) {
        if let Some(slot) = self.nodes.get_mut(id.0) {
            if slot.take().is_some() {
                self.free_slots.push(id.0);
                self.release();
            }
        }
    }

    fn next(&self, id: NodeId) -> Option<NodeId> {
        self.field(id, |n| n.next)
    }
    fn set_next(&mut self, id: NodeId, next: Option<NodeId>) {
        self.with_mut(id, |n| n.next = next);
    }
    fn prev(&self, id: NodeId) -> Option<NodeId> {
        self.field(id, |n| n.prev)
    }
    fn set_prev(&mut self, id: NodeId, prev: Option<NodeId>) {
        self.with_mut(id, |n| n.prev = prev);
    }
    fn child(&self, id: NodeId) -> Option<NodeId> {
        self.field(id, |n| n.child)
    }
    fn set_child(&mut self, id: NodeId, child: Option<NodeId>) {
        self.with_mut(id, |n| n.child = child);
    }

    fn type_bits(&self, id: NodeId) -> i32 {
        self.field(id, |n| n.type_bits)
    }
    fn set_type_bits(&mut self, id: NodeId, type_bits: i32) {
        self.with_mut(id, |n| n.type_bits = type_bits);
    }
    fn valueint(&self, id: NodeId) -> i32 {
        self.field(id, |n| n.valueint)
    }
    fn set_valueint(&mut self, id: NodeId, value: i32) {
        self.with_mut(id, |n| n.valueint = value);
    }
    fn valuedouble(&self, id: NodeId) -> f64 {
        self.field(id, |n| n.valuedouble)
    }
    fn set_valuedouble(&mut self, id: NodeId, value: f64) {
        self.with_mut(id, |n| n.valuedouble = value);
    }

    fn valuestring(&self, id: NodeId) -> Option<&[u8]> {
        self.node(id)
            .and_then(|n| n.valuestring.as_deref())
            .map(c_view)
    }
    fn set_valuestring_copy(&mut self, id: NodeId, bytes: &[u8]) -> Result<(), AllocError> {
        self.take_budget()?;
        let s: Rc<[u8]> = Rc::from(bytes);
        self.with_mut(id, |n| n.valuestring = Some(s));
        Ok(())
    }
    fn set_valuestring_borrowed(&mut self, id: NodeId, s: Rc<[u8]>) {
        self.with_mut(id, |n| n.valuestring = Some(s));
    }
    fn free_valuestring(&mut self, id: NodeId) {
        let had = self
            .node_mut(id)
            .and_then(|n| n.valuestring.take())
            .is_some();
        if had {
            self.release();
        }
    }
    fn forget_valuestring(&mut self, id: NodeId) {
        self.with_mut(id, |n| n.valuestring = None);
    }
    fn replace_valuestring_copy(
        &mut self,
        id: NodeId,
        bytes: &[u8],
        free_old: bool,
    ) -> Result<(), AllocError> {
        self.take_budget()?;
        let s: Rc<[u8]> = Rc::from(bytes);
        let old = self.node_mut(id).and_then(|n| n.valuestring.replace(s));
        if free_old && old.is_some() {
            self.release();
        }
        Ok(())
    }

    fn string(&self, id: NodeId) -> Option<&[u8]> {
        self.node(id).and_then(|n| n.string.as_deref()).map(c_view)
    }
    fn set_string_copy(&mut self, id: NodeId, bytes: &[u8]) -> Result<(), AllocError> {
        self.take_budget()?;
        let s: Rc<[u8]> = Rc::from(bytes);
        self.with_mut(id, |n| n.string = Some(s));
        Ok(())
    }
    fn set_string_borrowed(&mut self, id: NodeId, s: Rc<[u8]>) {
        self.with_mut(id, |n| n.string = Some(s));
    }
    fn free_string(&mut self, id: NodeId) {
        let had = self.node_mut(id).and_then(|n| n.string.take()).is_some();
        if had {
            self.release();
        }
    }
    fn forget_string(&mut self, id: NodeId) {
        self.with_mut(id, |n| n.string = None);
    }
    fn replace_string_copy(
        &mut self,
        id: NodeId,
        bytes: &[u8],
        free_old: bool,
    ) -> Result<(), AllocError> {
        self.take_budget()?;
        let s: Rc<[u8]> = Rc::from(bytes);
        let old = self.node_mut(id).and_then(|n| n.string.replace(s));
        if free_old && old.is_some() {
            self.release();
        }
        Ok(())
    }

    fn copy_node_fields(&mut self, dst: NodeId, src: NodeId) {
        let copy = self.node(src).cloned().unwrap_or_default();
        self.with_mut(dst, |n| *n = copy);
    }
    fn alias_string(&mut self, dst: NodeId, src: NodeId) {
        let s = self.node(src).and_then(|n| n.string.clone());
        self.with_mut(dst, |n| n.string = s);
    }
}
