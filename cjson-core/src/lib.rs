//! Safe Rust port of cJSON 1.7.19.
//!
//! Every algorithm from `cJSON.c` lives here and is written against the
//! [`store::NodeStore`] trait, which models the C `cJSON` node (linked
//! `next`/`prev`/`child`, `type` bits, `valuestring`, `valueint`,
//! `valuedouble`, `string`) through opaque handles. Two stores exist:
//!
//! * [`store::Arena`] in this crate, used by Rust callers, the Rust parity
//!   driver and the ported Unity tests.
//! * `CStore` in `cjson-ffi`, which implements the same trait over the live
//!   `#[repr(C)]` tree that C callers own.
//!
//! Behavior (including quirks) must match `cJSON.c` byte for byte. See
//! `PARITY.md` and `MIGRATION.md` at the repository root.
#![forbid(unsafe_code)]

pub mod compare;
pub mod consts;
pub mod duplicate;
pub mod minify;
pub mod number;
pub mod parse;
pub mod print;
pub mod store;
pub mod tree;

pub use consts::*;
pub use store::{AllocError, Arena, NodeId, NodeStore};
