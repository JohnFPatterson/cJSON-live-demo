//! Port of tests/misc_tests.c, one `#[test]` per `RUN_TEST`.
//!
//! The C tests build nodes on the stack (`cJSON list[4]`, `memset`); here
//! they are zeroed `Arena` nodes linked by hand through the store API.
//! Header macros (`cJSON_ArrayForEach`, `cJSON_SetNumberValue`,
//! `cJSON_SetBoolValue`) are emulated exactly as they expand, and
//! `cJSON_SetValuestring` is emulated on top of `tree::set_valuestring_plan`
//! the way the FFI shim performs it.

mod common_tree;

use std::rc::Rc;

use cjson_core::compare::compare;
use cjson_core::duplicate::duplicate;
use cjson_core::parse::internals::{skip_utf8_bom, ParseBuffer};
use cjson_core::parse::parse;
use cjson_core::print::internals::{ensure, PrintBuffer};
use cjson_core::print::{print, print_buffered, print_preallocated};
use cjson_core::tree::{self, SetValuestringPlan};
use cjson_core::{
    Arena, NodeId, NodeStore, CJSON_ARRAY, CJSON_FALSE, CJSON_INVALID, CJSON_IS_REFERENCE,
    CJSON_NESTING_LIMIT, CJSON_NULL, CJSON_NUMBER, CJSON_OBJECT, CJSON_RAW, CJSON_STRING,
    CJSON_STRING_IS_CONST, CJSON_TRUE,
};
use common_tree::{cstr, init_hooks};

// ------------------------------------------------------------------ helpers

/// `cJSON_Parse(json)`: the root, or None where C returns NULL.
fn parse_root(arena: &mut Arena, json: &str) -> Option<NodeId> {
    parse(arena, json.as_bytes())
        .ok()
        .map(|success| success.root)
}

/// `cJSON_ArrayForEach(element, array) { body(element) }`
/// (cJSON.h:296: `element = array ? array->child : NULL; element; element = element->next`).
fn array_for_each(arena: &Arena, array: Option<NodeId>, mut body: impl FnMut(NodeId)) {
    let mut element = match array {
        Some(array) => arena.child(array),
        None => None,
    };
    while let Some(e) = element {
        body(e);
        element = arena.next(e);
    }
}

/// `cJSON_SetNumberValue(object, number)` (cJSON.h:284).
fn set_number_value(arena: &mut Arena, object: Option<NodeId>, number: f64) -> f64 {
    if object.is_some() {
        tree::set_number_helper(arena, object, number)
    } else {
        number
    }
}

/// `cJSON_SetBoolValue(object, boolValue)` (cJSON.h:289-293): the value of
/// the assignment when `object` is a bool, `cJSON_Invalid` otherwise.
fn set_bool_value(arena: &mut Arena, object: Option<NodeId>, bool_value: bool) -> i32 {
    match object {
        Some(o) if arena.type_bits(o) & (CJSON_FALSE | CJSON_TRUE) != 0 => {
            let new_type = (arena.type_bits(o) & !(CJSON_FALSE | CJSON_TRUE))
                | if bool_value { CJSON_TRUE } else { CJSON_FALSE };
            arena.set_type_bits(o, new_type);
            new_type
        }
        _ => CJSON_INVALID,
    }
}

/// Address of a node's `valuestring` buffer: the C `char *` value that the
/// C tests compare. Only the address is kept, so the buffer stays unshared.
fn valuestring_ptr(arena: &Arena, id: NodeId) -> Option<*const u8> {
    arena
        .node(id)
        .and_then(|n| n.valuestring.as_ref())
        .map(|s| s.as_ptr())
}

/// `cJSON_SetValuestring(object, new)` as the FFI shim performs it:
/// `tree::set_valuestring_plan` decides, then `CopyInPlace` does C's
/// `strcpy` into the existing buffer (same address) and `Reallocate` stores
/// a fresh copy. Returns the address C would return, or None for NULL.
/// `overlaps` is the result of C's pointer check (cJSON.c:463), which only
/// the caller can compute.
fn set_valuestring(
    arena: &mut Arena,
    object: Option<NodeId>,
    new: &[u8],
    overlaps: bool,
) -> Option<*const u8> {
    match tree::set_valuestring_plan(arena, object, Some(new.len()), overlaps) {
        SetValuestringPlan::Reject => None,
        SetValuestringPlan::CopyInPlace => {
            let node = arena.node_mut(object?)?;
            let buffer =
                Rc::get_mut(node.valuestring.as_mut()?).expect("test buffers are not shared");
            buffer[..new.len()].copy_from_slice(new);
            if let Some(terminator) = buffer.get_mut(new.len()) {
                *terminator = 0;
            }
            valuestring_ptr(arena, object?)
        }
        SetValuestringPlan::Reallocate => {
            arena.replace_valuestring_copy(object?, new, true).ok()?;
            valuestring_ptr(arena, object?)
        }
    }
}

/// C's overlap test from `cJSON_SetValuestring` (cJSON.c:463),
/// `!(valuestring + v1_len < object->valuestring || object->valuestring + v2_len < valuestring)`,
/// with both pointers expressed as offsets from the start of
/// `object->valuestring`.
fn c_overlap_check(new_start: isize, new_len: usize, old_len: usize) -> bool {
    let new_end = new_start + new_len as isize;
    !(new_end < 0 || (old_len as isize) < new_start)
}

/// A node with every field zero, like `memset(&node, 0, sizeof(cJSON))`.
fn zeroed(arena: &mut Arena) -> NodeId {
    arena.alloc_node().unwrap()
}

