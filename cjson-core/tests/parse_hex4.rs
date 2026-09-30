//! Port of tests/parse_hex4.c.

use cjson_core::parse::internals::parse_hex4;

#[test]
fn parse_hex4_should_parse_all_combinations() {
    // test all combinations
    for number in 0u32..=0xFFFF {
        let digits_lower = format!("{number:04x}");
        let digits_upper = format!("{number:04X}");
        assert_eq!(4, digits_lower.len(), "sprintf failed.");
        assert_eq!(4, digits_upper.len(), "sprintf failed.");

        assert_eq!(
            number,
            parse_hex4(digits_lower.as_bytes()),
            "Failed to parse lowercase digits."
        );
        assert_eq!(
            number,
            parse_hex4(digits_upper.as_bytes()),
            "Failed to parse uppercase digits."
        );
    }
}

#[test]
fn parse_hex4_should_parse_mixed_case() {
    assert_eq!(0xBEEF, parse_hex4(b"beef"));
    assert_eq!(0xBEEF, parse_hex4(b"beeF"));
    assert_eq!(0xBEEF, parse_hex4(b"beEf"));
    assert_eq!(0xBEEF, parse_hex4(b"beEF"));
    assert_eq!(0xBEEF, parse_hex4(b"bEef"));
    assert_eq!(0xBEEF, parse_hex4(b"bEeF"));
    assert_eq!(0xBEEF, parse_hex4(b"bEEf"));
    assert_eq!(0xBEEF, parse_hex4(b"bEEF"));
    assert_eq!(0xBEEF, parse_hex4(b"Beef"));
    assert_eq!(0xBEEF, parse_hex4(b"BeeF"));
    assert_eq!(0xBEEF, parse_hex4(b"BeEf"));
    assert_eq!(0xBEEF, parse_hex4(b"BeEF"));
    assert_eq!(0xBEEF, parse_hex4(b"BEef"));
    assert_eq!(0xBEEF, parse_hex4(b"BEeF"));
    assert_eq!(0xBEEF, parse_hex4(b"BEEf"));
    assert_eq!(0xBEEF, parse_hex4(b"BEEF"));
}
