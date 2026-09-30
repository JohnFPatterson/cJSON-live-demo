//! `#[repr(C)]` mirrors of the structs declared in `cJSON.h`.
//!
//! The header macros `cJSON_ArrayForEach`, `cJSON_SetIntValue`,
//! `cJSON_SetNumberValue` and `cJSON_SetBoolValue` (cJSON.h:281-296) access
//! these fields directly from C, so the layout must be exact. The `const`
//! assertions at the bottom fail the build if it drifts.

use core::ffi::{c_char, c_int, c_void};
use core::mem::{align_of, offset_of, size_of};

/// `typedef struct cJSON { ... } cJSON;` (cJSON.h:103-123).
// The name is fixed by cJSON.h; C callers see this struct through the header.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub r#type: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: f64,
    pub string: *mut c_char,
}

/// `void *(CJSON_CDECL *malloc_fn)(size_t sz)` (cJSON.h:128).
///
/// # Safety
///
/// Implementations have `malloc` semantics; any size may be passed.
pub type MallocFn = unsafe extern "C" fn(usize) -> *mut c_void;
/// `void (CJSON_CDECL *free_fn)(void *ptr)` (cJSON.h:129).
///
/// # Safety
///
/// Callers pass NULL or a live pointer from the paired [`MallocFn`].
pub type FreeFn = unsafe extern "C" fn(*mut c_void);
/// `void *(CJSON_CDECL *reallocate)(void *pointer, size_t size)` (cJSON.c:164).
///
/// # Safety
///
/// `realloc` semantics: callers pass NULL or a live `malloc` pointer.
pub type ReallocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;

/// `typedef struct cJSON_Hooks { ... } cJSON_Hooks;` (cJSON.h:125-130).
/// `Option<fn>` has the same ABI as a nullable C function pointer.
// The name is fixed by cJSON.h; C callers see this struct through the header.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<MallocFn>,
    pub free_fn: Option<FreeFn>,
}

// ---------------------------------------------------------------- layout
//
// Expected offsets follow the C struct layout rule (each member at the next
// multiple of its alignment, size rounded up to the struct alignment) using
// the sizes of the C types on the target, so the check holds on any ABI.

const fn align_up(offset: usize, align: usize) -> usize {
    offset.div_ceil(align) * align
}

const PTR_SIZE: usize = size_of::<*mut c_void>();
const PTR_ALIGN: usize = align_of::<*mut c_void>();
const INT_SIZE: usize = size_of::<c_int>();
const INT_ALIGN: usize = align_of::<c_int>();
const DBL_SIZE: usize = size_of::<f64>();
const DBL_ALIGN: usize = align_of::<f64>();

const C_NEXT: usize = 0;
const C_PREV: usize = align_up(C_NEXT + PTR_SIZE, PTR_ALIGN);
const C_CHILD: usize = align_up(C_PREV + PTR_SIZE, PTR_ALIGN);
const C_TYPE: usize = align_up(C_CHILD + PTR_SIZE, INT_ALIGN);
const C_VALUESTRING: usize = align_up(C_TYPE + INT_SIZE, PTR_ALIGN);
const C_VALUEINT: usize = align_up(C_VALUESTRING + PTR_SIZE, INT_ALIGN);
const C_VALUEDOUBLE: usize = align_up(C_VALUEINT + INT_SIZE, DBL_ALIGN);
const C_STRING: usize = align_up(C_VALUEDOUBLE + DBL_SIZE, PTR_ALIGN);
const C_ALIGN: usize = if PTR_ALIGN > DBL_ALIGN {
    PTR_ALIGN
} else {
    DBL_ALIGN
};
const C_SIZE: usize = align_up(C_STRING + PTR_SIZE, C_ALIGN);

const _: () = assert!(offset_of!(cJSON, next) == C_NEXT);
const _: () = assert!(offset_of!(cJSON, prev) == C_PREV);
const _: () = assert!(offset_of!(cJSON, child) == C_CHILD);
const _: () = assert!(offset_of!(cJSON, r#type) == C_TYPE);
const _: () = assert!(offset_of!(cJSON, valuestring) == C_VALUESTRING);
const _: () = assert!(offset_of!(cJSON, valueint) == C_VALUEINT);
const _: () = assert!(offset_of!(cJSON, valuedouble) == C_VALUEDOUBLE);
const _: () = assert!(offset_of!(cJSON, string) == C_STRING);
const _: () = assert!(size_of::<cJSON>() == C_SIZE);
const _: () = assert!(align_of::<cJSON>() == C_ALIGN);

const _: () = assert!(offset_of!(cJSON_Hooks, malloc_fn) == 0);
const _: () = assert!(offset_of!(cJSON_Hooks, free_fn) == PTR_SIZE);
const _: () = assert!(size_of::<cJSON_Hooks>() == 2 * PTR_SIZE);
const _: () = assert!(size_of::<Option<MallocFn>>() == PTR_SIZE);

// LP64 (this platform: aarch64/x86_64 macOS and Linux): the concrete numbers
// printed by `offsetof` in C, pinned so a wrong layout rule above cannot hide.
#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(offset_of!(cJSON, next) == 0);
    assert!(offset_of!(cJSON, prev) == 8);
    assert!(offset_of!(cJSON, child) == 16);
    assert!(offset_of!(cJSON, r#type) == 24);
    assert!(offset_of!(cJSON, valuestring) == 32);
    assert!(offset_of!(cJSON, valueint) == 40);
    assert!(offset_of!(cJSON, valuedouble) == 48);
    assert!(offset_of!(cJSON, string) == 56);
    assert!(size_of::<cJSON>() == 64);
    assert!(size_of::<cJSON_Hooks>() == 16);
};
