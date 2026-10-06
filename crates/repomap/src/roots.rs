//! The multi-repository workspace: which roots a request names, and the one
//! join over them. A request that names one root never reaches this module;
//! one that names several gets the joined answer, free, like every other tool.

use crate::tools::Output;
use repomap_core::index::Index;
use repomap_core::multirepo::{self, NotJoined};
use repomap_core::workspace_graph::{find_workspace_file, read_workspace_file};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The file that lists several repository roots: `roots = ["../api", "../web"]`.
pub use repomap_core::workspace_graph::WORKSPACE_FILE;

/// The one decision: does this request name more than one workspace root?
/// A `workspace` argument (a list of roots, a comma-separated string, or the
/// path to a `repomap.workspace.toml` or to a directory holding one), the CLI's
/// `--workspace`, or a `repomap.workspace.toml` in the current root. Returns
/// every root, the current root first, or an empty list when there is one.
pub fn named_roots(args: &Value, root: &Path) -> Vec<PathBuf> {
    let listed = match args.get("workspace") {
        Some(Value::Array(v)) => v
            .iter()
            .filter_map(|x| x.as_str())
            .map(|s| root.join(s))
            .collect(),
        Some(Value::String(s)) if s.contains(',') => s
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| root.join(p))
            .collect(),
        Some(Value::String(s)) if !s.trim().is_empty() => {
            let p = root.join(s.trim());
            let file = if p.is_dir() {
                p.join(WORKSPACE_FILE)
            } else {
                p.clone()
            };
            if file.is_file() {
                read_roots(&file)
            } else {
                vec![p]
            }
        }
        Some(_) => Vec::new(),
        None => find_workspace_file(root)
            .map(|f| read_roots(&f))
            .unwrap_or_default(),
    };
    let key = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let mut roots = vec![root.to_path_buf()];
    let current = key(root);
    let home = dirs::home_dir().map(|h| key(&h));
    let mut seen = vec![current.clone()];
    for p in listed {
        let k = key(&p);
        // A root must be an existing directory, and never `/`, the home
        // directory or a parent of the current root (indexing those would
        // walk the whole machine).
        let unsafe_root = !k.is_dir()
            || k.parent().is_none()
            || home.as_ref() == Some(&k)
            || current.starts_with(&k);
        if !unsafe_root && !seen.contains(&k) {
            seen.push(k);
            roots.push(p);
        }
    }
    if roots.len() > 1 {
        roots
    } else {
        Vec::new()
    }
}

/// The roots in a workspace file, resolved against the file's directory.
fn read_roots(file: &Path) -> Vec<PathBuf> {
    read_workspace_file(file).unwrap_or_default()
}

/// Answer a request that names several roots. `single` answers for the
/// current root. `per_root` is for a tool the binary answers itself, once per
/// root (`db`); `None` means "use the joined graph". When the roots cannot be
/// joined the current root's answer comes back with one line saying so.
pub fn join(
    roots: &[PathBuf],
    tool: &str,
    args: &Value,
    single: impl FnOnce() -> Result<Output, String>,
    prebuilt: impl FnOnce() -> HashMap<PathBuf, Arc<Index>>,
    per_root: impl FnOnce() -> Option<Result<Output, String>>,
) -> Result<Output, String> {
    if let Some(answer) = per_root() {
        return answer;
    }
    match multirepo::join_workspace_with(roots, tool, args, prebuilt) {
        Ok(joined) => Ok(Output {
            text: joined.text,
            json: joined.json,
        }),
        Err(NotJoined) => {
            let mut out = single()?;
            if !out.text.ends_with('\n') {
                out.text.push('\n');
            }
            out.text.push_str("\nThe other repos in this workspace were not joined (not yet joined); this answers for the current repo only.\n");
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn workspace_file_is_read_by_the_core_parser() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join(WORKSPACE_FILE);
        std::fs::write(
            &f,
            "# repos\nroots = [\n  \"../api\", # the api\n  '../web',\n]\n",
        )
        .unwrap();
        assert_eq!(read_roots(&f).len(), 2);
        std::fs::write(&f, "[other]\nroots = [\"../x\"]\n").unwrap();
        assert!(read_roots(&f).is_empty());
    }

    #[test]
    fn unsafe_and_missing_roots_are_dropped() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path().join("a/app");
        std::fs::create_dir_all(&r).unwrap();
        let args = json!({"workspace": ["/", "..", "../..", "../missing", "../../../.."]});
        assert!(named_roots(&args, &r).is_empty());
        if let Some(h) = dirs::home_dir() {
            let args = json!({"workspace": [h.to_string_lossy()]});
            assert!(named_roots(&args, &r).is_empty());
        }
    }

    #[test]
    fn one_root_is_not_a_workspace() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        assert!(named_roots(&json!({}), r).is_empty());
        assert!(named_roots(&json!({"workspace": []}), r).is_empty());
        assert!(named_roots(&json!({"workspace": ["."]}), r).is_empty());
        std::fs::write(r.join(WORKSPACE_FILE), "roots = [\".\"]\n").unwrap();
        assert!(named_roots(&json!({}), r).is_empty());
    }

    #[test]
    fn many_roots_from_each_source() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path().join("app");
        for n in ["app", "api", "web"] {
            std::fs::create_dir_all(d.path().join(n)).unwrap();
        }
        assert_eq!(
            named_roots(&json!({"workspace": ["../api", "../web"]}), &r).len(),
            3
        );
        assert_eq!(
            named_roots(&json!({"workspace": "../api, ../web"}), &r).len(),
            3
        );
        assert_eq!(named_roots(&json!({"workspace": "../api"}), &r).len(), 2);
        std::fs::write(r.join(WORKSPACE_FILE), "roots = [\"../api\", \"../web\"]\n").unwrap();
        assert_eq!(named_roots(&json!({}), &r).len(), 3);
        assert_eq!(named_roots(&json!({"workspace": "."}), &r).len(), 3);
        assert_eq!(
            named_roots(&json!({"workspace": WORKSPACE_FILE}), &r).len(),
            3
        );
    }

    fn single() -> Result<Output, String> {
        Ok(Output {
            text: "one repo".into(),
            json: json!({"root": "a"}),
        })
    }

    /// Guard for the end of repomap Team: a multi-root request never needs a
    /// licence and never carries `pro_required`, whatever token is set.
    #[test]
    fn several_roots_need_no_licence() {
        std::env::set_var("REPOMAP_LICENCE_TOKEN", "anything");
        let roots = [PathBuf::from("a"), PathBuf::from("b")];
        for tool in ["map", "search", "context", "trace", "impact", "db"] {
            let out = join(&roots, tool, &json!({}), single, Default::default, || None).unwrap();
            assert!(out.json.get("pro_required").is_none(), "{tool}");
            assert!(!out.text.contains("licence"), "{tool}: {}", out.text);
        }
        let out = join(&roots, "db", &json!({}), single, Default::default, || {
            Some(Ok(Output {
                text: "per root".into(),
                json: json!({"repos": {}}),
            }))
        })
        .unwrap();
        assert_eq!(out.text, "per root");
        std::env::remove_var("REPOMAP_LICENCE_TOKEN");
    }
}
