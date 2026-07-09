# Roadmap: Video Reader MCP

## Category Position

Video Reader MCP is the temporal evidence reader for agents. Its job is to turn
video into inspectable timelines with metadata, subtitles, scenes, transcripts,
frames, warnings, and time-based evidence.

## Current Boundary

The current package exposes `read_video`.

## SOTA End-State

The final product should let agents answer "what happened when" with timestamped
evidence, not vague video summaries. It should be local-first, fast on common
formats, honest about codec or extraction limits, and able to produce follow-up
frame, crop, OCR, and transcript evidence.

## Target Architecture

- Rust core for probing, timeline assembly, hashing, cache management, and
  orchestration.
- Rust MCP server using `modelcontextprotocol/rust-sdk` / `rmcp`.
- Native media tool integration through controlled adapters.
- Shared evidence envelope for file hash, stream id, timestamp, frame index,
  subtitle span, transcript span, scene id, and extraction route.
- Optional adapter for speech, OCR, and visual-region providers.

## Feature Pillars

- Probe: container, streams, codecs, duration, frame rate, chapters, metadata,
  and warnings.
- Timeline: scenes, subtitles, transcript segments, silence, frame samples, and
  key events.
- Evidence: thumbnails, frame crops, OCR over frames, subtitle locators, and
  transcript locators.
- Performance: bounded sampling, caching, and large-file streaming.
- Safety: codec failures, truncated files, missing timestamps, and unsupported
  streams reported clearly.

## Roadmap

### Phase 0: Timeline Contract

- Freeze `read_video` output envelope.
- Add fixtures for subtitle, no-subtitle, long, corrupted, and multi-stream
  videos.
- Add install diagnostics for required native media dependencies.

### Phase 1: Rust Timeline Core

- Implement native timeline model, cache keys, hash policy, and stream metadata
  handling.
- Add deterministic scene and subtitle fixtures.
- Add benchmark gates for probe and timeline assembly.

### Phase 2: Evidence Operations

- Add frame render, thumbnail, crop, and OCR follow-up operations.
- Add transcript provider contract with local and remote policy controls.
- Add route disclosure for every generated observation.

### Phase 3: Agent Temporal Twin

- Add compact timeline summaries for agent token budgets.
- Add search over transcript, subtitles, OCR, and scene labels.
- Add cross-media delegation through Smart Reader.

### Phase 4: Release Scale

- Ship native binary packaging where possible.
- Publish benchmark scorecard across fixture formats.
- Ship the Rust MCP server as the canonical runtime once media dependency
  packaging is stable.

## Validation Gates

- Timestamp locators are stable across repeated runs.
- Unsupported codecs return structured warnings.
- Large files stream without full memory load.
- Extracted frames can be reproduced by follow-up calls.
- Install diagnostics identify missing native media capabilities.

## ADRs To Land In Video Reader

- Native media adapter boundary.
- Rust MCP server boundary.
- Timeline evidence schema.
- Transcript and OCR provider policy.
- Large-file cache model.
- Media dependency packaging.
