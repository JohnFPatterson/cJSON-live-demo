//! Port of tests/compare_tests.c, one `#[test]` per `RUN_TEST`.

mod common_misc;

use cjson_core::compare::compare;
use cjson_core::tree::delete;
use cjson_core::{Arena, NodeStore, CJSON_NUMBER, CJSON_RAW, CJSON_STRING};
use common_misc::parse;

/// `compare_from_string` (tests/compare_tests.c:27-44).
fn compare_from_string(a: &str, b: &str, case_sensitive: bool) -> bool {
    let mut store = Arena::new();
    let a_json = parse(&mut store, a);
    assert!(a_json.is_some(), "Failed to parse a.");
    let b_json = parse(&mut store, b);
    assert!(b_json.is_some(), "Failed to parse b.");

    let result = compare(&store, a_json, b_json, case_sensitive);

    delete(&mut store, a_json);
    delete(&mut store, b_json);

    result
}

#[test]
fn cjson_compare_should_compare_null_pointer_as_not_equal() {
    let store = Arena::new();
    assert!(!compare(&store, None, None, true));
    assert!(!compare(&store, None, None, false));
}

#[test]
fn cjson_compare_should_compare_invalid_as_not_equal() {
    // `cJSON invalid[1]; memset(invalid, '\0', sizeof(invalid));`
    let mut store = Arena::new();
    let invalid = store.alloc_node().unwrap();

    assert!(!compare(&store, Some(invalid), Some(invalid), false));
    assert!(!compare(&store, Some(invalid), Some(invalid), true));
}

#[test]
fn cjson_compare_should_compare_numbers() {
    assert!(compare_from_string("1", "1", true));
    assert!(compare_from_string("1", "1", false));
    assert!(compare_from_string("0.0001", "0.0001", true));
    assert!(compare_from_string("0.0001", "0.0001", false));
    assert!(compare_from_string("1E100", "10E99", false));

    assert!(!compare_from_string("0.5E-100", "0.5E-101", false));

    assert!(!compare_from_string("1", "2", true));
    assert!(!compare_from_string("1", "2", false));
}

#[test]
fn cjson_compare_should_compare_booleans() {
    // true
    assert!(compare_from_string("true", "true", true));
    assert!(compare_from_string("true", "true", false));

    // false
    assert!(compare_from_string("false", "false", true));
    assert!(compare_from_string("false", "false", false));

    // mixed
    assert!(!compare_from_string("true", "false", true));
    assert!(!compare_from_string("true", "false", false));
    assert!(!compare_from_string("false", "true", true));
    assert!(!compare_from_string("false", "true", false));
}

#[test]
fn cjson_compare_should_compare_null() {
    assert!(compare_from_string("null", "null", true));
    assert!(compare_from_string("null", "null", false));

    assert!(!compare_from_string("null", "true", true));
    assert!(!compare_from_string("null", "true", false));
}

#[test]
fn cjson_compare_should_not_accept_invalid_types() {
    let mut store = Arena::new();
    let invalid = store.alloc_node().unwrap();

    store.set_type_bits(invalid, CJSON_NUMBER | CJSON_STRING);

    assert!(!compare(&store, Some(invalid), Some(invalid), true));
    assert!(!compare(&store, Some(invalid), Some(invalid), false));
}

#[test]
fn cjson_compare_should_compare_strings() {
    assert!(compare_from_string("\"abcdefg\"", "\"abcdefg\"", true));
    assert!(compare_from_string("\"abcdefg\"", "\"abcdefg\"", false));

    assert!(!compare_from_string("\"ABCDEFG\"", "\"abcdefg\"", true));
    assert!(!compare_from_string("\"ABCDEFG\"", "\"abcdefg\"", false));
}

#[test]
fn cjson_compare_should_compare_raw() {
    let mut store = Arena::new();
    let raw1 = parse(&mut store, "\"[true, false]\"");
    assert!(raw1.is_some());
    let raw2 = parse(&mut store, "\"[true, false]\"");
    assert!(raw2.is_some());

    store.set_type_bits(raw1.unwrap(), CJSON_RAW);
    store.set_type_bits(raw2.unwrap(), CJSON_RAW);

    assert!(compare(&store, raw1, raw2, true));
    assert!(compare(&store, raw1, raw2, false));

    delete(&mut store, raw1);
    delete(&mut store, raw2);
}

#[test]
fn cjson_compare_should_compare_arrays() {
    assert!(compare_from_string("[]", "[]", true));
    assert!(compare_from_string("[]", "[]", false));

    assert!(compare_from_string(
        "[false,true,null,42,\"string\",[],{}]",
        "[false, true, null, 42, \"string\", [], {}]",
        true
    ));
    assert!(compare_from_string(
        "[false,true,null,42,\"string\",[],{}]",
        "[false, true, null, 42, \"string\", [], {}]",
        false
    ));

    assert!(compare_from_string("[[[1], 2]]", "[[[1], 2]]", true));
    assert!(compare_from_string("[[[1], 2]]", "[[[1], 2]]", false));

    assert!(!compare_from_string(
        "[true,null,42,\"string\",[],{}]",
        "[false, true, null, 42, \"string\", [], {}]",
        true
    ));
    assert!(!compare_from_string(
        "[true,null,42,\"string\",[],{}]",
        "[false, true, null, 42, \"string\", [], {}]",
        false
    ));

    // Arrays that are a prefix of another array
    assert!(!compare_from_string("[1,2,3]", "[1,2]", true));
    assert!(!compare_from_string("[1,2,3]", "[1,2]", false));
}

#[test]
fn cjson_compare_should_compare_objects() {
    assert!(compare_from_string("{}", "{}", true));
    assert!(compare_from_string("{}", "{}", false));

    assert!(compare_from_string(
        "{\"false\": false, \"true\": true, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        "{\"true\": true, \"false\": false, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        true
    ));
    assert!(!compare_from_string(
        "{\"False\": false, \"true\": true, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        "{\"true\": true, \"false\": false, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        true
    ));
    assert!(compare_from_string(
        "{\"False\": false, \"true\": true, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        "{\"true\": true, \"false\": false, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        false
    ));
    assert!(!compare_from_string(
        "{\"Flse\": false, \"true\": true, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        "{\"true\": true, \"false\": false, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        false
    ));
    // test objects that are a subset of each other
    assert!(!compare_from_string(
        "{\"one\": 1, \"two\": 2}",
        "{\"one\": 1, \"two\": 2, \"three\": 3}",
        true
    ));
    assert!(!compare_from_string(
        "{\"one\": 1, \"two\": 2}",
        "{\"one\": 1, \"two\": 2, \"three\": 3}",
        false
    ));
}
