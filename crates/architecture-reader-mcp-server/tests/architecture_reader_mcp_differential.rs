//! TRUE pure differential parity residual: TS pure-contract oracle vs Rust rmcp SSOT.
//!
//! Fail-closed — no SKIP-as-pass. Bounded pure slices only (BW2 residual):
//! - tool-route-contract / allow-list / server-contract
//!
//! Explicitly NOT claimed: architecture_* graph effect parity, HTTP transport,
//! parity_proven, authority_rust, ts_deleted.
//! See scripts/run-architecture-reader-differential.sh.

use architecture_reader_mcp_server::tool_routes::{
    is_rust_core_tool, route_for_tool, ToolRoute, PRIMARY_TOOLS,
};
use architecture_reader_mcp_server::{SERVER_NAME, SERVER_VERSION};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    fs::canonicalize(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .expect("canonicalize repo root")
}

#[derive(Debug, Deserialize)]
struct OracleCase {
    id: String,
    slice: String,
    domain: String,
    input: Value,
    output: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OracleCorpus {
    corpus_version: u32,
    fixture_corpus_hash: String,
    cases: Vec<OracleCase>,
}

fn run_ts_oracle() -> OracleCorpus {
    if let Ok(path) = std::env::var("ARCHITECTURE_READER_MCP_ORACLE_JSON") {
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read ARCHITECTURE_READER_MCP_ORACLE_JSON at {path}: {error}"));
        return serde_json::from_str(&raw).expect("oracle JSON must be valid");
    }

    let script = repo_root().join("scripts/differential/architecture-reader-mcp-oracle.ts");
    let output = spawn_oracle(&script);

    assert!(
        output.status.success(),
        "TS pure oracle failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("oracle output must be valid JSON")
}

fn spawn_oracle(script: &Path) -> std::process::Output {
    if Command::new("bun")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Command::new("bun")
            .arg("run")
            .arg(script)
            .current_dir(repo_root())
            .output()
            .unwrap_or_else(|error| panic!("spawn bun oracle at {}: {error}", script.display()));
    }

    Command::new("node")
        .arg("--experimental-strip-types")
        .arg(script)
        .current_dir(repo_root())
        .output()
        .unwrap_or_else(|error| panic!("spawn node oracle at {}: {error}", script.display()))
}

fn route_label(route: Option<ToolRoute>) -> Option<&'static str> {
    match route {
        Some(ToolRoute::RustCore) => Some("RustCore"),
        Some(ToolRoute::LegacyOptIn) => Some("LegacyOptIn"),
        None => None,
    }
}

fn compare_tool_route_case(case: &OracleCase) {
    let tool = case.input["tool"].as_str().expect("tool route tool");
    let actual = route_label(route_for_tool(tool));
    let expected = case.output["route"].as_str();
    assert_eq!(
        actual,
        expected,
        "{}: route mismatch for tool {tool}",
        case.id
    );
}

fn compare_server_contract(case: &OracleCase) {
    assert_eq!(
        case.output["name"].as_str(),
        Some(SERVER_NAME),
        "{}: server name",
        case.id
    );
    assert_eq!(
        case.output["version"].as_str(),
        Some(SERVER_VERSION),
        "{}: server version",
        case.id
    );
    let tools = case.output["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|v| v.as_str().expect("tool str"))
        .collect::<Vec<_>>();
    assert_eq!(tools, PRIMARY_TOOLS.to_vec(), "{}: primary tools", case.id);
}

fn compare_allow_list(case: &OracleCase) {
    let tools = case.output["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|v| v.as_str().expect("tool str"))
        .collect::<Vec<_>>();
    assert_eq!(tools, PRIMARY_TOOLS.to_vec(), "{}: allow-list tools", case.id);
    for tool in &tools {
        assert!(
            is_rust_core_tool(tool),
            "{}: allow-listed tool {tool} must route RustCore",
            case.id
        );
    }
}

#[test]
fn pure_residual_differential_matches_ts_oracle() {
    let oracle = run_ts_oracle();
    assert_eq!(oracle.corpus_version, 1);
    assert!(!oracle.fixture_corpus_hash.is_empty());
    assert!(
        !oracle.cases.is_empty(),
        "oracle must emit at least one pure residual case"
    );

    let mut route_count = 0usize;
    let mut server_count = 0usize;
    let mut allow_count = 0usize;

    for case in &oracle.cases {
        match case.domain.as_str() {
            "toolRouteContract" => {
                compare_tool_route_case(case);
                route_count += 1;
            }
            "serverContract" => {
                compare_server_contract(case);
                server_count += 1;
            }
            "allowList" => {
                compare_allow_list(case);
                allow_count += 1;
            }
            other => panic!("unexpected oracle domain {other} for case {}", case.id),
        }
        let _ = &case.slice;
    }

    assert!(route_count >= 7, "expected ≥7 tool route cases, got {route_count}");
    assert_eq!(server_count, 1);
    assert_eq!(allow_count, 1);
}
