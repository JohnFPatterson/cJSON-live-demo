//! Port of the helpers in `tests/common.h` used by the parse suites, plus
//! Unity's `TEST_ASSERT_EQUAL_DOUBLE` semantics.
//!
//! Each suite compiles this module separately and uses only some helpers.
#![allow(dead_code)] // Sound: test-only helpers; unused ones in a given suite are just not called.

use std::path::PathBuf;

use cjson_core::store::Node;
use cjson_core::{Arena, NodeId, NodeStore, CJSON_IS_REFERENCE, CJSON_STRING_IS_CONST};

/// `static cJSON item[1]; memset(item, 0, sizeof(cJSON));`
pub fn new_item() -> (Arena, NodeId) {
    let mut arena = Arena::new();
    let item = arena.alloc_node().expect("allocation");
    (arena, item)
}

/// The bytes of a C string literal plus its terminator, as the suites build
/// their buffers (`buffer.length = strlen(string) + sizeof("")`).
pub fn c_string(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

/// `reset` (tests/common.h:28-44).
pub fn reset(arena: &mut Arena, item: NodeId) {
    if let Some(child) = arena.child(item) {
        cjson_core::tree::delete(arena, Some(child));
    }
    let type_bits = arena.type_bits(item);
    if type_bits & CJSON_IS_REFERENCE == 0 {
        arena.free_valuestring(item);
    }
    if type_bits & CJSON_STRING_IS_CONST == 0 {
        arena.free_string(item);
    }
    *arena.node_mut(item).expect("live item") = Node::default();
}

/// `read_file` (tests/common.h:47-100), relative to the C `tests/` dir.
pub fn read_file(relative: &str) -> Option<Vec<u8>> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "tests", relative]
        .iter()
        .collect();
    std::fs::read(path).ok()
}

/// `TEST_ASSERT_EQUAL_DOUBLE(expected, actual)`: Unity compares within
/// `expected * UNITY_DOUBLE_PRECISION` (1e-12), unity_internals.h:228/856,
/// unity.c:680-692.
pub fn assert_equal_double(expected: f64, actual: f64) {
    let same_infinity =
        expected.is_infinite() && actual.is_infinite() && ((expected < 0.0) == (actual < 0.0));
    let both_nan = expected.is_nan() && actual.is_nan();
    let within = same_infinity || both_nan || {
        let delta = (expected * 1e-12).abs();
        let diff = (actual - expected).abs();
        !(diff.is_nan() || diff.is_infinite() || diff > delta)
    };
    assert!(within, "Expected {expected:e} Was {actual:e}");
}

// ----- assertion helper macros (tests/common.h:102-120) --------------------

pub fn assert_has_type(arena: &Arena, item: NodeId, item_type: i32) {
    assert_eq!(
        arena.type_bits(item) & 0xFF,
        item_type,
        "Item doesn't have expected type."
    );
}

pub fn assert_has_no_reference(arena: &Arena, item: NodeId) {
    assert_eq!(
        arena.type_bits(item) & CJSON_IS_REFERENCE,
        0,
        "Item should not have a string as reference."
    );
}

pub fn assert_has_no_const_string(arena: &Arena, item: NodeId) {
    assert_eq!(
        arena.type_bits(item) & CJSON_STRING_IS_CONST,
        0,
        "Item should not have a const string."
    );
}

pub fn assert_has_valuestring(arena: &Arena, item: NodeId) {
    assert!(arena.valuestring(item).is_some(), "Valuestring is NULL.");
}

pub fn assert_has_no_valuestring(arena: &Arena, item: NodeId) {
    assert!(
        arena.valuestring(item).is_none(),
        "Valuestring is not NULL."
    );
}

pub fn assert_has_string(arena: &Arena, item: NodeId) {
    assert!(arena.string(item).is_some(), "String is NULL");
}

pub fn assert_has_no_string(arena: &Arena, item: NodeId) {
    assert!(arena.string(item).is_none(), "String is not NULL.");
}

pub fn assert_not_in_list(arena: &Arena, item: NodeId) {
    assert!(
        arena.next(item).is_none(),
        "Linked list next pointer is not NULL."
    );
    assert!(
        arena.prev(item).is_none(),
        "Linked list previous pointer is not NULL."
    );
}

pub fn assert_has_child(arena: &Arena, item: NodeId) {
    assert!(arena.child(item).is_some(), "Item doesn't have a child.");
}

pub fn assert_has_no_child(arena: &Arena, item: NodeId) {
    assert!(arena.child(item).is_none(), "Item has a child.");
}

pub fn assert_is_invalid(arena: &Arena, item: NodeId) {
    assert_has_type(arena, item, cjson_core::CJSON_INVALID);
    assert_not_in_list(arena, item);
    assert_has_no_child(arena, item);
    assert_has_no_string(arena, item);
    assert_has_no_valuestring(arena, item);
}
