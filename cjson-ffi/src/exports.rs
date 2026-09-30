//! Every `CJSON_PUBLIC` function of `cJSON.h`, in header order.
//!
//! Each body only translates arguments (NULL -> `None`, C string -> bytes,
//! `(ptr, count)` -> slice), calls the matching `cjson_core` function, and
//! translates the result back. Functions that take pointers are
//! `unsafe extern "C"`; their `# Safety` sections refer to the
//! [crate-level pointer contract](crate#pointer-contract).

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use cjson_core::parse::{self, ParseFailure, ParseSuccess};
use cjson_core::tree::{self, SetValuestringPlan};
use cjson_core::{compare, duplicate, minify, print, NodeStore};

use crate::globals;
use crate::store::{alloc_c_bytes, c_array, c_str, CBorrowed, CNode, CStore};
use crate::types::{cJSON, cJSON_Hooks};

/// `cJSON_Version()` result: `consts::VERSION_STRING` plus the terminator.
static VERSION: [u8; 7] = *b"1.7.19\0";

const _: () = {
    let v = cjson_core::VERSION_STRING.as_bytes();
    assert!(v.len() + 1 == VERSION.len());
    let mut i = 0;
    while i < v.len() {
        assert!(v[i] == VERSION[i]);
        i += 1;
    }
    assert!(VERSION[v.len()] == 0);
};

fn out(node: Option<CNode>) -> *mut cJSON {
    node.map_or(ptr::null_mut(), CNode::as_ptr)
}

fn c_bool(b: bool) -> c_int {
    c_int::from(b)
}

/// Copy printed bytes into a NUL-terminated buffer from the allocate hook;
/// NULL where C returns NULL (print failed or allocation failed).
fn printed(bytes: Option<Vec<u8>>) -> *mut c_char {
    bytes
        .and_then(|b| alloc_c_bytes(&b))
        .map_or(ptr::null_mut(), ptr::NonNull::as_ptr)
}

/// Shared tail of the parse family (cJSON.c:1194-1230).
///
/// # Safety
///
/// `return_parse_end` is NULL or valid for a pointer-sized write; `value`
/// is the caller's input buffer (only offset, never dereferenced here).
unsafe fn finish_parse(
    value: *const c_char,
    result: Result<ParseSuccess<CNode>, ParseFailure>,
    return_parse_end: *mut *const c_char,
) -> *mut cJSON {
    let (root, end) = match result {
        Ok(ok) => (ok.root.as_ptr(), ok.end),
        Err(fail) => {
            globals::set_error(value, fail.position);
            (ptr::null_mut(), fail.position)
        }
    };
    if !return_parse_end.is_null() {
        // SAFETY: non-NULL and writable per this function's contract. `end`
        // is an offset inside the caller's buffer, so the stored pointer is
        // the one C computes; `wrapping_add` itself never dereferences.
        unsafe { return_parse_end.write(value.wrapping_add(end)) };
    }
    root
}

// ------------------------------------------------------------ version, hooks

/// `const char* cJSON_Version(void)` (cJSON.h:147).
#[no_mangle]
pub extern "C" fn cJSON_Version() -> *const c_char {
    VERSION.as_ptr().cast()
}

/// `void cJSON_InitHooks(cJSON_Hooks* hooks)` (cJSON.h:150).
///
/// # Safety
///
/// `hooks` is NULL or points to a readable `cJSON_Hooks` whose non-NULL
/// members have `malloc` / `free` semantics.
#[no_mangle]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    // SAFETY: NULL or a valid, aligned `cJSON_Hooks` (contract above).
    let hooks = unsafe { hooks.as_ref() }.copied();
    globals::init_hooks(hooks);
}

// ------------------------------------------------------------ parse

/// `cJSON *cJSON_Parse(const char *value)` (cJSON.h:154).
///
/// # Safety
///
/// `value` is NULL or a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    // SAFETY: forwarded contract; NULL return-end pointer.
    unsafe { cJSON_ParseWithOpts(value, ptr::null_mut(), 0) }
}

/// `cJSON *cJSON_ParseWithLength(const char *value, size_t buffer_length)` (cJSON.h:155).
///
/// # Safety
///
/// `value` is NULL or points to `buffer_length` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn cJSON_ParseWithLength(
    value: *const c_char,
    buffer_length: usize,
) -> *mut cJSON {
    // SAFETY: forwarded contract; NULL return-end pointer.
    unsafe { cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0) }
}

