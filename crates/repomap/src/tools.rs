//! Tool dispatch shared by the MCP server and the HTTP API.

use crate::workspace::Workspace;
use repomap_core::query::Direction;
use repomap_core::{ContextOptions, ImpactOptions, MapOptions, SearchOptions, TraceOptions};
use serde_json::{json, Value};


/// Legacy tool names from Spine and Locus, kept callable for one major.
pub fn canonical(name: &str) -> Option<&'static str> {
    Some(match name {
        "map" | "architecture_index" | "architecture_status" | "architecture_overview" | "architecture_explain" => "map",
        "search" | "architecture_search" | "codebase_search" => "search",
        "context" | "architecture_context_pack" | "architecture_evidence" | "find_related" => "context",
        "trace" | "architecture_path" | "architecture_trace" => "trace",
        "impact" | "architecture_impact" => "impact",
        "db" | "database" | "schema" => "db",
        _ => return None,
    })
}

pub fn definitions(include_legacy: bool) -> Vec<Value> {
    let root = json!({"type": "string", "description": "Repository root, or a shared map link (https://review.repomap.sylphx.com/m/{owner}/{repo}) for a repository you can read on GitHub but have not cloned. Defaults to the client's workspace root or the server's working directory."});
    let format = json!({"type": "string", "enum": ["text", "json"], "description": "text (default, compact, file:line cited) or json."});
    let mut tools = vec![
        json!({
            "name": "map",
            "title": "Map the codebase",
            "description": "Start here. A map of the repository: modules (communities of files that depend on each other), the most central files (PageRank), the most used symbols, and entry points. Pass `focus` (a directory) to zoom in and get an outline of its files and symbols with line numbers.",
            "inputSchema": {"type": "object", "properties": {
                "focus": {"type": "string", "description": "Directory to zoom into, e.g. src/server."},
                "limit": {"type": "integer", "description": "Items per section (default 12)."},
                "tokens": {"type": "integer", "description": "Token budget (estimated at 4 characters per token). Fills the map with the highest-ranked modules, files and symbols up to the budget and says what was omitted. Default: no budget."},
                "root": root, "format": format
            }},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        }),
        json!({
            "name": "search",
            "title": "Search code",
            "description": "Hybrid code search over AST chunks (whole functions, methods, classes): symbol names, BM25 keywords and a local code embedding model, so identifiers and plain questions both work, e.g. \"parseConfig\", \"refresh token expiry\" or \"where are failed requests retried\". Returns ranked file:line ranges with the matching lines.",
            "inputSchema": {"type": "object", "required": ["query"], "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer", "description": "Max results (default 10)."},
                "path": {"type": "string", "description": "Only files whose path starts with or contains this."},
                "kind": {"type": "string", "enum": ["function", "method", "class", "struct", "interface", "trait", "enum", "type", "module", "macro"]},
                "include_tests": {"type": "boolean", "description": "Default true (tests rank lower)."},
                "root": root, "format": format
            }},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        }),
        json!({
            "name": "context",
            "title": "360° view of a symbol or file",
            "description": "Everything about one symbol or file: its code, who calls it (with call-site lines), what it calls, subtypes, members, imports, importers and the tests that touch it. Large fixture trees are skipped until targeted (target one, or pass --include-fixtures). Target forms: `path/to/file.ts`, `file.ts:42`, `Class.method`, `Class::method`, or a bare name.",
            "inputSchema": {"type": "object", "required": ["target"], "properties": {
                "target": {"type": "string"},
                "code_lines": {"type": "integer", "description": "Lines of source to include for a symbol (default 60, 0 for none)."},
                "root": root, "format": format
            }},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        }),
        json!({
            "name": "trace",
            "title": "Trace call paths",
            "description": "With `from` and `to`: the shortest call path between two symbols or files, each hop cited file:line (falls back to the file dependency path). With only `from`: the call tree below it (direction callees) or above it (direction callers).",
            "inputSchema": {"type": "object", "required": ["from"], "properties": {
                "from": {"type": "string"},
                "to": {"type": "string"},
                "direction": {"type": "string", "enum": ["callees", "callers"]},
                "depth": {"type": "integer", "description": "Tree depth (default 3)."},
                "root": root, "format": format
            }},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        }),
        json!({
            "name": "impact",
            "title": "Change impact (blast radius)",
            "description": "What breaks if this changes. Give `target` (symbol or file, or a list) or `changed: true` to analyse the current git diff. Returns a risk level, direct and indirect callers with call sites, importing files, modules touched, and the tests to run. Fixture files in deferred trees are not analysed unless targeted; the result says so.",
            "inputSchema": {"type": "object", "properties": {
                "target": {"oneOf": [{"type": "string"}, {"type": "array", "items": {"type": "string"}}]},
                "changed": {"type": "boolean", "description": "Use the working-tree git diff (against `base`)."},
                "base": {"type": "string", "description": "Git ref for `changed` (default HEAD)."},
                "depth": {"type": "integer", "description": "Caller depth (default 3)."},
                "root": root, "format": format
            }},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        }),
        json!({
            "name": "db",
            "title": "Database map",
            "description": "Map the database: tables, columns, primary and foreign keys, indexes, and the code that queries each table (file:line). By default it reads schema sources in the repo: SQL migrations, Prisma, Drizzle, SQLAlchemy, Diesel and Django models. To inspect a live Postgres, MySQL or SQLite database, pass `url_env` (the NAME of an environment variable holding the connection string). The connection is strictly read-only and the string is never stored or shown. Pass `table` for one table's full detail.",
            "inputSchema": {"type": "object", "properties": {
                "table": {"type": "string", "description": "Focus on one table: columns, indexes, who references it, and where the code queries it."},
                "url_env": {"type": "string", "description": "Name of an env var with the connection string, e.g. DATABASE_URL."},
                "url": {"type": "string", "description": "Connection string (prefer url_env so secrets stay out of transcripts)."},
                "root": root, "format": format
            }},
            "annotations": {"readOnlyHint": true, "openWorldHint": true}
        }),
    ];
    if include_legacy {
        for (old, new) in [
            ("architecture_overview", "map"),
            ("architecture_search", "search"),
            ("architecture_path", "trace"),
            ("architecture_impact", "impact"),
            ("architecture_context_pack", "context"),
            ("codebase_search", "search"),
        ] {
            tools.push(json!({
                "name": old,
                "description": format!("Deprecated alias of `{new}` (removed in repomap 2.0)."),
                "inputSchema": {"type": "object", "additionalProperties": true}
            }));
        }
    }
    tools
}

