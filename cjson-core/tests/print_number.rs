//! Port of `tests/print_number.c`, case for case.

use cjson_core::print::internals::{print_number, PrintBuffer};
use cjson_core::{tree, Arena, NodeStore};

/// The C helper's MinGW / old MSVC workaround: removes a third exponent
/// digit ("1e-009" -> "1e-09"). A no-op for the two-digit exponents C99
/// printf (and this port) produce; kept so the assertion matches C.
fn strip_extra_exponent_zero(new_buffer: &mut [u8; 26]) {
    let mut i = 0usize;
    while i < new_buffer.len() {
        if i > 3 && new_buffer[i] == b'0' {
            let minus =
                new_buffer[i - 3] == b'e' && new_buffer[i - 2] == b'-' && new_buffer[i] == b'0';
            let plus = new_buffer[i - 2] == b'e' && new_buffer[i - 1] == b'+';
            if minus || plus {
                while i < new_buffer.len() && new_buffer[i] != 0 {
                    new_buffer[i] = new_buffer.get(i + 1).copied().unwrap_or(0);
                    i += 1;
                }
            }
        }
        i += 1;
    }
}

fn assert_print_number(expected: &str, input: f64) {
    // C: buffer.length = sizeof(printed) (1024) but buffer.buffer points at
    // the zeroed 26-byte new_buffer; noalloc = true.
    let mut buffer = PrintBuffer::new(1024);
    buffer.noalloc = true;

    let mut arena = Arena::new();
    let item = arena.alloc_node().unwrap();
    tree::set_number_helper(&mut arena, Some(item), input);
    assert!(
        print_number(&arena, item, &mut buffer),
        "Failed to print number."
    );

    let mut new_buffer = [0u8; 26];
    let written = buffer.buffer.as_deref().unwrap();
    assert!(
        written.len() <= new_buffer.len(),
        "print_number wrote past number_buffer size"
    );
    new_buffer[..written.len()].copy_from_slice(written);
    strip_extra_exponent_zero(&mut new_buffer);
    let end = new_buffer.iter().position(|&b| b == 0).unwrap();
    assert_eq!(
        std::str::from_utf8(&new_buffer[..end]).unwrap(),
        expected,
        "Printed number is not as expected."
    );
}

#[test]
fn print_number_should_print_zero() {
    assert_print_number("0", 0.0);
}

#[test]
fn print_number_should_print_negative_integers() {
    assert_print_number("-1", -1.0);
    assert_print_number("-32768", -32768.0);
    assert_print_number("-2147483648", -2147483648.0);
}

#[test]
fn print_number_should_print_positive_integers() {
    assert_print_number("1", 1.0);
    assert_print_number("32767", 32767.0);
    assert_print_number("2147483647", 2147483647.0);
}

#[test]
fn print_number_should_print_positive_reals() {
    assert_print_number("0.123", 0.123);
    assert_print_number("1e-09", 10e-10);
    assert_print_number("1000000000000", 10e11);
    assert_print_number("1.23e+129", 123e+127);
    assert_print_number("1.23e-126", 123e-128);
    // C input literal 3.1415926535897931 is this exact double.
    assert_eq!(std::f64::consts::PI.to_bits(), 0x4009_21FB_5444_2D18);
    assert_print_number("3.1415926535897931", std::f64::consts::PI);
}

#[test]
fn print_number_should_print_negative_reals() {
    assert_print_number("-0.0123", -0.0123);
    assert_print_number("-1e-09", -10e-10);
    assert_print_number("-1e+21", -10e20);
    assert_print_number("-1.23e+129", -123e+127);
    assert_print_number("-1.23e-126", -123e-128);
}

/// C: `TEST_IGNORE()` ("FIXME: Cannot test this easily in C89!"), with the
/// assertions commented out. Kept ignored to mirror the C suite; the body
/// holds those commented assertions so `cargo test -- --ignored` runs them.
#[test]
#[ignore = "TEST_IGNORE() in tests/print_number.c (FIXME: cannot test NaN/Inf in C89)"]
fn print_number_should_print_non_number() {
    assert_print_number("null", f64::NAN);
    assert_print_number("null", f64::INFINITY);
    assert_print_number("null", f64::NEG_INFINITY);
}
