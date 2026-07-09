# SOTA Family Roadmap

Status: adoption plan
Owner: Architecture Reader MCP
Scope: repo-local future plan and its role in the SylphxAI MCP family
Decision record: `docs/adr/ADR-2-mcp-family-sota-roadmap.md`

## Family Role

Architecture Reader MCP is the architecture intelligence member of the MCP
family. It answers questions about system shape: boundaries, components,
dependencies, routes, schemas, workflows, ownership, drift, and impact.

It does not replace CodeRAG, Reader MCPs, Filesystem MCP, or Consultant MCP.
It consumes their outputs where useful and turns repository structure into an
evidence graph that agents can query before planning or editing.

## Family Fit

| Project | Relationship |
| --- | --- |
| CodeRAG | Provides code retrieval candidates and context packs. Architecture Reader owns graph facts, traces, and impact semantics. |
| GroundAtlas | Provides source-truth routing, manifest semantics, freshness gates, and fleet orientation. Architecture Reader owns architecture graph extraction and MCP answer contracts. |
| Filesystem MCP | Provides safe file access and write operations. Architecture Reader may recommend affected files but does not edit them. |
| Reader MCPs | Provide evidence from PDFs, images, videos, and media docs that live beside source code. Architecture Reader links that evidence to repo concepts when relevant. |
| Codec | Provides reusable media codec and conversion primitives that Reader MCPs can consume. Architecture Reader does not own media codec behavior. |
| Consultant MCP | Reviews architecture decisions and roadmap tradeoffs using Architecture Reader evidence as input. |
| Smart Reader MCP | Routes attached design artifacts and repo media into reader-specific evidence. |

## SOTA End State

The product should become the default architecture context provider for AI
agents. A strong agent should call it before large edits, reviews, migrations,
incident analysis, or release-risk assessment.

The final form is a Rust-first local graph engine with:

- incremental repository indexing;
- typed architecture graph nodes and edges;
- evidence locators for every node, edge, and derived claim;
- trace queries across dependency, call, route, schema, workflow, and ownership
  relationships;
- impact analysis for changed files and symbols;
- compact agent context packs;
- native binary distribution through npm optional packages;
- a direct Rust MCP server using the official `modelcontextprotocol/rust-sdk`
  `rmcp` crate.

## Runtime Direction

Rust owns graph construction, storage, traversal, ranking, incremental indexing,
and MCP serving. The MCP runtime target is the official
`modelcontextprotocol/rust-sdk` `rmcp` crate. TypeScript can generate clients,
fixtures, or npm wrapper metadata, but it is not the MCP adapter runtime. WASM is
reserved for sandboxed extractors and portable plugins, not the default local
server runtime.

## Roadmap

### Phase 0: Contract And Fixtures

- Freeze the tool contracts for `architecture_index`, `architecture_status`,
  `architecture_overview`, `architecture_search`, `architecture_trace`,
  `architecture_impact`, and `architecture_evidence`.
- Add a small polyglot repository fixture with expected graph facts.
- Define the shared evidence envelope fields used by graph responses.
- Add install diagnostics to the Rust MCP server and npm wrapper.

### Phase 1: Rust Graph Core

- Implement the graph model, extractor registry, and snapshot format in Rust.
- Add deterministic graph snapshot tests.
- Add TypeScript, Rust, JSON/YAML, package manifest, and workflow extractors.
- Add benchmark gates for full index, incremental index, graph query, and
  memory ceiling.

### Phase 2: Trace And Impact

- Add symbol-aware dependency traces.
- Add route, schema, config, workflow, and test ownership extraction.
- Add changed-file and changed-symbol impact analysis.
- Return confidence and known gaps for every inferred edge.

### Phase 3: Cross-Project Context Packs

- Consume CodeRAG retrieval candidates where graph coverage is incomplete.
- Link Reader evidence for repo-adjacent design documents.
- Produce compact context packs for edit planning, review, incident, and
  migration workflows.

### Phase 4: Native Rust MCP Distribution

- Ship platform-specific npm optional binary packages.
- Add standalone release binaries and a `doctor` command.
- Gate stdio behavior, generated schemas, logging, and release evidence directly
  against the Rust MCP server.

## Star And Adoption Strategy

The public promise is simple: "ask your codebase how it is built." The README
must lead with a one-command demo that returns a real architecture answer with
file and line proof. Star growth comes from instant value, public benchmarks,
clear agent examples, and trustable evidence rather than a dashboard-first
experience.

## Validation Gates

- Every public claim in the README is backed by a command, test, benchmark, or
  fixture.
- Graph snapshots are deterministic across repeated runs.
- Trace and impact outputs expose evidence and uncertainty.
- Install succeeds on supported platforms without network postinstall binary
  downloads.
- Rust MCP server fixtures match the core graph contract.