/// README rows for each tool: (name, "ask it", "returns"). `repomap tools
/// --markdown` renders the README table from these plus the registry above.
pub const DOCS: &[(&str, &str, &str)] = &[
    ("map", "\"Give me the lay of the land\" / `focus: \"src/server\"`", "Modules, central files, key symbols, entry points; an outline with line numbers when focused"),
    ("search", "`\"where are failed requests retried\"`, `\"parseConfig\"`", "Ranked `file:line` ranges (functions, methods, classes) by keywords, names and meaning, with the matching lines"),
    ("context", "`SessionStore.refresh`, `src/auth/token.ts`, `token.ts:42`", "Code, callers (with call sites), callees, subtypes, members, imports, importers, tests"),
    ("trace", "`from: handleRequest, to: db.query`", "Shortest call path, or the call tree above/below a symbol"),
    ("impact", "`target: verifyToken` or `changed: true`", "Risk level, callers by depth, importing files, modules, tests to run"),
    ("db", "`table: users`, or `url_env: DATABASE_URL` for a live database", "Tables, keys, indexes, and the code (`file:line`) that queries each table"),
];

const NUMBERS: [&str; 10] = ["Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine"];

/// The README tool table, generated from the registry.
pub fn markdown() -> String {
    let defs = definitions(false);
    let n = defs.len();
    let mut o = format!("{} tools, each with an obvious job:\n\n| Tool | Ask it | Returns |\n|---|---|---|\n", NUMBERS.get(n).copied().unwrap_or("Many"));
    for d in &defs {
        let name = d["name"].as_str().unwrap_or("");
        let (ask, ret) = DOCS.iter().find(|x| x.0 == name).map(|x| (x.1, x.2)).unwrap_or(("", ""));
        o.push_str(&format!("| `{name}` | {ask} | {ret} |\n"));
    }
    o
}