// ------------------------------------------------------------------ tests

#[test]
fn cjson_array_foreach_should_loop_over_arrays() {
    let mut arena = Arena::new();
    let array = zeroed(&mut arena);
    let elements: Vec<NodeId> = (0..10).map(|_| zeroed(&mut arena)).collect();

    // create array
    arena.set_child(array, Some(elements[0]));
    arena.set_prev(elements[0], None);
    arena.set_next(elements[9], None);
    for i in 0..9 {
        arena.set_next(elements[i], Some(elements[i + 1]));
        arena.set_prev(elements[i + 1], Some(elements[i]));
    }

    let mut i = 0;
    array_for_each(&arena, Some(array), |element_pointer| {
        assert!(
            element_pointer == elements[i],
            "Not iterating over array properly"
        );
        i += 1;
    });
}

#[test]
fn cjson_array_foreach_should_not_dereference_null_pointer() {
    let arena = Arena::new();
    let array: Option<NodeId> = None;
    array_for_each(&arena, array, |_element| {});
}

#[test]
fn cjson_get_object_item_should_get_object_items() {
    let mut arena = Arena::new();
    let item = parse_root(&mut arena, "{\"one\":1, \"Two\":2, \"tHree\":3}");

    let found = tree::get_object_item(&arena, None, cstr("test"), false);
    assert!(found.is_none(), "Failed to fail on NULL pointer.");

    let found = tree::get_object_item(&arena, item, None, false);
    assert!(found.is_none(), "Failed to fail on NULL string.");

    let found = tree::get_object_item(&arena, item, cstr("one"), false);
    assert!(found.is_some(), "Failed to find first item.");
    assert_eq!(arena.valuedouble(found.unwrap()), 1.0);

    let found = tree::get_object_item(&arena, item, cstr("tWo"), false);
    assert!(found.is_some(), "Failed to find first item.");
    assert_eq!(arena.valuedouble(found.unwrap()), 2.0);

    let found = tree::get_object_item(&arena, item, cstr("three"), false);
    assert!(found.is_some(), "Failed to find item.");
    assert_eq!(arena.valuedouble(found.unwrap()), 3.0);

    let found = tree::get_object_item(&arena, item, cstr("four"), false);
    assert!(
        found.is_none(),
        "Should not find something that isn't there."
    );

    tree::delete(&mut arena, item);
}

#[test]
fn cjson_get_object_item_case_sensitive_should_get_object_items() {
    let mut arena = Arena::new();
    let item = parse_root(&mut arena, "{\"one\":1, \"Two\":2, \"tHree\":3}");

    let found = tree::get_object_item(&arena, None, cstr("test"), true);
    assert!(found.is_none(), "Failed to fail on NULL pointer.");

    let found = tree::get_object_item(&arena, item, None, true);
    assert!(found.is_none(), "Failed to fail on NULL string.");

    let found = tree::get_object_item(&arena, item, cstr("one"), true);
    assert!(found.is_some(), "Failed to find first item.");
    assert_eq!(arena.valuedouble(found.unwrap()), 1.0);

    let found = tree::get_object_item(&arena, item, cstr("Two"), true);
    assert!(found.is_some(), "Failed to find first item.");
    assert_eq!(arena.valuedouble(found.unwrap()), 2.0);

    let found = tree::get_object_item(&arena, item, cstr("tHree"), true);
    assert!(found.is_some(), "Failed to find item.");
    assert_eq!(arena.valuedouble(found.unwrap()), 3.0);

    let found = tree::get_object_item(&arena, item, cstr("One"), true);
    assert!(
        found.is_none(),
        "Should not find something that isn't there."
    );

    tree::delete(&mut arena, item);
}

#[test]
fn cjson_get_object_item_should_not_crash_with_array() {
    let mut arena = Arena::new();
    let array = parse_root(&mut arena, "[1]");

    let found = tree::get_object_item(&arena, array, cstr("name"), false);
    assert!(found.is_none());

    tree::delete(&mut arena, array);
}

#[test]
fn cjson_get_object_item_case_sensitive_should_not_crash_with_array() {
    let mut arena = Arena::new();
    let array = parse_root(&mut arena, "[1]");

    let found = tree::get_object_item(&arena, array, cstr("name"), true);
    assert!(found.is_none());

    tree::delete(&mut arena, array);
}

