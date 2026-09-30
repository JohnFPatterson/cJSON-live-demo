//! Port of tests/parse_value.c.

mod common_parse;

use cjson_core::parse::internals::{parse_value, ParseBuffer};
use cjson_core::{
    Arena, NodeId, CJSON_ARRAY, CJSON_FALSE, CJSON_NULL, CJSON_NUMBER, CJSON_OBJECT, CJSON_STRING,
    CJSON_TRUE,
};
use common_parse::*;

fn assert_is_value(arena: &Arena, value_item: NodeId, item_type: i32) {
    // TEST_ASSERT_NOT_NULL(value_item) holds by construction (NodeId).
    assert_not_in_list(arena, value_item);
    assert_has_type(arena, value_item, item_type);
    assert_has_no_reference(arena, value_item);
    assert_has_no_const_string(arena, value_item);
    assert_has_no_string(arena, value_item);
}

fn assert_parse_value(arena: &mut Arena, item: NodeId, string: &[u8], item_type: i32) {
    let content = c_string(string);
    let mut buffer = ParseBuffer::new(&content);

    assert!(parse_value(arena, item, &mut buffer));
    assert_is_value(arena, item, item_type);
}

#[test]
fn parse_value_should_parse_null() {
    let (mut a, item) = new_item();
    assert_parse_value(&mut a, item, b"null", CJSON_NULL);
    reset(&mut a, item);
}

#[test]
fn parse_value_should_parse_true() {
    let (mut a, item) = new_item();
    assert_parse_value(&mut a, item, b"true", CJSON_TRUE);
    reset(&mut a, item);
}

#[test]
fn parse_value_should_parse_false() {
    let (mut a, item) = new_item();
    assert_parse_value(&mut a, item, b"false", CJSON_FALSE);
    reset(&mut a, item);
}

#[test]
fn parse_value_should_parse_number() {
    let (mut a, item) = new_item();
    assert_parse_value(&mut a, item, b"1.5", CJSON_NUMBER);
    reset(&mut a, item);
}

#[test]
fn parse_value_should_parse_string() {
    let (mut a, item) = new_item();
    assert_parse_value(&mut a, item, b"\"\"", CJSON_STRING);
    reset(&mut a, item);
    assert_parse_value(&mut a, item, b"\"hello\"", CJSON_STRING);
    reset(&mut a, item);
}

#[test]
fn parse_value_should_parse_array() {
    let (mut a, item) = new_item();
    assert_parse_value(&mut a, item, b"[]", CJSON_ARRAY);
    reset(&mut a, item);
}

#[test]
fn parse_value_should_parse_object() {
    let (mut a, item) = new_item();
    assert_parse_value(&mut a, item, b"{}", CJSON_OBJECT);
    reset(&mut a, item);
}