fn s<'a>(a: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| a.get(*k).and_then(|v| v.as_str())).filter(|v| !v.is_empty())
}

fn n(a: &Value, keys: &[&str]) -> Option<usize> {
    keys.iter().find_map(|k| a.get(*k).and_then(|v| v.as_u64())).map(|v| v as usize)
}

fn strings(a: &Value, keys: &[&str]) -> Vec<String> {
    for k in keys {
        match a.get(*k) {
            Some(Value::String(v)) if !v.is_empty() => return vec![v.clone()],
            Some(Value::Array(v)) => {
                let out: Vec<String> = v.iter().filter_map(|x| x.as_str().map(String::from)).collect();
                if !out.is_empty() {
                    return out;
                }
            }
            _ => {}
        }
    }
    Vec::new()
}

#[derive(Debug)]
pub struct Output {
    pub text: String,
    pub json: Value,
}

/// Run a tool. `root` must already be resolved. A request that names more than
/// one workspace root goes through the Team gate; every other request is free.
/// A root or workspace entry may be a shared-map link (see `remote`).
pub fn call(ws: &Workspace, name: &str, args: &Value, root: &std::path::Path) -> Result<Output, String> {
    let roots = crate::team::named_roots(args, root);
    if roots.len() < 2 {
        if crate::team::is_link(root) {
            return call_shared(name, args, root.to_str().unwrap_or_default());
        }
        return call_root(ws, name, args, root);
    }
    let tool = canonical(name).unwrap_or(name);
    if roots.iter().any(|r| crate::team::is_link(r)) {
        return call_with_shared(ws, name, tool, args, root, &roots);
    }
    crate::team::gate(
        &crate::team::POLICY,
        &roots,
        tool,
        args,
        || call_root(ws, name, args, root),
        // The free workspace's indexes, so a join does not index a root twice.
        || {
            roots
                .iter()
                .filter_map(|r| Some((r.canonicalize().ok()?, ws.get(r).ok()?)))
                .collect()
        },
        // A database is not one graph: `db` answers once per root.
        || (tool == "db").then(|| db_per_root(ws, args, &roots)),
    )
}

/// The notice for a shared map whose publisher's Team is not active.
fn team_inactive() -> Output {
    let required = mcp_kit::licence::ProRequired {
        feature: "Private shared maps".into(),
        product: crate::team::POLICY.product.into(),
        tier: crate::team::POLICY.tier.into(),
        url: crate::team::POLICY.upgrade_url.into(),
    };
    let notice = mcp_kit::licence::required_result_json(&required)["structuredContent"]["pro_required"].clone();
    Output { text: format!("{}\n", required), json: json!({ "pro_required": notice }) }
}

/// One shared-map link as the whole request.
fn call_shared(name: &str, args: &Value, link: &str) -> Result<Output, String> {
    let cfg = crate::remote::Config::from_env();
    let l = cfg.link(link)?;
    match crate::remote::load(&cfg, &l) {
        Ok(index) => run_shared(name, args, &index),
        Err(crate::remote::Fail::TeamInactive) => Ok(team_inactive()),
        Err(f) => Err(f.message()),
    }
}

/// A tool against a shared map, which has a graph and names but no source and
/// no working tree.
fn run_shared(name: &str, args: &Value, index: &repomap_core::Index) -> Result<Output, String> {
    let tool = canonical(name).ok_or_else(|| format!("unknown tool `{name}`"))?;
    match tool {
        "db" => {
            let text = "db is not available on a shared map: it holds names, paths and the graph, no schema files or queries. Run repomap in a clone of the repository.".to_string();
            Ok(Output { json: json!({ "unavailable": text }), text })
        }
        "impact" if is_changed(args) || strings(args, &["target", "targets", "symbol", "paths", "changed_paths", "files"]).is_empty() => {
            Err("a shared map has no working tree; pass `target` (a symbol or file)".into())
        }
        _ => {
            let mut out = run_index(tool, args, index)?;
            if tool == "search" {
                let line = "Search on a shared map matches symbol names and file paths only; it holds no source text.";
                out.text.push_str(&format!("\n{line}\n"));
                if let Value::Object(m) = &mut out.json {
                    m.insert("note".into(), json!(line));
                }
            }
            Ok(out)
        }
    }
}