/// `cJSON *cJSON_ParseWithOpts(const char *value, const char **return_parse_end, cJSON_bool require_null_terminated)` (cJSON.h:158).
///
/// # Safety
///
/// `value` is NULL or a NUL-terminated string; `return_parse_end` is NULL
/// or writable.
#[no_mangle]
pub unsafe extern "C" fn cJSON_ParseWithOpts(
    value: *const c_char,
    return_parse_end: *mut *const c_char,
    require_null_terminated: c_int,
) -> *mut cJSON {
    // SAFETY: `value` is NULL or NUL-terminated and not modified during the call.
    let Some(input) = (unsafe { c_str(value) }) else {
        return ptr::null_mut();
    };
    globals::reset_error();
    let result = parse::parse_with_opts(&mut CStore::new(), input, require_null_terminated != 0);
    // SAFETY: `return_parse_end` contract forwarded.
    unsafe { finish_parse(value, result, return_parse_end) }
}

/// `cJSON *cJSON_ParseWithLengthOpts(const char *value, size_t buffer_length, const char **return_parse_end, cJSON_bool require_null_terminated)` (cJSON.h:159).
///
/// # Safety
///
/// `value` is NULL or points to `buffer_length` readable bytes (at most
/// `isize::MAX`); `return_parse_end` is NULL or writable.
#[no_mangle]
pub unsafe extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: usize,
    return_parse_end: *mut *const c_char,
    require_null_terminated: c_int,
) -> *mut cJSON {
    globals::reset_error();
    if value.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: non-NULL and `buffer_length` readable bytes (contract above),
    // not modified during the call.
    let input = unsafe { core::slice::from_raw_parts(value.cast::<u8>(), buffer_length) };
    let result =
        parse::parse_with_length_opts(&mut CStore::new(), input, require_null_terminated != 0);
    // SAFETY: `return_parse_end` contract forwarded.
    unsafe { finish_parse(value, result, return_parse_end) }
}

// ------------------------------------------------------------ print

/// `char *cJSON_Print(const cJSON *item)` (cJSON.h:162).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    printed(print::print(&CStore::new(), item, true))
}

/// `char *cJSON_PrintUnformatted(const cJSON *item)` (cJSON.h:164).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    printed(print::print(&CStore::new(), item, false))
}

/// `char *cJSON_PrintBuffered(const cJSON *item, int prebuffer, cJSON_bool fmt)` (cJSON.h:166).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_PrintBuffered(
    item: *const cJSON,
    prebuffer: c_int,
    fmt: c_int,
) -> *mut c_char {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    printed(print::print_buffered(
        &CStore::new(),
        item,
        prebuffer,
        fmt != 0,
    ))
}

/// `cJSON_bool cJSON_PrintPreallocated(cJSON *item, char *buffer, const int length, const cJSON_bool format)` (cJSON.h:169).
///
/// # Safety
///
/// `item` follows the node contract; `buffer` is NULL or valid for
/// `length` byte writes and does not overlap the tree's strings.
#[no_mangle]
pub unsafe extern "C" fn cJSON_PrintPreallocated(
    item: *mut cJSON,
    buffer: *mut c_char,
    length: c_int,
    format: c_int,
) -> c_int {
    let Ok(length) = usize::try_from(length) else {
        return 0;
    };
    if buffer.is_null() {
        return 0;
    }
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    let outcome = print::print_preallocated(&CStore::new(), item, length, format != 0);
    let n = outcome.written.len().min(length);
    // SAFETY: `buffer` is valid for `length` writes and `n <= length`;
    // `written` is a Rust-owned Vec, so the regions cannot overlap.
    unsafe { ptr::copy_nonoverlapping(outcome.written.as_ptr(), buffer.cast::<u8>(), n) };
    c_bool(outcome.ok)
}

/// `void cJSON_Delete(cJSON *item)` (cJSON.h:171).
///
/// # Safety
///
/// `item` follows the node contract and is not used after the call.
#[no_mangle]
pub unsafe extern "C" fn cJSON_Delete(item: *mut cJSON) {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    tree::delete(&mut CStore::new(), item);
}

// ------------------------------------------------------------ getters

/// `int cJSON_GetArraySize(const cJSON *array)` (cJSON.h:174).
///
/// # Safety
///
/// `array` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let array = unsafe { CNode::from_raw(array) };
    tree::get_array_size(&CStore::new(), array)
}

