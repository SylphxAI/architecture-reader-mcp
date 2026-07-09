# Competitive And Internal Analysis

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

### Dashboard-First Codebase Graph Competitor

Observed a public plugin/dashboard product in the codebase-graph category. Its
README describes a multi-agent pipeline that scans projects, builds a local
knowledge graph JSON artifact, and serves an interactive dashboard.

Important implementation evidence:

- Its core package defines node and edge types for a knowledge graph.
- Its graph builder creates file, function, class, service, endpoint, schema,
  pipeline, and resource nodes.
- Its search module uses fuzzy search over graph nodes.
- Its skill workflow defines multi-phase scan, batch, analyze, assemble, review,
  and persist behavior.

Conclusion: Useful category signals are graph schema, incremental scan, diff
impact, domain extraction, and documentation awareness. Weak fit for the Sylphx
target: visualization and plugin workflow are primary, while our target is a
typed MCP answer surface for agents. Architecture Reader MCP should compete by
owning the agent-native architecture evidence category instead of advertising or
anchoring itself to a named competitor.

## External Systems

External references were checked through public GitHub metadata on 2026-07-09.

| System | Observed description | Design lesson |
| --- | --- | --- |
| `tree-sitter/tree-sitter` | Incremental parsing system for programming tools, Rust implementation | Parser substrate must support incremental, multi-language parsing. |
| `kythe/kythe` | Pluggable mostly language-agnostic ecosystem for code tools | Architecture facts should be language-agnostic and graph-shaped. |
| `facebookincubator/Glean` | Collecting, deriving, and working with facts about source code | Separate raw facts from derived facts and keep derivation provenance. |
| `github/codeql` | Libraries and queries powering code scanning | Rich query languages and derived analyses matter, but security query semantics are out of scope. |
| `semgrep/semgrep` | Lightweight static analysis across many languages | Structural matching is powerful when syntax patterns are user-facing. |
| `ast-grep/ast-grep` | Rust CLI for structural search, lint, and rewriting | Rust plus AST structural query is a strong performance precedent. |
| `joernio/joern` | Code property graphs across multiple languages | Graph paths can answer impact and data/control relationship questions. |

## Product Gap

The gap is not another parser, another generic code search tool, or another
dashboard. The gap is an agent-facing architecture evidence service:

- canonical graph of architecture-level entities and relationships;
- deterministic evidence first, inference second and labeled;
- query tools shaped for AI agent workflows;
- compact answers with file/line proof;
- freshness and confidence exposed in every response.

## Stack Judgment

Rust is the right long-term home for the heavy engine: graph traversal, ranking,
incremental indexes, large-repository performance, and CLI/service reuse.

Bun/TypeScript is the right adapter and first integration layer because existing
Sylphx MCP patterns, Synth packages, and CodeRAG are TypeScript-facing today.

The strongest design is hybrid, with a narrow engine boundary and a thin MCP
adapter.
