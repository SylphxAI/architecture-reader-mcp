use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::types::{
    ArchitectureGraph, Confidence, EvidenceRef, GraphClaim, GraphEdge, GraphNode, RepositorySnapshot,
    GRAPH_SCHEMA_VERSION,
};

const DEFAULT_EXCLUDES: &[&str] = &[
    "node_modules",
    "dist",
    "target",
    ".git",
    ".architecture-reader",
];

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub max_file_bytes: u64,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            include: vec![],
            exclude: DEFAULT_EXCLUDES.iter().map(|s| (*s).to_string()).collect(),
            max_file_bytes: 1_048_576,
        }
    }
}

#[derive(Debug, Default)]
struct GraphBuilder {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    claims: Vec<GraphClaim>,
    evidence: Vec<EvidenceRef>,
    evidence_seq: u32,
}

impl GraphBuilder {
    fn push_evidence(&mut self, kind: &str, path: &str, extractor: &str, start: Option<u32>, end: Option<u32>) -> String {
        self.evidence_seq += 1;
        let id = format!("ev_{:02}", self.evidence_seq);
        self.evidence.push(EvidenceRef {
            id: id.clone(),
            kind: kind.into(),
            path: path.into(),
            start_line: start,
            end_line: end,
            extractor: extractor.into(),
            confidence: Confidence::Deterministic,
        });
        id
    }

    fn push_node(&mut self, id: &str, kind: &str, label: &str, path: Option<&str>, evidence_id: &str) {
        self.nodes.push(GraphNode {
            id: id.into(),
            kind: kind.into(),
            label: label.into(),
            path: path.map(str::to_string),
            evidence_ids: vec![evidence_id.into()],
        });
    }

    fn push_edge(&mut self, kind: &str, from: &str, to: &str, evidence_id: &str) {
        let id = format!("edge:{kind}:{from}->{to}");
        self.edges.push(GraphEdge {
            id,
            kind: kind.into(),
            from: from.into(),
            to: to.into(),
            evidence_ids: vec![evidence_id.into()],
        });
    }
}

pub fn scan_repository(root: &Path, options: &ScanOptions, git_commit: Option<String>, dirty: bool) -> ArchitectureGraph {
    let mut builder = GraphBuilder::default();
    let root_str = root.to_string_lossy().to_string();
    let repo_ev = builder.push_evidence("manifest", &root_str, "repo-scanner@0.1.0", None, None);
    builder.push_node("node:repository:root", "repository", &root_str, Some(&root_str), &repo_ev);

    let mut files_scanned = 0u32;
    let mut files_indexed = 0u32;

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if !should_include(path, root, options) {
            continue;
        }
        files_scanned += 1;
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() > options.max_file_bytes {
                continue;
            }
        }

        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if file_name == "package.json" || file_name == "Cargo.toml" {
            index_manifest(&mut builder, &rel_str, path);
            files_indexed += 1;
            continue;
        }

        if rel_str.starts_with("docs/") && rel_str.ends_with(".md") {
            index_document(&mut builder, &rel_str, if rel_str.contains("/adr/") { "adr" } else { "document" });
            files_indexed += 1;
            continue;
        }

        if rel_str.ends_with(".ts")
            || rel_str.ends_with(".tsx")
            || rel_str.ends_with(".js")
            || rel_str.ends_with(".mjs")
        {
            index_ts_imports(&mut builder, &rel_str, path);
            files_indexed += 1;
            continue;
        }

        if rel_str.ends_with(".rs") {
            index_rs_imports(&mut builder, &rel_str, path);
            files_indexed += 1;
        }
    }

    let package_count = builder.nodes.iter().filter(|n| n.kind == "package").count();
    if package_count > 0 {
        let claim_ev = builder.push_evidence("derived", &root_str, "repo-scanner@0.1.0", None, None);
        builder.claims.push(GraphClaim {
            id: "claim:workspace-packages".into(),
            text: format!("Repository exposes {package_count} package manifest(s)."),
            confidence: Confidence::Derived,
            node_ids: builder
                .nodes
                .iter()
                .filter(|n| n.kind == "package")
                .map(|n| n.id.clone())
                .collect(),
            edge_ids: vec![],
            evidence_ids: vec![claim_ev],
        });
    }

    let _ = (files_scanned, files_indexed);

    ArchitectureGraph {
        schema_version: GRAPH_SCHEMA_VERSION.into(),
        repository: RepositorySnapshot {
            root: root_str,
            git_commit,
            worktree_dirty: dirty,
        },
        extractors: vec![
            "manifest@0.1.0".into(),
            "import-graph@0.1.0".into(),
            "docs@0.1.0".into(),
        ],
        nodes: builder.nodes,
        edges: builder.edges,
        claims: builder.claims,
        evidence: builder.evidence,
    }
}

