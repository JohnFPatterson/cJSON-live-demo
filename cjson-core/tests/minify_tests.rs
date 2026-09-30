//! Port of tests/minify_tests.c, one `#[test]` per `RUN_TEST`.
//!
//! C minifies the buffer in place; `minify` returns the bytes C leaves
//! before its new terminator, which is what `TEST_ASSERT_EQUAL_STRING` sees.

use cjson_core::minify::minify;

#[test]
fn cjson_minify_should_not_overflow_buffer() {
    let unclosed_multiline_comment = "/* bla";
    let pending_escape = "\"\\";

    assert_eq!(minify(unclosed_multiline_comment.as_bytes()), b"");

    assert_eq!(minify(pending_escape.as_bytes()), b"\"\\");
}

#[test]
fn cjson_minify_should_remove_single_line_comments() {
    let to_minify = "{// this is {} \"some kind\" of [] comment /*, don't you see\n}";

    assert_eq!(minify(to_minify.as_bytes()), b"{}");
}

#[test]
fn cjson_minify_should_remove_spaces() {
    let to_minify = "{ \"key\":\ttrue\r\n    }";

    assert_eq!(minify(to_minify.as_bytes()), b"{\"key\":true}");
}

#[test]
fn cjson_minify_should_remove_multiline_comments() {
    let to_minify = "{/* this is\n a /* multi\n //line \n {comment \"\\\" */}";

    assert_eq!(minify(to_minify.as_bytes()), b"{}");
}

#[test]
fn cjson_minify_should_not_modify_strings() {
    let to_minify = "\"this is a string \\\" \\t bla\"";

    assert_eq!(minify(to_minify.as_bytes()), to_minify.as_bytes());
}

#[test]
fn cjson_minify_should_minify_json() {
    let to_minify = concat!(
        "{\n",
        "    \"glossary\": { // comment\n",
        "        \"title\": \"example glossary\",\n",
        "  /* multi\n",
        " line */\n",
        "\t\t\"GlossDiv\": {\n",
        "            \"title\": \"S\",\n",
        "\t\t\t\"GlossList\": {\n",
        "                \"GlossEntry\": {\n",
        "                    \"ID\": \"SGML\",\n",
        "\t\t\t\t\t\"SortAs\": \"SGML\",\n",
        "\t\t\t\t\t\"Acronym\": \"SGML\",\n",
        "\t\t\t\t\t\"Abbrev\": \"ISO 8879:1986\",\n",
        "\t\t\t\t\t\"GlossDef\": {\n",
        "\t\t\t\t\t\t\"GlossSeeAlso\": [\"GML\", \"XML\"]\n",
        "                    },\n",
        "\t\t\t\t\t\"GlossSee\": \"markup\"\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}"
    );
    let minified = concat!(
        "{",
        "\"glossary\":{",
        "\"title\":\"example glossary\",",
        "\"GlossDiv\":{",
        "\"title\":\"S\",",
        "\"GlossList\":{",
        "\"GlossEntry\":{",
        "\"ID\":\"SGML\",",
        "\"SortAs\":\"SGML\",",
        "\"Acronym\":\"SGML\",",
        "\"Abbrev\":\"ISO 8879:1986\",",
        "\"GlossDef\":{",
        "\"GlossSeeAlso\":[\"GML\",\"XML\"]",
        "},",
        "\"GlossSee\":\"markup\"",
        "}",
        "}",
        "}",
        "}",
        "}"
    );

    assert_eq!(minify(to_minify.as_bytes()), minified.as_bytes());
}

#[test]
fn cjson_minify_should_not_loop_infinitely() {
    let string: [u8; 7] = *b"8 / 5\n\0";
    // this should not be an infinite loop
    let _ = minify(&string);
}
