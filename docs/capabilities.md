# repomap capabilities

What repomap can do today and where the code is. CI checks that every path in
the Code column exists (`scripts/check-capabilities.ts`). The destination is in
[vision.md](vision.md).

| ID | Capability | Status | Code | Depends on |
| --- | --- | --- | --- | --- |
| RM-PARSE | Parse 20+ languages with tree-sitter into symbols, imports and calls | supported | crates/repomap-core/src/parse.rs, crates/repomap-core/src/lang.rs | |
| RM-GRAPH | Resolve imports and calls into one file and symbol graph; PageRank centrality and module detection; per-file parse cache | supported | crates/repomap-core/src/graph.rs, crates/repomap-core/src/index.rs | RM-PARSE |
| RM-SEARCH | Hybrid search: BM25, query terms, a local embedding model and tuned ranking | supported | crates/repomap-core/src/bm25.rs, crates/repomap-core/src/qterms.rs, crates/repomap-core/src/semantic.rs, crates/repomap-core/src/tune.rs | RM-GRAPH |
| RM-QUERY | `map` (with a token budget), `context`, `trace` and `impact` (including `--changed`, risk and tests to run) | supported | crates/repomap-core/src/query.rs | RM-GRAPH |
| RM-DB | Database map from migrations and live read-only connections, with the code that queries each table | supported | crates/repomap-core/src/db.rs, crates/repomap/src/dblive.rs | RM-GRAPH |
| RM-WORKSPACE | Team: one graph across several repository roots. `map`, `search`, `context`, `trace` and `impact` join across repos through shared packages (npm, Cargo, Go, Python); `db` answers once per root, never as a merged schema. Without a Team licence every tool answers for the current repo with `pro_required` | supported | crates/repomap-core/src/workspace_graph.rs, crates/repomap-core/src/multirepo.rs, crates/repomap/src/team.rs | RM-QUERY, RM-DB |
| RM-SHARED | Team: private shared maps. `export --shared` writes a source-free map (names, paths, graph; no code, signatures, comments or strings); `repomap login` signs in with GitHub's device flow; a `https://review.repomap.sylphx.com/m/{owner}/{repo}` link is accepted as `root` or a `workspace` entry, with `context` giving a GitHub permalink, `search` over names and paths, and `impact --changed` and `db` not available. The hosted side (building, storing and serving maps) is `RC-MAPS` in repomap-cloud and is not live yet | partial | crates/repomap-core/src/shared.rs, crates/repomap/src/remote.rs, crates/repomap/src/login.rs, crates/repomap/src/tools.rs, crates/repomap/src/main.rs | RM-WORKSPACE |
| RM-MCP | MCP server with six tools over stdio, refreshed when files change | supported | crates/repomap/src/mcp.rs, crates/repomap/src/tools.rs, crates/repomap/src/workspace.rs | RM-QUERY |
| RM-CLI | Command line with the same queries, plus `index`, `setup` and the Claude Code hook | supported | crates/repomap/src/main.rs, crates/repomap/src/setup.rs, crates/repomap/src/hook.rs | RM-QUERY |
| RM-UI | Graph UI: `serve` (local, token for non-loopback) and a self-contained `export` | supported | crates/repomap/src/serve.rs, crates/repomap-core/src/export.rs, crates/repomap/assets, ui | RM-GRAPH |
| RM-SCORE | Agent-readiness score, badge and GitHub Action | supported | crates/repomap-core/src/score.rs, action.yml | RM-GRAPH |
| RM-DIST | npm launcher and native binaries, GHCR image, `.mcpb` bundles | supported | packages/repomap, packages/npm, Dockerfile.release | RM-CLI |
| RM-BENCH | Search and localisation benchmarks on GitHub-hosted runners | supported | bench/search, bench/localization, .github/workflows/bench.yml, .github/workflows/bench-localization.yml | RM-SEARCH |

The hosted App's capabilities are recorded in `SylphxAI/repomap-cloud`'s own `docs/capabilities.md`; private paths are not listed here.
