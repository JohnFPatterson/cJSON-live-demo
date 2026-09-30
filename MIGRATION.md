# Migration: cJSON 1.7.19 from C to Rust

Result: all 78 functions in `cJSON.h` exported from Rust; 406 of 406 parity inputs identical; 178 Rust tests pass (1 ignored, as in C); 0 parity exceptions. 5 intentional changes (CH-1 to CH-5), approved by the project owner, all outside parse/print output.

Reproduce everything with `make parity` (or `./parity.sh`). The per-input evidence is in `PARITY.md`.

## Layout

```
cJSON.c, cJSON.h            original C, unmodified; the spec and the oracle
cJSON_Utils.c/.h, tests/    original, unmodified (tests/parity/ is new fixtures only)
cjson-core/                 all logic, #![forbid(unsafe_code)]
  src/store.rs              NodeStore trait (the cJSON node: next/prev/child, type, valuestring,
                            valueint, valuedouble, string) and Arena, the safe in-memory store
  src/parse.rs              parse_value/number/string/array/object, error offsets
  src/print.rs              print_value/number/string_ptr/array/object, ensure() accounting
  src/tree.rs               create/add/detach/insert/replace/delete, accessors, setters
  src/compare.rs, duplicate.rs, minify.rs, number.rs, consts.rs
  tests/*.rs                case-for-case ports of the 18 Unity suites + pinning tests
cjson-ffi/                  C ABI shim; the only crate with unsafe
  src/types.rs              #[repr(C)] cJSON and cJSON_Hooks, layout asserted against cJSON.h
  src/store.rs              CStore: NodeStore over the caller's live C tree
  src/globals.rs            hooks and the error pointer (process-wide, as in C)
  src/exports.rs            the 78 extern "C" functions
tools/
  cjson-oracle.c            C driver over the public API (links cJSON.c or libcjson_ffi.a)
  rust-driver/              the same driver over cjson-core
  DRIVER_FORMAT.md          report format both drivers print
  gen-parity-fixtures.py    generator for tests/parity/ (--check verifies)
  ffi-unity/run.sh          runs the public-API Unity suites against libcjson_ffi.a
  hook-trace.c              allocator-hook call trace, C vs Rust (make hook-trace)
parity.sh, Makefile         one-command parity run (targets appended below the original ones)
.cursor/parity.json         parity gate config
```

Every algorithm in `cjson-core` is generic over `NodeStore`. Rust callers use `Arena`. The FFI crate implements `NodeStore` for the C tree itself, so C callers get exactly the `cJSON` struct layout the header documents. That includes linked lists, `child->prev` pointing at the last element, and interior string pointers. C programs keep including `cJSON.h` and link `target/release/libcjson_ffi.a` (or the cdylib) instead of `cJSON.c`.

## `unsafe` audit

```
$ grep -rn "unsafe" --include=*.rs --exclude-dir=target . | grep -v "^./cjson-ffi/"
./tools/rust-driver/src/main.rs:7:#![forbid(unsafe_code)]
./cjson-core/src/lib.rs:15:#![forbid(unsafe_code)]
$ grep -rn "unsafe" --include=*.rs cjson-ffi | cut -d: -f1 | sort | uniq -c
 145 cjson-ffi/src/exports.rs
   3 cjson-ffi/src/globals.rs
   4 cjson-ffi/src/lib.rs
  13 cjson-ffi/src/store.rs
   3 cjson-ffi/src/types.rs
  20 cjson-ffi/tests/abi.rs
  11 cjson-ffi/tests/hook_behavior.rs
unsafe blocks: 110, with SAFETY: comment within 8 lines above: 110
unsafe fn declarations: 81
```

- `cjson-ffi` has `#![deny(unsafe_op_in_unsafe_fn)]` and `#![deny(clippy::undocumented_unsafe_blocks)]` (cjson-ffi/src/lib.rs:34-35), so clippy fails on any block without a `SAFETY:` comment.
- Each exported `unsafe extern "C" fn` has a `# Safety` section giving the pointer contract. The crate-level contract is in cjson-ffi/src/lib.rs.
- The three `unsafe` mentions in `types.rs` are function-pointer type aliases for the hooks, not unsafe code.
- The only lint suppressions are `#[allow(non_camel_case_types)]` on `cJSON` and `cJSON_Hooks` (the names are fixed by cJSON.h), `#[allow(non_snake_case)]` on two tests that keep their C names, and `#![allow(dead_code)]` on a shared test helper module. Each has an adjacent comment giving the reason.

