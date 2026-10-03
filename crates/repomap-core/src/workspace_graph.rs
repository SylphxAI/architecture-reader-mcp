//! Multi-repo workspace graph.
//!
//! `repomap.workspace.toml` lists several repository roots. Each root keeps its
//! own [`Index`] (built by `Index::build`, with its per-file fact cache), and
//! this module joins them through *package identity*: a package one repo
//! publishes (npm `name`, Cargo package or `[lib]` name, Go module path, Python
//! project name) and another repo imports. `impact`, `trace` and `search` then
//! cross repo boundaries.
//!
//! The join never parses anything itself. It reads the raw import specifiers
//! the per-root indexes already hold (`Index::imports_raw`) and the package
//! manifests, so joining an unchanged root costs a manifest scan, and
//! [`WorkspaceGraph::refresh`] rebuilds only the roots whose files changed.
//!
//! Extension point: [`IdentitySource`]. A source says which identities a
//! member publishes and which identities each of its files consumes. The four
//! package ecosystems are built in; proto and OpenAPI contracts are a later
//! source (publish a `proto:<package>` key from the `.proto` file, consume it
//! from the generated-code imports) added through [`JoinOptions::sources`]
//! with no change to the join or the queries.

use crate::index::{BuildOptions, Index};
use crate::lang::Lang;
use crate::query::{
    Direction, ImpactOptions, ImpactResult, SearchHit, SearchOptions, Target, TraceOptions,
    TraceResult,
};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Name of the workspace file.
pub const WORKSPACE_FILE: &str = "repomap.workspace.toml";

type Node = (usize, u32);

/// Cross-repo hops followed by `impact` (repo A to B to C ...).
const MAX_REPO_HOPS: usize = 3;

// ------------------------------------------------------------ config file

/// The roots a `repomap.workspace.toml` lists, resolved against its directory.
///
/// Accepted form (a minimal TOML subset, no dependency on a TOML crate):
/// a top-level `roots = ["../api", "../web"]`, over one or several lines (a
/// `roots` key inside a `[table]` is not read).
pub fn read_workspace_file(file: &Path) -> Result<Vec<PathBuf>> {
    let text =
        std::fs::read_to_string(file).with_context(|| format!("cannot read {}", file.display()))?;
    let base = file.parent().unwrap_or(Path::new("."));
    let mut roots = Vec::new();
    let mut in_roots = false;
    let mut in_table = false;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if !in_roots && line.starts_with('[') {
            in_table = true;
        }
        if in_table {
            continue;
        }
        let rest = if in_roots {
            line
        } else if let Some(r) = line.strip_prefix("roots") {
            let r = r.trim_start();
            let Some(r) = r.strip_prefix('=') else {
                continue;
            };
            in_roots = true;
            r
        } else {
            continue;
        };
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
            match c {
                '"' | '\'' => {
                    let s: String = chars.by_ref().take_while(|x| *x != c).collect();
                    roots.push(base.join(s));
                }
                ']' => in_roots = false,
                _ => {}
            }
        }
    }
    if roots.is_empty() {
        bail!(
            "{} lists no roots (expected `roots = [\"../a\", \"../b\"]`)",
            file.display()
        );
    }
    Ok(roots)
}

/// The nearest `repomap.workspace.toml` at or above `start`.
pub fn find_workspace_file(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|d| d.join(WORKSPACE_FILE))
        .find(|p| p.is_file())
}

// ------------------------------------------------------- identity sources

/// An identity one member publishes: `key` of a `kind` (ecosystem or contract
/// type), living under `dir` (relative to the member root, "" for the root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    pub kind: String,
    pub key: String,
    pub dir: String,
}

/// An import a file makes: the identities it could name, most specific first,
/// each with the path inside the package it reaches (`None`: the whole package).
#[derive(Debug, Clone)]
pub struct Consumed {
    pub file: u32,
    pub candidates: Vec<(String, Option<String>)>,
}

/// Where identities come from. Implement this for proto/OpenAPI contracts.
pub trait IdentitySource: Send + Sync {
    /// The kind published keys carry (matches `Published::kind`).
    fn kind(&self) -> &'static str;
    fn published(&self, member: &Member) -> Vec<Published>;
    fn consumed(&self, member: &Member) -> Vec<Consumed>;
}

/// The built-in package ecosystems.
pub fn builtin_sources() -> Vec<Arc<dyn IdentitySource>> {
    vec![
        Arc::new(Npm),
        Arc::new(CargoCrates),
        Arc::new(GoModules),
        Arc::new(PythonProjects),
    ]
}