fn is_changed(args: &Value) -> bool {
    args.get("changed").and_then(|v| v.as_bool()).unwrap_or(false) || args.get("use_git_diff").and_then(|v| v.as_bool()).unwrap_or(false)
}

/// A request whose roots include shared-map links. A map the server answered
/// is licensed by its publisher's Team and the reader's GitHub access, so a
/// join with at least one served shared map needs no local licence.
fn call_with_shared(ws: &Workspace, name: &str, tool: &str, args: &Value, root: &std::path::Path, roots: &[std::path::PathBuf]) -> Result<Output, String> {
    use std::path::PathBuf;
    if tool == "db" {
        return db_per_root(ws, args, roots);
    }
    let cfg = crate::remote::Config::from_env();
    // Each link keyed with its commit, so a newer map is a new graph.
    let mut served: std::collections::HashMap<PathBuf, std::sync::Arc<repomap_core::Index>> = Default::default();
    let mut keyed: Vec<PathBuf> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut inactive = false;
    for r in roots {
        let Some(link) = r.to_str().filter(|s| repomap_core::shared::is_link(s)) else {
            keyed.push(r.clone());
            continue;
        };
        let loaded = cfg.link(link).map_err(crate::remote::Fail::Other).and_then(|l| crate::remote::load(&cfg, &l));
        match loaded {
            Ok(index) => {
                let key = PathBuf::from(format!("{link}#{}", index.shared.as_ref().and_then(|i| i.commit.clone()).unwrap_or_default()));
                served.insert(key.clone(), index);
                keyed.push(key);
            }
            Err(f) if r.as_path() == root => {
                return match f {
                    crate::remote::Fail::TeamInactive => Ok(team_inactive()),
                    f => Err(f.message()),
                }
            }
            Err(f) => {
                inactive |= f == crate::remote::Fail::TeamInactive;
                notes.push(format!("{link} was not joined: {}.", f.message()));
            }
        }
    }
    let mut out = if served.is_empty() {
        let mut o = call_root(ws, name, args, root)?;
        if inactive {
            o.json = match o.json {
                Value::Object(mut m) => {
                    m.insert("pro_required".into(), team_inactive().json["pro_required"].clone());
                    Value::Object(m)
                }
                other => json!({ "result": other, "pro_required": team_inactive().json["pro_required"] }),
            };
        }
        o
    } else {
        let current = keyed.first().cloned().unwrap_or_default();
        let shared = served.clone();
        let joined = repomap_core::multirepo::join_workspace_shared(&keyed, shared, tool, args, || {
            keyed
                .iter()
                .filter(|r| !served.contains_key(*r))
                .filter_map(|r| Some((r.canonicalize().ok()?, ws.get(r).ok()?)))
                .collect()
        });
        match joined {
            Ok(j) => Output { text: j.text, json: j.json },
            Err(_) => {
                let mut o = match served.get(&current) {
                    Some(index) => run_shared(name, args, index)?,
                    None => call_root(ws, name, args, root)?,
                };
                crate::team::note(&mut o, "The other repos in this workspace were not joined (not yet joined); this answers for the current repo only.", None, None);
                o
            }
        }
    };
    for n in notes {
        crate::team::note(&mut out, &n, None, None);
    }
    Ok(out)
}

