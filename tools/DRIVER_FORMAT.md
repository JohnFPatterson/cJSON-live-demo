# Parity driver report format

Both parity drivers print this report for one fixture:

| Driver | Source | Links against |
|---|---|---|
| `build/oracle` | `tools/cjson-oracle.c` | the unmodified `cJSON.c` (the spec) |
| `build/oracle-ffi` | `tools/cjson-oracle.c` (same source) | `target/release/libcjson_ffi.a` (the Rust C ABI shim) |
| `target/release/rust-driver` | `tools/rust-driver/src/main.rs` | `cjson-core` with `Arena` |

For every fixture, all three must write **identical stdout bytes** and exit
with the same status. The parity gate (`.cursor/parity.json`) compares
`build/oracle` with `rust-driver`. `parity.sh` also compares `build/oracle`
with `build/oracle-ffi`.

## Conventions

- Usage: `<driver> <fixture-path>`. The fixture path is the only argument.
- Every line ends with a single `\n`. There is no trailing whitespace and no `\r`.
- Numbers are unsigned decimal with no leading zeros, `+`, or padding. The only
  exceptions are `size`, which C prints with `%d` (`cJSON_GetArraySize` returns
  `int` and is never negative for a parsed tree), and `type`, which prints
  `root->type` with `%d`.
- Offsets (`end`, `pos`) are byte offsets from the start of the fixture bytes,
  i.e. `pointer - buf`.
- A "blob" line is `<tag> <L>\n`, then exactly `L` raw bytes, then `\n`. The
  bytes are written with `fwrite` in C and `write_all` in Rust, never through a
  format string. They may contain any byte except NUL. None of the cJSON
  outputs used here can contain NUL: printed strings stop at the first NUL,
  control characters are escaped, and minify stops at the first NUL.
- Output is written only to stdout. Diagnostics go to stderr, which the gate
  does not compare.

## Allocator hooks (C drivers only)

Before anything else, `tools/cjson-oracle.c` calls `cJSON_InitHooks` with a
`malloc_fn` that allocates `size + 1` bytes and a `free_fn` that calls `free`.
The reason is `cJSON_strdup` in this tree (cJSON.c:203-209, commit 427291d
"Quick edit for memcpy length"). It allocates `strlen + 1` bytes and then
`memcpy`s `strlen + 2`, a one-byte heap overflow on every string that
`cJSON_Duplicate` copies. Under default hooks the oracle crashed
nondeterministically on `tests/inputs/test4`, `tests/inputs/test4.expected` and
`tests/json-patch-tests/spec_tests.json` (SIGSEGV, once SIGKILL). With the spare
byte the overrun stays inside the allocation, and the byte it writes is never
part of any output. The report therefore shows what `cJSON.c` computes, every
run. With non-default hooks, cJSON grows buffers with allocate + copy + free
instead of `realloc`, which produces the same bytes. `build/oracle-ffi` runs the
same source, so the Rust shim is exercised through `cJSON_InitHooks` too.
`rust-driver` has no hooks. The Rust core copies strings correctly, so it does
not reproduce the overflow.

## Input

1. Read the fixture file as `n` raw bytes. If it cannot be opened or read, or
   the argument count is wrong, write a message to stderr and exit with
   status **2** without writing anything to stdout.
2. `buf` = those `n` bytes followed by one NUL byte (`n + 1` bytes in total).
   The file may contain NUL bytes of its own. C string APIs stop at the first
   one; the length-based parse (section C) sees all `n` bytes.

In Rust terms: `cstr` = the bytes of the file before its first NUL (all `n`
bytes if it has none), and `all` = all `n` bytes.

## Report

```
input_len <n>
```

### A. `cJSON_ParseWithOpts(buf, &end, 0)`

`end` is initialised to `NULL` before the call. Rust:
`parse::parse_with_opts(&mut arena, cstr, false)`.

On success:

```
opts0 ok end <end - buf>
```

On failure:

```
opts0 fail pos <cJSON_GetErrorPtr() - buf>
opts0 endmismatch          (only if end != cJSON_GetErrorPtr())
```

`cJSON.c` always sets `*return_parse_end` to the same pointer it stores as the
error position, so C never emits `endmismatch`. The Rust core returns a single
`ParseFailure::position`, so `rust-driver` can never emit it. The line exists
to catch an FFI shim that reports the two pointers differently.

If A succeeded with root `r`, these lines follow, in this order:

```
compact <L>\n<L bytes of cJSON_PrintUnformatted(r)>\n      or   compact null
pretty <P>\n<P bytes of cJSON_Print(r)>\n                  or   pretty null
buffered0 same|diff|null
buffered1 same|diff|null
prealloc_compact <len>:<0|1>[!] ...                          (omitted if compact is null)
prealloc_pretty <len>:<0|1>[!] ...                           (omitted if pretty is null)
dup same|diff|null
cmp <0|1> <0|1>                                   or   cmp null   or   cmp skipped
size <cJSON_GetArraySize(r)>
type <r->type>
```

