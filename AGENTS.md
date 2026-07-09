# Architecture Reader MCP Agent Instructions

This repository follows the SylphxAI Doctrine as the upstream operating
authority: https://github.com/SylphxAI/doctrine

## Local Boundary

Architecture Reader MCP owns the agent-facing architecture evidence graph,
architecture search tool contracts, indexing/query engine contracts, and MCP
adapter for repository architecture understanding.

It does not own generic code search, AST parser package internals, media reader
behavior, visualization dashboards, enterprise doctrine, or deployment
infrastructure owned by other repositories.

## Required Local SSOT

- `PROJECT.md` is the human-readable project boundary.
- `.doctrine/project.json` is the Sylphx governance adapter.
- `project.manifest.json` is the vendor-neutral machine manifest.
- `docs/architecture.md` is the durable architecture overview.
- `docs/specs/` owns product and protocol specifications.
- `docs/adr/` owns durable architectural decisions.

Do not duplicate facts across these files. Link to the owning SSOT instead.

## Delivery Standard

Do not claim this project is shipped, published, released, or production-ready
from local files alone. Completion evidence requires the normal repository path:
branch, CI, merge, release, registry/server readback, and production verification
as applicable.

For design-only changes, valid evidence is a clean git status, readable docs,
schema/JSON validation, and explicit acknowledgement that no release occurred.
