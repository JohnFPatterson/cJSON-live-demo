//! Port of tests/cjson_add.c, one `#[test]` per `RUN_TEST`.
//!
//! `cJSON_InitHooks(&failing_hooks)` (whose `malloc_fn` always returns NULL)
//! is modeled with `Arena::set_alloc_budget(Some(0))`, and
//! `cJSON_InitHooks(NULL)` with `set_alloc_budget(None)`.

mod common_tree;

use cjson_core::tree;
use cjson_core::{
    Arena, NodeStore, CJSON_ARRAY, CJSON_FALSE, CJSON_NULL, CJSON_NUMBER, CJSON_OBJECT, CJSON_RAW,
    CJSON_STRING, CJSON_TRUE,
};
use common_tree::{cstr, init_hooks};

#[test]
fn cjson_add_null_should_add_null() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    tree::add_null_to_object(&mut arena, root, cstr("null"));

    let null = tree::get_object_item(&arena, root, cstr("null"), true);
    assert!(null.is_some());
    assert_eq!(arena.type_bits(null.unwrap()), CJSON_NULL);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_null_should_fail_with_null_pointers() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    assert!(tree::add_null_to_object(&mut arena, None, cstr("null")).is_none());
    assert!(tree::add_null_to_object(&mut arena, root, None).is_none());

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_null_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    init_hooks(&mut arena, true);

    assert!(tree::add_null_to_object(&mut arena, root, cstr("null")).is_none());

    init_hooks(&mut arena, false);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_true_should_add_true() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    tree::add_true_to_object(&mut arena, root, cstr("true"));

    let true_item = tree::get_object_item(&arena, root, cstr("true"), true);
    assert!(true_item.is_some());
    assert_eq!(arena.type_bits(true_item.unwrap()), CJSON_TRUE);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_true_should_fail_with_null_pointers() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    assert!(tree::add_true_to_object(&mut arena, None, cstr("true")).is_none());
    assert!(tree::add_true_to_object(&mut arena, root, None).is_none());

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_true_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    init_hooks(&mut arena, true);

    assert!(tree::add_true_to_object(&mut arena, root, cstr("true")).is_none());

    init_hooks(&mut arena, false);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_create_int_array_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let numbers: [i32; 3] = [1, 2, 3];

    init_hooks(&mut arena, true);

    assert!(tree::create_int_array(&mut arena, Some(&numbers)).is_none());

    init_hooks(&mut arena, false);
}

#[test]
fn cjson_create_float_array_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let numbers: [f32; 3] = [1.0, 2.0, 3.0];

    init_hooks(&mut arena, true);

    assert!(tree::create_float_array(&mut arena, Some(&numbers)).is_none());

    init_hooks(&mut arena, false);
}

#[test]
fn cjson_create_double_array_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let numbers: [f64; 3] = [1.0, 2.0, 3.0];

    init_hooks(&mut arena, true);

    assert!(tree::create_double_array(&mut arena, Some(&numbers)).is_none());

    init_hooks(&mut arena, false);
}

#[test]
fn cjson_create_string_array_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let strings: [Option<&[u8]>; 3] = [cstr("1"), cstr("2"), cstr("3")];

    init_hooks(&mut arena, true);

    assert!(tree::create_string_array(&mut arena, Some(&strings)).is_none());

    init_hooks(&mut arena, false);
}

