//! Cache overrides are tested in a child harness so environment changes do
//! not race the other integration tests.

use repomap_core::index::{cache_dir, cache_root};
use std::path::PathBuf;

#[test]
fn cache_override_is_shared_by_global_state_and_repo_facts() {
    const CHILD: &str = "REPOMAP_CACHE_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let expected = PathBuf::from(std::env::var("REPOMAP_CACHE_DIR").unwrap());
        assert_eq!(cache_root(), expected);
        let facts = cache_dir(&PathBuf::from("project"));
        assert_eq!(facts.parent(), Some(expected.as_path()));
        assert_eq!(cache_root().join("star-hint"), expected.join("star-hint"));
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for override_value in [dir.path().as_os_str(), std::ffi::OsStr::new("")] {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "cache_override_is_shared_by_global_state_and_repo_facts"])
            .env(CHILD, "1")
            .env("REPOMAP_CACHE_DIR", override_value)
            .status()
            .unwrap();
        assert!(status.success());
    }
}
