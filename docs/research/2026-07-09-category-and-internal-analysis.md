# Category And Internal Analysis

Date: 2026-07-09

This note records evidence used to shape the initial Architecture Reader MCP
design. It is research input, not the product SSOT; durable decisions live in
`docs/adr/`.

## Internal Assets

### Reader MCP Portfolio

Observed Sylphx Reader MCP repositories cover document/media reading:

- `pdf-reader-mcp` has mature PDF reading, search, evidence, provenance, and
  Agent Document Twin concepts.
- `image-reader-mcp` reads images.
- `video-reader-mcp` reads video.
- `smart-reader-mcp` delegates to the format-specific readers.

These are adjacent but not the owner of repository architecture indexing.

### CodeRAG

Observed repository: `work/repos/coderag`.

CodeRAG owns generic codebase search through a single MCP tool,
`codebase_search`. It uses chunk-level indexing, persistent storage, TF-IDF/BM25
style retrieval, optional embeddings, file watching, and Synth AST chunking.

Important implementation evidence:

- `packages/mcp-server/src/index.ts` exposes `codebase_search`.
- `packages/core/src/ast-chunking.ts` loads Synth parsers dynamically and chunks
  by language semantic boundaries.
- `packages/core/src/language-config.ts` maps languages to Synth parser packages
  and boundary node types.
- `packages/core/src/indexer.ts` owns indexing, incremental detection, chunk
  storage, file watching, and search status.

Conclusion: Architecture Reader MCP must not replace CodeRAG. It can call or
reuse it for generic chunk retrieval, but architecture graph ownership belongs
in the new repository.

### Synth

Observed repository: `work/repos/synth`; public repo metadata confirms
`SylphxAI/synth` is active. npm readback returned `@sylphx/synth@0.3.2` and
`@sylphx/synth-rust@0.3.1`.

Synth owns the `@sylphx/synth*` parser/tool package family. `PROJECT.md` states
that Synth owns AST contracts, parser implementations, package exports, docs,
and benchmark/tooling surfaces directly tied to Synth packages.

Important implementation evidence:

- README describes a universal AST interface for 19+ languages.
- `packages/synth/src/types/tree.ts` stores nodes in a flat arena array for
  cache locality.
- `packages/synth/src/types/node.ts` defines the common `BaseNode` contract.
- `packages/synth-rust/src/parser.ts` converts Rust tree-sitter output into the
  Synth universal AST via async WASM parsing.

Conclusion: Synth is the best current Sylphx parser substrate for a first
Architecture Reader implementation.

### SylphxAI/ast

Observed repository: `/Users/kyle/ast`, remote `https://github.com/SylphxAI/ast`.

The local checkout is on `codex/adr81-selector-facts`, one commit ahead of
`origin/main`. The project is a TypeScript monorepo for AST parsing tools,
currently centered on JavaScript parsing through ANTLR and shared CST/AST core
interfaces.

Important implementation evidence:

- `PROJECT.md` says it starts with JavaScript parsing via ANTLR.
- `packages/javascript/src/index.ts` exports `parseJavaScript`.
- `packages/core/src/index.ts` defines generic CST interfaces but still contains
  refinement comments and placeholder exports.

Conclusion: `SylphxAI/ast` is useful as internal AST knowledge, but it is not
yet the best multi-language substrate for this product. Use Synth first; keep an
adapter boundary so `SylphxAI/ast` can provide JavaScript-specific extraction if
that package matures.

### Visualization-First Codebase Graph Category

Category research shows demand for repository scanning, graph-shaped knowledge,
incremental updates, domain extraction, documentation awareness, and human
exploration surfaces.

Important design lessons:

- Graph schemas need typed nodes and typed edges.
- Repository scans should be incremental and resumable.
- Domain entities such as services, endpoints, schemas, jobs, configs, and
  resources matter more than raw files alone.
- Search over graph nodes is useful, but evidence-backed agent answers need a
  stronger contract than fuzzy node lookup.
- Multi-phase scan pipelines are useful when they expose progress, coverage,
  and reviewable evidence.

Conclusion: Useful category signals are graph schema, incremental scan, diff
impact, domain extraction, and documentation awareness. The Sylphx target is
stronger and narrower: a typed MCP answer surface for agents, backed by source
evidence.

## External Category Signals

External category research was used as background input. The product documents
do not anchor on external project names because the durable decision is what
Architecture Reader MCP must become.

Design lessons:

- Parser substrate must support incremental, multi-language parsing.
- Architecture facts should be language-agnostic and graph-shaped.
- Raw facts and derived facts must stay separate.
- Derived facts must keep derivation provenance.
- Structural matching is powerful when syntax patterns are user-facing.
- Rust plus AST structural query is a strong performance direction.
- Graph paths can answer impact, dependency, and data/control relationship
  questions.

## Product Gap

The gap is not another parser, another generic code search tool, or another
dashboard. The gap is an agent-facing architecture evidence service:

- canonical graph of architecture-level entities and relationships;
- deterministic evidence first, inference second and labeled;
- query tools shaped for AI agent workflows;
- compact answers with file/line proof;
- freshness and confidence exposed in every response.

## Stack Judgment

Rust is the right home for both the heavy engine and the MCP server: graph
traversal, ranking, incremental indexes, large-repository performance,
CLI/service reuse, stdio serving, and future streamable HTTP support.

Existing Sylphx MCP patterns, Synth packages, and CodeRAG are
TypeScript-facing today. They should be consumed through stable contracts,
fixtures, generated metadata, or process boundaries rather than by making
TypeScript the MCP adapter runtime.

The strongest design is Rust-native, with a narrow core/server boundary and
explicit integration contracts for TypeScript-facing sibling packages.
