//! The shared map: no source in it, and the same answers from it.

use repomap_core::query::{ContextOptions, Direction, ImpactOptions, MapOptions, SearchOptions, TraceOptions};
use repomap_core::shared::SharedMap;
use repomap_core::workspace_graph::shared_packages;
use repomap_core::{BuildOptions, Index};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

const SENTINEL: &str = "SENTINEL_q8z7x6w5";

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body.replace("SENTINEL", SENTINEL)).unwrap();
}

fn build(root: &Path) -> Arc<Index> {
    Arc::new(Index::build(root, &BuildOptions { use_cache: false, ..Default::default() }).expect("index"))
}

fn shared_of(idx: &Arc<Index>) -> SharedMap {
    idx.to_shared("test", shared_packages(idx))
}

/// Every language repomap parses, with the sentinel in a function body, a
/// comment, a string literal, a docstring and a default argument.
fn sentinel_fixture(root: &Path) {
    write(root, "src/a.ts", "// SENTINEL in a comment\nexport function greet(name: string = \"SENTINEL default\"): string {\n  const s = `SENTINEL body ${name}`;\n  /* SENTINEL block */\n  return helper(s) + \"SENTINEL literal\";\n}\nfunction helper(x: string) { return x; }\n");
    write(root, "src/b.py", "# SENTINEL comment\ndef compute(a, b=\"SENTINEL default\"):\n    \"\"\"SENTINEL docstring\"\"\"\n    s = 'SENTINEL body'\n    return util(s)\n\ndef util(x):\n    return x\n");
    write(root, "src/c.rs", "// SENTINEL comment\n/// SENTINEL doc\npub fn run(x: &str) -> String {\n    let s = \"SENTINEL literal\";\n    inner(s)\n}\nfn inner(x: &str) -> String { x.to_string() }\n");
    write(root, "src/d.go", "package d\n\n// SENTINEL comment\nfunc Run(x string) string {\n\ts := \"SENTINEL literal\"\n\treturn Inner(s)\n}\n\nfunc Inner(x string) string { return x }\n");
    write(root, "src/E.java", "public class E {\n  // SENTINEL comment\n  public String run(String x) {\n    String s = \"SENTINEL literal\";\n    return inner(s);\n  }\n  private String inner(String x) { return x; }\n}\n");
    write(root, "src/f.rb", "# SENTINEL comment\ndef run(x = 'SENTINEL default')\n  s = \"SENTINEL literal\"\n  inner(s)\nend\n\ndef inner(x)\n  x\nend\n");
    write(root, "src/g.php", "<?php\n// SENTINEL comment\nfunction run($x = 'SENTINEL default') {\n  $s = \"SENTINEL literal\";\n  return inner($s);\n}\nfunction inner($x) { return $x; }\n");
    write(root, "src/H.cs", "class H {\n  // SENTINEL comment\n  string Run(string x = \"SENTINEL default\") {\n    var s = \"SENTINEL literal\";\n    return Inner(s);\n  }\n  string Inner(string x) { return x; }\n}\n");
    write(root, "src/i.c", "/* SENTINEL comment */\nstatic int inner(int x) { return x; }\nint run(int x) {\n  const char *s = \"SENTINEL literal\";\n  return inner(x);\n}\n");
    write(root, "src/j.kt", "// SENTINEL comment\nfun run(x: String = \"SENTINEL default\"): String {\n  val s = \"SENTINEL literal\"\n  return inner(s)\n}\nfun inner(x: String): String = x\n");
    write(root, "src/k.swift", "// SENTINEL comment\nfunc run(_ x: String = \"SENTINEL default\") -> String {\n  let s = \"SENTINEL literal\"\n  return inner(s)\n}\nfunc inner(_ x: String) -> String { return x }\n");
    write(root, "README.md", "SENTINEL in prose\n");
    write(root, "package.json", "{\"name\":\"fixture\",\"description\":\"SENTINEL manifest\"}\n");
    write(root, "db/001.sql", "-- SENTINEL comment\nCREATE TABLE t (id int primary key, note text default 'SENTINEL');\n");
}

