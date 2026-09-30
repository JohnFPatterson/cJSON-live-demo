//! Pins the allocator-hook call sequences of the Rust library, which differ
//! from cJSON.c (MIGRATION.md "Behavior changed on purpose", CH-1 to CH-3). The C
//! sequence for each case is quoted in the test's doc comment; it is the
//! output of `make hook-trace` (tools/hook-trace.c linked against cJSON.c).
//! Parse and print results are identical; only the calls into the hooks
//! differ.
//!
//! The hooks are process-wide, so every test holds `LOCK`.
#![deny(clippy::undocumented_unsafe_blocks)]

use core::ffi::{c_char, c_void, CStr};
use core::ptr;
use std::sync::{Mutex, MutexGuard, PoisonError};

use cjson_ffi::cJSON_Hooks;
use cjson_ffi::exports::*;

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ev {
    M(usize),
    F(usize),
}

/// Events recorded by the hooks, and `(address, size)` of every live block.
type Trace = (Vec<Ev>, Vec<(usize, usize)>);

static TRACE: Mutex<Trace> = Mutex::new((Vec::new(), Vec::new()));

fn trace() -> MutexGuard<'static, Trace> {
    TRACE.lock().unwrap_or_else(PoisonError::into_inner)
}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// # Safety
///
/// Same contract as libc `malloc`.
unsafe extern "C" fn trace_malloc(size: usize) -> *mut c_void {
    // SAFETY: plain forwarding to libc malloc.
    let p = unsafe { malloc(size) };
    if !p.is_null() {
        let mut t = trace();
        t.0.push(Ev::M(size));
        t.1.push((p as usize, size));
    }
    p
}

/// # Safety
///
/// Same contract as libc `free`: `p` is NULL or from `trace_malloc`.
unsafe extern "C" fn trace_free(p: *mut c_void) {
    if !p.is_null() {
        let mut t = trace();
        let pos = t.1.iter().position(|&(q, _)| q == p as usize);
        let size = pos.map_or(usize::MAX, |i| t.1.swap_remove(i).1);
        t.0.push(Ev::F(size));
    }
    // SAFETY: `p` came from `trace_malloc` (libc malloc) or is NULL.
    unsafe { free(p) }
}

/// Holds `LOCK` with the tracing hooks installed; restores the default
/// hooks on drop.
struct Traced {
    _guard: MutexGuard<'static, ()>,
}

impl Traced {
    fn install() -> Self {
        let guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let hooks = cJSON_Hooks {
            malloc_fn: Some(trace_malloc),
            free_fn: Some(trace_free),
        };
        // SAFETY: valid hooks struct; both functions have malloc/free semantics.
        unsafe { cJSON_InitHooks(&hooks as *const cJSON_Hooks as *mut cJSON_Hooks) };
        take();
        Traced { _guard: guard }
    }
}

impl Drop for Traced {
    fn drop(&mut self) {
        // SAFETY: NULL resets to the default malloc/free (cJSON.c:217-223).
        unsafe { cJSON_InitHooks(ptr::null_mut()) };
    }
}

fn take() -> Vec<Ev> {
    core::mem::take(&mut trace().0)
}

const SAMPLE: &[u8] = b"{\"key\":[\"v\",1,null],\"s\":\"text\"}\0";

use Ev::{F, M};

/// C: `M 64, M 64, M 5, M 64, M 3, M 64, M 2, F 2, M 64, M 64, M 3, M 6`.
/// C allocates each parsed string one byte larger than needed (the length
/// includes the opening quote, cJSON.c parse_string) and frees the number
/// scratch buffer at once. Rust allocates `strlen + 1`, replays the scratch
/// allocation as a node-sized block, and frees it when the call returns.
#[test]
fn parse_allocations() {
    let _t = Traced::install();
    // SAFETY: NUL-terminated literal; the tree is deleted once.
    unsafe {
        let root = cJSON_Parse(SAMPLE.as_ptr().cast());
        assert!(!root.is_null());
        assert_eq!(
            take(),
            [
                M(64),
                M(64),
                M(4),
                M(64),
                M(2),
                M(64),
                M(64),
                M(64),
                M(64),
                M(2),
                M(5),
                F(64)
            ]
        );
        cJSON_Delete(root);
    }
}

