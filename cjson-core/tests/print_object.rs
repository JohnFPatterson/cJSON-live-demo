//! Port of `tests/print_object.c`, case for case.

mod common_print;

use cjson_core::parse::internals::{parse_object, ParseBuffer};
use cjson_core::print::internals::print_object;
use cjson_core::{Arena, NodeStore};
use common_print::{noalloc_buffer, parse_content, reset};

fn assert_print_object(expected: &str, input: &str) {
    let mut formatted_buffer = noalloc_buffer(1024);
    let mut unformatted_buffer = noalloc_buffer(1024);

    let content = parse_content(input);
    let mut parsebuffer = ParseBuffer::new(&content);

    let mut arena = Arena::new();
    let item = arena.alloc_node().unwrap();
    assert!(
        parse_object(&mut arena, item, &mut parsebuffer),
        "Failed to parse object."
    );

    unformatted_buffer.format = false;
    assert!(
        print_object(&arena, item, &mut unformatted_buffer),
        "Failed to print unformatted string."
    );
    assert_eq!(
        String::from_utf8_lossy(unformatted_buffer.c_str()),
        input,
        "Unformatted object is not correct."
    );

    formatted_buffer.format = true;
    assert!(
        print_object(&arena, item, &mut formatted_buffer),
        "Failed to print formatted string."
    );
    assert_eq!(
        String::from_utf8_lossy(formatted_buffer.c_str()),
        expected,
        "Formatted object is not correct."
    );

    reset(&mut arena, item);
}

#[test]
fn print_object_should_print_empty_objects() {
    assert_print_object("{\n}", "{}");
}

#[test]
fn print_object_should_print_objects_with_one_element() {
    assert_print_object("{\n\t\"one\":\t1\n}", "{\"one\":1}");
    assert_print_object("{\n\t\"hello\":\t\"world!\"\n}", "{\"hello\":\"world!\"}");
    assert_print_object("{\n\t\"array\":\t[]\n}", "{\"array\":[]}");
    assert_print_object("{\n\t\"null\":\tnull\n}", "{\"null\":null}");
}

#[test]
fn print_object_should_print_objects_with_multiple_elements() {
    assert_print_object(
        "{\n\t\"one\":\t1,\n\t\"two\":\t2,\n\t\"three\":\t3\n}",
        "{\"one\":1,\"two\":2,\"three\":3}",
    );
    assert_print_object(
        "{\n\t\"one\":\t1,\n\t\"NULL\":\tnull,\n\t\"TRUE\":\ttrue,\n\t\"FALSE\":\tfalse,\n\t\"array\":\t[],\n\t\"world\":\t\"hello\",\n\t\"object\":\t{\n\t}\n}",
        "{\"one\":1,\"NULL\":null,\"TRUE\":true,\"FALSE\":false,\"array\":[],\"world\":\"hello\",\"object\":{}}",
    );
}
