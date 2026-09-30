//! Parser: port of `parse_value`, `parse_number`, `parse_string`,
//! `parse_array`, `parse_object`, `parse_hex4`, `utf16_literal_to_utf8`,
//! `buffer_skip_whitespace`, `skip_utf8_bom` and
//! `cJSON_ParseWithLengthOpts` (cJSON.c:307-1240).
//!
//! Public signatures in this file are a contract used by `cjson-ffi` and `tools/rust-driver`; do not change them without
//! updating those callers.
//!
//! The algorithms live in [`internals`], which mirrors the C static
//! functions one for one (same control flow, same offset bookkeeping) so
//! error positions match `cJSON_GetErrorPtr()` exactly.

use crate::store::NodeStore;

/// Successful parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseSuccess<Id> {
    /// Root node (caller owns it; free with `tree::delete`).
    pub root: Id,
    /// Offset `*return_parse_end` would point to, relative to the input start.
    pub end: usize,
}

/// Failed parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseFailure {
    /// `global_error.position`: offset `cJSON_GetErrorPtr()` (and
    /// `*return_parse_end`) point to, relative to the input start.
    pub position: usize,
}

/// `cJSON_ParseWithLengthOpts(value, buffer_length, &end, require_null_terminated)`
/// where `input` is exactly the `buffer_length` bytes at `value`
/// (it may or may not contain NUL bytes). All nodes allocated for a failed
/// parse are freed before returning.
///
/// A NULL `value` cannot be expressed here; the FFI handles it (C fails with
/// `global_error` reset to position 0 and `json` NULL, cJSON.c:1160-1166).
pub fn parse_with_length_opts<S: NodeStore>(
    store: &mut S,
    input: &[u8],
    require_null_terminated: bool,
) -> Result<ParseSuccess<S::Id>, ParseFailure> {
    use internals::*;

    let mut buffer = ParseBuffer::new(input);

    // cJSON.c:1163: `0 == buffer_length` fails before anything is set up;
    // the fail path then reports position 0 (buffer.length is still 0).
    if input.is_empty() {
        return Err(ParseFailure { position: 0 });
    }

    let item = match store.alloc_node() {
        Ok(item) => item,
        Err(_) => return Err(failure_position(&buffer)),
    };

    // cJSON.c:1179: parse_value(item, buffer_skip_whitespace(skip_utf8_bom(&buffer))).
    // skip_utf8_bom only returns NULL when offset != 0, and a NULL buffer
    // makes both buffer_skip_whitespace and parse_value fail.
    let parsed = skip_utf8_bom(&mut buffer) && {
        buffer_skip_whitespace(&mut buffer);
        parse_value(store, item, &mut buffer)
    };

    let ok = parsed
        && (!require_null_terminated || {
            // cJSON.c:1186-1192
            buffer_skip_whitespace(&mut buffer);
            buffer.offset < buffer.length && buffer.byte_at(0) == Some(0)
        });

    if ok {
        return Ok(ParseSuccess {
            root: item,
            end: buffer.offset,
        });
    }

    // cJSON.c:1201-1205
    crate::tree::delete(store, Some(item));
    Err(failure_position(&buffer))
}

/// Error position computed on the fail path of `cJSON_ParseWithLengthOpts`
/// (cJSON.c:1213-1220): the current offset if it is inside the buffer, else
/// the last byte, else 0.
fn failure_position(buffer: &internals::ParseBuffer<'_>) -> ParseFailure {
    let position = if buffer.offset < buffer.length {
        buffer.offset
    } else if buffer.length > 0 {
        buffer.length - 1
    } else {
        0
    };
    ParseFailure { position }
}