fn should_include(path: &Path, root: &Path, options: &ScanOptions) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    for part in rel_str.split('/') {
        if options.exclude.iter().any(|e| e == part) {
            return false;
        }
    }
    if options.include.is_empty() {
        return true;
    }
    options.include.iter().any(|prefix| rel_str.starts_with(prefix))
}

fn index_manifest(builder: &mut GraphBuilder, rel: &str, path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let label = if rel.ends_with("package.json") {
        extract_json_name(&content).unwrap_or_else(|| "npm-package".into())
    } else {
        extract_toml_name(&content).unwrap_or_else(|| "cargo-package".into())
    };
    let ev = builder.push_evidence("manifest", rel, "manifest@0.1.0", None, None);
    let node_id = format!("node:package:{label}");
    builder.push_node(&node_id, "package", &label, Some(rel), &ev);
}

fn index_document(builder: &mut GraphBuilder, rel: &str, kind: &str) {
    let label = rel.rsplit('/').next().unwrap_or(rel).to_string();
    let ev = builder.push_evidence("documentation", rel, "docs@0.1.0", None, None);
    let node_id = format!("node:{kind}:{}", rel.replace('/', ":"));
    builder.push_node(&node_id, kind, &label, Some(rel), &ev);
    builder.push_edge("documents", "node:repository:root", &node_id, &ev);
}

fn index_ts_imports(builder: &mut GraphBuilder, rel: &str, path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let file_ev = builder.push_evidence("ast", rel, "import-graph@0.1.0", None, None);
    let file_id = format!("node:module:{}", rel.replace('/', ":"));
    builder.push_node(&file_id, "module", rel, Some(rel), &file_ev);

    let import_re = Regex::new(r#"(?m)^\s*import\s+.*?from\s+['"]([^'"]+)['"]"#).unwrap();
    let require_re = Regex::new(r#"require\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
    let mut deps = BTreeSet::new();
    for cap in import_re.captures_iter(&content) {
        deps.insert(cap[1].to_string());
    }
    for cap in require_re.captures_iter(&content) {
        deps.insert(cap[1].to_string());
    }
    for dep in deps {
        let dep_id = format!("node:module:dep:{dep}");
        if !builder.nodes.iter().any(|n| n.id == dep_id) {
            builder.push_node(&dep_id, "module", &dep, None, &file_ev);
        }
        builder.push_edge("imports", &file_id, &dep_id, &file_ev);
    }
    let _ = path;
}

fn index_rs_imports(builder: &mut GraphBuilder, rel: &str, path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let file_ev = builder.push_evidence("ast", rel, "import-graph@0.1.0", None, None);
    let file_id = format!("node:module:{}", rel.replace('/', ":"));
    builder.push_node(&file_id, "module", rel, Some(rel), &file_ev);

    let use_re = Regex::new(r#"(?m)^\s*use\s+([a-zA-Z0-9_:]+)"#).unwrap();
    let mod_re = Regex::new(r#"(?m)^\s*mod\s+([a-zA-Z0-9_]+)"#).unwrap();
    let mut deps = BTreeSet::new();
    for cap in use_re.captures_iter(&content) {
        deps.insert(cap[1].to_string());
    }
    for cap in mod_re.captures_iter(&content) {
        deps.insert(cap[1].to_string());
    }
    for dep in deps {
        let dep_id = format!("node:module:dep:{dep}");
        if !builder.nodes.iter().any(|n| n.id == dep_id) {
            builder.push_node(&dep_id, "module", &dep, None, &file_ev);
        }
        builder.push_edge("imports", &file_id, &dep_id, &file_ev);
    }
    let _ = path;
}

fn extract_json_name(content: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    value.get("name")?.as_str().map(str::to_string)
}

fn extract_toml_name(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name = ") {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

pub fn graph_digest(graph: &ArchitectureGraph) -> String {
    let json = serde_json::to_vec(graph).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json);
    format!("{:x}", hasher.finalize())
}

pub fn index_dir(root: &Path) -> PathBuf {
    root.join(".architecture-reader")
}

pub fn graph_path(root: &Path) -> PathBuf {
    index_dir(root).join("graph.json")
}