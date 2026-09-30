//! Tree operations: accessors, type checks, create/add/detach/insert/replace/
//! delete (cJSON.c:253-282, 410-482, 1908-2830, 2972-3070).
//!
//! Public signatures in this file are a contract used by `cjson-ffi`; do not
//! change them without updating that caller.
//!
//! Conventions: `Option<S::Id>` parameters model C pointers that may be
//! NULL; `Option<&[u8]>` models `const char *` that may be NULL (bytes
//! without the terminator). Return values mirror the C return values.
//!
//! Byte-string inputs are read the way C reads a `const char *`: only the
//! bytes before the first NUL count (`strlen`, `strcmp`, `cJSON_strdup`).
//! Like C, list walks do not guard against cycles.

use crate::consts::{
    CJSON_ARRAY, CJSON_FALSE, CJSON_INVALID, CJSON_IS_REFERENCE, CJSON_NULL, CJSON_NUMBER,
    CJSON_OBJECT, CJSON_RAW, CJSON_STRING, CJSON_STRING_IS_CONST, CJSON_TRUE,
};
use crate::number::saturate_to_int;
use crate::store::NodeStore;

/// The bytes C sees through a `const char *`: everything before the first NUL.
fn c_str(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(end) => bytes.get(..end).unwrap_or(bytes),
        None => bytes,
    }
}

/// Byte at `index` of a C string, reading the implicit terminator (and
/// anything past it) as NUL.
fn byte_at(s: &[u8], index: usize) -> u8 {
    s.get(index).copied().unwrap_or(0)
}

/// `case_insensitive_strcmp` (cJSON.c:137-158): 1 when either side is NULL
/// (two NULLs are *not* equal), otherwise the difference of the first
/// `tolower`ed bytes that differ. `tolower` in the default "C" locale only
/// folds ASCII `A-Z`.
///
/// C also returns 0 early when both pointers are identical; identical
/// pointers have identical contents, so comparing contents gives the same 0.
pub fn case_insensitive_strcmp(string1: Option<&[u8]>, string2: Option<&[u8]>) -> i32 {
    let (Some(s1), Some(s2)) = (string1, string2) else {
        return 1;
    };
    let (s1, s2) = (c_str(s1), c_str(s2));
    let mut i = 0usize;
    loop {
        let a = byte_at(s1, i).to_ascii_lowercase();
        let b = byte_at(s2, i).to_ascii_lowercase();
        if a != b {
            return i32::from(a) - i32::from(b);
        }
        if a == 0 {
            return 0;
        }
        i = i.saturating_add(1);
    }
}

// ----------------------------------------------------------------- lifetime

/// `cJSON_Delete(item)`: frees `item`, its `next` chain, children, and
/// strings, honoring `cJSON_IsReference` and `cJSON_StringIsConst`.
///
/// C recurses into `child` (cJSON.c:265-268); this walks an explicit stack
/// instead so deep API-built trees cannot overflow the Rust stack. The
/// sequence of frees is identical to C: a node's whole child chain is freed
/// before its own valuestring, key, and the node itself, then its `next`
/// (read before descending, cJSON.c:264) follows.
pub fn delete<S: NodeStore>(store: &mut S, item: Option<S::Id>) {
    // Nodes whose child chain is being freed, with the `next` C saved.
    let mut pending: Vec<(S::Id, Option<S::Id>)> = Vec::new();
    let mut current = item;
    loop {
        if let Some(id) = current {
            let next = store.next(id);
            let child = store.child(id);
            if store.type_bits(id) & CJSON_IS_REFERENCE == 0 && child.is_some() {
                pending.push((id, next));
                current = child;
                continue;
            }
            delete_node_payload(store, id);
            current = next;
        } else if let Some((id, next)) = pending.pop() {
            delete_node_payload(store, id);
            current = next;
        } else {
            return;
        }
    }
}

