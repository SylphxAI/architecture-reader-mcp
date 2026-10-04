//! Team: one graph across several repository roots (T2).
//!
//! This is the hook the binary calls once a request names more than one root
//! and a Team licence is present. The join itself (package identity across
//! npm, Cargo, Go and Python manifests) lives in [`crate::workspace_graph`];
//! a tool this hook cannot answer returns [`NotJoined`] and the binary falls
//! back to the current root.

use crate::index::Index;
use crate::query::{
    ContextOptions, Direction, ImpactOptions, MapOptions, SearchOptions, TraceOptions,
};
use crate::workspace_graph::{join_roots, JoinOptions, WorkspaceGraph};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

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
/// `map`, `search`, `context`, `trace` and `impact` cross repo boundaries (see
/// [`crate::workspace_graph`]). `db` is answered by the binary, one schema per
/// root, because a database is not one graph; here it is [`NotJoined`]. The joined graph is kept
/// per root set and refreshed on each call, so only changed roots are
/// re-indexed.
pub fn join_workspace(roots: &[PathBuf], tool: &str, args: &Value) -> Result<Joined, NotJoined> {
    join_workspace_with(roots, tool, args, HashMap::new)
}

/// [`join_workspace`] with indexes the caller already holds (keyed by canonical
/// root). `prebuilt` is only called when the graph is built, so a root the
/// free single-repo path already indexed is not indexed twice.
pub fn join_workspace_with(
    roots: &[PathBuf],
    tool: &str,
    args: &Value,
    prebuilt: impl FnOnce() -> HashMap<PathBuf, Arc<Index>>,
) -> Result<Joined, NotJoined> {
    join_workspace_shared(roots, HashMap::new(), tool, args, prebuilt)
}

/// [`join_workspace_with`] where some `roots` are shared-map links: `shared`
/// maps each such link to its index. The link must carry the map's commit, so
/// a newer map is a new graph.
pub fn join_workspace_shared(
    roots: &[PathBuf],
    shared: HashMap<PathBuf, Arc<Index>>,
    tool: &str,
    args: &Value,
    prebuilt: impl FnOnce() -> HashMap<PathBuf, Arc<Index>>,
) -> Result<Joined, NotJoined> {
    if !matches!(tool, "map" | "search" | "context" | "trace" | "impact") {
        return Err(NotJoined);
    }
    let graph = graph_for(roots, shared, prebuilt).ok_or(NotJoined)?;
    let g = graph.lock().map_err(|_| NotJoined)?;
    answer(&g, tool, args).ok_or(NotJoined)
}

/// Graphs kept, least recently used dropped first.
const MAX_GRAPHS: usize = 4;
/// Disk is rechecked at most this often per graph (same as the free workspace).
const RECHECK: Duration = Duration::from_millis(1500);

struct Cached {
    key: Vec<PathBuf>,
    graph: Arc<Mutex<WorkspaceGraph>>,
    checked: Instant,
}

/// Most recently used last.
type Cache = Mutex<Vec<Cached>>;

