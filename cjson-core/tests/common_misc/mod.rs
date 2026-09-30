//! Shared helper for the compare/readme ported suites.

use cjson_core::{Arena, NodeId};

/// `cJSON_Parse(text)` for a C string literal (bytes without the NUL);
/// None where C returns NULL.
pub fn parse(store: &mut Arena, text: &str) -> Option<NodeId> {
    cjson_core::parse::parse(store, text.as_bytes())
        .ok()
        .map(|ok| ok.root)
}