/// The part of the `cJSON_Delete` loop body after the child recursion
/// (cJSON.c:269-279).
fn delete_node_payload<S: NodeStore>(store: &mut S, id: S::Id) {
    let type_bits = store.type_bits(id);
    if type_bits & CJSON_IS_REFERENCE == 0 && store.valuestring(id).is_some() {
        store.free_valuestring(id);
    }
    if type_bits & CJSON_STRING_IS_CONST == 0 && store.string(id).is_some() {
        store.free_string(id);
    }
    store.free_node(id);
}

// ----------------------------------------------------------------- getters

/// `cJSON_GetArraySize` (cJSON.c:1911-1932). C counts in `size_t` and
/// casts to `int` (the FIXME there), which truncates like `as i32`.
pub fn get_array_size<S: NodeStore>(store: &S, array: Option<S::Id>) -> i32 {
    let Some(array) = array else {
        return 0;
    };
    let mut size: usize = 0;
    let mut child = store.child(array);
    while let Some(c) = child {
        size = size.wrapping_add(1);
        child = store.next(c);
    }
    size as i32
}

/// Static `get_array_item` (cJSON.c:1935-1952).
fn get_array_item_at<S: NodeStore>(
    store: &S,
    array: Option<S::Id>,
    mut index: usize,
) -> Option<S::Id> {
    let array = array?;
    let mut current = store.child(array);
    while let Some(c) = current {
        if index == 0 {
            break;
        }
        index -= 1;
        current = store.next(c);
    }
    current
}

/// `cJSON_GetArrayItem` (cJSON.c:1955-1963): negative index is NULL.
pub fn get_array_item<S: NodeStore>(store: &S, array: Option<S::Id>, index: i32) -> Option<S::Id> {
    let index = usize::try_from(index).ok()?;
    get_array_item_at(store, array, index)
}

/// Static `get_object_item` (cJSON.c:1966-1996), behind
/// `cJSON_GetObjectItem` (case-insensitive) and
/// `cJSON_GetObjectItemCaseSensitive`.
///
/// Quirks kept from C:
/// * The case-sensitive walk stops at the first child whose key is NULL and
///   returns NULL, even if a later child matches (cJSON.c:1978).
/// * The case-insensitive walk skips NULL-keyed children, because
///   `case_insensitive_strcmp` reports NULL as "different" (cJSON.c:139-142).
/// * The first match wins when keys are duplicated.
pub fn get_object_item<S: NodeStore>(
    store: &S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
    case_sensitive: bool,
) -> Option<S::Id> {
    let (Some(object), Some(name)) = (object, name) else {
        return None;
    };
    let name = c_str(name);
    let mut current = store.child(object);
    if case_sensitive {
        while let Some(c) = current {
            match store.string(c) {
                Some(key) if key != name => current = store.next(c),
                _ => break,
            }
        }
    } else {
        while let Some(c) = current {
            if case_insensitive_strcmp(Some(name), store.string(c)) == 0 {
                break;
            }
            current = store.next(c);
        }
    }
    let found = current?;
    store.string(found)?;
    Some(found)
}

/// `cJSON_HasObjectItem` (case-insensitive lookup).
pub fn has_object_item<S: NodeStore>(
    store: &S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
) -> bool {
    get_object_item(store, object, name, false).is_some()
}

/// `cJSON_GetStringValue`: Some(item) iff C returns `item->valuestring`
/// (the FFI returns the raw pointer of that node; it may still be NULL).
pub fn get_string_value_node<S: NodeStore>(store: &S, item: Option<S::Id>) -> Option<S::Id> {
    if is_string(store, item) {
        item
    } else {
        None
    }
}

/// `cJSON_GetNumberValue` (NaN when not a number).
pub fn get_number_value<S: NodeStore>(store: &S, item: Option<S::Id>) -> f64 {
    match item {
        Some(id) if is_number(store, item) => store.valuedouble(id),
        _ => f64::NAN,
    }
}

// ----------------------------------------------------------------- type checks

/// Shared body of the `cJSON_Is*` checks that compare `type & 0xFF`
/// (cJSON.c:3037-3144).
fn low_type_is<S: NodeStore>(store: &S, item: Option<S::Id>, expected: i32) -> bool {
    match item {
        Some(id) => store.type_bits(id) & 0xFF == expected,
        None => false,
    }
}

