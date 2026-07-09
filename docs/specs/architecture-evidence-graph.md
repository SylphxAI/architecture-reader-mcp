# Spec: Architecture Evidence Graph

Date: 2026-07-09

## Status

Planning note

## Context

Architecture questions require relationships across files, symbols, packages,
routes, schemas, workflows, docs, and decisions. Search snippets alone do not
preserve enough structure. Pure LLM summaries are not sufficiently auditable.

Source-code fact systems and code property graph systems show the value of
durable facts and graph relationships. Visualization-first codebase graph
products show demand for architecture learning, but their primary surface is
visual exploration rather than agent-native evidence retrieval.

## Decision

Use an architecture evidence graph as the canonical model. Store nodes, edges,
claims, and evidence separately:

- nodes represent architecture entities;
- edges represent relationships;
- claims represent derived architecture statements;
- evidence references prove nodes, edges, and claims.

Every edge and claim must reference evidence.

## Consequences

Positive:

- Answers can cite exact proof.
- Derived claims remain auditable.
- Trace and impact tools can operate on graph paths.
- Conflicts can be represented instead of overwritten.

Negative:

- Graph normalization is harder than simple text search.
- Extractor quality directly affects answer quality.
- Versioning and migration are required as the schema evolves.

## Alternatives Considered

### Text Chunks Only

Rejected. Text chunks are useful retrieval candidates, but they cannot be the
canonical architecture model.

### LLM Summary Cache

Rejected. Summary caches drift and lack durable evidence unless backed by a
graph.
