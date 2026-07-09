use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::scanner::{graph_path, index_dir};
use crate::types::{ArchitectureGraph, GRAPH_SCHEMA_VERSION};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHashManifest {
    pub schema_version: String,
    pub file_hashes: HashMap<String, String>,
}

impl Default for FileHashManifest {
    fn default() -> Self {
        Self {
            schema_version: GRAPH_SCHEMA_VERSION.into(),
            file_hashes: HashMap::new(),
        }
    }
}

pub fn file_hashes_path(root: &Path) -> std::path::PathBuf {
    index_dir(root).join("file-hashes.json")
}

pub fn load_graph(root: &Path) -> Option<ArchitectureGraph> {
    let path = graph_path(root);
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn load_file_hashes(root: &Path) -> Option<FileHashManifest> {
    let path = file_hashes_path(root);
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

pub fn save_file_hashes(root: &Path, manifest: &FileHashManifest) -> std::io::Result<()> {
    let dir = index_dir(root);
    fs::create_dir_all(&dir)?;
    let path = file_hashes_path(root);
    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(path, json)
}