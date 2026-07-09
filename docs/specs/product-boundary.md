# Spec: Product Boundary

Date: 2026-07-09

## Status

Planning note

## Context

Sylphx has existing repositories for Reader MCP tools, CodeRAG, Synth, and
`SylphxAI/ast`. The new product should help AI agents understand project
architecture. The risk is overlapping existing ownership:

- CodeRAG already owns generic code chunk search.
- Synth already owns universal AST parser packages.
- `SylphxAI/ast` owns ANTLR/TypeScript AST tooling, currently JavaScript-first.
- Reader MCP repositories own document and media readers.
- Visualization-first codebase graph tools optimize for human visual exploration
  before agent tool use.

## Decision

Architecture Reader MCP owns architecture-level evidence graphs and MCP answer
contracts for agents.

It does not own parser internals, generic code search, visual dashboard UX,
media extraction, or doctrine.

## Consequences

Positive:

- Clear gap: architecture answers, not raw code search.
- Existing Sylphx packages can be reused through stable public surfaces.
- Agent response quality can be measured through evidence and freshness.

Negative:

- Requires careful integration boundaries with CodeRAG and Synth.
- Early product value depends on graph schema quality, not only parser coverage.

## Alternatives Considered

### Extend CodeRAG

Rejected. CodeRAG's primary shape is chunk search. Architecture graph queries,
impact paths, and provenance-rich claims would broaden its scope.

### Build A Visualization-First Product

Rejected. The user target is MCP for agents, not a visualization-first plugin
and dashboard product.

### Build A Parser Repository

Rejected. Synth and `SylphxAI/ast` already own parser-package concerns.