#[test]
fn typecheck_functions_should_check_type() {
    let mut arena = Arena::new();
    let invalid = zeroed(&mut arena);
    let item = zeroed(&mut arena);
    arena.set_type_bits(invalid, CJSON_INVALID);
    arena.set_type_bits(invalid, arena.type_bits(invalid) | CJSON_STRING_IS_CONST);
    arena.set_type_bits(item, CJSON_FALSE);
    arena.set_type_bits(item, arena.type_bits(item) | CJSON_STRING_IS_CONST);
    let (invalid, item) = (Some(invalid), Some(item));
    let set = |arena: &mut Arena, t: i32| arena.set_type_bits(item.unwrap(), t);

    assert!(!tree::is_invalid(&arena, None));
    assert!(!tree::is_invalid(&arena, item));
    assert!(tree::is_invalid(&arena, invalid));

    set(&mut arena, CJSON_FALSE | CJSON_STRING_IS_CONST);
    assert!(!tree::is_false(&arena, None));
    assert!(!tree::is_false(&arena, invalid));
    assert!(tree::is_false(&arena, item));
    assert!(tree::is_bool(&arena, item));

    set(&mut arena, CJSON_TRUE | CJSON_STRING_IS_CONST);
    assert!(!tree::is_true(&arena, None));
    assert!(!tree::is_true(&arena, invalid));
    assert!(tree::is_true(&arena, item));
    assert!(tree::is_bool(&arena, item));

    set(&mut arena, CJSON_NULL | CJSON_STRING_IS_CONST);
    assert!(!tree::is_null(&arena, None));
    assert!(!tree::is_null(&arena, invalid));
    assert!(tree::is_null(&arena, item));

    set(&mut arena, CJSON_NUMBER | CJSON_STRING_IS_CONST);
    assert!(!tree::is_number(&arena, None));
    assert!(!tree::is_number(&arena, invalid));
    assert!(tree::is_number(&arena, item));

    set(&mut arena, CJSON_STRING | CJSON_STRING_IS_CONST);
    assert!(!tree::is_string(&arena, None));
    assert!(!tree::is_string(&arena, invalid));
    assert!(tree::is_string(&arena, item));

    set(&mut arena, CJSON_ARRAY | CJSON_STRING_IS_CONST);
    assert!(!tree::is_array(&arena, None));
    assert!(!tree::is_array(&arena, invalid));
    assert!(tree::is_array(&arena, item));

    set(&mut arena, CJSON_OBJECT | CJSON_STRING_IS_CONST);
    assert!(!tree::is_object(&arena, None));
    assert!(!tree::is_object(&arena, invalid));
    assert!(tree::is_object(&arena, item));

    set(&mut arena, CJSON_RAW | CJSON_STRING_IS_CONST);
    assert!(!tree::is_raw(&arena, None));
    assert!(!tree::is_raw(&arena, invalid));
    assert!(tree::is_raw(&arena, item));
}

#[test]
fn cjson_should_not_parse_to_deeply_nested_jsons() {
    let mut arena = Arena::new();
    // `char deep_json[CJSON_NESTING_LIMIT + 1]` filled with '[' and
    // terminated in its last byte: CJSON_NESTING_LIMIT brackets.
    let deep_json = vec![b'['; CJSON_NESTING_LIMIT];

    assert!(
        parse(&mut arena, &deep_json).is_err(),
        "To deep JSONs should not be parsed."
    );
}

#[test]
fn cjson_should_not_follow_too_deep_circular_references() {
    let mut arena = Arena::new();
    let o = tree::create_array(&mut arena);
    let a = tree::create_array(&mut arena);
    let b = tree::create_array(&mut arena);

    tree::add_item_to_array(&mut arena, o, a);
    tree::add_item_to_array(&mut arena, a, b);
    tree::add_item_to_array(&mut arena, b, o);

    let x = duplicate(&mut arena, o, true);
    assert!(x.is_none());
    tree::detach_item_from_array(&mut arena, b, 0);
    tree::delete(&mut arena, o);
}

#[test]
fn cjson_set_number_value_should_set_numbers() {
    let mut arena = Arena::new();
    let n = zeroed(&mut arena);
    arena.set_type_bits(n, CJSON_NUMBER);
    let number = Some(n);

    set_number_value(&mut arena, number, 1.5);
    assert_eq!(1, arena.valueint(n));
    assert_eq!(1.5, arena.valuedouble(n));

    set_number_value(&mut arena, number, -1.5);
    assert_eq!(-1, arena.valueint(n));
    assert_eq!(-1.5, arena.valuedouble(n));

    set_number_value(&mut arena, number, 1.0 + f64::from(i32::MAX));
    assert_eq!(i32::MAX, arena.valueint(n));
    assert_eq!(1.0 + f64::from(i32::MAX), arena.valuedouble(n));

    set_number_value(&mut arena, number, -1.0 + f64::from(i32::MIN));
    assert_eq!(i32::MIN, arena.valueint(n));
    assert_eq!(-1.0 + f64::from(i32::MIN), arena.valuedouble(n));
}

#[test]
fn cjson_detach_item_via_pointer_should_detach_items() {
    let mut arena = Arena::new();
    let list: Vec<NodeId> = (0..4).map(|_| zeroed(&mut arena)).collect();
    let parent = zeroed(&mut arena);

    // link the list
    arena.set_next(list[0], Some(list[1]));
    arena.set_next(list[1], Some(list[2]));
    arena.set_next(list[2], Some(list[3]));

    arena.set_prev(list[3], Some(list[2]));
    arena.set_prev(list[2], Some(list[1]));
    arena.set_prev(list[1], Some(list[0]));
    arena.set_prev(list[0], Some(list[3]));

    arena.set_child(parent, Some(list[0]));

    // detach in the middle (list[1])
    assert!(
        tree::detach_item_via_pointer(&mut arena, Some(parent), Some(list[1])) == Some(list[1]),
        "Failed to detach in the middle."
    );
    assert!(
        arena.prev(list[1]).is_none() && arena.next(list[1]).is_none(),
        "Didn't set pointers of detached item to NULL."
    );
    assert!(arena.next(list[0]) == Some(list[2]) && arena.prev(list[2]) == Some(list[0]));

    // detach beginning (list[0])
    assert!(
        tree::detach_item_via_pointer(&mut arena, Some(parent), Some(list[0])) == Some(list[0]),
        "Failed to detach beginning."
    );
    assert!(
        arena.prev(list[0]).is_none() && arena.next(list[0]).is_none(),
        "Didn't set pointers of detached item to NULL."
    );
    assert!(
        arena.prev(list[2]) == Some(list[3]) && arena.child(parent) == Some(list[2]),
        "Didn't set the new beginning."
    );

    // detach end (list[3])
    assert!(
        tree::detach_item_via_pointer(&mut arena, Some(parent), Some(list[3])) == Some(list[3]),
        "Failed to detach end."
    );
    assert!(
        arena.prev(list[3]).is_none() && arena.next(list[3]).is_none(),
        "Didn't set pointers of detached item to NULL."
    );
    assert!(
        arena.next(list[2]).is_none() && arena.child(parent) == Some(list[2]),
        "Didn't set the new end"
    );

    // detach single item (list[2])
    assert!(
        tree::detach_item_via_pointer(&mut arena, Some(parent), Some(list[2])) == Some(list[2]),
        "Failed to detach single item."
    );
    assert!(
        arena.prev(list[2]).is_none() && arena.next(list[2]).is_none(),
        "Didn't set pointers of detached item to NULL."
    );
    assert!(
        arena.child(parent).is_none(),
        "Child of the parent wasn't set to NULL."
    );
}

