# Spec: Rust-Native MCP Server

Date: 2026-07-09

## Status

Planning note

## Context

Architecture Reader MCP needs high-performance repository indexing and a stable
agent protocol surface. The original design considered a TypeScript/Bun MCP
adapter over a Rust core because several Sylphx packages are currently
TypeScript-facing.

The product target has changed: the MCP server itself should be Rust. The
official `modelcontextprotocol/rust-sdk` now provides the `rmcp` crate with
server support, typed tool schemas, stdio transport, examples for streamable
HTTP, and a direct Rust protocol path.

## Decision

Use a Rust-native architecture:

- `architecture-reader-core` owns graph contracts, query planning, traversal,
  ranking, impact analysis, storage abstraction, and index validation.
- `architecture-reader-mcp` owns MCP serving through `rmcp`, including tool
  registration, request validation, response shaping, stdio transport, logging,
  and future streamable HTTP support.
- TypeScript is allowed only for generated consumers, fixtures, npm wrapper
  metadata, or compatibility tests. It is not the target MCP adapter runtime.

## Consequences

Positive:

- One runtime owns protocol and engine semantics.
- Tool schemas, evidence fixtures, logging, and release gates do not need a
  duplicate TypeScript adapter layer.
- Native binary distribution, performance benchmarks, and install diagnostics
  align with the rest of the Rust-first MCP portfolio.

Costs:

- Existing TypeScript-facing parser/search integrations need stable CLI,
  process, file, or generated-contract boundaries before they are consumed.
- Early implementation must invest in Rust MCP conformance and schema tests
  rather than relying on existing TypeScript MCP conventions.

## Rejected Alternatives

### Rust Core With TypeScript MCP Adapter

Rejected because it preserves a permanent runtime split and turns the adapter
into a second contract surface. It may be used only as a temporary compatibility
bridge in already-shipped repositories with an explicit retirement plan.

### All TypeScript

Rejected for graph traversal, indexing, ranking, and large-repository work, and
because it no longer matches the portfolio runtime direction.

## Validation

- `cargo metadata --format-version 1`
- `cargo test`
- Golden MCP fixture tests once `rmcp` tool handlers are implemented.
