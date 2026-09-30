//! `cJSON_Compare` (cJSON.c:3147-3270) and its private `compare_double`
//! helper (cJSON.c:595-599).
//!
//! Like C, the recursion into arrays and objects is unbounded: a cyclic tree
//! (only constructible through the FFI) overflows the stack in both
//! implementations.

use crate::consts::{
    CJSON_ARRAY, CJSON_FALSE, CJSON_NULL, CJSON_NUMBER, CJSON_OBJECT, CJSON_RAW, CJSON_STRING,
    CJSON_TRUE,
};
use crate::store::NodeStore;
use crate::tree::get_object_item;

/// `compare_double` (cJSON.c:595-599):
/// `fabs(a - b) <= max(fabs(a), fabs(b)) * DBL_EPSILON`.
///
/// Kept private so compare does not depend on the printer's copy. NaN never
/// compares equal (every comparison with NaN is false), and neither do two
/// equal infinities (`inf - inf` is NaN), exactly as in C.
fn compare_double(a: f64, b: f64) -> bool {
    let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    (a - b).abs() <= max_val * f64::EPSILON
}

/// `cJSON_Compare(a, b, case_sensitive)`. `None` models NULL.
///
/// Quirks reproduced from cJSON.c:
/// * Types are compared on `type & 0xFF` only, so `cJSON_IsReference` /
///   `cJSON_StringIsConst` flags are ignored (cJSON.c:3149).
/// * An invalid type returns false even when `a == b` (the validity switch at
///   cJSON.c:3155-3169 runs before the identity shortcut at cJSON.c:3172).
/// * Strings and raw values use `strcmp`, so only the bytes before the first
///   NUL matter; a NULL `valuestring` on either side is unequal
///   (cJSON.c:3194-3203).
/// * Objects are compared by looking every key of `a` up in `b` and then every
///   key of `b` up in `a` with `get_object_item` (cJSON.c:3233-3262). With
///   duplicate keys only the first match is ever found, and a member with a
///   NULL key makes the comparison fail.
pub fn compare<S: NodeStore>(
    store: &S,
    a: Option<S::Id>,
    b: Option<S::Id>,
    case_sensitive: bool,
) -> bool {
    let (a, b) = match (a, b) {
        (Some(a), Some(b)) => (a, b),
        _ => return false,
    };
    let a_type = store.type_bits(a) & 0xFF;
    if a_type != store.type_bits(b) & 0xFF {
        return false;
    }

    match a_type {
        CJSON_FALSE | CJSON_TRUE | CJSON_NULL | CJSON_NUMBER | CJSON_STRING | CJSON_RAW
        | CJSON_ARRAY | CJSON_OBJECT => {}
        _ => return false,
    }

    if a == b {
        return true;
    }

    match a_type {
        CJSON_FALSE | CJSON_TRUE | CJSON_NULL => true,

        CJSON_NUMBER => compare_double(store.valuedouble(a), store.valuedouble(b)),

        CJSON_STRING | CJSON_RAW => match (store.valuestring(a), store.valuestring(b)) {
            (Some(a_str), Some(b_str)) => a_str == b_str,
            _ => false,
        },

        CJSON_ARRAY => {
            let mut a_element = store.child(a);
            let mut b_element = store.child(b);
            while let (Some(a_el), Some(b_el)) = (a_element, b_element) {
                if !compare(store, Some(a_el), Some(b_el), case_sensitive) {
                    return false;
                }
                a_element = store.next(a_el);
                b_element = store.next(b_el);
            }
            // cJSON.c:3222: one of the arrays is longer than the other.
            a_element == b_element
        }

        CJSON_OBJECT => {
            object_members_found_in(store, a, b, case_sensitive)
                && object_members_found_in(store, b, a, case_sensitive)
        }

        _ => false,
    }
}

/// One direction of the object comparison (cJSON.c:3233-3245 and
/// 3250-3262): every member of `from` must be found in `other` by key and
/// compare equal as `cJSON_Compare(member, found)`.
fn object_members_found_in<S: NodeStore>(
    store: &S,
    from: S::Id,
    other: S::Id,
    case_sensitive: bool,
) -> bool {
    let mut element = store.child(from);
    while let Some(el) = element {
        let found = get_object_item(store, Some(other), store.string(el), case_sensitive);
        if found.is_none() {
            return false;
        }
        if !compare(store, Some(el), found, case_sensitive) {
            return false;
        }
        element = store.next(el);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::compare_double;

    #[test]
    fn compare_double_matches_c_edge_cases() {
        assert!(compare_double(1.0, 1.0));
        assert!(compare_double(0.0, -0.0));
        assert!(compare_double(1e100, 10e99));
        assert!(!compare_double(0.5e-100, 0.5e-101));
        assert!(!compare_double(f64::NAN, f64::NAN));
        assert!(!compare_double(f64::INFINITY, f64::INFINITY));
        assert!(!compare_double(1.0, 2.0));
    }
}
