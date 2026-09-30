//! `cJSON_Duplicate` / `cJSON_Duplicate_rec` including the
//! `CJSON_CIRCULAR_LIMIT` depth guard (cJSON.c:2840-2933).

use crate::consts::{CJSON_CIRCULAR_LIMIT, CJSON_IS_REFERENCE, CJSON_STRING_IS_CONST};
use crate::store::NodeStore;
use crate::tree::delete;

/// `cJSON_Duplicate(item, recurse)`. `None` models NULL; returns None
/// where C returns NULL (NULL input, allocation failure, depth limit).
pub fn duplicate<S: NodeStore>(store: &mut S, item: Option<S::Id>, recurse: bool) -> Option<S::Id> {
    duplicate_rec(store, item, 0, recurse)
}

/// `cJSON_Duplicate_rec(item, depth, recurse)` (cJSON.c:2849-2933).
///
/// Quirks reproduced from cJSON.c:
/// * The copy's type is `item->type & ~cJSON_IsReference`, so a reference
///   becomes an owning copy while `cJSON_StringIsConst` is kept
///   (cJSON.c:2868).
/// * A const key is aliased, not copied (cJSON.c:2881); every other key and
///   `valuestring` is `strdup`ed, which copies only the bytes before the
///   first NUL (cJSON.c:2873, 2881).
/// * The depth guard is only checked inside the child loop, so a leaf is
///   always copied and the failure happens when a node at depth
///   `CJSON_CIRCULAR_LIMIT` has a child (cJSON.c:2896-2898).
/// * Children are always duplicated with `recurse = true` (cJSON.c:2899),
///   and the head's `prev` is pointed at the last copied child only after
///   the whole list is built (cJSON.c:2919-2922).
/// * Any failure frees the partially built copy with `cJSON_Delete`,
///   including already linked children (cJSON.c:2926-2932). Every
///   enclosing level then fails and deletes its own copy, innermost first.
///
/// C recurses up to `CJSON_CIRCULAR_LIMIT` (10000) levels deep. That fits
/// C's main-thread stack but can overflow a Rust thread stack, so the
/// recursion is replayed with an explicit heap stack of [`Frame`]s holding
/// C's locals. Allocation order, the depth check, list linking and cleanup
/// order are those of the recursive C code.
pub fn duplicate_rec<S: NodeStore>(
    store: &mut S,
    item: Option<S::Id>,
    depth: usize,
    recurse: bool,
) -> Option<S::Id> {
    let item = item?;
    let newitem = copy_item(store, item)?;
    if !recurse {
        return Some(newitem);
    }

    let mut stack = vec![Frame {
        newitem,
        child: store.child(item),
        next: None,
        newchild: None,
        depth,
    }];
    loop {
        let (child, depth) = stack.last().map(|top| (top.child, top.depth))?;
        match child {
            Some(current) => {
                // cJSON.c:2896-2899: depth guard, then the recursive call.
                let copy = if depth >= CJSON_CIRCULAR_LIMIT {
                    None
                } else {
                    depth
                        .checked_add(1)
                        .and_then(|d| copy_item(store, current).map(|c| (c, d)))
                };
                match copy {
                    Some((newitem, depth)) => {
                        stack.push(Frame {
                            newitem,
                            child: store.child(current),
                            next: None,
                            newchild: None,
                            depth,
                        });
                    }
                    None => {
                        while let Some(frame) = stack.pop() {
                            delete(store, Some(frame.newitem));
                        }
                        return None;
                    }
                }
            }
            None => {
                let done = stack.pop()?;
                // cJSON.c:2919-2922
                if let Some(head) = store.child(done.newitem) {
                    store.set_prev(head, done.newchild);
                }
                let Some(parent) = stack.last_mut() else {
                    return Some(done.newitem);
                };
                // cJSON.c:2904-2917, back in the caller's loop.
                let copy = done.newitem;
                parent.newchild = Some(copy);
                match parent.next {
                    Some(tail) => {
                        store.set_next(tail, Some(copy));
                        store.set_prev(copy, Some(tail));
                    }
                    None => store.set_child(parent.newitem, Some(copy)),
                }
                parent.next = Some(copy);
                parent.child = parent.child.and_then(|c| store.next(c));
            }
        }
    }
}

