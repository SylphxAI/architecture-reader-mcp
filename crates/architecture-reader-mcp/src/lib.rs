//! Rust MCP server scaffold for Architecture Reader.
//!
//! The first implementation slice should replace this manifest-only scaffold
//! with `rmcp` tool handlers backed by `architecture-reader-core`.

pub const SERVER_NAME: &str = "@sylphx/architecture-reader-mcp";
pub const SERVER_VERSION: &str = "0.0.0";
pub const MCP_SDK: &str = "modelcontextprotocol/rust-sdk rmcp";

pub const PLANNED_TOOLS: &[&str] = &[
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
    fn exposes_rust_mcp_identity() {
        assert_eq!(SERVER_NAME, "@sylphx/architecture-reader-mcp");
        assert_eq!(MCP_SDK, "modelcontextprotocol/rust-sdk rmcp");
        assert!(PLANNED_TOOLS.contains(&"architecture_search"));
    }
}
