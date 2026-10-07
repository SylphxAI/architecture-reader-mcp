# repomap vision

## What we are building

repomap gives AI coding agents, and the people who work with them, a map of a
codebase: its modules, the files and symbols that matter, how calls flow, and
what a change will break. It parses the code itself with tree-sitter, builds
one graph, and answers in a few hundred tokens and a few milliseconds.

An agent that is about to change a function should know who calls it, which
modules it touches and which tests to run, before it edits, not after review.
That is the one job.

## Who it is for

- Developers who use an AI coding agent (Claude Code, Codex, Cursor, VS Code,
  Windsurf, Gemini CLI) on codebases too large to read into a prompt.
- Teams that want every pull request to show its blast radius, and that work
  across more than one repository.

## Boundaries

- Local first: one binary, no account and no API key. Indexing and queries make
  no network calls; the only download is the embedding model, once.
- Read-only: repomap never changes the code it maps. Live database connections
  stay read-only and never store or print the connection string.
- Six MCP tools: `map`, `search`, `context`, `trace`, `impact`, `db`. New
  abilities, Team included, go into those tools, not into more tools.
- Build-free: answers come from parsing, never from running or building the
  project. Compiler-grade enrichment (SCIP, language servers) may be added
  where a build already exists, and the default stays build-free.
- Local by default: everything runs on the user's machine. A hosted part, if
  one is ever added, runs on Sylphx and is paid as metered use of that service,
  never as a licence key on the local tool.

## Free

Everything repomap does is free under MIT, including the multi-repository
workspace and `impact` in your own CI. There is no licence key and no paid
tier; a capability never moves from free to paid.

## What good looks like

- Search leads the published benchmarks (SWE-bench Verified localisation
  Acc@10, NDCG@10 per language), measured in CI on GitHub-hosted runners.
- A first map of a large repository takes seconds, a query takes milliseconds,
  and a map fits the token budget the agent asks for.
- `impact` names the callers and tests a change actually reaches, measured
  against a caller/callee gold set.
- A pull request review appears within a minute of the push.

Current capabilities and their code: [capabilities.md](capabilities.md).
