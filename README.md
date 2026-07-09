<div align="center">

# Architecture Reader MCP

### Your agent mapped the repo. **Did it trace the right boundary?**

Agent-native MCP server for **repository architecture evidence graphs** — boundaries,
dependencies, routes, schemas, and impact radius with file-level provenance, not
dashboard screenshots or keyword grep.

[![CI/CD](https://img.shields.io/github/actions/workflow/status/SylphxAI/architecture-reader-mcp/ci.yml?style=flat-square&label=CI/CD)](https://github.com/SylphxAI/architecture-reader-mcp/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-core-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-MCP%20adapter-blue?style=flat-square)](https://www.typescriptlang.org/)

**Beta 0.1** · **Rust core + Bun MCP adapter** · **7 typed MCP tools** · **Evidence envelope** · **5 tests**

[⭐ Star this repo](https://github.com/SylphxAI/architecture-reader-mcp) if agents should answer architecture questions with proof, not graphviz guesses.
· [Quick start](#quick-start) · [Planned contract](#planned-contract) · [Why not grep or a dashboard?](#why-not-grep-or-a-dashboard)

Complements generic code search in [CodeRAG](https://github.com/SylphxAI/coderag) — it does
not replace it. Reader portfolio media tools live in
[smart-reader-mcp](https://github.com/SylphxAI/smart-reader-mcp).

</div>

---

## The problem

Agents onboard, review, and refactor codebases every day. Most paths give you one
of three bad outcomes:

1. **grep / ripgrep** — fast, but literal. Finds strings, not boundaries, routes,
   or ownership.
2. **Generic code search** — great chunks, weak architecture map. You still guess
   which module owns auth, billing, or deployment.
3. **Visualization-first graph UIs** — rich for humans, heavy for agents. Screenshots
   and pan/zoom do not fit MCP context windows.

The model still hallucinates structure — confidently.

**Architecture Reader MCP is built for the moment your agent needs to prove how the
repo is shaped, what depends on what, and which files back each claim.**

## Current delivery state

**Beta 0.1** ships a runnable Rust evidence-graph engine with a thin Bun MCP
adapter. Manifest/import/docs extraction works on local fixtures; route/schema
extractors and npm publish are still in progress. See
[roadmap](./docs/portfolio/roadmaps/architecture-reader-mcp.md).

## Why not grep or a dashboard?

| Typical path | Architecture Reader MCP (target) |
| --- | --- |
| Keyword hits over files | Architecture map: components, boundaries, routes, schemas |
| "Trust the summary" | Evidence refs: path, line range, extractor, confidence |
| Human graph explorer | Compact MCP answers with trace + impact tools |
| Generic chunk search | Purpose-built `architecture_*` tools with shared envelope |
| Ship and pray | Deterministic extraction first; inference explicitly labeled |

Generic code search stays in [CodeRAG](https://github.com/SylphxAI/coderag). AST
substrate integration is planned through Synth — see
[integration ADR](./docs/adr/ADR-DRAFT-synth-coderag-integration.md).

## See it work

**Build the Rust engine, then run the MCP adapter locally:**

```bash
git clone https://github.com/SylphxAI/architecture-reader-mcp.git
cd architecture-reader-mcp
bun install
cargo build --release
ARCHITECTURE_READER_CLI=$PWD/target/release/architecture-reader-cli bun run packages/mcp-server/src/index.ts
```

Index once, then ask architecture questions:

```json
{
  "root": "/absolute/path/to/repo",
  "mode": "auto"
}
```

`architecture_search` (planned) returns ranked nodes with evidence, not prose:

```json
{
  "status": "ok",
  "repository": {
    "root": "/abs/path",
    "indexedCommit": "abc123",
    "currentCommit": "abc123",
    "freshness": "fresh"
  },
  "answer": {
    "matches": [
      {
        "id": "cmp_auth",
        "kind": "boundary",
        "label": "Authentication",
        "score": 0.94
      }
    ]
  },
  "evidence": [
    {
      "id": "ev_01",
      "kind": "ast",
      "path": "src/auth/middleware.ts",
      "startLine": 10,
      "endLine": 42,
      "extractor": "synth-typescript@0.3.x",
      "confidence": "deterministic"
    }
  ],
  "gaps": [],
  "metrics": { "elapsedMs": 12, "nodeCount": 430, "edgeCount": 910 }
}
```

Abbreviated shape — full envelope in [tool contract spec](./docs/specs/2026-07-09-tool-contract.md).

Trace dependency or impact before editing:

```json
{
  "from": "src/auth/middleware.ts",
  "to": "src/billing/webhook.ts",
  "relation": "depends_on"
}
```

## Why agents will use it

| Need | Planned tool |
| --- | --- |
| Build or refresh the index | `architecture_index` |
| Check freshness and coverage | `architecture_status` |
| Top-level repo map | `architecture_overview` |
| Find boundaries, routes, schemas | `architecture_search` |
| Follow dependency or call paths | `architecture_trace` |
| Estimate diff blast radius | `architecture_impact` |
| Fetch proof behind a claim | `architecture_evidence` |

Every answer shares one evidence envelope: path, optional line range, extraction
source, freshness, confidence, and known gaps.

## Quick start

### Clone and validate the scaffold

```bash
git clone https://github.com/SylphxAI/architecture-reader-mcp.git
cd architecture-reader-mcp
bun install
bun run validate
cargo test
bun test test/readmeDiscovery.test.ts
```

### Implementation stack

- **Rust core** — graph engine, index formats, query planning, traversal, impact.
- **TypeScript/Bun MCP adapter** — protocol ergonomics and Sylphx MCP conventions.

See [hybrid runtime ADR](./docs/adr/ADR-DRAFT-hybrid-rust-core-bun-mcp-adapter.md).

## Repository layout

```text
architecture-reader-mcp/
  crates/
    architecture-reader-core/    # Rust architecture graph contracts and engine
  packages/
    mcp-server/                  # TypeScript/Bun MCP adapter (stub)
  docs/
    adr/                         # Architecture decisions
    specs/                       # Product, graph, indexing, and tool specs
    research/                    # Evidence and category analysis
    portfolio/                   # MCP portfolio ADRs and roadmaps
  server.json                    # Draft MCP server metadata
```

## Design documents

| Topic | Link |
| --- | --- |
| Architecture overview | [docs/architecture.md](./docs/architecture.md) |
| Product spec | [docs/specs/2026-07-09-product-spec.md](./docs/specs/2026-07-09-product-spec.md) |
| Tool contract | [docs/specs/2026-07-09-tool-contract.md](./docs/specs/2026-07-09-tool-contract.md) |
| Evidence graph | [docs/specs/2026-07-09-evidence-graph.md](./docs/specs/2026-07-09-evidence-graph.md) |
| Indexing pipeline | [docs/specs/2026-07-09-indexing-pipeline.md](./docs/specs/2026-07-09-indexing-pipeline.md) |
| Category research | [docs/research/2026-07-09-category-and-internal-analysis.md](./docs/research/2026-07-09-category-and-internal-analysis.md) |
| Portfolio plan | [docs/portfolio/README.md](./docs/portfolio/README.md) |
| Roadmap | [docs/portfolio/roadmaps/architecture-reader-mcp.md](./docs/portfolio/roadmaps/architecture-reader-mcp.md) |

## Next implementation slice

1. Synth-backed AST extractor adapter.
2. Route/schema/workflow extractors.
3. Incremental index refresh and public benchmark gate in CI.
4. npm publish + MCP Registry metadata.

## Development

```bash
bun install
bun run validate
cargo build --release
cargo test
bun test
bun run benchmark:public-proof
```

## Benchmark proof

Reproduce locally:

```bash
bun run benchmark:public-proof
```

Fixture: `fixtures/sample-repo` (auth middleware + ADR + package manifest).

## Help this reach more builders

If your agent has ever refactored the wrong module because it guessed the architecture,
this project is for you.

**[⭐ Star the repo](https://github.com/SylphxAI/architecture-reader-mcp)** — it helps
more agent builders find evidence-backed architecture answers before irreversible edits.

### Discovery (in progress)

| Channel | Status |
| --- | --- |
| [Official MCP Registry](https://registry.modelcontextprotocol.io/) | Not listed yet — draft scaffold, no publish workflow |
| [Glama MCP directory](https://glama.ai/mcp/servers) | Not listed yet |
| [mcpservers.org submit](https://mcpservers.org/submit) | Not listed yet — free web-form submission |
| [mcp.so submit](https://mcp.so/submit) | Not listed yet — directory submission |

Know another MCP directory? [Open an issue](https://github.com/SylphxAI/architecture-reader-mcp/issues/new) with the link.

## License

UNLICENSED — private SylphxAI repository until an explicit OSS license is adopted.