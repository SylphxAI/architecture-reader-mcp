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
    fn extracts_routes_and_schemas_from_fixture() {
        let root = fixture_root();
        let git = git::read_git_state(&root);
        let graph = scanner::scan_repository(&root, &scanner::ScanOptions::default(), git.commit, git.dirty);

        let routes: Vec<_> = graph.nodes.iter().filter(|n| n.kind == "route").collect();
        assert!(
            routes.iter().any(|n| n.label.contains("POST") && n.label.contains("/api/auth/login")),
            "expected login route, got: {:?}",
            routes.iter().map(|n| &n.label).collect::<Vec<_>>()
        );
        assert!(
            routes.iter().any(|n| n.label.contains("GET") && n.label.contains("/health")),
            "expected health route"
        );

        let schemas: Vec<_> = graph.nodes.iter().filter(|n| n.kind == "schema").collect();
        assert!(
            schemas.iter().any(|n| n.label == "LoginRequest" || n.label == "User"),
            "expected schema nodes, got: {:?}",
            schemas.iter().map(|n| &n.label).collect::<Vec<_>>()
        );
        assert!(graph.extractors.iter().any(|e| e.starts_with("routes@")));
        assert!(graph.extractors.iter().any(|e| e.starts_with("schema@")));
    }

    #[test]
    fn status_no_longer_reports_route_or_schema_gaps_after_index() {
        let root = fixture_root();
        let index_input = serde_json::json!({ "root": root.to_string_lossy(), "mode": "full" });
        let _ = engine::handle_tool("architecture_index", index_input);
        let status_input = serde_json::json!({ "root": root.to_string_lossy() });
        let envelope = engine::handle_tool("architecture_status", status_input);
        assert_eq!(envelope.status, "ok");
        let gaps = envelope.gaps;
        assert!(!gaps.iter().any(|g| g.contains("Route extraction")));
        assert!(!gaps.iter().any(|g| g.contains("Schema extraction")));
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