/// `db` over a workspace: one answer per root, never a merged schema. Without
/// a URL each repo's own schema (its migrations and models) is shown with the
/// code that queries it, because separate repos usually mean separate
/// databases. With `url` or `url_env` the one live database is shown to each
/// repo, so the answer says which repo's code queries each table.
fn db_per_root(ws: &Workspace, args: &Value, roots: &[std::path::PathBuf]) -> Result<Output, String> {
    let url = match (s(args, &["url"]), s(args, &["url_env"])) {
        (Some(u), _) => Some(u.to_string()),
        (None, Some(var)) => Some(std::env::var(var).map_err(|_| format!("environment variable `{var}` is not set"))?),
        _ => None,
    };
    let table = s(args, &["table"]);
    let mut text = format!(
        "# db across {} repos\n\n{}\n",
        roots.len(),
        if url.is_some() {
            "One live database, shown once per repo: each section lists the code in that repo that queries each table."
        } else {
            "No `url` given, so each repo's own schema is shown; repos are not assumed to share a database."
        }
    );
    let mut repos = serde_json::Map::new();
    for r in roots {
        if crate::team::is_link(r) {
            let label = r.to_str().and_then(repomap_core::shared::parse_link).map_or_else(|| r.display().to_string(), |l| l.repo);
            text.push_str(&format!("\n## Repo {label}\n\nNot in shared maps: a shared map holds names, paths and the graph, no schema files or queries.\n"));
            continue;
        }
        let mut name = r.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| r.display().to_string());
        if repos.contains_key(&name) {
            name = r.display().to_string();
        }
        let index = ws.get(r).map_err(|e| format!("indexing {} failed: {e}", r.display()))?;
        let schema = db_schema(&index, url.as_deref()).map_err(|e| format!("{name}: {e}"))?;
        let json = match table.and_then(|t| schema.table(t)) {
            Some(t) => serde_json::to_value(t).unwrap_or(Value::Null),
            None => serde_json::to_value(&schema).unwrap_or(Value::Null),
        };
        text.push_str(&format!("\n## Repo {name}\n\n{}", schema.text(table)));
        if !text.ends_with('\n') {
            text.push('\n');
        }
        repos.insert(name, json);
    }
    Ok(Output { text, json: serde_json::json!({ "repos": repos }) })
}

/// Run a tool against one root.
fn call_root(ws: &Workspace, name: &str, args: &Value, root: &std::path::Path) -> Result<Output, String> {
    let tool = canonical(name).ok_or_else(|| format!("unknown tool `{name}`"))?;
    // A query that names a path inside a deferred fixture tree indexes it.
    let targets: Vec<String> = ["target", "targets", "symbol", "paths", "changed_paths", "files", "focus", "path", "scope", "from", "to", "file", "source", "start", "end", "node", "id"]
        .iter()
        .flat_map(|k| strings(args, &[k]))
        .collect();
    let mut targets = targets;
    // Edited files inside a deferred fixture tree must be indexed too.
    let changed_mode = canonical(name) == Some("impact")
        && (args.get("changed").and_then(|v| v.as_bool()).unwrap_or(false) || args.get("use_git_diff").and_then(|v| v.as_bool()).unwrap_or(false));
    if changed_mode {
        let base = repomap_core::index::check_ref(s(args, &["base", "git_base"]).unwrap_or("HEAD"))?;
        for a in [vec!["diff", "--name-only", "--end-of-options", base], vec!["ls-files", "--others", "--exclude-standard"]] {
            if let Some(out) = repomap_core::index::git_run(root, &a) {
                targets.extend(out.lines().filter(|l| !l.is_empty()).map(str::to_string));
            }
        }
    }
    let index = ws.get_with(root, &targets).map_err(|e| format!("indexing {} failed: {e}", root.display()))?;
    run_index(tool, args, &index)
}

