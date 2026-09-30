//! The process-wide state `cJSON.c` keeps: `global_hooks` (cJSON.c:160-190,
//! `cJSON_InitHooks` at cJSON.c:215-244) and `global_error` (cJSON.c:88-98).
//!
//! C keeps both in plain `static` variables with no synchronization. Here the
//! hooks sit behind a `Mutex` and the error behind atomics so concurrent
//! callers cannot tear a function pointer; the pair (json, position) can
//! still be observed half-updated across threads, which is no weaker than C.

use core::ffi::{c_char, c_void};
use core::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::types::{cJSON_Hooks, FreeFn, MallocFn, ReallocFn};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub(crate) fn strlen(s: *const c_char) -> usize;
}

/// `internal_hooks` (cJSON.c:160-165).
#[derive(Clone, Copy)]
struct InternalHooks {
    allocate: MallocFn,
    deallocate: FreeFn,
    reallocate: Option<ReallocFn>,
}

const DEFAULT_HOOKS: InternalHooks = InternalHooks {
    allocate: malloc,
    deallocate: free,
    reallocate: Some(realloc),
};

static GLOBAL_HOOKS: Mutex<InternalHooks> = Mutex::new(DEFAULT_HOOKS);

/// Snapshot of the hooks. The lock is released before any hook runs, so a
/// hook that calls back into cJSON cannot deadlock.
fn current() -> InternalHooks {
    *GLOBAL_HOOKS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn install(hooks: InternalHooks) {
    *GLOBAL_HOOKS.lock().unwrap_or_else(PoisonError::into_inner) = hooks;
}

fn same_fn(a: *const (), b: *const ()) -> bool {
    ptr::eq(a, b)
}

/// `cJSON_InitHooks` body (cJSON.c:215-244); `None` models `hooks == NULL`.
pub(crate) fn init_hooks(hooks: Option<cJSON_Hooks>) {
    let Some(hooks) = hooks else {
        install(DEFAULT_HOOKS);
        return;
    };
    let allocate = hooks.malloc_fn.unwrap_or(malloc);
    let deallocate = hooks.free_fn.unwrap_or(free);
    let libc_pair = same_fn(allocate as *const (), malloc as MallocFn as *const ())
        && same_fn(deallocate as *const (), free as FreeFn as *const ());
    install(InternalHooks {
        allocate,
        deallocate,
        reallocate: if libc_pair { Some(realloc) } else { None },
    });
}

/// Whether `global_hooks.reallocate` is set (cJSON.c:239-243). The Rust
/// print path allocates its result once and never reallocates, so nothing
/// else reads it; exposed so tests can pin the `cJSON_InitHooks` rule.
#[doc(hidden)]
pub fn reallocate_hook_installed() -> bool {
    current().reallocate.is_some()
}

/// `global_hooks.allocate(size)`.
pub(crate) fn allocate(size: usize) -> *mut c_void {
    let hooks = current();
    // SAFETY: `allocate` is libc `malloc` or a `malloc_fn` the C caller
    // installed through `cJSON_InitHooks`, whose contract (cJSON.h:125-130)
    // is malloc semantics: any size is a valid argument.
    unsafe { (hooks.allocate)(size) }
}

/// `global_hooks.deallocate(pointer)`.
///
/// # Safety
///
/// `pointer` is NULL or was returned by [`allocate`] (or by the hook that is
/// installed now, as in C) and has not been freed yet.
pub(crate) unsafe fn deallocate(pointer: *mut c_void) {
    let hooks = current();
    // SAFETY: forwarded from this function's contract; `deallocate` is libc
    // `free` or the caller's `free_fn` with free semantics.
    unsafe { (hooks.deallocate)(pointer) }
}

// ---------------------------------------------------------------- global_error

static ERROR_JSON: AtomicPtr<c_char> = AtomicPtr::new(ptr::null_mut());
static ERROR_POSITION: AtomicUsize = AtomicUsize::new(0);

/// `global_error.json = NULL; global_error.position = 0;` (cJSON.c:1160-1161).
pub(crate) fn reset_error() {
    set_error(ptr::null(), 0);
}

/// `global_error = local_error;` (cJSON.c:1227).
pub(crate) fn set_error(json: *const c_char, position: usize) {
    ERROR_JSON.store(json.cast_mut(), Ordering::SeqCst);
    ERROR_POSITION.store(position, Ordering::SeqCst);
}

/// `global_error.json + global_error.position` (cJSON.c:97). With the reset
/// state this is `NULL + 0`, i.e. NULL. `wrapping_add` never dereferences;
/// for a recorded error the result lies inside the caller's input buffer.
pub(crate) fn error_ptr() -> *const c_char {
    let json = ERROR_JSON.load(Ordering::SeqCst).cast_const();
    json.wrapping_add(ERROR_POSITION.load(Ordering::SeqCst))
}