/// The locals of one `cJSON_Duplicate_rec` activation that is walking its
/// child list (cJSON.c:2851-2854).
struct Frame<Id> {
    newitem: Id,
    /// C's `child`: the original child to copy next.
    child: Option<Id>,
    /// C's `next`: tail of the copied child list.
    next: Option<Id>,
    /// C's `newchild`: the most recently copied child.
    newchild: Option<Id>,
    depth: usize,
}

/// cJSON.c:2862-2886: allocate `newitem` and copy type, numbers and
/// strings. On allocation failure the partial node is deleted (`goto fail`).
fn copy_item<S: NodeStore>(store: &mut S, item: S::Id) -> Option<S::Id> {
    let newitem = store.alloc_node().ok()?;
    if copy_fields(store, item, newitem).is_none() {
        delete(store, Some(newitem));
        return None;
    }
    Some(newitem)
}

fn copy_fields<S: NodeStore>(store: &mut S, item: S::Id, newitem: S::Id) -> Option<()> {
    let item_type = store.type_bits(item);
    store.set_type_bits(newitem, item_type & !CJSON_IS_REFERENCE);
    let valueint = store.valueint(item);
    store.set_valueint(newitem, valueint);
    let valuedouble = store.valuedouble(item);
    store.set_valuedouble(newitem, valuedouble);

    if let Some(valuestring) = store.valuestring(item).map(<[u8]>::to_vec) {
        store.set_valuestring_copy(newitem, &valuestring).ok()?;
    }
    if item_type & CJSON_STRING_IS_CONST != 0 {
        if store.string(item).is_some() {
            store.alias_string(newitem, item);
        }
    } else if let Some(key) = store.string(item).map(<[u8]>::to_vec) {
        store.set_string_copy(newitem, &key).ok()?;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::duplicate;
    use crate::consts::{CJSON_ARRAY, CJSON_IS_REFERENCE, CJSON_STRING, CJSON_STRING_IS_CONST};
    use crate::store::{Arena, NodeId, NodeStore};
    use crate::tree::{
        add_item_to_array, add_item_to_object_cs, create_array, create_string,
        create_string_reference, delete, detach_item_from_array,
    };
    use std::rc::Rc;

    /// `nodes` nested arrays `[[[...]]]`.
    fn chain(store: &mut Arena, nodes: usize) -> Option<NodeId> {
        let root = create_array(store);
        let mut current = root;
        for _ in 1..nodes {
            let next = create_array(store);
            assert!(add_item_to_array(store, current, next));
            current = next;
        }
        root
    }

    /// Recorded from the original cJSON.c (build/probes/duplicate_probe.c):
    /// 10000 and 10001 nested arrays duplicate, 10002 and 10003 return NULL.
    #[test]
    fn depth_limit_boundary_matches_c() {
        for (nodes, expect_ok) in [(10000, true), (10001, true), (10002, false), (10003, false)] {
            let mut store = Arena::new();
            let original = chain(&mut store, nodes);
            let before = store.live_allocations();
            let copy = duplicate(&mut store, original, true);
            assert_eq!(copy.is_some(), expect_ok, "nodes={nodes}");
            if copy.is_none() {
                assert_eq!(
                    store.live_allocations(),
                    before,
                    "failed duplicate leaked, nodes={nodes}"
                );
            }
            delete(&mut store, copy);
            delete(&mut store, original);
            assert_eq!(store.live_allocations(), 0);
        }
    }

    /// Same shape as misc_tests.c `cjson_should_not_follow_too_deep_circular_references`.
    #[test]
    fn circular_reference_returns_none() {
        let mut store = Arena::new();
        let o = create_array(&mut store);
        let a = create_array(&mut store);
        let b = create_array(&mut store);
        add_item_to_array(&mut store, o, a);
        add_item_to_array(&mut store, a, b);
        add_item_to_array(&mut store, b, o);

        assert_eq!(duplicate(&mut store, o, true), None);
        detach_item_from_array(&mut store, b, 0);
        delete(&mut store, o);
        assert_eq!(store.live_allocations(), 0);
    }

    /// Recorded from the original cJSON.c: duplicating this document needs
    /// exactly 12 allocations, and every failing budget leaks nothing.
    #[test]
    fn allocation_failures_match_c() {
        let mut store = Arena::new();
        let doc = crate::parse::parse(&mut store, b"{\"a\":[1,\"x\",{\"b\":null}],\"c\":\"d\"}")
            .map(|ok| ok.root)
            .ok();
        assert!(doc.is_some());
        for budget in 0..=16 {
            let before = store.live_allocations();
            store.set_alloc_budget(Some(budget));
            let copy = duplicate(&mut store, doc, true);
            store.set_alloc_budget(None);
            assert_eq!(copy.is_some(), budget >= 12, "budget={budget}");
            if copy.is_none() {
                assert_eq!(store.live_allocations(), before, "budget={budget}");
            }
            delete(&mut store, copy);
        }
        delete(&mut store, doc);
        assert_eq!(store.live_allocations(), 0);
    }

    #[test]
    fn null_item_returns_none() {
        let mut store = Arena::new();
        assert_eq!(duplicate(&mut store, None, true), None);
        assert_eq!(store.live_allocations(), 0);
    }

    /// cJSON.c:2868 clears `cJSON_IsReference` and cJSON.c:2881 aliases a
    /// const key while keeping `cJSON_StringIsConst`.
    #[test]
    fn reference_flag_cleared_and_const_key_aliased() {
        let mut store = Arena::new();
        let object = crate::tree::create_object(&mut store);
        let value: Rc<[u8]> = Rc::from(&b"value"[..]);
        let key: Rc<[u8]> = Rc::from(&b"key"[..]);
        let item = create_string_reference(&mut store, Some(value));
        assert!(add_item_to_object_cs(
            &mut store,
            object,
            Some(Rc::clone(&key)),
            item
        ));
        let item = item.unwrap();

        let copy = duplicate(&mut store, Some(item), false).unwrap();
        assert_eq!(store.type_bits(copy), CJSON_STRING | CJSON_STRING_IS_CONST);
        assert_eq!(store.type_bits(copy) & CJSON_IS_REFERENCE, 0);
        assert_eq!(store.valuestring(copy), Some(&b"value"[..]));
        let copied_key = store.node(copy).and_then(|n| n.string.clone()).unwrap();
        assert!(Rc::ptr_eq(&copied_key, &key));
        let copied_value = store
            .node(copy)
            .and_then(|n| n.valuestring.clone())
            .unwrap();
        let original_value = store
            .node(item)
            .and_then(|n| n.valuestring.clone())
            .unwrap();
        assert!(!Rc::ptr_eq(&copied_value, &original_value));

        delete(&mut store, Some(copy));
        delete(&mut store, object);
    }

    /// `strdup` copies only the bytes before the first NUL (cJSON.c:2873).
    #[test]
    fn valuestring_copy_stops_at_nul() {
        let mut store = Arena::new();
        let item = create_string(&mut store, Some(b"ab")).unwrap();
        if let Some(node) = store.node_mut(item) {
            node.valuestring = Some(Rc::from(&b"a\0b"[..]));
        }
        let copy = duplicate(&mut store, Some(item), false).unwrap();
        let raw = store
            .node(copy)
            .and_then(|n| n.valuestring.clone())
            .unwrap();
        assert_eq!(&*raw, b"a");
        delete(&mut store, Some(copy));
        delete(&mut store, Some(item));
    }

    /// Children are linked in order and the head's `prev` points at the last
    /// child (cJSON.c:2904-2922); `recurse = false` copies no children.
    #[test]
    fn child_links_and_shallow_copy() {
        let mut store = Arena::new();
        let array = create_array(&mut store);
        for text in [&b"1"[..], b"2", b"3"] {
            let s = create_string(&mut store, Some(text));
            add_item_to_array(&mut store, array, s);
        }

        let deep = duplicate(&mut store, array, true).unwrap();
        assert_eq!(store.type_bits(deep), CJSON_ARRAY);
        let first = store.child(deep).unwrap();
        let second = store.next(first).unwrap();
        let third = store.next(second).unwrap();
        assert_eq!(store.next(third), None);
        assert_eq!(store.prev(first), Some(third));
        assert_eq!(store.prev(second), Some(first));
        assert_eq!(store.prev(third), Some(second));
        assert_eq!(store.valuestring(third), Some(&b"3"[..]));

        let shallow = duplicate(&mut store, array, false).unwrap();
        assert_eq!(store.child(shallow), None);

        delete(&mut store, Some(deep));
        delete(&mut store, Some(shallow));
        delete(&mut store, array);
        assert_eq!(store.live_allocations(), 0);
    }
}
