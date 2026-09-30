//! Helpers shared by the ported tree suites (`misc_tests`, `cjson_add`).
//! Every item here must be used by every suite that includes this module.

use cjson_core::Arena;

/// A C string literal argument (`const char *`, never NULL).
pub fn cstr(s: &str) -> Option<&[u8]> {
    Some(s.as_bytes())
}

/// `cJSON_InitHooks(&failing_hooks)` when `failing` (every allocation
/// fails, like `failing_malloc`), `cJSON_InitHooks(NULL)` otherwise.
pub fn init_hooks(arena: &mut Arena, failing: bool) {
    arena.set_alloc_budget(if failing { Some(0) } else { None });
}
