use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::time::Instant;

use serde_json::json;

use crate::git::{freshness, read_git_state};
use crate::scanner::{
    incremental_refresh, inventory_files, scan_repository, ScanOptions,
};
use crate::store::{load_file_hashes, load_graph, save_file_hashes, save_graph, FileHashManifest};
use crate::types::{
    ArchitectureGraph, EvidenceRef, Metrics, RepositoryState, ToolEnvelope, GRAPH_SCHEMA_VERSION,
};

pub fn handle_tool(tool: &str, input: serde_json::Value) -> ToolEnvelope {
    let started = Instant::now();
    let result = match tool {
        "architecture_index" => architecture_index(input),
        "architecture_status" => architecture_status(input),
        "architecture_overview" => architecture_overview(input),
        "architecture_search" => architecture_search(input),
        "architecture_trace" => architecture_trace(input),
        "architecture_impact" => architecture_impact(input),
        "architecture_evidence" => architecture_evidence(input),
        _ => ToolEnvelope::error("UNSUPPORTED_QUERY", &format!("Unknown tool: {tool}"), None),
    };
    let _ = started;
    result
}

fn resolve_root(input: &serde_json::Value) -> Result<std::path::PathBuf, ToolEnvelope> {
    let root = input
        .get("root")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolEnvelope::error("INVALID_ROOT", "Missing required field: root", None))?;
    let path = std::path::PathBuf::from(root);
    if !path.is_dir() {
        return Err(ToolEnvelope::error(
            "INVALID_ROOT",
            &format!("Root is not a directory: {root}"),
            None,
        ));
    }
    Ok(path.canonicalize().unwrap_or(path))
}

fn repository_state(root: &Path, graph: Option<&ArchitectureGraph>) -> RepositoryState {
    let git = read_git_state(root);
    let indexed_commit = graph.and_then(|g| g.repository.git_commit.clone());
    RepositoryState {
        root: root.to_string_lossy().to_string(),
        indexed_commit: indexed_commit.clone(),
        current_commit: git.commit.clone(),
        freshness: freshness(
            indexed_commit.as_deref(),
            git.commit.as_deref(),
            git.dirty,
        ),
        worktree_dirty: git.dirty,
    }
}

fn metrics(started: Instant, graph: &ArchitectureGraph) -> Metrics {
    Metrics {
        elapsed_ms: started.elapsed().as_millis() as u64,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
    }
}

fn require_graph(root: &Path) -> Result<ArchitectureGraph, ToolEnvelope> {
    load_graph(root).ok_or_else(|| {
        ToolEnvelope::error(
            "INDEX_NOT_FOUND",
            "No architecture index exists for this repository.",
            Some("Call architecture_index with mode auto."),
        )
    })
}

