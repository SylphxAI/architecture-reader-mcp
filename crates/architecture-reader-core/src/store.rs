use std::fs;
use std::path::Path;

use crate::scanner::{graph_path, index_dir};
use crate::types::ArchitectureGraph;

pub fn load_graph(root: &Path) -> Option<ArchitectureGraph> {
    let path = graph_path(root);
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_graph(root: &Path, graph: &ArchitectureGraph) -> std::io::Result<()> {
    let dir = index_dir(root);
    fs::create_dir_all(&dir)?;
    let path = graph_path(root);
    let json = serde_json::to_string_pretty(graph)?;
    fs::write(path, json)
}