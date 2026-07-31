//! Architecture Reader MCP — Rust evidence graph engine.

pub mod engine;
pub mod git;
pub mod scanner;
pub mod store;
pub mod synth;
pub mod synth_probe;
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
    use std::sync::{Mutex, OnceLock};

    fn fixture_index_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

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
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
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
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
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
    fn indexes_fixture_with_synth_extractor_when_probe_is_available() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let graph = scanner::scan_repository(
            &root,
            &scanner::ScanOptions {
                use_synth: true,
                ..scanner::ScanOptions::default()
            },
            None,
            false,
        );

        if graph
            .extractors
            .iter()
            .any(|extractor| extractor.starts_with("synth-"))
        {
            assert!(graph.evidence.iter().any(|ev| ev.extractor.starts_with("synth-")));
            assert!(graph.nodes.iter().any(|node| node.kind == "symbol"));
        }
    }

    #[test]
    fn auto_mode_returns_cache_hit_when_inventory_is_unchanged() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let full_input = serde_json::json!({ "root": root.to_string_lossy(), "mode": "full" });
        let first = engine::handle_tool("architecture_index", full_input.clone());
        assert_eq!(first.status, "ok");
        assert_eq!(first.answer.as_ref().and_then(|a| a.get("refreshMode")).and_then(|v| v.as_str()), Some("full"));

        let auto_input = serde_json::json!({ "root": root.to_string_lossy(), "mode": "auto" });
        let second = engine::handle_tool("architecture_index", auto_input);
        assert_eq!(second.status, "ok");
        assert_eq!(
            second.answer.as_ref().and_then(|a| a.get("refreshMode")).and_then(|v| v.as_str()),
            Some("cache_hit")
        );
    }

    #[test]
    fn indexes_python_imports_and_call_edges() {
        let root = fixture_root();
        let git = git::read_git_state(&root);
        let graph = scanner::scan_repository(&root, &scanner::ScanOptions::default(), git.commit, git.dirty);

        assert!(graph.nodes.iter().any(|node| {
            node.kind == "module" && node.path.as_deref() == Some("src/ml/scorer.py")
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == "imports"
                && edge.from.contains("scorer.py")
                && edge.to.contains("auth.token")
        }));
        assert!(graph.extractors.iter().any(|extractor| extractor.starts_with("python@")));
    }

    #[test]
    fn trace_finds_symbol_call_path_in_fixture() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let index_input = serde_json::json!({
            "root": root.to_string_lossy(),
            "mode": "full",
            "useSynth": false
        });
        let _ = engine::handle_tool("architecture_index", index_input);
        let trace_input = serde_json::json!({
            "root": root.to_string_lossy(),
            "from": "authMiddleware",
            "to": "validateToken",
            "relation": "calls",
            "maxDepth": 4
        });
        let envelope = engine::handle_tool("architecture_trace", trace_input);
        assert_eq!(envelope.status, "ok");
        let path = envelope
            .answer
            .as_ref()
            .and_then(|answer| answer.get("path"))
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(
            path.len() >= 2,
            "expected symbol call trace path, got {:?}",
            path
        );
    }

    #[test]
    fn search_finds_auth_module_in_fixture() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
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

    #[test]
    fn path_returns_hops_with_provenance_in_fixture() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let index_input = serde_json::json!({
            "root": root.to_string_lossy(),
            "mode": "full",
            "useSynth": false
        });
        let _ = engine::handle_tool("architecture_index", index_input);
        let path_input = serde_json::json!({
            "root": root.to_string_lossy(),
            "from": "authMiddleware",
            "to": "validateToken",
            "relation": "calls",
            "maxDepth": 6
        });
        let envelope = engine::handle_tool("architecture_path", path_input);
        assert_eq!(envelope.status, "ok", "{:?}", envelope.message);
        let answer = envelope.answer.expect("answer");
        assert!(answer.get("hopCount").is_some());
        assert!(answer.get("hops").is_some());
        assert!(answer.get("nodes").is_some());
        let hops = answer["hops"].as_array().cloned().unwrap_or_default();
        if !hops.is_empty() {
            assert!(hops[0].get("provenance").is_some());
            assert!(hops[0].get("edgeKind").is_some());
            let nodes = answer["nodes"].as_array().cloned().unwrap_or_default();
            assert!(nodes.len() >= 2);
        }
    }
    #[test]
    fn impact_reports_direct_nodes_for_changed_path() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let _ = engine::handle_tool(
            "architecture_index",
            serde_json::json!({ "root": root.to_string_lossy(), "mode": "full", "useSynth": false }),
        );
        let envelope = engine::handle_tool(
            "architecture_impact",
            serde_json::json!({
                "root": root.to_string_lossy(),
                "changedPaths": ["src/auth/token.ts"]
            }),
        );
        assert_eq!(envelope.status, "ok", "{:?}", envelope.message);
        let answer = envelope.answer.expect("answer");
        assert_eq!(answer["changedPathSource"], "explicit");
        let direct = answer["directImpact"].as_array().cloned().unwrap_or_default();
        assert!(
            !direct.is_empty(),
            "expected direct impact for auth token module, got {:?}",
            answer
        );
    }

    #[test]
    fn impact_use_git_diff_sets_source_git() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let _ = engine::handle_tool(
            "architecture_index",
            serde_json::json!({ "root": root.to_string_lossy(), "mode": "full", "useSynth": false }),
        );
        let envelope = engine::handle_tool(
            "architecture_impact",
            serde_json::json!({
                "root": root.to_string_lossy(),
                "useGitDiff": true
            }),
        );
        assert_eq!(envelope.status, "ok", "{:?}", envelope.message);
        let answer = envelope.answer.expect("answer");
        assert_eq!(answer["changedPathSource"], "git");
        assert!(answer.get("changedPaths").is_some());
    }



    #[test]
    fn indexes_rust_and_go_fixtures() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        // ensure fixture files present
        assert!(root.join("src/rust/token.rs").exists());
        assert!(root.join("src/go/auth/token.go").exists());
        let envelope = engine::handle_tool(
            "architecture_index",
            serde_json::json!({ "root": root.to_string_lossy(), "mode": "full", "useSynth": false }),
        );
        assert_eq!(envelope.status, "ok", "{:?}", envelope.message);
        let overview = engine::handle_tool(
            "architecture_search",
            serde_json::json!({ "root": root.to_string_lossy(), "query": "issue_token" }),
        );
        assert_eq!(overview.status, "ok", "{:?}", overview.message);
        let answer = overview.answer.expect("answer");
        let matches = answer["matches"].as_array().cloned().unwrap_or_default();
        assert!(
            matches.iter().any(|m| m["id"].as_str().unwrap_or("").contains("issue_token")
                || m["label"].as_str().unwrap_or("").contains("issue_token")),
            "expected issue_token match, got {:?}",
            matches
        );
        let go = engine::handle_tool(
            "architecture_search",
            serde_json::json!({ "root": root.to_string_lossy(), "query": "IssueToken" }),
        );
        assert_eq!(go.status, "ok");
        let gans = go.answer.expect("answer");
        let gmatches = gans["matches"].as_array().cloned().unwrap_or_default();
        assert!(
            gmatches.iter().any(|m| m["label"].as_str().unwrap_or("").contains("IssueToken")
                || m["id"].as_str().unwrap_or("").contains("IssueToken")),
            "expected IssueToken match {:?}",
            gmatches
        );
    }


    #[test]
    fn search_ranks_exact_symbol_above_path_substring() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let _ = engine::handle_tool(
            "architecture_index",
            serde_json::json!({ "root": root.to_string_lossy(), "mode": "full", "useSynth": false }),
        );
        let envelope = engine::handle_tool(
            "architecture_search",
            serde_json::json!({ "root": root.to_string_lossy(), "query": "issue_token", "limit": 10 }),
        );
        assert_eq!(envelope.status, "ok", "{:?}", envelope.message);
        let matches = envelope.answer.expect("answer")["matches"].as_array().cloned().unwrap_or_default();
        assert!(!matches.is_empty());
        let top = &matches[0];
        assert!(
            top["label"].as_str() == Some("issue_token") || top["id"].as_str().unwrap_or("").contains("issue_token"),
            "top match should prefer exact symbol, got {:?}",
            top
        );
        let scores: Vec<f64> = matches.iter().filter_map(|m| m["score"].as_f64()).collect();
        assert!(scores.windows(2).all(|w| w[0] >= w[1]), "scores should be non-increasing: {:?}", scores);

        let overview = engine::handle_tool(
            "architecture_overview",
            serde_json::json!({ "root": root.to_string_lossy(), "depth": 3 }),
        );
        assert_eq!(overview.status, "ok");
        let ans = overview.answer.expect("answer");
        assert!(ans.get("counts").is_some());
        assert!(ans.get("languages").is_some());
        assert!(ans.get("extractors").is_some());
    }


    #[test]
    fn impact_reports_incoming_dependents() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let _ = engine::handle_tool(
            "architecture_index",
            serde_json::json!({ "root": root.to_string_lossy(), "mode": "full", "useSynth": false }),
        );
        // token.ts is imported by middleware / routes in fixture — expect reverse edges when present
        let envelope = engine::handle_tool(
            "architecture_impact",
            serde_json::json!({
                "root": root.to_string_lossy(),
                "changedPaths": ["src/auth/token.ts"],
                "maxDepth": 2
            }),
        );
        assert_eq!(envelope.status, "ok", "{:?}", envelope.message);
        let answer = envelope.answer.expect("answer");
        assert!(answer.get("incomingImpact").is_some());
        assert!(answer.get("outgoingImpact").is_some());
        assert_eq!(answer["maxDepth"], 2);
        let direct = answer["directImpact"].as_array().cloned().unwrap_or_default();
        assert!(!direct.is_empty(), "expected direct impact nodes");
        // At least one of incoming/outgoing should be non-empty on this fixture
        let inc = answer["incomingImpact"].as_array().cloned().unwrap_or_default();
        let out = answer["outgoingImpact"].as_array().cloned().unwrap_or_default();
        assert!(
            !inc.is_empty() || !out.is_empty(),
            "expected blast-radius edges, answer={:?}",
            answer
        );
    }


    #[test]
    fn overview_returns_neighbors_for_focus() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let _ = engine::handle_tool(
            "architecture_index",
            serde_json::json!({ "root": root.to_string_lossy(), "mode": "full", "useSynth": false }),
        );
        let envelope = engine::handle_tool(
            "architecture_overview",
            serde_json::json!({
                "root": root.to_string_lossy(),
                "focus": "src/auth/token.ts",
                "depth": 3
            }),
        );
        assert_eq!(envelope.status, "ok", "{:?}", envelope.message);
        let answer = envelope.answer.expect("answer");
        assert!(answer.get("focusNodeId").is_some());
        let neighbors = answer["neighbors"].as_array().cloned().unwrap_or_default();
        assert!(
            !neighbors.is_empty(),
            "expected neighbors for token.ts focus, answer={:?}",
            answer
        );
    }


    #[test]
    fn search_score_explain_and_neighbors() {
        let _guard = fixture_index_lock().lock().expect("fixture index lock");
        let root = fixture_root();
        let _ = engine::handle_tool(
            "architecture_index",
            serde_json::json!({ "root": root.to_string_lossy(), "mode": "full", "useSynth": false }),
        );
        let envelope = engine::handle_tool(
            "architecture_search",
            serde_json::json!({
                "root": root.to_string_lossy(),
                "query": "issue_token",
                "limit": 5,
                "includeNeighbors": true
            }),
        );
        assert_eq!(envelope.status, "ok", "{:?}", envelope.message);
        let answer = envelope.answer.expect("answer");
        assert_eq!(answer["includeNeighbors"], true);
        let matches = answer["matches"].as_array().cloned().unwrap_or_default();
        assert!(!matches.is_empty());
        let top = &matches[0];
        let explain = top["scoreExplain"].as_array().cloned().unwrap_or_default();
        assert!(
            explain.iter().any(|e| e.as_str() == Some("exact_label") || e.as_str() == Some("label_substring") || e.as_str() == Some("label_prefix")),
            "scoreExplain={:?}",
            explain
        );
        assert!(top.get("neighbors").is_some(), "expected neighbors on match");
    }

}
