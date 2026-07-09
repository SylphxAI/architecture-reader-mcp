# Roadmap: PDF Reader MCP

## Category Position

PDF Reader MCP is the evidence-grade document reader for agents. Its job is to
turn PDFs into citeable document twins with text, layout, tables, visual
regions, trust warnings, and source evidence.

## Current Boundary

The current package exposes `read_pdf`, `search_pdf`, and `pdf_evidence`.

## SOTA End-State

The final product should be the default local PDF instrument for agents:
fast, private, layout-aware, OCR-capable, table-aware, citation-safe, and honest
about unsupported or degraded extraction.

## Target Architecture

- Keep the existing tool surface stable.
- Migrate parsing, search, rendering, OCR orchestration, hashing, layout
  indexing, and MCP serving to Rust.
- Use `modelcontextprotocol/rust-sdk` / `rmcp` for the Rust MCP server.
- Preserve the Agent Document Twin contract and source evidence semantics.
- Use the portfolio evidence envelope for page, bbox, text offset, image region,
  table cell, and rendering route.

## Feature Pillars

- Auto-read policy: choose the best extraction path from document inspection.
- Citation safety: page, bounding box, text offsets, source hash, and renderer
  route.
- Table and layout extraction: headers, cells, spans, figures, formulas, and
  reading order.
- Visual evidence: page render, crop, OCR, and region analysis.
- Trust warnings: scanned pages, hidden text, malformed files, encryption,
  lossy OCR, redactions, and suspicious metadata.
- Batch and cache: repeated agent reads should be fast and deterministic.

## Roadmap

### Phase 0: Evidence Contract Lock

- Freeze V3 evidence semantics.
- Add minimal and rich JSON examples for every operation.
- Add package install diagnostics.
- Add benchmark fixture coverage for text, scanned, table-heavy, and malformed
  PDFs.

### Phase 1: Native Performance Layer

- Move hot-path hashing, text indexing, region lookup, and page cache into Rust
  where benchmarks justify it.
- Add a Rust MCP server facade that preserves the existing tool contracts.
- Add streaming for large PDFs.
- Add deterministic cache invalidation by source hash and options hash.

### Phase 2: Layout And Table Depth

- Improve table structure, merged cells, captions, and layout hierarchy.
- Add confidence and extraction route per table.
- Add cross-page table handling.

### Phase 3: Visual Evidence Expansion

- Add richer crop, region, formula, chart, and annotation evidence.
- Add OCR provider contract with clear local and remote policies.
- Add redaction and hidden-text checks.

### Phase 4: Release Scale

- Ship optional binary packages for the Rust MCP server.
- Publish public benchmark scorecard.
- Add enterprise install mode with no network runtime dependency.

## Validation Gates

- Every extracted citation can be rendered or re-inspected.
- Malformed and encrypted PDFs return structured warnings.
- OCR output is never presented as selectable text without route disclosure.
- Benchmarks prove Rust native acceleration before it becomes default.
- Package install succeeds across supported platforms.

## ADRs To Land In PDF Reader

- Native acceleration boundary.
- Rust MCP server boundary.
- OCR provider and privacy policy.
- Layout and table confidence model.
- Cache invalidation model.
- Enterprise offline install mode.
