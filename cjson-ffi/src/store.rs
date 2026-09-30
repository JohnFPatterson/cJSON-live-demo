//! [`CStore`]: `cjson_core::NodeStore` over the live, C-owned `cJSON` tree.
//!
//! # Soundness model
//!
//! * A [`CNode`] can only be made from a raw pointer through the `unsafe`
//!   [`CNode::from_raw`], whose contract is the crate-level pointer contract
//!   (see the crate docs), or by the store itself (`alloc_node`, link reads).
//! * A [`CStore`] lives for exactly one exported call. It never releases
//!   memory before it is dropped: every `free_*` queues the pointer and
//!   `Drop` hands the queue to the deallocate hook in the order C would have
//!   freed them. So every handle and every `&[u8]` the core holds, including
//!   byte slices of caller strings that alias a buffer being replaced or
//!   deleted (`cJSON_SetValuestring`, `cJSON_AddItemToObject(o, i->string, i)`,
//!   `cJSON_ReplaceItemInObject(o, old->string, n)`), stays valid for the
//!   whole core call. Allocation order relative to frees is unchanged.
//! * Node fields are accessed through short-lived references created in
//!   [`CStore::node`] / [`CStore::node_mut`]; no reference to a node outlives
//!   a single trait method. String slices returned by `valuestring`/`string`
//!   borrow `&self`, so the store cannot be mutated while they are alive, and
//!   the store never writes into string bytes.

use core::ffi::{c_char, c_void};
use core::ptr::{self, NonNull};
use core::slice;
use std::mem::size_of;

use cjson_core::store::{AllocError, NodeStore};

use crate::globals::{allocate, deallocate, strlen};
use crate::types::cJSON;

/// `cJSON *` handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CNode(NonNull<cJSON>);

impl CNode {
    /// `None` for NULL.
    ///
    /// # Safety
    ///
    /// `p` is NULL or satisfies the crate-level node contract: it points to
    /// a live, aligned `cJSON` whose `next`/`prev`/`child` are NULL or live
    /// nodes (recursively), whose `valuestring`/`string` are NULL or
    /// NUL-terminated strings, and which nobody else frees or mutates while
    /// the exported call that received it runs.
    pub(crate) unsafe fn from_raw(p: *const cJSON) -> Option<Self> {
        NonNull::new(p.cast_mut()).map(Self)
    }

    pub(crate) fn as_ptr(self) -> *mut cJSON {
        self.0.as_ptr()
    }
}

/// Caller-owned `const char *` stored without copying
/// (`cJSON_AddItemToObjectCS`, `cJSON_CreateStringReference`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct CBorrowed(NonNull<c_char>);

impl CBorrowed {
    /// `None` for NULL.
    ///
    /// # Safety
    ///
    /// `p` is NULL or a NUL-terminated string that outlives every node it
    /// gets stored in (the cJSON.h:230-232 / cJSON.h:212-213 contract).
    pub(crate) unsafe fn from_raw(p: *const c_char) -> Option<Self> {
        NonNull::new(p.cast_mut()).map(Self)
    }
}

/// The bytes of a C string before its NUL; `None` for NULL.
///
/// # Safety
///
/// `p` is NULL or points to a NUL-terminated string that stays valid and
/// unmodified for `'a`.
pub(crate) unsafe fn c_str<'a>(p: *const c_char) -> Option<&'a [u8]> {
    if p.is_null() {
        return None;
    }
    // SAFETY: `p` is a valid NUL-terminated string (contract above), so
    // `strlen` stays in bounds and the `len` bytes before the NUL are
    // initialized, readable and unmodified for `'a`.
    Some(unsafe { slice::from_raw_parts(p.cast::<u8>(), strlen(p)) })
}

/// `count` elements at `p`; `None` when `p == NULL || count < 0`.
///
/// # Safety
///
/// If `p` is non-NULL and `count >= 0`, `p` points to `count` initialized,
/// aligned `T`s that stay valid and unmodified for `'a` (the cJSON.h:220-221
/// contract of the `cJSON_Create*Array` functions).
pub(crate) unsafe fn c_array<'a, T>(p: *const T, count: core::ffi::c_int) -> Option<&'a [T]> {
    let len = usize::try_from(count).ok()?;
    if p.is_null() {
        return None;
    }
    // SAFETY: non-NULL, aligned, `len` readable elements for `'a` (contract).
    Some(unsafe { slice::from_raw_parts(p, len) })
}

