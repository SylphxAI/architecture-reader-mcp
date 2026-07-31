# Spine — local architecture engine for agents

Use Spine when you need **proof of repository structure**, not grep guesses.

## Install

```bash
# MCP (stdio)
npx @sylphx/architecture-reader-mcp
# CLI
./bin/spine doctor
./bin/spine index .
./bin/spine search auth
./bin/spine path src/auth/token.ts src/server/routes.ts
./bin/spine impact --git-diff
```

## Tools (clear, not merged)

| Tool | When |
| --- | --- |
| `architecture_index` | First: build/refresh local graph |
| `architecture_status` | Freshness / dirty worktree |
| `architecture_overview` | Packages, languages, counts |
| `architecture_search` | Ranked symbol/module/route lookup |
| `architecture_path` | Shortest path with hop provenance |
| `architecture_trace` | Compact path between entities |
| `architecture_impact` | Blast radius: direct + **incoming** dependents + outgoing deps (`useGitDiff` / `changedPaths`) |
| `architecture_evidence` | Resolve evidence ids from prior answers |

## Evidence contract

Every answer includes file/line evidence when known, extractor id, and explicit gaps.
There is **no** `evidence_first` tool — proof is on results.

## Rules

1. Prefer deterministic extractors; Synth AST is opt-in (`ARCHITECTURE_READER_USE_SYNTH=1`).
2. Local-first: no required cloud API key.
3. Do not invent architecture — call tools, cite evidence.
4. Sibling instruments (PDF/image/video/web) are **other repos**; do not assume monorepo imports.

Family knowledge: https://github.com/SylphxAI/instruments
