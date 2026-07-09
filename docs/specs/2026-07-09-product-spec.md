# Product Spec

## Product

Architecture Reader MCP is an MCP server for AI agents that need to understand a
repository's architecture from source evidence.

## Primary Users

- Coding agents entering an unfamiliar repository.
- Review agents checking impact, boundaries, and architecture consistency.
- Delivery agents deciding where a feature or bug fix belongs.
- Documentation agents generating source-backed architecture summaries.

## Jobs To Be Done

1. Produce a compact overview of the architecture for a repo or subpath.
2. Find architecture entities by natural language, symbol, route, package,
   service, schema, workflow, or file.
3. Trace relationships between components, files, symbols, packages, services,
   schemas, and docs.
4. Explain the impact radius of a diff or changed file set.
5. Return exact evidence for every architecture claim.
6. Report freshness, coverage, gaps, and uncertainty.

## Product Principles

- Evidence first: deterministic facts outrank inferred summaries.
- Agent-native: return machine-readable objects before prose.
- Compact by default: answers fit within agent context budgets.
- Drilldown ready: every summary links to nodes, edges, and evidence refs.
- Boundary-aware: do not duplicate CodeRAG, Synth, Reader, or doctrine scope.
- Freshness-aware: stale or dirty indexes are visible in every answer.

## Scope

### In Scope

- local repository indexing;
- project/package/workspace detection;
- AST-backed symbols, imports, exports, routes, and structural entities;
- docs, ADRs, schemas, CI/workflows, infra/config extraction;
- architecture graph storage and query;
- MCP tools for overview, search, trace, impact, status, and evidence;
- optional integration with CodeRAG for generic code retrieval.

### Out Of Scope

- full visualization dashboard;
- generic code search as the primary product;
- parser package internals;
- code rewriting or automated refactoring;
- security vulnerability analysis;
- production deployment control plane.

## Success Criteria

The first usable release should prove:

- index a TypeScript repository and a Rust repository from clean checkout;
- return architecture overview with evidence refs;
- search for at least package, file, symbol, route, workflow, and doc nodes;
- trace import/dependency paths between two architecture nodes;
- compute impact radius for a changed file list;
- report stale index when git commit changed;
- expose MCP schemas that agents can call without reading prose docs.

## Non-Functional Requirements

- deterministic extraction works without model credentials;
- local-first storage;
- bounded memory on large repositories;
- incremental refresh for changed files;
- stable JSON response contracts;
- extractor version recorded in every index;
- no silent network calls during indexing unless explicitly configured.

## Delivery Phases

### Phase 0: Design Scaffold

Create repository identity, specs, ADRs, and a minimal Rust workspace with core
and MCP server crates.

### Phase 1: Deterministic Graph MVP

Implement repository scan, manifest extraction, docs/ADR extraction,
parser-substrate symbol extraction, graph serialization, and MCP
status/overview.

### Phase 2: Query Tools

Implement architecture search, evidence lookup, trace, impact, ranking, and
golden fixture tests.

### Phase 3: Scale And Quality

Add incremental refresh, large repo benchmarks, CodeRAG adapter, semantic
ranking, and cross-repo evidence packs.

### Phase 4: Release

Publish through normal Sylphx repository path with CI, package release, MCP
install proof, and representative repo readback.