#[test]
fn cjson_detach_item_via_pointer_should_return_null_if_item_prev_is_null() {
    let mut arena = Arena::new();
    let list: Vec<NodeId> = (0..2).map(|_| zeroed(&mut arena)).collect();
    let parent = zeroed(&mut arena);

    // link the list
    arena.set_next(list[0], Some(list[1]));

    arena.set_child(parent, Some(list[0]));
    assert!(
        tree::detach_item_via_pointer(&mut arena, Some(parent), Some(list[1])).is_none(),
        "Failed to detach in the middle."
    );
    assert!(
        tree::detach_item_via_pointer(&mut arena, Some(parent), Some(list[0])) == Some(list[0]),
        "Failed to detach in the middle."
    );
}

#[test]
fn cjson_replace_item_via_pointer_should_replace_items() {
    let mut arena = Arena::new();

    let beginning = tree::create_null(&mut arena);
    assert!(beginning.is_some());
    let middle = tree::create_null(&mut arena);
    assert!(middle.is_some());
    let end = tree::create_null(&mut arena);
    assert!(end.is_some());

    let array = tree::create_array(&mut arena);
    assert!(array.is_some());

    tree::add_item_to_array(&mut arena, array, beginning);
    tree::add_item_to_array(&mut arena, array, middle);
    tree::add_item_to_array(&mut arena, array, end);

    let replacements: Vec<NodeId> = (0..3).map(|_| zeroed(&mut arena)).collect();
    let (middle_id, end_id) = (middle.unwrap(), end.unwrap());

    // replace beginning
    assert!(tree::replace_item_via_pointer(
        &mut arena,
        array,
        beginning,
        Some(replacements[0])
    ));
    assert!(arena.prev(replacements[0]) == end);
    assert!(arena.next(replacements[0]) == middle);
    assert!(arena.prev(middle_id) == Some(replacements[0]));
    assert!(arena.child(array.unwrap()) == Some(replacements[0]));

    // replace middle
    assert!(tree::replace_item_via_pointer(
        &mut arena,
        array,
        middle,
        Some(replacements[1])
    ));
    assert!(arena.prev(replacements[1]) == Some(replacements[0]));
    assert!(arena.next(replacements[1]) == end);
    assert!(arena.prev(end_id) == Some(replacements[1]));

    // replace end
    assert!(tree::replace_item_via_pointer(
        &mut arena,
        array,
        end,
        Some(replacements[2])
    ));
    assert!(arena.prev(replacements[2]) == Some(replacements[1]));
    assert!(arena.next(replacements[2]).is_none());
    assert!(arena.next(replacements[1]) == Some(replacements[2]));

    // cJSON_free(array): frees the array node only.
    arena.free_node(array.unwrap());
}

#[test]
fn cjson_replace_item_in_object_should_preserve_name() {
    let mut arena = Arena::new();
    let root_id = zeroed(&mut arena);
    let root = Some(root_id);

    let child = tree::create_number(&mut arena, 1.0);
    assert!(child.is_some());
    let replacement = tree::create_number(&mut arena, 2.0);
    assert!(replacement.is_some());

    let flag = tree::add_item_to_object(&mut arena, root, cstr("child"), child);
    assert!(flag, "add item to object failed");
    tree::replace_item_in_object(&mut arena, root, cstr("child"), replacement, false);

    assert!(arena.child(root_id) == replacement);
    assert_eq!(arena.string(replacement.unwrap()), Some(&b"child"[..]));

    tree::delete(&mut arena, replacement);
}

