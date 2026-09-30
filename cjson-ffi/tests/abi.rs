//! Calls the exported C ABI through Rust. Expected values were produced by
//! the same calls against the original `cJSON.c` (build/ffi/abi_oracle.c).
//!
//! The hooks and the error pointer are process-wide, so every test holds
//! `LOCK` to keep the parallel test threads from observing each other.
#![deny(clippy::undocumented_unsafe_blocks)]

use core::ffi::{c_char, c_void, CStr};
use core::mem::{offset_of, size_of};
use core::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use cjson_ffi::exports::*;
use cjson_ffi::{cJSON, cJSON_Hooks, reallocate_hook_installed};

static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

fn c(s: &'static [u8]) -> *const c_char {
    assert_eq!(s.last(), Some(&0), "test literal must be NUL-terminated");
    s.as_ptr().cast()
}

/// Copy a C string returned by the library and release it with `cJSON_free`.
fn take_printed(p: *mut c_char) -> String {
    assert!(!p.is_null());
    // SAFETY: the print family returns a NUL-terminated buffer from the
    // allocate hook; it is read once and then freed with the matching hook.
    unsafe {
        let s = CStr::from_ptr(p).to_str().expect("utf-8").to_owned();
        cJSON_free(p.cast());
        s
    }
}

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);

/// # Safety
///
/// Same contract as libc `malloc`.
unsafe extern "C" fn counting_malloc(size: usize) -> *mut c_void {
    ALLOCS.fetch_add(1, Ordering::SeqCst);
    // SAFETY: plain forwarding to libc malloc.
    unsafe { malloc(size) }
}

/// # Safety
///
/// Same contract as libc `free`: `p` is NULL or from `counting_malloc`.
unsafe extern "C" fn counting_free(p: *mut c_void) {
    if !p.is_null() {
        FREES.fetch_add(1, Ordering::SeqCst);
    }
    // SAFETY: `p` came from `counting_malloc` (libc malloc) or is NULL.
    unsafe { free(p) }
}

