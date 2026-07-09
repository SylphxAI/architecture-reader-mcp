# Portfolio Note: Rust-Native MCP Runtime And Reliable Distribution

Status: planning note
Date: 2026-07-09  
Decision owner: SylphxAI

## Context

The MCP portfolio is currently mostly Bun and TypeScript. That helped early
shipping, but the long-term product promise is high-performance local agent
infrastructure: parsing, indexing, extraction, search, graph reasoning, and
deterministic media/document reading over large local inputs.

The company is Rust-first, and the official
`modelcontextprotocol/rust-sdk` now provides a viable Rust MCP server path
through the `rmcp` crate. Users still expect npm installation to be simple.
Native Rust binaries can fail to install if the package relies on network
downloads, unsupported platforms, blocked lifecycle scripts, missing executable
permissions, libc mismatches, or quarantine policies.

## Decision

New SOTA MCPs will use Rust for both the core runtime and MCP serving by
default.

The target server stack is:

1. Rust engine crates for product logic, extraction, indexing, search, media
   handling, evidence envelopes, and policy.
2. Rust MCP server crates using `modelcontextprotocol/rust-sdk` / `rmcp` for
   tools, resources, prompts where needed, stdio transport, future streamable
   HTTP support, logging, and typed schemas.
3. npm wrapper packages only for distribution ergonomics. They may select and
   launch platform binaries, but they must not become TypeScript MCP adapters.

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

Using Rust for MCP serving removes the runtime split between protocol and engine
logic. It also avoids duplicated validation, duplicated schemas, and a permanent
adapter migration tax.

## Consequences

Implementation agents must separate product contracts from runtime choice. Tool
schemas, output envelopes, benchmark fixtures, and evidence semantics are stable
contracts. Runtime implementations may evolve underneath them.

Every Rust MCP project needs:

- a crate-level API for local engine calls;
- a Rust MCP server crate using `rmcp`;
- deterministic fixture tests;
- benchmark gates;
- release automation for binary artifacts;
- install diagnostics.

TypeScript remains allowed for generated client types, docs examples, npm
wrapper metadata, or tests that exercise consumer ergonomics. It is not the
target MCP server runtime.

## Rejected Alternatives

Pure TypeScript for MCP servers was rejected for performance-sensitive parsing,
indexing, graph, filesystem, and media workloads, and because the official Rust
SDK removes the need for a permanent protocol adapter split.

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
- The same fixture output is produced by Rust core tests and Rust MCP server
  tests.
- CI benchmark gates prevent severe regressions.
