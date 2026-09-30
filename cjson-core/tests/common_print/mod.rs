//! Helpers shared by the ported `print_array.c`, `print_object.c` and
//! `print_value.c` suites (the parts of `tests/common.h` they use).

use cjson_core::print::internals::PrintBuffer;
use cjson_core::store::Node;
use cjson_core::{tree, Arena, NodeId, NodeStore, CJSON_IS_REFERENCE, CJSON_STRING_IS_CONST};

/// A `printbuffer` over a caller-owned `unsigned char printed[1024]`:
/// `length = sizeof(printed)`, `offset = 0`, `noalloc = true`.
pub fn noalloc_buffer(length: usize) -> PrintBuffer {
    let mut buffer = PrintBuffer::new(length);
    buffer.noalloc = true;
    buffer
}

/// Parse buffer content for `parsebuffer.content = input;
/// parsebuffer.length = strlen(input) + sizeof("")`: the input plus its NUL.
pub fn parse_content(input: &str) -> Vec<u8> {
    let mut content = input.as_bytes().to_vec();
    content.push(0);
    content
}

/// `reset(cJSON *item)` from `tests/common.h`.
pub fn reset(arena: &mut Arena, item: NodeId) {
    let child = arena.child(item);
    if child.is_some() {
        tree::delete(arena, child);
    }
    let type_bits = arena.type_bits(item);
    if arena.valuestring(item).is_some() && (type_bits & CJSON_IS_REFERENCE) == 0 {
        arena.free_valuestring(item);
    }
    if arena.string(item).is_some() && (type_bits & CJSON_STRING_IS_CONST) == 0 {
        arena.free_string(item);
    }
    if let Some(node) = arena.node_mut(item) {
        *node = Node::default();
    }
}
