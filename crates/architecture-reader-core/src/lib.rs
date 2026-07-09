//! Core contracts for Architecture Reader MCP.
//!
//! This crate is intentionally small in the scaffold. The first implementation
//! slice should add serde-backed graph model tests before query logic.

pub const ENGINE_NAME: &str = "architecture-reader-core";
pub const ENGINE_VERSION: &str = "0.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceSource {
    Manifest,
    Ast,
    ImportGraph,
    Documentation,
    Workflow,
    Schema,
    Inference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRef {
    pub id: String,
    pub source: EvidenceSource,
    pub path: String,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_engine_identity() {
        assert_eq!(ENGINE_NAME, "architecture-reader-core");
        assert_eq!(ENGINE_VERSION, "0.0.0");
    }
}
