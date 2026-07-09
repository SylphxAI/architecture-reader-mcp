pub mod cli_bridge;
pub mod tool_routes;

use rmcp::{
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData, ServerHandler,
};
use serde_json::Value;

pub const SERVER_NAME: &str = "architecture-reader-mcp";
pub const SERVER_VERSION: &str = "0.1.0";
pub const SERVER_INSTRUCTIONS: &str =
    "Architecture Reader MCP server (Rust rmcp transport). Index, search, trace, impact, and evidence tools run through the Rust evidence-graph engine.";

#[derive(Clone)]
pub struct ArchitectureReaderMcp {
    pub tool_router: ToolRouter<Self>,
}

impl ArchitectureReaderMcp {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    fn invoke(&self, tool: &str, args: Value) -> Result<rmcp::model::CallToolResult, ErrorData> {
        cli_bridge::invoke_cli_tool(tool, args)
    }
}

#[tool_router]
impl ArchitectureReaderMcp {
    #[tool(description = "Index or refresh the architecture evidence graph for a repository.")]
    fn architecture_index(
        &self,
        Parameters(args): Parameters<Value>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        self.invoke("architecture_index", args)
    }

    #[tool(description = "Report indexed repository status, extractors, and coverage gaps.")]
    fn architecture_status(
        &self,
        Parameters(args): Parameters<Value>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        self.invoke("architecture_status", args)
    }

    #[tool(description = "Return package and module overview slices for the indexed graph.")]
    fn architecture_overview(
        &self,
        Parameters(args): Parameters<Value>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        self.invoke("architecture_overview", args)
    }

    #[tool(description = "Search the architecture graph with evidence locators.")]
    fn architecture_search(
        &self,
        Parameters(args): Parameters<Value>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        self.invoke("architecture_search", args)
    }

    #[tool(description = "Trace relations between architecture nodes or symbols.")]
    fn architecture_trace(
        &self,
        Parameters(args): Parameters<Value>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        self.invoke("architecture_trace", args)
    }

    #[tool(description = "Compute impact for changed paths in the indexed graph.")]
    fn architecture_impact(
        &self,
        Parameters(args): Parameters<Value>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        self.invoke("architecture_impact", args)
    }

    #[tool(description = "Resolve architecture evidence records by id.")]
    fn architecture_evidence(
        &self,
        Parameters(args): Parameters<Value>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        self.invoke("architecture_evidence", args)
    }
}

#[tool_handler]
impl ServerHandler for ArchitectureReaderMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: SERVER_NAME.into(),
                title: None,
                version: SERVER_VERSION.into(),
                description: Some(
                    "Rust-native MCP server for architecture-reader-mcp (modelcontextprotocol/rust-sdk rmcp)"
                        .into(),
                ),
                icons: None,
                website_url: Some("https://github.com/SylphxAI/architecture-reader-mcp".into()),
            },
            instructions: Some(SERVER_INSTRUCTIONS.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArchitectureReaderMcp;
    use crate::tool_routes::PRIMARY_TOOLS;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn rmcp_server_routes_primary_tools_through_cli_bridge() {
        let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let lib_rs = fs::read_to_string(src_dir.join("lib.rs")).expect("read lib.rs");
        let production_lib = lib_rs.split("#[cfg(test)]").next().unwrap_or(&lib_rs);
        assert!(production_lib.contains("cli_bridge::invoke_cli_tool"));
    }

    #[test]
    fn exposes_all_primary_tools() {
        let tools = ArchitectureReaderMcp::new().tool_router.list_all();
        let names: Vec<_> = tools.iter().map(|tool| tool.name.to_string()).collect();
        for tool in PRIMARY_TOOLS {
            assert!(names.contains(&tool.to_string()), "missing tool {tool}");
        }
    }
}