fn manifests<'a>(m: &'a Member, name: &'a str) -> impl Iterator<Item = (String, String)> + 'a {
    m.index
        .files
        .iter()
        .filter(move |f| f.path == name || f.path.ends_with(&format!("/{name}")))
        .filter(|f| !f.path.split('/').any(|s| s == "node_modules"))
        .filter_map(move |f| {
            let text = std::fs::read_to_string(m.root.join(&f.path)).ok()?;
            let dir = f.path.rsplit_once('/').map_or("", |(d, _)| d).to_string();
            Some((dir, text))
        })
}

fn import_files<'a>(
    m: &'a Member,
    langs: &'a [Lang],
) -> impl Iterator<Item = (u32, &'a Vec<String>)> + 'a {
    m.index
        .files
        .iter()
        .enumerate()
        .filter(move |(_, f)| f.lang.is_some_and(|l| langs.contains(&l)))
        .map(|(i, _)| (i as u32, &m.index.imports_raw[i]))
}

/// The value of `key = "value"` inside TOML section `[section]` (first match).
fn toml_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut cur = String::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(s) = line.strip_prefix('[') {
            cur = s.trim_end_matches(']').trim().to_string();
            continue;
        }
        if cur == section {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == key {
                    let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

struct Npm;

impl IdentitySource for Npm {
    fn kind(&self) -> &'static str {
        "npm"
    }
    fn published(&self, m: &Member) -> Vec<Published> {
        manifests(m, "package.json")
            .filter_map(|(dir, text)| {
                let v: serde_json::Value = serde_json::from_str(&text).ok()?;
                let name = v.get("name")?.as_str()?.to_string();
                Some(Published {
                    kind: "npm".into(),
                    key: name,
                    dir,
                })
            })
            .collect()
    }
    fn consumed(&self, m: &Member) -> Vec<Consumed> {
        let langs = [Lang::TypeScript, Lang::Tsx, Lang::JavaScript];
        import_files(m, &langs)
            .filter_map(|(file, specs)| {
                let candidates: Vec<_> = specs
                    .iter()
                    .filter(|s| {
                        !s.starts_with('.') && !s.starts_with('/') && !s.starts_with("node:")
                    })
                    .map(|s| {
                        let n = if s.starts_with('@') { 2 } else { 1 };
                        let mut it = s.splitn(n + 1, '/');
                        let name: Vec<&str> = it.by_ref().take(n).collect();
                        (
                            name.join("/"),
                            it.next().filter(|r| !r.is_empty()).map(str::to_string),
                        )
                    })
                    .collect();
                (!candidates.is_empty()).then_some(Consumed { file, candidates })
            })
            .collect()
    }
}

struct CargoCrates;

impl IdentitySource for CargoCrates {
    fn kind(&self) -> &'static str {
        "cargo"
    }
    fn published(&self, m: &Member) -> Vec<Published> {
        manifests(m, "Cargo.toml")
            .filter_map(|(dir, text)| {
                let name = toml_value(&text, "package", "name")?;
                // The import identifier is the `[lib] name`, else the package name.
                let ident = toml_value(&text, "lib", "name").unwrap_or(name);
                Some(Published {
                    kind: "cargo".into(),
                    key: ident.replace('-', "_"),
                    dir,
                })
            })
            .collect()
    }
    fn consumed(&self, m: &Member) -> Vec<Consumed> {
        import_files(m, &[Lang::Rust])
            .filter_map(|(file, specs)| {
                let candidates: Vec<_> = specs
                    .iter()
                    .filter(|s| !s.starts_with("mod:"))
                    .filter_map(|s| {
                        let first = s.trim_start_matches("::").split("::").next()?.trim();
                        let ok = !first.is_empty()
                            && first.chars().all(|c| c.is_alphanumeric() || c == '_');
                        (ok && !matches!(
                            first,
                            "crate" | "self" | "super" | "std" | "core" | "alloc"
                        ))
                        .then(|| (first.to_string(), None))
                    })
                    .collect();
                (!candidates.is_empty()).then_some(Consumed { file, candidates })
            })
            .collect()
    }
}

struct GoModules;

impl IdentitySource for GoModules {
    fn kind(&self) -> &'static str {
        "go"
    }
    fn published(&self, m: &Member) -> Vec<Published> {
        manifests(m, "go.mod")
            .filter_map(|(dir, text)| {
                let line = text
                    .lines()
                    .find_map(|l| l.trim().strip_prefix("module "))?;
                Some(Published {
                    kind: "go".into(),
                    key: line.trim().trim_matches('"').to_string(),
                    dir,
                })
            })
            .collect()
    }
    fn consumed(&self, m: &Member) -> Vec<Consumed> {
        import_files(m, &[Lang::Go])
            .filter_map(|(file, specs)| {
                let mut candidates = Vec::new();
                for s in specs.iter().filter(|s| s.contains('.') && s.contains('/')) {
                    // Longest module-path prefix first: `a.io/m/sub/pkg` -> `a.io/m/sub`, `a.io/m`.
                    let parts: Vec<&str> = s.split('/').collect();
                    for n in (2..=parts.len()).rev() {
                        let rest = parts[n..].join("/");
                        candidates.push((parts[..n].join("/"), (!rest.is_empty()).then_some(rest)));
                    }
                }
                (!candidates.is_empty()).then_some(Consumed { file, candidates })
            })
            .collect()
    }
}

