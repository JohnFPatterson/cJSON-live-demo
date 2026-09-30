//! Port of tests/parse_examples.c. Inputs are read from the C suite's
//! `tests/inputs` directory.

mod common_parse;

use cjson_core::parse::{parse, parse_with_length};
use cjson_core::{Arena, NodeId};
use common_parse::read_file;

fn parse_file(arena: &mut Arena, filename: &str) -> Option<NodeId> {
    let content = read_file(filename)?;
    parse(arena, &content).ok().map(|s| s.root)
}

fn do_test(test_name: &str) {
    let mut a = Arena::new();

    let test_path = format!("inputs/{test_name}");
    let expected_path = format!("inputs/{test_name}.expected");

    // read expected output
    let expected = read_file(&expected_path).expect("Failed to read expected output.");

    // read and parse test
    let tree = parse_file(&mut a, &test_path).expect("Failed to read of parse test.");

    // print the parsed tree
    let actual =
        cjson_core::print::print(&a, Some(tree), true).expect("Failed to print tree back to JSON.");

    // TEST_ASSERT_EQUAL_STRING(expected, actual)
    assert!(
        expected == actual,
        "Expected:\n{}\nActual:\n{}",
        String::from_utf8_lossy(&expected),
        String::from_utf8_lossy(&actual)
    );

    cjson_core::tree::delete(&mut a, Some(tree));
}

#[test]
fn file_test1_should_be_parsed_and_printed() {
    do_test("test1");
}

#[test]
fn file_test2_should_be_parsed_and_printed() {
    do_test("test2");
}

#[test]
fn file_test3_should_be_parsed_and_printed() {
    do_test("test3");
}

#[test]
fn file_test4_should_be_parsed_and_printed() {
    do_test("test4");
}

#[test]
fn file_test5_should_be_parsed_and_printed() {
    do_test("test5");
}

#[test]
fn file_test6_should_not_be_parsed() {
    let mut a = Arena::new();
    let test6 = read_file("inputs/test6").expect("Failed to read test6 data.");

    let tree = parse(&mut a, &test6);
    assert!(tree.is_err(), "Should fail to parse what is not JSON.");

    // TEST_ASSERT_EQUAL_PTR(test6, cJSON_GetErrorPtr())
    assert_eq!(
        0,
        tree.err().map(|e| e.position).unwrap_or(usize::MAX),
        "Error pointer is incorrect."
    );
}

#[test]
fn file_test7_should_be_parsed_and_printed() {
    do_test("test7");
}

#[test]
fn file_test8_should_be_parsed_and_printed() {
    do_test("test8");
}

#[test]
fn file_test9_should_be_parsed_and_printed() {
    do_test("test9");
}

#[test]
fn file_test10_should_be_parsed_and_printed() {
    do_test("test10");
}

#[test]
fn file_test11_should_be_parsed_and_printed() {
    do_test("test11");
}

#[test]
fn test12_should_not_be_parsed() {
    let mut a = Arena::new();
    let test12: &[u8] = b"{ \"name\": ";

    let tree = parse(&mut a, test12);
    assert!(tree.is_err(), "Should fail to parse incomplete JSON.");

    // TEST_ASSERT_EQUAL_PTR(test12 + strlen(test12), cJSON_GetErrorPtr())
    assert_eq!(
        test12.len(),
        tree.err().map(|e| e.position).unwrap_or(usize::MAX),
        "Error pointer is incorrect."
    );
}

const TEST_13: &[u8] = b"{\
\"Image\":{\
\"Width\":800,\
\"Height\":600,\
\"Title\":\"Viewfrom15thFloor\",\
\"Thumbnail\":{\
\"Url\":\"http:/*www.example.com/image/481989943\",\
\"Height\":125,\
\"Width\":\"100\"\
},\
\"IDs\":[116,943,234,38793]\
}\
}";

#[test]
fn test13_should_be_parsed_without_null_termination() {
    let mut a = Arena::new();
    // char test_13_wo_null[sizeof(test_13) - 1]: the bytes without the NUL.
    let test_13_wo_null: Vec<u8> = TEST_13.to_vec();

    let tree = parse_with_length(&mut a, &test_13_wo_null);
    assert!(tree.is_ok(), "Failed to parse valid json.");

    cjson_core::tree::delete(&mut a, tree.ok().map(|s| s.root));
}

#[test]
fn test14_should_not_be_parsed() {
    let mut a = Arena::new();
    // cJSON_ParseWithLength(test_14, sizeof(test_14) - 2): all but the last byte.
    let test_14 = TEST_13;

    let tree = parse_with_length(&mut a, &test_14[..test_14.len() - 1]);
    assert!(
        tree.is_err(),
        "Should not continue after buffer_length is reached."
    );
}

/// Address Sanitizer
#[test]
fn test15_should_not_heap_buffer_overflow() {
    let strings: [&[u8]; 2] = [b"{\"1\":1,", b"{\"1\":1, "];

    for json_string in strings {
        let mut a = Arena::new();
        // malloc(len) + memcpy: an exact-size buffer without terminator.
        let exact_size_heap: Vec<u8> = json_string.to_vec();

        let json = parse_with_length(&mut a, &exact_size_heap);

        cjson_core::tree::delete(&mut a, json.ok().map(|s| s.root));
    }
}