/// `cJSON *cJSON_GetArrayItem(const cJSON *array, int index)` (cJSON.h:176).
///
/// # Safety
///
/// `array` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    // SAFETY: node contract.
    let array = unsafe { CNode::from_raw(array) };
    out(tree::get_array_item(&CStore::new(), array, index))
}

/// `cJSON *cJSON_GetObjectItem(const cJSON * const object, const char * const string)` (cJSON.h:178).
///
/// # Safety
///
/// `object` follows the node contract; `string` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_GetObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, string) = unsafe { (CNode::from_raw(object), c_str(string)) };
    out(tree::get_object_item(&CStore::new(), object, string, false))
}

/// `cJSON *cJSON_GetObjectItemCaseSensitive(const cJSON * const object, const char * const string)` (cJSON.h:179).
///
/// # Safety
///
/// `object` follows the node contract; `string` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, string) = unsafe { (CNode::from_raw(object), c_str(string)) };
    out(tree::get_object_item(&CStore::new(), object, string, true))
}

/// `cJSON_bool cJSON_HasObjectItem(const cJSON *object, const char *string)` (cJSON.h:180).
///
/// # Safety
///
/// `object` follows the node contract; `string` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_HasObjectItem(object: *const cJSON, string: *const c_char) -> c_int {
    // SAFETY: node and string contracts.
    let (object, string) = unsafe { (CNode::from_raw(object), c_str(string)) };
    c_bool(tree::has_object_item(&CStore::new(), object, string))
}

/// `const char *cJSON_GetErrorPtr(void)` (cJSON.h:182).
#[no_mangle]
pub extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    globals::error_ptr()
}

/// `char *cJSON_GetStringValue(const cJSON * const item)` (cJSON.h:185).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    let store = CStore::new();
    tree::get_string_value_node(&store, item).map_or(ptr::null_mut(), |n| store.raw_valuestring(n))
}

/// `double cJSON_GetNumberValue(const cJSON * const item)` (cJSON.h:186).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> f64 {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    tree::get_number_value(&CStore::new(), item)
}

// ------------------------------------------------------------ type checks

/// `cJSON_bool cJSON_IsInvalid(const cJSON * const item)` (cJSON.h:189).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_invalid(&CStore::new(), item))
}

/// `cJSON_bool cJSON_IsFalse(const cJSON * const item)` (cJSON.h:190).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsFalse(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_false(&CStore::new(), item))
}

/// `cJSON_bool cJSON_IsTrue(const cJSON * const item)` (cJSON.h:191).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsTrue(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_true(&CStore::new(), item))
}

/// `cJSON_bool cJSON_IsBool(const cJSON * const item)` (cJSON.h:192).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsBool(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_bool(&CStore::new(), item))
}

/// `cJSON_bool cJSON_IsNull(const cJSON * const item)` (cJSON.h:193).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsNull(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_null(&CStore::new(), item))
}

/// `cJSON_bool cJSON_IsNumber(const cJSON * const item)` (cJSON.h:194).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsNumber(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_number(&CStore::new(), item))
}

/// `cJSON_bool cJSON_IsString(const cJSON * const item)` (cJSON.h:195).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsString(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_string(&CStore::new(), item))
}

/// `cJSON_bool cJSON_IsArray(const cJSON * const item)` (cJSON.h:196).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsArray(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_array(&CStore::new(), item))
}

/// `cJSON_bool cJSON_IsObject(const cJSON * const item)` (cJSON.h:197).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsObject(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_object(&CStore::new(), item))
}

/// `cJSON_bool cJSON_IsRaw(const cJSON * const item)` (cJSON.h:198).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsRaw(item: *const cJSON) -> c_int {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    c_bool(tree::is_raw(&CStore::new(), item))
}

// ------------------------------------------------------------ create

/// `cJSON *cJSON_CreateNull(void)` (cJSON.h:201).
#[no_mangle]
pub extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    out(tree::create_null(&mut CStore::new()))
}

/// `cJSON *cJSON_CreateTrue(void)` (cJSON.h:202).
#[no_mangle]
pub extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    out(tree::create_true(&mut CStore::new()))
}

/// `cJSON *cJSON_CreateFalse(void)` (cJSON.h:203).
#[no_mangle]
pub extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    out(tree::create_false(&mut CStore::new()))
}

/// `cJSON *cJSON_CreateBool(cJSON_bool boolean)` (cJSON.h:204).
#[no_mangle]
pub extern "C" fn cJSON_CreateBool(boolean: c_int) -> *mut cJSON {
    out(tree::create_bool(&mut CStore::new(), boolean != 0))
}