- `buffered0`: `b = cJSON_PrintBuffered(r, 0, 1)`. `null` if `b == NULL`;
  otherwise `same` if pretty is non-null and `strcmp(b, pretty) == 0`, else
  `diff`. Rust: `print::print_buffered(&arena, Some(r), 0, true)`.
- `buffered1`: `b = cJSON_PrintBuffered(r, 1, 0)` compared with compact the
  same way. Rust: `print_buffered(&arena, Some(r), 1, false)`.
- `prealloc_compact`: the tag, then one ` <len>:<ok>` item for each `len` in
  this order: `L-1` (only when `L > 0`), `L`, `L+1`, `L+5`. For each item, `buf2`
  is a fresh buffer of `len + 16` bytes. Its first `len + 15` bytes are set to
  `0x5A` and its last byte is NUL (a guard, outside the `len` bytes cJSON may
  touch). Then `ok = cJSON_PrintPreallocated(r, buf2, (int)len, 0)`, printed as
  `0` or `1`. When `ok` is 1 and `strcmp(buf2, compact) != 0`, `!` follows
  immediately after the `1`. Rust: `print::print_preallocated(&arena, Some(r),
  len, false)`. `written` is copied over the start of `buf2`, and the C string
  of `buf2` (the bytes before its first NUL) is compared with compact.
- `prealloc_pretty`: the same, with `format = 1`, `P` and pretty.
- `d = cJSON_Duplicate(r, 1)`. `dup null` if `d == NULL`. Otherwise
  `p = cJSON_PrintUnformatted(d)`; `dup same` if both `p` and compact are NULL
  or both are non-null and equal, else `dup diff`.
- `cmp <cJSON_Compare(r, d, 1)> <cJSON_Compare(r, d, 0)>`, each `0` or `1`. If
  `d == NULL` this is `cmp null`. If `compare_cost(r) > 1000000` this is
  `cmp skipped` and `cJSON_Compare` is not called (see below).
- `compare_cost(node) = 1 + k * sum(compare_cost(child))` over the
  `child`/`next` list, where `k = 2` when `(node->type & 0xFF) == cJSON_Object`
  and `k = 1` otherwise. The value saturates at `1000001`: both drivers return
  `1000001` as soon as a running sum exceeds `1000000`. This is an upper bound on
  the number of `cJSON_Compare` calls needed to compare `r` with an equal tree.
  The bound doubles for each object level because the object branch
  (cJSON.c:3229-3263) walks `a`'s members and then `b`'s members, and recurses
  in both walks. A chain of `d` nested objects costs `2^d - 1`, so the C oracle
  cannot finish `cmp` on the valid 999- and 1000-deep object fixtures. A chain
  of 19 objects (cost 524287) is compared, and a chain of 20 (cost 1048575) is
  skipped. Parse, print, prealloc and duplicate still run on skipped inputs.
- `size`: `cJSON_GetArraySize(r)` with `%d`. For any container this counts the
  children (objects too); for scalars it is 0.
- `type`: `r->type` with `%d` (includes flag bits such as `cJSON_IsReference`
  if they were ever set; a freshly parsed root has only the type bit).
- Then `cJSON_Delete(d)` and `cJSON_Delete(r)`.

### B. `cJSON_ParseWithOpts(buf, &end, 1)`

Rust: `parse_with_opts(&mut arena, cstr, true)`.

```
opts1 ok end <end - buf>        or   opts1 fail pos <cJSON_GetErrorPtr() - buf>
```

The root, if any, is deleted.

### C. `cJSON_ParseWithLengthOpts(buf, n, &end, 0)`

The length is `n`, so the NUL that the driver appends is **not** included.
Rust: `parse_with_length_opts(&mut arena, all, false)`.

On success:

```
len ok end <end - buf>
len_compact <L>\n<L bytes of cJSON_PrintUnformatted(root)>\n     or   len_compact null
```

On failure:

```
len fail pos <cJSON_GetErrorPtr() - buf>
```

The root, if any, is deleted.

### D. `cJSON_Minify`

Copy all `n + 1` bytes of `buf` (including its NUL) into a fresh buffer and run
`cJSON_Minify` on it. Rust: `minify::minify(cstr)`.

```
minify <strlen(result)>\n<result bytes>\n
```

### Exit

Exit status **0** after the report, whatever the parse results were. If
writing stdout fails, exit status 2.

## Example

A fixture containing the 8 bytes `[1, "a"]`:

```
input_len 8
opts0 ok end 8
compact 7
[1,"a"]
pretty 8
[1, "a"]
buffered0 same
buffered1 same
prealloc_compact 6:0 7:0 8:0 12:1
prealloc_pretty 7:0 8:0 9:0 13:1
dup same
cmp 1 1
size 2
type 32
opts1 ok end 8
len ok end 8
len_compact 7
[1,"a"]
minify 7
[1,"a"]
```

This is real `build/oracle` output. `cJSON_PrintPreallocated` fails at `L` and
at `L+1` here: it needs room for the terminating NUL, and its internal
`ensure()` calls over-reserve (cJSON.h tells callers to "allocate 5 bytes more
than you actually need"). This is observable C behavior, and every driver must
report it the same way.

Exit status 2 is also used if the driver itself runs out of memory. In that
case the report stops early.
