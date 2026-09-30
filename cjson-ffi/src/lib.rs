//! C ABI shim for cJSON 1.7.19 (`cJSON.h`).
//!
//! This is the only crate allowed to contain `unsafe`;
//! every `unsafe` block carries a `SAFETY:` comment. All behavior lives in
//! `cjson-core`; this crate only translates pointers, allocates through the
//! `cJSON_Hooks`, and keeps the process-wide state C exposes.
//!
//! C programs keep including the original `cJSON.h` and link
//! `libcjson_ffi.a` (or the cdylib) instead of `cJSON.c`.
//!
//! # Pointer contract
//!
//! Every exported function relies on the contract `cJSON.h` already places
//! on C callers; violating it is undefined behavior here exactly as in C:
//!
//! * **Nodes.** A `cJSON *` argument is NULL or a live node created by this
//!   library (or laid out as `cJSON` and allocated with the current hooks).
//!   Its `next`/`prev`/`child` links are NULL or live nodes, and its
//!   `valuestring`/`string` are NULL or NUL-terminated strings (owned by the
//!   tree unless flagged `cJSON_IsReference` / `cJSON_StringIsConst`).
//! * **Strings.** A `const char *` argument is NULL or NUL-terminated, except
//!   the `value` of `cJSON_ParseWithLength[Opts]`, which must have
//!   `buffer_length` readable bytes.
//! * **Borrowed strings.** Strings passed to `cJSON_AddItemToObjectCS` and
//!   `cJSON_CreateStringReference` outlive the nodes that store them.
//! * **Arrays.** `cJSON_Create*Array(p, count)` reads `count` elements at `p`.
//! * **No concurrent access.** Nobody frees or mutates a tree (or an input
//!   buffer) while a call that received it is running. The hooks and
//!   `cJSON_GetErrorPtr` state are process-wide, as in C.
//!
//! Returned slices of C memory (`valuestring`, `string`, input buffers)
//! borrow memory the caller owns; the [`store`] module explains why they stay
//! valid for as long as the core uses them.
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

pub mod exports;
mod globals;
mod store;
pub mod types;

pub use globals::reallocate_hook_installed;
pub use types::{cJSON, cJSON_Hooks};