fn graph_for(
    roots: &[PathBuf],
    shared: HashMap<PathBuf, Arc<Index>>,
    prebuilt: impl FnOnce() -> HashMap<PathBuf, Arc<Index>>,
) -> Option<Arc<Mutex<WorkspaceGraph>>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let mut key: Vec<PathBuf> = roots
        .iter()
        .map(|r| if shared.contains_key(r) { r.clone() } else { r.canonicalize().unwrap_or_else(|_| r.clone()) })
        .collect();
    // The first root is the current one; the rest are a set, so order does not matter.
    if key.len() > 1 {
        key[1..].sort();
    }
    let cache = CACHE.get_or_init(Default::default);
    let held = {
        let mut c = cache.lock().ok()?;
        match c.iter().position(|e| e.key == key) {
            Some(i) => {
                let e = c.remove(i);
                let held = (e.graph.clone(), e.checked.elapsed() < RECHECK);
                c.push(e);
                Some(held)
            }
            None => None,
        }
    };
    if let Some((g, fresh)) = held {
        if fresh {
            return Some(g);
        }
        let refreshed = g.lock().ok().is_some_and(|mut w| w.refresh().is_ok());
        let mut c = cache.lock().ok()?;
        if !refreshed {
            // Drop the stale graph so the next call rebuilds it.
            c.retain(|e| !Arc::ptr_eq(&e.graph, &g));
            return None;
        }
        if let Some(e) = c.iter_mut().find(|e| Arc::ptr_eq(&e.graph, &g)) {
            e.checked = Instant::now();
        }
        return Some(g);
    }
    let opts = JoinOptions {
        prebuilt: prebuilt(),
        shared,
        ..Default::default()
    };
    let g = Arc::new(Mutex::new(join_roots(roots, &opts).ok()?));
    let mut c = cache.lock().ok()?;
    c.retain(|e| e.key != key);
    c.push(Cached {
        key,
        graph: g.clone(),
        checked: Instant::now(),
    });
    while c.len() > MAX_GRAPHS {
        c.remove(0);
    }
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
        "map" => {
            let opts = MapOptions {
                focus: str_arg(args, &["focus", "path", "scope"]).map(String::from),
                limit: num_arg(args, &["limit"]).unwrap_or(12),
            };
            let r = g.map(&opts, num_arg(args, &["tokens", "max_tokens"]));
            joined(r.text(), &r)
        }
        "context" => {
            let target = match (
                str_arg(args, &["target", "symbol", "node", "focus", "id", "file"]),
                str_arg(args, &["path"]),
                num_arg(args, &["line"]),
            ) {
                (Some(t), _, _) => t.to_string(),
                (None, Some(p), Some(l)) => format!("{p}:{l}"),
                (None, Some(p), None) => p.to_string(),
                _ => return None,
            };
            let opts = ContextOptions {
                code_lines: num_arg(args, &["code_lines"]).unwrap_or(60),
                limit: num_arg(args, &["limit"]).unwrap_or(25),
            };
            let r = g.context(&target, &opts).ok()?;
            joined(r.text(), &r)
        }
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
        for tool in ["map", "context", "db", "impact", "search", "trace"] {
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

    #[test]
    fn map_and_context_join_across_roots() {
        let d = tempfile::tempdir().unwrap();
        let w = |root: &std::path::Path, rel: &str, body: &str| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        };
        let (a, b) = (d.path().join("liba"), d.path().join("appb"));
        w(&a, "package.json", r#"{"name":"@x/liba"}"#);
        w(&a, "src/f.ts", "export function fa() {\n  return 1;\n}\n");
        w(&b, "package.json", r#"{"name":"appb"}"#);
        w(
            &b,
            "src/g.ts",
            "import { fa } from '@x/liba';\nexport function gb() {\n  return fa();\n}\n",
        );
        let roots = [a, b];
        let m = join_workspace(&roots, "map", &Value::Null).unwrap();
        assert_eq!(m.json["repos"].as_array().unwrap().len(), 2);
        assert!(m.text.contains("## Repo liba") && m.text.contains("## Repo appb"));
        assert_eq!(m.json["links"][0]["from"], "appb");
        let c = join_workspace(
            &roots,
            "context",
            &serde_json::json!({"target": "src/f.ts"}),
        )
        .unwrap();
        assert_eq!(c.json["found_in"][0]["repo"], "liba");
        assert_eq!(c.json["found_in"][0]["used_by"][0], "appb:src/g.ts");
        let c =
            join_workspace(&roots, "context", &serde_json::json!({"path": "src/g.ts"})).unwrap();
        assert_eq!(c.json["found_in"][0]["repo"], "appb");
        // No target, or one nobody knows, is not joined (the binary answers for the current repo).
        assert!(join_workspace(&roots, "context", &Value::Null).is_err());
        assert!(join_workspace(&roots, "context", &serde_json::json!({"target": "zzz"})).is_err());
    }
}
