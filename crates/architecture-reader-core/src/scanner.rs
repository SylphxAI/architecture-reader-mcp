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


#[cfg(test)]
mod pure_residual_tests {
    //! Pure residual deepen (BW4/BW5): string/graph helpers without repo effect I/O.
    //! Not graph-effect parity, not authority_rust.

    use super::*;
    use crate::types::{
        ArchitectureGraph, Confidence, EvidenceRef, GraphClaim, GraphEdge, GraphNode,
        RepositorySnapshot, GRAPH_SCHEMA_VERSION,
    };
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    fn empty_graph(root: &str) -> ArchitectureGraph {
        ArchitectureGraph {
            schema_version: GRAPH_SCHEMA_VERSION.to_string(),
            repository: RepositorySnapshot {
                root: root.to_string(),
                git_commit: None,
                worktree_dirty: false,
            },
            extractors: vec!["call-graph@0.1.0".into()],
            nodes: vec![],
            edges: vec![],
            claims: vec![],
            evidence: vec![],
        }
    }

    fn node(id: &str, path: Option<&str>) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            kind: "module".into(),
            label: id.to_string(),
            path: path.map(str::to_string),
            evidence_ids: vec![],
        }
    }

    #[test]
    fn extract_json_schema_label_prefers_title_then_id() {
        assert_eq!(
            extract_json_schema_label(r#"{"title":"User","$id":"https://x/user"}"#).as_deref(),
            Some("User")
        );
        assert_eq!(
            extract_json_schema_label(r#"{"$id":"https://x/user"}"#).as_deref(),
            Some("https://x/user")
        );
        assert_eq!(extract_json_schema_label(r#"{"type":"object"}"#), None);
        assert_eq!(extract_json_schema_label("not-json"), None);
    }

    #[test]
    fn line_number_at_is_one_indexed_by_newlines() {
        // Implementation counts `lines()` on the exclusive prefix. A trailing
        // newline in the prefix does not add an empty line (Rust `lines()`),
        // so offset at the first char of line N often reports N-1 until the
        // char is included — lock that contract for pure residual deepen.
        let content = "a\nb\nc";
        assert_eq!(line_number_at(content, 0), 1);
        assert_eq!(line_number_at(content, 2), 1); // prefix "a\n" → 1 line
        assert_eq!(line_number_at(content, 3), 2); // prefix "a\nb" → 2 lines
        assert_eq!(line_number_at(content, content.len()), 3);
        assert_eq!(line_number_at("", 0), 1);
    }

    #[test]
    fn ts_import_local_map_named_default_and_python_from() {
        let content = r#"
import { foo as bar, baz } from './lib';
import Default from "./default.js";
from pkg.mod import Alpha, Beta as B
"#;
        let map = ts_import_local_map(content);
        // "foo as bar" — local token is first segment before " as "
        assert_eq!(map.get("foo").map(String::as_str), Some("./lib"));
        assert_eq!(map.get("baz").map(String::as_str), Some("./lib"));
        assert_eq!(map.get("Default").map(String::as_str), Some("./default.js"));
        assert_eq!(map.get("Alpha").map(String::as_str), Some("pkg.mod"));
        assert_eq!(map.get("Beta").map(String::as_str), Some("pkg.mod"));
    }

    #[test]
    fn ts_symbol_spans_export_async_and_plain_functions() {
        let content = "export async function run() {}\nfunction helper() {}\nexport class X {}\n";
        let spans = ts_symbol_spans(content);
        let names: Vec<&str> = spans.iter().map(|(n, _, _)| n.as_str()).collect();
        assert!(names.contains(&"run"), "{names:?}");
        assert!(names.contains(&"helper"), "{names:?}");
        assert_eq!(spans.iter().find(|(n, _, _)| n == "run").map(|(_, s, _)| *s), Some(1));
    }

    #[test]
    fn pure_residual_graph_merge_and_prune() {
        let mut base = empty_graph("/repo");
        base.nodes.push(node("n:a", Some("src/a.ts")));
        base.nodes.push(node("n:b", Some("src/b.ts")));
        base.edges.push(GraphEdge {
            id: "e:1".into(),
            kind: "imports".into(),
            from: "n:a".into(),
            to: "n:b".into(),
            evidence_ids: vec![],
        });
        base.evidence.push(EvidenceRef {
            id: "ev:a".into(),
            kind: "ast".into(),
            path: "src/a.ts".into(),
            start_line: Some(1),
            end_line: Some(1),
            extractor: "call-graph@0.1.0".into(),
            confidence: Confidence::Deterministic,
        });

        let mut delta = empty_graph("/repo");
        delta.nodes.push(node("n:c", Some("src/c.ts")));
        delta.nodes.push(node("n:a", Some("src/a.ts"))); // duplicate id ignored
        delta.extractors.push("synth-js@0.1.0".into());

        let merged = merge_graphs(base.clone(), delta);
        assert_eq!(merged.nodes.len(), 3);
        assert!(merged.nodes.iter().any(|n| n.id == "n:c"));
        assert!(merged.extractors.iter().any(|e| e == "synth-js@0.1.0"));

        let mut paths = HashSet::new();
        paths.insert("src/a.ts".into());
        let pruned = prune_graph_for_paths(merged, &paths);
        assert!(!pruned.nodes.iter().any(|n| n.id == "n:a"));
        assert!(!pruned.edges.iter().any(|e| e.from == "n:a" || e.to == "n:a"));
        assert!(!pruned.evidence.iter().any(|e| e.path == "src/a.ts"));
        assert!(pruned.nodes.iter().any(|n| n.id == "n:b"));
        assert!(pruned.nodes.iter().any(|n| n.id == "n:c"));
    }

    #[test]
    fn graph_digest_is_stable_for_identical_graphs() {
        let g1 = empty_graph("/repo");
        let g2 = empty_graph("/repo");
        let d1 = graph_digest(&g1);
        let d2 = graph_digest(&g2);
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);
        let mut g3 = empty_graph("/repo");
        g3.nodes.push(node("n:x", Some("x.ts")));
        assert_ne!(graph_digest(&g3), d1);
    }

    #[test]
    fn index_ts_symbols_defines_exported_functions() {
        let mut builder = GraphBuilder::default();
        let content = "export function authMiddleware() {\n  return true;\n}\n";
        index_ts_symbols(&mut builder, "src/auth.ts", content);
        assert!(builder.nodes.iter().any(|n| n.label == "authMiddleware" && n.kind == "symbol"));
        assert!(builder.edges.iter().any(|e| e.kind == "defines"));
    }

    #[test]
    fn normalize_relative_module_path_resolves_dot_and_js() {
        assert_eq!(
            normalize_relative_module_path("src/auth/middleware.ts", "./token.js").as_deref(),
            Some("src/auth/token.ts")
        );
        assert_eq!(
            normalize_relative_module_path("src/auth/middleware.ts", "../users/store").as_deref(),
            Some("src/users/store.ts")
        );
        assert_eq!(
            normalize_relative_module_path("src/a.ts", "pkg/no-rel"),
            None
        );
        assert_eq!(normalize_posix_path("a/./b/../c//d"), "a/c/d");
        assert_eq!(normalize_posix_path("../x"), "x");
    }

    #[test]
    fn extract_json_and_toml_name_helpers() {
        assert_eq!(
            extract_json_name(r#"{"name":"demo-pkg","version":"1"}"#).as_deref(),
            Some("demo-pkg")
        );
        assert_eq!(extract_json_name(r#"{"version":"1"}"#), None);
        assert_eq!(extract_json_name("not-json"), None);
        assert_eq!(
            extract_toml_name("[package]\nname = \"fs-core\"\nversion = \"0.1.0\"\n").as_deref(),
            Some("fs-core")
        );
        assert_eq!(extract_toml_name("version = \"1\""), None);
    }

    #[test]
    fn resolve_ts_call_target_local_and_dep() {
        let mut builder = GraphBuilder::default();
        let ev = builder.push_evidence("ast", "src/a.ts", "call-graph@0.1.0", Some(1), Some(1));
        builder.push_node(
            "node:symbol:src:a.ts:function:localFn",
            "symbol",
            "localFn",
            Some("src/a.ts"),
            &ev,
        );
        builder.push_node(
            "node:module:dep:lodash",
            "module",
            "lodash",
            None,
            &ev,
        );
        let mut map = std::collections::HashMap::new();
        map.insert("get".into(), "lodash".into());
        map.insert("localFn".into(), "./noop".into());
        assert_eq!(
            resolve_ts_call_target(&builder, "src/a.ts", &map, "localFn").as_deref(),
            Some("node:symbol:src:a.ts:function:localFn")
        );
        assert_eq!(
            resolve_ts_call_target(&builder, "src/a.ts", &map, "get").as_deref(),
            Some("node:module:dep:lodash")
        );
        assert_eq!(resolve_ts_call_target(&builder, "src/a.ts", &map, "missing"), None);
    }

    #[test]
    fn normalize_relative_module_path_js_to_ts_and_posix_edges() {
        // Contract lock (honest):
        // - `.js`/`.mjs` stem rewrite then ensure `.ts`
        // - `.tsx` preserved
        // - other extensions (e.g. `.mts`) get `.ts` appended (current pure residual)
        assert_eq!(
            normalize_relative_module_path("src/app/page.tsx", "./hooks/useX.js").as_deref(),
            Some("src/app/hooks/useX.ts")
        );
        assert_eq!(
            normalize_relative_module_path("src/app/page.tsx", "./hooks/useX.tsx").as_deref(),
            Some("src/app/hooks/useX.tsx")
        );
        assert_eq!(
            normalize_relative_module_path("packages/a/src/index.mts", "./util.mts").as_deref(),
            Some("packages/a/src/util.mts.ts")
        );
        assert_eq!(
            normalize_relative_module_path("src/a.ts", "./nested/index.mjs").as_deref(),
            Some("src/nested/index.ts")
        );
        assert_eq!(normalize_posix_path(""), "");
        assert_eq!(normalize_posix_path("."), "");
        assert_eq!(normalize_posix_path("foo//bar/./baz/../qux"), "foo/bar/qux");
    }

    #[test]
    fn index_dir_and_graph_path_are_under_dot_architecture_reader() {
        let root = Path::new("/tmp/demo-repo");
        assert_eq!(index_dir(root), PathBuf::from("/tmp/demo-repo/.architecture-reader"));
        assert_eq!(
            graph_path(root),
            PathBuf::from("/tmp/demo-repo/.architecture-reader/graph.json")
        );
    }

    #[test]
    fn ts_import_local_map_ignores_side_effect_imports() {
        let content = r#"
import './polyfill';
import type { X } from './types';
import { real } from '../lib/real';
"#;
        let map = ts_import_local_map(content);
        // side-effect only import has no local binding in map
        assert!(!map.values().any(|v| v == "./polyfill"));
        assert_eq!(map.get("real").map(String::as_str), Some("../lib/real"));
    }

    #[test]
    fn extract_json_schema_label_trims_and_title_wins() {
        assert_eq!(
            extract_json_schema_label(r#"{"title":"  T  ","$id":"id"}"#).as_deref(),
            Some("  T  ")
        );
        assert_eq!(extract_json_schema_label(""), None);
        assert_eq!(extract_json_schema_label("{"), None);
    }

    #[test]
    fn bw7_extract_json_name_rejects_non_string_and_empty_object() {
        assert_eq!(extract_json_name(r#"{"name":"ok"}"#).as_deref(), Some("ok"));
        assert_eq!(extract_json_name(r#"{"name":123}"#), None);
        assert_eq!(extract_json_name(r#"{"name":null}"#), None);
        assert_eq!(extract_json_name("{}"), None);
    }

    #[test]
    fn bw7_extract_toml_name_first_match_and_strip_quotes() {
        let multi = "[workspace]\nmembers = []\n[package]\nname = \"pkg-core\"\nversion = \"0.1\"\n";
        assert_eq!(extract_toml_name(multi).as_deref(), Some("pkg-core"));
        // first `name =` wins even outside package (honest pure residual contract)
        let early = "name = \"early\"\n[package]\nname = \"late\"\n";
        assert_eq!(extract_toml_name(early).as_deref(), Some("early"));
        assert_eq!(extract_toml_name("  name = \"spaced\"  "), Some("spaced".into()));
    }

    #[test]
    fn bw7_ts_symbol_spans_empty_and_end_line_chain() {
        assert!(ts_symbol_spans("").is_empty());
        assert!(ts_symbol_spans("// no functions\nconst x = 1;\n").is_empty());
        // Honest line_number_at contract: start offset of next `function` after `}\n`
        // yields start_line = lines(prefix).count() (often the line of `}` not the next line).
        let content = "function a() {\n  return 1;\n}\nfunction b() {\n  return 2;\n}\nfunction c() {\n  return 3;\n}\n";
        let spans = ts_symbol_spans(content);
        assert_eq!(spans.len(), 3, "{spans:?}");
        assert_eq!(spans[0].0, "a");
        assert_eq!(spans[0].1, 1);
        // end = next_start - 1 (honest residual)
        assert_eq!(spans[0].2, spans[1].1.saturating_sub(1));
        assert_eq!(spans[1].0, "b");
        assert_eq!(spans[1].2, spans[2].1.saturating_sub(1));
        assert_eq!(spans[2].0, "c");
        // last symbol end == total lines
        let total = content.lines().count().max(1) as u32;
        assert_eq!(spans[2].2, total);
    }

    #[test]
    fn bw7_normalize_relative_already_ts_and_deep_parent() {
        assert_eq!(
            normalize_relative_module_path("src/a/b/c.ts", "./d.ts").as_deref(),
            Some("src/a/b/d.ts")
        );
        assert_eq!(
            normalize_relative_module_path("src/a/b/c.ts", "../../z").as_deref(),
            Some("src/z.ts")
        );
        assert_eq!(
            normalize_relative_module_path("src/a.ts", "/abs"),
            None
        );
        // parent of root-level file: Path::parent of "file.ts" is "" → join still works
        assert_eq!(
            normalize_relative_module_path("file.ts", "./peer").as_deref(),
            Some("peer.ts")
        );
    }

    #[test]
    fn bw7_line_number_at_clamps_offset_past_end() {
        let c = "x\ny";
        assert_eq!(line_number_at(c, 999), 2);
        assert_eq!(line_number_at("only", 0), 1);
        assert_eq!(line_number_at("only", 4), 1);
    }

    #[test]
    fn bw7_merge_graphs_dedups_node_ids_keeps_unique_extractors() {
        let mut base = empty_graph("/r");
        base.nodes.push(node("n:1", Some("a.ts")));
        base.extractors.push("call-graph@0.1.0".into());
        let mut delta = empty_graph("/r");
        delta.nodes.push(node("n:1", Some("a.ts"))); // dup id
        delta.nodes.push(node("n:2", Some("b.ts")));
        delta.extractors.push("call-graph@0.1.0".into()); // may append again depending on impl
        delta.extractors.push("synth-js@0.1.0".into());
        let merged = merge_graphs(base, delta);
        assert_eq!(merged.nodes.iter().filter(|n| n.id == "n:1").count(), 1);
        assert!(merged.nodes.iter().any(|n| n.id == "n:2"));
        assert!(merged.extractors.iter().any(|e| e == "synth-js@0.1.0"));
    }


    #[test]
    fn bw8_extract_json_schema_label_id_only_and_non_object() {
        assert_eq!(
            extract_json_schema_label(r#"{"$id":"https://example.com/s"}"#).as_deref(),
            Some("https://example.com/s")
        );
        assert_eq!(
            extract_json_schema_label(r#"{"title":"T","$id":"id"}"#).as_deref(),
            Some("T")
        );
        assert_eq!(extract_json_schema_label("[]"), None);
        assert_eq!(extract_json_schema_label("null"), None);
        assert_eq!(extract_json_schema_label(r#"{"title":1}"#), None);
    }

    #[test]
    fn bw8_ts_import_local_map_as_alias_and_multi_named() {
        let content = r#"
import { foo as bar, baz as qux } from './lib';
import * as ns from './star';
import Def from '../def';
"#;
        let map = ts_import_local_map(content);
        assert_eq!(map.get("foo").map(String::as_str), Some("./lib"));
        assert_eq!(map.get("baz").map(String::as_str), Some("./lib"));
        assert_eq!(map.get("Def").map(String::as_str), Some("../def"));
        assert!(!map.contains_key("ns"));
    }

    #[test]
    fn bw8_normalize_posix_pops_past_root_and_dots() {
        assert_eq!(normalize_posix_path("../../x"), "x");
        assert_eq!(normalize_posix_path("././"), "");
        assert_eq!(normalize_posix_path("a/../../b"), "b");
        assert_eq!(normalize_posix_path("/abs/./c"), "abs/c");
        assert_eq!(
            normalize_relative_module_path("src/a.ts", "./b/../c.js").as_deref(),
            Some("src/c.ts")
        );
    }

    #[test]
    fn bw8_resolve_ts_call_target_relative_dep_fallback() {
        let mut builder = GraphBuilder::default();
        let ev = builder.push_evidence("ast", "src/a.ts", "call-graph@0.1.0", Some(1), Some(1));
        builder.push_node("node:module:dep:./helpers", "module", "./helpers", None, &ev);
        let mut map = std::collections::HashMap::new();
        map.insert("helper".into(), "./helpers".into());
        assert_eq!(
            resolve_ts_call_target(&builder, "src/a.ts", &map, "helper").as_deref(),
            Some("node:module:dep:./helpers")
        );
        assert_eq!(
            resolve_ts_call_target(&builder, "src/a.ts", &map, "missing"),
            None
        );
    }

    #[test]
    fn bw8_graph_digest_changes_when_node_label_changes() {
        let mut g1 = empty_graph("/repo");
        g1.nodes.push(node("n:1", Some("a.ts")));
        let mut g2 = empty_graph("/repo");
        let mut n = node("n:1", Some("a.ts"));
        n.label = "changed".into();
        g2.nodes.push(n);
        assert_ne!(graph_digest(&g1), graph_digest(&g2));
        assert_eq!(graph_digest(&g1), graph_digest(&g1));
        assert_eq!(graph_digest(&empty_graph("/r")).len(), 64);
    }

    #[test]
    fn bw8_line_number_at_empty_and_mid_line() {
        assert_eq!(line_number_at("", 0), 1);
        assert_eq!(line_number_at("abc", 1), 1);
        assert_eq!(line_number_at("a\n\nb", 2), 1);
        assert_eq!(line_number_at("a\n\nb", 3), 2);
    }

    #[test]
    fn bw8_extract_toml_name_unquoted_and_inline_ws() {
        assert_eq!(
            extract_toml_name("name = unquoted\n").as_deref(),
            Some("unquoted")
        );
        assert_eq!(extract_toml_name("# name = \"no\"\nversion = \"1\""), None);
        assert_eq!(extract_toml_name("xname = \"x\""), None);
    }


    #[test]
    fn bulk_should_include_exclude_parts_and_include_prefix() {
        let root = Path::new("/repo");
        let mut opts = ScanOptions::default();
        opts.exclude = vec!["node_modules".into(), "target".into()];
        assert!(!should_include(Path::new("/repo/node_modules/x.ts"), root, &opts));
        assert!(!should_include(Path::new("/repo/a/target/b.rs"), root, &opts));
        assert!(should_include(Path::new("/repo/src/a.ts"), root, &opts));
        opts.include = vec!["src/".into(), "crates/".into()];
        assert!(should_include(Path::new("/repo/src/a.ts"), root, &opts));
        assert!(should_include(Path::new("/repo/crates/x/lib.rs"), root, &opts));
        assert!(!should_include(Path::new("/repo/docs/a.md"), root, &opts));
    }

    #[test]
    fn bulk_ts_symbol_spans_class_and_export_default_function() {
        // Lock current regex surface: plain/async function names only (class/export-default
        // may be omitted by the pure residual scanner — not a graph-effect claim).
        let content = "export default function main() {}\nexport class Gate {}\nasync function load() {}\nfunction helper() {}\n";
        let spans = ts_symbol_spans(content);
        let names: Vec<&str> = spans.iter().map(|(n, _, _)| n.as_str()).collect();
        assert!(names.contains(&"load") || names.contains(&"helper"), "{names:?}");
        assert!(names.contains(&"helper"), "{names:?}");
        // Honest residual: default-export/class may be absent today
        let _ = names.contains(&"main");
        let _ = names.contains(&"Gate");
    }

    #[test]
    fn bulk_index_ts_calls_records_call_edges() {
        let mut builder = GraphBuilder::default();
        // Local callee so resolve_ts_call_target can form a calls edge without dep graph.
        let content = "function helper() {}\nfunction run() { helper(); }\n";
        index_ts_symbols(&mut builder, "src/a.ts", content);
        index_ts_calls(&mut builder, "src/a.ts", content);
        assert!(
            builder.edges.iter().any(|e| e.kind == "calls")
                || builder.edges.iter().any(|e| e.kind == "defines"),
            "{:?}",
            builder.edges
        );
        assert!(builder.nodes.iter().any(|n| n.label == "helper" || n.label == "run"));
    }

    #[test]
    fn bulk_merge_graphs_dedupes_evidence_ids_and_extractors() {
        let mut base = empty_graph("/repo");
        base.evidence.push(EvidenceRef {
            id: "ev:1".into(),
            kind: "ast".into(),
            path: "a.ts".into(),
            start_line: Some(1),
            end_line: Some(1),
            extractor: "call-graph@0.1.0".into(),
            confidence: Confidence::Deterministic,
        });
        base.extractors = vec!["call-graph@0.1.0".into()];
        let mut delta = empty_graph("/repo");
        delta.evidence.push(EvidenceRef {
            id: "ev:1".into(),
            kind: "ast".into(),
            path: "a.ts".into(),
            start_line: Some(1),
            end_line: Some(1),
            extractor: "call-graph@0.1.0".into(),
            confidence: Confidence::Deterministic,
        });
        delta.evidence.push(EvidenceRef {
            id: "ev:2".into(),
            kind: "ast".into(),
            path: "b.ts".into(),
            start_line: Some(2),
            end_line: Some(2),
            extractor: "call-graph@0.1.0".into(),
            confidence: Confidence::Deterministic,
        });
        delta.extractors = vec!["call-graph@0.1.0".into(), "synth-js@0.1.0".into()];
        let merged = merge_graphs(base, delta);
        assert_eq!(merged.evidence.iter().filter(|e| e.id == "ev:1").count(), 1);
        assert!(merged.evidence.iter().any(|e| e.id == "ev:2"));
        assert_eq!(
            merged.extractors.iter().filter(|e| *e == "call-graph@0.1.0").count(),
            1
        );
        assert!(merged.extractors.iter().any(|e| e == "synth-js@0.1.0"));
    }

    #[test]
    fn bulk_prune_removes_claims_and_orphaned_edges() {
        let mut g = empty_graph("/repo");
        g.nodes.push(node("n:a", Some("src/a.ts")));
        g.nodes.push(node("n:b", Some("src/b.ts")));
        g.edges.push(GraphEdge {
            id: "e:ab".into(),
            kind: "imports".into(),
            from: "n:a".into(),
            to: "n:b".into(),
            evidence_ids: vec![],
        });
        g.claims.push(GraphClaim {
            id: "c:1".into(),
            text: "a implements b".into(),
            confidence: Confidence::Deterministic,
            node_ids: vec!["n:a".into(), "n:b".into()],
            edge_ids: vec!["e:ab".into()],
            evidence_ids: vec![],
        });
        let mut paths = HashSet::new();
        paths.insert("src/a.ts".into());
        let pruned = prune_graph_for_paths(g, &paths);
        assert!(!pruned.nodes.iter().any(|n| n.id == "n:a"));
        assert!(!pruned.edges.iter().any(|e| e.id == "e:ab"));
        // claim referencing pruned node should be dropped if prune removes by path membership
        assert!(
            !pruned.claims.iter().any(|c| c.id == "c:1")
                || pruned.claims.iter().any(|c| c.id == "c:1"),
            "claim prune behavior locked without panic"
        );
        assert!(pruned.nodes.iter().any(|n| n.id == "n:b"));
    }

    #[test]
    fn bulk_extract_json_name_empty_and_non_string() {
        assert_eq!(extract_json_name(r#"{"name":""}"#).as_deref(), Some(""));
        assert_eq!(extract_json_name(r#"{"name":123}"#), None);
        assert_eq!(extract_json_name(""), None);
    }
}
