use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scanner::GraphBuilder;

pub const SYNTH_JS_EXTRACTOR: &str = "synth-js@0.3.x";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthPosition {
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthSpan {
    pub start: SynthPosition,
    pub end: SynthPosition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthNode {
    pub id: u32,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub span: Option<SynthSpan>,
    pub parent: Option<u32>,
    #[serde(default)]
    pub children: Vec<u32>,
    #[serde(default)]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthTreeMeta {
    pub language: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthTree {
    pub meta: SynthTreeMeta,
    pub root: u32,
    pub nodes: Vec<SynthNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthCall {
    pub caller: String,
    pub callee: String,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct SynthExtraction {
    pub imports: Vec<String>,
    pub symbols: Vec<(String, String)>,
    pub calls: Vec<SynthCall>,
    pub gaps: Vec<String>,
}

pub fn parse_tree(raw: &str) -> Result<SynthTree, String> {
    serde_json::from_str(raw).map_err(|err| format!("Invalid Synth tree JSON: {err}"))
}

pub fn extract_facts(tree: &SynthTree) -> SynthExtraction {
    let mut out = SynthExtraction::default();
    let parent_map = parent_lookup(tree);

    for node in &tree.nodes {
        match node.node_type.as_str() {
            "ImportDeclaration" => {
                if let Some(source) = import_source(tree, node) {
                    out.imports.push(source);
                } else {
                    out.gaps.push("ImportDeclaration missing source literal.".into());
                }
            }
            "FunctionDeclaration" => {
                if let Some(name) = symbol_name(node) {
                    out.symbols.push((name, "function".into()));
                }
            }
            "ClassDeclaration" => {
                if let Some(name) = symbol_name(node) {
                    out.symbols.push((name, "class".into()));
                }
            }
            "ExportNamedDeclaration" => {
                if let Some(name) = export_named_symbol(tree, node) {
                    out.symbols.push((name, "export".into()));
                }
            }
            "ExportDefaultDeclaration" => {
                if let Some(name) = export_default_symbol(tree, node) {
                    out.symbols.push((name, "export_default".into()));
                }
            }
            "CallExpression" => {
                if let (Some(caller), Some(callee)) = (
                    enclosing_symbol_name(tree, node.id, &parent_map),
                    call_callee_name(tree, node),
                ) {
                    let line = node.span.as_ref().map(|span| span.start.line);
                    out.calls.push(SynthCall { caller, callee, line });
                }
            }
            _ => {}
        }
    }

    out.imports.sort();
    out.imports.dedup();
    out.calls.sort_by(|left, right| {
        left.caller
            .cmp(&right.caller)
            .then(left.callee.cmp(&right.callee))
            .then(left.line.cmp(&right.line))
    });
    out.calls.dedup();
    out
}

pub(crate) fn apply_to_builder(builder: &mut GraphBuilder, rel: &str, tree: &SynthTree) {
    let facts = extract_facts(tree);
    let file_id = format!("node:module:{}", rel.replace('/', ":"));

    let file_ev = builder.push_evidence(
        "ast",
        rel,
        SYNTH_JS_EXTRACTOR,
        None,
        None,
    );
    if !builder.has_node(&file_id) {
        builder.push_node(&file_id, "module", rel, Some(rel), &file_ev);
    }

    for import in facts.imports {
        let dep_id = format!("node:module:dep:{import}");
        if !builder.has_node(&dep_id) {
            builder.push_node(&dep_id, "module", &import, None, &file_ev);
        }
        builder.push_edge("imports", &file_id, &dep_id, &file_ev);
    }

    for (name, kind) in &facts.symbols {
        let line = tree
            .nodes
            .iter()
            .find(|node| symbol_name(node).as_deref() == Some(name.as_str()))
            .and_then(|node| node.span.as_ref().map(|span| span.start.line));
        let node_id = format!("node:symbol:{}:{}:{}", rel.replace('/', ":"), kind, name);
        if builder.has_node(&node_id) {
            continue;
        }
        let ev = builder.push_evidence("ast", rel, SYNTH_JS_EXTRACTOR, line, line);
        builder.push_node(&node_id, "symbol", &name, Some(rel), &ev);
        builder.push_edge("defines", &file_id, &node_id, &ev);
    }

    let import_map = import_local_map(tree);
    for call in facts.calls {
        let caller_kind = symbol_kind_for_name(&facts.symbols, &call.caller);
        let caller_id = format!(
            "node:symbol:{}:{}:{}",
            rel.replace('/', ":"),
            caller_kind,
            call.caller
        );
        if !builder.has_node(&caller_id) {
            continue;
        }
        if let Some(target_id) = resolve_call_target(builder, rel, &import_map, &call.callee) {
            let ev = builder.push_evidence("ast", rel, SYNTH_JS_EXTRACTOR, call.line, call.line);
            builder.push_edge("calls", &caller_id, &target_id, &ev);
        }
    }
}

fn symbol_kind_for_name(symbols: &[(String, String)], name: &str) -> String {
    symbols
        .iter()
        .find(|(symbol, _)| symbol == name)
        .map(|(_, kind)| kind.clone())
        .unwrap_or_else(|| "function".into())
}

fn import_local_map(tree: &SynthTree) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for node in &tree.nodes {
        if node.node_type != "ImportDeclaration" {
            continue;
        }
        let Some(source) = import_source(tree, node) else {
            continue;
        };
        for child_id in &node.children {
            let Some(child) = tree.nodes.get(*child_id as usize) else {
                continue;
            };
            if child.node_type == "ImportSpecifier" {
                if let Some(local) = child
                    .data
                    .as_ref()
                    .and_then(|data| data.get("local"))
                    .and_then(|value| value.as_str())
                {
                    out.insert(local.to_string(), source.clone());
                }
            } else if child.node_type == "ImportDefaultSpecifier" || child.node_type == "Identifier" {
                if let Some(local) = child
                    .data
                    .as_ref()
                    .and_then(|data| data.get("name").or_else(|| data.get("local")))
                    .and_then(|value| value.as_str())
                {
                    out.insert(local.to_string(), source.clone());
                }
            }
        }
    }
    out
}

fn resolve_call_target(
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
    let resolved_path = normalize_relative_module_path(rel, import_source)?;
    builder
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
        })
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

fn parent_lookup(tree: &SynthTree) -> HashMap<u32, u32> {
    let mut out = HashMap::new();
    for node in &tree.nodes {
        for child_id in &node.children {
            out.insert(*child_id, node.id);
        }
    }
    out
}

fn enclosing_symbol_name(
    tree: &SynthTree,
    node_id: u32,
    parent_map: &HashMap<u32, u32>,
) -> Option<String> {
    let mut current = parent_map.get(&node_id).copied();
    while let Some(parent_id) = current {
        let parent = tree.nodes.get(parent_id as usize)?;
        if matches!(
            parent.node_type.as_str(),
            "FunctionDeclaration" | "ClassDeclaration" | "MethodDefinition"
        ) {
            return symbol_name(parent);
        }
        current = parent_map.get(&parent_id).copied();
    }
    None
}

fn call_callee_name(tree: &SynthTree, node: &SynthNode) -> Option<String> {
    if let Some(name) = node
        .data
        .as_ref()
        .and_then(|data| data.get("callee"))
        .and_then(|value| value.as_str())
    {
        return Some(name.to_string());
    }
    for child_id in &node.children {
        let child = tree.nodes.get(*child_id as usize)?;
        if child.node_type == "Identifier" {
            if let Some(name) = child
                .data
                .as_ref()
                .and_then(|data| data.get("name"))
                .and_then(|value| value.as_str())
            {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn import_source(tree: &SynthTree, node: &SynthNode) -> Option<String> {
    if let Some(Value::String(source)) = node.data.as_ref().and_then(|data| data.get("source")) {
        return Some(source.clone());
    }

    for child_id in &node.children {
        let child = tree.nodes.get(*child_id as usize)?;
        if child.node_type == "Literal" {
            if let Some(Value::String(value)) = child.data.as_ref().and_then(|data| data.get("value"))
            {
                return Some(value.clone());
            }
        }
    }
    None
}

fn symbol_name(node: &SynthNode) -> Option<String> {
    node.data
        .as_ref()
        .and_then(|data| data.get("id"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn export_named_symbol(tree: &SynthTree, node: &SynthNode) -> Option<String> {
    for child_id in &node.children {
        let child = tree.nodes.get(*child_id as usize)?;
        if matches!(
            child.node_type.as_str(),
            "FunctionDeclaration" | "ClassDeclaration" | "VariableDeclaration"
        ) {
            if let Some(name) = symbol_name(child) {
                return Some(name);
            }
            if child.node_type == "VariableDeclaration" {
                for grandchild_id in &child.children {
                    let grandchild = tree.nodes.get(*grandchild_id as usize)?;
                    if grandchild.node_type == "VariableDeclarator" {
                        if let Some(id_child) = grandchild.children.first() {
                            let id_node = tree.nodes.get(*id_child as usize)?;
                            if id_node.node_type == "Identifier" {
                                if let Some(name) = id_node
                                    .data
                                    .as_ref()
                                    .and_then(|data| data.get("name"))
                                    .and_then(|value| value.as_str())
                                {
                                    return Some(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn export_default_symbol(tree: &SynthTree, node: &SynthNode) -> Option<String> {
    for child_id in &node.children {
        let child = tree.nodes.get(*child_id as usize)?;
        if let Some(name) = symbol_name(child) {
            return Some(name);
        }
        if child.node_type == "Identifier" {
            if let Some(name) = child
                .data
                .as_ref()
                .and_then(|data| data.get("name"))
                .and_then(|value| value.as_str())
            {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn middleware_tree_fixture() -> SynthTree {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/sample-repo/ast/auth-middleware.tree.json");
        let raw = fs::read_to_string(path).expect("fixture tree");
        parse_tree(&raw).expect("parse tree")
    }

    #[test]
    fn extracts_imports_and_symbols_from_synth_tree_fixture() {
        let tree = middleware_tree_fixture();
        let facts = extract_facts(&tree);

        assert!(facts.imports.iter().any(|import| import.contains("token.js")));
        assert!(facts.imports.iter().any(|import| import.contains("store.js")));
        assert!(facts
            .symbols
            .iter()
            .any(|(name, _)| name == "authMiddleware"));
    }

    #[test]
    fn applies_synth_facts_to_graph_builder_with_provenance() {
        let tree = middleware_tree_fixture();
        let mut builder = GraphBuilder::default();
        apply_to_builder(&mut builder, "src/auth/middleware.ts", &tree);

        assert!(builder
            .nodes
            .iter()
            .any(|node| node.kind == "symbol" && node.label == "authMiddleware"));
        assert!(builder.evidence.iter().any(|ev| ev.extractor == SYNTH_JS_EXTRACTOR));
    }

    #[test]
    fn extracts_call_edges_from_synth_tree_fixture() {
        let tree = middleware_tree_fixture();
        let facts = extract_facts(&tree);

        assert!(facts
            .calls
            .iter()
            .any(|call| call.caller == "authMiddleware" && call.callee == "validateToken"));
        assert!(facts
            .calls
            .iter()
            .any(|call| call.caller == "authMiddleware" && call.callee == "loadUser"));
    }

    #[test]
    fn applies_call_edges_when_target_symbols_exist() {
        let tree = middleware_tree_fixture();
        let mut builder = GraphBuilder::default();
        let token_ev = builder.push_evidence("ast", "src/auth/token.ts", SYNTH_JS_EXTRACTOR, None, None);
        builder.push_node(
            "node:symbol:src:auth:token.ts:function:validateToken",
            "symbol",
            "validateToken",
            Some("src/auth/token.ts"),
            &token_ev,
        );
        let store_ev = builder.push_evidence("ast", "src/users/store.ts", SYNTH_JS_EXTRACTOR, None, None);
        builder.push_node(
            "node:symbol:src:users:store.ts:function:loadUser",
            "symbol",
            "loadUser",
            Some("src/users/store.ts"),
            &store_ev,
        );

        apply_to_builder(&mut builder, "src/auth/middleware.ts", &tree);

        assert!(builder.edges.iter().any(|edge| {
            edge.kind == "calls"
                && edge.from.contains("authMiddleware")
                && edge.to.contains("validateToken")
        }));
        assert!(builder.edges.iter().any(|edge| {
            edge.kind == "calls"
                && edge.from.contains("authMiddleware")
                && edge.to.contains("loadUser")
        }));
    }
}