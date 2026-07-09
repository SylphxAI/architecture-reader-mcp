# ADR-DRAFT: Synth And CodeRAG Integration

Date: 2026-07-09

## Status

Proposed

## Context

Synth provides a universal AST package family with a shared `BaseNode` shape and
many language parsers. CodeRAG already uses Synth to create semantic chunks and
owns generic code retrieval.

Architecture Reader MCP needs AST and search capability but should not own
parser internals or generic code search.

## Decision

Use Synth as the first AST substrate through public package exports. Use CodeRAG
as an optional retrieval adapter for generic code snippets, not as the canonical
architecture graph.

Architecture Reader MCP normalizes extracted facts into its own evidence graph.
It does not reach into Synth or CodeRAG private workspace internals.

## Consequences

Positive:

- Fast path to multi-language AST coverage.
- Avoids duplicating CodeRAG indexing work.
- Keeps parser and retrieval package ownership clean.

Negative:

- Architecture Reader inherits parser coverage and node-type quirks from Synth.
- Some architecture extractors may require custom normalization per language.
- Optional CodeRAG integration needs version and freshness alignment.

## Adapter Requirements

- Record Synth package name and version in evidence refs.
- Preserve source spans from Synth nodes.
- Normalize language-specific node types into architecture node/edge types.
- Treat parser failures as gaps.
- Treat CodeRAG results as retrieval candidates unless promoted by deterministic
  graph evidence.