pub fn is_invalid<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    low_type_is(store, item, CJSON_INVALID)
}
pub fn is_false<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    low_type_is(store, item, CJSON_FALSE)
}
pub fn is_true<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    low_type_is(store, item, CJSON_TRUE)
}
/// `cJSON_IsBool` tests `type & (cJSON_True | cJSON_False)` rather than the
/// low byte (cJSON.c:3078), so a node with both bits (or other type bits
/// alongside one of them) still counts as a bool.
pub fn is_bool<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    match item {
        Some(id) => store.type_bits(id) & (CJSON_TRUE | CJSON_FALSE) != 0,
        None => false,
    }
}
pub fn is_null<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    low_type_is(store, item, CJSON_NULL)
}
pub fn is_number<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    low_type_is(store, item, CJSON_NUMBER)
}
pub fn is_string<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    low_type_is(store, item, CJSON_STRING)
}
pub fn is_array<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    low_type_is(store, item, CJSON_ARRAY)
}
pub fn is_object<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    low_type_is(store, item, CJSON_OBJECT)
}
pub fn is_raw<S: NodeStore>(store: &S, item: Option<S::Id>) -> bool {
    low_type_is(store, item, CJSON_RAW)
}

// ----------------------------------------------------------------- create

/// `cJSON_New_Item` followed by `item->type = type_bits`.
fn new_item_with_type<S: NodeStore>(store: &mut S, type_bits: i32) -> Option<S::Id> {
    let id = store.alloc_node().ok()?;
    store.set_type_bits(id, type_bits);
    Some(id)
}

pub fn create_null<S: NodeStore>(store: &mut S) -> Option<S::Id> {
    new_item_with_type(store, CJSON_NULL)
}
pub fn create_true<S: NodeStore>(store: &mut S) -> Option<S::Id> {
    new_item_with_type(store, CJSON_TRUE)
}
pub fn create_false<S: NodeStore>(store: &mut S) -> Option<S::Id> {
    new_item_with_type(store, CJSON_FALSE)
}
pub fn create_bool<S: NodeStore>(store: &mut S, boolean: bool) -> Option<S::Id> {
    new_item_with_type(store, if boolean { CJSON_TRUE } else { CJSON_FALSE })
}
/// `cJSON_CreateNumber` (cJSON.c:2553-2577): `valueint` saturates.
pub fn create_number<S: NodeStore>(store: &mut S, num: f64) -> Option<S::Id> {
    let id = new_item_with_type(store, CJSON_NUMBER)?;
    store.set_valuedouble(id, num);
    store.set_valueint(id, saturate_to_int(num));
    Some(id)
}

/// Shared body of `cJSON_CreateString` / `cJSON_CreateRaw`
/// (cJSON.c:2580-2595, 2634-2649): the node is allocated *before* the
/// NULL check on the text, and freed again when `cJSON_strdup` fails
/// (including for a NULL text).
fn create_with_copied_valuestring<S: NodeStore>(
    store: &mut S,
    type_bits: i32,
    text: Option<&[u8]>,
) -> Option<S::Id> {
    let id = new_item_with_type(store, type_bits)?;
    let copied = match text {
        Some(text) => store.set_valuestring_copy(id, c_str(text)).is_ok(),
        None => false,
    };
    if copied {
        Some(id)
    } else {
        delete(store, Some(id));
        None
    }
}

