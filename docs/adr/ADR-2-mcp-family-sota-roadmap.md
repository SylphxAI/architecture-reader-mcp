# ADR-2: Adopt Architecture Reader MCP Family SOTA Roadmap

Date: 2026-07-09
Status: Proposed in PR #2
Slug: mcp-family-sota-roadmap

## Context

Architecture Reader MCP is one member of a broader SylphxAI MCP family. Without
a repo-local roadmap, implementation agents can over-expand the project into
generic code search, file editing, document extraction, consultation, or visual
dashboard work.

## Decision

Adopt `docs/roadmap/sota-family-roadmap.md` as the repo-local roadmap for this
project's family role.

Architecture Reader MCP owns architecture evidence graphs, trace queries, and
impact semantics. It integrates with sibling MCPs through evidence contracts but
does not absorb their responsibilities.

## Consequences

- Rust remains the target runtime for graph construction, indexing, traversal,
  ranking, and impact analysis.
- TypeScript/Bun remains acceptable as a thin adapter while MCP protocol and
  package gates mature.
- Code retrieval, filesystem writes, media extraction, and deliberation stay in
  their owning projects.
- Future implementation work must cite the roadmap phase and validation gate it
  advances.

## Verification

- Roadmap added at `docs/roadmap/sota-family-roadmap.md`.
- README and PROJECT link to the roadmap.
- Docs-only validation: `git diff --check`.
