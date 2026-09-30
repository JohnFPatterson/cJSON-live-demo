//! Port of `tests/print_array.c`, case for case.

mod common_print;

use cjson_core::parse::internals::{parse_array, ParseBuffer};
use cjson_core::print::internals::print_array;
use cjson_core::{Arena, NodeStore};
use common_print::{noalloc_buffer, parse_content, reset};

fn assert_print_array(expected: &str, input: &str) {
    let mut formatted_buffer = noalloc_buffer(1024);
    let mut unformatted_buffer = noalloc_buffer(1024);

    let content = parse_content(input);
    let mut parsebuffer = ParseBuffer::new(&content);

    let mut arena = Arena::new();
    let item = arena.alloc_node().unwrap();
    assert!(
        parse_array(&mut arena, item, &mut parsebuffer),
        "Failed to parse array."
    );

    unformatted_buffer.format = false;
    assert!(
        print_array(&arena, item, &mut unformatted_buffer),
        "Failed to print unformatted string."
    );
    assert_eq!(
        String::from_utf8_lossy(unformatted_buffer.c_str()),
        input,
        "Unformatted array is not correct."
    );

    formatted_buffer.format = true;
    assert!(
        print_array(&arena, item, &mut formatted_buffer),
        "Failed to print formatted string."
    );
    assert_eq!(
        String::from_utf8_lossy(formatted_buffer.c_str()),
        expected,
        "Formatted array is not correct."
    );

    reset(&mut arena, item);
}

#[test]
fn print_array_should_print_empty_arrays() {
    assert_print_array("[]", "[]");
}

#[test]
fn print_array_should_print_arrays_with_one_element() {
    assert_print_array("[1]", "[1]");
    assert_print_array("[\"hello!\"]", "[\"hello!\"]");
    assert_print_array("[[]]", "[[]]");
    assert_print_array("[null]", "[null]");
}

#[test]
fn print_array_should_print_arrays_with_multiple_elements() {
    assert_print_array("[1, 2, 3]", "[1,2,3]");
    assert_print_array(
        "[1, null, true, false, [], \"hello\", {\n\t}]",
        "[1,null,true,false,[],\"hello\",{}]",
    );
}