/// C: `M 256, M 32, F 256` (default 256-byte buffer, then an exact copy
/// because no realloc hook is installed; cJSON.c:1250-1300). Rust: one
/// exact allocation, so a malloc hook that fails on its 2nd call makes
/// C return NULL while Rust succeeds.
#[test]
fn print_allocates_once() {
    let _t = Traced::install();
    // SAFETY: NUL-terminated literal; printed buffer freed with cJSON_free;
    // the tree is deleted once.
    unsafe {
        let root = cJSON_Parse(SAMPLE.as_ptr().cast());
        take();
        let out: *mut c_char = cJSON_PrintUnformatted(root);
        assert_eq!(take(), [M(32)]);
        assert_eq!(CStr::from_ptr(out).to_bytes(), &SAMPLE[..SAMPLE.len() - 1]);
        cJSON_free(out.cast());
        take();

        let out = cJSON_PrintBuffered(root, 4, 0);
        // C: M 4, M 16, F 4, M 40, F 16 (prebuffer, then growth by copy).
        assert_eq!(take(), [M(32)]);
        cJSON_free(out.cast());
        cJSON_Delete(root);
    }
}

/// C: `F 4, M 4, ...` (frees the replacement's old key, then copies the
/// new one; cJSON.c:2477-2481). Rust copies first and releases the old key
/// when the call returns, along with the replaced subtree.
#[test]
fn replace_item_in_object_copies_key_before_releasing() {
    let _t = Traced::install();
    // SAFETY: literals NUL-terminated; `rep` is detached before the
    // replace, which takes ownership of it; the tree is deleted once.
    unsafe {
        let root = cJSON_Parse(SAMPLE.as_ptr().cast());
        let rep = cJSON_CreateString(c"new".as_ptr());
        assert_eq!(cJSON_AddItemToObject(root, c"tmp".as_ptr(), rep), 1);
        assert_eq!(cJSON_DetachItemViaPointer(root, rep), rep);
        take();
        assert_eq!(cJSON_ReplaceItemInObject(root, c"key".as_ptr(), rep), 1);
        // C: F 4, M 4, F 3, F 64, F 64, F 64, F 5, F 64.
        assert_eq!(take(), [M(4), F(4), F(2), F(64), F(64), F(64), F(4), F(64)]);
        cJSON_Delete(root);
    }
}

/// MIGRATION.md CH-3. Passing the replacement's own key as the name is a
/// use-after-free in C (the key is freed, then strdup reads it). Rust reads
/// it before releasing it, so the replace succeeds with the right key.
#[test]
fn replace_item_in_object_with_aliased_key() {
    let _t = Traced::install();
    // SAFETY: literals NUL-terminated; `(*rep).string` is a valid C string
    // owned by `rep` for the duration of the call; the tree is deleted once.
    unsafe {
        let root = cJSON_Parse(SAMPLE.as_ptr().cast());
        let rep = cJSON_CreateString(c"new".as_ptr());
        assert_eq!(cJSON_AddItemToObject(root, c"key".as_ptr(), rep), 1);
        assert_eq!(cJSON_DetachItemViaPointer(root, rep), rep);
        assert_eq!(cJSON_ReplaceItemInObject(root, (*rep).string, rep), 1);
        let out = cJSON_PrintUnformatted(root);
        assert_eq!(
            CStr::from_ptr(out).to_bytes(),
            b"{\"key\":\"new\",\"s\":\"text\"}"
        );
        cJSON_free(out.cast());
        cJSON_Delete(root);
    }
}
