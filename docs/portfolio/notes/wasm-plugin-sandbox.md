# Portfolio Note: WASM As Plugin Sandbox, Not Default Server Runtime

Status: planning note
Date: 2026-07-09  
Decision owner: SylphxAI

## Context

WASM is increasingly used for portable execution, sandboxing, browser demos,
edge workloads, and extension systems. MCP-over-WASM experiments exist in the
wider ecosystem, so the portfolio should not assume that WASM MCP servers are
unexplored.

The portfolio target is high-performance local agent infrastructure over source
code, documents, media, and local files. These workloads need fast filesystem
access, large-file streaming, native parsers, process integration, and low
latency under repeated agent calls.

## Decision

WASM is a first-class extension and portability target, not the default local
MCP server runtime.

Use WASM for:

- sandboxed extractors supplied by users or partners;
- portable analysis components with narrow IO;
- browser or edge demos;
- policy-controlled plugins;
- deterministic transformations that fit a capability-limited runtime.

Use native Rust for:

- default MCP server binaries;
- repository indexing;
- graph and search engines;
- media probing and frame or page extraction;
- filesystem policy enforcement;
- high-throughput local batch operations.

When WASM components become part of a product, define their interface through a
versioned component contract and capability model. The host controls file,
network, process, time, memory, and cache access.

## Rationale

WASM is valuable where isolation and portability are more important than native
throughput and host integration. It is especially useful for allowing extension
code without trusting it with the full local environment.

Native Rust remains the better default for local MCP products that need deep OS
integration, broad file access, low overhead, native libraries, and predictable
performance on large inputs.

## Consequences

Projects may add WASM plugin points only after their core evidence contract and
benchmark suite are stable. A WASM plugin must not be able to bypass the host
project's safety policy, evidence envelope, or resource limits.

Documentation must describe WASM as an execution mode with tradeoffs, not as a
magic install-friction solution. WASM can reduce native binary concerns in some
cases, but it introduces host runtime, capability, performance, and library
compatibility constraints.

## Validation Gates

- WASM plugin interfaces are versioned and fixture-tested.
- Host capability policy denies network, filesystem, and process access by
  default.
- Plugin output is normalized into the same evidence envelope as native output.
- Benchmark reports compare native and WASM modes when both exist.
- Failure output identifies whether the host, runtime, plugin, or capability
  policy caused the failure.
