//! Port of `tests/print_value.c`, case for case.

mod common_print;

use cjson_core::parse::internals::{parse_value, ParseBuffer};
use cjson_core::print::internals::print_value;
use cjson_core::{Arena, NodeStore};
use common_print::{noalloc_buffer, parse_content, reset};

fn assert_print_value(input: &str) {
    let mut buffer = noalloc_buffer(1024);

    let content = parse_content(input);
    let mut parsebuffer = ParseBuffer::new(&content);

    let mut arena = Arena::new();
    let item = arena.alloc_node().unwrap();

    assert!(
        parse_value(&mut arena, item, &mut parsebuffer),
        "Failed to parse value."
    );

    assert!(
        print_value(&arena, item, &mut buffer),
        "Failed to print value."
    );
    assert_eq!(
        String::from_utf8_lossy(buffer.c_str()),
        input,
        "Printed value is not as expected."
    );

    reset(&mut arena, item);
}

#[test]
fn print_value_should_print_null() {
    assert_print_value("null");
}

#[test]
fn print_value_should_print_true() {
    assert_print_value("true");
}

#[test]
fn print_value_should_print_false() {
    assert_print_value("false");
}

#[test]
fn print_value_should_print_number() {
    assert_print_value("1.5");
}

#[test]
fn print_value_should_print_string() {
    assert_print_value("\"\"");
    assert_print_value("\"hello\"");
}

#[test]
fn print_value_should_print_array() {
    assert_print_value("[]");
}

#[test]
fn print_value_should_print_object() {
    assert_print_value("{}");
}