struct PythonProjects;

fn py_norm(s: &str) -> String {
    s.to_ascii_lowercase().replace(['-', '.'], "_")
}

impl IdentitySource for PythonProjects {
    fn kind(&self) -> &'static str {
        "python"
    }
    fn published(&self, m: &Member) -> Vec<Published> {
        let mut out = Vec::new();
        for (dir, text) in manifests(m, "pyproject.toml") {
            let Some(name) = toml_value(&text, "project", "name")
                .or_else(|| toml_value(&text, "tool.poetry", "name"))
            else {
                continue;
            };
            out.push(Published {
                kind: "python".into(),
                key: py_norm(&name),
                dir: dir.clone(),
            });
            // The import name often differs: a top-level package dir (also under `src/`).
            for base in [
                dir.clone(),
                if dir.is_empty() {
                    "src".to_string()
                } else {
                    format!("{dir}/src")
                },
            ] {
                let prefix = if base.is_empty() {
                    String::new()
                } else {
                    format!("{base}/")
                };
                let mut seen = BTreeSet::new();
                for f in &m.index.files {
                    if let Some(rest) = f.path.strip_prefix(&prefix) {
                        if let Some((pkg, "__init__.py")) = rest
                            .split_once('/')
                            .and_then(|(p, r)| (!r.contains('/')).then_some((p, r)))
                        {
                            if seen.insert(pkg.to_string()) {
                                out.push(Published {
                                    kind: "python".into(),
                                    key: py_norm(pkg),
                                    dir: format!("{prefix}{pkg}"),
                                });
                            }
                        }
                    }
                }
            }
        }
        out
    }
    fn consumed(&self, m: &Member) -> Vec<Consumed> {
        import_files(m, &[Lang::Python])
            .filter_map(|(file, specs)| {
                let candidates: Vec<_> = specs
                    .iter()
                    .filter(|s| !s.starts_with('.') && !s.is_empty())
                    .map(|s| {
                        let mut it = s.split('.');
                        let top = py_norm(it.next().unwrap_or(""));
                        let rest: Vec<&str> = it.collect();
                        (top, (!rest.is_empty()).then(|| rest.join("/")))
                    })
                    .collect();
                (!candidates.is_empty()).then_some(Consumed { file, candidates })
            })
            .collect()
    }
}

// ----------------------------------------------------------------- graph

/// One repository in the workspace.
pub struct Member {
    /// Short name (the root directory name, made unique).
    pub name: String,
    pub root: PathBuf,
    pub index: Arc<Index>,
}

/// A published identity, owned by a member.
#[derive(Debug, Clone, Serialize)]
pub struct Package {
    pub kind: String,
    pub key: String,
    pub repo: usize,
    pub dir: String,
}

/// A file in one repo importing a package another repo publishes.
#[derive(Debug, Clone, Serialize)]
pub struct CrossEdge {
    pub from_repo: usize,
    pub from_file: u32,
    /// Index into `WorkspaceGraph::packages`.
    pub package: usize,
    /// Path inside the package the import reaches, when it names files there.
    pub sub: Option<String>,
}

#[derive(Default)]
struct Scan {
    published: Vec<Published>,
    consumed: Vec<Consumed>,
}

/// Options for [`join_roots`].
#[derive(Default)]
pub struct JoinOptions {
    /// Skip the on-disk per-file fact cache (the default uses it).
    pub no_cache: bool,
    /// Indexes the caller already holds (the CLI/MCP workspace cache), keyed by
    /// canonical root. A root found here is not built again.
    pub prebuilt: HashMap<PathBuf, Arc<Index>>,
    /// Identity sources beyond the built-in package ecosystems.
    pub sources: Vec<Arc<dyn IdentitySource>>,
}

pub struct WorkspaceGraph {
    pub members: Vec<Member>,
    pub packages: Vec<Package>,
    pub edges: Vec<CrossEdge>,
    sources: Vec<Arc<dyn IdentitySource>>,
    scans: Vec<Scan>,
    by_package: HashMap<usize, Vec<usize>>,
    use_cache: bool,
}

fn build_opts(use_cache: bool) -> BuildOptions {
    BuildOptions {
        use_cache,
        ..Default::default()
    }
}