## Security scan (SonarQube MCP)

Project `JohnFPatterson_cJSON-live-demo`, open/confirmed issues with SECURITY impact, plus Security Hotspots in `cJSON.c` and `cJSON_Utils.c` with status TO_REVIEW. The query returned 15 issues (`"paging": {"total": 15}`) and 0 hotspots (`"hotspots": [], "total": 0`). SonarQube analyzed the file before the comments were added in 570bfcb, so its line numbers are shifted from the current `cJSON.c`.

| Sonar key | Rule | File:line (Sonar) | Current line | Code | Classification |
|---|---|---|---|---|---|
| AaDvZwqWwIxFzkbHQFA9 | c:S6069 | cJSON.c:127 | cJSON.c:131 | `sprintf(version, "%i.%i.%i", ...)` | Keep: `version[15]` (cJSON.c:130), constant output "1.7.19" |
| AaDvZwqWwIxFzkbHQFA- | c:S6069 | cJSON.c:614 | cJSON.c:620 | `sprintf(number_buffer, "null")` | Keep: `number_buffer[26]` (cJSON.c:608) |
| AaDvZwqWwIxFzkbHQFA_ | c:S6069 | cJSON.c:618 | cJSON.c:624 | `sprintf(number_buffer, "%d", valueint)` | Keep: at most 11 chars |
| AaDvZwqWwIxFzkbHQFBA | c:S6069 | cJSON.c:623 | cJSON.c:629 | `sprintf(number_buffer, "%1.15g", d)` | Keep: at most 22 chars; length checked at cJSON.c:639-642 |
| AaDvZwqWwIxFzkbHQFBB | c:S6069 | cJSON.c:629 | cJSON.c:635 | `sprintf(number_buffer, "%1.17g", d)` | Keep: at most 24 chars + NUL fits 26 |
| AaDvZwqWwIxFzkbHQFBC | c:S6069 | cJSON.c:1063 | cJSON.c:1069 | `sprintf(output_pointer, "u%04x", c)` | Keep: space reserved by `ensure` (cJSON.c:1013) |
| AaDvZwtQwIxFzkbHQFBE..BI | c:S6069 | cJSON_Utils.c:234, 1122, 1188, 1203, 1248 | — | `sprintf` in Utils | Out of scope (cJSON_Utils not ported) |
| AaDvZwpBwIxFzkbHQFA6, A7 | c:S6069 | tests/parse_examples.c:68, 69 | — | test code | Not a library module |
| AaDvZwuUwIxFzkbHQFBM, BN | githubactions:S7637 | .github/workflows/ci-fuzz.yml:8, 13 | — | CI config | Not a library module |

None of the six `cJSON.c` findings can overflow: each writes into a buffer whose bound covers the longest possible output. The Rust port formats into growable buffers, so there is no fixed-size buffer at all. Fixing them would not change parse, print, or error position, so per the port rules there are no exception rows (`PARITY_EXCEPTIONS.md`: None).

The `cJSON_strdup` overflow (cJSON.c:209) is not among the Sonar findings. See "Oracle defect found" in `PARITY.md`.

## Behavior kept on purpose

These are all C quirks, including odd ones, reproduced byte for byte. The full table with C source lines and evidence is in `PARITY.md` under "Quirks kept". Highlights:

- **Numbers:** `%d`, `%1.15g`, `%1.17g` print selection; NaN/Inf print as `null`; `valueint` saturation; locale-independent decimal point.
- **Objects and lookup:** duplicate keys are kept; `GetObjectItem` is case-insensitive via `tolower`.
- **Parsing:** BOM only at offset 0; nesting limit 1000; `require_null_terminated` error position; `cJSON_GetErrorPtr` semantics.
- **Printing:** `PrintPreallocated` 5-byte slack; `\u%04x` control escapes with `/` unescaped; `NULL` valuestring prints as `""`.
- **Tree and other APIs:**
  - Duplicate stops at depth 10000.
  - `SetValuestring` copies in place when the new string is not longer, and rejects overlapping input.
  - Minify comment handling.
  - Compare uses relative epsilon and has unbounded recursion.
