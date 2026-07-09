//! Architecture Reader MCP — Rust evidence graph engine.

pub mod engine;
pub mod git;
pub mod scanner;
pub mod store;
pub mod types;

pub use engine::handle_tool;
pub use types::{
    ArchitectureGraph, Confidence, ENGINE_NAME, ENGINE_VERSION, EvidenceRef, Freshness,
    ToolEnvelope, GRAPH_SCHEMA_VERSION,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/sample-repo")
            .canonicalize()
            .expect("fixture repo")
    }

    #[test]
    fn serializes_graph_snapshot_deterministically() {
        let root = fixture_root();
        let git = git::read_git_state(&root);
        let graph = scanner::scan_repository(&root, &scanner::ScanOptions::default(), git.commit, git.dirty);
        let a = serde_json::to_string(&graph).expect("serialize");
        let b = serde_json::to_string(&graph).expect("serialize");
        assert_eq!(a, b);
        assert!(!graph.nodes.is_empty());
        assert!(graph.nodes.iter().any(|n| n.kind == "package"));
    }

    #[test]
    fn indexes_fixture_and_returns_spec_envelope() {
        let root = fixture_root();
        let input = serde_json::json!({ "root": root.to_string_lossy(), "mode": "full" });
        let envelope = engine::handle_tool("architecture_index", input);
        assert_eq!(envelope.status, "ok");
        assert!(envelope.answer.is_some());
        assert!(envelope.metrics.unwrap().node_count > 0);
    }

    #[test]
    fn search_finds_auth_module_in_fixture() {
        let root = fixture_root();
        let index_input = serde_json::json!({ "root": root.to_string_lossy(), "mode": "full" });
        let _ = engine::handle_tool("architecture_index", index_input);
        let search_input = serde_json::json!({
            "root": root.to_string_lossy(),
            "query": "auth",
            "limit": 5
        });
        let envelope = engine::handle_tool("architecture_search", search_input);
        assert_eq!(envelope.status, "ok");
        let answer = envelope.answer.expect("answer");
        let matches = answer["matches"].as_array().expect("matches array");
        assert!(!matches.is_empty());
    }
}