pub fn create_string<S: NodeStore>(store: &mut S, string: Option<&[u8]>) -> Option<S::Id> {
    create_with_copied_valuestring(store, CJSON_STRING, string)
}
/// `cJSON_CreateStringReference`; `None` models a NULL `string`.
pub fn create_string_reference<S: NodeStore>(
    store: &mut S,
    string: Option<S::Borrowed>,
) -> Option<S::Id> {
    let id = new_item_with_type(store, CJSON_STRING | CJSON_IS_REFERENCE)?;
    if let Some(s) = string {
        store.set_valuestring_borrowed(id, s);
    }
    Some(id)
}
pub fn create_object_reference<S: NodeStore>(store: &mut S, child: Option<S::Id>) -> Option<S::Id> {
    let id = new_item_with_type(store, CJSON_OBJECT | CJSON_IS_REFERENCE)?;
    store.set_child(id, child);
    Some(id)
}
pub fn create_array_reference<S: NodeStore>(store: &mut S, child: Option<S::Id>) -> Option<S::Id> {
    let id = new_item_with_type(store, CJSON_ARRAY | CJSON_IS_REFERENCE)?;
    store.set_child(id, child);
    Some(id)
}
pub fn create_raw<S: NodeStore>(store: &mut S, raw: Option<&[u8]>) -> Option<S::Id> {
    create_with_copied_valuestring(store, CJSON_RAW, raw)
}
pub fn create_array<S: NodeStore>(store: &mut S) -> Option<S::Id> {
    new_item_with_type(store, CJSON_ARRAY)
}
pub fn create_object<S: NodeStore>(store: &mut S) -> Option<S::Id> {
    new_item_with_type(store, CJSON_OBJECT)
}

/// Shared body of `cJSON_Create{Int,Float,Double,String}Array`
/// (cJSON.c:2676-2836): elements are linked with `suffix_object`; if an
/// element cannot be created the whole array (with the elements linked so
/// far) is deleted; finally `a->child->prev = n` closes the
/// prev-of-first == last invariant.
fn create_array_of<S: NodeStore, T>(
    store: &mut S,
    values: &[T],
    mut make: impl FnMut(&mut S, &T) -> Option<S::Id>,
) -> Option<S::Id> {
    let array = create_array(store)?;
    let mut previous: Option<S::Id> = None;
    for value in values {
        let Some(node) = make(store, value) else {
            delete(store, Some(array));
            return None;
        };
        match previous {
            None => store.set_child(array, Some(node)),
            Some(p) => suffix_object(store, p, node),
        }
        previous = Some(node);
    }
    if let Some(first) = store.child(array) {
        store.set_prev(first, previous);
    }
    Some(array)
}

/// `cJSON_CreateIntArray(numbers, count)`. The FFI passes `None` when
/// `numbers == NULL || count < 0` (C returns NULL), else the `count` values.
pub fn create_int_array<S: NodeStore>(store: &mut S, numbers: Option<&[i32]>) -> Option<S::Id> {
    create_array_of(store, numbers?, |s, &n| create_number(s, f64::from(n)))
}
pub fn create_float_array<S: NodeStore>(store: &mut S, numbers: Option<&[f32]>) -> Option<S::Id> {
    create_array_of(store, numbers?, |s, &n| create_number(s, f64::from(n)))
}
pub fn create_double_array<S: NodeStore>(store: &mut S, numbers: Option<&[f64]>) -> Option<S::Id> {
    create_array_of(store, numbers?, |s, &n| create_number(s, n))
}
/// `cJSON_CreateStringArray(strings, count)`; inner `None` models a NULL
/// element pointer.
pub fn create_string_array<S: NodeStore>(
    store: &mut S,
    strings: Option<&[Option<&[u8]>]>,
) -> Option<S::Id> {
    create_array_of(store, strings?, |s, &text| create_string(s, text))
}

// ----------------------------------------------------------------- add

/// Static `suffix_object` (cJSON.c:2017-2021): `prev->next = item;
/// item->prev = prev;`.
pub fn suffix_object<S: NodeStore>(store: &mut S, prev: S::Id, item: S::Id) {
    store.set_next(prev, Some(item));
    store.set_prev(item, Some(prev));
}

/// Static `create_reference` (cJSON.c:2024-2043): `memcpy` of the whole
/// node (so `child`, `valuestring`, numbers and type are aliased), then
/// `string = NULL`, `type |= cJSON_IsReference`, `next = prev = NULL`.
pub fn create_reference<S: NodeStore>(store: &mut S, item: Option<S::Id>) -> Option<S::Id> {
    let item = item?;
    let reference = store.alloc_node().ok()?;
    store.copy_node_fields(reference, item);
    store.forget_string(reference);
    let type_bits = store.type_bits(reference);
    store.set_type_bits(reference, type_bits | CJSON_IS_REFERENCE);
    store.set_next(reference, None);
    store.set_prev(reference, None);
    Some(reference)
}

