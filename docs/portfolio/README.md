# SylphxAI MCP Portfolio Plan

Status: planning baseline  
Audience: implementation agents, maintainers, product owners  
Scope: SylphxAI MCP packages and their shared runtime, packaging, evidence,
performance, and market-positioning standards.

## Portfolio Thesis

SylphxAI should build a coherent suite of agent-native MCP products, not a
loose set of utilities. Each MCP must own one category-level job, install in
under a minute, return evidence that an agent can trust, and prove speed with a
public benchmark harness.

The target is an average of 10,000+ GitHub stars per MCP. That requires more
than functional correctness. Every repo must be obviously useful in the first
screen, have a one-command demo, publish reliable packages, show measurable
performance, and make agents better on real work without forcing users into a
dashboard.

## Product Lines

| Line | Projects | Category job |
| --- | --- | --- |
| Reader | `pdf-reader-mcp`, `image-reader-mcp`, `video-reader-mcp`, `smart-reader-mcp` | Turn files and media into citeable agent evidence. |
| Code Intelligence | `coderag`, `architecture-reader-mcp` | Let agents retrieve code facts, architectural maps, dependency traces, and impact evidence. |
| Control | `filesystem-mcp`, `consultant-mcp` | Give agents safe local operations and structured decision review. |

## Runtime Standard

Rust is the default runtime for new SOTA MCP engines and MCP servers. It should
own parsing, indexing, graph construction, search, streaming IO, policy
enforcement, deterministic media/document reading, tool schemas, stdio serving,
and future streamable HTTP serving.

The target MCP server stack is the official
[`modelcontextprotocol/rust-sdk`](https://github.com/modelcontextprotocol/rust-sdk)
`rmcp` crate plus project-owned Rust engine crates. Bun or TypeScript can remain
for generated clients, tests, npm wrapper metadata, and migration compatibility,
but they are not the target MCP adapter runtime.

Existing TypeScript MCPs should migrate by first freezing tool contracts and
evidence fixtures, then replacing the server runtime with Rust while preserving
the same public tool surface and package installation path.

## Install Standard

Rust core does not have to create user installation pain, but only if binary
distribution is treated as a product feature.

The standard npm package shape is:

- one small public wrapper package, for example `@sylphx/<project>-mcp`;
- platform-specific optional binary packages, for example
  `@sylphx/<project>-darwin-arm64`, `linux-x64-gnu`, `linux-x64-musl`,
  `linux-arm64-gnu`, and `win32-x64`;
- no required network download during `postinstall`;
- deterministic binary resolution at runtime;
- checksum and provenance verification in release CI;
- a `doctor` command that explains missing binary, unsupported platform,
  blocked lifecycle scripts, quarantine, PATH, and permission issues;
- standalone GitHub release binaries for users who do not want npm.

This avoids the weakest install pattern: a package that downloads an executable
from the network at install time and leaves users with a missing binary when
firewalls, registries, proxies, or lifecycle-script policies block the download.

## WASM Policy

WASM is a supported execution target for sandboxed plugins, portable extractors,
browser demos, edge experiments, and untrusted user extensions. It is not the
default runtime for local high-performance MCP servers.

The default server path remains native Rust. WASM can be introduced where
isolation and portability beat raw throughput or where a user-supplied extension
must run under strict capability boundaries.

## Evidence Standard

Every MCP output should be an agent-readable evidence object, not just text.
Where possible it should include:

- source path or URI;
- content hash and version;
- byte offsets, line ranges, page numbers, timestamps, bounding boxes, node ids,
  or symbol ids;
- confidence and extraction route;
- warnings for missing coverage, lossy extraction, unsafe paths, stale indexes,
  or provider failures;
- reproducible follow-up tool calls for deeper inspection.

The agent should be able to answer: "What was observed, where did it come from,
how fresh is it, how can I verify it, and what should I call next?"

## Performance Standard

Each project needs a benchmark fixture that runs in CI and locally:

- cold start;
- first useful response;
- p50 and p95 tool latency;
- indexing throughput;
- memory ceiling;
- incremental update latency;
- output determinism across repeated runs;
- failure behavior on corrupted, oversized, or adversarial inputs.

The public README should show headline numbers only after CI produces them.
Claims without benchmark proof are planning goals, not shipped facts.

## Brand And Growth Standard

Every MCP repo should have:

- a category-defining one-line promise;
- a 30-second install path;
- a copy-paste MCP client config;
- one tiny demo fixture checked into the repo;
- one real-world demo script;
- clear tool contracts and JSON examples;
- public benchmark commands;
- security and trust boundaries;
- a release badge that means the package is actually publishable;
- a short "why agents use this" section focused on the job, not implementation
  trivia.

The suite should look like one product family: consistent names, command names,
output envelopes, trust warnings, benchmark style, and release quality.

## ADR Index

- `ADR-DRAFT-rust-first-runtime-distribution.md`
- `ADR-DRAFT-wasm-plugin-sandbox.md`
- `ADR-DRAFT-agent-evidence-envelope.md`
- `ADR-DRAFT-portfolio-positioning-and-growth.md`

## Roadmap Index

- `roadmaps/architecture-reader-mcp.md`
- `roadmaps/coderag.md`
- `roadmaps/pdf-reader-mcp.md`
- `roadmaps/image-reader-mcp.md`
- `roadmaps/video-reader-mcp.md`
- `roadmaps/smart-reader-mcp.md`
- `roadmaps/filesystem-mcp.md`
- `roadmaps/consultant-mcp.md`
