//! In-memory indexes per repository root, refreshed when files change.

use anyhow::Result;
use repomap_core::{BuildOptions, Index};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Entry {
    index: Arc<Index>,
    checked: Instant,
    /// Deferred fixture paths a query has asked for; kept for later calls.
    include: Vec<String>,
}

#[derive(Default)]
pub struct Workspace {
    entries: Mutex<HashMap<PathBuf, Entry>>,
}

const RECHECK: Duration = Duration::from_millis(1500);

impl Workspace {
    pub fn get(&self, root: &Path) -> Result<Arc<Index>> {
        self.get_with(root, &[])
    }

    /// The index for `root`, with fixture paths that a query targets indexed
    /// too: `targets` that point into (or at) a deferred fixture tree.
    pub fn get_with(&self, root: &Path, targets: &[String]) -> Result<Arc<Index>> {
        let root = root.canonicalize()?;
        let mut map = self.entries.lock().unwrap();
        let mut include = Vec::new();
        if let Some(e) = map.get_mut(&root) {
            include = e.include.clone();
            let before = include.len();
            for t in targets {
                if let Some(p) = deferred_target(&e.index, t, &include) {
                    if !include.contains(&p) {
                        include.push(p);
                    }
                }
            }
            if include.len() == before {
                if e.checked.elapsed() < RECHECK {
                    return Ok(e.index.clone());
                }
                // Rebuild when files changed or the embedding model arrived.
                let opts = BuildOptions { include: include.clone(), ..Default::default() };
                if repomap_core::index::fingerprint(&root, &opts).ok() == Some(e.index.fingerprint)
                    && e.index.model_id == repomap_core::semantic::model_id()
                {
                    e.checked = Instant::now();
                    return Ok(e.index.clone());
                }
            }
        } else {
            // First build: targets are matched after it, below.
        }
        let opts = BuildOptions { include: include.clone(), ..Default::default() };
        let mut index = Arc::new(Index::build(&root, &opts)?);
        // A first call may already target a fixture tree: include and rebuild
        // (the per-file cache makes the second build cheap).
        let extra: Vec<String> = targets.iter().filter_map(|t| deferred_target(&index, t, &include)).filter(|p| !include.contains(p)).collect();
        if !extra.is_empty() {
            include.extend(extra);
            let opts = BuildOptions { include: include.clone(), ..Default::default() };
            index = Arc::new(Index::build(&root, &opts)?);
        }
        map.insert(root, Entry { index: index.clone(), checked: Instant::now(), include });
        Ok(index)
    }
}

/// The path to include when `target` (a file, `file:line`, or directory)
/// points into or at a deferred fixture tree.
fn deferred_target(index: &Index, target: &str, include: &[String]) -> Option<String> {
    let t = target.trim().trim_start_matches("./");
    let t = match t.rsplit_once(':') {
        Some((p, l)) if !l.is_empty() && l.bytes().all(|b| b.is_ascii_digit()) => p,
        _ => t,
    };
    let t = t.trim_end_matches('/');
    if t.is_empty() || !t.contains('/') && !t.contains('.') {
        return None;
    }
    // Already covered by an include, or already indexed: nothing to rebuild.
    if repomap_core::index::included(t, include) || index.path_ix.contains_key(t) {
        return None;
    }
    let under = |a: &str, b: &str| a.len() > b.len() && a.starts_with(b) && a.as_bytes()[b.len()] == b'/';
    index
        .deferred
        .iter()
        .any(|d| d.dir == t || under(t, &d.dir) || under(&d.dir, t))
        .then(|| t.to_string())
}