fn architecture_index(input: serde_json::Value) -> ToolEnvelope {
    let started = Instant::now();
    let root = match resolve_root(&input) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let mode = input.get("mode").and_then(|v| v.as_str()).unwrap_or("auto");
    let mut options = ScanOptions::default();
    if let Some(include) = input.get("include").and_then(|v| v.as_array()) {
        options.include = include
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
    }
    if let Some(exclude) = input.get("exclude").and_then(|v| v.as_array()) {
        options.exclude = exclude
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
    }
    if let Some(max) = input.get("maxFileBytes").and_then(|v| v.as_u64()) {
        options.max_file_bytes = max;
    }
    if let Some(use_synth) = input.get("useSynth").and_then(|v| v.as_bool()) {
        options.use_synth = use_synth;
    } else if crate::synth_probe::probe_enabled_from_env() {
        options.use_synth = true;
    }

    if mode == "status_only" {
        let graph = load_graph(&root);
        let repo = repository_state(&root, graph.as_ref());
        return ToolEnvelope::ok(
            repo,
            json!({ "indexed": graph.is_some(), "mode": "status_only" }),
            vec![],
            vec![],
            Metrics {
                elapsed_ms: started.elapsed().as_millis() as u64,
                node_count: graph.as_ref().map(|g| g.nodes.len()).unwrap_or(0),
                edge_count: graph.as_ref().map(|g| g.edges.len()).unwrap_or(0),
            },
        );
    }

    let git = read_git_state(&root);
    let inventory = inventory_files(&root, &options);
    let stored_hashes = load_file_hashes(&root);
    let refresh_mode;
    let graph = if mode == "auto" {
        if let (Some(existing), Some(stored)) = (load_graph(&root), stored_hashes.as_ref()) {
            if existing.schema_version == GRAPH_SCHEMA_VERSION
                && stored.schema_version == GRAPH_SCHEMA_VERSION
                && stored.file_hashes == inventory
            {
                refresh_mode = "cache_hit";
                ArchitectureGraph {
                    repository: crate::types::RepositorySnapshot {
                        root: existing.repository.root.clone(),
                        git_commit: git.commit.clone(),
                        worktree_dirty: git.dirty,
                    },
                    ..existing
                }
            } else if existing.schema_version == GRAPH_SCHEMA_VERSION
                && stored.schema_version == GRAPH_SCHEMA_VERSION
            {
                let mut changed = HashSet::new();
                let mut deleted = HashSet::new();
                for (path, hash) in &inventory {
                    match stored.file_hashes.get(path) {
                        Some(previous) if previous == hash => {}
                        Some(_) | None => {
                            changed.insert(path.clone());
                        }
                    }
                }
                for path in stored.file_hashes.keys() {
                    if !inventory.contains_key(path) {
                        deleted.insert(path.clone());
                    }
                }

                let affected = changed.len() + deleted.len();
                if affected > 0 && affected * 2 <= inventory.len().max(1) {
                    refresh_mode = "incremental";
                    incremental_refresh(
                        &root,
                        &options,
                        existing,
                        &changed,
                        &deleted,
                        git.commit.clone(),
                        git.dirty,
                    )
                } else {
                    refresh_mode = "full";
                    scan_repository(&root, &options, git.commit.clone(), git.dirty)
                }
            } else {
                refresh_mode = "full";
                scan_repository(&root, &options, git.commit.clone(), git.dirty)
            }
        } else {
            refresh_mode = "full";
            scan_repository(&root, &options, git.commit.clone(), git.dirty)
        }
    } else {
        refresh_mode = "full";
        scan_repository(&root, &options, git.commit.clone(), git.dirty)
    };

    if refresh_mode != "cache_hit" {
        if let Err(err) = save_graph(&root, &graph) {
            return ToolEnvelope::error(
                "INTERNAL_ERROR",
                &format!("Failed to persist index: {err}"),
                None,
            );
        }
        if let Err(err) = save_file_hashes(
            &root,
            &FileHashManifest {
                schema_version: GRAPH_SCHEMA_VERSION.into(),
                file_hashes: inventory,
            },
        ) {
            return ToolEnvelope::error(
                "INTERNAL_ERROR",
                &format!("Failed to persist file hash manifest: {err}"),
                None,
            );
        }
    }

    let manifest_nodes = graph.nodes.iter().filter(|n| n.kind == "package").count();
    let module_nodes = graph.nodes.iter().filter(|n| n.kind == "module").count();
    let doc_nodes = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "document" || n.kind == "adr")
        .count();
    let coverage_ast = if module_nodes == 0 {
        0.0
    } else {
        (module_nodes as f64 / graph.nodes.len().max(1) as f64).min(1.0)
    };
    let coverage_docs = if doc_nodes == 0 {
        0.0
    } else {
        (doc_nodes as f64 / graph.nodes.len().max(1) as f64).min(1.0)
    };

    let repo = repository_state(&root, Some(&graph));
    ToolEnvelope::ok(
        repo,
        json!({
            "indexed": true,
            "refreshMode": refresh_mode,
            "filesScanned": graph.nodes.len(),
            "filesIndexed": graph.nodes.len(),
            "nodes": graph.nodes.len(),
            "edges": graph.edges.len(),
            "coverage": {
                "ast": coverage_ast,
                "manifests": manifest_nodes,
                "docs": coverage_docs
            }
        }),
        graph.evidence.clone(),
        vec![],
        metrics(started, &graph),
    )
}

fn architecture_status(input: serde_json::Value) -> ToolEnvelope {
    let started = Instant::now();
    let root = match resolve_root(&input) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let graph = match require_graph(&root) {
        Ok(g) => g,
        Err(e) => return e,
    };
    let repo = repository_state(&root, Some(&graph));
    let synth_active = graph
        .extractors
        .iter()
        .any(|extractor| extractor.starts_with("synth-"));
    let gaps = coverage_gaps(&graph, synth_active);
    ToolEnvelope::ok(
        repo,
        json!({
            "indexed": true,
            "extractors": graph.extractors,
            "schemaVersion": graph.schema_version,
            "coverage": {
                "nodes": graph.nodes.len(),
                "edges": graph.edges.len(),
                "claims": graph.claims.len(),
                "synthMode": if synth_active { "active" } else { "off" },
                "importGraphRoute": if synth_active { "synth_ast" } else { "regex_fallback" }
            }
        }),
        graph.evidence.clone(),
        gaps,
        metrics(started, &graph),
    )
}

