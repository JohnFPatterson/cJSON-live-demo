//! Port of tests/parse_number.c.

mod common_parse;

use cjson_core::parse::internals::{parse_number, ParseBuffer};
use cjson_core::{Arena, NodeId, NodeStore, CJSON_NUMBER};
use common_parse::*;

fn assert_is_number(arena: &Arena, number_item: NodeId) {
    // TEST_ASSERT_NOT_NULL(number_item) holds by construction (NodeId).
    assert_not_in_list(arena, number_item);
    assert_has_no_child(arena, number_item);
    assert_has_type(arena, number_item, CJSON_NUMBER);
    assert_has_no_reference(arena, number_item);
    assert_has_no_const_string(arena, number_item);
    assert_has_no_valuestring(arena, number_item);
    assert_has_no_string(arena, number_item);
}

fn assert_parse_number(arena: &mut Arena, item: NodeId, string: &str, integer: i32, real: f64) {
    let content = c_string(string.as_bytes());
    let mut buffer = ParseBuffer::new(&content);

    assert!(parse_number(arena, item, &mut buffer));
    assert_is_number(arena, item);
    assert_eq!(integer, arena.valueint(item));
    assert_equal_double(real, arena.valuedouble(item));
}

fn assert_parse_big_number(arena: &mut Arena, item: NodeId, string: &str) {
    let content = c_string(string.as_bytes());
    let mut buffer = ParseBuffer::new(&content);

    assert!(parse_number(arena, item, &mut buffer));
    assert_is_number(arena, item);
}

#[test]
fn parse_number_should_parse_zero() {
    let (mut a, item) = new_item();
    assert_parse_number(&mut a, item, "0", 0, 0.0);
    assert_parse_number(&mut a, item, "0.0", 0, 0.0);
    assert_parse_number(&mut a, item, "-0", 0, -0.0);
}

#[test]
fn parse_number_should_parse_negative_integers() {
    let (mut a, item) = new_item();
    assert_parse_number(&mut a, item, "-1", -1, -1.0);
    assert_parse_number(&mut a, item, "-32768", -32768, -32768.0);
    assert_parse_number(
        &mut a,
        item,
        "-2147483648",
        -2147483648.0_f64 as i32,
        -2147483648.0,
    );
}

#[test]
fn parse_number_should_parse_positive_integers() {
    let (mut a, item) = new_item();
    assert_parse_number(&mut a, item, "1", 1, 1.0);
    assert_parse_number(&mut a, item, "32767", 32767, 32767.0);
    assert_parse_number(
        &mut a,
        item,
        "2147483647",
        2147483647.0_f64 as i32,
        2147483647.0,
    );
}

#[test]
fn parse_number_should_parse_positive_reals() {
    let (mut a, item) = new_item();
    assert_parse_number(&mut a, item, "0.001", 0, 0.001);
    assert_parse_number(&mut a, item, "10e-10", 0, 10e-10);
    assert_parse_number(&mut a, item, "10E-10", 0, 10e-10);
    assert_parse_number(&mut a, item, "10e10", i32::MAX, 10e10);
    assert_parse_number(&mut a, item, "123e+127", i32::MAX, 123e127);
    assert_parse_number(&mut a, item, "123e-128", 0, 123e-128);
}

#[test]
fn parse_number_should_parse_negative_reals() {
    let (mut a, item) = new_item();
    assert_parse_number(&mut a, item, "-0.001", 0, -0.001);
    assert_parse_number(&mut a, item, "-10e-10", 0, -10e-10);
    assert_parse_number(&mut a, item, "-10E-10", 0, -10e-10);
    assert_parse_number(&mut a, item, "-10e20", i32::MIN, -10e20);
    assert_parse_number(&mut a, item, "-123e+127", i32::MIN, -123e127);
    assert_parse_number(&mut a, item, "-123e-128", 0, -123e-128);
}

#[test]
fn parse_number_should_parse_big_numbers() {
    let (mut a, item) = new_item();
    assert_parse_big_number(
        &mut a,
        item,
        "9999999999999999999999999999999999999999999999912345678901234567",
    );
    assert_parse_big_number(
        &mut a,
        item,
        "9999999999999999999999999999999999999999999999912345678901234567E10",
    );
    assert_parse_big_number(
        &mut a,
        item,
        "999999999999999999999999999999999999999999999991234567890.1234567",
    );
}
