use repomap_core::query::{Direction, ImpactOptions, SearchOptions, TraceOptions};
use repomap_core::workspace_graph::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

/// Repo A publishes an npm package and a Cargo crate; repo B imports both;
/// repo C imports names nobody in the workspace publishes.
fn fixture(dir: &Path) -> Vec<PathBuf> {
    let (a, b, c) = (dir.join("core-lib"), dir.join("app"), dir.join("lonely"));
    write(
        &a,
        "package.json",
        r#"{"name":"@acme/core","version":"1.0.0"}"#,
    );
    write(
        &a,
        "src/greet.ts",
        "export function greetUser(name: string): string {\n  return `hello ${name}`;\n}\n",
    );
    write(&a, "src/util.ts", "import { greetUser } from './greet';\nexport function shout(n: string) {\n  return greetUser(n).toUpperCase();\n}\n");
    write(
        &a,
        "crate/Cargo.toml",
        "[package]\nname = \"acme-core\"\nversion = \"0.1.0\"\n",
    );
    write(
        &a,
        "crate/src/lib.rs",
        "pub fn compute_total(a: u32, b: u32) -> u32 {\n    a + b\n}\n",
    );
    write(&b, "package.json", r#"{"name":"app"}"#);
    write(
        &b,
        "src/main.ts",
        "import { greetUser } from '@acme/core/src/greet';\nimport pad from 'left-pad';\nexport function render(n: string) {\n  return pad(greetUser(n));\n}\n",
    );
    write(&b, "src/main.test.ts", "import { render } from './main';\nexport function testRender() {\n  return render('x');\n}\n");
    write(
        &b,
        "Cargo.toml",
        "[package]\nname = \"app-rs\"\nversion = \"0.1.0\"\n",
    );
    write(&b, "src/lib.rs", "use acme_core::compute_total;\nuse std::collections::HashMap;\npub fn sum() -> u32 {\n    compute_total(1, 2)\n}\n");
    write(&c, "package.json", r#"{"name":"lonely"}"#);
    write(&c, "src/x.ts", "import left from 'left-pad';\nimport { nothing } from '@other/missing';\nexport function lone() {\n  return left(nothing);\n}\n");
    write(&c, "src/y.rs", "use serde::Serialize;\npub fn y() {}\n");
    vec![a, b, c]
}

fn join(roots: &[PathBuf]) -> WorkspaceGraph {
    join_roots(
        roots,
        &JoinOptions {
            no_cache: true,
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn impact_names_dependents_in_other_repos() {
    let d = tempfile::tempdir().unwrap();
    let roots = fixture(d.path());
    let g = join(&roots[..2]);
    assert_eq!(g.packages.len(), 4, "{:?}", g.packages);
    let r = g
        .impact(&["src/greet.ts".into()], &ImpactOptions::default())
        .unwrap();
    assert_eq!(r.home, "core-lib");
    let app = r.repos.get("app").expect("app depends on @acme/core");
    assert!(
        app.direct
            .iter()
            .any(|x| x.file == "src/main.ts" && x.via == "npm:@acme/core"),
        "{:?}",
        app.direct
    );
    // The dependent's own blast radius and tests come along.
    assert!(
        app.files.contains(&"src/main.test.ts".to_string()),
        "{:?}",
        app.files
    );
    assert!(app.tests.contains(&"src/main.test.ts".to_string()));
    assert!(r.text().contains("Other repos affected"));

    // The Cargo crate is a second, separate join.
    let r = g
        .impact(&["crate/src/lib.rs".into()], &ImpactOptions::default())
        .unwrap();
    let app = r.repos.get("app").unwrap();
    assert!(
        app.direct
            .iter()
            .any(|x| x.file == "src/lib.rs" && x.via == "cargo:acme_core"),
        "{:?}",
        app.direct
    );

    // `repo:` prefix selects a repo explicitly.
    let r = g
        .impact(&["core-lib:src/util.ts".into()], &ImpactOptions::default())
        .unwrap();
    assert_eq!(r.home, "core-lib");
    // util.ts is not imported across repos (sub path `src/greet` only names greet.ts).
    assert!(r.repos.is_empty(), "{:?}", r.repos);
}

#[test]
fn trace_and_search_cross_the_boundary() {
    let d = tempfile::tempdir().unwrap();
    let roots = fixture(d.path());
    let g = join(&roots[..2]);
    let t = g
        .trace(
            "app:src/main.ts",
            Some("core-lib:src/greet.ts"),
            &TraceOptions {
                direction: Direction::Callees,
                depth: 3,
            },
        )
        .unwrap();
    assert!(t.found);
    assert_eq!(t.hops.len(), 1);
    assert_eq!(t.hops[0].via, "cross-repo import");
    assert_eq!(t.hops[0].to, "core-lib:src/greet.ts");
    // The reverse direction is found and noted.
    let t = g
        .trace(
            "core-lib:src/greet.ts",
            Some("app:src/main.ts"),
            &TraceOptions {
                direction: Direction::Callees,
                depth: 3,
            },
        )
        .unwrap();
    assert!(t.found && t.note.is_some());
    // Callers of a file in A include the file in B.
    let t = g
        .trace(
            "core-lib:src/greet.ts",
            None,
            &TraceOptions {
                direction: Direction::Callers,
                depth: 2,
            },
        )
        .unwrap();
    assert!(
        t.crossings.iter().any(|h| h.from == "app:src/main.ts"),
        "{:?}",
        t.crossings
    );

    let s = g.search("greetUser", &SearchOptions::default());
    let repos: Vec<&str> = s.hits.iter().map(|h| h.repo.as_str()).collect();
    assert!(
        repos.contains(&"core-lib") && repos.contains(&"app"),
        "{repos:?}"
    );
    let in_a = s
        .hits
        .iter()
        .find(|h| h.repo == "core-lib" && h.hit.file == "src/greet.ts")
        .unwrap();
    assert_eq!(in_a.used_by, vec!["app".to_string()]);
}

#[test]
fn unresolved_packages_stay_single_repo() {
    let d = tempfile::tempdir().unwrap();
    let roots = fixture(d.path());
    let g = join(&roots);
    // `left-pad`, `@other/missing` and `serde` are published by no member.
    assert!(
        g.edges
            .iter()
            .all(|e| g.members[e.from_repo].name != "lonely"),
        "{:?}",
        g.edges
    );
    let r = g
        .impact(&["lonely:src/x.ts".into()], &ImpactOptions::default())
        .unwrap();
    assert!(r.repos.is_empty());
    assert_eq!(r.risk, r.local.risk);
    let t = g
        .trace(
            "lonely:src/x.ts",
            Some("core-lib:src/greet.ts"),
            &TraceOptions {
                direction: Direction::Callees,
                depth: 3,
            },
        )
        .unwrap();
    assert!(!t.found);
    // One root alone is a valid workspace with no cross edges.
    assert!(join(&roots[..1]).edges.is_empty());
}

#[test]
fn workspace_file_lists_roots() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "repomap.workspace.toml",
        "# repos\nroots = [\"../a\",\n  '/abs/b', # note\n]\n",
    );
    let sub = d.path().join("x/y");
    std::fs::create_dir_all(&sub).unwrap();
    let f = find_workspace_file(&sub).unwrap();
    let roots = read_workspace_file(&f).unwrap();
    assert_eq!(roots, vec![d.path().join("../a"), std::path::PathBuf::from("/abs/b")]);
    write(d.path(), "empty.toml", "name = 1\n");
    assert!(read_workspace_file(&d.path().join("empty.toml")).is_err());
}

/// Two mid-size repos; the second imports the first.
fn big(dir: &Path, files: usize) -> Vec<PathBuf> {
    let (a, b) = (dir.join("big-a"), dir.join("big-b"));
    write(&a, "package.json", r#"{"name":"@big/a"}"#);
    write(&b, "package.json", r#"{"name":"@big/b"}"#);
    for (root, other) in [(&a, "@big/b"), (&b, "@big/a")] {
        for i in 0..files {
            let mut s = String::new();
            if i > 0 {
                s.push_str(&format!("import {{ f{} }} from './m{}';\n", i - 1, i - 1));
            }
            if i % 50 == 0 {
                s.push_str(&format!("import {{ ext }} from '{other}/m0';\n"));
            }
            for k in 0..8 {
                s.push_str(&format!("export function f{i}x{k}(a: number): number {{\n  const r = a * {k} + {i};\n  return r > 10 ? r : a;\n}}\n"));
            }
            s.push_str(&format!(
                "export function f{i}(a: number): number {{\n  return f{i}x0(a) + f{i}x1(a);\n}}\n"
            ));
            write(root, &format!("src/m{i}.ts"), &s);
        }
    }
    vec![a, b]
}

#[test]
fn join_does_not_reindex_unchanged_roots() {
    let d = tempfile::tempdir().unwrap();
    let roots = big(d.path(), 600);
    let t = Instant::now();
    let mut g = join_roots(
        &roots,
        &JoinOptions {
            no_cache: true,
            ..Default::default()
        },
    )
    .unwrap();
    let cold = t.elapsed();
    let files: usize = g.members.iter().map(|m| m.index.files.len()).sum();
    assert!(!g.edges.is_empty());

    // Unchanged roots: refresh stats the trees, parses nothing, rebuilds nothing.
    let before: Vec<_> = g
        .members
        .iter()
        .map(|m| std::sync::Arc::as_ptr(&m.index))
        .collect();
    let t = Instant::now();
    assert_eq!(g.refresh().unwrap(), 0);
    let refresh = t.elapsed();
    let after: Vec<_> = g
        .members
        .iter()
        .map(|m| std::sync::Arc::as_ptr(&m.index))
        .collect();
    assert_eq!(before, after);

    // Caller-held indexes are reused as is: the join is manifest + import reads only.
    let prebuilt = g
        .members
        .iter()
        .map(|m| (m.root.clone(), m.index.clone()))
        .collect();
    let t = Instant::now();
    let g2 = join_roots(
        &roots,
        &JoinOptions {
            prebuilt,
            ..Default::default()
        },
    )
    .unwrap();
    let rejoin = t.elapsed();
    assert_eq!(g2.edges.len(), g.edges.len());
    assert!(rejoin < cold, "rejoin {rejoin:?} vs cold {cold:?}");

    // Touching one root re-indexes that root only.
    write(
        &roots[1],
        "src/m3.ts",
        "export function f3(a: number): number {\n  return a;\n}\n",
    );
    assert_eq!(g.refresh().unwrap(), 1);
    assert!(std::sync::Arc::ptr_eq(
        &g.members[0].index,
        &g2.members[0].index
    ));

    eprintln!("workspace join timings ({files} files in 2 roots): cold join {cold:?}, refresh unchanged {refresh:?}, rejoin from held indexes {rejoin:?}");
}
