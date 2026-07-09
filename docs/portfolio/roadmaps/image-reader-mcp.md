# Roadmap: Image Reader MCP

## Category Position

Image Reader MCP is the deterministic image evidence reader for agents. Its job
is to extract measurable facts from images without turning uncertain visual
interpretation into unsupported claims.

## Current Boundary

The current package exposes `read_image`.

## SOTA End-State

The final product should return an Agent Media Twin for images: dimensions,
format, metadata, OCR text, regions, crops, thumbnails, warnings, and optional
vision-provider observations with explicit confidence and route.

## Target Architecture

- Rust core for image decode, metadata parsing, hashing, crop extraction,
  resizing, color/profile inspection, and batch operations.
- Thin MCP adapter during early releases.
- Optional OCR and vision providers behind explicit provider contracts.
- Shared evidence envelope for source hash, pixel region, OCR box, metadata
  route, and redaction policy.

## Feature Pillars

- Metadata and trust: EXIF, GPS redaction, dimensions, orientation, color
  profile, timestamps, and warnings.
- OCR: text lines, bounding boxes, language hints, confidence, and route.
- Region evidence: crops, thumbnails, object or layout regions, and exact pixel
  locators.
- Safety: privacy redaction, decompression limits, oversized image handling,
  malformed file detection.
- Batch: directory reads, deduplicated hashes, and cache reuse.

## Roadmap

### Phase 0: Contract And Safety

- Freeze `read_image` envelope.
- Add examples for metadata-only, OCR, region, and warning-heavy images.
- Add decompression and oversized input tests.

### Phase 1: Rust Decode Core

- Implement native image probe, hash, metadata, resize, and crop engine.
- Add deterministic output fixtures.
- Add optional binary packaging.

### Phase 2: OCR And Region Model

- Add OCR provider interface.
- Add bounding-box evidence and crop follow-up operations.
- Add warnings for rotated, low-resolution, handwritten, or degraded text.

### Phase 3: Agent Media Twin

- Add scene/object/layout summaries only when provider route and confidence are
  explicit.
- Add redaction and privacy policy fields.
- Add batch mode with cache reuse.

### Phase 4: Public Quality Positioning

- Publish speed and accuracy fixtures.
- Add demo gallery with expected JSON.
- Add direct Rust MCP server evaluation.

## Validation Gates

- Metadata extraction is deterministic.
- GPS redaction is tested.
- OCR claims include confidence and box locators.
- Large or malicious images fail safely.
- Native install has `doctor` diagnostics.

## ADRs To Land In Image Reader

- Rust image core boundary.
- OCR provider contract.
- Region and crop evidence schema.
- Privacy redaction policy.
- Batch cache model.
