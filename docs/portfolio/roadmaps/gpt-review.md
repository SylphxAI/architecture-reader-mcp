# Roadmap: GPT Review / Workflow MCP

> **Status: archived (2026-07-09).** [SylphxAI/gpt-review](https://github.com/SylphxAI/gpt-review)
> is read-only on GitHub. This roadmap is retained for historical context only.

## Category Position

`gpt-review` was a Bun-workspace monorepo for governed agent workflows:

- `@sylphx/workflow-engine` — zero-dependency workflow and review core
- `@sylphx/workflow-mcp` — stdio MCP server (`workflow_review` and related tools)
- `@sylphx/gpt-review` — external GPT/Codex review-gate CLI

Its job was to give agents a convergent second-mind review loop before declaring
work done.

## SOTA End-State (historical plan)

- Engine-owned safety invariants (cwd confinement, execute isolation, review authority)
- Thin MCP and CLI surfaces over one SDK
- Published npm packages with org-standard release control plane
- MCP `workflow_review` self-contained without shelling a separate gate binary

## Why Archived

Portfolio focus moved to document/code/architecture evidence MCPs. The governed
workflow and external review gate remain useful as a reference implementation,
but they are no longer an active public suite target.

## Historical Validation Gates

- Real Node matrix + bun check in CI
- Engine review orchestration in-process (ADR-0034)
- Package release intent via Changesets and org shared publish workflow