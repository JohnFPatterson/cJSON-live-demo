//! Printer: port of `print_value`, `print_number`, `print_string_ptr`,
//! `print_array`, `print_object`, `ensure`, `update_offset`, `print`,
//! `cJSON_PrintBuffered` and `cJSON_PrintPreallocated`
//! (cJSON.c:484-1080, 1239-1378, 1434-1905).
//!
//! Public signatures in this file are a contract used by `cjson-ffi` and `tools/rust-driver`; do not change them without
//! updating those callers.
//!
//! The C printer writes into a `printbuffer` through `ensure()` and relies on
//! the exact sequence of `ensure()` requests for `cJSON_PrintPreallocated`
//! success/failure, and on `strlen` (`update_offset`) to advance past bytes a
//! helper wrote without moving `offset`. [`internals`] reproduces both: every
//! helper requests the same sizes in the same order and writes the same bytes
//! at the same positions, including the NUL terminators C writes.

use crate::store::NodeStore;

use internals::PrintBuffer;

/// `default_buffer_size` in `print()` (cJSON.c:1250).
const DEFAULT_BUFFER_SIZE: usize = 256;

/// `cJSON_Print` (format = true) / `cJSON_PrintUnformatted` (format = false).
/// Returns the output bytes WITHOUT the trailing NUL, or None where C returns
/// NULL (invalid type bits, nesting limit, raw with NULL valuestring, ...).
/// `item == None` models a NULL `cJSON *`.
///
/// Port of `print()` (cJSON.c:1248-1313): a growable buffer of 256 bytes,
/// `print_value`, then `update_offset`; the result is the first `offset`
/// bytes. The final `reallocate`/`allocate` + `memcpy` step does not change
/// the bytes.
pub fn print<S: NodeStore>(store: &S, item: Option<S::Id>, format: bool) -> Option<Vec<u8>> {
    let mut buffer = PrintBuffer::new(DEFAULT_BUFFER_SIZE);
    buffer.format = format;
    let item = item?;
    if !internals::print_value(store, item, &mut buffer) {
        return None;
    }
    internals::update_offset(&mut buffer);
    let offset = buffer.offset;
    buffer
        .buffer
        .and_then(|b| b.get(..offset).map(<[u8]>::to_vec))
}

/// `cJSON_PrintBuffered(item, prebuffer, fmt)`: None when `prebuffer < 0`
/// or printing fails, else the same bytes C returns (without NUL).
///
/// Port of cJSON.c:1328-1357. C returns `p.buffer` itself (no
/// `update_offset`), so the caller sees the bytes before the first NUL.
/// `malloc(0)` for `prebuffer == 0` is modeled as returning a non-NULL
/// pointer (glibc and macOS behavior); growth then goes through `ensure`.
pub fn print_buffered<S: NodeStore>(
    store: &S,
    item: Option<S::Id>,
    prebuffer: i32,
    format: bool,
) -> Option<Vec<u8>> {
    let length = usize::try_from(prebuffer).ok()?;
    let mut p = PrintBuffer::new(length);
    p.format = format;
    let item = item?;
    if !internals::print_value(store, item, &mut p) {
        return None;
    }
    Some(p.c_str().to_vec())
}

/// Result of [`print_preallocated`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreallocatedOutcome {
    /// The `cJSON_bool` C returns.
    pub ok: bool,
    /// The bytes C writes into the caller's buffer, starting at index 0,
    /// including the NUL terminator(s) C writes. Never longer than `length`.
    /// On failure this is whatever prefix C had written before giving up.
    pub written: Vec<u8>,
}

/// `cJSON_PrintPreallocated(item, buffer, length, format)`, emulating
/// `ensure()` with `noalloc = true` so success/failure matches C exactly
/// at every buffer length. `length < 0` is rejected by the FFI before this
/// is called (C returns false without writing).
///
/// Port of cJSON.c:1360-1377. Note C calls `print_value` only (no
/// `update_offset`), and a `length` of 0 always fails because `ensure`
/// needs `offset + needed + 1 <= length` (cJSON.c:518-522). The caller's
/// buffer is not allocated here: only the bytes C writes are materialized.
pub fn print_preallocated<S: NodeStore>(
    store: &S,
    item: Option<S::Id>,
    length: usize,
    format: bool,
) -> PreallocatedOutcome {
    let mut p = PrintBuffer::new(length);
    p.noalloc = true;
    p.format = format;
    let ok = match item {
        Some(id) => internals::print_value(store, id, &mut p),
        None => false,
    };
    PreallocatedOutcome {
        ok,
        written: p.buffer.unwrap_or_default(),
    }
}

