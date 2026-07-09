# Architecture Reader MCP

Architecture Reader MCP is an agent-native MCP server for understanding project
architecture from source evidence. It is designed for AI agents that need to
answer questions like:

- What are the major architecture boundaries in this repository?
- Which files prove how authentication, billing, routing, deployment, or storage
  works?
- What depends on this module, route, schema, workflow, or service?
- What is the impact radius of this diff?
- Which architecture claim is deterministic, inferred, stale, or uncertain?

The product is not visualization-first. It may export graph data for dashboards,
but its primary interface is a compact, typed, provenance-rich MCP tool surface
for agents.

## Current Delivery State

This repository is a local scaffold and architecture design package. It is not
published, merged, released, deployed, or production-verified.

## Why This Exists

Adjacent tools cover important slices but leave an agent-specific gap:

- CodeRAG answers generic code search over chunks.
- Synth provides a universal AST package family across many languages.
- SylphxAI AST provides ANTLR-backed typed parser contracts and fixtures.
- Visualization-first codebase graph tools optimize for human exploration.
- Source-code fact systems model code as durable facts.
- Static analysis systems provide structural query and code property graph
  patterns.

Architecture Reader MCP claims the stronger agent-native category: fast,
evidence-backed architecture answers for AI agents, with deterministic
extraction first and LLM inference explicitly labeled when used. The goal is not
to imitate visual graph tools; it is to make them less necessary for serious AI
engineering workflows.

## Recommended Implementation Stack

Use a Rust-native stack:

- Rust core for the architecture graph engine, index formats, query planning,
  traversal, ranking, diff/impact computation, and high-volume repository work.
- Rust MCP server crate using the official
  [`modelcontextprotocol/rust-sdk`](https://github.com/modelcontextprotocol/rust-sdk)
  `rmcp` crate for protocol handling, stdio transport, typed tool schemas, and
  future streamable HTTP support.

TypeScript can remain a consumer language for generated clients, fixtures, or
npm wrapper metadata, but the MCP server runtime and product logic should not be
implemented as a TypeScript adapter.

See [`docs/portfolio/notes/rust-first-runtime-distribution.md`](./docs/portfolio/notes/rust-first-runtime-distribution.md).

## Tool Surface

Initial MCP tools:

- `architecture_index` - create or refresh the local architecture index.
- `architecture_status` - report index freshness, git commit, coverage, and
  known gaps.
- `architecture_overview` - return the top architecture map for a repository or
  subpath.
- `architecture_search` - find components, boundaries, routes, schemas,
  workflows, decisions, and architecture concepts.
- `architecture_trace` - trace dependency, call, route, ownership, or evidence
  paths between nodes.
- `architecture_impact` - estimate impact radius for changed files or symbols.
- `architecture_evidence` - fetch exact evidence behind a node, edge, or claim.

Every answer must include provenance: file path, optional line range, extraction
source, freshness, confidence, and uncertainty.

## Repository Layout

```text
architecture-reader-mcp/
  crates/
    architecture-reader-core/    # Rust architecture graph contracts and engine
    architecture-reader-mcp/     # Rust MCP server using rmcp
  docs/
    adr/                         # Architecture decisions
    specs/                       # Product, graph, indexing, and tool specs
    research/                    # Evidence and category analysis
    portfolio/                   # MCP portfolio ADRs and roadmaps
  server.json                    # MCP server metadata
```

## Design Documents

- [Architecture](./docs/architecture.md)
- [Product Spec](./docs/specs/2026-07-09-product-spec.md)
- [Tool Contract](./docs/specs/2026-07-09-tool-contract.md)
- [Evidence Graph Spec](./docs/specs/2026-07-09-evidence-graph.md)
- [Indexing Pipeline Spec](./docs/specs/2026-07-09-indexing-pipeline.md)
- [Category And Internal Research](./docs/research/2026-07-09-category-and-internal-analysis.md)
- [MCP Portfolio Plan](./docs/portfolio/README.md)
- [SOTA Family Roadmap](./docs/roadmap/sota-family-roadmap.md)

## Development

This scaffold intentionally contains minimal code. The next implementation slice
should add:

1. Rust graph model serialization tests.
2. Rust MCP server tool schemas and stubbed `rmcp` handlers.
3. Deterministic repository scanner for manifests, workspaces, docs, and import
   edges.
4. Parser-substrate extractor adapter with Synth and AST fixture support.
5. Golden-fixture tests against small TypeScript and Rust repositories.

Local validation for this scaffold:

```bash
python3 -m json.tool .doctrine/project.json
python3 -m json.tool project.manifest.json
cargo metadata --format-version 1
git diff --check
```