/// `cJSON *cJSON_CreateNumber(double num)` (cJSON.h:205).
#[no_mangle]
pub extern "C" fn cJSON_CreateNumber(num: f64) -> *mut cJSON {
    out(tree::create_number(&mut CStore::new(), num))
}

/// `cJSON *cJSON_CreateString(const char *string)` (cJSON.h:206).
///
/// # Safety
///
/// `string` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    // SAFETY: string contract.
    let string = unsafe { c_str(string) };
    out(tree::create_string(&mut CStore::new(), string))
}

/// `cJSON *cJSON_CreateRaw(const char *raw)` (cJSON.h:208).
///
/// # Safety
///
/// `raw` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    // SAFETY: string contract.
    let raw = unsafe { c_str(raw) };
    out(tree::create_raw(&mut CStore::new(), raw))
}

/// `cJSON *cJSON_CreateArray(void)` (cJSON.h:209).
#[no_mangle]
pub extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    out(tree::create_array(&mut CStore::new()))
}

/// `cJSON *cJSON_CreateObject(void)` (cJSON.h:210).
#[no_mangle]
pub extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    out(tree::create_object(&mut CStore::new()))
}

/// `cJSON *cJSON_CreateStringReference(const char *string)` (cJSON.h:214).
///
/// # Safety
///
/// `string` is NULL or a NUL-terminated string that outlives the node.
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    // SAFETY: borrowed-string contract.
    let string = unsafe { CBorrowed::from_raw(string) };
    out(tree::create_string_reference(&mut CStore::new(), string))
}

/// `cJSON *cJSON_CreateObjectReference(const cJSON *child)` (cJSON.h:217).
///
/// # Safety
///
/// `child` follows the node contract and outlives the reference node.
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    // SAFETY: node contract.
    let child = unsafe { CNode::from_raw(child) };
    out(tree::create_object_reference(&mut CStore::new(), child))
}

/// `cJSON *cJSON_CreateArrayReference(const cJSON *child)` (cJSON.h:218).
///
/// # Safety
///
/// `child` follows the node contract and outlives the reference node.
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    // SAFETY: node contract.
    let child = unsafe { CNode::from_raw(child) };
    out(tree::create_array_reference(&mut CStore::new(), child))
}

/// `cJSON *cJSON_CreateIntArray(const int *numbers, int count)` (cJSON.h:222).
///
/// # Safety
///
/// `numbers` is NULL or points to `count` readable `int`s.
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    // SAFETY: array contract.
    let numbers = unsafe { c_array(numbers, count) };
    out(tree::create_int_array(&mut CStore::new(), numbers))
}

/// `cJSON *cJSON_CreateFloatArray(const float *numbers, int count)` (cJSON.h:223).
///
/// # Safety
///
/// `numbers` is NULL or points to `count` readable `float`s.
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateFloatArray(numbers: *const f32, count: c_int) -> *mut cJSON {
    // SAFETY: array contract.
    let numbers = unsafe { c_array(numbers, count) };
    out(tree::create_float_array(&mut CStore::new(), numbers))
}

/// `cJSON *cJSON_CreateDoubleArray(const double *numbers, int count)` (cJSON.h:224).
///
/// # Safety
///
/// `numbers` is NULL or points to `count` readable `double`s.
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateDoubleArray(numbers: *const f64, count: c_int) -> *mut cJSON {
    // SAFETY: array contract.
    let numbers = unsafe { c_array(numbers, count) };
    out(tree::create_double_array(&mut CStore::new(), numbers))
}

/// `cJSON *cJSON_CreateStringArray(const char *const *strings, int count)` (cJSON.h:225).
///
/// # Safety
///
/// `strings` is NULL or points to `count` readable pointers, each NULL or
/// NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateStringArray(
    strings: *const *const c_char,
    count: c_int,
) -> *mut cJSON {
    // SAFETY: array contract.
    let pointers = unsafe { c_array(strings, count) };
    let strings: Option<Vec<Option<&[u8]>>> = pointers.map(|ps| {
        ps.iter()
            // SAFETY: each element is NULL or NUL-terminated (contract above).
            .map(|&p| unsafe { c_str(p) })
            .collect()
    });
    out(tree::create_string_array(
        &mut CStore::new(),
        strings.as_deref(),
    ))
}

// ------------------------------------------------------------ add

