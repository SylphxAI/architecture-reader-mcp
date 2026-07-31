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


/// Paths changed in the worktree / index (git status --porcelain), relative to root.
/// Also supports `base` commit via `git diff --name-only <base>` when provided.
pub fn list_changed_paths(root: &Path, base: Option<&str>) -> Vec<String> {
    let root_s = root.to_string_lossy();
    let output = if let Some(base) = base {
        Command::new("git")
            .args(["-C", root_s.as_ref(), "diff", "--name-only", "--diff-filter=ACMRTUXB", base])
            .output()
    } else {
        // Unstaged + staged names
        Command::new("git")
            .args(["-C", root_s.as_ref(), "status", "--porcelain"])
            .output()
    };

    let Ok(output) = output else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut paths = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if base.is_some() {
            paths.push(line.to_string());
            continue;
        }
        // porcelain: XY PATH or XY ORIG -> PATH
        let rest = if line.len() >= 3 { line[3..].trim() } else { line };
        let path = if let Some((_, right)) = rest.split_once(" -> ") {
            right
        } else {
            rest
        };
        if !path.is_empty() {
            paths.push(path.to_string());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

#[cfg(test)]
mod pure_residual_tests {
    use super::*;
    use crate::types::Freshness;

    #[test]
    fn freshness_matrix_covers_dirty_fresh_stale_unknown() {
        assert_eq!(
            freshness(Some("aaa"), Some("aaa"), true),
            Freshness::Dirty
        );
        assert_eq!(
            freshness(Some("aaa"), Some("aaa"), false),
            Freshness::Fresh
        );
        assert_eq!(
            freshness(Some("aaa"), Some("bbb"), false),
            Freshness::Stale
        );
        assert_eq!(
            freshness(None, Some("aaa"), false),
            Freshness::Unknown
        );
        assert_eq!(
            freshness(Some("aaa"), None, false),
            Freshness::Unknown
        );
        assert_eq!(
            freshness(None, None, false),
            Freshness::Unknown
        );
    }
    #[test]
    fn freshness_dirty_wins_over_commit_mismatch() {
        // Dirty short-circuits regardless of commit equality.
        assert_eq!(
            freshness(Some("aaa"), Some("bbb"), true),
            Freshness::Dirty
        );
        assert_eq!(
            freshness(None, None, true),
            Freshness::Dirty
        );
    }

    #[test]
    fn bw7_freshness_unknown_when_indexed_missing_even_if_current_present() {
        // Matrix lock: None indexed + Some current + clean => Unknown (not Fresh/Stale).
        assert_eq!(freshness(None, Some("deadbeef"), false), Freshness::Unknown);
        // dirty still wins over that branch
        assert_eq!(freshness(None, Some("deadbeef"), true), Freshness::Dirty);
        // Some indexed + None current + clean => Unknown
        assert_eq!(freshness(Some("deadbeef"), None, false), Freshness::Unknown);
    }


    #[test]
    fn bw8_freshness_stale_and_dirty_precedence_matrix() {
        assert_eq!(freshness(Some("a"), Some("a"), true), Freshness::Dirty);
        assert_eq!(freshness(Some("a"), Some("b"), true), Freshness::Dirty);
        assert_eq!(freshness(None, Some("b"), true), Freshness::Dirty);
        assert_eq!(freshness(Some("a"), Some("b"), false), Freshness::Stale);
        assert_eq!(freshness(Some("same"), Some("same"), false), Freshness::Fresh);
        assert_eq!(freshness(None, None, false), Freshness::Unknown);
        assert_eq!(freshness(Some("x"), None, false), Freshness::Unknown);
        assert_eq!(freshness(None, Some("x"), false), Freshness::Unknown);
    }


    #[test]
    fn bulk_freshness_same_commit_clean_is_fresh() {
        assert_eq!(freshness(Some("abc"), Some("abc"), false), Freshness::Fresh);
        assert_eq!(freshness(Some("abc"), Some("abc"), true), Freshness::Dirty);
        assert_eq!(freshness(Some("abc"), Some("def"), true), Freshness::Dirty);
        assert_eq!(freshness(Some("abc"), None, false), Freshness::Unknown);
    }
    #[test]
    fn list_changed_paths_returns_empty_outside_git() {
        let dir = std::env::temp_dir().join(format!("spine-git-none-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let paths = list_changed_paths(&dir, None);
        assert!(paths.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

}
