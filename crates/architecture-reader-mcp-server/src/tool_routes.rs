//! Explicit shipped routing table for architecture-reader-mcp primary tools.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRoute {
    RustCore,
    LegacyOptIn,
}

pub fn route_for_tool(tool: &str) -> Option<ToolRoute> {
    match tool {
        "architecture_index"
        | "architecture_status"
        | "architecture_overview"
        | "architecture_search"
        | "architecture_trace"
        | "architecture_impact"
        | "architecture_evidence" => Some(ToolRoute::RustCore),
        _ => None,
    }
}

pub fn is_rust_core_tool(tool: &str) -> bool {
    matches!(route_for_tool(tool), Some(ToolRoute::RustCore))
}

pub const PRIMARY_TOOLS: [&str; 7] = [
    "architecture_index",
    "architecture_status",
    "architecture_overview",
    "architecture_search",
    "architecture_trace",
    "architecture_impact",
    "architecture_evidence",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_primary_tools_to_rust_core() {
        for tool in PRIMARY_TOOLS {
            assert_eq!(route_for_tool(tool), Some(ToolRoute::RustCore));
        }
    }
}