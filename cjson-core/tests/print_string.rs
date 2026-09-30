//! Port of `tests/print_string.c`, case for case.

use cjson_core::print::internals::{print_string_ptr, PrintBuffer};

fn assert_print_string(expected: &[u8], input: Option<&[u8]>) {
    // C: printbuffer over `unsigned char printed[1024]`, noalloc = true.
    let mut buffer = PrintBuffer::new(1024);
    buffer.noalloc = true;

    assert!(
        print_string_ptr(input, &mut buffer),
        "Failed to print string."
    );
    assert_eq!(
        String::from_utf8_lossy(buffer.c_str()),
        String::from_utf8_lossy(expected),
        "The printed string isn't as expected."
    );
    assert_eq!(
        buffer.c_str(),
        expected,
        "The printed string isn't as expected."
    );
}

#[test]
fn print_string_should_print_empty_strings() {
    assert_print_string(b"\"\"", Some(b""));
    assert_print_string(b"\"\"", None);
}

#[test]
fn print_string_should_print_ascii() {
    // create ascii table: bytes 0x01..=0x7E, then the terminator.
    let ascii: Vec<u8> = (1u8..0x7F).collect();

    assert_print_string(
        b"\"\\u0001\\u0002\\u0003\\u0004\\u0005\\u0006\\u0007\\b\\t\\n\\u000b\\f\\r\\u000e\\u000f\\u0010\\u0011\\u0012\\u0013\\u0014\\u0015\\u0016\\u0017\\u0018\\u0019\\u001a\\u001b\\u001c\\u001d\\u001e\\u001f !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\"",
        Some(&ascii),
    );
}

#[test]
fn print_string_should_print_utf8() {
    assert_print_string("\"ü猫慕\"".as_bytes(), Some("ü猫慕".as_bytes()));
}
