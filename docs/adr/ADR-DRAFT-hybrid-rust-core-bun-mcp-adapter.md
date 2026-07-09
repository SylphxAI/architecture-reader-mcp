# ADR-DRAFT: Hybrid Rust Core With Bun MCP Adapter

Date: 2026-07-09

## Status

Proposed

## Context

The project needs both high-performance repository indexing and fast integration
with existing Sylphx MCP and AST/search packages.

Evidence:

- Synth and CodeRAG are TypeScript-facing packages today.
- Existing Reader MCP repositories use TypeScript/Bun conventions.
- Architecture graph traversal, ranking, and impact analysis are engine work
  that benefits from Rust performance and memory discipline.
- Doctrine engineering guidance favors Rust for performance-critical services,
  CLIs, runtimes, and parsers.

## Decision

Use a hybrid architecture:

- Rust core owns graph contracts, query planning, traversal, ranking, impact
  analysis, storage abstraction, and index validation.
- TypeScript/Bun MCP adapter owns MCP protocol, tool schemas, existing package
  integration, and response shaping.

The adapter can call the Rust engine through CLI, JSON-RPC, N-API, or WASM after
the first implementation spike measures ergonomics. The engine boundary is
defined now so the adapter cannot absorb core logic by accident.

## Consequences

Positive:

- Reuses Synth and CodeRAG without rewriting their public surfaces.
- Keeps the performance-critical architecture engine portable.
- Allows future CLI, CI, or service adapters without MCP coupling.

Negative:

- Two-language repo increases build and release complexity.
- Boundary tests are required to prevent logic from leaking into the adapter.
- First slice may start more slowly than all-Bun.

## If Forced To Choose One Runtime For MVP

Choose Bun/TypeScript for the first runnable MCP slice because current Sylphx
MCP, Synth, and CodeRAG surfaces are TypeScript-facing. Keep Rust workspace and
engine contracts in place from day one. Move heavy graph/index/query execution
into Rust as soon as graph model tests stabilize.

## Rejected Alternatives

### All Rust

Rejected for MVP because it would duplicate existing TypeScript-facing Synth and
MCP package integration before product shape is proven.

### All Bun/TypeScript

Rejected for the long-term engine because large repository graph traversal,
incremental indexing, and ranking should not be tied to the adapter runtime.
