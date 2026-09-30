//! Port of tests/parse_object.c.

mod common_parse;

use cjson_core::parse::internals::{parse_object, ParseBuffer};
use cjson_core::{
    Arena, NodeId, NodeStore, CJSON_ARRAY, CJSON_FALSE, CJSON_NULL, CJSON_NUMBER, CJSON_OBJECT,
    CJSON_STRING, CJSON_TRUE,
};
use common_parse::*;

fn assert_is_object(arena: &Arena, object_item: NodeId) {
    // TEST_ASSERT_NOT_NULL(object_item) holds by construction (NodeId).
    assert_not_in_list(arena, object_item);
    assert_has_type(arena, object_item, CJSON_OBJECT);
    assert_has_no_reference(arena, object_item);
    assert_has_no_const_string(arena, object_item);
    assert_has_no_valuestring(arena, object_item);
    assert_has_no_string(arena, object_item);
}

fn assert_is_child(arena: &Arena, child_item: Option<NodeId>, name: &str, item_type: i32) {
    let child_item = child_item.expect("Child item is NULL.");
    assert!(
        arena.string(child_item).is_some(),
        "Child item doesn't have a name."
    );
    assert_eq!(
        Some(name.as_bytes()),
        arena.string(child_item),
        "Child item has the wrong name."
    );
    assert_eq!(item_type, arena.type_bits(child_item) & 0xFF);
}

fn assert_not_object(arena: &mut Arena, item: NodeId, json: &[u8]) {
    let content = c_string(json);
    let mut parsebuffer = ParseBuffer::new(&content);

    assert!(!parse_object(arena, item, &mut parsebuffer));
    assert_is_invalid(arena, item);
    reset(arena, item);
}

fn assert_parse_object(arena: &mut Arena, item: NodeId, json: &[u8]) {
    let content = c_string(json);
    let mut parsebuffer = ParseBuffer::new(&content);

    assert!(parse_object(arena, item, &mut parsebuffer));
    assert_is_object(arena, item);
}

fn next(arena: &Arena, item: Option<NodeId>) -> Option<NodeId> {
    item.and_then(|id| arena.next(id))
}

#[test]
fn parse_object_should_parse_empty_objects() {
    let (mut a, item) = new_item();
    assert_parse_object(&mut a, item, b"{}");
    assert_has_no_child(&a, item);
    reset(&mut a, item);

    assert_parse_object(&mut a, item, b"{\n\t}");
    assert_has_no_child(&a, item);
    reset(&mut a, item);
}

#[test]
fn parse_object_should_parse_objects_with_one_element() {
    let (mut a, item) = new_item();

    assert_parse_object(&mut a, item, b"{\"one\":1}");
    assert_is_child(&a, a.child(item), "one", CJSON_NUMBER);
    reset(&mut a, item);

    assert_parse_object(&mut a, item, b"{\"hello\":\"world!\"}");
    assert_is_child(&a, a.child(item), "hello", CJSON_STRING);
    reset(&mut a, item);

    assert_parse_object(&mut a, item, b"{\"array\":[]}");
    assert_is_child(&a, a.child(item), "array", CJSON_ARRAY);
    reset(&mut a, item);

    assert_parse_object(&mut a, item, b"{\"null\":null}");
    assert_is_child(&a, a.child(item), "null", CJSON_NULL);
    reset(&mut a, item);
}

#[test]
fn parse_object_should_parse_objects_with_multiple_elements() {
    let (mut a, item) = new_item();

    assert_parse_object(&mut a, item, b"{\"one\":1\t,\t\"two\"\n:2, \"three\":3}");
    let first = a.child(item);
    assert_is_child(&a, first, "one", CJSON_NUMBER);
    assert_is_child(&a, next(&a, first), "two", CJSON_NUMBER);
    assert_is_child(&a, next(&a, next(&a, first)), "three", CJSON_NUMBER);
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
        let expected_names: [&str; 7] =
            ["one", "NULL", "TRUE", "FALSE", "array", "world", "object"];
        assert_parse_object(
            &mut a,
            item,
            b"{\"one\":1, \"NULL\":null, \"TRUE\":true, \"FALSE\":false, \"array\":[], \"world\":\"hello\", \"object\":{}}",
        );

        let mut node = a.child(item);
        let mut i = 0usize;
        while i < expected_types.len() {
            let Some(n) = node else { break };
            assert_is_child(&a, Some(n), expected_names[i], expected_types[i]);
            i += 1;
            node = a.next(n);
        }
        assert_eq!(i, 7);
        reset(&mut a, item);
    }
}

#[test]
fn parse_object_should_not_parse_non_objects() {
    let (mut a, item) = new_item();
    assert_not_object(&mut a, item, b"");
    assert_not_object(&mut a, item, b"{");
    assert_not_object(&mut a, item, b"}");
    assert_not_object(&mut a, item, b"[\"hello\",{}]");
    assert_not_object(&mut a, item, b"42");
    assert_not_object(&mut a, item, b"3.14");
    assert_not_object(&mut a, item, b"\"{}hello world!\n\"");
}
