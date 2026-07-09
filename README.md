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

Existing tools cover important slices but leave an agent-specific gap:

- CodeRAG answers generic code search over chunks.
- Synth provides a universal AST substrate across many languages.
- Dashboard-first codebase graph tools build interactive knowledge graphs for
  humans.
- Source-code fact systems such as Kythe and Glean model code as facts.
- Static analysis systems such as CodeQL, Semgrep, ast-grep, and Joern provide
  structural query or code property graph patterns.

Architecture Reader MCP claims the stronger agent-native category: fast,
evidence-backed architecture answers for AI agents, with deterministic
extraction first and LLM inference explicitly labeled when used. The goal is not
to imitate visual graph tools; it is to make them less necessary for serious AI
engineering workflows.

## Recommended Implementation Stack

Use a hybrid stack:

- Rust core for the architecture graph engine, index formats, query planning,
  traversal, ranking, diff/impact computation, and high-volume repository work.
- TypeScript/Bun MCP adapter for MCP protocol ergonomics, existing Sylphx MCP
  conventions, and first-class integration with Synth and CodeRAG package
  surfaces.

If forced to ship a first slice in one runtime, start with Bun/TypeScript because
the internal parser/search/MCP packages are already TypeScript-facing. Keep the
Rust core boundary in the repo from day one so the performance-critical engine
does not become coupled to the MCP adapter.

See [`docs/adr/ADR-DRAFT-hybrid-rust-core-bun-mcp-adapter.md`](./docs/adr/ADR-DRAFT-hybrid-rust-core-bun-mcp-adapter.md).

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
  packages/
    mcp-server/                  # TypeScript/Bun MCP adapter
  docs/
    adr/                         # Architecture decisions
    specs/                       # Product, graph, indexing, and tool specs
    research/                    # Evidence and competitive analysis
  server.json                    # MCP server metadata
```

## Design Documents

- [Architecture](./docs/architecture.md)
- [Product Spec](./docs/specs/2026-07-09-product-spec.md)
- [Tool Contract](./docs/specs/2026-07-09-tool-contract.md)
- [Evidence Graph Spec](./docs/specs/2026-07-09-evidence-graph.md)
- [Indexing Pipeline Spec](./docs/specs/2026-07-09-indexing-pipeline.md)
- [Competitive And Internal Research](./docs/research/2026-07-09-competitive-and-internal-analysis.md)

## Development

This scaffold intentionally contains minimal code. The next implementation slice
should add:

1. Rust graph model serialization tests.
2. Bun MCP adapter with tool schemas and stubbed handlers.
3. Deterministic repository scanner for manifests, workspaces, docs, and import
   edges.
4. Synth-backed AST extractor adapter.
5. Golden-fixture tests against small TypeScript and Rust repositories.

Local validation for this scaffold:

```bash
python3 -m json.tool .doctrine/project.json
python3 -m json.tool project.manifest.json
cargo metadata --format-version 1
git diff --check
```
