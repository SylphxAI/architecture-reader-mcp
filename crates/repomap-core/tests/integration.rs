use repomap_core::{BuildOptions, ContextOptions, ImpactOptions, Index, MapOptions, SearchOptions, TraceOptions};
use std::path::PathBuf;

fn fixture() -> Index {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixture");
    Index::build(&root, &BuildOptions { use_cache: false }).expect("index")
}

#[test]
fn map_lists_modules_and_symbols() {
    let idx = fixture();
    let m = idx.map(&MapOptions { focus: None, limit: 10 });
    assert_eq!(m.code_files, 4);
    assert!(m.symbols >= 7, "{}", m.symbols);
    assert!(m.text().contains("token.ts"));
}

#[test]
fn search_finds_symbol_and_text() {
    let idx = fixture();
    let r = idx.search("verifyToken", &SearchOptions::default());
    assert_eq!(r.hits[0].symbol.as_ref().unwrap().name, "verifyToken");
    let r = idx.search("token expired", &SearchOptions::default());
    assert_eq!(r.hits[0].file, "src/auth/token.ts");
}

#[test]
fn search_reads_identifiers_and_paths_out_of_a_report() {
    let idx = fixture();
    let opts = SearchOptions { limit: 5, ..SearchOptions::default() };
    // The report names a method; the file is only in a stack trace.
    let r = idx.search("Login keeps the old credentials\n\nAfter login `SessionStore.refresh` returns the same thing every time and nothing changes for the user.", &opts);
    assert_eq!(r.hits[0].file, "src/auth/session.ts", "{:?}", r.hits.iter().map(|h| &h.file).collect::<Vec<_>>());
    let r = idx.search("Something odd happens when requests come in. Traceback: at handle (src/api/router.ts:12)", &opts);
    assert_eq!(r.hits[0].file, "src/api/router.ts", "{:?}", r.hits.iter().map(|h| &h.file).collect::<Vec<_>>());
}

#[test]
fn report_search_keeps_path_and_kind_filters() {
    let idx = fixture();
    let opts = SearchOptions {
        limit: 20,
        path: Some("src/auth/token.ts".to_string()),
        kind: Some("function".to_string()),
        include_tests: false,
        ..SearchOptions::default()
    };
    // Named paths and identifier candidates must not escape the caller's filters.
    let r = idx.search("Refreshing `SessionStore.refresh` fails in src/auth/session.ts, after verifyToken reports token expired", &opts);
    assert!(!r.hits.is_empty());
    assert!(r.hits.iter().all(|h| h.file == "src/auth/token.ts"));
    assert!(r.hits.iter().all(|h| h.symbol.as_ref().is_some_and(|s| s.kind == "function")));
}

#[test]
fn context_shows_callers() {
    let idx = fixture();
    let c = idx.context("verifyToken", &ContextOptions::default()).unwrap();
    assert!(c.callers.iter().any(|s| s.symbol.name == "SessionStore.refresh"), "{:?}", c.callers);
    assert!(c.callees.iter().any(|s| s.symbol.name == "decode"));
    let f = idx.context("src/auth/token.ts", &ContextOptions::default()).unwrap();
    assert!(f.imported_by.contains(&"src/auth/session.ts".to_string()));
}

#[test]
fn trace_finds_call_path() {
    let idx = fixture();
    let t = idx
        .trace("handleRefresh", Some("decode"), &TraceOptions { direction: repomap_core::Direction::Callees, depth: 3 })
        .unwrap();
    assert!(t.found);
    assert_eq!(t.level, "calls");
    assert_eq!(t.hops.len(), 3, "{:?}", t.hops);
}

#[test]
fn impact_reaches_tests() {
    let idx = fixture();
    let r = idx.impact(&["decode".to_string()], &ImpactOptions { depth: 4, limit: 30 }).unwrap();
    let names: Vec<String> = r.by_depth.iter().flatten().map(|s| s.symbol.name.clone()).collect();
    assert!(names.contains(&"verifyToken".to_string()));
    assert!(names.contains(&"handleRefresh".to_string()), "{names:?}");
    assert!(r.tests.contains(&"tests/session.test.ts".to_string()), "{:?}", r.tests);
}

#[test]
fn graph_json_is_complete() {
    let idx = fixture();
    let g = idx.graph_json(&Default::default(), "test");
    assert_eq!(g["nodes"].as_array().unwrap().len(), 4);
    assert!(!g["edges"].as_array().unwrap().is_empty());
}
