<div align="center">

# Spine

### Your agent mapped the repo. **Did it trace the right boundary?**

**Spine** is the architecture instrument in the **Sylphx Instruments** product family —
a **local-first architecture engine** for agents and apps (SDK · CLI · MCP).

Agent-native **repository architecture evidence graphs** — boundaries,
dependencies, routes, schemas, and impact radius with file-level provenance — not
dashboard screenshots, not required LLM multi-agent burns, not keyword grep.

Transitional package/repo: `@sylphx/architecture-reader-mcp` / `architecture-reader-mcp`.

[![npm version](https://img.shields.io/npm/v/@sylphx/architecture-reader-mcp?style=flat-square)](https://www.npmjs.com/package/@sylphx/architecture-reader-mcp)
[![CI/CD](https://img.shields.io/github/actions/workflow/status/SylphxAI/architecture-reader-mcp/ci.yml?style=flat-square&label=CI/CD)](https://github.com/SylphxAI/architecture-reader-mcp/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-core-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-MCP%20adapter-blue?style=flat-square)](https://www.typescriptlang.org/)

**Beta 0.1** · **Rust core + Bun MCP adapter** · **8 typed MCP tools** · **Evidence envelope** · **Rust/Go/Python/TS extractors**

[⭐ Star this repo](https://github.com/SylphxAI/architecture-reader-mcp) if agents should answer architecture questions with proof, not graphviz guesses.
· [Quick start](#quick-start) · [Tool contract](#tool-contract) · [Why not grep or a dashboard?](#why-not-grep-or-a-dashboard)

Complements generic code search in [CodeRAG](https://github.com/SylphxAI/coderag) — it does
not replace it. Reader portfolio media tools live in
[Prism / smart-reader-mcp](https://github.com/SylphxAI/smart-reader-mcp).

</div>

---


## Language extractors (deterministic)

| Language | What is extracted | Extractor id |
| --- | --- | --- |
| TypeScript / JavaScript | modules, imports, symbols, calls, routes, zod schemas | `import-graph` / `call-graph` / `routes` / `schema` (+ opt-in Synth AST) |
| Python | modules, imports, classes, functions, calls | `python@0.1.0` |
| Rust | modules, `use`/`mod`, functions, local calls | `rust@0.1.0` |
| Go | package, imports, functions, local calls | `go@0.1.0` |
| Java | package, imports, classes, methods, local calls | `java@0.1.0` |
| C# | namespace, usings, types, methods, local calls | `csharp@0.1.0` |
| Kotlin | package, imports, classes, fun, local calls | `kotlin@0.1.0` |
| Manifests / docs | `package.json`, `Cargo.toml`, ADRs/docs | `manifest` / `docs` |

Every node/edge carries **file:line evidence** when known. Inference is labeled separately from deterministic structure.

## Competitive position

| Product | Spine difference |
| --- | --- |
| [Understand-Anything](https://github.com/Egonex-AI/Understand-Anything) | Human dashboard + skill + LLM semantics — Spine prioritizes **deterministic structure** and **agent/SDK query** |
| [Graphify](https://github.com/Graphify-Labs/graphify) | Excellent local AST graph + CLI — Spine targets the same local honesty **plus first-class MCP/SDK** and architecture/impact workflows |
| [Serena](https://github.com/oraios/serena) | Symbol **edit** IDE for agents — Spine does **not** own edits |

Targets: [docs/portfolio/specs/spine-product-spec-v0.md](docs/portfolio/specs/spine-product-spec-v0.md) (product-local). Other instruments are separate repositories.

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
adapter. Manifest/import/docs/route/schema extraction, symbol call tracing,
TypeScript/Python/Rust/Go/Java/C# indexing, path/impact with git-diff, and incremental refresh are implemented with release-gate proof.
First npm publish is workflow-owned via Changesets on `main` — see
[roadmap](./docs/portfolio/roadmaps/architecture-reader-mcp.md).

## Why not grep or a dashboard?

| Typical path | Architecture Reader MCP (target) |
| --- | --- |
| Keyword hits over files | Architecture map: components, boundaries, routes, schemas |
| "Trust the summary" | Evidence refs: path, line range, extractor, confidence |
| Human graph explorer | Compact MCP answers with trace + impact tools |
| Generic chunk search | Purpose-built `architecture_*` tools with shared envelope |
| Ship and pray | Deterministic extraction first; inference explicitly labeled |

Generic code search stays in [CodeRAG](https://github.com/SylphxAI/coderag). Synth
AST extraction is opt-in for TypeScript/JavaScript modules — see
[integration spec](./docs/specs/synth-coderag-integration.md).

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

`architecture_search` returns ranked nodes with evidence, not prose:

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

| Need | Tool |
| --- | --- |
| Build or refresh the index | `architecture_index` |
| Check freshness and coverage | `architecture_status` |
| Top-level repo map | `architecture_overview` |
| Find boundaries, routes, schemas | `architecture_path` | Shortest path with hop provenance (`extracted`/`inferred`) |
| `architecture_search` |
| Follow dependency or call paths | `architecture_trace` |
| Estimate diff blast radius (incoming dependents + outgoing deps) | `architecture_impact` |
| Local neighborhood (Graphify-class) | `architecture_overview` with `focus=<path\|label\|id>` → `neighbors` |
| Fetch proof behind a claim | `architecture_evidence` |

Every answer shares one evidence envelope: path, optional line range, extraction
source, freshness, confidence, and known gaps.

## Quick start

### Claude Code

```bash
claude mcp add architecture-reader -- npx @sylphx/architecture-reader-mcp
```

### Claude Desktop / any MCP host

```json
{
  "mcpServers": {
    "architecture-reader": {
      "command": "npx",
      "args": ["-y", "@sylphx/architecture-reader-mcp"]
    }
  }
}
```

Set the host working directory to the repository you want indexed.

### Clone and validate locally

```bash
git clone https://github.com/SylphxAI/architecture-reader-mcp.git
cd architecture-reader-mcp
bun install
bun run build:rust
bun run validate
cargo test
bun test test/readmeDiscovery.test.ts
```

### Implementation stack

- **Rust core** — graph engine, index formats, query planning, traversal, impact.
- **TypeScript/Bun MCP adapter** — protocol ergonomics and Sylphx MCP conventions.

See [rust-first runtime note](./docs/portfolio/notes/rust-first-runtime-distribution.md).

## Repository layout

```text
architecture-reader-mcp/
  crates/
    architecture-reader-core/    # Rust architecture graph contracts and engine
  packages/
    mcp-server/                  # TypeScript/Bun MCP adapter
  docs/
    adr/                         # Architecture decisions
    specs/                       # Product, graph, indexing, and tool specs
    research/                    # Evidence and category analysis
    portfolio/                   # MCP portfolio ADRs and roadmaps
  server.json                    # MCP server metadata
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

## Agent skill surface

Codex/Claude-style skill: [`skills/spine/SKILL.md`](./skills/spine/SKILL.md) — install, tools, evidence contract (Graphify-class agent UX without multi-GB weight).

## Tool contract

All seven `architecture_*` tools share the evidence envelope defined in
[tool contract spec](./docs/specs/2026-07-09-tool-contract.md). Run
`bun run benchmark:release-gate` after `cargo build --release` for boundary proof.

## Development

```bash
bun install
bun run validate
cargo build --release
cargo test
bun test
bun run benchmark:public-proof
```

## Security model

- **Repository scope** — all tools operate relative to the configured project root; absolute paths are rejected.
- **Evidence envelope** — every answer includes path, line range, extractor route, freshness, and explicit coverage gaps.
- **Deterministic first** — regex and manifest extractors are labeled; Synth AST is opt-in via `ARCHITECTURE_READER_USE_SYNTH=1`.
- **Local-first** — indexing and queries run on your machine; no document upload to Sylphx cloud by default.

## Benchmark proof

Reproduce locally:

```bash
bun run build:rust
bun run benchmark:public-proof
bun run benchmark:release-gate
```

Fixture: `fixtures/sample-repo` (auth middleware + ADR + package manifest). Example requests: [`examples/`](examples/).

## Help this reach more builders

If your agent has ever refactored the wrong module because it guessed the architecture,
this project is for you.

**[⭐ Star the repo](https://github.com/SylphxAI/architecture-reader-mcp)** — it helps
more agent builders find evidence-backed architecture answers before irreversible edits.

### Discovery (in progress)

| Channel | Status |
| --- | --- |
| [Official MCP Registry](https://registry.modelcontextprotocol.io/) | Not listed yet — Beta 0.1 local ship, no publish workflow |
| [Glama MCP directory](https://glama.ai/mcp/servers) | Not listed yet |
| [mcpservers.org submit](https://mcpservers.org/submit) | Not listed yet — free web-form submission |
| [mcp.so submit](https://mcp.so/submit) | Not listed yet — directory submission |

Know another MCP directory? [Open an issue](https://github.com/SylphxAI/architecture-reader-mcp/issues/new) with the link.


## Spine CLI / SDK

```bash
bun run build:rust
./bin/spine index .
./bin/spine search . auth
./bin/spine path . authMiddleware validateToken --relation calls
```

TypeScript:
```ts
import { Spine } from '@sylphx/architecture-reader-mcp/sdk'
const spine = Spine.create({ root: process.cwd() })
await spine.index({ mode: 'full' })
const path = await spine.path('authMiddleware', 'validateToken', { relation: 'calls' })
```

## License

MIT — see [LICENSE](LICENSE).