#[test]
fn cjson_functions_should_not_crash_with_null_pointers() {
    let mut arena = Arena::new();
    let item = tree::create_string(&mut arena, cstr("item"));
    let array = tree::create_array(&mut arena);
    let item1 = tree::create_string(&mut arena, cstr("item1"));
    let item2 = tree::create_string(&mut arena, cstr("corrupted array item3"));
    let corrupted_string = tree::create_string(&mut arena, cstr("corrupted"));
    let item_cs_key = || Some(Rc::<[u8]>::from(&b"item"[..]));

    tree::add_item_to_array(&mut arena, array, item1);
    tree::add_item_to_array(&mut arena, array, item2);

    let original_prev = arena.prev(item2.unwrap());
    arena.set_prev(item2.unwrap(), None);
    arena.free_valuestring(corrupted_string.unwrap());

    init_hooks(&mut arena, false);
    // `cJSON_Parse(NULL)` and `cJSON_ParseWithOpts(NULL, NULL, true)`: the
    // core parser takes `&[u8]`, which cannot be NULL; the NULL-input
    // rejection is in the FFI shim and covered by the FFI Unity run.
    assert!(print(&arena, None, true).is_none());
    assert!(print(&arena, None, false).is_none());
    assert!(print_buffered(&arena, None, 10, true).is_none());
    assert!(!print_preallocated(&arena, None, 10, true).ok);
    // `cJSON_PrintPreallocated(item, NULL, 1, true)`: the core printer takes
    // no output pointer, so a NULL buffer cannot be expressed here; the FFI
    // shim rejects it and the FFI Unity run covers it.
    tree::delete(&mut arena, None);
    tree::get_array_size(&arena, None);
    assert!(tree::get_array_item(&arena, None, 0).is_none());
    assert!(tree::get_object_item(&arena, None, cstr("item"), false).is_none());
    assert!(tree::get_object_item(&arena, item, None, false).is_none());
    assert!(tree::get_object_item(&arena, None, cstr("item"), true).is_none());
    assert!(tree::get_object_item(&arena, item, None, true).is_none());
    assert!(!tree::has_object_item(&arena, None, cstr("item")));
    assert!(!tree::has_object_item(&arena, item, None));
    assert!(!tree::is_invalid(&arena, None));
    assert!(!tree::is_false(&arena, None));
    assert!(!tree::is_true(&arena, None));
    assert!(!tree::is_bool(&arena, None));
    assert!(!tree::is_null(&arena, None));
    assert!(!tree::is_number(&arena, None));
    assert!(!tree::is_string(&arena, None));
    assert!(!tree::is_array(&arena, None));
    assert!(!tree::is_object(&arena, None));
    assert!(!tree::is_raw(&arena, None));
    assert!(tree::create_string(&mut arena, None).is_none());
    assert!(tree::create_raw(&mut arena, None).is_none());
    assert!(tree::create_int_array(&mut arena, None).is_none());
    assert!(tree::create_float_array(&mut arena, None).is_none());
    assert!(tree::create_double_array(&mut arena, None).is_none());
    assert!(tree::create_string_array(&mut arena, None).is_none());
    tree::add_item_to_array(&mut arena, None, item);
    tree::add_item_to_array(&mut arena, item, None);
    tree::add_item_to_object(&mut arena, item, cstr("item"), None);
    tree::add_item_to_object(&mut arena, item, None, item);
    tree::add_item_to_object(&mut arena, None, cstr("item"), item);
    tree::add_item_to_object_cs(&mut arena, item, item_cs_key(), None);
    tree::add_item_to_object_cs(&mut arena, item, None, item);
    tree::add_item_to_object_cs(&mut arena, None, item_cs_key(), item);
    tree::add_item_reference_to_array(&mut arena, None, item);
    tree::add_item_reference_to_array(&mut arena, item, None);
    tree::add_item_reference_to_object(&mut arena, item, cstr("item"), None);
    tree::add_item_reference_to_object(&mut arena, item, None, item);
    tree::add_item_reference_to_object(&mut arena, None, cstr("item"), item);
    assert!(tree::detach_item_via_pointer(&mut arena, None, item).is_none());
    assert!(tree::detach_item_via_pointer(&mut arena, item, None).is_none());
    assert!(tree::detach_item_from_array(&mut arena, None, 0).is_none());
    tree::delete_item_from_array(&mut arena, None, 0);
    assert!(tree::detach_item_from_object(&mut arena, None, cstr("item"), false).is_none());
    assert!(tree::detach_item_from_object(&mut arena, item, None, false).is_none());
    assert!(tree::detach_item_from_object(&mut arena, None, cstr("item"), true).is_none());
    assert!(tree::detach_item_from_object(&mut arena, item, None, true).is_none());
    tree::delete_item_from_object(&mut arena, None, cstr("item"), false);
    tree::delete_item_from_object(&mut arena, item, None, false);
    tree::delete_item_from_object(&mut arena, None, cstr("item"), true);
    tree::delete_item_from_object(&mut arena, item, None, true);
    assert!(!tree::insert_item_in_array(&mut arena, array, 0, None));
    assert!(!tree::insert_item_in_array(&mut arena, array, 1, item));
    assert!(!tree::insert_item_in_array(&mut arena, None, 0, item));
    assert!(!tree::insert_item_in_array(&mut arena, item, 0, None));
    assert!(!tree::replace_item_via_pointer(
        &mut arena, None, item, item
    ));
    assert!(!tree::replace_item_via_pointer(
        &mut arena, item, None, item
    ));
    assert!(!tree::replace_item_via_pointer(
        &mut arena, item, item, None
    ));
    assert!(!tree::replace_item_in_array(&mut arena, item, 0, None));
    assert!(!tree::replace_item_in_array(&mut arena, None, 0, item));
    assert!(!tree::replace_item_in_object(
        &mut arena,
        None,
        cstr("item"),
        item,
        false
    ));
    assert!(!tree::replace_item_in_object(
        &mut arena, item, None, item, false
    ));
    assert!(!tree::replace_item_in_object(
        &mut arena,
        item,
        cstr("item"),
        None,
        false
    ));
    assert!(!tree::replace_item_in_object(
        &mut arena,
        None,
        cstr("item"),
        item,
        true
    ));
    assert!(!tree::replace_item_in_object(
        &mut arena, item, None, item, true
    ));
    assert!(!tree::replace_item_in_object(
        &mut arena,
        item,
        cstr("item"),
        None,
        true
    ));
    assert!(duplicate(&mut arena, None, true).is_none());
    assert!(!compare(&arena, item, None, false));
    assert!(!compare(&arena, None, item, false));
    // cJSON_SetValuestring(NULL, "test"), (corruptedString, "test"), (item, NULL)
    assert_eq!(
        tree::set_valuestring_plan(&arena, None, Some(4), false),
        SetValuestringPlan::Reject
    );
    assert_eq!(
        tree::set_valuestring_plan(&arena, corrupted_string, Some(4), false),
        SetValuestringPlan::Reject
    );
    assert_eq!(
        tree::set_valuestring_plan(&arena, item, None, false),
        SetValuestringPlan::Reject
    );
    // `cJSON_Minify(NULL)`: the core minifier takes `&[u8]`, which cannot be
    // NULL; the FFI shim returns early and the FFI Unity run covers it.
    // cJSON_SetNumberHelper should handle NULL gracefully
    assert!(tree::set_number_helper(&mut arena, None, 0.0).is_nan());

    // restore corrupted item2 to delete it
    arena.set_prev(item2.unwrap(), original_prev);
    tree::delete(&mut arena, corrupted_string);
    tree::delete(&mut arena, array);
    tree::delete(&mut arena, item);
}