/// Join the graphs of `roots` into one workspace graph.
///
/// Each root's index comes from `opts.prebuilt` or `Index::build` (so its
/// per-file cache applies); the join itself only reads manifests and the
/// import specifiers those indexes already hold. A single root is valid and
/// yields a graph with no cross-repo edges.
pub fn join_roots(roots: &[PathBuf], opts: &JoinOptions) -> Result<WorkspaceGraph> {
    if roots.is_empty() {
        bail!("workspace has no roots");
    }
    let mut members: Vec<Member> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for r in roots {
        let root = r
            .canonicalize()
            .with_context(|| format!("cannot open {}", r.display()))?;
        if !seen.insert(root.clone()) {
            continue;
        }
        let index = match opts.prebuilt.get(&root) {
            Some(i) => i.clone(),
            None => Arc::new(Index::build(&root, &build_opts(!opts.no_cache))?),
        };
        let base = root
            .file_name()
            .map_or("repo".to_string(), |n| n.to_string_lossy().to_string());
        let mut name = base.clone();
        let mut n = 2;
        while members.iter().any(|m| m.name == name) {
            name = format!("{base}-{n}");
            n += 1;
        }
        members.push(Member { name, root, index });
    }
    let mut sources = builtin_sources();
    sources.extend(opts.sources.iter().cloned());
    let mut g = WorkspaceGraph {
        scans: members.iter().map(|_| Scan::default()).collect(),
        members,
        packages: Vec::new(),
        edges: Vec::new(),
        sources,
        by_package: HashMap::new(),
        use_cache: !opts.no_cache,
    };
    for i in 0..g.members.len() {
        g.scans[i] = g.scan(i);
    }
    g.link();
    Ok(g)
}

impl WorkspaceGraph {
    fn scan(&self, repo: usize) -> Scan {
        let m = &self.members[repo];
        let mut s = Scan::default();
        for src in &self.sources {
            s.published.extend(src.published(m));
            // Consumed identities are tagged with the source kind via a prefix.
            for mut c in src.consumed(m) {
                for cand in &mut c.candidates {
                    cand.0 = format!("{}\u{0}{}", src.kind(), cand.0);
                }
                s.consumed.push(c);
            }
        }
        s
    }

    /// Recompute packages and cross-repo edges from the stored scans.
    fn link(&mut self) {
        self.packages.clear();
        self.edges.clear();
        self.by_package.clear();
        let mut ix: HashMap<String, usize> = HashMap::new();
        for (repo, s) in self.scans.iter().enumerate() {
            for p in &s.published {
                let id = format!("{}\u{0}{}", p.kind, p.key);
                // First repo listed wins a duplicate identity.
                ix.entry(id).or_insert_with(|| {
                    self.packages.push(Package {
                        kind: p.kind.clone(),
                        key: p.key.clone(),
                        repo,
                        dir: p.dir.clone(),
                    });
                    self.packages.len() - 1
                });
            }
        }
        let mut seen: HashSet<(usize, u32, usize, Option<String>)> = HashSet::new();
        let mut sub_ok: HashMap<(usize, String), bool> = HashMap::new();
        let mut edges = Vec::new();
        for (repo, s) in self.scans.iter().enumerate() {
            for c in &s.consumed {
                // The first (most specific) candidate that names a foreign package wins.
                for (id, sub) in &c.candidates {
                    let Some(&pkg) = ix.get(id) else { continue };
                    if self.packages[pkg].repo == repo {
                        continue;
                    }
                    let sub = sub.clone().filter(|sub| {
                        *sub_ok
                            .entry((pkg, sub.clone()))
                            .or_insert_with(|| !self.package_files(pkg, Some(sub)).is_empty())
                    });
                    if seen.insert((repo, c.file, pkg, sub.clone())) {
                        edges.push(CrossEdge {
                            from_repo: repo,
                            from_file: c.file,
                            package: pkg,
                            sub,
                        });
                    }
                    break;
                }
            }
        }
        for (i, e) in edges.iter().enumerate() {
            self.by_package.entry(e.package).or_default().push(i);
        }
        self.edges = edges;
    }

    /// Rebuild only the members whose files changed on disk, then re-join.
    /// Returns how many members were re-indexed. An unchanged member costs a
    /// stat walk (`index::fingerprint`), never a parse.
    pub fn refresh(&mut self) -> Result<usize> {
        let mut rebuilt = 0;
        for i in 0..self.members.len() {
            let opts = build_opts(self.use_cache);
            let m = &self.members[i];
            let fresh = crate::index::fingerprint(&m.root, &opts).ok() == Some(m.index.fingerprint)
                && m.index.model_id == crate::semantic::model_id();
            if fresh {
                continue;
            }
            let root = m.root.clone();
            self.members[i].index = Arc::new(Index::build(&root, &opts)?);
            self.scans[i] = self.scan(i);
            rebuilt += 1;
        }
        if rebuilt > 0 {
            self.link();
        }
        Ok(rebuilt)
    }

