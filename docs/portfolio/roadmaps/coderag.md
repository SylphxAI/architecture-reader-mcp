# Roadmap: CodeRAG

## Category Position

CodeRAG is the agent-native code retrieval engine. Its job is to return the
right code evidence fast enough that agents can reason before editing.

## Current Boundary

The current project is a Bun and TypeScript monorepo with a TypeScript core and
an MCP server exposing `codebase_search`. The target runtime is Rust for both
the retrieval engine and MCP serving.

## SOTA End-State

CodeRAG should become a hybrid lexical, semantic, and structural retrieval
engine. It should combine fast local indexing, symbol-aware chunking,
deterministic ranking, optional embeddings, and explainable results with line
and symbol provenance.

## Target Architecture

- Rust core for filesystem scanning, indexing, ranking, snippet extraction, and
  incremental updates.
- Rust MCP server using `modelcontextprotocol/rust-sdk` / `rmcp`, preserving the
  existing public tool contract during migration.
- Shared index contract that Architecture Reader can consume for structural
  context and context-pack generation.
- Optional embedding providers behind a deterministic cache and fallback lexical
  route.

## Feature Pillars

- Hybrid search: lexical, semantic, filename, symbol, dependency, and recency
  signals.
- Structural chunking: functions, classes, modules, tests, configs, and docs.
- Explainable ranking: score components and why a result was returned.
- Incremental indexing: file hash, parser version, branch, and commit aware.
- Context packs: compact, deduplicated bundles for agent prompts.
- Retrieval evals: fixture questions with expected evidence files and ranges.

## Roadmap

### Phase 0: Retrieval Contract

- Freeze `codebase_search` request and response schema.
- Add evidence envelope fields for file, line range, symbol, score, route, and
  freshness.
- Add query fixtures for exact, fuzzy, semantic, and structural searches.

### Phase 1: Rust Index Core

- Build Rust scanner and ranking library.
- Add persistent index snapshots.
- Add deterministic snippet extraction.
- Add a Rust MCP server facade for the frozen `codebase_search` contract.

### Phase 2: Hybrid Ranking

- Add lexical and symbol ranking.
- Add optional embedding cache with clear degraded mode.
- Add explainability fields for result scoring.
- Add monorepo package and ownership signals.

### Phase 3: Agent Retrieval Workflows

- Add context-pack mode for "explain", "edit", "review", and "test" tasks.
- Add stale index warnings and recommended refresh calls.
- Add integration route for Architecture Reader graph ids.

### Phase 4: Public Performance Positioning

- Publish benchmark suite for cold index, incremental index, search p95, and
  memory ceiling.
- Ship native binary packages.
- Make the Rust MCP server the canonical package entrypoint.

## Validation Gates

- Fixture queries hit expected files and line ranges.
- Repeated indexing produces deterministic snapshots.
- Search returns freshness and route warnings.
- p95 search latency stays inside published budget.
- Install diagnostics explain missing native engine.

## ADRs To Land In CodeRAG

- Rust index core migration.
- Ranking signal model.
- Embedding cache and degraded-mode policy.
- Architecture Reader interoperability.
- Rust MCP server and native binary packaging.