/// `cJSON_bool cJSON_AddItemToArray(cJSON *array, cJSON *item)` (cJSON.h:228).
///
/// # Safety
///
/// `array` and `item` follow the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> c_int {
    // SAFETY: node contract.
    let (array, item) = unsafe { (CNode::from_raw(array), CNode::from_raw(item)) };
    c_bool(tree::add_item_to_array(&mut CStore::new(), array, item))
}

/// `cJSON_bool cJSON_AddItemToObject(cJSON *object, const char *string, cJSON *item)` (cJSON.h:229).
///
/// # Safety
///
/// `object` and `item` follow the node contract; `string` is NULL or
/// NUL-terminated (it may be `item->string`).
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> c_int {
    // SAFETY: node and string contracts.
    let (object, string, item) = unsafe {
        (
            CNode::from_raw(object),
            c_str(string),
            CNode::from_raw(item),
        )
    };
    c_bool(tree::add_item_to_object(
        &mut CStore::new(),
        object,
        string,
        item,
    ))
}

/// `cJSON_bool cJSON_AddItemToObjectCS(cJSON *object, const char *string, cJSON *item)` (cJSON.h:233).
///
/// # Safety
///
/// `object` and `item` follow the node contract; `string` is NULL or a
/// NUL-terminated string that outlives `item`.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> c_int {
    // SAFETY: node and borrowed-string contracts.
    let (object, string, item) = unsafe {
        (
            CNode::from_raw(object),
            CBorrowed::from_raw(string),
            CNode::from_raw(item),
        )
    };
    c_bool(tree::add_item_to_object_cs(
        &mut CStore::new(),
        object,
        string,
        item,
    ))
}

/// `cJSON_bool cJSON_AddItemReferenceToArray(cJSON *array, cJSON *item)` (cJSON.h:235).
///
/// # Safety
///
/// `array` and `item` follow the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(
    array: *mut cJSON,
    item: *mut cJSON,
) -> c_int {
    // SAFETY: node contract.
    let (array, item) = unsafe { (CNode::from_raw(array), CNode::from_raw(item)) };
    c_bool(tree::add_item_reference_to_array(
        &mut CStore::new(),
        array,
        item,
    ))
}

/// `cJSON_bool cJSON_AddItemReferenceToObject(cJSON *object, const char *string, cJSON *item)` (cJSON.h:236).
///
/// # Safety
///
/// `object` and `item` follow the node contract; `string` is NULL or
/// NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> c_int {
    // SAFETY: node and string contracts.
    let (object, string, item) = unsafe {
        (
            CNode::from_raw(object),
            c_str(string),
            CNode::from_raw(item),
        )
    };
    c_bool(tree::add_item_reference_to_object(
        &mut CStore::new(),
        object,
        string,
        item,
    ))
}

// ------------------------------------------------------------ detach / delete

/// `cJSON *cJSON_DetachItemViaPointer(cJSON *parent, cJSON * const item)` (cJSON.h:239).
///
/// # Safety
///
/// `parent` and `item` follow the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_DetachItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
) -> *mut cJSON {
    // SAFETY: node contract.
    let (parent, item) = unsafe { (CNode::from_raw(parent), CNode::from_raw(item)) };
    out(tree::detach_item_via_pointer(
        &mut CStore::new(),
        parent,
        item,
    ))
}

/// `cJSON *cJSON_DetachItemFromArray(cJSON *array, int which)` (cJSON.h:240).
///
/// # Safety
///
/// `array` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON {
    // SAFETY: node contract.
    let array = unsafe { CNode::from_raw(array) };
    out(tree::detach_item_from_array(
        &mut CStore::new(),
        array,
        which,
    ))
}

/// `void cJSON_DeleteItemFromArray(cJSON *array, int which)` (cJSON.h:241).
///
/// # Safety
///
/// `array` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    // SAFETY: node contract.
    let array = unsafe { CNode::from_raw(array) };
    tree::delete_item_from_array(&mut CStore::new(), array, which);
}

/// `cJSON *cJSON_DetachItemFromObject(cJSON *object, const char *string)` (cJSON.h:242).
///
/// # Safety
///
/// `object` follows the node contract; `string` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_DetachItemFromObject(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, string) = unsafe { (CNode::from_raw(object), c_str(string)) };
    out(tree::detach_item_from_object(
        &mut CStore::new(),
        object,
        string,
        false,
    ))
}