/// `cJSON_AddItemToArray` / static `add_item_to_array` (cJSON.c:2046-2077).
///
/// The last element is found through `child->prev`. Quirk: when the list is
/// non-empty but `child->prev` is NULL (a corrupted list), C does nothing
/// and still returns true (cJSON.c:2069-2073). The item's own `next` is
/// only cleared when it becomes the first element.
pub fn add_item_to_array<S: NodeStore>(
    store: &mut S,
    array: Option<S::Id>,
    item: Option<S::Id>,
) -> bool {
    let (Some(array), Some(item)) = (array, item) else {
        return false;
    };
    if array == item {
        return false;
    }
    match store.child(array) {
        None => {
            store.set_child(array, Some(item));
            store.set_prev(item, Some(item));
            store.set_next(item, None);
        }
        Some(child) => {
            if let Some(last) = store.prev(child) {
                suffix_object(store, last, item);
                store.set_prev(child, Some(item));
            }
        }
    }
    true
}

/// Checks shared by both `add_item_to_object` flavours (cJSON.c:2107).
fn object_add_allowed<S: NodeStore>(
    object: Option<S::Id>,
    item: Option<S::Id>,
) -> Option<(S::Id, S::Id)> {
    let (object, item) = (object?, item?);
    if object == item {
        None
    } else {
        Some((object, item))
    }
}

/// `cJSON_AddItemToObject` (key is copied).
///
/// Static `add_item_to_object` with `constant_key = false`
/// (cJSON.c:2102-2137): the key is duplicated *before* the old key is
/// freed (so a key aliasing `item->string` is safe), the old key is freed
/// only if it was not `cJSON_StringIsConst`, and the flag is cleared.
/// Duplicate keys are not detected.
pub fn add_item_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    key: Option<&[u8]>,
    item: Option<S::Id>,
) -> bool {
    let Some(key) = key else {
        return false;
    };
    let Some((object, item)) = object_add_allowed::<S>(object, item) else {
        return false;
    };
    let type_bits = store.type_bits(item);
    let free_old = type_bits & CJSON_STRING_IS_CONST == 0;
    if store
        .replace_string_copy(item, c_str(key), free_old)
        .is_err()
    {
        return false;
    }
    store.set_type_bits(item, type_bits & !CJSON_STRING_IS_CONST);
    add_item_to_array(store, Some(object), Some(item))
}

/// `cJSON_AddItemToObjectCS` (key stored without copying, flagged const).
///
/// Static `add_item_to_object` with `constant_key = true`: the old key is
/// freed unless it was itself const, then the caller's pointer is stored
/// and `cJSON_StringIsConst` is set (cJSON.c:2112-2134).
pub fn add_item_to_object_cs<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    key: Option<S::Borrowed>,
    item: Option<S::Id>,
) -> bool {
    let Some(key) = key else {
        return false;
    };
    let Some((object, item)) = object_add_allowed::<S>(object, item) else {
        return false;
    };
    let type_bits = store.type_bits(item);
    if type_bits & CJSON_STRING_IS_CONST == 0 {
        store.free_string(item);
    }
    store.set_string_borrowed(item, key);
    store.set_type_bits(item, type_bits | CJSON_STRING_IS_CONST);
    add_item_to_array(store, Some(object), Some(item))
}

/// `cJSON_AddItemReferenceToArray` (cJSON.c:2152-2160).
pub fn add_item_reference_to_array<S: NodeStore>(
    store: &mut S,
    array: Option<S::Id>,
    item: Option<S::Id>,
) -> bool {
    if array.is_none() {
        return false;
    }
    let reference = create_reference(store, item);
    add_item_to_array(store, array, reference)
}

