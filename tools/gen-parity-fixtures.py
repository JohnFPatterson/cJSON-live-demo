#!/usr/bin/python3
"""Generate the parity fixture corpus in tests/parity/.

Deterministic: the same script always writes the same bytes. No randomness
from the environment is used; the "rnd_" and "mut_" families come from a fixed
64-bit LCG seed.

The parity gate pins every fixture on its first run, so existing fixtures
must never change. This script therefore refuses to overwrite an existing file
with different bytes, and it never deletes files. To add fixtures, append new
entries (with new names) and run it again.

Usage:
    /usr/bin/python3 tools/gen-parity-fixtures.py            # write missing files
    /usr/bin/python3 tools/gen-parity-fixtures.py --check    # verify, write nothing
    /usr/bin/python3 tools/gen-parity-fixtures.py --out DIR  # write elsewhere (dry run)

Every file is <category>_<name>.json. Categories:
    num_  numbers: integers, int limits, doubles, %1.15g vs %1.17g, non-JSON forms
    str_  strings: escapes, \\u handling, surrogates, raw bytes, invalid UTF-8
    arr_  arrays: empty, nested, commas, whitespace, embedded NUL
    obj_  objects: keys, duplicates, case variants, missing pieces
    lit_  literals and near-literals
    trail_ what follows the first value (garbage, second value, NUL + garbage)
    ws_   empty and whitespace-only inputs, control bytes as whitespace
    bom_  UTF-8 BOM handling (cJSON.c skip_utf8_bom)
    deep_ nesting depth around CJSON_NESTING_LIMIT (1000)
    big_  long strings and large arrays/objects
    min_  cJSON_Minify inputs: comments, strings with comment markers, escapes
    rnd_  deterministic token soup (valid and invalid)
    mut_  deterministic single-byte mutations of valid documents
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "parity"

FIXTURES: dict[str, bytes] = {}


def add(name: str, data: bytes | str) -> None:
    if isinstance(data, str):
        data = data.encode("utf-8")
    if name in FIXTURES:
        raise SystemExit(f"duplicate fixture name {name}")
    FIXTURES[name] = data


# ------------------------------------------------------------------ numbers

NUMBERS = [
    # integers and int limits (valueint saturation, cJSON.c parse_number)
    ("zero", "0"),
    ("neg_zero", "-0"),
    ("neg_zero_frac", "-0.0"),
    ("one", "1"),
    ("neg_one", "-1"),
    ("int_max", "2147483647"),
    ("int_max_plus1", "2147483648"),
    ("int_min", "-2147483648"),
    ("int_min_minus1", "-2147483649"),
    ("two_pow_53", "9007199254740992"),
    ("two_pow_53_plus1", "9007199254740993"),
    ("big_int_18", "123456789012345678"),
    ("big_int_40", "1234567890123456789012345678901234567890"),
    ("big_int_80", "1" * 80),
    ("uint64_max", "18446744073709551615"),
    # extremes
    ("1e308", "1e308"),
    ("1e309", "1e309"),
    ("neg_1e309", "-1e309"),
    ("1e_minus400", "1e-400"),
    ("denorm_min", "5e-324"),
    ("dbl_min", "2.2250738585072014e-308"),
    ("dbl_max", "1.7976931348623157e308"),
    ("huge_exp", "1e999999999999"),
    ("tiny_exp", "1e-999999999999"),
    # %1.15g vs %1.17g round trip (cJSON.c print_number)
    ("0_1", "0.1"),
    ("third", "0.3333333333333333"),
    ("0_30000000000000004", "0.30000000000000004"),
    ("0_3", "0.3"),
    ("1e21", "1e21"),
    ("1e20", "1e20"),
    ("1e15", "1e15"),
    ("1e16", "1e16"),
    ("1e17", "1e17"),
    ("1e_minus7", "1e-7"),
    ("1e_minus5", "1e-5"),
    ("0_000001", "0.000001"),
    ("100", "100"),
    ("1e2", "1e2"),
    ("1E2_plus", "1E+2"),
    ("1e_minus2", "1e-2"),
    ("0_1e1", "0.1e1"),
    ("12_5", "12.5"),
    ("neg_12_5e_minus3", "-12.5e-3"),
    ("pi", "3.141592653589793"),
    ("pi_long", "3.14159265358979323846"),
    ("2_5", "2.5"),
    ("1_0", "1.0"),
    ("123_456e_minus789", "123.456e-789"),
    ("1_5e300", "1.5e300"),
    ("neg_frac", "-0.000123"),
    ("max_safe_frac", "4503599627370495.5"),
    ("int_as_float", "2147483647.0"),
    ("frac_near_int", "0.99999999999999999"),
    ("digits_17", "0.12345678901234567"),
    # non-JSON forms cJSON accepts or rejects (strtod decides the length)
    ("dot_5_neg", "-.5"),
    ("trailing_dot", "1."),
    ("leading_zero", "01"),
    ("leading_zeros", "00012"),
    ("exp_empty", "1e"),
    ("exp_plus_empty", "1e+"),
    ("exp_minus_empty", "1e-"),
    ("upper_E5", "1E5"),
    ("plus_one", "+1"),
    ("minus_only", "-"),
    ("minus_minus_one", "--1"),
    ("double_exp", "1e5e3"),
    ("hex", "0x10"),
    ("dot_5", ".5"),
    ("two_dots", "1.5.5"),
    ("nan_word", "NaN"),
    ("infinity_word", "Infinity"),
    ("neg_infinity_word", "-Infinity"),
    ("inf_word", "inf"),
    ("comma_decimal", "1,5"),
    ("space_inside", "1 2"),
    ("exp_dot", "1e5.5"),
    ("trailing_letters", "42abc"),
    ("padded", "  42  "),
    ("in_array", "[1,2.5,-3e2,0.1,1e400,-0]"),
    ("valueint_saturate", "[1e10,-1e10,2147483647.5,-2147483648.5]"),
    ("long_mantissa_70", "0." + "1234567890" * 7),
]
for name, text in NUMBERS:
    add(f"num_{name}", text)

# ------------------------------------------------------------------ strings

STRINGS: list[tuple[str, bytes]] = [
    ("empty", b'""'),
    ("simple", b'"hello"'),
    ("all_escapes", b'"\\" \\\\ \\/ \\b \\f \\n \\r \\t"'),
    ("u0000", b'"a\\u0000b"'),
    ("u0000_only", b'"\\u0000"'),
    ("u0001", b'"\\u0001\\u001f\\u007f"'),
    ("u00e9", b'"\\u00e9"'),
    ("u00C9_upper", b'"\\u00C9"'),
    ("u0080", b'"\\u0080"'),
    ("u07ff", b'"\\u07ff"'),
    ("u0800", b'"\\u0800"'),
    ("u20ac", b'"\\u20ac"'),
    ("ud7ff", b'"\\ud7ff"'),
    ("ue000", b'"\\ue000"'),
    ("uffff", b'"\\uffff"'),
    ("surrogate_pair", b'"\\ud83d\\ude00"'),
    ("surrogate_pair_upper", b'"\\uD834\\uDD1E"'),
    ("surrogate_max", b'"\\udbff\\udfff"'),
    ("lone_high", b'"\\ud83d"'),
    ("lone_high_then_char", b'"\\ud83dx"'),
    ("lone_low", b'"\\ude00"'),
    ("high_then_nonlow", b'"\\ud83d\\u0041"'),
    ("high_then_high", b'"\\ud83d\\ud83d"'),
    ("high_then_escape_n", b'"\\ud83d\\n"'),
    ("high_then_truncated", b'"\\ud83d\\u12"'),
    ("bad_hex", b'"\\u12G4"'),
    ("bad_hex_first", b'"\\uZ234"'),
    ("truncated_u12", b'"\\u12"'),
    ("truncated_u12_eof", b'"\\u12'),
    ("truncated_u_eof", b'"\\u'),
    ("invalid_escape_x", b'"\\x41"'),
    ("invalid_escape_a", b'"\\a"'),
    ("invalid_escape_quote_single", b"\"\\'\""),
    ("unterminated", b'"abc'),
    ("unterminated_escape_quote", b'"abc\\"'),
    ("quote_only", b'"'),
    ("raw_control_01", b'"a\x01b"'),
    ("raw_tab", b'"a\tb"'),
    ("raw_newline", b'"a\nb"'),
    ("raw_cr", b'"a\rb"'),
    ("raw_del", b'"a\x7fb"'),
    ("raw_utf8", '"h\u00e9llo \u20ac \U0001F600"'.encode("utf-8")),
    ("invalid_utf8_ff_fe", b'"\xff\xfe"'),
    ("invalid_utf8_overlong", b'"\xc0\xaf"'),
    ("invalid_utf8_continuation", b'"\x80\x81"'),
    ("invalid_utf8_truncated", b'"\xe2\x82"'),
    ("utf8_surrogate_encoded", b'"\xed\xa0\x80"'),
    ("backslash_eof", b'"abc\\'),
    ("backslash_only", b"\\"),
    ("escaped_quote", b'"a\\"b"'),
    ("escaped_backslash_end", b'"a\\\\"'),
    ("slash_unescaped", b'"a/b</script>"'),
    ("single_quoted", b"'abc'"),
    ("in_array", b'["a","\\u00e9","\\ud83d\\ude00",""]'),
    ("key_unicode", b'{"\\u00e9\\u20ac":"\\ud83d\\ude00"}'),
    ("key_raw_utf8", '{"cl\u00e9":"v\u00e0l"}'.encode("utf-8")),
    ("nul_inside", b'"a\x00b"'),
    ("nul_escape_then_text", b'["\\u0000tail","x"]'),
    ("all_control_escaped", b'"' + b"".join(b"\\u%04x" % i for i in range(32)) + b'"'),
    ("mixed_escapes_long", b'"' + b"\\n\\t\\\"\\\\\\/\\u00e9" * 20 + b'"'),
]
for name, data in STRINGS:
    add(f"str_{name}", data)

# ------------------------------------------------------------------ arrays

ARRAYS: list[tuple[str, bytes]] = [
    ("empty", b"[]"),
    ("empty_ws", b"[ ]"),
    ("empty_ws_all", b"[ \t\r\n]"),
    ("nested", b"[[[]]]"),
    ("nested_values", b"[[1,[2,[3]]],[],[[]]]"),
    ("mixed", b'[1,"a",true,false,null,{},[],-0.5,{"k":[1]}]'),
    ("trailing_comma", b"[1,2,]"),
    ("missing_comma", b"[1 2]"),
    ("leading_comma", b"[,1]"),
    ("only_comma", b"[,]"),
    ("double_comma", b"[1,,2]"),
    ("unclosed", b"[1,2"),
    ("unclosed_after_comma", b"[1,"),
    ("unclosed_empty", b"["),
    ("close_only", b"]"),
    ("mismatched", b"[1}"),
    ("ws_variants", b"[\r\n\t 1 \r\n\t, 2\t,\n3\r]"),
    ("ctrl_ws", b"[\x01 1\x02,\x1f2\x0b]"),
    ("all_ctrl_ws", b"[" + bytes(range(1, 33)) + b"1" + bytes(range(1, 33)) + b"]"),
    ("nul_as_ws", b"[1,\x002]"),
    ("nul_after_open", b"[\x00]"),
    ("strings", b'["a", "b" ,"c"]'),
    ("literals", b"[true,false,null]"),
    ("nested_objects", b'[{"a":1},{"b":[{"c":{}}]}]'),
    ("colon", b"[1:2]"),
    ("value_then_garbage", b"[1x]"),
    ("single_number", b"[42]"),
    ("single_neg", b"[-42]"),
]
for name, data in ARRAYS:
    add(f"arr_{name}", data)

# ------------------------------------------------------------------ objects

OBJECTS: list[tuple[str, bytes]] = [
    ("empty", b"{}"),
    ("empty_ws", b"{ }"),
    ("simple", b'{"a":1}'),
    ("nested", b'{"a":{"b":{"c":[1,2,{"d":null}]}}}'),
    ("all_types", b'{"n":null,"t":true,"f":false,"i":1,"d":1.5,"s":"x","a":[],"o":{}}'),
    ("trailing_comma", b'{"a":1,}'),
    ("missing_comma", b'{"a":1 "b":2}'),
    ("missing_colon", b'{"a" 1}'),
    ("missing_value", b'{"a":}'),
    ("colon_only", b"{:}"),
    ("key_only", b'{"a"}'),
    ("non_string_key", b"{1:2}"),
    ("bare_key", b"{a:1}"),
    ("single_quote_key", b"{'a':1}"),
    ("null_key", b"{null:1}"),
    ("duplicate_keys", b'{"a":1,"a":2}'),
    ("duplicate_keys_nested", b'{"a":{"x":1},"a":{"x":2}}'),
    ("case_variant_keys", b'{"a":1,"A":2}'),
    ("case_variant_keys_3", b'{"Key":1,"kEY":2,"key":3}'),
    ("case_variant_values", b'{"k":"Value","K":"value"}'),
    ("empty_key", b'{"":1}'),
    ("unclosed", b'{"a":1'),
    ("unclosed_after_comma", b'{"a":1,'),
    ("unclosed_empty", b"{"),
    ("unclosed_key", b'{"a'),
    ("close_only", b"}"),
    ("mismatched", b'{"a":1]'),
    ("ws", b'{\r\n\t"a"\t:\r\n1\n,\t"b" : [ ] }'),
    ("ctrl_ws", b'{\x01"a"\x02:\x031\x04}'),
    ("escaped_key", b'{"a\\nb":1,"q\\"":2}'),
    ("key_u0000", b'{"a\\u0000b":1}'),
    ("key_bad_escape", b'{"a\\xb":1}'),
    ("comma_first", b'{,"a":1}'),
    ("double_colon", b'{"a"::1}'),
    ("many_keys", ("{" + ",".join(f'"k{i}":{i}' for i in range(100)) + "}").encode()),
    ("array_value_garbage", b'{"a":[1,2]x}'),
]
for name, data in OBJECTS:
    add(f"obj_{name}", data)

# ------------------------------------------------------------------ literals

LITERALS = [
    ("null", "null"),
    ("true", "true"),
    ("false", "false"),
    ("nul", "nul"),
    ("tru", "tru"),
    ("fals", "fals"),
    ("nullx", "nullx"),
    ("truefalse", "truefalse"),
    ("null_upper", "NULL"),
    ("true_title", "True"),
    ("n", "n"),
    ("t", "t"),
    ("f", "f"),
    ("null_null", "null null"),
    ("true_ws", "true \n"),
    ("in_array_nul", "[nul]"),
    ("in_array_tru", "[tru]"),
    ("in_object_fals", '{"a":fals}'),
    ("undefined", "undefined"),
]
for name, text in LITERALS:
    add(f"lit_{name}", text)

# ------------------------------------------------------------------ trailing data

TRAILING: list[tuple[str, bytes]] = [
    ("garbage", b"[1]x"),
    ("ws", b"[1]  \n"),
    ("second_value", b"[1][2]"),
    ("second_value_ws", b"{} {}"),
    ("nul_garbage", b"[1] \x00garbage"),
    ("nul_immediately", b"[1]\x00"),
    ("nul_ws_nul", b"{} \x00 \x00"),
    ("nul_first", b"\x00[1]"),
    ("number_nul_garbage", b"42\x00xyz"),
    ("string_nul_garbage", b'"s"\x00"t"'),
    ("comma", b"[1],"),
    ("close_extra", b"[1]]"),
    ("newline_garbage", b"true\nfalse"),
]
for name, data in TRAILING:
    add(f"trail_{name}", data)

# ------------------------------------------------------------------ whitespace

WHITESPACE: list[tuple[str, bytes]] = [
    ("empty_file", b""),
    ("space", b" "),
    ("only", b" \t\r\n"),
    ("only_ctrl", b"\x01\x02\x1f"),
    ("only_nul", b"\x00"),
    ("leading", b"\n\n  [1]"),
    ("formfeed", b"\x0c[1]\x0c"),
    ("vtab", b"\x0b{}\x0b"),
    ("nbsp", b"\xa0[1]"),
    ("del", b"\x7f[1]"),
    ("crlf_doc", b'{\r\n  "a": [\r\n    1,\r\n    2\r\n  ]\r\n}\r\n'),
]
for name, data in WHITESPACE:
    add(f"ws_{name}", data)

# ------------------------------------------------------------------ BOM

BOM = b"\xef\xbb\xbf"
BOMS: list[tuple[str, bytes]] = [
    ("obj", BOM + b"{}"),
    ("arr", BOM + b"[1,2]"),
    ("num", BOM + b"42"),
    ("str", BOM + b'"x"'),
    ("only", BOM),
    ("ws", BOM + b"  \n"),
    ("ws_then_value", BOM + b"  \n[true]"),
    ("double", BOM + BOM + b"{}"),
    ("after_ws", b" " + BOM + b"{}"),
    ("partial", b"\xef\xbb{}"),
    ("inside_array", b"[" + BOM + b"1]"),
    ("in_string", b'"' + BOM + b'"'),
]
for name, data in BOMS:
    add(f"bom_{name}", data)

# ------------------------------------------------------------------ depth

def deep_array(depth: int, inner: str = "") -> str:
    return "[" * depth + inner + "]" * depth


def deep_object(depth: int) -> str:
    # depth braces in total; the innermost object is empty
    return '{"a":' * (depth - 1) + "{}" + "}" * (depth - 1)


for depth in (999, 1000, 1001):
    add(f"deep_arr_{depth}", deep_array(depth))
    add(f"deep_obj_{depth}", deep_object(depth))
add("deep_arr_1000_value", deep_array(1000, "1"))
add("deep_arr_1001_value", deep_array(1001, "1"))
add("deep_arr_1000_unclosed", "[" * 1000)
add("deep_arr_2000", deep_array(2000))
add("deep_mixed_1000", '[{"a":' * 500 + "null" + "}]" * 500)
add("deep_mixed_1001", '[{"a":' * 500 + "[]" + "}]" * 500)
add("deep_arr_100", deep_array(100, '"x"'))
# cJSON_Compare is O(2^depth) on nested objects; 19 is compared, 20 is
# "cmp skipped" (tools/DRIVER_FORMAT.md, compare_cost).
add("deep_obj_19", deep_object(19))
add("deep_obj_20", deep_object(20))

# ------------------------------------------------------------------ big

add("big_str_4k", '"' + ("abcdefghijklmnopqrstuvwxyz0123456789" * 114)[:4096] + '"')
add("big_str_4k_escapes", '"' + "\\n\\u00e9" * 512 + '"')
add("big_str_4k_unterminated", '"' + "x" * 4096)
add("big_arr_1000_ints", "[" + ",".join(str(i * 7 - 3000) for i in range(1000)) + "]")
add("big_arr_1000_floats", "[" + ",".join(f"{i}.{(i * 37) % 1000:03d}e{(i % 21) - 10}" for i in range(1000)) + "]")
add("big_arr_1000_mixed", "[" + ",".join(["1", '"s"', "true", "null", "[]", "{}", "-0.25"][i % 7] for i in range(1000)) + "]")
add("big_obj_500", "{" + ",".join(f'"key{i:03d}":[{i},"v{i}"]' for i in range(500)) + "}")

# ------------------------------------------------------------------ minify

MINIFY: list[tuple[str, bytes]] = [
    ("line_comment", b"[1, // comment\n 2]"),
    ("block_comment", b"[1, /* comment */ 2]"),
    ("block_multiline", b"{\n  /* a\n     b */\n  \"k\": 1\n}"),
    ("unterminated_block", b"[1, /* never closed"),
    ("unterminated_line", b"[1] // no newline"),
    ("string_comment_markers", b'["a // b", "c /* d */"]'),
    ("escaped_quote_marker", b'["a\\"b // c", 1]'),
    ("backslash_backslash_quote", b'["a\\\\", 1 , 2]'),
    ("lone_slash", b"[1 / 2]"),
    ("empty_block", b"/**/[1]"),
    ("nested_block", b"/* /* */ */[1]"),
    ("all_ws", b"\t\r\n [ 1 ] \x0c"),
    ("comment_only", b"// only"),
    ("block_only", b"/* only */"),
    ("slash_eof", b"[1]/"),
    ("slash_star_slash", b"/*/"),
    ("star_eof", b"/*"),
    ("unterminated_string", b'["abc'),
    ("string_backslash_eof", b'"abc\\'),
    ("crlf_comments", b"{\r\n// c\r\n\"a\" : 1 /* x */\r\n}"),
    ("comment_in_key", b'{"a/*b*/c": 1}'),
    ("comment_between_tokens", b"{\"a\"/*x*/:/*y*/1//z\n}"),
    ("ws_inside_string", b'[" a \t b "]'),
    ("nul_then_more", b"[1, 2]\x00 [3, 4]"),
    ("slash_in_string_end", b'["http://x/"]'),
]
for name, data in MINIFY:
    add(f"min_{name}", data)

# ------------------------------------------------------------------ deterministic random


class Lcg:
    """Knuth MMIX LCG; fixed seed so the corpus never changes."""

    def __init__(self, seed: int) -> None:
        self.state = seed & 0xFFFFFFFFFFFFFFFF

    def next(self) -> int:
        self.state = (self.state * 6364136223846793005 + 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
        return self.state >> 33

    def below(self, n: int) -> int:
        return self.next() % n


TOKENS = [
    "[", "]", "{", "}", ",", ":", " ", "\n", '"a"', '"\\u00e9"', '"\\ud83d\\ude00"',
    '"k"', "1", "-0.5e3", "0", "1e400", "true", "false", "null", '"\\n"', "12.5",
    "/*c*/", '"x\\"y"', "\t", "-", "tru", '"', "\\",
]

rng = Lcg(0xC0FFEE)
for i in range(30):
    count = 3 + rng.below(25)
    add(f"rnd_{i:03d}", "".join(TOKENS[rng.below(len(TOKENS))] for _ in range(count)))

MUTATION_BASES = [
    b'{"name":"cJSON","version":[1,7,19],"ok":true,"pi":3.14159,"nested":{"a":null}}',
    b'[1,-2.5e-3,"s\\u00e9",{"k":[true,false]},[],{}]',
    b'{"a":"\\ud83d\\ude00","b":[0.1,1e21,-0],"c":"x\\ty"}',
]
MUTATION_BYTES = b'[]{},:" \\0-.e+tfnu/\x00\x01\xff'
rng = Lcg(0xBADC0DE)
for i in range(30):
    base = bytearray(MUTATION_BASES[i % len(MUTATION_BASES)])
    pos = rng.below(len(base))
    base[pos] = MUTATION_BYTES[rng.below(len(MUTATION_BYTES))]
    add(f"mut_{i:03d}", bytes(base))


# ------------------------------------------------------------------ write


def main(argv: list[str]) -> int:
    check_only = "--check" in argv[1:]
    out = OUT
    if "--out" in argv[1:]:
        idx = argv.index("--out")
        if idx + 1 >= len(argv):
            print("--out needs a directory", file=sys.stderr)
            return 2
        out = Path(argv[idx + 1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    written = unchanged = 0
    conflicts: list[str] = []
    for name, data in FIXTURES.items():
        path = out / f"{name}.json"
        if path.exists():
            if path.read_bytes() == data:
                unchanged += 1
            else:
                conflicts.append(str(path))
            continue
        if check_only:
            conflicts.append(f"{path} (missing)")
            continue
        path.write_bytes(data)
        written += 1

    counts: dict[str, int] = {}
    for name in FIXTURES:
        cat = name.split("_", 1)[0]
        counts[cat] = counts.get(cat, 0) + 1
    print(f"{len(FIXTURES)} fixtures: {written} written, {unchanged} unchanged, {len(conflicts)} conflicts")
    print("by category: " + ", ".join(f"{k} {v}" for k, v in counts.items()))
    for c in conflicts:
        print(f"CONFLICT: {c} differs from the generator; pinned fixtures must not change", file=sys.stderr)
    return 1 if conflicts else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