/// `cJSON *cJSON_DetachItemFromObjectCaseSensitive(cJSON *object, const char *string)` (cJSON.h:243).
///
/// # Safety
///
/// `object` follows the node contract; `string` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, string) = unsafe { (CNode::from_raw(object), c_str(string)) };
    out(tree::detach_item_from_object(
        &mut CStore::new(),
        object,
        string,
        true,
    ))
}

/// `void cJSON_DeleteItemFromObject(cJSON *object, const char *string)` (cJSON.h:244).
///
/// # Safety
///
/// `object` follows the node contract; `string` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    // SAFETY: node and string contracts.
    let (object, string) = unsafe { (CNode::from_raw(object), c_str(string)) };
    tree::delete_item_from_object(&mut CStore::new(), object, string, false);
}

/// `void cJSON_DeleteItemFromObjectCaseSensitive(cJSON *object, const char *string)` (cJSON.h:245).
///
/// # Safety
///
/// `object` follows the node contract; `string` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) {
    // SAFETY: node and string contracts.
    let (object, string) = unsafe { (CNode::from_raw(object), c_str(string)) };
    tree::delete_item_from_object(&mut CStore::new(), object, string, true);
}

// ------------------------------------------------------------ insert / replace

/// `cJSON_bool cJSON_InsertItemInArray(cJSON *array, int which, cJSON *newitem)` (cJSON.h:248).
///
/// # Safety
///
/// `array` and `newitem` follow the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_InsertItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> c_int {
    // SAFETY: node contract.
    let (array, newitem) = unsafe { (CNode::from_raw(array), CNode::from_raw(newitem)) };
    c_bool(tree::insert_item_in_array(
        &mut CStore::new(),
        array,
        which,
        newitem,
    ))
}

/// `cJSON_bool cJSON_ReplaceItemViaPointer(cJSON * const parent, cJSON * const item, cJSON * replacement)` (cJSON.h:249).
///
/// # Safety
///
/// `parent`, `item` and `replacement` follow the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
    replacement: *mut cJSON,
) -> c_int {
    // SAFETY: node contract.
    let nodes = unsafe {
        (
            CNode::from_raw(parent),
            CNode::from_raw(item),
            CNode::from_raw(replacement),
        )
    };
    let (parent, item, replacement) = nodes;
    c_bool(tree::replace_item_via_pointer(
        &mut CStore::new(),
        parent,
        item,
        replacement,
    ))
}

/// `cJSON_bool cJSON_ReplaceItemInArray(cJSON *array, int which, cJSON *newitem)` (cJSON.h:250).
///
/// # Safety
///
/// `array` and `newitem` follow the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> c_int {
    // SAFETY: node contract.
    let (array, newitem) = unsafe { (CNode::from_raw(array), CNode::from_raw(newitem)) };
    c_bool(tree::replace_item_in_array(
        &mut CStore::new(),
        array,
        which,
        newitem,
    ))
}

/// `cJSON_bool cJSON_ReplaceItemInObject(cJSON *object,const char *string,cJSON *newitem)` (cJSON.h:251).
///
/// # Safety
///
/// `object` and `newitem` follow the node contract; `string` is NULL or
/// NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> c_int {
    // SAFETY: node and string contracts.
    let (object, string, newitem) = unsafe {
        (
            CNode::from_raw(object),
            c_str(string),
            CNode::from_raw(newitem),
        )
    };
    c_bool(tree::replace_item_in_object(
        &mut CStore::new(),
        object,
        string,
        newitem,
        false,
    ))
}

/// `cJSON_bool cJSON_ReplaceItemInObjectCaseSensitive(cJSON *object,const char *string,cJSON *newitem)` (cJSON.h:252).
///
/// # Safety
///
/// `object` and `newitem` follow the node contract; `string` is NULL or
/// NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> c_int {
    // SAFETY: node and string contracts.
    let (object, string, newitem) = unsafe {
        (
            CNode::from_raw(object),
            c_str(string),
            CNode::from_raw(newitem),
        )
    };
    c_bool(tree::replace_item_in_object(
        &mut CStore::new(),
        object,
        string,
        newitem,
        true,
    ))
}

// ------------------------------------------------------------ duplicate, compare, minify

/// `cJSON *cJSON_Duplicate(const cJSON *item, cJSON_bool recurse)` (cJSON.h:255).
///
/// # Safety
///
/// `item` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: c_int) -> *mut cJSON {
    // SAFETY: node contract.
    let item = unsafe { CNode::from_raw(item) };
    out(duplicate::duplicate(&mut CStore::new(), item, recurse != 0))
}

