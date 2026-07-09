use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitState {
    pub commit: Option<String>,
    pub dirty: bool,
}

pub fn read_git_state(root: &Path) -> GitState {
    let commit = Command::new("git")
        .args(["-C", root.to_string_lossy().as_ref(), "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let dirty = Command::new("git")
        .args([
            "-C",
            root.to_string_lossy().as_ref(),
            "status",
            "--porcelain",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    GitState { commit, dirty }
}

pub fn freshness(
    indexed_commit: Option<&str>,
    current_commit: Option<&str>,
    dirty: bool,
) -> crate::types::Freshness {
    use crate::types::Freshness;
    if dirty {
        return Freshness::Dirty;
    }
    match (indexed_commit, current_commit) {
        (Some(indexed), Some(current)) if indexed == current => Freshness::Fresh,
        (Some(_), Some(_)) => Freshness::Stale,
        (None, Some(_)) => Freshness::Unknown,
        _ => Freshness::Unknown,
    }
}