#[test]
fn layout_matches_c_offsetof() {
    // Values printed by build/ffi/layout.c compiled against cJSON.h.
    assert_eq!(size_of::<cJSON>(), 64);
    assert_eq!(offset_of!(cJSON, next), 0);
    assert_eq!(offset_of!(cJSON, prev), 8);
    assert_eq!(offset_of!(cJSON, child), 16);
    assert_eq!(offset_of!(cJSON, r#type), 24);
    assert_eq!(offset_of!(cJSON, valuestring), 32);
    assert_eq!(offset_of!(cJSON, valueint), 40);
    assert_eq!(offset_of!(cJSON, valuedouble), 48);
    assert_eq!(offset_of!(cJSON, string), 56);
    assert_eq!(size_of::<cJSON_Hooks>(), 16);
    assert_eq!(offset_of!(cJSON_Hooks, free_fn), 8);
}

#[test]
fn version_is_static_c_string() {
    let _g = lock();
    let v = cJSON_Version();
    // SAFETY: cJSON_Version returns a pointer to a static NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(v) }.to_bytes(), b"1.7.19");
    assert_eq!(v, cJSON_Version());
}

#[test]
fn init_hooks_custom_then_reset() {
    let _g = lock();
    let custom = cJSON_Hooks {
        malloc_fn: Some(counting_malloc),
        free_fn: Some(counting_free),
    };
    // SAFETY: valid hooks struct; both functions have malloc/free semantics.
    unsafe { cJSON_InitHooks(&custom as *const cJSON_Hooks as *mut cJSON_Hooks) };
    assert!(!reallocate_hook_installed());

    ALLOCS.store(0, Ordering::SeqCst);
    FREES.store(0, Ordering::SeqCst);
    // SAFETY: literals are NUL-terminated; the tree is deleted once.
    unsafe {
        let root = cJSON_Parse(c(b"{\"key\":[\"v\",1,null]}\0"));
        assert!(!root.is_null());
        take_printed(cJSON_PrintUnformatted(root));
        cJSON_Delete(root);
    }
    assert!(ALLOCS.load(Ordering::SeqCst) > 0);
    assert_eq!(ALLOCS.load(Ordering::SeqCst), FREES.load(Ordering::SeqCst));

    // SAFETY: NULL resets to malloc/free/realloc (cJSON.c:217-223).
    unsafe { cJSON_InitHooks(ptr::null_mut()) };
    assert!(reallocate_hook_installed());
    let before = ALLOCS.load(Ordering::SeqCst);
    let node = cJSON_CreateNull();
    // SAFETY: node from cJSON_CreateNull, deleted once.
    unsafe { cJSON_Delete(node) };
    assert_eq!(
        ALLOCS.load(Ordering::SeqCst),
        before,
        "reset hooks must not call the custom malloc"
    );

    // Explicit libc pair keeps realloc; a custom malloc with default free drops it.
    let libc_pair = cJSON_Hooks {
        malloc_fn: Some(malloc),
        free_fn: Some(free),
    };
    // SAFETY: valid hooks struct.
    unsafe { cJSON_InitHooks(&libc_pair as *const cJSON_Hooks as *mut cJSON_Hooks) };
    assert!(reallocate_hook_installed());
    let half = cJSON_Hooks {
        malloc_fn: Some(counting_malloc),
        free_fn: None,
    };
    // SAFETY: valid hooks struct; counting_malloc returns libc memory, so
    // the default free is the right deallocator.
    unsafe { cJSON_InitHooks(&half as *const cJSON_Hooks as *mut cJSON_Hooks) };
    assert!(!reallocate_hook_installed());
    // SAFETY: reset.
    unsafe { cJSON_InitHooks(ptr::null_mut()) };
}

#[test]
fn parse_print_round_trip() {
    let _g = lock();
    let input = b"{\"a\":[1,2.5,{\"b\":null}],\"c\":\"x\\ny\",\"d\":true}\0";
    // SAFETY: NUL-terminated literal; the tree is deleted once.
    unsafe {
        let root = cJSON_Parse(c(input));
        assert!(!root.is_null());
        assert!(cJSON_GetErrorPtr().is_null());
        assert_eq!(
            take_printed(cJSON_PrintUnformatted(root)).as_bytes(),
            &input[..input.len() - 1]
        );
        assert_eq!(
            take_printed(cJSON_Print(root)),
            "{\n\t\"a\":\t[1, 2.5, {\n\t\t\t\"b\":\tnull\n\t\t}],\n\t\"c\":\t\"x\\ny\",\n\t\"d\":\ttrue\n}"
        );
        assert_eq!(
            take_printed(cJSON_PrintBuffered(root, 1, 0)).as_bytes(),
            &input[..input.len() - 1]
        );

        // Walk the tree through the struct fields, as cJSON_ArrayForEach does.
        let a = cJSON_GetObjectItem(root, c(b"A\0"));
        assert!(!a.is_null());
        let mut n = 0;
        let mut el = (*a).child;
        while !el.is_null() {
            n += 1;
            el = (*el).next;
        }
        assert_eq!(n, cJSON_GetArraySize(a));
        assert_eq!((*(*a).child).valueint, 1);
        cJSON_Delete(root);
    }
}

#[test]
fn parse_errors_match_c() {
    let _g = lock();
    let bad = c(b"[1,}\0");
    let trail = c(b"{\"a\":1} x\0");
    let mut end: *const c_char = ptr::null();
    // SAFETY: NUL-terminated literals, valid out-pointer; trees deleted once.
    unsafe {
        assert!(cJSON_Parse(bad).is_null());
        assert_eq!(cJSON_GetErrorPtr(), bad.add(3));

        assert!(cJSON_ParseWithOpts(trail, &mut end, 1).is_null());
        assert_eq!(end, trail.add(8));
        assert_eq!(cJSON_GetErrorPtr(), trail.add(8));

        let root = cJSON_ParseWithOpts(trail, &mut end, 0);
        assert!(!root.is_null());
        assert_eq!(end, trail.add(7));
        assert!(cJSON_GetErrorPtr().is_null());

        let mut small = [0x55 as c_char; 5];
        assert_eq!(cJSON_PrintPreallocated(root, small.as_mut_ptr(), 5, 0), 0);
        let mut big = [0 as c_char; 64];
        assert_eq!(cJSON_PrintPreallocated(root, big.as_mut_ptr(), 64, 0), 1);
        assert_eq!(CStr::from_ptr(big.as_ptr()).to_bytes(), b"{\"a\":1}");
        assert_eq!(cJSON_PrintPreallocated(root, big.as_mut_ptr(), -1, 0), 0);
        cJSON_Delete(root);

        // Zero length: error recorded at the input start (not NULL).
        assert!(cJSON_ParseWithLength(c(b"[1] \0"), 0).is_null());
        assert!(!cJSON_GetErrorPtr().is_null());
        // NULL input: returns NULL and leaves the reset error state.
        assert!(cJSON_ParseWithLengthOpts(ptr::null(), 4, &mut end, 0).is_null());
        assert!(cJSON_GetErrorPtr().is_null());
    }
}

#[test]
fn minify_in_place() {
    let _g = lock();
    let mut buf = *b" { \"a\" : [ 1 , 2 ] , /* c */ \"b\" : \"x y\" } // tail\n\0";
    // SAFETY: writable NUL-terminated buffer.
    unsafe {
        cJSON_Minify(buf.as_mut_ptr().cast());
        assert_eq!(
            CStr::from_ptr(buf.as_ptr().cast()).to_bytes(),
            b"{\"a\":[1,2],\"b\":\"x y\"}"
        );
        cJSON_Minify(ptr::null_mut());
    }
}

#[test]
fn set_valuestring_paths() {
    let _g = lock();
    // SAFETY: node from cJSON_CreateString, literals NUL-terminated, deleted once.
    unsafe {
        let s = cJSON_CreateString(c(b"hello\0"));
        let original = (*s).valuestring;
        // Overlapping source is rejected (C returns NULL).
        assert!(cJSON_SetValuestring(s, original.add(1)).is_null());
        // Shorter: copied in place, same buffer returned.
        assert_eq!(cJSON_SetValuestring(s, c(b"hi\0")), original);
        assert_eq!(CStr::from_ptr(cJSON_GetStringValue(s)).to_bytes(), b"hi");
        // Longer: reallocated.
        let longer = cJSON_SetValuestring(s, c(b"a much longer value\0"));
        assert!(!longer.is_null());
        assert_eq!(longer, (*s).valuestring);
        assert_eq!(CStr::from_ptr(longer).to_bytes(), b"a much longer value");
        assert!(cJSON_SetValuestring(s, ptr::null()).is_null());
        cJSON_Delete(s);
    }
}

#[test]
fn add_item_with_its_own_key_is_copied_before_free() {
    let _g = lock();
    // SAFETY: nodes created here; `item->string` is a valid key while the
    // call copies it; every tree is deleted once.
    unsafe {
        let first = cJSON_CreateObject();
        let second = cJSON_CreateObject();
        let item = cJSON_AddNumberToObject(first, c(b"key\0"), 42.0);
        assert!(!item.is_null());
        assert_eq!(cJSON_DetachItemViaPointer(first, item), item);
        assert_eq!(cJSON_AddItemToObject(second, (*item).string, item), 1);
        assert_eq!(cJSON_GetObjectItemCaseSensitive(second, c(b"key\0")), item);
        assert_eq!(take_printed(cJSON_PrintUnformatted(second)), "{\"key\":42}");
        cJSON_Delete(first);
        cJSON_Delete(second);
    }
}

#[test]
fn create_arrays_and_references() {
    let _g = lock();
    let ints = [1, -2, 3];
    let strings = [c(b"a\0"), c(b"b\0")];
    // SAFETY: arrays live for the calls; the reference target outlives the
    // reference node; every tree is deleted once.
    unsafe {
        let a = cJSON_CreateIntArray(ints.as_ptr(), 3);
        assert_eq!(take_printed(cJSON_PrintUnformatted(a)), "[1,-2,3]");
        assert!(cJSON_CreateIntArray(ptr::null(), 3).is_null());
        assert!(cJSON_CreateIntArray(ints.as_ptr(), -1).is_null());
        let s = cJSON_CreateStringArray(strings.as_ptr(), 2);
        assert_eq!(take_printed(cJSON_PrintUnformatted(s)), "[\"a\",\"b\"]");

        let key = c(b"const\0");
        let obj = cJSON_CreateObject();
        assert_eq!(
            cJSON_AddItemToObjectCS(obj, key, cJSON_CreateStringReference(c(b"ref\0"))),
            1
        );
        assert_eq!((*(*obj).child).string.cast_const(), key);
        assert_eq!(
            take_printed(cJSON_PrintUnformatted(obj)),
            "{\"const\":\"ref\"}"
        );

        let dup = cJSON_Duplicate(a, 1);
        assert_eq!(cJSON_Compare(a, dup, 1), 1);
        cJSON_Delete(dup);
        cJSON_Delete(obj);
        cJSON_Delete(s);
        cJSON_Delete(a);
    }
}