/// Allocate `bytes.len() + 1` bytes with the allocate hook and store
/// `bytes` + NUL, like `cJSON_strdup` (cJSON.c:193-212). Interior NULs are
/// copied verbatim, so C readers see the string end at the first one.
pub(crate) fn alloc_c_bytes(bytes: &[u8]) -> Option<NonNull<c_char>> {
    // A slice never exceeds isize::MAX bytes, so `+ 1` cannot overflow.
    let size = bytes.len() + 1;
    let p = NonNull::new(allocate(size).cast::<c_char>())?;
    // SAFETY: `p` is a fresh allocation of `size` bytes, so it is valid for
    // `bytes.len() + 1` writes and cannot overlap `bytes`.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), p.as_ptr().cast::<u8>(), bytes.len());
        p.as_ptr().add(bytes.len()).write(0);
    }
    Some(p)
}

/// Store over the caller's C tree for the duration of one exported call.
#[derive(Default)]
pub(crate) struct CStore {
    /// Pointers freed by the core, released on drop (see module docs).
    pending_free: Vec<NonNull<c_void>>,
}

impl CStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn node(&self, id: CNode) -> &cJSON {
        // SAFETY: `id` is a live node (CNode contract, or allocated by this
        // store) and stays allocated until this store drops. No `&mut` to the
        // same node exists: `node_mut` references never escape a method.
        unsafe { id.0.as_ref() }
    }

    fn node_mut(&mut self, id: CNode) -> &mut cJSON {
        // SAFETY: as in `node`; `&mut self` guarantees no reference obtained
        // from this store is alive, and the C caller does not touch the tree
        // during the call (crate pointer contract), so this is unique.
        unsafe { &mut *id.0.as_ptr() }
    }

    fn defer_free(&mut self, p: *mut c_void) {
        if let Some(p) = NonNull::new(p) {
            self.pending_free.push(p);
        }
    }

    /// The node's raw `valuestring` pointer (what C returns from
    /// `cJSON_GetStringValue` / `cJSON_SetValuestring`).
    pub(crate) fn raw_valuestring(&self, id: CNode) -> *mut c_char {
        self.node(id).valuestring
    }

    fn view(&self, p: *mut c_char) -> Option<&[u8]> {
        // SAFETY: node contract: a non-NULL `valuestring`/`string` of a node
        // is a NUL-terminated string. The slice borrows `&self`; the bytes are
        // not freed before this store drops and never written through it.
        unsafe { c_str(p) }
    }
}

impl Drop for CStore {
    fn drop(&mut self) {
        for p in self.pending_free.drain(..) {
            // SAFETY: every queued pointer was a node or string allocated with
            // the allocate hook (by C or by this crate) that the core freed
            // exactly once, mirroring a `global_hooks.deallocate` call in C.
            unsafe { deallocate(p.as_ptr()) };
        }
    }
}

impl NodeStore for CStore {
    type Id = CNode;
    type Borrowed = CBorrowed;

    fn alloc_node(&mut self) -> Result<CNode, AllocError> {
        let p = NonNull::new(allocate(size_of::<cJSON>()).cast::<cJSON>()).ok_or(AllocError)?;
        // SAFETY: fresh allocation of sizeof(cJSON) bytes; malloc (and hook)
        // results are aligned for any object type. All-zero bytes are a valid
        // `cJSON` (NULL pointers, 0, 0.0); this is the memset in
        // `cJSON_New_Item` (cJSON.c:247-256), padding included.
        unsafe { p.as_ptr().write_bytes(0, 1) };
        Ok(CNode(p))
    }

    fn free_node(&mut self, id: CNode) {
        self.defer_free(id.as_ptr().cast());
    }