/// `cJSON_bool cJSON_Compare(const cJSON * const a, const cJSON * const b, const cJSON_bool case_sensitive)` (cJSON.h:261).
///
/// # Safety
///
/// `a` and `b` follow the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_Compare(
    a: *const cJSON,
    b: *const cJSON,
    case_sensitive: c_int,
) -> c_int {
    // SAFETY: node contract.
    let (a, b) = unsafe { (CNode::from_raw(a), CNode::from_raw(b)) };
    c_bool(compare::compare(&CStore::new(), a, b, case_sensitive != 0))
}

/// `void cJSON_Minify(char *json)` (cJSON.h:266).
///
/// # Safety
///
/// `json` is NULL or a writable NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn cJSON_Minify(json: *mut c_char) {
    // SAFETY: NULL or NUL-terminated; the slice is dropped before the write.
    let Some(input) = (unsafe { c_str(json) }) else {
        return;
    };
    let len = input.len();
    let minified = minify::minify(input);
    let n = minified.len().min(len);
    // SAFETY: `json` is writable for `len + 1` bytes and `n <= len`, so both
    // the copy and the terminator stay inside the caller's string; the
    // source is a Rust-owned Vec, so the regions cannot overlap. `input` is
    // not used after this point.
    unsafe {
        ptr::copy_nonoverlapping(minified.as_ptr(), json.cast::<u8>(), n);
        json.add(n).write(0);
    }
}

// ------------------------------------------------------------ add helpers

/// `cJSON* cJSON_AddNullToObject(cJSON * const object, const char * const name)` (cJSON.h:270).
///
/// # Safety
///
/// `object` follows the node contract; `name` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddNullToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, name) = unsafe { (CNode::from_raw(object), c_str(name)) };
    out(tree::add_null_to_object(&mut CStore::new(), object, name))
}

/// `cJSON* cJSON_AddTrueToObject(cJSON * const object, const char * const name)` (cJSON.h:271).
///
/// # Safety
///
/// `object` follows the node contract; `name` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddTrueToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, name) = unsafe { (CNode::from_raw(object), c_str(name)) };
    out(tree::add_true_to_object(&mut CStore::new(), object, name))
}

/// `cJSON* cJSON_AddFalseToObject(cJSON * const object, const char * const name)` (cJSON.h:272).
///
/// # Safety
///
/// `object` follows the node contract; `name` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddFalseToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, name) = unsafe { (CNode::from_raw(object), c_str(name)) };
    out(tree::add_false_to_object(&mut CStore::new(), object, name))
}

/// `cJSON* cJSON_AddBoolToObject(cJSON * const object, const char * const name, const cJSON_bool boolean)` (cJSON.h:273).
///
/// # Safety
///
/// `object` follows the node contract; `name` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddBoolToObject(
    object: *mut cJSON,
    name: *const c_char,
    boolean: c_int,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, name) = unsafe { (CNode::from_raw(object), c_str(name)) };
    out(tree::add_bool_to_object(
        &mut CStore::new(),
        object,
        name,
        boolean != 0,
    ))
}

/// `cJSON* cJSON_AddNumberToObject(cJSON * const object, const char * const name, const double number)` (cJSON.h:274).
///
/// # Safety
///
/// `object` follows the node contract; `name` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddNumberToObject(
    object: *mut cJSON,
    name: *const c_char,
    number: f64,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, name) = unsafe { (CNode::from_raw(object), c_str(name)) };
    out(tree::add_number_to_object(
        &mut CStore::new(),
        object,
        name,
        number,
    ))
}

/// `cJSON* cJSON_AddStringToObject(cJSON * const object, const char * const name, const char * const string)` (cJSON.h:275).
///
/// # Safety
///
/// `object` follows the node contract; `name` and `string` are NULL or
/// NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddStringToObject(
    object: *mut cJSON,
    name: *const c_char,
    string: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, name, string) = unsafe { (CNode::from_raw(object), c_str(name), c_str(string)) };
    out(tree::add_string_to_object(
        &mut CStore::new(),
        object,
        name,
        string,
    ))
}

/// `cJSON* cJSON_AddRawToObject(cJSON * const object, const char * const name, const char * const raw)` (cJSON.h:276).
///
/// # Safety
///
/// `object` follows the node contract; `name` and `raw` are NULL or
/// NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddRawToObject(
    object: *mut cJSON,
    name: *const c_char,
    raw: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, name, raw) = unsafe { (CNode::from_raw(object), c_str(name), c_str(raw)) };
    out(tree::add_raw_to_object(
        &mut CStore::new(),
        object,
        name,
        raw,
    ))
}