/// `cJSON_AddItemReferenceToObject` (cJSON.c:2163-2171).
///
/// Quirk: if copying the key fails, the freshly created reference node is
/// leaked, exactly as in C (nothing frees it).
pub fn add_item_reference_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    key: Option<&[u8]>,
    item: Option<S::Id>,
) -> bool {
    if object.is_none() || key.is_none() {
        return false;
    }
    let reference = create_reference(store, item);
    add_item_to_object(store, object, key, reference)
}

/// Shared body of the `cJSON_Add*ToObject` helpers (cJSON.c:2174-2288):
/// create, add under a copied key, delete the new item if adding fails.
fn add_created_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
    created: Option<S::Id>,
) -> Option<S::Id> {
    if add_item_to_object(store, object, name, created) {
        created
    } else {
        delete(store, created);
        None
    }
}

pub fn add_null_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
) -> Option<S::Id> {
    let item = create_null(store);
    add_created_to_object(store, object, name, item)
}
pub fn add_true_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
) -> Option<S::Id> {
    let item = create_true(store);
    add_created_to_object(store, object, name, item)
}
pub fn add_false_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
) -> Option<S::Id> {
    let item = create_false(store);
    add_created_to_object(store, object, name, item)
}
pub fn add_bool_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
    boolean: bool,
) -> Option<S::Id> {
    let item = create_bool(store, boolean);
    add_created_to_object(store, object, name, item)
}
pub fn add_number_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
    number: f64,
) -> Option<S::Id> {
    let item = create_number(store, number);
    add_created_to_object(store, object, name, item)
}
pub fn add_string_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
    string: Option<&[u8]>,
) -> Option<S::Id> {
    let item = create_string(store, string);
    add_created_to_object(store, object, name, item)
}
pub fn add_raw_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
    raw: Option<&[u8]>,
) -> Option<S::Id> {
    let item = create_raw(store, raw);
    add_created_to_object(store, object, name, item)
}
pub fn add_object_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
) -> Option<S::Id> {
    let item = create_object(store);
    add_created_to_object(store, object, name, item)
}
pub fn add_array_to_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
) -> Option<S::Id> {
    let item = create_array(store);
    add_created_to_object(store, object, name, item)
}

// ----------------------------------------------------------------- detach / delete

/// `cJSON_DetachItemViaPointer` (cJSON.c:2291-2325).
///
/// Quirks kept from C: a non-first item whose `prev` is NULL is rejected
/// (cJSON.c:2293); the item is not checked to actually belong to `parent`.
/// When the last element is detached, the new last is recorded in
/// `parent->child->prev`. (C would dereference a NULL `parent->child` if a
/// non-first item were detached from a childless parent; that crash path
/// is a no-op here.)
pub fn detach_item_via_pointer<S: NodeStore>(
    store: &mut S,
    parent: Option<S::Id>,
    item: Option<S::Id>,
) -> Option<S::Id> {
    let (parent, item) = (parent?, item?);
    let is_first = store.child(parent) == Some(item);
    if !is_first && store.prev(item).is_none() {
        return None;
    }
    if !is_first {
        if let Some(prev) = store.prev(item) {
            let next = store.next(item);
            store.set_next(prev, next);
        }
    }
    if let Some(next) = store.next(item) {
        let prev = store.prev(item);
        store.set_prev(next, prev);
    }
    if store.child(parent) == Some(item) {
        let next = store.next(item);
        store.set_child(parent, next);
    } else if store.next(item).is_none() {
        if let Some(first) = store.child(parent) {
            let prev = store.prev(item);
            store.set_prev(first, prev);
        }
    }
    store.set_prev(item, None);
    store.set_next(item, None);
    Some(item)
}

