//! repomap Team: the licence policy and the gate for the multi-repository
//! workspace (T2). Everything free stays free: a request that names one root
//! never reaches this gate, and an unlicensed multi-root request still answers
//! for the current root.

use crate::tools::Output;
use mcp_kit::licence::{self, LicencePolicy};
use repomap_core::index::Index;
use repomap_core::multirepo::{self, NotJoined};
use repomap_core::workspace_graph::{find_workspace_file, read_workspace_file};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Marker for the key list below. No token verifies while it is the only entry.
pub const KEY_PLACEHOLDER: &str = "PLACEHOLDER-repomap-team-issuer-public-key-not-issued-yet";

/// The repomap Team licence policy.
///
/// TODO(Services S1): replace `KEY_PLACEHOLDER` with the issued repomap-team
/// Ed25519 public key (base64url). Until then this list contains no valid key,
/// so every token is invalid and nothing Team unlocks.
pub const POLICY: LicencePolicy<'static> = LicencePolicy {
    product: "repomap",
    tier: "Team",
    require_product: true,
    accepted_plans: &["team"],
    public_keys: &[KEY_PLACEHOLDER],
    env_var: "REPOMAP_LICENCE_TOKEN",
    file_name: "licence",
    upgrade_url: "https://sylphxai.github.io/repomap/team",
};

/// The feature name in the `pro_required` notice.
pub const WORKSPACE_FEATURE: &str = "Multi-repository workspace";

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

/// Gate a request that names several roots. `single` answers for the current
/// root. Unlicensed: that answer plus `pro_required` and one line. Licensed:
/// the joined answer, or the current root's answer with a note while the join
/// is not available.
pub fn gate(
    policy: &LicencePolicy,
    roots: &[PathBuf],
    tool: &str,
    args: &Value,
    single: impl FnOnce() -> Result<Output, String>,
    prebuilt: impl FnOnce() -> HashMap<PathBuf, Arc<Index>>,
) -> Result<Output, String> {
    match licence::require(policy, WORKSPACE_FEATURE) {
        Err(required) => {
            let mut out = single()?;
            let notice = licence::required_result_json(&required)["structuredContent"]
                ["pro_required"]
                .clone();
            note(&mut out, "The other repos in this workspace were not joined; this answers for the current repo only.", Some(&required.to_string()), Some(notice));
            Ok(out)
        }
        Ok(_) => match multirepo::join_workspace_with(roots, tool, args, prebuilt) {
            Ok(joined) => Ok(Output {
                text: joined.text,
                json: joined.json,
            }),
            Err(NotJoined) => {
                let mut out = single()?;
                note(&mut out, "The other repos in this workspace were not joined (not yet joined); this answers for the current repo only.", None, None);
                Ok(out)
            }
        },
    }
}

fn note(out: &mut Output, line: &str, more: Option<&str>, pro_required: Option<Value>) {
    if !out.text.ends_with('\n') {
        out.text.push('\n');
    }
    out.text.push('\n');
    out.text.push_str(line);
    out.text.push('\n');
    if let Some(m) = more {
        out.text.push_str(m);
        out.text.push('\n');
    }
    if let Some(p) = pro_required {
        match &mut out.json {
            Value::Object(m) => {
                m.insert("pro_required".into(), p);
            }
            other => *other = json!({"result": other.clone(), "pro_required": p}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn placeholder_key_verifies_nothing() {
        assert_eq!(POLICY.public_keys, &[KEY_PLACEHOLDER]);
        for t in ["", "x.y", "e30.AAAA"] {
            assert!(POLICY.verify(t).is_err());
        }
        assert_eq!(POLICY.tier, "Team");
    }

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

    fn token(key: &SigningKey, payload: &str) -> String {
        let sig = key.sign(payload.as_bytes());
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(sig.to_bytes())
        )
    }

    fn single() -> Result<Output, String> {
        Ok(Output {
            text: "one repo".into(),
            json: json!({"root": "a"}),
        })
    }

    // One test owns the process-global token env, so it cannot race.
    #[test]
    fn gate_answers_for_the_current_root_and_marks_it() {
        const ENV: &str = "REPOMAP_TEST_LICENCE_TOKEN";
        let key = SigningKey::from_bytes(&[7; 32]);
        let public: &'static str = Box::leak(
            URL_SAFE_NO_PAD
                .encode(key.verifying_key().to_bytes())
                .into_boxed_str(),
        );
        let keys: &'static [&'static str] = Box::leak(vec![public].into_boxed_slice());
        let policy = LicencePolicy {
            file_name: "licence-test-never-exists",
            env_var: ENV,
            public_keys: keys,
            upgrade_url: "https://example.com/team",
            ..POLICY
        };
        let roots = [PathBuf::from("a"), PathBuf::from("b")];
        let args = json!({});

        // Unlicensed, and tokens that must not unlock: the single-root answer plus pro_required.
        let other = SigningKey::from_bytes(&[8; 32]);
        for t in [
            None,
            Some("junk".to_string()),
            Some(token(
                &key,
                r#"{"plan":"team","issuedAt":1,"product":"lockdocs"}"#,
            )),
            Some(token(
                &key,
                r#"{"plan":"pro","issuedAt":1,"product":"repomap"}"#,
            )),
            Some(token(
                &key,
                r#"{"plan":"team","issuedAt":1,"product":"repomap","expiresAt":1000}"#,
            )),
            Some(token(
                &other,
                r#"{"plan":"team","issuedAt":1,"product":"repomap"}"#,
            )),
        ] {
            match &t {
                Some(t) => std::env::set_var(ENV, t),
                None => std::env::remove_var(ENV),
            }
            let out = gate(&policy, &roots, "impact", &args, single, Default::default).unwrap();
            assert!(out.text.starts_with("one repo"), "{}", out.text);
            assert!(out.text.contains("were not joined"), "{}", out.text);
            assert!(
                out.text.contains("https://example.com/team"),
                "{}",
                out.text
            );
            assert_eq!(out.json["pro_required"]["product"], "repomap", "{t:?}");
            assert_eq!(out.json["pro_required"]["tier"], "Team");
            assert_eq!(out.json["root"], "a");
        }

        // Licensed: the hook is called; while it is not joined the answer is the current root's, no pro_required.
        std::env::set_var(
            ENV,
            token(
                &key,
                r#"{"plan":"team","issuedAt":1,"product":"repomap","seats":5}"#,
            ),
        );
        let out = gate(&policy, &roots, "impact", &args, single, Default::default).unwrap();
        assert!(
            out.text.starts_with("one repo") && out.text.contains("not yet joined"),
            "{}",
            out.text
        );
        assert!(out.json.get("pro_required").is_none());
        std::env::remove_var(ENV);
    }
}