#[test]
fn shared_map_has_no_source() {
    let d = tempfile::tempdir().unwrap();
    sentinel_fixture(d.path());
    let idx = build(d.path());
    // The sentinel is really in the index's source files (the test would be
    // vacuous otherwise) and the fixture parsed into symbols.
    assert!(idx.symbols.len() >= 20, "{} symbols", idx.symbols.len());
    let bytes = serde_json::to_vec(&shared_of(&idx)).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(!text.contains(SENTINEL), "source text reached the shared map");
    assert!(!text.contains("SENTINEL"), "source text reached the shared map");
    // Nor does a signature (a declaration line) or a code body.
    assert!(!text.contains("signature") && !text.contains("string = "));
    // The map survives its own round trip with none of it either.
    let back = Index::from_shared(serde_json::from_str(&text).unwrap(), "o", "r").unwrap();
    for s in &back.symbols {
        assert!(s.signature.is_empty());
    }
    let search = back.search("greet", &SearchOptions::default());
    assert!(!serde_json::to_string(&search).unwrap().contains("SENTINEL"));
}

fn app_fixture(root: &Path) {
    write(root, "package.json", r#"{"name":"@acme/app"}"#);
    write(root, "src/greet.ts", "export function greetUser(name: string): string {\n  return decorate(name);\n}\nexport function decorate(s: string): string {\n  return `<${s}>`;\n}\n");
    write(root, "src/util.ts", "import { greetUser } from './greet';\nexport function shout(n: string) {\n  return greetUser(n).toUpperCase();\n}\n");
    write(root, "src/main.ts", "import { shout } from './util';\nimport pad from 'left-pad';\nexport function render(n: string) {\n  return pad(shout(n));\n}\n");
    write(root, "src/main.test.ts", "import { render } from './main';\nexport function testRender() {\n  return render('x');\n}\n");
    write(root, "src/jobs.py", "from helpers import clean\n\nclass Job:\n    def run(self):\n        return clean(self)\n\ndef schedule(job):\n    return job.run()\n");
    write(root, "src/helpers.py", "def clean(x):\n    return x\n");
}

fn strip(mut v: Value, keys: &[&str]) -> Value {
    if let Value::Object(m) = &mut v {
        for k in keys {
            m.remove(*k);
        }
    }
    v
}

fn same_answers(local: &Index, shared: &Index, targets: &[&str]) {
    let opts = MapOptions { focus: None, limit: 12 };
    let drop = ["root", "indexed_ms", "index_ms", "parsed", "cached", "stats", "summary_line"];
    let (a, b) = (strip(serde_json::to_value(local.map(&opts)).unwrap(), &drop), strip(serde_json::to_value(shared.map(&opts)).unwrap(), &drop));
    assert_eq!(a, b, "map differs");
    for t in targets {
        let imp = ImpactOptions::default();
        match (local.impact(&[t.to_string()], &imp), shared.impact(&[t.to_string()], &imp)) {
            (Ok(x), Ok(y)) => assert_eq!(serde_json::to_value(&x).unwrap(), serde_json::to_value(&y).unwrap(), "impact {t}"),
            (Err(x), Err(y)) => assert_eq!(x, y),
            other => panic!("impact {t}: {other:?}"),
        }
        let copts = ContextOptions::default();
        let (cx, cy) = (local.context(t, &copts), shared.context(t, &copts));
        match (cx, cy) {
            (Ok(x), Ok(y)) => {
                let keep = |r: &repomap_core::query::ContextResult| strip(serde_json::to_value(r).unwrap(), &["code", "signature", "source_url"]);
                assert_eq!(keep(&x), keep(&y), "context {t}");
                // The shared answer has a permalink and no code.
                assert!(y.code.is_none() && y.source_url.is_some(), "context {t}");
            }
            (Err(x), Err(y)) => assert_eq!(x, y),
            other => panic!("context {t}: {:?}", other.0.is_ok()),
        }
        for dir in [Direction::Callers, Direction::Callees] {
            let topts = TraceOptions { direction: dir, depth: 3 };
            let (x, y) = (local.trace(t, None, &topts), shared.trace(t, None, &topts));
            assert_eq!(x.map(|r| serde_json::to_value(&r).unwrap()), y.map(|r| serde_json::to_value(&r).unwrap()), "trace {t}");
        }
    }
}

#[test]
fn round_trip_gives_the_same_graph_answers() {
    let d = tempfile::tempdir().unwrap();
    app_fixture(d.path());
    let local = build(d.path());
    let wire = serde_json::to_string(&shared_of(&local)).unwrap();
    let shared = Index::from_shared(serde_json::from_str(&wire).unwrap(), "acme", "app").unwrap();
    same_answers(&local, &shared, &["greetUser", "src/greet.ts", "shout", "render", "Job.run", "src/helpers.py:2", "clean"]);
    // trace between two symbols
    let t = TraceOptions { direction: Direction::Callees, depth: 3 };
    assert_eq!(
        serde_json::to_value(local.trace("render", Some("greetUser"), &t).unwrap()).unwrap(),
        serde_json::to_value(shared.trace("render", Some("greetUser"), &t).unwrap()).unwrap()
    );
}

#[test]
fn round_trip_on_the_repomap_source_itself() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let local = build(&root);
    assert!(local.symbols.len() > 300);
    let wire = serde_json::to_string(&shared_of(&local)).unwrap();
    let shared = Index::from_shared(serde_json::from_str(&wire).unwrap(), "SylphxAI", "repomap").unwrap();
    same_answers(&local, &shared, &["Index::build", "graph.rs", "finish_graph", "pagerank", "Bm25.search"]);
}