    /// The member named `name` or rooted at `root`.
    pub fn member(&self, name_or_root: &str) -> Option<usize> {
        let canon = Path::new(name_or_root).canonicalize().ok();
        self.members
            .iter()
            .position(|m| m.name == name_or_root || Some(&m.root) == canon.as_ref())
    }

    /// Files of `pkg` that `sub` names (all code files under the package dir when `None`).
    fn package_files(&self, pkg: usize, sub: Option<&String>) -> Vec<u32> {
        let p = &self.packages[pkg];
        let idx = &self.members[p.repo].index;
        let prefix = if p.dir.is_empty() {
            String::new()
        } else {
            format!("{}/", p.dir)
        };
        idx.code_files()
            .filter(|(_, f)| {
                f.path
                    .strip_prefix(&prefix)
                    .is_some_and(|rel| sub.is_none_or(|s| sub_matches(rel, s)))
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Cross-repo files that import something living in `repo:file`.
    pub fn importers_of(&self, repo: usize, file: u32) -> Vec<&CrossEdge> {
        let path = &self.members[repo].index.files[file as usize].path;
        let mut deepest: HashMap<&str, usize> = HashMap::new();
        for (i, p) in self.packages.iter().enumerate() {
            let inside =
                p.repo == repo && (p.dir.is_empty() || path.starts_with(&format!("{}/", p.dir)));
            if inside
                && deepest
                    .get(p.kind.as_str())
                    .is_none_or(|&j| self.packages[j].dir.len() < p.dir.len())
            {
                deepest.insert(p.kind.as_str(), i);
            }
        }
        let mut out = Vec::new();
        for pkg in deepest.values() {
            let dir = &self.packages[*pkg].dir;
            let rel = path.strip_prefix(&format!("{dir}/")).unwrap_or(path);
            for &e in self.by_package.get(pkg).into_iter().flatten() {
                let e = &self.edges[e];
                if e.sub.as_ref().is_none_or(|s| sub_matches(rel, s)) {
                    out.push(e);
                }
            }
        }
        out
    }

    fn label(&self, repo: usize, file: u32) -> String {
        format!(
            "{}:{}",
            self.members[repo].name, self.members[repo].index.files[file as usize].path
        )
    }

    /// Split an optional `repo:` prefix from a target; unprefixed targets
    /// resolve in the first member (the home repo) that knows them.
    fn locate(&self, q: &str) -> Result<(usize, String), String> {
        if let Some((p, rest)) = q.split_once(':') {
            if let Some(i) = self.members.iter().position(|m| m.name == p) {
                if !rest.is_empty() {
                    return self.members[i]
                        .index
                        .resolve(rest)
                        .map(|_| (i, rest.to_string()));
                }
            }
        }
        let mut first_err = None;
        for (i, m) in self.members.iter().enumerate() {
            match m.index.resolve(q) {
                Ok(_) => return Ok((i, q.to_string())),
                Err(e) => first_err = first_err.or(Some(e)),
            }
        }
        Err(first_err.unwrap_or_else(|| "empty workspace".into()))
    }

    fn file_of(&self, repo: usize, q: &str) -> Option<u32> {
        let idx = &self.members[repo].index;
        match idx.resolve(q).ok()?.0 {
            Target::File(f) => Some(f),
            Target::Symbol(s) => Some(idx.symbols[s as usize].file),
        }
    }
}

/// `rel` (a file path inside a package) is the file or directory `sub` names.
fn sub_matches(rel: &str, sub: &str) -> bool {
    let sub = sub.trim_matches('/');
    let stem = rel.rsplit_once('.').map_or(rel, |(s, _)| s);
    stem == sub || rel.starts_with(&format!("{sub}/")) || stem.starts_with(&format!("{sub}/"))
}

// ---------------------------------------------------------------- search

#[derive(Debug, Serialize)]
pub struct WorkspaceHit {
    pub repo: String,
    pub hit: SearchHit,
    /// Other repos that import the package this hit lives in.
    pub used_by: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceSearch {
    pub query: String,
    pub hits: Vec<WorkspaceHit>,
}

impl WorkspaceGraph {
    /// Search every member and merge by score (BM25 scores come from separate
    /// indexes, so ties between repos are broken by repo order).
    pub fn search(&self, query: &str, opts: &SearchOptions) -> WorkspaceSearch {
        let mut hits = Vec::new();
        for (r, m) in self.members.iter().enumerate() {
            let o = SearchOptions {
                limit: opts.limit,
                path: opts.path.clone(),
                kind: opts.kind.clone(),
                snippet_lines: opts.snippet_lines,
                include_tests: opts.include_tests,
            };
            for hit in m.index.search(query, &o).hits {
                let used_by = m
                    .index
                    .path_ix
                    .get(&hit.file)
                    .map(|&f| {
                        self.importers_of(r, f)
                            .iter()
                            .map(|e| self.members[e.from_repo].name.clone())
                            .collect::<BTreeSet<_>>()
                            .into_iter()
                            .collect()
                    })
                    .unwrap_or_default();
                hits.push(WorkspaceHit {
                    repo: m.name.clone(),
                    hit,
                    used_by,
                });
            }
        }
        hits.sort_by(|a, b| {
            b.hit
                .score
                .partial_cmp(&a.hit.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(opts.limit);
        WorkspaceSearch {
            query: query.to_string(),
            hits,
        }
    }
}

impl WorkspaceSearch {
    pub fn text(&self) -> String {
        let mut o = format!("# Workspace search: {}\n", self.query);
        for h in &self.hits {
            let _ = write!(
                o,
                "\n{}:{}:{}-{} (score {:.2})",
                h.repo, h.hit.file, h.hit.start, h.hit.end, h.hit.score
            );
            if !h.used_by.is_empty() {
                let _ = write!(o, "  [used by {}]", h.used_by.join(", "));
            }
            if let Some(s) = &h.hit.symbol {
                let _ = write!(o, "\n  {} {}", s.kind, s.name);
            }
        }
        o.push('\n');
        o
    }
}

// ---------------------------------------------------------------- impact

#[derive(Debug, Serialize)]
pub struct Dependent {
    pub repo: String,
    pub file: String,
    /// The package the dependent imports (`kind:key`), for a direct cross-repo import.
    pub via: String,
    /// 1 for a file importing the changed repo directly; higher for files
    /// reached through further cross-repo hops.
    pub hop: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct RepoImpact {
    pub direct: Vec<Dependent>,
    /// Files in this repo affected, directly or through its own dependents.
    pub files: Vec<String>,
    pub tests: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceImpact {
    /// The repo the change is in.
    pub home: String,
    /// That repo's own impact, unchanged.
    pub local: ImpactResult,
    /// Other repos affected, by repo name.
    pub repos: BTreeMap<String, RepoImpact>,
    /// Local risk, raised when other repos depend on the change.
    pub risk: &'static str,
}

impl WorkspaceGraph {
    /// Impact of changing `targets` (files or symbols, optionally `repo:`-prefixed;
    /// all in one repo) across the whole workspace.
    pub fn impact(
        &self,
        targets: &[String],
        opts: &ImpactOptions,
    ) -> Result<WorkspaceImpact, String> {
        let mut home = None;
        let mut local_targets = Vec::new();
        for t in targets {
            let (r, local) = self.locate(t)?;
            if *home.get_or_insert(r) != r {
                return Err("targets must be in one repo; run impact once per repo".into());
            }
            local_targets.push(local);
        }
        let home = home.ok_or("no targets")?;
        let idx = &self.members[home].index;
        let local = idx.impact(&local_targets, opts)?;
        let mut start: BTreeSet<u32> = local_targets
            .iter()
            .filter_map(|t| self.file_of(home, t))
            .collect();
        Ok(self.spread(home, local, &mut start, opts))
    }

    /// Impact of the working-tree git diff of the `home` repo (against `base`),
    /// across the workspace.
    pub fn impact_changed(
        &self,
        home: usize,
        base: Option<&str>,
        opts: &ImpactOptions,
    ) -> Result<WorkspaceImpact, String> {
        let idx = &self.members[home].index;
        let local = idx.impact_changed(base, opts)?;
        let mut start: BTreeSet<u32> = local
            .targets
            .iter()
            .filter_map(|l| idx.path_ix.get(label_file(l)).copied())
            .collect();
        Ok(self.spread(home, local, &mut start, opts))
    }

    /// Follow `local`'s affected files, and `start`, across repo boundaries.
    fn spread(
        &self,
        home: usize,
        local: ImpactResult,
        start: &mut BTreeSet<u32>,
        opts: &ImpactOptions,
    ) -> WorkspaceImpact {
        let idx = &self.members[home].index;
        start.extend(
            local
                .files
                .iter()
                .filter_map(|p| idx.path_ix.get(p).copied()),
        );
        let mut visited: HashSet<(usize, u32)> = start.iter().map(|&f| (home, f)).collect();
        let mut frontier: Vec<(usize, u32)> = visited.iter().copied().collect();
        let mut repos: BTreeMap<String, RepoImpact> = BTreeMap::new();
        for hop in 1..=MAX_REPO_HOPS {
            let mut next = Vec::new();
            for (r, f) in frontier {
                for e in self.importers_of(r, f) {
                    if !visited.insert((e.from_repo, e.from_file)) {
                        continue;
                    }
                    let dep = &self.members[e.from_repo];
                    let p = &self.packages[e.package];
                    let path = dep.index.files[e.from_file as usize].path.clone();
                    let entry = repos.entry(dep.name.clone()).or_default();
                    entry.direct.push(Dependent {
                        repo: dep.name.clone(),
                        file: path.clone(),
                        via: format!("{}:{}", p.kind, p.key),
                        hop,
                    });
                    // The dependent's own blast radius inside its repo.
                    next.push((e.from_repo, e.from_file));
                    if let Ok(li) = dep.index.impact(std::slice::from_ref(&path), opts) {
                        entry.tests.extend(li.tests);
                        for lf in li.files.into_iter().chain([path]) {
                            if let Some(&id) = dep.index.path_ix.get(&lf) {
                                if visited.insert((e.from_repo, id)) {
                                    next.push((e.from_repo, id));
                                }
                                if !entry.files.contains(&lf) {
                                    entry.files.push(lf);
                                }
                            }
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        for r in repos.values_mut() {
            r.files.sort();
            r.tests.sort();
            r.tests.dedup();
        }
        let direct: usize = repos.values().map(|r| r.direct.len()).sum();
        let risk = match (local.risk, direct) {
            (r, d) if d >= 10 => bump(r, "high"),
            (r, d) if d >= 3 || repos.len() >= 2 => bump(r, "medium"),
            (r, _) => r,
        };
        WorkspaceImpact {
            home: self.members[home].name.clone(),
            local,
            repos,
            risk,
        }
    }
}

/// The file in an impact target label: a path, or `kind name (path:line)`.
fn label_file(l: &str) -> &str {
    match (l.rfind('('), l.ends_with(')')) {
        (Some(i), true) => l[i + 1..l.len() - 1]
            .rsplit_once(':')
            .map_or(&l[i + 1..l.len() - 1], |(p, _)| p),
        _ => l,
    }
}

fn bump(cur: &'static str, floor: &'static str) -> &'static str {
    let rank = |s: &str| match s {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    };
    if rank(cur) >= rank(floor) {
        cur
    } else {
        floor
    }
}

impl WorkspaceImpact {
    pub fn text(&self) -> String {
        let mut o = format!(
            "# Workspace impact ({} risk; {} in repo `{}`)\n\n",
            self.risk.to_uppercase(),
            self.local.risk,
            self.home
        );
        o.push_str(&self.local.text());
        if self.repos.is_empty() {
            o.push_str("\n\nNo other repo in the workspace depends on this.\n");
            return o;
        }
        let _ = writeln!(o, "\n\n## Other repos affected ({})", self.repos.len());
        for (name, r) in &self.repos {
            let _ = writeln!(
                o,
                "\n### {name}: {} importing file(s), {} affected file(s)",
                r.direct.len(),
                r.files.len()
            );
            for d in &r.direct {
                let _ = writeln!(o, "- {} imports {} (hop {})", d.file, d.via, d.hop);
            }
            if !r.tests.is_empty() {
                let _ = writeln!(o, "Tests to run in {name}: {}", r.tests.join(", "));
            }
        }
        o
    }
}

// ----------------------------------------------------------------- trace

#[derive(Debug, Serialize)]
pub struct WorkspaceHop {
    pub from: String,
    pub to: String,
    /// `imports` inside a repo, `cross-repo import` across repos.
    pub via: &'static str,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceTrace {
    pub from: String,
    pub to: Option<String>,
    pub found: bool,
    pub level: &'static str,
    /// The single-repo trace, when it answered the question.
    pub local: Option<TraceResult>,
    /// File-level path across repos, when the repos had to be crossed.
    pub hops: Vec<WorkspaceHop>,
    /// Without `to`: the other-repo files this one reaches or is reached from.
    pub crossings: Vec<WorkspaceHop>,
    pub note: Option<String>,
}

impl WorkspaceGraph {
    /// Local edges plus cross-repo edges, from one file node. `forward`: the
    /// files it depends on; otherwise the files that depend on it.
    fn neighbours(&self, (r, f): (usize, u32), forward: bool) -> Vec<((usize, u32), &'static str)> {
        let idx = &self.members[r].index;
        let mut out = Vec::new();
        let (list, pick): (&Vec<u32>, fn(&crate::index::FileEdge) -> u32) = if forward {
            (&idx.file_out[f as usize], |e| e.to)
        } else {
            (&idx.file_in[f as usize], |e| e.from)
        };
        for &e in list {
            out.push(((r, pick(&idx.file_edges[e as usize])), "imports"));
        }
        if forward {
            for e in self
                .edges
                .iter()
                .filter(|e| e.from_repo == r && e.from_file == f)
            {
                for t in self.package_files(e.package, e.sub.as_ref()) {
                    out.push(((self.packages[e.package].repo, t), "cross-repo import"));
                }
            }
        } else {
            for e in self.importers_of(r, f) {
                out.push(((e.from_repo, e.from_file), "cross-repo import"));
            }
        }
        out
    }

    fn bfs(&self, from: (usize, u32), to: (usize, u32)) -> Option<Vec<WorkspaceHop>> {
        let mut prev: HashMap<Node, (Node, &'static str)> = HashMap::new();
        let mut q = VecDeque::from([from]);
        let mut seen: HashSet<(usize, u32)> = HashSet::from([from]);
        while let Some(n) = q.pop_front() {
            if n == to {
                let mut hops = Vec::new();
                let mut cur = to;
                while let Some(&(p, via)) = prev.get(&cur) {
                    hops.push(WorkspaceHop {
                        from: self.label(p.0, p.1),
                        to: self.label(cur.0, cur.1),
                        via,
                    });
                    cur = p;
                }
                hops.reverse();
                return Some(hops);
            }
            for (m, via) in self.neighbours(n, true) {
                if seen.insert(m) {
                    prev.insert(m, (n, via));
                    q.push_back(m);
                }
            }
        }
        None
    }

    /// Trace from `from` to `to`, or the neighbours of `from` when `to` is
    /// `None`. A path inside one repo is answered by that repo's own trace;
    /// otherwise the search runs over file imports and cross-repo imports.
    pub fn trace(
        &self,
        from: &str,
        to: Option<&str>,
        opts: &TraceOptions,
    ) -> Result<WorkspaceTrace, String> {
        let (fr, fl) = self.locate(from)?;
        let mut t = WorkspaceTrace {
            from: format!("{}:{}", self.members[fr].name, fl),
            to: None,
            found: false,
            level: "none",
            local: None,
            hops: vec![],
            crossings: vec![],
            note: None,
        };
        let Some(to) = to else {
            let local = self.members[fr].index.trace(&fl, None, opts)?;
            if let Some(file) = self.file_of(fr, &fl) {
                let forward = opts.direction == Direction::Callees;
                for (n, via) in self.neighbours((fr, file), forward) {
                    if n.0 != fr {
                        let (a, b) = (self.label(fr, file), self.label(n.0, n.1));
                        t.crossings.push(if forward {
                            WorkspaceHop {
                                from: a,
                                to: b,
                                via,
                            }
                        } else {
                            WorkspaceHop {
                                from: b,
                                to: a,
                                via,
                            }
                        });
                    }
                }
            }
            t.found = true;
            t.level = "calls";
            t.local = Some(local);
            return Ok(t);
        };
        let (tr, tl) = self.locate(to)?;
        t.to = Some(format!("{}:{}", self.members[tr].name, tl));
        if fr == tr {
            if let Ok(local) = self.members[fr].index.trace(&fl, Some(&tl), opts) {
                if local.found {
                    t.found = true;
                    t.level = local.level;
                    t.local = Some(local);
                    return Ok(t);
                }
            }
        }
        let (Some(a), Some(b)) = (self.file_of(fr, &fl), self.file_of(tr, &tl)) else {
            t.note = Some("could not resolve a file for one end".into());
            return Ok(t);
        };
        if let Some(h) = self.bfs((fr, a), (tr, b)) {
            t.found = true;
            t.level = "files";
            t.hops = h;
        } else if let Some(h) = self.bfs((tr, b), (fr, a)) {
            t.found = true;
            t.level = "files";
            t.hops = h;
            t.note = Some("No path in that direction; this is the reverse path (the target reaches the source).".into());
        } else {
            t.note = Some("No import path found, within or across repos (dynamic dispatch, contracts and config wiring are not followed).".into());
        }
        Ok(t)
    }
}

impl WorkspaceTrace {
    pub fn text(&self) -> String {
        let mut o = format!("# Workspace trace: {}", self.from);
        if let Some(to) = &self.to {
            let _ = write!(o, " -> {to}");
        }
        o.push('\n');
        if let Some(l) = &self.local {
            o.push_str(&l.text());
        }
        if !self.hops.is_empty() {
            let _ = writeln!(o, "\nPath ({}):", self.level);
            for h in &self.hops {
                let _ = writeln!(o, "- {} -> {} ({})", h.from, h.to, h.via);
            }
        }
        if !self.crossings.is_empty() {
            o.push_str("\nAcross repos:\n");
            for h in &self.crossings {
                let _ = writeln!(o, "- {} -> {} ({})", h.from, h.to, h.via);
            }
        }
        if let Some(n) = &self.note {
            let _ = writeln!(o, "\n{n}");
        }
        o
    }
}