fn architecture_overview(input: serde_json::Value) -> ToolEnvelope {
    let started = Instant::now();
    let root = match resolve_root(&input) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let graph = match require_graph(&root) {
        Ok(g) => g,
        Err(e) => return e,
    };
    let depth = input.get("depth").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
    let focus = input.get("focus").and_then(|v| v.as_str()).unwrap_or("all");

    let packages: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "package")
        .take(depth)
        .map(|n| json!({ "id": n.id, "label": n.label, "path": n.path }))
        .collect();
    let modules: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "module" && n.path.is_some())
        .take(depth * 4)
        .map(|n| json!({ "id": n.id, "label": n.label, "path": n.path }))
        .collect();
    let docs: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "document" || n.kind == "adr")
        .take(depth * 2)
        .map(|n| json!({ "id": n.id, "kind": n.kind, "path": n.path }))
        .collect();

    let repo = repository_state(&root, Some(&graph));
    ToolEnvelope::ok(
        repo,
        json!({
            "focus": focus,
            "packages": packages,
            "modules": modules,
            "documents": docs,
            "claims": graph.claims.iter().take(depth).map(|c| json!({ "id": c.id, "text": c.text })).collect::<Vec<_>>()
        }),
        graph.evidence.clone(),
        coverage_gaps(
            &graph,
            graph
                .extractors
                .iter()
                .any(|extractor| extractor.starts_with("synth-")),
        ),
        metrics(started, &graph),
    )
}

fn architecture_search(input: serde_json::Value) -> ToolEnvelope {
    let started = Instant::now();
    let root = match resolve_root(&input) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let graph = match require_graph(&root) {
        Ok(g) => g,
        Err(e) => return e,
    };
    let query = input
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let types: HashSet<String> = input
        .get("types")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();

    let mut matches = Vec::new();
    for node in &graph.nodes {
        if !types.is_empty() && !types.contains(&node.kind) {
            continue;
        }
        let hay = format!(
            "{} {} {}",
            node.label,
            node.path.clone().unwrap_or_default(),
            node.kind
        )
        .to_lowercase();
        if query.is_empty() || hay.contains(&query) {
            matches.push(json!({
                "id": node.id,
                "kind": node.kind,
                "label": node.label,
                "path": node.path,
                "score": if query.is_empty() { 0.5 } else { 1.0 }
            }));
        }
        if matches.len() >= limit {
            break;
        }
    }

    let include_evidence = input
        .get("includeEvidence")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let evidence = if include_evidence {
        collect_evidence_for_nodes(&graph, &matches)
    } else {
        vec![]
    };

    let repo = repository_state(&root, Some(&graph));
    ToolEnvelope::ok(
        repo,
        json!({ "matches": matches }),
        evidence,
        coverage_gaps(
            &graph,
            graph
                .extractors
                .iter()
                .any(|extractor| extractor.starts_with("synth-")),
        ),
        metrics(started, &graph),
    )
}

fn architecture_trace(input: serde_json::Value) -> ToolEnvelope {
    let started = Instant::now();
    let root = match resolve_root(&input) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let graph = match require_graph(&root) {
        Ok(g) => g,
        Err(e) => return e,
    };
    let from = input.get("from").and_then(|v| v.as_str()).unwrap_or("");
    let to = input.get("to").and_then(|v| v.as_str()).unwrap_or("");
    let relation = input.get("relation").and_then(|v| v.as_str()).unwrap_or("any");
    let max_depth = input.get("maxDepth").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

    let from_id = resolve_node_id(&graph, from);
    let to_id = resolve_node_id(&graph, to);
    let path = if let (Some(from_id), Some(to_id)) = (from_id, to_id) {
        bfs_path(&graph, &from_id, &to_id, relation, max_depth)
    } else {
        vec![]
    };

    let repo = repository_state(&root, Some(&graph));
    ToolEnvelope::ok(
        repo,
        json!({
            "from": from,
            "to": to,
            "relation": relation,
            "path": path
        }),
        graph.evidence.clone(),
        if path.is_empty() {
            vec!["No trace path found between the requested entities.".into()]
        } else {
            vec![]
        },
        metrics(started, &graph),
    )
}

fn architecture_impact(input: serde_json::Value) -> ToolEnvelope {
    let started = Instant::now();
    let root = match resolve_root(&input) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let graph = match require_graph(&root) {
        Ok(g) => g,
        Err(e) => return e,
    };
    let changed: Vec<String> = input
        .get("changedPaths")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();

    let mut direct = Vec::new();
    let mut transitive = Vec::new();
    for path in &changed {
        for node in graph.nodes.iter().filter(|n| n.path.as_deref() == Some(path.as_str())) {
            direct.push(json!({ "id": node.id, "path": node.path, "kind": node.kind }));
            for edge in graph.edges.iter().filter(|e| e.from == node.id) {
                transitive.push(json!({ "edge": edge.kind, "to": edge.to }));
            }
        }
    }

    let repo = repository_state(&root, Some(&graph));
    ToolEnvelope::ok(
        repo,
        json!({
            "changedPaths": changed,
            "directImpact": direct,
            "transitiveImpact": transitive,
            "unknownImpact": []
        }),
        graph.evidence.clone(),
        coverage_gaps(
            &graph,
            graph
                .extractors
                .iter()
                .any(|extractor| extractor.starts_with("synth-")),
        ),
        metrics(started, &graph),
    )
}

