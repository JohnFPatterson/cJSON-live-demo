//! Port of tests/parse_string.c.

mod common_parse;

use cjson_core::parse::internals::{parse_string, ParseBuffer};
use cjson_core::{Arena, NodeId, NodeStore, CJSON_STRING};
use common_parse::*;

fn assert_is_string(arena: &Arena, string_item: NodeId) {
    // TEST_ASSERT_NOT_NULL(string_item) holds by construction (NodeId).
    assert_not_in_list(arena, string_item);
    assert_has_no_child(arena, string_item);
    assert_has_type(arena, string_item, CJSON_STRING);
    assert_has_no_reference(arena, string_item);
    assert_has_no_const_string(arena, string_item);
    assert_has_valuestring(arena, string_item);
    assert_has_no_string(arena, string_item);
}

fn assert_parse_string(arena: &mut Arena, item: NodeId, string: &[u8], expected: &[u8]) {
    let content = c_string(string);
    let mut buffer = ParseBuffer::new(&content);

    assert!(
        parse_string(arena, item, &mut buffer),
        "Couldn't parse string."
    );
    assert_is_string(arena, item);
    assert_eq!(
        Some(expected),
        arena.valuestring(item),
        "The parsed result isn't as expected."
    );
    // global_hooks.deallocate(item->valuestring); item->valuestring = NULL;
    arena.free_valuestring(item);
}

fn assert_not_parse_string(arena: &mut Arena, item: NodeId, string: &[u8]) {
    let content = c_string(string);
    let mut buffer = ParseBuffer::new(&content);

    assert!(
        !parse_string(arena, item, &mut buffer),
        "Malformed string should not be accepted."
    );
    assert_is_invalid(arena, item);
}

#[test]
fn parse_string_should_parse_strings() {
    let (mut a, item) = new_item();
    assert_parse_string(&mut a, item, b"\"\"", b"");
    assert_parse_string(
        &mut a,
        item,
        b"\" !\\\"#$%&'()*+,-./\\/0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_'abcdefghijklmnopqrstuvwxyz{|}~\"",
        b" !\"#$%&'()*+,-.//0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_'abcdefghijklmnopqrstuvwxyz{|}~",
    );
    assert_parse_string(
        &mut a,
        item,
        b"\"\\\"\\\\\\/\\b\\f\\n\\r\\t\\u20AC\\u732b\"",
        "\"\\/\u{8}\u{c}\n\r\t€猫".as_bytes(),
    );
    reset(&mut a, item);
    assert_parse_string(&mut a, item, b"\"\x08\x0c\n\r\t\"", b"\x08\x0c\n\r\t");
    reset(&mut a, item);
}

#[test]
fn parse_string_should_parse_utf16_surrogate_pairs() {
    let (mut a, item) = new_item();
    assert_parse_string(&mut a, item, b"\"\\uD83D\\udc31\"", "🐱".as_bytes());
    reset(&mut a, item);
}

#[test]
fn parse_string_should_not_parse_non_strings() {
    let (mut a, item) = new_item();
    assert_not_parse_string(&mut a, item, b"this\" is not a string\"");
    reset(&mut a, item);
    assert_not_parse_string(&mut a, item, b"");
    reset(&mut a, item);
}

#[test]
fn parse_string_should_not_parse_invalid_backslash() {
    let (mut a, item) = new_item();
    assert_not_parse_string(&mut a, item, b"Abcdef\\123");
    reset(&mut a, item);
    assert_not_parse_string(&mut a, item, b"Abcdef\\e23");
    reset(&mut a, item);
}

#[test]
fn parse_string_should_not_overflow_with_closing_backslash() {
    let (mut a, item) = new_item();
    assert_not_parse_string(&mut a, item, b"\"000000000000000000\\");
    reset(&mut a, item);
}

#[test]
fn parse_string_should_parse_bug_94() {
    let (mut a, item) = new_item();
    let string: &[u8] =
        b"\"~!@\\\\#$%^&*()\\\\\\\\-\\\\+{}[]:\\\\;\\\\\\\"\\\\<\\\\>?/.,DC=ad,DC=com\"";
    assert_parse_string(
        &mut a,
        item,
        string,
        b"~!@\\#$%^&*()\\\\-\\+{}[]:\\;\\\"\\<\\>?/.,DC=ad,DC=com",
    );
    reset(&mut a, item);
}
