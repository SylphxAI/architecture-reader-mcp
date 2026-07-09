# Roadmap: Smart Reader MCP

## Category Position

Smart Reader MCP is the universal reader router for agents. Its job is to
accept a file, identify the real format, delegate to the right specialist, and
return one consistent evidence envelope.

## Current Boundary

The current package exposes `read_media` and delegates to PDF, image, and video
reader siblings.

## SOTA End-State

The final product should be the one safe entrypoint an agent calls when it does
not know the file type. It should sniff format by bytes, enforce path policy,
delegate with traceability, normalize outputs, and explain the chosen route.

## Target Architecture

- Rust core for MIME sniffing, path normalization, archive handling, hash
  calculation, and policy enforcement.
- Thin adapter for MCP and sibling invocation.
- Shared evidence envelope for source hash, detected format, declared format,
  delegation route, warnings, and child tool evidence.

## Feature Pillars

- Format truth: byte sniffing beats extension names.
- Delegation: PDF, image, video, and future readers behind versioned contracts.
- Policy: path sandbox, symlink handling, size limits, archive limits, and
  remote-fetch controls.
- Normalization: one top-level envelope across media types.
- Routing diagnostics: why this reader was selected and what else was possible.

## Roadmap

### Phase 0: Router Contract

- Freeze `read_media` envelope.
- Add examples for PDF, image, video, unknown, mislabeled, and unsupported files.
- Add delegated evidence preservation tests.

### Phase 1: Rust Sniffing And Policy Core

- Implement byte-based format detection.
- Add path and symlink policy tests.
- Add size, archive, and recursion limits.
- Add native binary packaging plan.

### Phase 2: Multi-Reader Normalization

- Add versioned child-reader contracts.
- Normalize warnings, hashes, locators, and next actions.
- Add router trace for every delegation.

### Phase 3: Expansion

- Add archive, audio, HTML, Office document, and text routing once specialist
  contracts exist.
- Add batch mode with per-file evidence.
- Add optional content-addressed cache.

### Phase 4: One-Command Reader Suite

- Ship as the recommended first install for agents that need media reading.
- Add suite-level demos and benchmark scorecard.
- Evaluate direct Rust MCP server after sibling contracts stabilize.

## Validation Gates

- Mislabeled files route by content, not extension.
- Symlink escapes are denied by default.
- Delegated child evidence remains intact.
- Unsupported files return precise diagnostics.
- Router overhead stays inside published budget.

## ADRs To Land In Smart Reader

- Rust sniffing and path policy boundary.
- Delegated reader contract.
- Archive and recursion policy.
- Unified evidence normalization.
- Suite-level install profile.
