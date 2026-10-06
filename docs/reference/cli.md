# CLI

```text
repomap setup [--client a,b] [--claude-hooks] [--dry-run] [--remove]
                                                     configure MCP clients (+ Claude Code hook)
repomap serve [dir] [--port 7878] [--host 127.0.0.1] [--no-open]
repomap export [dir] [--out repomap.html] [--json]    self-contained HTML (or graph JSON)
repomap map [focus-dir] [-C root] [--limit N] [--tokens N] [--json]
repomap search <query…> [--path P] [--kind K] [--limit N] [--json]
repomap context <target> [--code-lines N] [--json]
repomap trace <from> [to] [--callers] [--depth N] [--json]
repomap impact [target…] [--changed] [--base REF] [--depth N] [--json]
repomap db [table] [--url-env VAR|--url URL] [--serve|--out f.html] [--json]
repomap score [dir] [--json] [--min N] [--update-readme README.md [--insert]]
repomap index [dir] [--no-cache] [--json]            build and print timings

repomap mcp [--root dir]                             MCP server on stdio
repomap hook                                         Claude Code PreToolUse hook (reads JSON on stdin)
repomap version
```

Any command accepts `--include-fixtures` (or `REPOMAP_INCLUDE_FIXTURES=1`) to index huge fixture trees (`tests/cases`, `testdata`, `fixtures`, `__fixtures__`, `__snapshots__` with 1,000+ files, under 10% named like tests), which are deferred by default. A target or `--path` inside one indexes it on demand; results say how many files were not analysed.

`map --tokens N` caps the map at about N tokens (estimated as 4 characters per token). It includes modules, central files and symbols in rank order and drops the lowest-ranked first (outline symbols, then key symbols, files and modules), ending with a line that says what was omitted. The MCP `map` tool takes the same `tokens` argument. Without it the map is not budgeted.

`--workspace a,b` (or a `repomap.workspace.toml`) names several repository roots for the query commands. `map`, `search`, `context`, `trace` and `impact` join across them; `db` answers once per repository. It is free and needs no licence.

`-C/--root` sets the repository for the query commands (the default is the current directory). With no arguments, and stdin not a terminal, `repomap` runs the MCP server.

## Star reminder

After the fifth successful interactive query run (`map`, `search`, `context`, `trace`, `impact`, `index`, `score`, `db`, `export`), repomap prints one line to stderr asking for a GitHub star, then never again. The run counter is the `star-hint` file in `REPOMAP_CACHE_DIR` when set, else `<user cache dir>/repomap` (or the temporary cache fallback). The hint and cache-root selection come from `sylphx-mcp-kit`. It stays silent for the MCP server and the hook, with `--json`, when stderr is not a terminal, and when `CI` is set. Set `REPOMAP_NO_STAR_HINT=1` to turn it off.