fn architecture_evidence(input: serde_json::Value) -> ToolEnvelope {
    let started = Instant::now();
    let root = match resolve_root(&input) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let graph = match require_graph(&root) {
        Ok(g) => g,
        Err(e) => return e,
    };
    let ids: Vec<String> = input
        .get("ids")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();

    let mut found = Vec::new();
    for id in &ids {
        if let Some(ev) = graph.evidence.iter().find(|e| e.id == *id) {
            found.push(ev.clone());
            continue;
        }
        if let Some(node) = graph.nodes.iter().find(|n| n.id == *id) {
            for ev_id in &node.evidence_ids {
                if let Some(ev) = graph.evidence.iter().find(|e| &e.id == ev_id) {
                    found.push(ev.clone());
                }
            }
        }
    }

    if found.is_empty() {
        return ToolEnvelope::error(
            "EVIDENCE_NOT_FOUND",
            "No evidence found for the requested ids.",
            Some("Call architecture_search or architecture_overview first."),
        );
    }

    let repo = repository_state(&root, Some(&graph));
    ToolEnvelope::ok(
        repo,
        json!({ "items": found.len(), "ids": ids }),
        found,
        vec![],
        metrics(started, &graph),
    )
}

fn coverage_gaps(graph: &ArchitectureGraph, synth_active: bool) -> Vec<String> {
    let mut gaps = Vec::new();
    if graph.nodes.iter().all(|n| n.kind != "route") {
        gaps.push("Route extraction not yet implemented.".into());
    }
    if graph.nodes.iter().all(|n| n.kind != "schema") {
        gaps.push("Schema extraction not yet implemented.".into());
    }
    if graph.repository.worktree_dirty {
        gaps.push("Working tree has uncommitted changes.".into());
    }
    if !synth_active {
        gaps.push(
            "Synth AST substrate is off by default (importGraphRoute=regex_fallback). Set ARCHITECTURE_READER_USE_SYNTH=1 to enable synth_ast in CI or local runs.".into(),
        );
    }
    gaps
}

fn resolve_node_id(graph: &ArchitectureGraph, needle: &str) -> Option<String> {
    if let Some(node) = graph
        .nodes
        .iter()
        .find(|n| n.id == needle || n.path.as_deref() == Some(needle) || n.label == needle)
    {
        return Some(node.id.clone());
    }

    let symbol_matches: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "symbol" && n.label == needle)
        .collect();
    if symbol_matches.len() == 1 {
        return Some(symbol_matches[0].id.clone());
    }

    if let Some((path, symbol)) = needle.split_once("::") {
        return graph
            .nodes
            .iter()
            .find(|n| n.kind == "symbol" && n.label == symbol && n.path.as_deref() == Some(path))
            .map(|n| n.id.clone());
    }

    None
}

fn bfs_path(
    graph: &ArchitectureGraph,
    from: &str,
    to: &str,
    relation: &str,
    max_depth: usize,
) -> Vec<String> {
    let adjacency: HashMap<&str, Vec<&str>> = graph
        .edges
        .iter()
        .filter(|e| relation == "any" || e.kind == relation)
        .fold(HashMap::new(), |mut acc, edge| {
            acc.entry(edge.from.as_str()).or_default().push(edge.to.as_str());
            acc
        });

    let mut queue = VecDeque::from([(from.to_string(), vec![from.to_string()])]);
    let mut visited = HashSet::from([from.to_string()]);

    while let Some((current, path)) = queue.pop_front() {
        if path.len() > max_depth {
            continue;
        }
        if current == to {
            return path;
        }
        if let Some(neighbors) = adjacency.get(current.as_str()) {
            for next in neighbors {
                if visited.insert((*next).to_string()) {
                    let mut next_path = path.clone();
                    next_path.push((*next).to_string());
                    queue.push_back(((*next).to_string(), next_path));
                }
            }
        }
    }
    vec![]
}

fn collect_evidence_for_nodes(graph: &ArchitectureGraph, matches: &[serde_json::Value]) -> Vec<EvidenceRef> {
    let mut out = Vec::new();
    for m in matches {
        let Some(id) = m.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        if let Some(node) = graph.nodes.iter().find(|n| n.id == id) {
            for ev_id in &node.evidence_ids {
                if let Some(ev) = graph.evidence.iter().find(|e| &e.id == ev_id) {
                    out.push(ev.clone());
                }
            }
        }
    }
    out
}