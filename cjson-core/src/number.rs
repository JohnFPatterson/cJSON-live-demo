//! Number helpers shared by parser, tree operations and FFI.

/// `valueint` saturation used by `parse_number` (cJSON.c:394-406),
/// `cJSON_SetNumberHelper` (cJSON.c:423-436) and `cJSON_CreateNumber`.
///
/// C: `if (n >= INT_MAX) INT_MAX; else if (n <= (double)INT_MIN) INT_MIN; else (int)n;`
/// NaN fails both comparisons and C then evaluates `(int)NaN`, which is
/// undefined behavior; on the platforms we test (x86-64 and arm64) the
/// observed result is `INT_MIN` on x86-64 and `0` on arm64. Rust's `as`
/// cast yields `0`, which matches arm64.
pub fn saturate_to_int(number: f64) -> i32 {
    if number >= i32::MAX as f64 {
        i32::MAX
    } else if number <= i32::MIN as f64 {
        i32::MIN
    } else {
        number as i32
    }
}
