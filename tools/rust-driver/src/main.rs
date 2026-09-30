//! Parity driver: prints the report specified in `tools/DRIVER_FORMAT.md` for
//! one fixture, using `cjson-core` over an [`Arena`]. Its stdout must match
//! `tools/cjson-oracle.c` (built against the original `cJSON.c`) byte for byte.
//!
//! Usage: `rust-driver <fixture-path>`. Exit status 0 after a full report,
//! 2 on usage, read, or write errors.
#![forbid(unsafe_code)]

use std::io::Write;
use std::process::ExitCode;

use cjson_core::compare::compare;
use cjson_core::duplicate::duplicate;
use cjson_core::minify::minify;
use cjson_core::parse::{parse_with_length_opts, parse_with_opts};
use cjson_core::print::{print, print_buffered, print_preallocated};
use cjson_core::tree::{delete, get_array_size};
use cjson_core::{Arena, NodeId, NodeStore};

/// Same bound as `COMPARE_COST_LIMIT` in tools/cjson-oracle.c.
const COMPARE_COST_LIMIT: u64 = 1_000_000;

/// Upper bound on the `cJSON_Compare` calls needed to compare `item` with an
/// equal tree (doubles per object level; see tools/DRIVER_FORMAT.md).
/// Saturates at `COMPARE_COST_LIMIT + 1`, exactly like the C driver.
fn compare_cost(arena: &Arena, item: NodeId) -> u64 {
    let mut total: u64 = 0;
    let mut child = arena.child(item);
    while let Some(c) = child {
        total += compare_cost(arena, c);
        if total > COMPARE_COST_LIMIT {
            return COMPARE_COST_LIMIT + 1;
        }
        child = arena.next(c);
    }
    if arena.type_bits(item) & 0xFF == cjson_core::CJSON_OBJECT {
        total *= 2;
    }
    total += 1;
    total.min(COMPARE_COST_LIMIT + 1)
}

/// The bytes a C string API sees: everything before the first NUL.
fn c_str(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(end) => bytes.get(..end).unwrap_or(bytes),
        None => bytes,
    }
}

#[derive(Default)]
struct Report {
    out: Vec<u8>,
}

impl Report {
    fn put(&mut self, text: &str) {
        self.out.extend_from_slice(text.as_bytes());
    }

    fn line(&mut self, text: &str) {
        self.put(text);
        self.out.push(b'\n');
    }

    /// `<tag> <L>\n<bytes>\n` or `<tag> null\n`.
    fn blob(&mut self, tag: &str, bytes: Option<&[u8]>) {
        match bytes {
            Some(bytes) => {
                self.line(&format!("{tag} {}", bytes.len()));
                self.out.extend_from_slice(bytes);
                self.out.push(b'\n');
            }
            None => self.line(&format!("{tag} null")),
        }
    }

    /// `null` if `candidate` is None, `same` if it equals a non-null
    /// `reference`, else `diff`.
    fn same(&mut self, tag: &str, candidate: Option<&[u8]>, reference: Option<&[u8]>) {
        let verdict = match (candidate, reference) {
            (None, _) => "null",
            (Some(c), Some(r)) if c == r => "same",
            _ => "diff",
        };
        self.line(&format!("{tag} {verdict}"));
    }

    /// One ` <len>:<0|1>[!]` item of a `prealloc_*` line.
    fn prealloc_item(
        &mut self,
        arena: &Arena,
        root: NodeId,
        len: usize,
        format: bool,
        reference: &[u8],
    ) {
        let outcome = print_preallocated(arena, Some(root), len, format);
        // Mirrors the C driver's buffer: len + 15 bytes of 0x5A, then a NUL guard.
        let mut buf2 = vec![0x5A_u8; len.saturating_add(16)];
        if let Some(guard) = buf2.last_mut() {
            *guard = 0;
        }
        for (dst, src) in buf2.iter_mut().zip(outcome.written.iter().take(len)) {
            *dst = *src;
        }
        if outcome.ok {
            let bang = if c_str(&buf2) == reference { "" } else { "!" };
            self.put(&format!(" {len}:1{bang}"));
        } else {
            self.put(&format!(" {len}:0"));
        }
    }

    fn prealloc(&mut self, tag: &str, arena: &Arena, root: NodeId, reference: &[u8], format: bool) {
        let len = reference.len();
        self.put(tag);
        if let Some(shorter) = len.checked_sub(1) {
            self.prealloc_item(arena, root, shorter, format, reference);
        }
        for candidate in [len, len.saturating_add(1), len.saturating_add(5)] {
            self.prealloc_item(arena, root, candidate, format, reference);
        }
        self.out.push(b'\n');
    }
}

