//! The shared map: a source-free copy of an index that a team can share.
//!
//! `Index::to_shared` writes only names, paths, line numbers and the resolved
//! graph. It never writes source text, declaration lines (`signature`), the
//! BM25 postings (built from code bodies), embeddings, comments or string
//! literals. `Index::from_shared` rebuilds a queryable index from that
//! document: ranks and adjacency are recomputed with the same code as a local
//! index, and search runs over names and paths only.

use crate::bm25::Bm25;
use crate::graph;
use crate::index::{Community, FileEdge, FileEntry, GitInfo, Index, Role, Stats, SymEdge, Symbol};
use crate::lang::Lang;
use crate::parse::{Chunk, FileFacts, Kind};
use crate::tokenize::tokenize;
use crate::workspace_graph::{Consumed, Published};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// The document format this build writes and reads.
pub const FORMAT: u32 = 1;
pub const MEDIA_TYPE: &str = "application/vnd.repomap.map+json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedMap {
    pub format: u32,
    pub repomap_version: String,
    pub commit: Option<String>,
    pub default_branch: Option<String>,
    pub web: Option<String>,
    pub files: Vec<SharedFile>,
    pub symbols: Vec<SharedSymbol>,
    pub sym_edges: Vec<SymEdge>,
    pub file_edges: Vec<FileEdge>,
    /// Import specifiers per file (module paths and package names), parallel to `files`.
    pub imports: Vec<Vec<String>>,
    pub communities: Vec<Community>,
    pub packages: SharedPackages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFile {
    pub path: String,
    pub lang: Option<Lang>,
    pub lines: u32,
    pub is_test: bool,
    pub role: Role,
}

/// A symbol without its `signature` (a declaration line is source).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedSymbol {
    pub name: String,
    pub owner: Option<String>,
    pub kind: Kind,
    pub file: u32,
    pub start: u32,
    pub end: u32,
    pub parent: Option<u32>,
}

/// Package identities, computed from manifests when the map is made so a
/// reader never needs the manifest text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedPackages {
    pub published: Vec<SharedPublished>,
    pub consumed: Vec<SharedConsumed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedPublished {
    pub kind: String,
    pub key: String,
    pub dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedConsumed {
    pub file: u32,
    pub kind: String,
    /// (identity, path inside the package), most specific first.
    pub candidates: Vec<(String, Option<String>)>,
}

impl SharedPackages {
    pub fn published_list(&self) -> Vec<Published> {
        self.published
            .iter()
            .map(|p| Published { kind: p.kind.clone(), key: p.key.clone(), dir: p.dir.clone() })
            .collect()
    }

    /// Consumed identities of one source kind, as the join reads them.
    pub fn consumed_of(&self, kind: &str) -> Vec<Consumed> {
        self.consumed
            .iter()
            .filter(|c| c.kind == kind)
            .map(|c| Consumed { file: c.file, candidates: c.candidates.clone() })
            .collect()
    }
}

/// What an index that came from a shared map remembers about its origin.
#[derive(Debug, Clone)]
pub struct SharedInfo {
    pub owner: String,
    pub repo: String,
    pub commit: Option<String>,
    pub packages: SharedPackages,
}

impl SharedInfo {
    /// A GitHub permalink to `path` (and lines), so a reader with GitHub
    /// access reads the source there.
    pub fn permalink(&self, web: Option<&str>, path: &str, lines: Option<(u32, u32)>) -> String {
        let base = web.map(String::from).unwrap_or_else(|| format!("https://github.com/{}/{}", self.owner, self.repo));
        let rev = self.commit.as_deref().unwrap_or("HEAD");
        let mut enc = String::with_capacity(path.len());
        for c in path.chars() {
            match c {
                ' ' => enc.push_str("%20"),
                '#' => enc.push_str("%23"),
                '?' => enc.push_str("%3F"),
                '%' => enc.push_str("%25"),
                c => enc.push(c),
            }
        }
        match lines {
            Some((s, e)) => format!("{base}/blob/{rev}/{enc}#L{s}-L{e}"),
            None => format!("{base}/blob/{rev}/{enc}"),
        }
    }
}

/// `https://host/m/{owner}/{repo}` split into its parts. Syntax only: which
/// hosts are trusted is decided by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// `scheme://host[:port]`
    pub base: String,
    pub owner: String,
    pub repo: String,
}