/// `cJSON_DetachItemFromArray` (cJSON.c:2328-2336).
pub fn detach_item_from_array<S: NodeStore>(
    store: &mut S,
    array: Option<S::Id>,
    which: i32,
) -> Option<S::Id> {
    let which = usize::try_from(which).ok()?;
    let item = get_array_item_at(store, array, which);
    detach_item_via_pointer(store, array, item)
}
pub fn delete_item_from_array<S: NodeStore>(store: &mut S, array: Option<S::Id>, which: i32) {
    let detached = detach_item_from_array(store, array, which);
    delete(store, detached);
}
/// `cJSON_DetachItemFromObject[CaseSensitive]` (cJSON.c:2345-2358).
pub fn detach_item_from_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
    case_sensitive: bool,
) -> Option<S::Id> {
    let to_detach = get_object_item(store, object, name, case_sensitive);
    detach_item_via_pointer(store, object, to_detach)
}
pub fn delete_item_from_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
    case_sensitive: bool,
) {
    let detached = detach_item_from_object(store, object, name, case_sensitive);
    delete(store, detached);
}

// ----------------------------------------------------------------- insert / replace

/// `cJSON_InsertItemInArray` (cJSON.c:2373-2405).
///
/// Negative indices fail; an index at or past the end appends via
/// `add_item_to_array`; a non-first element with a NULL `prev` is treated
/// as corrupted and fails. Inserting at the head takes over the old head's
/// `prev` (the last element), preserving prev-of-first == last.
pub fn insert_item_in_array<S: NodeStore>(
    store: &mut S,
    array: Option<S::Id>,
    which: i32,
    newitem: Option<S::Id>,
) -> bool {
    let Ok(which) = usize::try_from(which) else {
        return false;
    };
    let Some(newitem) = newitem else {
        return false;
    };
    let Some(after_inserted) = get_array_item_at(store, array, which) else {
        return add_item_to_array(store, array, Some(newitem));
    };
    let Some(array) = array else {
        return false;
    };
    let is_first = store.child(array) == Some(after_inserted);
    if !is_first && store.prev(after_inserted).is_none() {
        return false;
    }
    store.set_next(newitem, Some(after_inserted));
    let before = store.prev(after_inserted);
    store.set_prev(newitem, before);
    store.set_prev(after_inserted, Some(newitem));
    if store.child(array) == Some(after_inserted) {
        store.set_child(array, Some(newitem));
    } else if let Some(before) = store.prev(newitem) {
        store.set_next(before, Some(newitem));
    }
    true
}

/// `cJSON_ReplaceItemViaPointer` (cJSON.c:2408-2455).
///
/// Replacing an item with itself succeeds without doing anything. The
/// replaced item is deleted. `item` is not checked to belong to `parent`.
pub fn replace_item_via_pointer<S: NodeStore>(
    store: &mut S,
    parent: Option<S::Id>,
    item: Option<S::Id>,
    replacement: Option<S::Id>,
) -> bool {
    let (Some(parent), Some(replacement), Some(item)) = (parent, replacement, item) else {
        return false;
    };
    if store.child(parent).is_none() {
        return false;
    }
    if replacement == item {
        return true;
    }

    let next = store.next(item);
    let prev = store.prev(item);
    store.set_next(replacement, next);
    store.set_prev(replacement, prev);

    if let Some(next) = store.next(replacement) {
        store.set_prev(next, Some(replacement));
    }
    if let Some(first) = store.child(parent).filter(|&first| first == item) {
        if store.prev(first) == Some(first) {
            store.set_prev(replacement, Some(replacement));
        }
        store.set_child(parent, Some(replacement));
    } else {
        if let Some(prev) = store.prev(replacement) {
            store.set_next(prev, Some(replacement));
        }
        if store.next(replacement).is_none() {
            if let Some(first) = store.child(parent) {
                store.set_prev(first, Some(replacement));
            }
        }
    }

    store.set_next(item, None);
    store.set_prev(item, None);
    delete(store, Some(item));
    true
}

/// `cJSON_ReplaceItemInArray` (cJSON.c:2458-2466).
pub fn replace_item_in_array<S: NodeStore>(
    store: &mut S,
    array: Option<S::Id>,
    which: i32,
    newitem: Option<S::Id>,
) -> bool {
    let Ok(which) = usize::try_from(which) else {
        return false;
    };
    let item = get_array_item_at(store, array, which);
    replace_item_via_pointer(store, array, item, newitem)
}