#[test]
fn cjson_set_valuestring_should_return_null_if_strings_overlap() {
    let mut arena = Arena::new();
    let obj = parse_root(&mut arena, "\"foo0z\"");

    let str1 = set_valuestring(&mut arena, obj, b"abcde", false);
    assert!(str1.is_some());
    // `str += 1;` then `cJSON_SetValuestring(obj, str)`: the argument is the
    // tail of obj's own buffer. The overlap facts below are what C computes
    // from those pointers (offset 1, strlen 4, old strlen 5); the pointer
    // arithmetic itself is exercised by the FFI Unity run.
    let str_bytes = arena.valuestring(obj.unwrap()).unwrap()[1..].to_vec();
    let old_len = arena.valuestring(obj.unwrap()).unwrap().len();
    let overlaps = c_overlap_check(1, str_bytes.len(), old_len);
    // The string passed to strcpy overlap which is not allowed.
    let str2 = set_valuestring(&mut arena, obj, &str_bytes, overlaps);
    // If it overlaps, the string will be messed up.
    assert_eq!(&arena.valuestring(obj.unwrap()).unwrap()[1..], b"bcde");
    assert!(str2.is_none());
    tree::delete(&mut arena, obj);
}

#[test]
fn ensure_should_fail_on_failed_realloc() {
    let mut buffer = PrintBuffer::new(10);
    buffer.fail_realloc = true;

    assert!(
        ensure(&mut buffer, 200).is_none(),
        "Ensure didn't fail with failing realloc."
    );
}

#[test]
fn skip_utf8_bom_should_skip_bom() {
    // `sizeof(string)` includes the NUL terminator.
    let string = b"\xEF\xBB\xBF{}\0";
    let mut buffer = ParseBuffer::new(string);

    assert!(skip_utf8_bom(&mut buffer));
    assert_eq!(3, buffer.offset);
}

#[test]
fn skip_utf8_bom_should_not_skip_bom_if_not_at_beginning() {
    // `sizeof(string)` includes the NUL terminator.
    let string = b" \xEF\xBB\xBF{}\0";
    let mut buffer = ParseBuffer::new(string);
    buffer.offset = 1;

    assert!(!skip_utf8_bom(&mut buffer));
}

#[test]
fn cjson_get_string_value_should_get_a_string() {
    let mut arena = Arena::new();
    let string = tree::create_string(&mut arena, cstr("test"));
    let number = tree::create_number(&mut arena, 1.0);

    // `cJSON_GetStringValue(string) == string->valuestring`: the node whose
    // valuestring C returns.
    assert!(tree::get_string_value_node(&arena, string) == string);
    assert!(tree::get_string_value_node(&arena, number).is_none());
    assert!(tree::get_string_value_node(&arena, None).is_none());

    tree::delete(&mut arena, number);
    tree::delete(&mut arena, string);
}

#[test]
fn cjson_get_number_value_should_get_a_number() {
    let mut arena = Arena::new();
    let string = tree::create_string(&mut arena, cstr("test"));
    let number = tree::create_number(&mut arena, 1.0);

    assert_eq!(
        tree::get_number_value(&arena, number),
        arena.valuedouble(number.unwrap())
    );
    assert!(tree::get_number_value(&arena, string).is_nan());
    assert!(tree::get_number_value(&arena, None).is_nan());

    tree::delete(&mut arena, number);
    tree::delete(&mut arena, string);
}

#[test]
fn cjson_create_string_reference_should_create_a_string_reference() {
    let mut arena = Arena::new();
    let string: Rc<[u8]> = Rc::from(&b"I am a string!"[..]);

    let string_reference = tree::create_string_reference(&mut arena, Some(string.clone()));
    assert!(valuestring_ptr(&arena, string_reference.unwrap()) == Some(string.as_ptr()));
    assert_eq!(
        CJSON_IS_REFERENCE | CJSON_STRING,
        arena.type_bits(string_reference.unwrap())
    );

    tree::delete(&mut arena, string_reference);
}

