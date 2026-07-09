# Roadmap: Filesystem MCP

## Category Position

Filesystem MCP is the safe local filesystem instrument for agents. Its job is
to let agents inspect and edit project files without escaping the intended root,
silently damaging data, or hiding what changed.

## Current Boundary

The current package exposes filesystem tools including file listing, search, and
multi-file replacement operations relative to a project root.

## SOTA End-State

The final product should be the highest-trust local filesystem MCP: root-scoped,
symlink-safe, fast on large repositories, explicit about write operations, and
auditable by humans and agents.

## Target Architecture

- Rust core for path canonicalization, policy enforcement, directory walking,
  search, diff application, hashing, and file IO.
- MCP adapter may remain thin during migration.
- Shared evidence envelope for path, root, content hash, byte or line range,
  operation id, diff summary, and policy decision.

## Feature Pillars

- Safety: root confinement, symlink handling, deny patterns, size limits, binary
  detection, and explicit write policy.
- Speed: parallel directory walk, ignore-file support, streaming reads, and fast
  search.
- Edit integrity: preview, apply, rollback metadata, conflict detection, and
  exact diff evidence.
- Audit: operation log, changed file hashes, and dry-run mode.
- Agent ergonomics: clear diagnostics and next actions.

## Roadmap

### Phase 0: Safety Baseline

- Document current tool surface and write boundaries.
- Add root, symlink, hidden file, binary file, and oversized file fixtures.
- Add dry-run examples for write-capable tools.

### Phase 1: Rust Policy And IO Core

- Implement canonical path and root policy in Rust.
- Add fast walk and search engine.
- Add deterministic diff preview and apply primitives.
- Keep adapter thin.

### Phase 2: Audit And Recovery

- Add operation ids and change ledgers.
- Add rollback metadata where safe.
- Add conflict detection for stale file hashes.
- Add structured write warnings.

### Phase 3: Enterprise Controls

- Add allowlist and denylist profiles.
- Add read-only mode, write-approval mode, and CI-safe mode.
- Add policy export for agents.

### Phase 4: Release Scale

- Ship optional binary packages.
- Publish performance benchmarks against large repo fixtures.
- Evaluate direct Rust MCP server.

## Validation Gates

- Symlink escape attempts are denied.
- Writes require exact current content hash or conflict detection.
- Search respects ignore and policy rules.
- Large repos meet published walk and search p95 gates.
- Audit log reconstructs every write operation.

## ADRs To Land In Filesystem MCP

- Rust filesystem policy engine.
- Write operation safety model.
- Search and walk performance model.
- Audit ledger format.
- Direct Rust server migration.