/// Section A: `cJSON_ParseWithOpts(buf, &end, 0)` and everything done with its root.
fn report_parse_with_opts(report: &mut Report, arena: &mut Arena, cstr: &[u8]) {
    let parsed = match parse_with_opts(arena, cstr, false) {
        Ok(parsed) => parsed,
        Err(failure) => {
            // The core reports a single position, so `opts0 endmismatch`
            // (end != cJSON_GetErrorPtr()) cannot occur here.
            report.line(&format!("opts0 fail pos {}", failure.position));
            return;
        }
    };
    let root = parsed.root;
    report.line(&format!("opts0 ok end {}", parsed.end));

    let compact = print(arena, Some(root), false);
    report.blob("compact", compact.as_deref());
    let pretty = print(arena, Some(root), true);
    report.blob("pretty", pretty.as_deref());

    let buffered = print_buffered(arena, Some(root), 0, true);
    report.same("buffered0", buffered.as_deref(), pretty.as_deref());
    let buffered = print_buffered(arena, Some(root), 1, false);
    report.same("buffered1", buffered.as_deref(), compact.as_deref());

    if let Some(compact) = compact.as_deref() {
        report.prealloc("prealloc_compact", arena, root, compact, false);
    }
    if let Some(pretty) = pretty.as_deref() {
        report.prealloc("prealloc_pretty", arena, root, pretty, true);
    }

    let dup = duplicate(arena, Some(root), true);
    match dup {
        None => {
            report.line("dup null");
            report.line("cmp null");
        }
        Some(d) => {
            let dup_compact = print(arena, Some(d), false);
            let same = match (dup_compact.as_deref(), compact.as_deref()) {
                (None, None) => true,
                (Some(a), Some(b)) => a == b,
                _ => false,
            };
            report.line(if same { "dup same" } else { "dup diff" });
            if compare_cost(arena, root) > COMPARE_COST_LIMIT {
                report.line("cmp skipped");
            } else {
                let sensitive = u8::from(compare(arena, Some(root), Some(d), true));
                let insensitive = u8::from(compare(arena, Some(root), Some(d), false));
                report.line(&format!("cmp {sensitive} {insensitive}"));
            }
        }
    }

    report.line(&format!("size {}", get_array_size(arena, Some(root))));
    report.line(&format!("type {}", arena.type_bits(root)));

    delete(arena, dup);
    delete(arena, Some(root));
}

/// Section B: `cJSON_ParseWithOpts(buf, &end, 1)`.
fn report_parse_strict(report: &mut Report, arena: &mut Arena, cstr: &[u8]) {
    match parse_with_opts(arena, cstr, true) {
        Ok(parsed) => {
            report.line(&format!("opts1 ok end {}", parsed.end));
            delete(arena, Some(parsed.root));
        }
        Err(failure) => report.line(&format!("opts1 fail pos {}", failure.position)),
    }
}

/// Section C: `cJSON_ParseWithLengthOpts(buf, n, &end, 0)`.
fn report_parse_with_length(report: &mut Report, arena: &mut Arena, all: &[u8]) {
    match parse_with_length_opts(arena, all, false) {
        Ok(parsed) => {
            report.line(&format!("len ok end {}", parsed.end));
            let compact = print(arena, Some(parsed.root), false);
            report.blob("len_compact", compact.as_deref());
            delete(arena, Some(parsed.root));
        }
        Err(failure) => report.line(&format!("len fail pos {}", failure.position)),
    }
}

fn main() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let path = match args.as_slice() {
        [_, path] => path,
        _ => {
            eprintln!("usage: rust-driver <fixture-path>");
            return ExitCode::from(2);
        }
    };
    let all = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("rust-driver: cannot read {}: {err}", path.to_string_lossy());
            return ExitCode::from(2);
        }
    };
    let cstr = c_str(&all);

    let mut report = Report::default();
    let mut arena = Arena::new();
    report.line(&format!("input_len {}", all.len()));
    report_parse_with_opts(&mut report, &mut arena, cstr);
    report_parse_strict(&mut report, &mut arena, cstr);
    report_parse_with_length(&mut report, &mut arena, &all);
    report.blob("minify", Some(&minify(cstr)));

    let mut stdout = std::io::stdout().lock();
    if let Err(err) = stdout.write_all(&report.out).and_then(|()| stdout.flush()) {
        eprintln!("rust-driver: write to stdout failed: {err}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}