/// Run a canonical tool against an index.
fn run_index(tool: &str, args: &Value, index: &repomap_core::Index) -> Result<Output, String> {
    macro_rules! out {
        ($r:expr) => {{
            let r = $r;
            Ok(Output { text: r.text(), json: serde_json::to_value(&r).unwrap_or(Value::Null) })
        }};
    }
    match tool {
        "map" => {
            let focus = s(args, &["focus", "path", "scope"]).map(String::from);
            let limit = n(args, &["limit"]).unwrap_or(12);
            // With a token budget, build a generous map and let `fit_tokens` trim it by rank.
            match n(args, &["tokens", "max_tokens"]) {
                Some(t) => {
                    let mut m = index.map(&MapOptions { focus, limit: limit.max(500) });
                    m.fit_tokens(t);
                    out!(m)
                }
                None => out!(index.map(&MapOptions { focus, limit })),
            }
        }
        "search" => {
            let query = s(args, &["query", "q", "text"]).ok_or("`query` is required")?;
            let opts = SearchOptions {
                limit: n(args, &["limit"]).unwrap_or(10).clamp(1, 100),
                path: s(args, &["path", "path_filter"]).map(String::from),
                kind: s(args, &["kind"]).map(String::from),
                snippet_lines: if args.get("include_content").and_then(|v| v.as_bool()) == Some(false) { 0 } else { 4 },
                include_tests: args.get("include_tests").and_then(|v| v.as_bool()).unwrap_or(true),
            };
            out!(index.search(query, &opts))
        }
        "context" => {
            let target = match (s(args, &["target", "symbol", "node", "focus", "id", "file"]), s(args, &["path"]), n(args, &["line"])) {
                (Some(t), _, _) => t.to_string(),
                (None, Some(p), Some(l)) => format!("{p}:{l}"),
                (None, Some(p), None) => p.to_string(),
                _ => return Err("`target` is required".into()),
            };
            let opts = ContextOptions { code_lines: n(args, &["code_lines"]).unwrap_or(60), limit: n(args, &["limit"]).unwrap_or(25) };
            out!(index.context(&target, &opts)?)
        }
        "trace" => {
            let from = s(args, &["from", "source", "start", "symbol", "node", "target"]).ok_or("`from` is required")?;
            let to = if s(args, &["from", "source", "start"]).is_some() { s(args, &["to", "target", "end"]) } else { s(args, &["to", "end"]) };
            let direction = match s(args, &["direction"]) {
                Some("callers") | Some("up") | Some("in") | Some("incoming") => Direction::Callers,
                _ => Direction::Callees,
            };
            let opts = TraceOptions { direction, depth: n(args, &["depth", "max_depth"]).unwrap_or(3) };
            out!(index.trace(from, to, &opts)?)
        }
        "impact" => {
            let opts = ImpactOptions { depth: n(args, &["depth", "max_depth"]).unwrap_or(3), limit: n(args, &["limit"]).unwrap_or(30) };
            let targets = strings(args, &["target", "targets", "symbol", "paths", "changed_paths", "files"]);
            let changed = args.get("changed").and_then(|v| v.as_bool()).unwrap_or(false)
                || args.get("use_git_diff").and_then(|v| v.as_bool()).unwrap_or(false);
            if changed || targets.is_empty() {
                out!(index.impact_changed(s(args, &["base", "git_base"]), &opts)?)
            } else {
                out!(index.impact(&targets, &opts)?)
            }
        }
        "db" => {
            let url = match (s(args, &["url"]), s(args, &["url_env"])) {
                (Some(u), _) => Some(u.to_string()),
                (None, Some(var)) => Some(std::env::var(var).map_err(|_| format!("environment variable `{var}` is not set"))?),
                _ => None,
            };
            let schema = db_schema(index, url.as_deref()).map_err(|e| e.to_string())?;
            let table = s(args, &["table"]);
            let json = match table.and_then(|t| schema.table(t)) {
                Some(t) => serde_json::to_value(t).unwrap_or(Value::Null),
                None => serde_json::to_value(&schema).unwrap_or(Value::Null),
            };
            Ok(Output { text: schema.text(table), json })
        }
        _ => Err(format!("unknown tool `{tool}`")),
    }
}