#[test]
fn cjson_add_false_should_add_false() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    tree::add_false_to_object(&mut arena, root, cstr("false"));

    let false_item = tree::get_object_item(&arena, root, cstr("false"), true);
    assert!(false_item.is_some());
    assert_eq!(arena.type_bits(false_item.unwrap()), CJSON_FALSE);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_false_should_fail_with_null_pointers() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    assert!(tree::add_false_to_object(&mut arena, None, cstr("false")).is_none());
    assert!(tree::add_false_to_object(&mut arena, root, None).is_none());

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_false_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    init_hooks(&mut arena, true);

    assert!(tree::add_false_to_object(&mut arena, root, cstr("false")).is_none());

    init_hooks(&mut arena, false);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_bool_should_add_bool() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    // true
    tree::add_bool_to_object(&mut arena, root, cstr("true"), true);
    let true_item = tree::get_object_item(&arena, root, cstr("true"), true);
    assert!(true_item.is_some());
    assert_eq!(arena.type_bits(true_item.unwrap()), CJSON_TRUE);

    // false
    tree::add_bool_to_object(&mut arena, root, cstr("false"), false);
    let false_item = tree::get_object_item(&arena, root, cstr("false"), true);
    assert!(false_item.is_some());
    assert_eq!(arena.type_bits(false_item.unwrap()), CJSON_FALSE);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_bool_should_fail_with_null_pointers() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    assert!(tree::add_bool_to_object(&mut arena, None, cstr("false"), false).is_none());
    assert!(tree::add_bool_to_object(&mut arena, root, None, false).is_none());

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_bool_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    init_hooks(&mut arena, true);

    assert!(tree::add_bool_to_object(&mut arena, root, cstr("false"), false).is_none());

    init_hooks(&mut arena, false);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_number_should_add_number() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    tree::add_number_to_object(&mut arena, root, cstr("number"), 42.0);

    let number = tree::get_object_item(&arena, root, cstr("number"), true);
    assert!(number.is_some());
    let number = number.unwrap();

    assert_eq!(arena.type_bits(number), CJSON_NUMBER);
    assert_eq!(arena.valuedouble(number), 42.0);
    assert_eq!(arena.valueint(number), 42);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_number_should_fail_with_null_pointers() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    assert!(tree::add_number_to_object(&mut arena, None, cstr("number"), 42.0).is_none());
    assert!(tree::add_number_to_object(&mut arena, root, None, 42.0).is_none());

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_number_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    init_hooks(&mut arena, true);

    assert!(tree::add_number_to_object(&mut arena, root, cstr("number"), 42.0).is_none());

    init_hooks(&mut arena, false);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_string_should_add_string() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    tree::add_string_to_object(&mut arena, root, cstr("string"), cstr("Hello World!"));

    let string = tree::get_object_item(&arena, root, cstr("string"), true);
    assert!(string.is_some());
    let string = string.unwrap();
    assert_eq!(arena.type_bits(string), CJSON_STRING);
    assert_eq!(arena.valuestring(string), Some(&b"Hello World!"[..]));

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_string_should_fail_with_null_pointers() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    assert!(tree::add_string_to_object(&mut arena, None, cstr("string"), cstr("string")).is_none());
    assert!(tree::add_string_to_object(&mut arena, root, None, cstr("string")).is_none());

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_string_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    init_hooks(&mut arena, true);

    assert!(tree::add_string_to_object(&mut arena, root, cstr("string"), cstr("string")).is_none());

    init_hooks(&mut arena, false);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_raw_should_add_raw() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    tree::add_raw_to_object(&mut arena, root, cstr("raw"), cstr("{}"));

    let raw = tree::get_object_item(&arena, root, cstr("raw"), true);
    assert!(raw.is_some());
    let raw = raw.unwrap();
    assert_eq!(arena.type_bits(raw), CJSON_RAW);
    assert_eq!(arena.valuestring(raw), Some(&b"{}"[..]));

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_raw_should_fail_with_null_pointers() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    assert!(tree::add_raw_to_object(&mut arena, None, cstr("raw"), cstr("{}")).is_none());
    assert!(tree::add_raw_to_object(&mut arena, root, None, cstr("{}")).is_none());

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_raw_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    init_hooks(&mut arena, true);

    assert!(tree::add_raw_to_object(&mut arena, root, cstr("raw"), cstr("{}")).is_none());

    init_hooks(&mut arena, false);

    tree::delete(&mut arena, root);
}

// The name keeps the C test's `cJSON_` prefix so it matches its RUN_TEST
// entry one-to-one; the lint only concerns identifier style.
#[allow(non_snake_case)]
#[test]
fn cJSON_add_object_should_add_object() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    tree::add_object_to_object(&mut arena, root, cstr("object"));
    let object = tree::get_object_item(&arena, root, cstr("object"), true);
    assert!(object.is_some());
    assert_eq!(arena.type_bits(object.unwrap()), CJSON_OBJECT);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_object_should_fail_with_null_pointers() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    assert!(tree::add_object_to_object(&mut arena, None, cstr("object")).is_none());
    assert!(tree::add_object_to_object(&mut arena, root, None).is_none());

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_object_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    init_hooks(&mut arena, true);

    assert!(tree::add_object_to_object(&mut arena, root, cstr("object")).is_none());

    init_hooks(&mut arena, false);

    tree::delete(&mut arena, root);
}

// The name keeps the C test's `cJSON_` prefix so it matches its RUN_TEST
// entry one-to-one; the lint only concerns identifier style.
#[allow(non_snake_case)]
#[test]
fn cJSON_add_array_should_add_array() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    tree::add_array_to_object(&mut arena, root, cstr("array"));
    let array = tree::get_object_item(&arena, root, cstr("array"), true);
    assert!(array.is_some());
    assert_eq!(arena.type_bits(array.unwrap()), CJSON_ARRAY);

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_array_should_fail_with_null_pointers() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    assert!(tree::add_array_to_object(&mut arena, None, cstr("array")).is_none());
    assert!(tree::add_array_to_object(&mut arena, root, None).is_none());

    tree::delete(&mut arena, root);
}

#[test]
fn cjson_add_array_should_fail_on_allocation_failure() {
    let mut arena = Arena::new();
    let root = tree::create_object(&mut arena);

    init_hooks(&mut arena, true);

    assert!(tree::add_array_to_object(&mut arena, root, cstr("array")).is_none());

    init_hooks(&mut arena, false);

    tree::delete(&mut arena, root);
}
