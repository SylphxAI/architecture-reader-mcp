use std::collections::{BTreeSet, HashMap, HashSet};
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
    pub use_synth: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            include: vec![],
            exclude: DEFAULT_EXCLUDES.iter().map(|s| (*s).to_string()).collect(),
            max_file_bytes: 1_048_576,
            use_synth: crate::synth_probe::probe_enabled_from_env(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct GraphBuilder {
    pub(crate) nodes: Vec<GraphNode>,
    pub(crate) edges: Vec<GraphEdge>,
    claims: Vec<GraphClaim>,
    pub(crate) evidence: Vec<EvidenceRef>,
    evidence_seq: u32,
}

impl GraphBuilder {
    pub(crate) fn has_node(&self, id: &str) -> bool {
        self.nodes.iter().any(|node| node.id == id)
    }

    pub(crate) fn push_evidence(&mut self, kind: &str, path: &str, extractor: &str, start: Option<u32>, end: Option<u32>) -> String {
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

    pub(crate) fn push_node(&mut self, id: &str, kind: &str, label: &str, path: Option<&str>, evidence_id: &str) {
        self.nodes.push(GraphNode {
            id: id.into(),
            kind: kind.into(),
            label: label.into(),
            path: path.map(str::to_string),
            evidence_ids: vec![evidence_id.into()],
        });
    }

    pub(crate) fn push_edge(&mut self, kind: &str, from: &str, to: &str, evidence_id: &str) {
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

pub fn hash_file_bytes(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(format!("{:x}", Sha256::digest(&bytes)))
}

pub fn inventory_files(root: &Path, options: &ScanOptions) -> HashMap<String, String> {
    let mut inventory = HashMap::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() || !should_include(path, root, options) {
            continue;
        }
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() > options.max_file_bytes {
                continue;
            }
        }
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if let Some(hash) = hash_file_bytes(path) {
            inventory.insert(rel_str, hash);
        }
    }
    inventory
}

pub fn scan_repository(root: &Path, options: &ScanOptions, git_commit: Option<String>, dirty: bool) -> ArchitectureGraph {
    scan_repository_paths(root, options, None, git_commit, dirty)
}

pub fn scan_repository_paths(
    root: &Path,
    options: &ScanOptions,
    only_paths: Option<&HashSet<String>>,
    git_commit: Option<String>,
    dirty: bool,
) -> ArchitectureGraph {
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
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if let Some(allowed) = only_paths {
            if !allowed.contains(&rel_str) {
                continue;
            }
        }
        files_scanned += 1;
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() > options.max_file_bytes {
                continue;
            }
        }

        index_file(&mut builder, root, path, &rel_str, options);
        files_indexed += 1;
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

    let mut extractors = vec![
        "manifest@0.1.0".into(),
        "import-graph@0.1.0".into(),
        "call-graph@0.1.0".into(),
        "docs@0.1.0".into(),
        "routes@0.1.0".into(),
        "schema@0.1.0".into(),
        "python@0.1.0".into(),
    ];
    if options.use_synth && builder.evidence.iter().any(|ev| ev.extractor.starts_with("synth-")) {
        extractors.push(crate::synth::SYNTH_JS_EXTRACTOR.into());
    }

    ArchitectureGraph {
        schema_version: GRAPH_SCHEMA_VERSION.into(),
        repository: RepositorySnapshot {
            root: root_str,
            git_commit,
            worktree_dirty: dirty,
        },
        extractors,
        nodes: builder.nodes,
        edges: builder.edges,
        claims: builder.claims,
        evidence: builder.evidence,
    }
}

fn index_file(builder: &mut GraphBuilder, _root: &Path, path: &Path, rel_str: &str, options: &ScanOptions) {
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if file_name == "package.json" || file_name == "Cargo.toml" {
        index_manifest(builder, rel_str, path);
        return;
    }

    if rel_str.starts_with("docs/") && rel_str.ends_with(".md") {
        index_document(
            builder,
            rel_str,
            if rel_str.contains("/adr/") { "adr" } else { "document" },
        );
        return;
    }

    if rel_str.ends_with(".json") && (rel_str.contains("/schemas/") || rel_str.ends_with(".schema.json")) {
        index_json_schema(builder, rel_str, path);
        return;
    }

    if rel_str.ends_with(".ts")
        || rel_str.ends_with(".tsx")
        || rel_str.ends_with(".js")
        || rel_str.ends_with(".mjs")
    {
        let content = fs::read_to_string(path).unwrap_or_default();
        let used_synth = index_ts_with_optional_synth(builder, rel_str, path, options.use_synth);
        if !used_synth {
            index_ts_imports(builder, rel_str, path);
        }
        index_ts_symbols(builder, rel_str, &content);
        index_ts_calls(builder, rel_str, &content);
        index_ts_routes(builder, rel_str, path);
        index_ts_schemas(builder, rel_str, path);
        return;
    }

    if rel_str.ends_with(".rs") {
        index_rs_imports(builder, rel_str, path);
    }

    if rel_str.ends_with(".py") {
        index_py_module(builder, rel_str, path);
    }
}

pub fn prune_graph_for_paths(mut graph: ArchitectureGraph, paths: &HashSet<String>) -> ArchitectureGraph {
    if paths.is_empty() {
        return graph;
    }

    let removed_node_ids: HashSet<String> = graph
        .nodes
        .iter()
        .filter(|node| node.path.as_ref().is_some_and(|p| paths.contains(p)))
        .map(|node| node.id.clone())
        .collect();

    graph.nodes.retain(|node| {
        node.path.as_ref().is_none_or(|path| !paths.contains(path)) && !removed_node_ids.contains(&node.id)
    });

    graph.evidence.retain(|evidence| !paths.contains(&evidence.path));

    let live_nodes: HashSet<String> = graph.nodes.iter().map(|node| node.id.clone()).collect();
    graph.edges.retain(|edge| live_nodes.contains(&edge.from) && live_nodes.contains(&edge.to));

    let live_evidence: HashSet<String> = graph.evidence.iter().map(|evidence| evidence.id.clone()).collect();
    graph.claims.retain(|claim| {
        claim.evidence_ids.iter().all(|id| live_evidence.contains(id))
            && claim.node_ids.iter().all(|id| live_nodes.contains(id))
            && claim.edge_ids.iter().all(|edge_id| graph.edges.iter().any(|edge| edge.id == *edge_id))
    });

    graph
}

pub fn merge_graphs(mut base: ArchitectureGraph, delta: ArchitectureGraph) -> ArchitectureGraph {
    let mut seen_nodes: HashSet<String> = base.nodes.iter().map(|node| node.id.clone()).collect();
    for node in delta.nodes {
        if seen_nodes.insert(node.id.clone()) {
            base.nodes.push(node);
        }
    }

    let mut seen_edges: HashSet<String> = base.edges.iter().map(|edge| edge.id.clone()).collect();
    for edge in delta.edges {
        if seen_edges.insert(edge.id.clone()) {
            base.edges.push(edge);
        }
    }

    let mut seen_claims: HashSet<String> = base.claims.iter().map(|claim| claim.id.clone()).collect();
    for claim in delta.claims {
        if seen_claims.insert(claim.id.clone()) {
            base.claims.push(claim);
        }
    }

    let mut seen_evidence: HashSet<String> = base.evidence.iter().map(|evidence| evidence.id.clone()).collect();
    for evidence in delta.evidence {
        if seen_evidence.insert(evidence.id.clone()) {
            base.evidence.push(evidence);
        }
    }

    for extractor in delta.extractors {
        if !base.extractors.contains(&extractor) {
            base.extractors.push(extractor);
        }
    }

    base.repository = delta.repository;
    base.schema_version = delta.schema_version;
    base
}

pub fn incremental_refresh(
    root: &Path,
    options: &ScanOptions,
    existing: ArchitectureGraph,
    changed_paths: &HashSet<String>,
    deleted_paths: &HashSet<String>,
    git_commit: Option<String>,
    dirty: bool,
) -> ArchitectureGraph {
    let mut affected = changed_paths.clone();
    affected.extend(deleted_paths.iter().cloned());

    let pruned = prune_graph_for_paths(existing, &affected);
    let delta = scan_repository_paths(root, options, Some(changed_paths), git_commit, dirty);
    merge_graphs(pruned, delta)
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

fn index_ts_with_optional_synth(
    builder: &mut GraphBuilder,
    rel: &str,
    path: &Path,
    use_synth: bool,
) -> bool {
    if !use_synth {
        return false;
    }

    let script = crate::synth_probe::default_probe_script();
    match crate::synth_probe::probe_synth_tree(path, &script) {
        Ok(tree) => {
            crate::synth::apply_to_builder(builder, rel, &tree);
            true
        }
        Err(_) => false,
    }
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

fn index_ts_routes(builder: &mut GraphBuilder, rel: &str, path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let route_re =
        Regex::new(r#"(?m)(?:app|router)\s*\.\s*(get|post|put|delete|patch|head|options|all)\s*\(\s*['"]([^'"]+)['"]"#)
            .unwrap();
    let file_id = format!("node:module:{}", rel.replace('/', ":"));
    if !builder.nodes.iter().any(|n| n.id == file_id) {
        let file_ev = builder.push_evidence("ast", rel, "routes@0.1.0", None, None);
        builder.push_node(&file_id, "module", rel, Some(rel), &file_ev);
    }

    for cap in route_re.captures_iter(&content) {
        let method = cap[1].to_uppercase();
        let route_path = &cap[2];
        let line = line_number_at(&content, cap.get(0).map(|m| m.start()).unwrap_or(0));
        let label = format!("{method} {route_path}");
        let node_id = format!(
            "node:route:{}:{}:{}",
            rel.replace('/', ":"),
            method.to_lowercase(),
            route_path.replace('/', "_")
        );
        if builder.nodes.iter().any(|n| n.id == node_id) {
            continue;
        }
        let ev = builder.push_evidence("ast", rel, "routes@0.1.0", Some(line), Some(line));
        builder.push_node(&node_id, "route", &label, Some(rel), &ev);
        builder.push_edge("defines", &file_id, &node_id, &ev);
    }
}

fn index_ts_schemas(builder: &mut GraphBuilder, rel: &str, path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let zod_const_re =
        Regex::new(r#"(?m)^\s*(?:export\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*Schema)\s*=\s*z\."#).unwrap();
    let zod_type_re =
        Regex::new(r#"(?m)^\s*export\s+type\s+([A-Za-z_][A-Za-z0-9_]*Schema)\s*="#).unwrap();
    let file_id = format!("node:module:{}", rel.replace('/', ":"));
    if !builder.nodes.iter().any(|n| n.id == file_id) {
        let file_ev = builder.push_evidence("ast", rel, "schema@0.1.0", None, None);
        builder.push_node(&file_id, "module", rel, Some(rel), &file_ev);
    }

    let mut names = BTreeSet::new();
    for cap in zod_const_re.captures_iter(&content) {
        names.insert(cap[1].to_string());
    }
    for cap in zod_type_re.captures_iter(&content) {
        names.insert(cap[1].to_string());
    }
    for name in names {
        let line = content
            .lines()
            .position(|line| line.contains(&name))
            .map(|i| (i + 1) as u32);
        let node_id = format!("node:schema:ts:{}:{}", rel.replace('/', ":"), name);
        if builder.nodes.iter().any(|n| n.id == node_id) {
            continue;
        }
        let ev = builder.push_evidence("ast", rel, "schema@0.1.0", line, line);
        builder.push_node(&node_id, "schema", &name, Some(rel), &ev);
        builder.push_edge("defines", &file_id, &node_id, &ev);
    }
}

fn index_json_schema(builder: &mut GraphBuilder, rel: &str, path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let label = extract_json_schema_label(&content).unwrap_or_else(|| {
        rel.rsplit('/').next().unwrap_or(rel).trim_end_matches(".json").to_string()
    });
    let line = content
        .lines()
        .position(|line| line.contains("\"title\"") || line.contains("\"$id\""))
        .map(|i| (i + 1) as u32);
    let ev = builder.push_evidence("schema", rel, "schema@0.1.0", line, line);
    let node_id = format!("node:schema:json:{}", rel.replace('/', ":"));
    builder.push_node(&node_id, "schema", &label, Some(rel), &ev);
    builder.push_edge("documents", "node:repository:root", &node_id, &ev);
    let _ = path;
}

fn extract_json_schema_label(content: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    if let Some(title) = value.get("title").and_then(|v| v.as_str()) {
        return Some(title.to_string());
    }
    if let Some(id) = value.get("$id").and_then(|v| v.as_str()) {
        return Some(id.to_string());
    }
    None
}

fn line_number_at(content: &str, byte_offset: usize) -> u32 {
    let prefix = &content[..byte_offset.min(content.len())];
    prefix.lines().count().max(1) as u32
}

fn index_ts_symbols(builder: &mut GraphBuilder, rel: &str, content: &str) {
    let file_id = format!("node:module:{}", rel.replace('/', ":"));
    if !builder.has_node(&file_id) {
        let file_ev = builder.push_evidence("ast", rel, "call-graph@0.1.0", None, None);
        builder.push_node(&file_id, "module", rel, Some(rel), &file_ev);
    }

    let patterns = [
        (
            Regex::new(r#"(?m)^export\s+(?:async\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)"#).unwrap(),
            "function",
        ),
        (
            Regex::new(r#"(?m)^export\s+class\s+([A-Za-z_][A-Za-z0-9_]*)"#).unwrap(),
            "class",
        ),
        (
            Regex::new(r#"(?m)^function\s+([A-Za-z_][A-Za-z0-9_]*)"#).unwrap(),
            "function",
        ),
    ];

    for (pattern, kind) in patterns {
        for cap in pattern.captures_iter(content) {
            let name = cap[1].to_string();
            let line = line_number_at(content, cap.get(0).map(|m| m.start()).unwrap_or(0));
            let node_id = format!("node:symbol:{}:{}:{}", rel.replace('/', ":"), kind, name);
            if builder.has_node(&node_id) {
                continue;
            }
            let ev = builder.push_evidence("ast", rel, "call-graph@0.1.0", Some(line), Some(line));
            builder.push_node(&node_id, "symbol", &name, Some(rel), &ev);
            builder.push_edge("defines", &file_id, &node_id, &ev);
        }
    }
}

fn index_ts_calls(builder: &mut GraphBuilder, rel: &str, content: &str) {
    let import_map = ts_import_local_map(content);
    let symbols = ts_symbol_spans(content);
    if symbols.is_empty() {
        return;
    }

    let call_re = Regex::new(r#"(?m)\b([A-Za-z_][A-Za-z0-9_]*)\s*\("#).unwrap();
    for (caller, start_line, end_line) in symbols {
        let caller_id = format!("node:symbol:{}:function:{}", rel.replace('/', ":"), caller);
        if !builder.has_node(&caller_id) {
            continue;
        }
        for (line_no, line) in content.lines().enumerate() {
            let line_number = (line_no + 1) as u32;
            if line_number < start_line || line_number > end_line {
                continue;
            }
            for cap in call_re.captures_iter(line) {
                let callee = cap[1].to_string();
                if callee == caller || matches!(callee.as_str(), "if" | "for" | "while" | "switch" | "return") {
                    continue;
                }
                if let Some(target_id) = resolve_ts_call_target(builder, rel, &import_map, &callee) {
                    let ev = builder.push_evidence(
                        "ast",
                        rel,
                        "call-graph@0.1.0",
                        Some(line_number),
                        Some(line_number),
                    );
                    builder.push_edge("calls", &caller_id, &target_id, &ev);
                }
            }
        }
    }
}

fn ts_import_local_map(content: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let import_re = Regex::new(
        r#"(?m)^\s*import\s+(?:\{([^}]+)\}|([A-Za-z_][A-Za-z0-9_]*))\s+from\s+['"]([^'"]+)['"]"#,
    )
    .unwrap();
    let from_import_re =
        Regex::new(r#"(?m)^\s*from\s+([A-Za-z0-9_.]+)\s+import\s+([A-Za-z0-9_,\s]+)"#).unwrap();

    for cap in import_re.captures_iter(content) {
        let source = cap[3].to_string();
        if let Some(named) = cap.get(1) {
            for part in named.as_str().split(',') {
                let token = part.trim();
                let local = token
                    .split(" as ")
                    .next()
                    .unwrap_or(token)
                    .trim()
                    .to_string();
                if !local.is_empty() {
                    out.insert(local, source.clone());
                }
            }
        } else if let Some(default_import) = cap.get(2) {
            out.insert(default_import.as_str().to_string(), source);
        }
    }

    for cap in from_import_re.captures_iter(content) {
        let source = cap[1].to_string();
        for part in cap[2].split(',') {
            let token = part.trim();
            let local = token.split(" as ").next().unwrap_or(token).trim().to_string();
            if !local.is_empty() {
                out.insert(local, source.clone());
            }
        }
    }
    out
}

fn ts_symbol_spans(content: &str) -> Vec<(String, u32, u32)> {
    let symbol_re = Regex::new(
        r#"(?m)^(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)"#,
    )
    .unwrap();
    let mut starts = Vec::new();
    for cap in symbol_re.captures_iter(content) {
        let name = cap[1].to_string();
        let start = line_number_at(content, cap.get(0).map(|m| m.start()).unwrap_or(0));
        starts.push((name, start));
    }

    let total_lines = content.lines().count().max(1) as u32;
    starts
        .iter()
        .enumerate()
        .map(|(index, (name, start))| {
            let end = starts
                .get(index + 1)
                .map(|(_, next_start)| next_start.saturating_sub(1))
                .unwrap_or(total_lines);
            (name.clone(), *start, end)
        })
        .collect()
}

fn resolve_ts_call_target(
    builder: &GraphBuilder,
    rel: &str,
    import_map: &HashMap<String, String>,
    callee: &str,
) -> Option<String> {
    if let Some(local_target) = builder
        .nodes
        .iter()
        .find(|node| node.kind == "symbol" && node.path.as_deref() == Some(rel) && node.label == callee)
        .map(|node| node.id.clone())
    {
        return Some(local_target);
    }

    let import_source = import_map.get(callee)?;
    if import_source.starts_with('.') {
        let resolved_path = normalize_relative_module_path(rel, import_source)?;
        return builder
            .nodes
            .iter()
            .find(|node| {
                node.kind == "symbol"
                    && node.label == callee
                    && node.path.as_deref() == Some(resolved_path.as_str())
            })
            .map(|node| node.id.clone())
            .or_else(|| {
                let dep_id = format!("node:module:dep:{import_source}");
                if builder.has_node(&dep_id) {
                    Some(dep_id)
                } else {
                    None
                }
            });
    }

    let dep_id = format!("node:module:dep:{import_source}");
    if builder.has_node(&dep_id) {
        Some(dep_id)
    } else {
        None
    }
}

fn normalize_relative_module_path(from_rel: &str, import_source: &str) -> Option<String> {
    if !import_source.starts_with('.') {
        return None;
    }
    let from = Path::new(from_rel);
    let parent = from.parent()?;
    let joined = parent.join(import_source);
    let mut normalized = normalize_posix_path(&joined.to_string_lossy().replace('\\', "/"));
    if normalized.ends_with(".js") || normalized.ends_with(".mjs") {
        let stem = normalized[..normalized.rfind('.')?].to_string();
        normalized = stem;
    }
    if !normalized.ends_with(".ts") && !normalized.ends_with(".tsx") {
        normalized.push_str(".ts");
    }
    Some(normalized)
}

fn normalize_posix_path(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            stack.pop();
        } else {
            stack.push(part);
        }
    }
    stack.join("/")
}

fn index_py_module(builder: &mut GraphBuilder, rel: &str, path: &Path) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let file_ev = builder.push_evidence("ast", rel, "python@0.1.0", None, None);
    let file_id = format!("node:module:{}", rel.replace('/', ":"));
    builder.push_node(&file_id, "module", rel, Some(rel), &file_ev);

    let import_re = Regex::new(r#"(?m)^\s*import\s+([A-Za-z0-9_.]+)"#).unwrap();
    let from_import_re =
        Regex::new(r#"(?m)^\s*from\s+([A-Za-z0-9_.]+)\s+import\s+([A-Za-z0-9_,\s]+)"#).unwrap();
    let mut import_map = HashMap::new();

    for cap in import_re.captures_iter(&content) {
        let module = cap[1].to_string();
        let dep_id = format!("node:module:dep:{module}");
        if !builder.has_node(&dep_id) {
            builder.push_node(&dep_id, "module", &module, None, &file_ev);
        }
        builder.push_edge("imports", &file_id, &dep_id, &file_ev);
        import_map.insert(module.clone(), module);
    }

    for cap in from_import_re.captures_iter(&content) {
        let module = cap[1].to_string();
        let dep_id = format!("node:module:dep:{module}");
        if !builder.has_node(&dep_id) {
            builder.push_node(&dep_id, "module", &module, None, &file_ev);
        }
        builder.push_edge("imports", &file_id, &dep_id, &file_ev);
        for part in cap[2].split(',') {
            let token = part.trim();
            let local = token.split(" as ").next().unwrap_or(token).trim().to_string();
            if !local.is_empty() {
                import_map.insert(local, module.clone());
            }
        }
    }

    let def_re = Regex::new(r#"(?m)^def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\("#).unwrap();
    let call_re = Regex::new(r#"(?m)\b([A-Za-z_][A-Za-z0-9_]*)\s*\("#).unwrap();
    let mut symbols = Vec::new();

    for cap in def_re.captures_iter(&content) {
        let name = cap[1].to_string();
        let line = line_number_at(&content, cap.get(0).map(|m| m.start()).unwrap_or(0));
        let node_id = format!("node:symbol:{}:function:{}", rel.replace('/', ":"), name);
        let ev = builder.push_evidence("ast", rel, "python@0.1.0", Some(line), Some(line));
        builder.push_node(&node_id, "symbol", &name, Some(rel), &ev);
        builder.push_edge("defines", &file_id, &node_id, &ev);
        symbols.push((name, line, line + 64));
    }

    for (caller, start_line, end_line) in symbols {
        let caller_id = format!("node:symbol:{}:function:{}", rel.replace('/', ":"), caller);
        for (line_no, line) in content.lines().enumerate() {
            let line_number = (line_no + 1) as u32;
            if line_number < start_line || line_number > end_line {
                continue;
            }
            for cap in call_re.captures_iter(line) {
                let callee = cap[1].to_string();
                if callee == caller || callee == "def" {
                    continue;
                }
                let target_id = if let Some(local) = builder
                    .nodes
                    .iter()
                    .find(|node| {
                        node.kind == "symbol"
                            && node.path.as_deref() == Some(rel)
                            && node.label == callee
                    })
                    .map(|node| node.id.clone())
                {
                    local
                } else if let Some(module) = import_map.get(&callee) {
                    format!("node:module:dep:{module}")
                } else {
                    continue;
                };
                let ev = builder.push_evidence(
                    "ast",
                    rel,
                    "python@0.1.0",
                    Some(line_number),
                    Some(line_number),
                );
                builder.push_edge("calls", &caller_id, &target_id, &ev);
            }
        }
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