/// `cJSON_ParseWithOpts(value, &end, require_null_terminated)` where
/// `input` is the bytes of the C string `value` *without* its terminating
/// NUL (C computes `buffer_length = strlen(value) + 1`, so the terminator is
/// part of the parse buffer).
///
/// Like C's `strlen`, the input is cut at its first NUL byte, if any.
pub fn parse_with_opts<S: NodeStore>(
    store: &mut S,
    input: &[u8],
    require_null_terminated: bool,
) -> Result<ParseSuccess<S::Id>, ParseFailure> {
    let c_len = input.iter().position(|&b| b == 0).unwrap_or(input.len());
    let mut terminated = Vec::with_capacity(c_len.saturating_add(1));
    terminated.extend_from_slice(input.get(..c_len).unwrap_or(input));
    terminated.push(0);
    parse_with_length_opts(store, &terminated, require_null_terminated)
}

/// `cJSON_Parse(value)`: `parse_with_opts(store, input, false)`.
pub fn parse<S: NodeStore>(
    store: &mut S,
    input: &[u8],
) -> Result<ParseSuccess<S::Id>, ParseFailure> {
    parse_with_opts(store, input, false)
}

/// `cJSON_ParseWithLength(value, buffer_length)`:
/// `parse_with_length_opts(store, input, false)` (cJSON.c:1240-1243).
pub fn parse_with_length<S: NodeStore>(
    store: &mut S,
    input: &[u8],
) -> Result<ParseSuccess<S::Id>, ParseFailure> {
    parse_with_length_opts(store, input, false)
}

/// Low-level ports of the C static parse functions, exposed for the ported
/// Unity tests (which call them directly on a hand-built `parse_buffer`).
///
/// Offsets use wrapping `usize` arithmetic, which is exactly C's `size_t`
/// behavior; every byte read goes through [`ParseBuffer::byte_at`], so a
/// caller-supplied `length` larger than `content` (undefined behavior in C)
/// reads as "no byte" and never panics.
pub mod internals {
    use crate::consts::{
        CJSON_ARRAY, CJSON_FALSE, CJSON_NESTING_LIMIT, CJSON_NULL, CJSON_NUMBER, CJSON_OBJECT,
        CJSON_STRING, CJSON_TRUE,
    };
    use crate::number::saturate_to_int;
    use crate::store::NodeStore;

