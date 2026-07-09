# Roadmap: Architecture Reader MCP

## Category Position

Architecture Reader MCP is the agent-native architecture understanding engine.
It should let an AI agent inspect a repository, ask architectural questions, and
receive evidence-backed answers about components, dependencies, boundaries,
routes, schemas, ownership, and impact.

## SOTA End-State

The final product is a local architecture evidence graph with fast incremental
indexing, precise trace queries, impact analysis, and stable evidence locators.
It is not primarily a visualization tool. Its primary user is an AI agent that
needs high-confidence structural context before editing, reviewing, or planning.

## Target Architecture

- Rust core for scanning, parsing, graph construction, search, and impact
  analysis.
- Rust MCP server using `modelcontextprotocol/rust-sdk` / `rmcp` from the first
  implementation slice.
- Shared evidence envelope across every tool.
- Incremental index keyed by git commit, file hash, language, parser version,
  and extractor version.
- Optional integration with code retrieval and AST substrates through stable
  contracts.

## Tool Surface

- `architecture_index`
- `architecture_status`
- `architecture_overview`
- `architecture_search`
- `architecture_trace`
- `architecture_impact`
- `architecture_evidence`

## Feature Pillars

- Repository architecture map: components, boundaries, modules, routes, schemas,
  configs, tests, jobs, and ownership.
- Evidence graph: every node and edge links to source files, line ranges,
  symbols, commits, and extractor route.
- Trace: dependency, call, route, data-flow, configuration, and ownership paths.
- Impact: changed files, changed symbols, changed contracts, affected tests,
  affected runtime entrypoints.
- Agent query plans: answer with coverage, gaps, and next tool calls.
- Drift detection: stale index, moved code, deleted symbols, and changed
  contracts.

## Roadmap

### Phase 0: Contract Hardening

- Freeze tool schemas and evidence envelope.
- Add golden fixtures for a small polyglot repo.
- Add `architecture_status` freshness and coverage diagnostics.
- Publish install and fixture demo.

### Phase 1: Rust Graph Core

- Implement Rust scanner, graph store, and query planner.
- Add language adapters for TypeScript, Rust, Python, JSON/YAML, and package
  manifests.
- Add deterministic graph snapshots and diff tests.
- Benchmark cold start, full index, incremental index, and query latency.

### Phase 2: Trace And Impact Engine

- Add symbol-aware dependency tracing.
- Add route, schema, workflow, and config extraction.
- Add changed-file impact analysis.
- Add missing-coverage warnings for unsupported languages or dynamic edges.

### Phase 3: Agent Workflow Integration

- Add planning summaries for common agent tasks: onboarding, review, refactor,
  incident triage, release readiness.
- Add integration hooks for code search and document evidence.
- Add compact context packs for agent token budgets.

### Phase 4: Rust-Native Server And Release Scale

- Ship the Rust MCP server as the canonical runtime.
- Ship optional binary npm packages and standalone binaries.
- Add public benchmark suite and release scorecard.

## Validation Gates

- Fixture queries return stable evidence locators.
- Incremental indexing updates only touched graph regions.
- Impact analysis includes confidence and known gaps.
- Tool latency meets published p95 gates on fixture repos.
- Install works across supported platforms without network postinstall downloads.

## ADRs To Land In This Repo

- Rust graph storage format.
- Language adapter contract.
- Incremental index invalidation.
- Rust MCP server tool contract.
- Cross-project integration with code retrieval.
