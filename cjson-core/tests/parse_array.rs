//! Port of tests/parse_array.c.

mod common_parse;

use cjson_core::parse::internals::{parse_array, ParseBuffer};
use cjson_core::{
    Arena, NodeId, NodeStore, CJSON_ARRAY, CJSON_FALSE, CJSON_NULL, CJSON_NUMBER, CJSON_OBJECT,
    CJSON_STRING, CJSON_TRUE,
};
use common_parse::*;

fn assert_is_array(arena: &Arena, array_item: NodeId) {
    // TEST_ASSERT_NOT_NULL(array_item) holds by construction (NodeId).
    assert_not_in_list(arena, array_item);
    assert_has_type(arena, array_item, CJSON_ARRAY);
    assert_has_no_reference(arena, array_item);
    assert_has_no_const_string(arena, array_item);
    assert_has_no_valuestring(arena, array_item);
    assert_has_no_string(arena, array_item);
}

fn assert_not_array(arena: &mut Arena, item: NodeId, json: &[u8]) {
    let content = c_string(json);
    let mut buffer = ParseBuffer::new(&content);

    assert!(!parse_array(arena, item, &mut buffer));
    assert_is_invalid(arena, item);
}

fn assert_parse_array(arena: &mut Arena, item: NodeId, json: &[u8]) {
    let content = c_string(json);
    let mut buffer = ParseBuffer::new(&content);

    assert!(parse_array(arena, item, &mut buffer));
    assert_is_array(arena, item);
}

fn child(arena: &Arena, item: NodeId) -> NodeId {
    arena.child(item).expect("item->child")
}

fn next(arena: &Arena, item: NodeId) -> NodeId {
    arena.next(item).expect("item->next")
}

#[test]
fn parse_array_should_parse_empty_arrays() {
    let (mut a, item) = new_item();
    assert_parse_array(&mut a, item, b"[]");
    assert_has_no_child(&a, item);

    assert_parse_array(&mut a, item, b"[\n\t]");
    assert_has_no_child(&a, item);
}

#[test]
fn parse_array_should_parse_arrays_with_one_element() {
    let (mut a, item) = new_item();

    assert_parse_array(&mut a, item, b"[1]");
    assert_has_child(&a, item);
    assert_has_type(&a, child(&a, item), CJSON_NUMBER);
    reset(&mut a, item);

    assert_parse_array(&mut a, item, b"[\"hello!\"]");
    assert_has_child(&a, item);
    assert_has_type(&a, child(&a, item), CJSON_STRING);
    assert_eq!(Some(&b"hello!"[..]), a.valuestring(child(&a, item)));
    reset(&mut a, item);

    assert_parse_array(&mut a, item, b"[[]]");
    assert_has_child(&a, item);
    assert!(a.child(item).is_some());
    assert_has_type(&a, child(&a, item), CJSON_ARRAY);
    assert_has_no_child(&a, child(&a, item));
    reset(&mut a, item);

    assert_parse_array(&mut a, item, b"[null]");
    assert_has_child(&a, item);
    assert_has_type(&a, child(&a, item), CJSON_NULL);
    reset(&mut a, item);
}

#[test]
fn parse_array_should_parse_arrays_with_multiple_elements() {
    let (mut a, item) = new_item();

    assert_parse_array(&mut a, item, b"[1\t,\n2, 3]");
    assert_has_child(&a, item);
    let first = child(&a, item);
    assert!(a.next(first).is_some());
    assert!(a.next(next(&a, first)).is_some());
    assert!(a.next(next(&a, next(&a, first))).is_none());
    assert_has_type(&a, first, CJSON_NUMBER);
    assert_has_type(&a, next(&a, first), CJSON_NUMBER);
    assert_has_type(&a, next(&a, next(&a, first)), CJSON_NUMBER);
    reset(&mut a, item);

    {
        let expected_types: [i32; 7] = [
            CJSON_NUMBER,
            CJSON_NULL,
            CJSON_TRUE,
            CJSON_FALSE,
            CJSON_ARRAY,
            CJSON_STRING,
            CJSON_OBJECT,
        ];
        assert_parse_array(&mut a, item, b"[1, null, true, false, [], \"hello\", {}]");

        let mut node = a.child(item);
        let mut i = 0usize;
        while i < expected_types.len() {
            let Some(n) = node else { break };
            assert_eq!(expected_types[i], a.type_bits(n) & 0xFF);
            i += 1;
            node = a.next(n);
        }
        assert_eq!(i, 7);
        reset(&mut a, item);
    }
}

#[test]
fn parse_array_should_not_parse_non_arrays() {
    let (mut a, item) = new_item();
    assert_not_array(&mut a, item, b"");
    assert_not_array(&mut a, item, b"[");
    assert_not_array(&mut a, item, b"]");
    assert_not_array(&mut a, item, b"{\"hello\":[]}");
    assert_not_array(&mut a, item, b"42");
    assert_not_array(&mut a, item, b"3.14");
    assert_not_array(&mut a, item, b"\"[]hello world!\n\"");
}
