//! `cJSON_Minify`, `skip_oneline_comment`, `skip_multiline_comment`,
//! `minify_string` (cJSON.c:2935-3034).
//!
//! C minifies in place with a write cursor that never passes the read
//! cursor, so every byte it reads is still the original input. Writing into
//! a separate output buffer is therefore observably identical.

/// Read cursor over the C string: bytes past the end read as the NUL
/// terminator, exactly where C would find it.
struct Input<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Input<'_> {
    fn at(&self, offset: usize) -> u8 {
        self.pos
            .checked_add(offset)
            .and_then(|i| self.bytes.get(i))
            .copied()
            .unwrap_or(0)
    }

    /// C's `*input += n`. C never advances past the terminator; saturating
    /// keeps that true even for inputs no C string could produce.
    fn advance(&mut self, n: usize) {
        self.pos = self.pos.saturating_add(n).min(self.bytes.len());
    }
}

/// `cJSON_Minify(json)` on the C string whose bytes (before the NUL) are
/// `input`. Returns the bytes C leaves before the new terminator. C works in
/// place and never writes past the new terminator, so the FFI writes these
/// bytes followed by one NUL at the start of the caller's buffer.
///
/// Quirks reproduced from cJSON.c:
/// * Only `' '`, `'\t'`, `'\r'`, `'\n'` are dropped outside strings
///   (cJSON.c:3001-3006); other whitespace such as `'\f'` is kept.
/// * A `/` that does not start a comment is dropped, not copied
///   (cJSON.c:3016-3018).
/// * An unterminated `/* ...` comment swallows the rest of the input
///   (cJSON.c:2954-2961); a `// ...` comment ends after its `'\n'`, which is
///   consumed with it (cJSON.c:2942-2944).
/// * Inside strings only a backslash directly followed by `"` is treated as
///   an escape (cJSON.c:2979-2983). So in `"a\\"` the second backslash
///   escapes the closing quote and the string keeps going. An unterminated
///   string is copied up to the end of the input.
pub fn minify(input: &[u8]) -> Vec<u8> {
    let bytes = match input.iter().position(|&b| b == 0) {
        Some(end) => input.get(..end).unwrap_or(input),
        None => input,
    };
    let mut json = Input { bytes, pos: 0 };
    let mut into = Vec::with_capacity(bytes.len());

    loop {
        match json.at(0) {
            0 => break,
            b' ' | b'\t' | b'\r' | b'\n' => json.advance(1),
            b'/' => match json.at(1) {
                b'/' => skip_oneline_comment(&mut json),
                b'*' => skip_multiline_comment(&mut json),
                _ => json.advance(1),
            },
            b'"' => minify_string(&mut json, &mut into),
            other => {
                into.push(other);
                json.advance(1);
            }
        }
    }
    into
}

/// `skip_oneline_comment` (cJSON.c:2936-2947).
fn skip_oneline_comment(input: &mut Input<'_>) {
    input.advance(2);
    while input.at(0) != 0 {
        if input.at(0) == b'\n' {
            input.advance(1);
            return;
        }
        input.advance(1);
    }
}

/// `skip_multiline_comment` (cJSON.c:2950-2962).
fn skip_multiline_comment(input: &mut Input<'_>) {
    input.advance(2);
    while input.at(0) != 0 {
        if input.at(0) == b'*' && input.at(1) == b'/' {
            input.advance(2);
            return;
        }
        input.advance(1);
    }
}

/// `minify_string` (cJSON.c:2965-2985).
fn minify_string(input: &mut Input<'_>, output: &mut Vec<u8>) {
    output.push(input.at(0));
    input.advance(1);

    loop {
        let current = input.at(0);
        if current == 0 {
            return;
        }
        output.push(current);
        if current == b'"' {
            input.advance(1);
            return;
        }
        if current == b'\\' && input.at(1) == b'"' {
            output.push(b'"');
            input.advance(1);
        }
        input.advance(1);
    }
}

#[cfg(test)]
mod tests {
    use super::minify;

    #[test]
    fn stops_at_first_nul() {
        assert_eq!(minify(b"[1, 2]\0 [3]"), b"[1,2]");
    }

    #[test]
    fn backslash_backslash_quote_does_not_close_string() {
        assert_eq!(minify(b"\"a\\\\\" , 1\""), b"\"a\\\\\" , 1\"");
    }

    /// Outputs recorded from the original `cJSON_Minify` (C probe over
    /// cJSON.c) for inputs that hit unterminated comments/strings, stray
    /// slashes, escapes and non-JSON whitespace.
    #[test]
    fn matches_recorded_c_outputs() {
        let cases: &[(&[u8], &[u8])] = &[
            (b"", b""),
            (b"/", b""),
            (b"/*", b""),
            (b"/*/", b""),
            (b"a/b", b"ab"),
            (b"// no newline", b""),
            (b"//x\r\ny", b"y"),
            (b"/* x * / y", b""),
            (b"/* x **/y", b"y"),
            (b"\"", b"\""),
            (b"\"abc", b"\"abc"),
            (b"\"a\\\\\\\"b\" c", b"\"a\\\\\\\"b\"c"),
            (b"\"\\\\", b"\"\\\\"),
            (b"\"//not comment\" //c\n1", b"\"//not comment\"1"),
            (b"[1,\x0c2]", b"[1,\x0c2]"),
            (b"{\"k\" :\x0b 1}", b"{\"k\":\x0b1}"),
            (b"\x01 \x7f \xff", b"\x01\x7f\xff"),
            (b"\"unterminated \\\" still", b"\"unterminated \\\" still"),
            (b"/\"a b\"", b"\"a b\""),
            (b"*/ x", b"*x"),
            (b"\\\" x", b"\\\" x"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                minify(input),
                *expected,
                "input {:?}",
                String::from_utf8_lossy(input)
            );
        }
    }
}