/// Live schema when a URL is given (with code names borrowed from the repo's
/// schema sources), otherwise the repo's own schema; then link tables to code.
pub fn db_schema(index: &repomap_core::Index, url: Option<&str>) -> anyhow::Result<repomap_core::db::DbSchema> {
    let from_repo = repomap_core::db::from_repo(index);
    let mut schema = match url {
        Some(u) => {
            let mut live = crate::dblive::introspect(u)?;
            for t in live.tables.iter_mut() {
                if let Some(st) = from_repo.table(&t.key()) {
                    t.aliases = st.aliases.clone();
                    t.source = format!("{} (defined in {})", t.source, st.source);
                }
            }
            live
        }
        None => from_repo,
    };
    repomap_core::db::link_code(index, &mut schema);
    Ok(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_readme_docs() {
        for d in definitions(false) {
            let name = d["name"].as_str().unwrap();
            assert!(DOCS.iter().any(|x| x.0 == name), "tool `{name}` has no row in DOCS");
            assert_eq!(canonical(name), Some(name), "tool `{name}` is not routed");
        }
        assert!(markdown().starts_with("Six tools"));
    }

    #[test]
    fn db_answers_once_per_root_without_merging() {
        let d = tempfile::tempdir().unwrap();
        let w = |root: &std::path::Path, rel: &str, body: &str| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        };
        let (a, b) = (d.path().join("api"), d.path().join("jobs"));
        w(&a, "migrations/001.sql", "CREATE TABLE users (id int primary key, name text);\n");
        w(&b, "migrations/001.sql", "CREATE TABLE queue (id int primary key, body text);\n");
        let ws = Workspace::default();
        let out = db_per_root(&ws, &serde_json::json!({}), &[a.clone(), b.clone()]).unwrap();
        assert!(out.text.contains("## Repo api") && out.text.contains("## Repo jobs"), "{}", out.text);
        assert!(out.text.contains("each repo's own schema"), "{}", out.text);
        let repos = out.json["repos"].as_object().unwrap();
        assert_eq!(repos.len(), 2);
        assert!(repos["api"].to_string().contains("users") && !repos["api"].to_string().contains("queue"));
        assert!(repos["jobs"].to_string().contains("queue") && !repos["jobs"].to_string().contains("users"));
        // A missing environment variable is a clear error, not a silent fallback.
        let e = db_per_root(&ws, &serde_json::json!({"url_env": "REPOMAP_TEST_NO_SUCH_URL"}), &[a, b]).err().unwrap();
        assert!(e.contains("REPOMAP_TEST_NO_SUCH_URL"), "{e}");
    }

    fn call_json(name: &str, args: Value) -> Result<Output, String> {
        call(&Workspace::default(), name, &args, std::path::Path::new(args["root"].as_str().unwrap_or(".")))
    }

    // One test owns the process-global environment the remote path reads.
    #[test]
    fn shared_map_links_answer_the_six_tools() {
        use crate::remote::fake::{reply, serve, Reply};
        use crate::remote::tests::sample_map;
        let api = sample_map(
            &[
                ("package.json", r#"{"name":"@acme/api"}"#),
                ("src/greet.ts", "export function greetUser(name: string): string {\n  return decorate(name);\n}\nexport function decorate(s: string): string {\n  return s;\n}\n"),
                ("src/tests/greet.test.ts", "import { greetUser } from '../greet';\nexport function testGreet() {\n  return greetUser('x');\n}\n"),
            ],
            "c0ffee",
        );
        let mode = std::sync::Arc::new(std::sync::Mutex::new(200u16));
        let m2 = mode.clone();
        let fake = serve(move |r| {
            let status = *m2.lock().unwrap();
            if status != 200 {
                return reply(status, r#"{"error":{"type":"authentication_error","code":"x","message":"m"}}"#);
            }
            assert_eq!(r.header("authorization"), Some("Bearer rms_test"));
            Reply { status: 200, headers: vec![("etag", "\"c0ffee.1\"".into())], body: api.clone() }
        });
        let d = tempfile::tempdir().unwrap();
        std::env::set_var("REPOMAP_REVIEW_URL", &fake.base);
        std::env::set_var("REPOMAP_TOKEN", "rms_test");
        std::env::set_var("REPOMAP_CACHE_DIR", d.path());
        let link = format!("{}/m/acme/api", fake.base);

        // map, trace, impact(target) and context answer from the downloaded map.
        let out = call_json("map", json!({"root": link})).unwrap();
        assert!(out.text.contains("greet.ts"), "{}", out.text);
        let out = call_json("context", json!({"root": link, "target": "greetUser"})).unwrap();
        assert!(out.text.contains("https://github.com/acme/api/blob/c0ffee/src/greet.ts#L1-L3"), "{}", out.text);
        assert!(!out.text.contains("```"), "no code in a shared map: {}", out.text);
        assert!(out.json["callers"].to_string().contains("testGreet"), "{}", out.json);
        let out = call_json("trace", json!({"root": link, "from": "testGreet", "to": "decorate"})).unwrap();
        assert!(out.text.contains("greetUser"), "{}", out.text);
        let out = call_json("impact", json!({"root": link, "target": "decorate"})).unwrap();
        assert!(out.text.contains("greetUser"), "{}", out.text);

        // search: names and paths only, and it says so.
        let out = call_json("search", json!({"root": link, "query": "greetUser"})).unwrap();
        assert!(out.text.contains("names and file paths only"), "{}", out.text);
        assert!(out.text.contains("src/greet.ts"));

        // impact --changed and db say so.
        let e = call_json("impact", json!({"root": link, "changed": true})).unwrap_err();
        assert!(e.contains("no working tree"), "{e}");
        let out = call_json("db", json!({"root": link})).unwrap();
        assert!(out.text.contains("not available on a shared map"), "{}", out.text);

        // A local repo and the shared map join into one cross-repo answer,
        // with no local licence: the publisher's Team and GitHub access cover it.
        let app = d.path().join("app");
        std::fs::create_dir_all(app.join("src")).unwrap();
        std::fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();
        std::fs::write(app.join("src/main.ts"), "import { greetUser } from '@acme/api/src/greet';\nexport function render(n: string) {\n  return greetUser(n);\n}\n").unwrap();
        let out = call_json("impact", json!({"root": app.to_string_lossy(), "workspace": [link], "target": "api:src/greet.ts"})).unwrap();
        assert!(out.json.get("pro_required").is_none(), "{}", out.json);
        assert!(out.text.contains("Other repos affected") && out.text.contains("src/main.ts"), "{}", out.text);
        // db over the same workspace says where it is not available.
        let out = call_json("db", json!({"root": app.to_string_lossy(), "workspace": [link]})).unwrap();
        assert!(out.text.contains("## Repo api") && out.text.contains("Not in shared maps"), "{}", out.text);

        // Errors reach the agent in one clear line each.
        *mode.lock().unwrap() = 401;
        std::env::set_var("REPOMAP_CACHE_DIR", d.path().join("fresh1"));
        crate::remote::forget_memory();
        let e = call_json("map", json!({"root": link})).unwrap_err();
        assert!(e.contains("repomap login"), "{e}");
        *mode.lock().unwrap() = 404;
        std::env::set_var("REPOMAP_CACHE_DIR", d.path().join("fresh2"));
        crate::remote::forget_memory();
        let e = call_json("map", json!({"root": link})).unwrap_err();
        assert!(e.contains("no map you can read at this link"), "{e}");
        *mode.lock().unwrap() = 402;
        std::env::set_var("REPOMAP_CACHE_DIR", d.path().join("fresh3"));
        crate::remote::forget_memory();
        let out = call_json("map", json!({"root": link})).unwrap();
        assert_eq!(out.json["pro_required"]["tier"], "Team", "{}", out.json);
        assert!(out.text.contains("Team"), "{}", out.text);
        // A workspace link that fails leaves the local answer, with a note.
        let out = call_json("map", json!({"root": app.to_string_lossy(), "workspace": [link]})).unwrap();
        assert!(out.text.contains("was not joined"), "{}", out.text);
        assert!(out.json.get("pro_required").is_some());
        // A link to another host is never sent a token.
        let e = call_json("map", json!({"root": "https://evil.example/m/acme/api"})).unwrap_err();
        assert!(e.contains("not on the repomap review server"), "{e}");
        assert!(fake.seen.lock().unwrap().iter().all(|r| r.header("authorization").is_some() || r.url.starts_with("/v1/")));
    }

    #[test]
    fn the_root_text_names_the_link_and_there_are_still_six_tools() {
        let defs = definitions(false);
        assert_eq!(defs.len(), 6);
        for d in &defs {
            let text = d["inputSchema"]["properties"]["root"]["description"].as_str().unwrap();
            assert!(text.contains("shared map link"), "{text}");
        }
    }
}
