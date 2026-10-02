//! Team: one graph across several repository roots (T2).
//!
//! This is the hook the binary calls once a request names more than one root
//! and a Team licence is present. The join itself (package identity across
//! npm, Cargo, Go and Python manifests) lands here; until then the hook answers
//! [`NotJoined`] and the binary falls back to the current root.

use crate::query::{Direction, ImpactOptions, SearchOptions, TraceOptions};
use crate::workspace_graph::{join_roots, JoinOptions, WorkspaceGraph};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// A tool answer over the joined graph, in the same shape as a single-root answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Joined {
    pub text: String,
    pub json: Value,
}

/// The roots could not be joined (not implemented yet, or a root failed to index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotJoined;

impl fmt::Display for NotJoined {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not yet joined")
    }
}

impl std::error::Error for NotJoined {}

/// Answer `tool` (canonical name: `map`, `search`, `context`, `trace`, `impact`,
/// `db`) with `args` over the graph joined from `roots`. `roots` holds every
/// root named by the request, current root first, already resolved.
///
/// `search`, `trace` and `impact` cross repo boundaries (see
/// [`crate::workspace_graph`]); the other tools answer [`NotJoined`], so the
/// binary falls back to the current root for them. The joined graph is kept
/// per root set and refreshed on each call, so only changed roots are
/// re-indexed.
pub fn join_workspace(roots: &[PathBuf], tool: &str, args: &Value) -> Result<Joined, NotJoined> {
    if !matches!(tool, "search" | "trace" | "impact") {
        return Err(NotJoined);
    }
    let graph = graph_for(roots).ok_or(NotJoined)?;
    let g = graph.lock().map_err(|_| NotJoined)?;
    answer(&g, tool, args).ok_or(NotJoined)
}

type Cache = Mutex<HashMap<Vec<PathBuf>, Arc<Mutex<WorkspaceGraph>>>>;

fn graph_for(roots: &[PathBuf]) -> Option<Arc<Mutex<WorkspaceGraph>>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let key: Vec<PathBuf> = roots
        .iter()
        .map(|r| r.canonicalize().unwrap_or_else(|_| r.clone()))
        .collect();
    let cache = CACHE.get_or_init(Default::default);
    let held = cache.lock().ok()?.get(&key).cloned();
    if let Some(g) = held {
        g.lock().ok()?.refresh().ok()?;
        return Some(g);
    }
    let g = Arc::new(Mutex::new(join_roots(roots, &JoinOptions::default()).ok()?));
    cache.lock().ok()?.insert(key, g.clone());
    Some(g)
}

fn str_arg<'a>(args: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|k| args.get(*k).and_then(Value::as_str))
}

fn num_arg(args: &Value, keys: &[&str]) -> Option<usize> {
    keys.iter()
        .find_map(|k| args.get(*k).and_then(Value::as_u64))
        .map(|n| n as usize)
}

fn strings(args: &Value, keys: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for k in keys {
        match args.get(*k) {
            Some(Value::String(s)) => out.push(s.clone()),
            Some(Value::Array(a)) => {
                out.extend(a.iter().filter_map(|v| v.as_str().map(String::from)))
            }
            _ => {}
        }
    }
    out
}

fn answer(g: &WorkspaceGraph, tool: &str, args: &Value) -> Option<Joined> {
    fn joined<T: serde::Serialize>(text: String, v: &T) -> Option<Joined> {
        Some(Joined {
            text,
            json: serde_json::to_value(v).ok()?,
        })
    }
    match tool {
        "search" => {
            let opts = SearchOptions {
                limit: num_arg(args, &["limit"]).unwrap_or(10).clamp(1, 100),
                path: str_arg(args, &["path", "path_filter"]).map(String::from),
                kind: str_arg(args, &["kind"]).map(String::from),
                snippet_lines: 4,
                include_tests: args
                    .get("include_tests")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            };
            let r = g.search(str_arg(args, &["query", "q", "text"])?, &opts);
            joined(r.text(), &r)
        }
        "trace" => {
            let from = str_arg(
                args,
                &["from", "source", "start", "symbol", "node", "target"],
            )?;
            let to = if str_arg(args, &["from", "source", "start"]).is_some() {
                str_arg(args, &["to", "target", "end"])
            } else {
                str_arg(args, &["to", "end"])
            };
            let direction = match str_arg(args, &["direction"]) {
                Some("callers") | Some("up") | Some("in") | Some("incoming") => Direction::Callers,
                _ => Direction::Callees,
            };
            let r = g
                .trace(
                    from,
                    to,
                    &TraceOptions {
                        direction,
                        depth: num_arg(args, &["depth", "max_depth"]).unwrap_or(3),
                    },
                )
                .ok()?;
            joined(r.text(), &r)
        }
        "impact" => {
            let opts = ImpactOptions {
                depth: num_arg(args, &["depth", "max_depth"]).unwrap_or(3),
                limit: num_arg(args, &["limit"]).unwrap_or(30),
            };
            let targets = strings(
                args,
                &[
                    "target",
                    "targets",
                    "symbol",
                    "paths",
                    "changed_paths",
                    "files",
                ],
            );
            let changed = args
                .get("changed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || args
                    .get("use_git_diff")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            let r = if changed || targets.is_empty() {
                g.impact_changed(0, str_arg(args, &["base", "git_base"]), &opts)
            } else {
                g.impact(&targets, &opts)
            }
            .ok()?;
            joined(r.text(), &r)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untouched_tools_and_bad_roots_are_not_joined() {
        let roots = [
            PathBuf::from("/nonexistent/a"),
            PathBuf::from("/nonexistent/b"),
        ];
        for tool in ["map", "context", "db", "impact"] {
            assert_eq!(
                join_workspace(&roots, tool, &Value::Null)
                    .unwrap_err()
                    .to_string(),
                "not yet joined"
            );
        }
    }

    #[test]
    fn answers_across_two_roots() {
        let d = tempfile::tempdir().unwrap();
        let w = |root: &std::path::Path, rel: &str, body: &str| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        };
        let (a, b) = (d.path().join("a"), d.path().join("b"));
        w(&a, "package.json", r#"{"name":"@x/a"}"#);
        w(&a, "src/f.ts", "export function fa() {\n  return 1;\n}\n");
        w(&b, "package.json", r#"{"name":"b"}"#);
        w(
            &b,
            "src/g.ts",
            "import { fa } from '@x/a';\nexport function gb() {\n  return fa();\n}\n",
        );
        let roots = [a, b];
        let r =
            join_workspace(&roots, "impact", &serde_json::json!({"target": "src/f.ts"})).unwrap();
        assert!(r.text.contains("src/g.ts"), "{}", r.text);
        assert!(r.json["repos"]["b"]["direct"][0]["file"] == "src/g.ts");
        let r = join_workspace(&roots, "search", &serde_json::json!({"query": "gb"})).unwrap();
        assert!(r.json["hits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|h| h["repo"] == "b"));
    }
}
