//! Port of tests/parse_with_opts.c.
//!
//! `cJSON_GetErrorPtr()` and `*return_parse_end` map to
//! `ParseFailure::position` / `ParseSuccess::end` (offsets from the input
//! start); passing a NULL `return_parse_end` maps to ignoring them.

use cjson_core::parse::parse_with_opts;
use cjson_core::Arena;

#[test]
fn parse_with_opts_should_handle_null() {
    let mut a = Arena::new();
    // `cJSON_ParseWithOpts(NULL, &error_pointer, false)` and
    // `cJSON_ParseWithOpts(NULL, NULL, false)`: a NULL `value` cannot be
    // expressed with the core's `&[u8]` input. C returns NULL before touching
    // anything (cJSON.c:1142-1145); that check lives in cjson-ffi.
    let item = parse_with_opts(&mut a, b"{}", false);
    assert!(item.is_ok(), "Failed to handle NULL error pointer.");
    cjson_core::tree::delete(&mut a, item.ok().map(|s| s.root));
    assert!(
        parse_with_opts(&mut a, b"{", false).is_err(),
        "Failed to handle NULL error pointer with parse error."
    );
}

#[test]
fn parse_with_opts_should_handle_empty_strings() {
    let mut a = Arena::new();
    let empty_string: &[u8] = b"";

    let result = parse_with_opts(&mut a, empty_string, false);
    assert!(result.is_err());
    // TEST_ASSERT_EQUAL_PTR(empty_string, cJSON_GetErrorPtr())
    assert_eq!(0, result.err().map(|e| e.position).unwrap_or(usize::MAX));

    let result = parse_with_opts(&mut a, empty_string, false);
    assert!(result.is_err());
    // error_pointer and cJSON_GetErrorPtr() both equal empty_string.
    assert_eq!(0, result.err().map(|e| e.position).unwrap_or(usize::MAX));
}

#[test]
fn parse_with_opts_should_handle_incomplete_json() {
    let mut a = Arena::new();
    let json: &[u8] = b"{ \"name\": ";

    let result = parse_with_opts(&mut a, json, false);
    assert!(result.is_err());
    // parse_end and cJSON_GetErrorPtr() both equal json + strlen(json).
    assert_eq!(
        json.len(),
        result.err().map(|e| e.position).unwrap_or(usize::MAX)
    );
}

#[test]
fn parse_with_opts_should_require_null_if_requested() {
    let mut a = Arena::new();
    let item = parse_with_opts(&mut a, b"{}", true);
    assert!(item.is_ok());
    cjson_core::tree::delete(&mut a, item.ok().map(|s| s.root));
    let item = parse_with_opts(&mut a, b"{} \n", true);
    assert!(item.is_ok());
    cjson_core::tree::delete(&mut a, item.ok().map(|s| s.root));
    assert!(parse_with_opts(&mut a, b"{}x", true).is_err());
}

#[test]
fn parse_with_opts_should_return_parse_end() {
    let mut a = Arena::new();
    let json: &[u8] = b"[] empty array XD";

    let item = parse_with_opts(&mut a, json, false).expect("item");
    assert_eq!(2, item.end);
    cjson_core::tree::delete(&mut a, Some(item.root));
}

#[test]
fn parse_with_opts_should_parse_utf8_bom() {
    let mut a = Arena::new();

    let with_bom = parse_with_opts(&mut a, b"\xEF\xBB\xBF{}", true);
    assert!(with_bom.is_ok());
    let without_bom = parse_with_opts(&mut a, b"{}", true);
    // The C test checks `with_bom` twice (its second assertion is on
    // `with_bom`, not `without_bom`); kept as is.
    assert!(with_bom.is_ok());

    let with_bom = with_bom.ok().map(|s| s.root);
    let without_bom = without_bom.ok().map(|s| s.root);
    assert!(cjson_core::compare::compare(
        &a,
        with_bom,
        without_bom,
        true
    ));

    cjson_core::tree::delete(&mut a, with_bom);
    cjson_core::tree::delete(&mut a, without_bom);
}