/// A root or workspace entry that is a URL rather than a directory.
pub fn is_link(s: &str) -> bool {
    s.starts_with("https://") || s.starts_with("http://")
}

pub fn parse_link(s: &str) -> Option<Link> {
    let (scheme, rest) = s.trim().split_once("://")?;
    if scheme != "https" && scheme != "http" {
        return None;
    }
    let rest = rest.split(['?', '#']).next()?;
    let (host, path) = rest.split_once('/')?;
    let mut seg = path.trim_end_matches('/').split('/');
    if seg.next()? != "m" {
        return None;
    }
    let (owner, repo) = (seg.next()?, seg.next()?);
    let ok = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if seg.next().is_some() || host.is_empty() || host.contains('@') || !ok(owner) || !ok(repo) {
        return None;
    }
    Some(Link { base: format!("{scheme}://{host}"), owner: owner.to_string(), repo: repo.to_string() })
}

impl Index {
    /// The source-free document for this index. `packages` comes from
    /// `workspace_graph::shared_packages`, which reads the manifests once.
    pub fn to_shared(&self, version: &str, packages: SharedPackages) -> SharedMap {
        SharedMap {
            format: FORMAT,
            repomap_version: version.to_string(),
            commit: self.git.commit.clone(),
            default_branch: self.git.branch.clone().filter(|b| b != "HEAD"),
            web: self.git.remote_web.clone(),
            files: self
                .files
                .iter()
                .map(|f| SharedFile { path: f.path.clone(), lang: f.lang, lines: f.lines, is_test: f.is_test, role: f.role })
                .collect(),
            symbols: self
                .symbols
                .iter()
                .map(|s| SharedSymbol { name: s.name.clone(), owner: s.owner.clone(), kind: s.kind, file: s.file, start: s.start, end: s.end, parent: s.parent })
                .collect(),
            sym_edges: self.sym_edges.clone(),
            file_edges: self.file_edges.clone(),
            imports: self.imports_raw.clone(),
            communities: self.communities.clone(),
            packages,
        }
    }

    /// Rebuild a queryable index from a shared map. Every index in the
    /// document is checked, so a corrupt or hostile map is an error and never
    /// a panic.
    pub fn from_shared(map: SharedMap, owner: &str, repo: &str) -> Result<Index, String> {
        if map.format != FORMAT {
            return Err(format!(
                "this shared map is format {}, and this repomap reads format {FORMAT}; update repomap",
                map.format
            ));
        }
        let n = map.files.len();
        let ns = map.symbols.len();
        let bad = |what: &str| Err(format!("the shared map is damaged ({what})"));
        if map.imports.len() > n {
            return bad("imports");
        }
        // Symbols are grouped by file, in file order, as `Index` keeps them.
        let mut sym_range = vec![(0u32, 0u32); n];
        let mut prev = 0u32;
        let mut symbols = Vec::with_capacity(ns);
        for (i, s) in map.symbols.iter().enumerate() {
            if s.file as usize >= n || s.file < prev || s.parent.is_some_and(|p| p as usize >= ns) || s.end < s.start {
                return bad("symbols");
            }
            prev = s.file;
            let r = &mut sym_range[s.file as usize];
            if r.1 == 0 {
                r.0 = i as u32;
            }
            r.1 = i as u32 + 1;
            symbols.push(Symbol {
                name: s.name.clone(),
                kind: s.kind,
                file: s.file,
                start: s.start,
                end: s.end,
                owner: s.owner.clone(),
                parent: s.parent,
                signature: String::new(),
            });
        }
        if map.sym_edges.iter().any(|e| e.from as usize >= ns || e.to as usize >= ns)
            || map.file_edges.iter().any(|e| e.from as usize >= n || e.to as usize >= n)
            || map.communities.iter().any(|c| c.files.iter().any(|f| *f as usize >= n))
            || map.packages.consumed.iter().any(|c| c.file as usize >= n)
        {
            return bad("edges");
        }
        let mut files = Vec::with_capacity(n);
        let mut path_ix = HashMap::with_capacity(n);
        for (i, f) in map.files.iter().enumerate() {
            let (a, b) = sym_range[i];
            let (a, b) = if b == 0 { (0, 0) } else { (a, b) };
            files.push(FileEntry {
                path: f.path.clone(),
                lang: f.lang,
                lines: f.lines,
                bytes: 0,
                is_test: f.is_test,
                role: f.role,
                sym_start: a,
                sym_end: b,
            });
            path_ix.insert(f.path.clone(), i as u32);
        }
        let mut by_name: HashMap<String, Vec<u32>> = HashMap::new();
        for (i, s) in symbols.iter().enumerate() {
            by_name.entry(s.name.clone()).or_default().push(i as u32);
        }
        let names_lower: Vec<String> = symbols.iter().map(|s| s.name.to_ascii_lowercase()).collect();
        let bm25 = names_bm25(&files, &symbols);
        let mut imports_raw = map.imports;
        imports_raw.resize(n, Vec::new());
        let info = SharedInfo { owner: owner.to_string(), repo: repo.to_string(), commit: map.commit.clone(), packages: map.packages };
        let mut index = Index {
            root: PathBuf::from(format!("{owner}/{repo}")),
            files,
            path_ix,
            symbols,
            sym_edges: Vec::new(),
            sym_out: Vec::new(),
            sym_in: Vec::new(),
            file_edges: Vec::new(),
            file_out: Vec::new(),
            file_in: Vec::new(),
            imports_raw,
            file_rank: Vec::new(),
            sym_rank: Vec::new(),
            community: vec![u32::MAX; n],
            communities: Vec::new(),
            by_name,
            names_lower,
            bm25,
            dense: crate::semantic::Dense::default(),
            stats: Stats::default(),
            git: GitInfo { commit: map.commit, branch: map.default_branch, remote_web: map.web },
            fingerprint: 0,
            model_id: "",
            deferred: Vec::new(),
            shared: Some(info),
        };
        graph::finish_graph(&mut index, map.sym_edges, map.file_edges);
        for (k, c) in map.communities.iter().enumerate() {
            for &f in &c.files {
                index.community[f as usize] = k as u32;
            }
        }
        index.communities = map.communities;
        Ok(index)
    }
}

