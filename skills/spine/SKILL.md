# Spine — local architecture engine for agents

Use Spine when you need **proof of repository structure**, not grep guesses.

## Install

```bash
# MCP (stdio)
npx @sylphx/architecture-reader-mcp
# CLI
./bin/spine doctor
./bin/spine index . --mode full --exclude target
./bin/spine search auth --neighbors --type symbol
./bin/spine browse --limit 15
./bin/spine overview --focus src/auth/token.ts
./bin/spine path src/auth/token.ts src/server/routes.ts
./bin/spine impact --git-diff --max-depth 2
```

## Tools (clear, not merged)

| Tool | When |
| --- | --- |
| `architecture_index` | First: build/refresh local graph |
| `architecture_status` | Freshness, languages, topFanIn/Out, **orphans**, cycles |
| `architecture_overview` | Packages, languages, counts, **cycles**, **orphans** (zero inbound); **`focus=` for neighbors** |
| `architecture_search` | Ranked symbol/module/route lookup |
| `architecture_path` | Shortest path with hop provenance + **mermaid** |
| `architecture_trace` | Compact path + **hop provenance** (edge kinds) |
| `architecture_impact` | Blast radius + **unknownImpact** + **mermaid** in/out |
| `architecture_evidence` | Resolve evidence ids from prior answers |
| `architecture_context_pack` *(advanced)* | Focus neighborhood + co-located + evidence pack |

## Languages

TS/JS, Python, Rust, Go, Java, C#, Kotlin, Ruby, PHP, **C/C++, Shell, GitHub workflows** (`c@0.1.0`).

## Evidence contract

Every answer includes file/line evidence when known, extractor id, and explicit gaps.
There is **no** `evidence_first` tool — proof is on results.

## Rules

1. Prefer deterministic extractors; Synth AST is opt-in (`ARCHITECTURE_READER_USE_SYNTH=1`).
2. Local-first: no required cloud API key.
3. Do not invent architecture — call tools, cite evidence.
4. Sibling instruments (PDF/image/video/web) are **other repos**; do not assume monorepo imports.

Family knowledge: https://github.com/SylphxAI/instruments
