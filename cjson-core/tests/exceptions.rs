//! One `#[test]` per row of `PARITY_EXCEPTIONS.md`.
//!
//! The table has no rows: the SonarQube SECURITY findings on `cJSON.c`
//! (rule c:S6069, `sprintf` into fixed buffers) do not change observable
//! parse/print behavior, so no C/Rust divergence was introduced.