/// `cJSON* cJSON_AddObjectToObject(cJSON * const object, const char * const name)` (cJSON.h:277).
///
/// # Safety
///
/// `object` follows the node contract; `name` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddObjectToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, name) = unsafe { (CNode::from_raw(object), c_str(name)) };
    out(tree::add_object_to_object(&mut CStore::new(), object, name))
}

/// `cJSON* cJSON_AddArrayToObject(cJSON * const object, const char * const name)` (cJSON.h:278).
///
/// # Safety
///
/// `object` follows the node contract; `name` is NULL or NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_AddArrayToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    // SAFETY: node and string contracts.
    let (object, name) = unsafe { (CNode::from_raw(object), c_str(name)) };
    out(tree::add_array_to_object(&mut CStore::new(), object, name))
}

// ------------------------------------------------------------ setters

/// `double cJSON_SetNumberHelper(cJSON *object, double number)` (cJSON.h:283).
///
/// # Safety
///
/// `object` follows the node contract.
#[no_mangle]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: f64) -> f64 {
    // SAFETY: node contract.
    let object = unsafe { CNode::from_raw(object) };
    tree::set_number_helper(&mut CStore::new(), object, number)
}

/// `char* cJSON_SetValuestring(cJSON *object, const char *valuestring)` (cJSON.h:286).
///
/// The FFI supplies the pointer facts C computes (cJSON.c:457-463); the
/// core decides (`tree::set_valuestring_plan`); the FFI performs the write.
///
/// # Safety
///
/// `object` follows the node contract; `valuestring` is NULL or
/// NUL-terminated.
#[no_mangle]
pub unsafe extern "C" fn cJSON_SetValuestring(
    object: *mut cJSON,
    valuestring: *const c_char,
) -> *mut c_char {
    // SAFETY: node and string contracts. `new` borrows caller memory that may
    // alias the node's buffer; it is only read before any write below.
    let (object, new) = unsafe { (CNode::from_raw(object), c_str(valuestring)) };
    let mut store = CStore::new();
    let old = object.map_or(ptr::null_mut(), |o| store.raw_valuestring(o));
    let old_len = object.and_then(|o| store.valuestring(o)).map(<[u8]>::len);
    let new_len = new.map(<[u8]>::len);
    // `!(valuestring + v1_len < object->valuestring || object->valuestring + v2_len < valuestring)`
    let overlaps = match (new_len, old_len) {
        (Some(v1_len), Some(v2_len)) => {
            let old = old.cast_const();
            !(valuestring.wrapping_add(v1_len) < old || old.wrapping_add(v2_len) < valuestring)
        }
        _ => false,
    };
    match tree::set_valuestring_plan(&store, object, new_len, overlaps) {
        SetValuestringPlan::Reject => ptr::null_mut(),
        SetValuestringPlan::CopyInPlace => {
            // Memory-safety guard only: the copy must fit the old buffer.
            let (Some(v1_len), Some(v2_len)) = (new_len, old_len) else {
                return ptr::null_mut();
            };
            if v1_len > v2_len {
                return ptr::null_mut();
            }
            // SAFETY: `old` is a live NUL-terminated buffer of at least
            // `v2_len + 1 >= v1_len + 1` bytes; `valuestring` has `v1_len`
            // bytes plus its NUL. `ptr::copy` tolerates overlap, and no Rust
            // reference to either region is used after this point.
            unsafe { ptr::copy(valuestring, old, v1_len + 1) };
            old
        }
        SetValuestringPlan::Reallocate => {
            let (Some(object), Some(new)) = (object, new) else {
                return ptr::null_mut();
            };
            match store.replace_valuestring_copy(object, new, true) {
                Ok(()) => store.raw_valuestring(object),
                Err(_) => ptr::null_mut(),
            }
        }
    }
}

// ------------------------------------------------------------ allocator

/// `void *cJSON_malloc(size_t size)` (cJSON.h:299).
#[no_mangle]
pub extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    globals::allocate(size)
}

/// `void cJSON_free(void *object)` (cJSON.h:300).
///
/// # Safety
///
/// `object` is NULL or was allocated by the current allocate hook and not
/// yet freed.
#[no_mangle]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    // SAFETY: forwarded contract.
    unsafe { globals::deallocate(object) }
}