    fn next(&self, id: CNode) -> Option<CNode> {
        NonNull::new(self.node(id).next).map(CNode)
    }
    fn set_next(&mut self, id: CNode, next: Option<CNode>) {
        self.node_mut(id).next = next.map_or(ptr::null_mut(), CNode::as_ptr);
    }
    fn prev(&self, id: CNode) -> Option<CNode> {
        NonNull::new(self.node(id).prev).map(CNode)
    }
    fn set_prev(&mut self, id: CNode, prev: Option<CNode>) {
        self.node_mut(id).prev = prev.map_or(ptr::null_mut(), CNode::as_ptr);
    }
    fn child(&self, id: CNode) -> Option<CNode> {
        NonNull::new(self.node(id).child).map(CNode)
    }
    fn set_child(&mut self, id: CNode, child: Option<CNode>) {
        self.node_mut(id).child = child.map_or(ptr::null_mut(), CNode::as_ptr);
    }

    fn type_bits(&self, id: CNode) -> i32 {
        self.node(id).r#type
    }
    fn set_type_bits(&mut self, id: CNode, type_bits: i32) {
        self.node_mut(id).r#type = type_bits;
    }
    fn valueint(&self, id: CNode) -> i32 {
        self.node(id).valueint
    }
    fn set_valueint(&mut self, id: CNode, value: i32) {
        self.node_mut(id).valueint = value;
    }
    fn valuedouble(&self, id: CNode) -> f64 {
        self.node(id).valuedouble
    }
    fn set_valuedouble(&mut self, id: CNode, value: f64) {
        self.node_mut(id).valuedouble = value;
    }

    fn valuestring(&self, id: CNode) -> Option<&[u8]> {
        self.view(self.node(id).valuestring)
    }
    fn set_valuestring_copy(&mut self, id: CNode, bytes: &[u8]) -> Result<(), AllocError> {
        let copy = alloc_c_bytes(bytes).ok_or(AllocError)?;
        self.node_mut(id).valuestring = copy.as_ptr();
        Ok(())
    }
    fn set_valuestring_borrowed(&mut self, id: CNode, s: CBorrowed) {
        self.node_mut(id).valuestring = s.0.as_ptr();
    }
    fn free_valuestring(&mut self, id: CNode) {
        let old = core::mem::replace(&mut self.node_mut(id).valuestring, ptr::null_mut());
        self.defer_free(old.cast());
    }
    fn forget_valuestring(&mut self, id: CNode) {
        self.node_mut(id).valuestring = ptr::null_mut();
    }
    fn replace_valuestring_copy(
        &mut self,
        id: CNode,
        bytes: &[u8],
        free_old: bool,
    ) -> Result<(), AllocError> {
        // Copy first: `bytes` may alias the old buffer.
        let copy = alloc_c_bytes(bytes).ok_or(AllocError)?;
        let old = core::mem::replace(&mut self.node_mut(id).valuestring, copy.as_ptr());
        if free_old {
            self.defer_free(old.cast());
        }
        Ok(())
    }

    fn string(&self, id: CNode) -> Option<&[u8]> {
        self.view(self.node(id).string)
    }
    fn set_string_copy(&mut self, id: CNode, bytes: &[u8]) -> Result<(), AllocError> {
        let copy = alloc_c_bytes(bytes).ok_or(AllocError)?;
        self.node_mut(id).string = copy.as_ptr();
        Ok(())
    }
    fn set_string_borrowed(&mut self, id: CNode, s: CBorrowed) {
        self.node_mut(id).string = s.0.as_ptr();
    }
    fn free_string(&mut self, id: CNode) {
        let old = core::mem::replace(&mut self.node_mut(id).string, ptr::null_mut());
        self.defer_free(old.cast());
    }
    fn forget_string(&mut self, id: CNode) {
        self.node_mut(id).string = ptr::null_mut();
    }
    fn replace_string_copy(
        &mut self,
        id: CNode,
        bytes: &[u8],
        free_old: bool,
    ) -> Result<(), AllocError> {
        // Copy first: `bytes` may alias the old key.
        let copy = alloc_c_bytes(bytes).ok_or(AllocError)?;
        let old = core::mem::replace(&mut self.node_mut(id).string, copy.as_ptr());
        if free_old {
            self.defer_free(old.cast());
        }
        Ok(())
    }

    fn copy_node_fields(&mut self, dst: CNode, src: CNode) {
        let fields = *self.node(src);
        *self.node_mut(dst) = fields;
    }
    fn alias_string(&mut self, dst: CNode, src: CNode) {
        let s = self.node(src).string;
        self.node_mut(dst).string = s;
    }
}