    /// `parse_buffer` (cJSON.c:295-302) without the hooks, which live in the
    /// store.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ParseBuffer<'a> {
        pub content: &'a [u8],
        pub length: usize,
        pub offset: usize,
        /// How deeply nested (in arrays/objects) the input is at `offset`.
        pub depth: usize,
    }

    impl<'a> ParseBuffer<'a> {
        /// Buffer over all of `content`: length = `content.len()`, offset 0,
        /// depth 0.
        pub fn new(content: &'a [u8]) -> Self {
            Self {
                content,
                length: content.len(),
                offset: 0,
                depth: 0,
            }
        }

        /// `can_read(buffer, size)` (cJSON.c:305).
        pub fn can_read(&self, size: usize) -> bool {
            self.offset.wrapping_add(size) <= self.length
        }

        /// `can_access_at_index(buffer, index)` (cJSON.c:307).
        pub fn can_access_at_index(&self, index: usize) -> bool {
            self.offset.wrapping_add(index) < self.length
        }

        /// `buffer_at_offset(buffer)[index]`; None where C would read
        /// outside the backing memory.
        pub fn byte_at(&self, index: usize) -> Option<u8> {
            self.content.get(self.offset.wrapping_add(index)).copied()
        }

        /// `strncmp(buffer_at_offset(buffer), literal, n) == 0` for a
        /// literal without NUL bytes.
        fn starts_with(&self, literal: &[u8]) -> bool {
            self.content
                .get(self.offset..)
                .is_some_and(|rest| rest.starts_with(literal))
        }
    }

    /// Stand-in for a temporary heap buffer the C code allocates and frees
    /// through `hooks` (the `strtod` copy in `parse_number`, cJSON.c:363,
    /// and a string output buffer that is freed on failure, cJSON.c:869).
    /// `NodeStore` has no raw-buffer allocator, so one node allocation is
    /// made and released immediately; this keeps the number and order of
    /// allocator calls (and therefore injected-failure behavior) identical
    /// to C. Returns false when the allocation fails.
    fn scratch_allocation<S: NodeStore>(store: &mut S) -> bool {
        match store.alloc_node() {
            Ok(id) => {
                store.free_node(id);
                true
            }
            Err(_) => false,
        }
    }

    /// `parse_hex4` (cJSON.c:672-705): four hex digits, or 0 if any of them
    /// is not a hex digit (so `\u` followed by garbage decodes as U+0000).
    /// Missing bytes (input shorter than 4) count as invalid digits.
    pub fn parse_hex4(input: &[u8]) -> u32 {
        let mut h: u32 = 0;
        for i in 0..4 {
            let c = match input.get(i) {
                Some(&c) => c,
                None => return 0,
            };
            let digit = match c {
                b'0'..=b'9' => u32::from(c - b'0'),
                b'A'..=b'F' => 10 + u32::from(c - b'A'),
                b'a'..=b'f' => 10 + u32::from(c - b'a'),
                _ => return 0,
            };
            h += digit;
            if i < 3 {
                h <<= 4;
            }
        }
        h
    }

    /// `utf16_literal_to_utf8` (cJSON.c:709-827). `input_pointer` is the
    /// index of the backslash of `\uXXXX`, `input_end` the index of the
    /// closing quote. Appends the UTF-8 encoding to `output` and returns the
    /// number of input bytes consumed (6 or 12), or 0 on failure.
    pub fn utf16_literal_to_utf8(
        content: &[u8],
        input_pointer: usize,
        input_end: usize,
        output: &mut Vec<u8>,
    ) -> u8 {
        let first_sequence = input_pointer;
        if input_end < first_sequence || input_end - first_sequence < 6 {
            return 0;
        }

        let hex_at = |start: usize| content.get(start..).map_or(0, parse_hex4);

        let first_code = hex_at(first_sequence.wrapping_add(2));

        if (0xDC00..=0xDFFF).contains(&first_code) {
            return 0;
        }

        let (sequence_length, mut codepoint): (u8, u32) = if (0xD800..=0xDBFF).contains(&first_code)
        {
            let second_sequence = first_sequence.wrapping_add(6);
            if input_end < second_sequence || input_end - second_sequence < 6 {
                return 0;
            }
            if content.get(second_sequence) != Some(&b'\\')
                || content.get(second_sequence.wrapping_add(1)) != Some(&b'u')
            {
                return 0;
            }
            let second_code = hex_at(second_sequence.wrapping_add(2));
            if !(0xDC00..=0xDFFF).contains(&second_code) {
                return 0;
            }
            (
                12,
                0x10000 + (((first_code & 0x3FF) << 10) | (second_code & 0x3FF)),
            )
        } else {
            (6, first_code)
        };

        let (utf8_length, first_byte_mark): (usize, u32) = if codepoint < 0x80 {
            (1, 0)
        } else if codepoint < 0x800 {
            (2, 0xC0)
        } else if codepoint < 0x10000 {
            (3, 0xE0)
        } else if codepoint <= 0x10FFFF {
            (4, 0xF0)
        } else {
            return 0;
        };

        let mut encoded = [0u8; 4];
        for slot in encoded.iter_mut().take(utf8_length).skip(1).rev() {
            *slot = ((codepoint | 0x80) & 0xBF) as u8;
            codepoint >>= 6;
        }
        if let Some(first) = encoded.first_mut() {
            *first = if utf8_length > 1 {
                ((codepoint | first_byte_mark) & 0xFF) as u8
            } else {
                (codepoint & 0x7F) as u8
            };
        }
        output.extend_from_slice(encoded.get(..utf8_length).unwrap_or(&[]));

        sequence_length
    }

    /// Emulates C `strtod` on the NUL-terminated copy `parse_number` makes
    /// (cJSON.c:384): the longest prefix of the form
    /// `[+-]? (digits [. digits?] | . digits) ([eE] [+-]? digits)?` is
    /// converted; an exponent without digits is not consumed. Returns the
    /// value and the number of bytes consumed, or None when nothing was
    /// consumed (`number_c_string == after_end`).
    ///
    /// `bytes` only ever contains `[0-9+-eE.]`, so the `inf`/`nan`/hex forms
    /// of `strtod` cannot occur. Rust's `f64` parser and the platform
    /// `strtod` are both correctly rounded, including overflow to infinity
    /// and gradual underflow to subnormals.
    ///
    /// Locale: without `ENABLE_LOCALES` (the Makefile default)
    /// `get_decimal_point` returns `'.'` (cJSON.c:285-293). With it, C
    /// substitutes the locale decimal point before calling the equally
    /// localized `strtod`, which yields the same value, so '.' is always
    /// used here.
    pub fn strtod_prefix(bytes: &[u8]) -> Option<(f64, usize)> {
        let is_digit = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);

        let mut i = 0;
        if matches!(bytes.first(), Some(b'+' | b'-')) {
            i += 1;
        }
        let mut mantissa_digits = 0usize;
        while is_digit(i) {
            i += 1;
            mantissa_digits += 1;
        }
        if bytes.get(i) == Some(&b'.') {
            i += 1;
            while is_digit(i) {
                i += 1;
                mantissa_digits += 1;
            }
        }
        if mantissa_digits == 0 {
            return None;
        }
        if matches!(bytes.get(i), Some(b'e' | b'E')) {
            let mut j = i + 1;
            if matches!(bytes.get(j), Some(b'+' | b'-')) {
                j += 1;
            }
            let exponent_start = j;
            while is_digit(j) {
                j += 1;
            }
            if j > exponent_start {
                i = j;
            }
        }

        let text = std::str::from_utf8(bytes.get(..i)?).ok()?;
        let value = text.parse::<f64>().ok()?;
        Some((value, i))
    }

    /// `parse_number` (cJSON.c:313-414). On failure the offset is left
    /// unchanged.
    pub fn parse_number<S: NodeStore>(
        store: &mut S,
        item: S::Id,
        input_buffer: &mut ParseBuffer<'_>,
    ) -> bool {
        // cJSON.c:331-361: collect [0-9+-eE.] (bounded by the buffer length,
        // so a missing NUL terminator is fine).
        let mut number_string_length = 0usize;
        while input_buffer.can_access_at_index(number_string_length) {
            match input_buffer.byte_at(number_string_length) {
                Some(b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' | b'.') => number_string_length += 1,
                _ => break,
            }
        }

        // cJSON.c:363: temporary buffer allocation.
        if !scratch_allocation(store) {
            return false;
        }

        let start = input_buffer.offset;
        let number_string = input_buffer
            .content
            .get(start..)
            .and_then(|rest| rest.get(..number_string_length))
            .unwrap_or(&[]);

        let (number, consumed) = match strtod_prefix(number_string) {
            Some(parsed) => parsed,
            None => return false,
        };

        store.set_valuedouble(item, number);
        store.set_valueint(item, saturate_to_int(number));
        store.set_type_bits(item, CJSON_NUMBER);

        input_buffer.offset = input_buffer.offset.wrapping_add(consumed);
        true
    }

    /// Where `parse_string` stores its result.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum StringTarget {
        /// `item->valuestring` (every string value).
        ValueString,
        /// `item->string`: object keys. C parses the key into `valuestring`
        /// and then moves the pointer (cJSON.c:1744-1746); storing it
        /// directly keeps the single allocation C makes.
        Key,
    }

    /// `parse_string` (cJSON.c:830-957).
    ///
    /// Offset on failure (cJSON.c:951-954): `input_pointer`, which starts at
    /// `offset + 1`. So "not a string", an unterminated string, a trailing
    /// backslash and an allocation failure all report `offset + 1`; an
    /// invalid escape reports the index of its backslash. On success the
    /// offset is just past the closing quote. Bytes are copied raw (no UTF-8
    /// validation) and `\u0000` stores an interior NUL.
    pub fn parse_string<S: NodeStore>(
        store: &mut S,
        item: S::Id,
        input_buffer: &mut ParseBuffer<'_>,
    ) -> bool {
        parse_string_into(store, item, input_buffer, StringTarget::ValueString)
    }

    fn parse_string_into<S: NodeStore>(
        store: &mut S,
        item: S::Id,
        input_buffer: &mut ParseBuffer<'_>,
        target: StringTarget,
    ) -> bool {
        let content = input_buffer.content;
        let length = input_buffer.length;
        let start = input_buffer.offset;
        let first_pointer = start.wrapping_add(1);

        let fail_at = |buffer: &mut ParseBuffer<'_>, input_pointer: usize| {
            buffer.offset = input_pointer;
            false
        };

        // cJSON.c:838-841: not a string.
        if content.get(start) != Some(&b'"') {
            return fail_at(input_buffer, first_pointer);
        }

        // cJSON.c:843-865: find the closing quote.
        let mut input_end = first_pointer;
        loop {
            if input_end >= length {
                break;
            }
            let c = match content.get(input_end) {
                Some(&c) => c,
                None => return fail_at(input_buffer, first_pointer),
            };
            if c == b'"' {
                break;
            }
            if c == b'\\' {
                if input_end.wrapping_add(1) >= length {
                    return fail_at(input_buffer, first_pointer);
                }
                input_end = input_end.wrapping_add(1);
            }
            input_end = input_end.wrapping_add(1);
        }
        if input_end >= length || content.get(input_end) != Some(&b'"') {
            return fail_at(input_buffer, first_pointer);
        }

        // cJSON.c:876-931: unescape. C allocates the output first (869) and
        // runs this loop afterwards; the allocation is replayed below in
        // that order so an allocation failure takes precedence.
        let mut output = Vec::with_capacity(input_end - start);
        let mut input_pointer = first_pointer;
        let mut escape_error: Option<usize> = None;
        while input_pointer < input_end {
            let c = match content.get(input_pointer) {
                Some(&c) => c,
                None => {
                    escape_error = Some(input_pointer);
                    break;
                }
            };
            if c != b'\\' {
                output.push(c);
                input_pointer += 1;
                continue;
            }
            let sequence_length: u8 = match content.get(input_pointer.wrapping_add(1)) {
                Some(b'b') => {
                    output.push(0x08);
                    2
                }
                Some(b'f') => {
                    output.push(0x0C);
                    2
                }
                Some(b'n') => {
                    output.push(b'\n');
                    2
                }
                Some(b'r') => {
                    output.push(b'\r');
                    2
                }
                Some(b't') => {
                    output.push(b'\t');
                    2
                }
                Some(&e @ (b'"' | b'\\' | b'/')) => {
                    output.push(e);
                    2
                }
                Some(b'u') => utf16_literal_to_utf8(content, input_pointer, input_end, &mut output),
                _ => 0,
            };
            if sequence_length == 0 {
                escape_error = Some(input_pointer);
                break;
            }
            input_pointer = input_pointer.wrapping_add(usize::from(sequence_length));
        }

        if let Some(error_pointer) = escape_error {
            // C allocated the output buffer, then frees it on the fail path.
            if !scratch_allocation(store) {
                return fail_at(input_buffer, first_pointer);
            }
            return fail_at(input_buffer, error_pointer);
        }

        let stored = match target {
            StringTarget::ValueString => store.set_valuestring_copy(item, &output),
            StringTarget::Key => store.set_string_copy(item, &output),
        };
        if stored.is_err() {
            return fail_at(input_buffer, first_pointer);
        }

        store.set_type_bits(item, CJSON_STRING);
        // cJSON.c:939-940
        input_buffer.offset = input_end.wrapping_add(1);
        true
    }

    /// `buffer_skip_whitespace` (cJSON.c:1096-1119): skips bytes <= 32. If
    /// that reaches the end of the buffer the offset is stepped back by one
    /// (cJSON.c:1113-1116), so it points at the last byte.
    pub fn buffer_skip_whitespace(buffer: &mut ParseBuffer<'_>) {
        if !buffer.can_access_at_index(0) {
            return;
        }
        while buffer.can_access_at_index(0) && buffer.byte_at(0).is_some_and(|c| c <= 32) {
            buffer.offset = buffer.offset.wrapping_add(1);
        }
        if buffer.offset == buffer.length {
            buffer.offset = buffer.offset.wrapping_sub(1);
        }
    }

    /// `skip_utf8_bom` (cJSON.c:1122-1135): true where C returns the buffer,
    /// false where it returns NULL (offset != 0). The BOM is only skipped if
    /// `can_access_at_index(buffer, 4)`, i.e. at least 5 bytes are present.
    pub fn skip_utf8_bom(buffer: &mut ParseBuffer<'_>) -> bool {
        if buffer.offset != 0 {
            return false;
        }
        if buffer.can_access_at_index(4) && buffer.starts_with(b"\xEF\xBB\xBF") {
            buffer.offset = buffer.offset.wrapping_add(3);
        }
        true
    }

    /// `parse_value` (cJSON.c:1380-1432).
    pub fn parse_value<S: NodeStore>(
        store: &mut S,
        item: S::Id,
        input_buffer: &mut ParseBuffer<'_>,
    ) -> bool {
        if input_buffer.can_read(4) && input_buffer.starts_with(b"null") {
            store.set_type_bits(item, CJSON_NULL);
            input_buffer.offset = input_buffer.offset.wrapping_add(4);
            return true;
        }
        if input_buffer.can_read(5) && input_buffer.starts_with(b"false") {
            store.set_type_bits(item, CJSON_FALSE);
            input_buffer.offset = input_buffer.offset.wrapping_add(5);
            return true;
        }
        if input_buffer.can_read(4) && input_buffer.starts_with(b"true") {
            store.set_type_bits(item, CJSON_TRUE);
            store.set_valueint(item, 1);
            input_buffer.offset = input_buffer.offset.wrapping_add(4);
            return true;
        }
        if !input_buffer.can_access_at_index(0) {
            return false;
        }
        match input_buffer.byte_at(0) {
            Some(b'"') => parse_string(store, item, input_buffer),
            Some(b'-' | b'0'..=b'9') => parse_number(store, item, input_buffer),
            Some(b'[') => parse_array(store, item, input_buffer),
            Some(b'{') => parse_object(store, item, input_buffer),
            _ => false,
        }
    }

    /// Appends a freshly allocated node to the list being built
    /// (cJSON.c:1553-1565 / 1716-1728).
    fn append_new_item<S: NodeStore>(
        store: &mut S,
        head: &mut Option<S::Id>,
        current_item: &mut Option<S::Id>,
    ) -> Option<S::Id> {
        let new_item = store.alloc_node().ok()?;
        match *current_item {
            Some(current) => {
                store.set_next(current, Some(new_item));
                store.set_prev(new_item, Some(current));
            }
            None => *head = Some(new_item),
        }
        *current_item = Some(new_item);
        Some(new_item)
    }

    /// `parse_array` (cJSON.c:1509-1604). Quirks kept: the depth counter is
    /// not decremented on failure; `head->prev` is the last element.
    pub fn parse_array<S: NodeStore>(
        store: &mut S,
        item: S::Id,
        input_buffer: &mut ParseBuffer<'_>,
    ) -> bool {
        let mut head: Option<S::Id> = None;
        let mut current_item: Option<S::Id> = None;

        if input_buffer.depth >= CJSON_NESTING_LIMIT {
            return false;
        }
        input_buffer.depth = input_buffer.depth.wrapping_add(1);

        let ok = 'parse: {
            if input_buffer.byte_at(0) != Some(b'[') {
                break 'parse false;
            }

            input_buffer.offset = input_buffer.offset.wrapping_add(1);
            buffer_skip_whitespace(input_buffer);
            if input_buffer.can_access_at_index(0) && input_buffer.byte_at(0) == Some(b']') {
                break 'parse true;
            }

            if !input_buffer.can_access_at_index(0) {
                input_buffer.offset = input_buffer.offset.wrapping_sub(1);
                break 'parse false;
            }

            input_buffer.offset = input_buffer.offset.wrapping_sub(1);
            loop {
                let new_item = match append_new_item(store, &mut head, &mut current_item) {
                    Some(new_item) => new_item,
                    None => break 'parse false,
                };

                input_buffer.offset = input_buffer.offset.wrapping_add(1);
                buffer_skip_whitespace(input_buffer);
                if !parse_value(store, new_item, input_buffer) {
                    break 'parse false;
                }
                buffer_skip_whitespace(input_buffer);

                if !(input_buffer.can_access_at_index(0) && input_buffer.byte_at(0) == Some(b',')) {
                    break;
                }
            }

            input_buffer.can_access_at_index(0) && input_buffer.byte_at(0) == Some(b']')
        };

        if !ok {
            if head.is_some() {
                crate::tree::delete(store, head);
            }
            return false;
        }

        // success: (cJSON.c:1583-1595)
        input_buffer.depth = input_buffer.depth.wrapping_sub(1);
        if let Some(h) = head {
            store.set_prev(h, current_item);
        }
        store.set_type_bits(item, CJSON_ARRAY);
        store.set_child(item, head);
        input_buffer.offset = input_buffer.offset.wrapping_add(1);
        true
    }

    /// `parse_object` (cJSON.c:1674-1789). Same list/depth quirks as
    /// [`parse_array`]; keys are parsed with `parse_string`, so a non-string
    /// key reports `offset + 1` of the key position.
    pub fn parse_object<S: NodeStore>(
        store: &mut S,
        item: S::Id,
        input_buffer: &mut ParseBuffer<'_>,
    ) -> bool {
        let mut head: Option<S::Id> = None;
        let mut current_item: Option<S::Id> = None;

        if input_buffer.depth >= CJSON_NESTING_LIMIT {
            return false;
        }
        input_buffer.depth = input_buffer.depth.wrapping_add(1);

        let ok = 'parse: {
            if !input_buffer.can_access_at_index(0) || input_buffer.byte_at(0) != Some(b'{') {
                break 'parse false;
            }

            input_buffer.offset = input_buffer.offset.wrapping_add(1);
            buffer_skip_whitespace(input_buffer);
            if input_buffer.can_access_at_index(0) && input_buffer.byte_at(0) == Some(b'}') {
                break 'parse true;
            }

            if !input_buffer.can_access_at_index(0) {
                input_buffer.offset = input_buffer.offset.wrapping_sub(1);
                break 'parse false;
            }

            input_buffer.offset = input_buffer.offset.wrapping_sub(1);
            loop {
                let new_item = match append_new_item(store, &mut head, &mut current_item) {
                    Some(new_item) => new_item,
                    None => break 'parse false,
                };

                // cJSON.c:1730: nothing comes after the comma.
                if !input_buffer.can_access_at_index(1) {
                    break 'parse false;
                }

                input_buffer.offset = input_buffer.offset.wrapping_add(1);
                buffer_skip_whitespace(input_buffer);
                if !parse_string_into(store, new_item, input_buffer, StringTarget::Key) {
                    break 'parse false;
                }
                buffer_skip_whitespace(input_buffer);

                if !input_buffer.can_access_at_index(0) || input_buffer.byte_at(0) != Some(b':') {
                    break 'parse false;
                }

                input_buffer.offset = input_buffer.offset.wrapping_add(1);
                buffer_skip_whitespace(input_buffer);
                if !parse_value(store, new_item, input_buffer) {
                    break 'parse false;
                }
                buffer_skip_whitespace(input_buffer);

                if !(input_buffer.can_access_at_index(0) && input_buffer.byte_at(0) == Some(b',')) {
                    break;
                }
            }

            input_buffer.can_access_at_index(0) && input_buffer.byte_at(0) == Some(b'}')
        };

        if !ok {
            if head.is_some() {
                crate::tree::delete(store, head);
            }
            return false;
        }

        input_buffer.depth = input_buffer.depth.wrapping_sub(1);
        if let Some(h) = head {
            store.set_prev(h, current_item);
        }
        store.set_type_bits(item, CJSON_OBJECT);
        store.set_child(item, head);
        input_buffer.offset = input_buffer.offset.wrapping_add(1);
        true
    }
}
