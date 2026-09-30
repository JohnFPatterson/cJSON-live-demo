//! Pins the intentional behavior differences listed in MIGRATION.md
//! ("Behavior changed on purpose") that are reachable through `cjson-core`. Each case
//! is one where the C implementation has undefined behavior (NULL
//! dereference, stack exhaustion), so there is no C output to match.
//! Hook-visible differences are pinned in `cjson-ffi/tests/hook_behavior.rs`.

use cjson_core::tree::{
    add_item_to_array, create_array, create_number, delete, detach_item_via_pointer,
};
use cjson_core::{Arena, NodeStore};

/// MIGRATION.md CH-4. cJSON.c:2314-2318 writes `parent->child->prev` when a
/// non-first last item is detached; with a childless parent that is a NULL
/// dereference. Here the write is skipped and the item is returned unlinked.
#[test]
fn detach_non_first_item_from_childless_parent_is_a_no_op_on_the_parent() {
    let mut arena = Arena::new();
    let parent = create_array(&mut arena).expect("alloc");
    let stray_prev = create_number(&mut arena, 1.0).expect("alloc");
    let item = create_number(&mut arena, 2.0).expect("alloc");
    arena.set_prev(item, Some(stray_prev));

    assert_eq!(
        detach_item_via_pointer(&mut arena, Some(parent), Some(item)),
        Some(item)
    );
    assert_eq!(arena.child(parent), None);
    assert_eq!(arena.prev(item), None);
    assert_eq!(arena.next(item), None);

    delete(&mut arena, Some(parent));
    delete(&mut arena, Some(stray_prev));
    delete(&mut arena, Some(item));
    assert_eq!(arena.live_nodes(), 0);
}

/// MIGRATION.md CH-5. `cJSON_Delete` (cJSON.c) recurses once per nesting
/// level, so a tree built through the tree API (parse stops at
/// CJSON_NESTING_LIMIT) can exhaust the C stack. The Rust delete is
/// iterative; this depth would overflow the 2 MiB test-thread stack if it
/// recursed.
#[test]
fn delete_of_very_deep_tree_does_not_recurse() {
    const DEPTH: usize = 200_000;
    let mut arena = Arena::new();
    let root = create_array(&mut arena).expect("alloc");
    let mut current = root;
    for _ in 0..DEPTH {
        let inner = create_array(&mut arena).expect("alloc");
        assert!(add_item_to_array(&mut arena, Some(current), Some(inner)));
        current = inner;
    }
    assert_eq!(arena.live_nodes(), DEPTH + 1);

    delete(&mut arena, Some(root));
    assert_eq!(arena.live_nodes(), 0);
}