#[test]
fn cjson_create_object_reference_should_create_an_object_reference() {
    let mut arena = Arena::new();
    let number_object = tree::create_object(&mut arena);
    let number = tree::create_number(&mut arena, 42.0);
    let key: Rc<[u8]> = Rc::from(&b"number"[..]);

    assert!(tree::is_number(&arena, number));
    assert!(tree::is_object(&arena, number_object));
    tree::add_item_to_object_cs(&mut arena, number_object, Some(key), number);

    let number_reference = tree::create_object_reference(&mut arena, number);
    assert!(arena.child(number_reference.unwrap()) == number);
    assert_eq!(
        CJSON_OBJECT | CJSON_IS_REFERENCE,
        arena.type_bits(number_reference.unwrap())
    );

    tree::delete(&mut arena, number_object);
    tree::delete(&mut arena, number_reference);
}

#[test]
fn cjson_create_array_reference_should_create_an_array_reference() {
    let mut arena = Arena::new();
    let number_array = tree::create_array(&mut arena);
    let number = tree::create_number(&mut arena, 42.0);

    assert!(tree::is_number(&arena, number));
    assert!(tree::is_array(&arena, number_array));
    tree::add_item_to_array(&mut arena, number_array, number);

    let number_reference = tree::create_array_reference(&mut arena, number);
    assert!(arena.child(number_reference.unwrap()) == number);
    assert_eq!(
        CJSON_ARRAY | CJSON_IS_REFERENCE,
        arena.type_bits(number_reference.unwrap())
    );

    tree::delete(&mut arena, number_array);
    tree::delete(&mut arena, number_reference);
}

#[test]
fn cjson_add_item_to_object_or_array_should_not_add_itself() {
    let mut arena = Arena::new();
    let object = tree::create_object(&mut arena);
    let array = tree::create_array(&mut arena);

    let flag = tree::add_item_to_object(&mut arena, object, cstr("key"), object);
    assert!(!flag, "add an object to itself should fail");

    let flag = tree::add_item_to_array(&mut arena, array, array);
    assert!(!flag, "add an array to itself should fail");

    tree::delete(&mut arena, object);
    tree::delete(&mut arena, array);
}

#[test]
fn cjson_add_item_to_object_should_not_use_after_free_when_string_is_aliased() {
    let mut arena = Arena::new();
    let object = tree::create_object(&mut arena);
    let number = tree::create_number(&mut arena, 42.0);
    // `name = cJSON_strdup("number")`, then `number->string = name`.
    let name_stored = arena.set_string_copy(number.unwrap(), b"number");

    assert!(object.is_some());
    assert!(number.is_some());
    assert!(name_stored.is_ok());

    // `cJSON_AddItemToObject(object, number->string, number)`: the Arena
    // cannot lend its own key while being mutated, so the key bytes are
    // copied first. The aliased-pointer path itself (copy before free,
    // cJSON.c:2119-2131) is exercised by the FFI Unity run.
    let key = arena.string(number.unwrap()).unwrap().to_vec();
    tree::add_item_to_object(&mut arena, object, Some(&key), number);

    tree::delete(&mut arena, object);
}