/// BM25 over names and paths only: one chunk per file (its path words) and one
/// per symbol (its qualified name words). No source text is involved.
fn names_bm25(files: &[FileEntry], symbols: &[Symbol]) -> Bm25 {
    let chunk = |start: u32, end: u32, symbol: Option<u32>, text: &str| {
        let mut terms = tokenize(text);
        terms.sort();
        terms.dedup();
        Chunk { start, end, symbol, tfs: vec![1; terms.len()], len: terms.len() as u32, terms: terms.join(" "), vec: None }
    };
    let facts: Vec<FileFacts> = files
        .iter()
        .map(|f| {
            let mut chunks = vec![chunk(1, f.lines.max(1), None, &f.path)];
            for s in f.sym_start..f.sym_end {
                let sym = &symbols[s as usize];
                chunks.push(chunk(sym.start, sym.end, Some(s - f.sym_start), &sym.qualified()));
            }
            FileFacts { lang: f.lang, lines: f.lines, chunks, ..Default::default() }
        })
        .collect();
    Bm25::build(files.iter().zip(&facts).enumerate().map(|(i, (e, f))| (i as u32, e, f)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links() {
        let l = parse_link("https://review.repomap.sylphx.com/m/acme/api").unwrap();
        assert_eq!((l.base.as_str(), l.owner.as_str(), l.repo.as_str()), ("https://review.repomap.sylphx.com", "acme", "api"));
        assert!(parse_link("https://h.example/m/a/b/").is_some());
        assert!(parse_link("https://h.example/m/a/b?x=1#y").is_some());
        for bad in ["https://h.example/m/a", "https://h.example/x/a/b", "https://h.example/m/a/b/c", "https://u@h.example/m/a/b", "ftp://h/m/a/b", "/m/a/b", "https://h.example/m/a/b c"] {
            assert!(parse_link(bad).is_none(), "{bad}");
        }
        assert!(is_link("http://127.0.0.1:1/m/a/b") && !is_link("../api"));
    }

    #[test]
    fn permalinks_encode_the_path() {
        let i = SharedInfo { owner: "a".into(), repo: "b".into(), commit: Some("abc".into()), packages: Default::default() };
        assert_eq!(i.permalink(None, "src/x y#.ts", Some((3, 9))), "https://github.com/a/b/blob/abc/src/x%20y%23.ts#L3-L9");
        assert_eq!(i.permalink(Some("https://github.com/o/r"), "a.rs", None), "https://github.com/o/r/blob/abc/a.rs");
    }
}
