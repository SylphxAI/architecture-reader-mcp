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
}