#[test]
fn cjson_delete_item_from_array_should_not_broken_list_structure() {
    let mut arena = Arena::new();
    let expected_json1 = b"{\"rd\":[{\"a\":\"123\"}]}";
    let expected_json2 = b"{\"rd\":[{\"a\":\"123\"},{\"b\":\"456\"}]}";
    let expected_json3 = b"{\"rd\":[{\"b\":\"456\"}]}";

    let root = parse_root(&mut arena, "{}");

    let array = tree::add_array_to_object(&mut arena, root, cstr("rd"));
    let item1 = parse_root(&mut arena, "{\"a\":\"123\"}");
    let item2 = parse_root(&mut arena, "{\"b\":\"456\"}");

    tree::add_item_to_array(&mut arena, array, item1);
    let str1 = print(&arena, root, false);
    assert_eq!(str1.as_deref(), Some(&expected_json1[..]));

    tree::add_item_to_array(&mut arena, array, item2);
    let str2 = print(&arena, root, false);
    assert_eq!(str2.as_deref(), Some(&expected_json2[..]));

    // this should not broken list structure
    tree::delete_item_from_array(&mut arena, array, 0);
    let str3 = print(&arena, root, false);
    assert_eq!(str3.as_deref(), Some(&expected_json3[..]));

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_set_valuestring_to_object_should_not_leak_memory() {
    let mut arena = Arena::new();
    let root = parse_root(&mut arena, "{}");
    let stringvalue = b"valuestring could be changed safely";
    let reference_valuestring: Rc<[u8]> =
        Rc::from(&b"reference item should be freed by yourself"[..]);
    let short_valuestring = b"shorter valuestring";
    let long_valuestring =
        b"new valuestring which much longer than previous should be changed safely";
    let item1 = tree::create_string(&mut arena, Some(stringvalue));
    let item2 = tree::create_string_reference(&mut arena, Some(reference_valuestring.clone()));

    tree::add_item_to_object(&mut arena, root, cstr("one"), item1);
    tree::add_item_to_object(&mut arena, root, cstr("two"), item2);

    // The literals never overlap the item's buffer, so C's check is false.
    let ptr1 = valuestring_ptr(&arena, item1.unwrap());
    let target = tree::get_object_item(&arena, root, cstr("one"), false);
    let return_value = set_valuestring(&mut arena, target, short_valuestring, false);
    assert!(return_value.is_some());
    assert_eq!(
        ptr1, return_value,
        "new valuestring shorter than old should not reallocate memory"
    );
    let one = tree::get_object_item(&arena, root, cstr("one"), false).unwrap();
    assert_eq!(arena.valuestring(one), Some(&short_valuestring[..]));

    // we needn't to free the original valuestring manually
    let ptr1 = valuestring_ptr(&arena, item1.unwrap());
    let target = tree::get_object_item(&arena, root, cstr("one"), false);
    let return_value = set_valuestring(&mut arena, target, long_valuestring, false);
    assert!(return_value.is_some());
    assert_ne!(
        ptr1, return_value,
        "new valuestring longer than old should reallocate memory"
    );
    let one = tree::get_object_item(&arena, root, cstr("one"), false).unwrap();
    assert_eq!(arena.valuestring(one), Some(&long_valuestring[..]));

    let target = tree::get_object_item(&arena, root, cstr("two"), false);
    let return_value = set_valuestring(&mut arena, target, long_valuestring, false);
    assert!(
        return_value.is_none(),
        "valuestring of reference object should not be changed"
    );
    let two = tree::get_object_item(&arena, root, cstr("two"), false).unwrap();
    assert_eq!(arena.valuestring(two), Some(&reference_valuestring[..]));

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_set_bool_value_must_not_break_objects() {
    let mut arena = Arena::new();
    let mut refobj: Option<NodeId> = None;

    assert!(set_bool_value(&mut arena, refobj, true) == CJSON_INVALID);

    let bobj = tree::create_false(&mut arena);
    assert!(tree::is_false(&arena, bobj));
    assert!(set_bool_value(&mut arena, bobj, true) == CJSON_TRUE);
    assert!(tree::is_true(&arena, bobj));
    set_bool_value(&mut arena, bobj, true);
    assert!(tree::is_true(&arena, bobj));
    assert!(set_bool_value(&mut arena, bobj, false) == CJSON_FALSE);
    assert!(tree::is_false(&arena, bobj));
    set_bool_value(&mut arena, bobj, false);
    assert!(tree::is_false(&arena, bobj));

    let sobj = tree::create_string(&mut arena, cstr("test"));
    assert!(tree::is_string(&arena, sobj));
    set_bool_value(&mut arena, sobj, true);
    assert!(tree::is_string(&arena, sobj));
    set_bool_value(&mut arena, sobj, false);
    assert!(tree::is_string(&arena, sobj));

    let oobj = tree::create_object(&mut arena);
    assert!(tree::is_object(&arena, oobj));
    set_bool_value(&mut arena, oobj, true);
    assert!(tree::is_object(&arena, oobj));
    set_bool_value(&mut arena, oobj, false);
    assert!(tree::is_object(&arena, oobj));

    refobj = tree::create_string_reference(&mut arena, Some(Rc::from(&b"conststring"[..])));
    assert!(tree::is_string(&arena, refobj));
    assert!(arena.type_bits(refobj.unwrap()) & CJSON_IS_REFERENCE != 0);
    set_bool_value(&mut arena, refobj, true);
    assert!(tree::is_string(&arena, refobj));
    assert!(arena.type_bits(refobj.unwrap()) & CJSON_IS_REFERENCE != 0);
    set_bool_value(&mut arena, refobj, false);
    assert!(tree::is_string(&arena, refobj));
    assert!(arena.type_bits(refobj.unwrap()) & CJSON_IS_REFERENCE != 0);
    tree::delete(&mut arena, refobj);

    refobj = tree::create_object_reference(&mut arena, oobj);
    assert!(tree::is_object(&arena, refobj));
    assert!(arena.type_bits(refobj.unwrap()) & CJSON_IS_REFERENCE != 0);
    set_bool_value(&mut arena, refobj, true);
    assert!(tree::is_object(&arena, refobj));
    assert!(arena.type_bits(refobj.unwrap()) & CJSON_IS_REFERENCE != 0);
    set_bool_value(&mut arena, refobj, false);
    assert!(tree::is_object(&arena, refobj));
    assert!(arena.type_bits(refobj.unwrap()) & CJSON_IS_REFERENCE != 0);
    tree::delete(&mut arena, refobj);

    tree::delete(&mut arena, oobj);
    tree::delete(&mut arena, bobj);
    tree::delete(&mut arena, sobj);
}

#[test]
fn cjson_parse_big_numbers_should_not_report_error() {
    let mut arena = Arena::new();
    let valid_big_number_json_object1 = parse_root(
        &mut arena,
        "{\"a\": true, \"b\": [ null,9999999999999999999999999999999999999999999999912345678901234567]}",
    );
    let valid_big_number_json_object2 = parse_root(
        &mut arena,
        "{\"a\": true, \"b\": [ null,999999999999999999999999999999999999999999999991234567890.1234567E3]}",
    );
    let invalid_big_number_json1 =
        "{\"a\": true, \"b\": [ null,99999999999999999999999999999999999999999999999.1234567890.1234567]}";
    let invalid_big_number_json2 =
        "{\"a\": true, \"b\": [ null,99999999999999999999999999999999999999999999999E1234567890e1234567]}";

    assert!(valid_big_number_json_object1.is_some());
    assert!(valid_big_number_json_object2.is_some());
    assert!(
        parse_root(&mut arena, invalid_big_number_json1).is_none(),
        "Invalid big number JSONs should not be parsed."
    );
    assert!(
        parse_root(&mut arena, invalid_big_number_json2).is_none(),
        "Invalid big number JSONs should not be parsed."
    );

    tree::delete(&mut arena, valid_big_number_json_object1);
    tree::delete(&mut arena, valid_big_number_json_object2);
}
