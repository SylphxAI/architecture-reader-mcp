# ADR Draft: Rust-First Runtime And Reliable MCP Distribution

Status: proposed  
Date: 2026-07-09  
Decision owner: SylphxAI

## Context

The MCP portfolio is currently mostly Bun and TypeScript. This is fast for early
shipping, but the long-term product promise is high-performance local agent
infrastructure: parsing, indexing, extraction, search, and graph reasoning over
large repositories and media files.

The company is Rust-first. Users still expect npm installation to be simple.
Native Rust binaries can fail to install if the package relies on network
downloads, unsupported platforms, blocked lifecycle scripts, missing executable
permissions, libc mismatches, or quarantine policies.

## Decision

New SOTA engines will use Rust as the core runtime by default.

MCP adapter choices are governed by product risk:

1. Use a Rust-native MCP server when protocol support, schema generation,
   transport behavior, logging, and release automation meet portfolio gates.
2. Use a thin Bun or TypeScript MCP adapter when it lets the project ship a
   stable protocol surface faster while the Rust core owns the heavy work.
3. Do not build high-throughput product logic in the TypeScript adapter unless
   the logic is provider orchestration, package glue, or temporary migration
   code with an explicit retirement plan.

The npm distribution shape is:

- public wrapper package;
- platform-specific optional binary packages;
- no required network binary download in `postinstall`;
- binary checksums and provenance;
- `doctor` command for install diagnosis;
- standalone release binaries outside npm;
- CI matrix across macOS, Linux glibc, Linux musl, and Windows.

## Rationale

Rust gives predictable performance, strong safety boundaries, low cold-start
overhead, and native access to high-quality parsers, search engines, filesystem
walkers, compression, hashing, and streaming IO.

The optional dependency binary model keeps npm installation familiar while
avoiding fragile install-time downloads. Users get the right binary from the
registry as part of dependency resolution. If the binary is missing, the wrapper
can fail with a precise diagnostic instead of a vague executable error.

The thin-adapter option keeps short-term protocol velocity without turning the
adapter into the engine. It also gives each project a clean migration path to a
single Rust server binary.

## Consequences

Implementation agents must separate product contracts from runtime choice. Tool
schemas, output envelopes, benchmark fixtures, and evidence semantics are stable
contracts. Runtime implementations may evolve underneath them.

Every Rust core project needs:

- a crate-level API for local engine calls;
- a CLI or stdio server boundary;
- deterministic fixture tests;
- benchmark gates;
- release automation for binary artifacts;
- install diagnostics.

Every TypeScript adapter needs:

- a minimal adapter surface;
- no duplicated core logic;
- golden tests against the Rust engine contract;
- an explicit migration checkpoint for direct Rust MCP serving.

## Rejected Alternatives

Pure TypeScript for all MCPs was rejected for performance-sensitive parsing,
indexing, graph, and media workloads.

Network binary download during npm installation was rejected because it fails in
common enterprise, offline, proxy, and package-manager environments.

WASM as the default server runtime was rejected for local high-performance tools
because native Rust provides better filesystem, process, media, and indexing
performance today.

## Validation Gates

- Clean install from npm in fresh macOS, Linux glibc, Linux musl, and Windows
  environments.
- Install works with lifecycle scripts disabled when optional binary packages
  are already present.
- `doctor` explains missing binary, unsupported platform, permission issue, and
  PATH issue.
- Binary artifacts have checksum and release provenance.
- The same fixture output is produced by Rust core tests and MCP adapter tests.
- CI benchmark gates prevent severe regressions.