- **Pointer-level contracts C callers rely on:**
  - `cJSON_SetValuestring` returns the same pointer when it copies in place.
  - `cJSON_AddItemToObjectCS` stores the caller's key pointer and sets `cJSON_StringIsConst`.
  - `cJSON_CreateStringReference` points at the caller's string.
  - `cJSON_GetErrorPtr` returns `json + position`.
  - `cJSON_Version` returns a pointer to a static buffer.
  - The printed buffer comes from the malloc hook and is freed with `cJSON_free`.
  - Covered by `cjson-ffi/tests/abi.rs` and ffi-unity.

## Behavior changed on purpose (approved)

None of these changes parse results, print output, error positions, or return values; all 406 fixtures and all ported suites are identical. They are visible only to a program that installs logging or failing `cJSON_Hooks`, or that triggers undefined behavior in C. Each is pinned by a test. `make hook-trace` diffs the full hook call sequence of `tools/hook-trace.c` under cJSON.c against the Rust library. It exits nonzero while CH-1 to CH-3 remain, and it is not part of `make parity`.

| ID | Change | C behavior | Rust behavior | Why | Pinning test |
|---|---|---|---|---|---|
| CH-1 | Allocation sizes | `parse_string` allocates 1 byte more than needed (the length includes the opening quote); number scratch buffer is `len + 1` bytes | Strings allocate `strlen + 1`; the scratch allocation is replayed as one node-sized (64-byte) block so the call count, and therefore "fail on Nth malloc" behavior, matches C | Core stores strings as exact byte slices; the store API does not carry C's over-allocation | `hook_behavior.rs::parse_allocations` |
| CH-2 | Print allocation pattern | `M 256, M 32, F 256` (default 256-byte buffer, then an exact copy without a realloc hook); `PrintBuffered` grows from `prebuffer` | One exact allocation (`M 32`). A malloc hook that fails on its 2nd call makes C's print return NULL; Rust succeeds | Core prints into a `Vec`, and FFI copies the result into one hook allocation | `hook_behavior.rs::print_allocates_once` |
| CH-3 | Free timing and ordering | Frees happen at the point in the algorithm where C calls `deallocate`; `replace_item_in_object` frees the old key before copying the new one (cJSON.c:2477-2481). That is a use-after-free when the new name aliases the old key | Frees requested by the core are queued and released when the exported call returns (cjson-ffi/src/store.rs:146-175); the new key is copied before the old one is released | Deferral keeps every byte slice the core borrows from the C tree valid for the whole call without `unsafe` in core; it also removes the aliasing use-after-free | `hook_behavior.rs::replace_item_in_object_copies_key_before_releasing`, `::replace_item_in_object_with_aliased_key` |
| CH-4 | Detach from a childless parent | Detaching a non-first last item whose parent has no child dereferences `parent->child` (NULL) at cJSON.c:2317 | The write is skipped; the item is returned unlinked (cjson-core/src/tree.rs:660-692) | Crash in C; core must not panic | `intentional_differences.rs::detach_non_first_item_from_childless_parent_is_a_no_op_on_the_parent` |
| CH-5 | Deep trees | `cJSON_Delete` recurses once per nesting level; a tree built through the tree API (parse stops at 1000) can exhaust the stack | `delete` is iterative, and so is `duplicate` internally (C's depth-10000 limit is kept) | Stack exhaustion is a crash; core must not panic | `intentional_differences.rs::delete_of_very_deep_tree_does_not_recurse` |

Hook trace, section by section (`make hook-trace`; C first, Rust second; `M n` = malloc of n bytes, `F n` = free of an n-byte block):

```
parse: DIFFERENT
   C  : M 64 | M 64 | M 5 | M 64 | M 3 | M 64 | M 2 | F 2 | M 64 | M 64 | M 3 | M 6
   FFI: M 64 | M 64 | M 4 | M 64 | M 2 | M 64 | M 64 | M 64 | M 64 | M 2 | M 5 | F 64
print_unformatted: DIFFERENT
   C  : M 256 | M 32 | F 256 | -> {"key":["v",1,null],"s":"text"} | F 32
   FFI: M 32 | -> {"key":["v",1,null],"s":"text"} | F 32
print_buffered_4: DIFFERENT
   C  : M 4 | M 16 | F 4 | M 40 | F 16 | -> {"key":["v",1,null],"s":"text"} | F 40
   FFI: M 32 | -> {"key":["v",1,null],"s":"text"} | F 32
duplicate: identical
set_valuestring_longer: identical
add_item_to_object: identical
replace_item_in_object: DIFFERENT
   C  : M 64 | M 4 | M 4 | -> 1 | -> 1 | F 4 | M 4 | F 3 | F 64 | F 64 | F 64 | F 5 | F 64 | -> 1
   FFI: M 64 | M 4 | M 4 | -> 1 | -> 1 | M 4 | F 4 | F 2 | F 64 | F 64 | F 64 | F 4 | F 64 | -> 1
delete: DIFFERENT (same order; sizes differ by CH-1 only)
parse_fail_after_0 .. parse_fail_after_11: same result (NULL/ok) at every N; sizes differ by CH-1
print_fail_after_1: DIFFERENT
   C  : M 256 | M 32 FAIL | F 256 | -> (null)
   FFI: M 32 | -> {"key":["v",1,null],"s":"text"} | F 32
```

If exact hook parity is required, CH-1 to CH-3 can be closed by adding a capacity hint to the store API and replaying C's print buffer growth in the FFI. CH-4 and CH-5 replace crashes and can't be matched without reintroducing the crash.

## Not covered

- **`cJSON_Utils` (cJSON_Utils.c/.h), including JSON Pointer, Patch, and Merge Patch:** not ported. C callers that need it must keep linking cJSON_Utils.c, which calls only the public cJSON API. A smoke test (`build/asan/utils.c`: `cJSONUtils_GetPointer` plus one `cJSONUtils_ApplyPatches`) linked against cJSON.c and against `libcjson_ffi.a` printed identical output. The Utils Unity suites (json_patch_tests, old_utils_tests, misc_utils_tests) were not run against the Rust library. Its 5 Sonar findings are unaddressed. `tests/json-patch-tests/*.json` are used only as parse/print fixtures.
- **`cJSON_Duplicate_rec`:** a non-static function in cJSON.c:2840 that is not declared in cJSON.h. The Rust library does not export it, so a C program that declares and calls it itself would fail to link.
- **Windows builds:** `CJSON_STDCALL` / `__stdcall` exports (cJSON.h:55-68) and `__declspec` import/export. The Rust exports are `extern "C"`, which is the same ABI on every non-Windows target.
- **`ENABLE_LOCALES`:** not modeled. C output is locale-independent anyway, because the decimal point is mapped both ways, and the Rust port always behaves as the C locale.
- **Thread safety of the globals:** unchanged from C. Hooks and the error pointer are process-wide; Rust guards them with a mutex and atomics, but concurrent parse calls still race on which error pointer a caller sees, as in C.
- **Unity suites that test static C functions** (parse_number, parse_hex4, parse_string, parse_array, parse_object, parse_value, print_string, print_number, print_array, print_object, print_value, and three `misc_tests` cases): they run only against C in `c-unity`, because a library cannot provide those symbols. Their Rust ports run against `cjson-core`.
- **Parity scope:** only the 406 fixtures are proven; there is no fuzzing campaign. stderr is not compared.
- **Compare timeouts:** the drivers skip `cJSON_Compare` when its cost would exceed 1,000,000 steps; C's compare is exponential on deep objects.
- **Print allocation failure:** cannot be injected into `cjson-core` (it prints into a `Vec`). It is exercised through the C ABI only.
- **`cJSON_ParseWithLength` with a length larger than the buffer:** undefined behavior in C and not exercised.
