# Architecture Reader MCP

Architecture Reader MCP is a SylphxAI MCP repository for agent-native repository
architecture understanding. It builds and serves a queryable architecture
evidence graph so AI agents can ask how a project is structured, where
boundaries live, how components depend on each other, and what files prove each
answer.

Project identity is split by boundary: vendor-neutral project facts live in
[`project.manifest.json`](./project.manifest.json), while Sylphx-specific
governance facts live in [`.doctrine/project.json`](./.doctrine/project.json).

## Lifecycle And Layer

- Lifecycle: `beta`
- Layer: `application`
- Delivery state: Beta 0.1 — Rust evidence-graph engine and Bun MCP adapter ship locally with release-gate proof; npm publish remains open

## Goals

- Provide an MCP server for architecture overview, architecture search,
  evidence lookup, dependency tracing, and impact analysis.
- Build an architecture evidence graph from deterministic sources first:
  manifests, package/workspace metadata, AST/symbol extraction, import graphs,
  routes, schemas, workflows, docs, and ADRs.
- Integrate with Sylphx parser/search assets through public package surfaces:
  Synth for universal AST parsing and CodeRAG for generic code retrieval where
  useful.
- Return agent-readable answers with file paths, line ranges, evidence IDs,
  extraction source, freshness, confidence, and known uncertainty.

## Non-Goals

- This repository is not a replacement for CodeRAG generic code search.
- This repository is not a fork of Synth or `SylphxAI/ast`.
- This repository is not a visualization-first dashboard product.
- This repository does not own Reader portfolio media extraction behavior.
- This repository does not own Sylphx doctrine, shared CI, or deployment
  infrastructure.

## Boundary Summary

The project owns the architecture evidence graph schema, architecture indexing
pipeline, query planner, MCP tool surface, and architecture answer contracts.
Parser packages, generic code search, dashboard UX, model providers, and
external platform runtime are consumed through stable public interfaces and
remain owned by their source repositories.

## Public Surfaces

- README: [`README.md`](./README.md)
- Architecture overview: [`docs/architecture.md`](./docs/architecture.md)
- Specifications: [`docs/specs/`](./docs/specs/)
- ADRs: [`docs/adr/`](./docs/adr/)
- Portfolio plan: [`docs/portfolio/`](./docs/portfolio/)
- Security boundary: [`SECURITY.md`](./SECURITY.md)
- MCP server metadata: [`server.json`](./server.json)
- Rust engine workspace: [`crates/`](./crates/)
- TypeScript MCP adapter workspace: [`packages/mcp-server/`](./packages/mcp-server/)
- Baseline CI workflow: [`.github/workflows/ci.yml`](./.github/workflows/ci.yml)

## Delivery Proof

Beta 0.1 ships a runnable Rust core (`architecture-reader-core`), stdio CLI,
and thin Bun MCP adapter with seven `architecture_*` tools. Local proof includes
`cargo test`, `bun test`, and `bun run benchmark:release-gate` on the fixture
repository (routes, schemas, symbol call tracing, Python adapter, incremental
index). Remaining adoption gaps: npm publish workflow, MCP Registry listing, and
protected-branch release automation.
