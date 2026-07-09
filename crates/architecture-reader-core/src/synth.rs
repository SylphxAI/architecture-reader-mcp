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

#[derive(Debug, Clone, Default)]
pub struct SynthExtraction {
    pub imports: Vec<String>,
    pub symbols: Vec<(String, String)>,
    pub gaps: Vec<String>,
}

pub fn parse_tree(raw: &str) -> Result<SynthTree, String> {
    serde_json::from_str(raw).map_err(|err| format!("Invalid Synth tree JSON: {err}"))
}

pub fn extract_facts(tree: &SynthTree) -> SynthExtraction {
    let mut out = SynthExtraction::default();

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
            _ => {}
        }
    }

    out.imports.sort();
    out.imports.dedup();
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

    for (name, kind) in facts.symbols {
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
}