#[test]
fn search_on_a_shared_map_matches_names_and_paths() {
    let d = tempfile::tempdir().unwrap();
    app_fixture(d.path());
    let local = build(d.path());
    let shared = Index::from_shared(shared_of(&local), "acme", "app").unwrap();
    let by_name = shared.search("greetUser", &SearchOptions::default());
    assert_eq!(by_name.hits[0].symbol.as_ref().map(|s| s.name.as_str()), Some("greetUser"));
    assert!(by_name.hits.iter().all(|h| h.snippet.is_empty()), "no code in a shared hit");
    let by_path = shared.search("jobs", &SearchOptions::default());
    assert!(by_path.hits.iter().any(|h| h.file == "src/jobs.py"), "{:?}", by_path.hits.iter().map(|h| &h.file).collect::<Vec<_>>());
    // Words that only appear in code bodies match nothing.
    assert!(shared.search("toUpperCase", &SearchOptions::default()).hits.is_empty());
}

#[test]
fn context_cites_a_github_permalink() {
    let d = tempfile::tempdir().unwrap();
    app_fixture(d.path());
    let local = build(d.path());
    let mut map = shared_of(&local);
    map.commit = Some("abc123".into());
    map.web = None;
    let shared = Index::from_shared(map, "acme", "app").unwrap();
    let c = shared.context("greetUser", &ContextOptions::default()).unwrap();
    assert_eq!(c.source_url.as_deref(), Some("https://github.com/acme/app/blob/abc123/src/greet.ts#L1-L3"));
    assert!(c.text().contains("https://github.com/acme/app/blob/abc123/src/greet.ts#L1-L3"));
    assert!(c.code.is_none());
    let f = shared.context("src/greet.ts", &ContextOptions::default()).unwrap();
    assert_eq!(f.source_url.as_deref(), Some("https://github.com/acme/app/blob/abc123/src/greet.ts"));
    // A local index adds nothing to its answers.
    let l = serde_json::to_value(local.context("greetUser", &ContextOptions::default()).unwrap()).unwrap();
    assert!(l.get("source_url").is_none() && l["code"].is_string());
}