/// Static `replace_item_in_object` behind `cJSON_ReplaceItemInObject` and
/// `cJSON_ReplaceItemInObjectCaseSensitive` (cJSON.c:2469-2490).
///
/// The replacement's old key is released (freed unless
/// `cJSON_StringIsConst`) and replaced with a copy of `name`, and the const
/// flag is cleared, *before* the lookup; so when no member matches, the
/// call returns false but the replacement keeps the new key. If copying the
/// key fails, the replacement is left with a NULL key and false is
/// returned, as in C.
///
/// C frees the old key before duplicating the new one (cJSON.c:2477-2481),
/// which is a use-after-free when `name` aliases the replacement's own key.
/// Here the copy is made first and the old key released afterwards; the
/// resulting node state is the same as C's in every non-aliased case.
pub fn replace_item_in_object<S: NodeStore>(
    store: &mut S,
    object: Option<S::Id>,
    name: Option<&[u8]>,
    newitem: Option<S::Id>,
    case_sensitive: bool,
) -> bool {
    let (Some(replacement), Some(name)) = (newitem, name) else {
        return false;
    };
    let type_bits = store.type_bits(replacement);
    let owns_key = type_bits & CJSON_STRING_IS_CONST == 0;
    if store
        .replace_string_copy(replacement, c_str(name), owns_key)
        .is_err()
    {
        if owns_key {
            store.free_string(replacement);
        } else {
            store.forget_string(replacement);
        }
        return false;
    }
    store.set_type_bits(replacement, type_bits & !CJSON_STRING_IS_CONST);
    let item = get_object_item(store, object, Some(name), case_sensitive);
    replace_item_via_pointer(store, object, item, Some(replacement))
}

// ----------------------------------------------------------------- setters

/// `cJSON_SetNumberHelper(object, number)` (NaN when object is NULL).
/// `valueint` saturates (cJSON.c:417-437).
pub fn set_number_helper<S: NodeStore>(store: &mut S, object: Option<S::Id>, number: f64) -> f64 {
    let Some(object) = object else {
        return f64::NAN;
    };
    store.set_valueint(object, saturate_to_int(number));
    store.set_valuedouble(object, number);
    number
}

/// Decision taken by `cJSON_SetValuestring` (cJSON.c:441-482).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetValuestringPlan {
    /// Return NULL without touching the object.
    Reject,
    /// `strcpy` the new bytes over the existing buffer and return it.
    CopyInPlace,
    /// Allocate a copy, free the old buffer, store and return the copy.
    Reallocate,
}

/// Policy half of `cJSON_SetValuestring`. The FFI supplies facts only it can
/// compute from raw pointers: `new_len` (None when `valuestring == NULL`)
/// and whether the new string overlaps the current buffer per C's check.
///
/// Quirks kept from C (cJSON.c:447-469):
/// * the type test is `type & cJSON_String` (a bit test, not `IsString`),
///   and `cJSON_IsReference` nodes are rejected;
/// * a node whose `valuestring` is NULL is rejected;
/// * the overlap check only applies when the new string fits
///   (`new_len <= strlen(old)`); a longer, overlapping string is copied
///   into a fresh allocation.
///
/// If reallocation fails at the FFI level, C returns NULL and leaves the
/// node unchanged.
pub fn set_valuestring_plan<S: NodeStore>(
    store: &S,
    object: Option<S::Id>,
    new_len: Option<usize>,
    overlaps: bool,
) -> SetValuestringPlan {
    let Some(object) = object else {
        return SetValuestringPlan::Reject;
    };
    let type_bits = store.type_bits(object);
    if type_bits & CJSON_STRING == 0 || type_bits & CJSON_IS_REFERENCE != 0 {
        return SetValuestringPlan::Reject;
    }
    let (Some(old), Some(new_len)) = (store.valuestring(object), new_len) else {
        return SetValuestringPlan::Reject;
    };
    if new_len <= old.len() {
        if overlaps {
            SetValuestringPlan::Reject
        } else {
            SetValuestringPlan::CopyInPlace
        }
    } else {
        SetValuestringPlan::Reallocate
    }
}
