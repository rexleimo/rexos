use std::collections::HashSet;

use super::transport::{allocate_local_tool_name, sanitize_component};

#[test]
fn sanitize_component_rewrites_non_ascii_to_underscores() {
    assert_eq!(sanitize_component("GitHub"), "github");
    assert_eq!(sanitize_component("a b:c"), "a_b_c");
    assert_eq!(sanitize_component("___x___"), "x");
}

#[test]
fn allocate_local_tool_name_is_namespaced_and_bounded() {
    let mut used = HashSet::new();
    let name = allocate_local_tool_name("my-server", "very_long_tool_name", &mut used);
    assert!(name.starts_with("mcp_"));
    assert!(name.len() <= 64);
}