#[test]
fn damaged_maps_are_errors_not_panics() {
    let d = tempfile::tempdir().unwrap();
    app_fixture(d.path());
    let good = shared_of(&build(d.path()));
    let check = |f: &dyn Fn(&mut SharedMap)| {
        let mut m = good.clone();
        f(&mut m);
        assert!(Index::from_shared(m, "o", "r").is_err());
    };
    check(&|m| m.format = 99);
    check(&|m| m.symbols[0].file = 9999);
    check(&|m| m.symbols[0].parent = Some(9999));
    check(&|m| m.sym_edges.push(repomap_core::index::SymEdge { from: 0, to: 9999, kind: repomap_core::index::EdgeKind::Calls, line: 1 }));
    check(&|m| m.file_edges.push(repomap_core::index::FileEdge { from: 9999, to: 0, imports: 1, calls: 0 }));
    check(&|m| m.communities[0].files.push(9999));
    check(&|m| m.symbols.swap(0, 3));
    // Unknown fields are refused, so a document cannot smuggle more in.
    let mut v = serde_json::to_value(&good).unwrap();
    v["source"] = json!("x");
    assert!(serde_json::from_value::<SharedMap>(v).is_err());
}

#[test]
fn package_identities_travel_in_the_map() {
    let d = tempfile::tempdir().unwrap();
    app_fixture(d.path());
    let map = shared_of(&build(d.path()));
    assert!(map.packages.published.iter().any(|p| p.kind == "npm" && p.key == "@acme/app"), "{:?}", map.packages.published);
    assert!(map.packages.consumed.iter().any(|c| c.kind == "npm" && c.candidates.iter().any(|x| x.0 == "left-pad")));
}

#[test]
fn a_shared_map_joins_a_local_repo_across_the_boundary() {
    use repomap_core::workspace_graph::{join_roots, JoinOptions};
    use std::collections::HashMap;
    let d = tempfile::tempdir().unwrap();
    let (lib, app) = (d.path().join("core-lib"), d.path().join("app"));
    write(&lib, "package.json", r#"{"name":"@acme/core","version":"1.0.0"}"#);
    write(&lib, "src/greet.ts", "export function greetUser(name: string): string {\n  return `hello ${name}`;\n}\n");
    write(&app, "package.json", r#"{"name":"app"}"#);
    write(&app, "src/main.ts", "import { greetUser } from '@acme/core/src/greet';\nexport function render(n: string) {\n  return greetUser(n);\n}\n");
    write(&app, "src/main.test.ts", "import { render } from './main';\nexport function testRender() {\n  return render('x');\n}\n");
    // The library is known only through its shared map: delete it from disk first.
    let lib_shared = Arc::new(Index::from_shared(shared_of(&build(&lib)), "acme", "core-lib").unwrap());
    std::fs::remove_dir_all(&lib).unwrap();
    let link = std::path::PathBuf::from("https://review.example/m/acme/core-lib#abc");
    let g = join_roots(
        &[app.clone(), link.clone()],
        &JoinOptions { no_cache: true, shared: HashMap::from([(link.clone(), lib_shared)]), ..Default::default() },
    )
    .unwrap();
    assert_eq!(g.members.len(), 2);
    let r = g.impact(&["core-lib:src/greet.ts".into()], &ImpactOptions::default()).unwrap();
    let dependent = r.repos.get("app").expect("app depends on the shared repo's package");
    assert!(dependent.direct.iter().any(|x| x.file == "src/main.ts" && x.via == "npm:@acme/core"), "{:?}", dependent.direct);
    assert!(dependent.tests.contains(&"src/main.test.ts".to_string()));
    // A shared member is never rebuilt from disk.
    let mut g = g;
    assert_eq!(g.refresh().unwrap(), 0);
}
