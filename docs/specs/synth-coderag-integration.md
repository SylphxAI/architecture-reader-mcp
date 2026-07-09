# Spec: Parser Substrates And CodeRAG Integration

Date: 2026-07-09

## Status

Planning note

## Context

Synth provides a universal AST package family with a shared `BaseNode` shape and
many language parsers. SylphxAI AST provides ANTLR-backed typed parser contracts
and grammar fixtures for the `@sylphlab/ast-*` package line. CodeRAG already
uses parser output to create semantic chunks and owns generic code retrieval.

Architecture Reader MCP needs AST and search capability but should not own
parser internals or generic code search.

## Decision

Use an explicit parser-substrate adapter layer. Synth can be the first
multi-language substrate through public package exports; SylphxAI AST can be
used where its ANTLR-backed fixtures and typed source spans better satisfy a
language contract. Use CodeRAG as an optional retrieval adapter for generic code
snippets, not as the canonical architecture graph.

Architecture Reader MCP normalizes extracted facts into its own evidence graph.
It does not reach into Synth, AST, or CodeRAG private workspace internals.

## Consequences

Positive:

- Fast path to multi-language parser coverage.
- AST-vs-Synth selection is evidence-based per language and use case.
- Avoids duplicating CodeRAG indexing work.
- Keeps parser and retrieval package ownership clean.

Negative:

- Architecture Reader inherits parser coverage and node-type quirks from the
  selected substrate.
- Some architecture extractors may require custom normalization per language.
- Optional CodeRAG integration needs version and freshness alignment.

## Adapter Requirements

- Record parser substrate package name and version in evidence refs.
- Preserve source spans from substrate nodes.
- Normalize language-specific node types into architecture node/edge types.
- Treat parser failures as gaps.
- Treat CodeRAG results as retrieval candidates unless promoted by deterministic
  graph evidence.