/// Low-level printer pieces with the same shape as the static C helpers,
/// for the ported Unity tests (`print_number.c`, `print_string.c`,
/// `print_array.c`, `print_object.c`, `print_value.c`, `misc_tests.c`) and
/// the parity driver.
pub mod internals {
    use crate::consts::{
        CJSON_ARRAY, CJSON_FALSE, CJSON_NESTING_LIMIT, CJSON_NULL, CJSON_NUMBER, CJSON_OBJECT,
        CJSON_RAW, CJSON_STRING, CJSON_TRUE,
    };
    use crate::store::NodeStore;

    /// `INT_MAX` as used by `ensure` (cJSON.c:512, 529-535).
    const INT_MAX: usize = i32::MAX as usize;

    /// `sizeof(number_buffer) - 1` in `print_number` (cJSON.c:608, 640).
    const NUMBER_BUFFER_MAX: usize = 25;

    /// The C `printbuffer` (cJSON.c:484-493).
    ///
    /// `buffer` holds the bytes C has written so far, from index 0 up to the
    /// highest index written (C memory past that point is uninitialized and
    /// never read by the printer). `None` models `buffer == NULL`. `length`
    /// is the C allocation size, which may be larger than `buffer.len()`.
    /// `hooks` are reduced to `fail_realloc`, which models a `reallocate`
    /// hook that returns NULL.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PrintBuffer {
        pub buffer: Option<Vec<u8>>,
        pub length: usize,
        pub offset: usize,
        pub depth: usize,
        pub noalloc: bool,
        pub format: bool,
        pub fail_realloc: bool,
    }

    impl PrintBuffer {
        /// A non-NULL buffer of `length` bytes, offset 0, depth 0,
        /// noalloc false, format false, fail_realloc false.
        pub fn new(length: usize) -> Self {
            Self {
                buffer: Some(Vec::new()),
                length,
                offset: 0,
                depth: 0,
                noalloc: false,
                format: false,
                fail_realloc: false,
            }
        }

        /// The buffer read as a C string: bytes before the first NUL
        /// (what `TEST_ASSERT_EQUAL_STRING(expected, buffer.buffer)` sees).
        pub fn c_str(&self) -> &[u8] {
            let bytes = self.buffer.as_deref().unwrap_or(&[]);
            match bytes.iter().position(|&b| b == 0) {
                Some(end) => bytes.get(..end).unwrap_or(bytes),
                None => bytes,
            }
        }
    }

    /// `ensure` (cJSON.c:496-579): make room for `needed` more bytes plus a
    /// terminator. Returns the offset where C returns `buffer + offset`, and
    /// None where C returns NULL.
    ///
    /// Quirks kept: fails when `length > 0 && offset >= length`
    /// (cJSON.c:506-510) and when `needed > INT_MAX` (cJSON.c:512-516);
    /// succeeds only if `offset + needed + 1 <= length` (cJSON.c:518-522);
    /// with `noalloc` never grows (cJSON.c:524-526); grows to `needed * 2`,
    /// or `INT_MAX` once `needed > INT_MAX / 2` (cJSON.c:529-544). A failed
    /// reallocation frees the buffer and sets `length = 0`, `buffer = NULL`
    /// (cJSON.c:550-556), so every later `ensure` fails too.
    pub fn ensure(p: &mut PrintBuffer, needed: usize) -> Option<usize> {
        p.buffer.as_ref()?;
        if p.length > 0 && p.offset >= p.length {
            return None;
        }
        if needed > INT_MAX {
            return None;
        }
        let needed = needed.checked_add(p.offset)?.checked_add(1)?;
        if needed <= p.length {
            return Some(p.offset);
        }
        if p.noalloc {
            return None;
        }
        let newsize = if needed > INT_MAX / 2 {
            if needed <= INT_MAX {
                INT_MAX
            } else {
                return None;
            }
        } else {
            needed.checked_mul(2)?
        };
        let grown = match p.buffer.as_mut() {
            Some(buf) if !p.fail_realloc => buf
                .try_reserve_exact(newsize.saturating_sub(buf.len()))
                .is_ok(),
            _ => false,
        };
        if !grown {
            p.buffer = None;
            p.length = 0;
            return None;
        }
        p.length = newsize;
        Some(p.offset)
    }

    /// `update_offset` (cJSON.c:582-592): `offset += strlen(buffer + offset)`.
    /// Every print helper leaves a NUL right after its output, so the scan
    /// never leaves bytes C wrote; if it did find none it stops at the end of
    /// the written bytes.
    pub fn update_offset(p: &mut PrintBuffer) {
        let Some(buf) = p.buffer.as_ref() else {
            return;
        };
        let tail = buf.get(p.offset..).unwrap_or(&[]);
        let len = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
        p.offset = p.offset.saturating_add(len);
    }

    /// Write `bytes` at absolute index `at`, as the C code does through the
    /// pointer `ensure` returned. Every caller writes inside the region
    /// `ensure` just granted, so the bounds check never fails in practice;
    /// if it did, the write is refused and the print fails instead of
    /// overrunning.
    fn write_at(p: &mut PrintBuffer, at: usize, bytes: &[u8]) -> bool {
        let Some(end) = at.checked_add(bytes.len()) else {
            return false;
        };
        if end > p.length {
            return false;
        }
        let Some(buf) = p.buffer.as_mut() else {
            return false;
        };
        if buf.len() < at {
            buf.resize(at, 0);
        }
        let overlap = buf.len().min(end) - at;
        let (head, tail) = bytes.split_at(overlap);
        if let Some(dst) = buf.get_mut(at..at + overlap) {
            dst.copy_from_slice(head);
        }
        buf.extend_from_slice(tail);
        true
    }

    /// `compare_double` (cJSON.c:595-599):
    /// `fabs(a - b) <= max(fabs(a), fabs(b)) * DBL_EPSILON`.
    pub fn compare_double(a: f64, b: f64) -> bool {
        let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
        (a - b).abs() <= max_val * f64::EPSILON
    }

    /// C `printf("%1.<precision>g", value)` for a finite `value`.
    ///
    /// Built from Rust's exact, correctly rounded `{:.*e}` formatting (the
    /// same digits glibc and macOS libc produce): with P = precision
    /// (0 treated as 1) and X the decimal exponent after rounding to P
    /// significant digits, C uses `%.(P-1-X)f` when `P > X >= -4` and
    /// `%.(P-1)e` otherwise, then removes trailing zeros and a trailing
    /// decimal point. The exponent has a sign and at least two digits; the
    /// sign of `-0.0` is kept. Width 1 never pads. Non-finite values are
    /// never passed here (`print_number` handles them first).
    pub fn format_g(value: f64, precision: usize) -> Vec<u8> {
        let p = precision.max(1);
        let sci = format!("{:.*e}", p - 1, value);
        let (negative, body) = match sci.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, sci.as_str()),
        };
        let (mantissa, exp_text) = body.split_once('e').unwrap_or((body, "0"));
        let exponent: i64 = exp_text.parse().unwrap_or(0);
        let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();

        let mut out = Vec::with_capacity(32);
        if negative {
            out.push(b'-');
        }
        let p_i64 = i64::try_from(p).unwrap_or(i64::MAX);
        if exponent < p_i64 && exponent >= -4 {
            let mut fixed = Vec::with_capacity(digits.len() + 6);
            let int_len = if exponent >= 0 {
                usize::try_from(exponent).unwrap_or(0).saturating_add(1)
            } else {
                0
            };
            if exponent >= 0 {
                let (int_part, frac_part) = digits.split_at(int_len.min(digits.len()));
                fixed.extend_from_slice(int_part);
                fixed.push(b'.');
                fixed.extend_from_slice(frac_part);
            } else {
                fixed.extend_from_slice(b"0.");
                let zeros = usize::try_from(-exponent - 1).unwrap_or(0);
                fixed.extend_from_slice(&vec![b'0'; zeros]);
                fixed.extend_from_slice(&digits);
            }
            strip_fraction_zeros(&mut fixed);
            out.extend_from_slice(&fixed);
        } else {
            let mut mant = Vec::with_capacity(digits.len() + 1);
            let (first, rest) = digits.split_at(digits.len().min(1));
            mant.extend_from_slice(first);
            mant.push(b'.');
            mant.extend_from_slice(rest);
            strip_fraction_zeros(&mut mant);
            out.extend_from_slice(&mant);
            out.push(b'e');
            out.push(if exponent < 0 { b'-' } else { b'+' });
            let magnitude = exponent.unsigned_abs();
            if magnitude < 10 {
                out.push(b'0');
            }
            out.extend_from_slice(magnitude.to_string().as_bytes());
        }
        out
    }

    /// `%g` without `#`: drop trailing zeros after the decimal point, then
    /// the point itself if nothing follows it.
    fn strip_fraction_zeros(text: &mut Vec<u8>) {
        if !text.contains(&b'.') {
            return;
        }
        while text.last() == Some(&b'0') {
            text.pop();
        }
        if text.last() == Some(&b'.') {
            text.pop();
        }
    }

    /// The text `print_number` puts into `number_buffer` (cJSON.c:617-637):
    /// NaN/Inf -> `null`; `d == (double)valueint` -> `%d` of `valueint`
    /// (so `-0.0` with `valueint == 0` prints `0`); else `%1.15g`, falling
    /// back to `%1.17g` when `sscanf("%lg")` of the 15-digit text does not
    /// round-trip per [`compare_double`].
    ///
    /// Locale: C prints with the locale's decimal point and then maps
    /// `get_decimal_point()` back to `.` while copying (cJSON.c:652-663). In
    /// the default build `get_decimal_point()` is `.`; with `ENABLE_LOCALES`
    /// the round trip is exact, and `sscanf` uses the same locale as
    /// `sprintf`, so the output is always the C-locale text produced here.
    pub fn format_number(valuedouble: f64, valueint: i32) -> Vec<u8> {
        let d = valuedouble;
        if d.is_nan() || d.is_infinite() {
            return b"null".to_vec();
        }
        if d == f64::from(valueint) {
            return valueint.to_string().into_bytes();
        }
        let short = format_g(d, 15);
        let round_trips = core::str::from_utf8(&short)
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .is_some_and(|test| compare_double(test, d));
        if round_trips {
            short
        } else {
            format_g(d, 17)
        }
    }

    /// `print_number` (cJSON.c:602-669). Requests `length + sizeof("")`
    /// bytes, writes the digits and a NUL, and advances `offset` by
    /// `length` (the NUL is left at `offset`).
    pub fn print_number<S: NodeStore>(store: &S, item: S::Id, p: &mut PrintBuffer) -> bool {
        let text = format_number(store.valuedouble(item), store.valueint(item));
        if text.len() > NUMBER_BUFFER_MAX {
            return false;
        }
        let Some(at) = ensure(p, text.len() + 1) else {
            return false;
        };
        if !write_at(p, at, &text) || !write_at(p, at.saturating_add(text.len()), b"\0") {
            return false;
        }
        p.offset = p.offset.saturating_add(text.len());
        true
    }

    /// `print_string_ptr` (cJSON.c:960-1079). `input == None` models a NULL
    /// pointer and prints `""`. Only bytes before the first NUL are printed
    /// (C iterates to the terminator). Does NOT advance `offset`: callers
    /// run [`update_offset`], as in C.
    ///
    /// Requests `3` bytes for NULL (cJSON.c:977), else `len + escapes + 3`
    /// (cJSON.c:1013), where `"`, `\`, `\b`, `\f`, `\n`, `\r`, `\t` cost one
    /// extra byte and other bytes below 0x20 cost five (`\u00XX`, lowercase
    /// hex via `sprintf("u%04x")`, whose NUL is overwritten afterwards).
    /// Bytes >= 0x7F (including invalid UTF-8) are copied verbatim.
    pub fn print_string_ptr(input: Option<&[u8]>, p: &mut PrintBuffer) -> bool {
        let Some(input) = input else {
            let Some(at) = ensure(p, 3) else {
                return false;
            };
            return write_at(p, at, b"\"\"\0");
        };
        let input = match input.iter().position(|&b| b == 0) {
            Some(end) => input.get(..end).unwrap_or(input),
            None => input,
        };

        let mut escape_characters: usize = 0;
        for &c in input {
            let extra = match c {
                b'"' | b'\\' | 0x08 | 0x0C | b'\n' | b'\r' | b'\t' => 1,
                c if c < 32 => 5,
                _ => 0,
            };
            escape_characters = escape_characters.saturating_add(extra);
        }
        let Some(output_length) = input.len().checked_add(escape_characters) else {
            return false;
        };
        let Some(request) = output_length.checked_add(3) else {
            return false;
        };
        let Some(at) = ensure(p, request) else {
            return false;
        };

        if escape_characters == 0 {
            let mut out = Vec::with_capacity(request);
            out.push(b'"');
            out.extend_from_slice(input);
            out.extend_from_slice(b"\"\0");
            return write_at(p, at, &out);
        }

        // Mirrors the C write sequence byte for byte, including the NUL that
        // `sprintf("u%04x")` writes one past the escape (cJSON.c:1069), which
        // the next byte or the closing quote overwrites.
        let mut out: Vec<u8> = Vec::with_capacity(request);
        out.push(b'"');
        let mut pos: usize = 1;
        for &c in input {
            if c > 31 && c != b'"' && c != b'\\' {
                put(&mut out, pos, c);
            } else {
                put(&mut out, pos, b'\\');
                pos += 1;
                match c {
                    b'\\' => put(&mut out, pos, b'\\'),
                    b'"' => put(&mut out, pos, b'"'),
                    0x08 => put(&mut out, pos, b'b'),
                    0x0C => put(&mut out, pos, b'f'),
                    b'\n' => put(&mut out, pos, b'n'),
                    b'\r' => put(&mut out, pos, b'r'),
                    b'\t' => put(&mut out, pos, b't'),
                    _ => {
                        let escaped = format!("u{c:04x}\0");
                        for (k, b) in escaped.bytes().enumerate() {
                            put(&mut out, pos + k, b);
                        }
                        pos += 4;
                    }
                }
            }
            pos += 1;
        }
        put(&mut out, output_length + 1, b'"');
        put(&mut out, output_length + 2, 0);
        write_at(p, at, &out)
    }

    /// Store `byte` at `index` of a scratch vector, growing it as needed.
    fn put(out: &mut Vec<u8>, index: usize, byte: u8) {
        if out.len() <= index {
            out.resize(index + 1, 0);
        }
        if let Some(slot) = out.get_mut(index) {
            *slot = byte;
        }
    }

    /// `print_string` (cJSON.c:1082-1085): `print_string_ptr(item->valuestring)`.
    pub fn print_string<S: NodeStore>(store: &S, item: S::Id, p: &mut PrintBuffer) -> bool {
        print_string_ptr(store.valuestring(item), p)
    }

    /// `print_value` (cJSON.c:1435-1506): dispatch on `type & 0xFF`.
    /// Only the exact single type bits print; anything else (including
    /// `cJSON_Invalid` and combined bits) fails. Literals request
    /// `strlen + 1` bytes and write their NUL without advancing `offset`.
    /// `cJSON_Raw` with a NULL valuestring fails; otherwise it copies the raw
    /// text verbatim (no validation) plus its NUL.
    pub fn print_value<S: NodeStore>(store: &S, item: S::Id, p: &mut PrintBuffer) -> bool {
        let literal: &[u8] = match store.type_bits(item) & 0xFF {
            CJSON_NULL => b"null\0",
            CJSON_FALSE => b"false\0",
            CJSON_TRUE => b"true\0",
            CJSON_NUMBER => return print_number(store, item, p),
            CJSON_RAW => {
                let Some(raw) = store.valuestring(item) else {
                    return false;
                };
                let Some(raw_length) = raw.len().checked_add(1) else {
                    return false;
                };
                let Some(at) = ensure(p, raw_length) else {
                    return false;
                };
                return write_at(p, at, raw) && write_at(p, at.saturating_add(raw.len()), b"\0");
            }
            CJSON_STRING => return print_string(store, item, p),
            CJSON_ARRAY => return print_array(store, item, p),
            CJSON_OBJECT => return print_object(store, item, p),
            _ => return false,
        };
        let Some(at) = ensure(p, literal.len()) else {
            return false;
        };
        write_at(p, at, literal)
    }

    /// `print_array` (cJSON.c:1607-1671). Fails at `depth >=
    /// CJSON_NESTING_LIMIT`. Separator is `,` or `, ` when formatted (no
    /// newlines in arrays). Leaves `]` + NUL at `offset` without advancing
    /// past them. `depth` is not restored on failure (as in C).
    pub fn print_array<S: NodeStore>(store: &S, item: S::Id, p: &mut PrintBuffer) -> bool {
        if p.depth >= CJSON_NESTING_LIMIT {
            return false;
        }
        let Some(at) = ensure(p, 1) else {
            return false;
        };
        if !write_at(p, at, b"[") {
            return false;
        }
        p.offset = p.offset.saturating_add(1);
        p.depth = p.depth.saturating_add(1);

        let mut current = store.child(item);
        while let Some(element) = current {
            if !print_value(store, element, p) {
                return false;
            }
            update_offset(p);
            if store.next(element).is_some() {
                let sep: &[u8] = if p.format { b", \0" } else { b",\0" };
                let length = sep.len() - 1;
                let Some(at) = ensure(p, length + 1) else {
                    return false;
                };
                if !write_at(p, at, sep) {
                    return false;
                }
                p.offset = p.offset.saturating_add(length);
            }
            current = store.next(element);
        }

        let Some(at) = ensure(p, 2) else {
            return false;
        };
        if !write_at(p, at, b"]\0") {
            return false;
        }
        p.depth = p.depth.saturating_sub(1);
        true
    }

    /// `print_object` (cJSON.c:1792-1908). Fails at `depth >=
    /// CJSON_NESTING_LIMIT`. Formatted output: `{\n`, `depth` tabs before
    /// each key, `:\t` after it, `,\n` / `\n` after each member, `depth - 1`
    /// tabs before `}`; so an empty object prints `{\n}`. Keys go through
    /// [`print_string_ptr`] (a NULL key prints `""`). Note the `:` request
    /// is `length` (not `length + 1`, cJSON.c:1849) because no NUL is written.
    pub fn print_object<S: NodeStore>(store: &S, item: S::Id, p: &mut PrintBuffer) -> bool {
        if p.depth >= CJSON_NESTING_LIMIT {
            return false;
        }
        let open: &[u8] = if p.format { b"{\n" } else { b"{" };
        let Some(at) = ensure(p, open.len() + 1) else {
            return false;
        };
        if !write_at(p, at, open) {
            return false;
        }
        p.depth = p.depth.saturating_add(1);
        p.offset = p.offset.saturating_add(open.len());

        let mut current = store.child(item);
        while let Some(member) = current {
            if p.format {
                let depth = p.depth;
                let Some(at) = ensure(p, depth) else {
                    return false;
                };
                if !write_at(p, at, &vec![b'\t'; depth]) {
                    return false;
                }
                p.offset = p.offset.saturating_add(depth);
            }

            if !print_string_ptr(store.string(member), p) {
                return false;
            }
            update_offset(p);

            let colon: &[u8] = if p.format { b":\t" } else { b":" };
            let Some(at) = ensure(p, colon.len()) else {
                return false;
            };
            if !write_at(p, at, colon) {
                return false;
            }
            p.offset = p.offset.saturating_add(colon.len());

            if !print_value(store, member, p) {
                return false;
            }
            update_offset(p);

            let has_next = store.next(member).is_some();
            let length = usize::from(p.format) + usize::from(has_next);
            let Some(at) = ensure(p, length + 1) else {
                return false;
            };
            let mut tail = Vec::with_capacity(3);
            if has_next {
                tail.push(b',');
            }
            if p.format {
                tail.push(b'\n');
            }
            tail.push(0);
            if !write_at(p, at, &tail) {
                return false;
            }
            p.offset = p.offset.saturating_add(length);

            current = store.next(member);
        }

        let request = if p.format {
            p.depth.saturating_add(1)
        } else {
            2
        };
        let Some(at) = ensure(p, request) else {
            return false;
        };
        let mut close = Vec::with_capacity(request);
        if p.format {
            close.extend_from_slice(&vec![b'\t'; p.depth.saturating_sub(1)]);
        }
        close.extend_from_slice(b"}\0");
        if !write_at(p, at, &close) {
            return false;
        }
        p.depth = p.depth.saturating_sub(1);
        true
    }
}
