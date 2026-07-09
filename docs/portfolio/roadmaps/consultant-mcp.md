# Roadmap: Consultant MCP

## Category Position

Consultant MCP is the structured decision-review and research panel for agents.
Its job is to turn high-stakes reasoning into typed requests, model fan-out,
judge synthesis, evidence gaps, and actionable recommendations.

## Current Boundary

The current package exposes four typed tools:

- `consultant.review_decision`
- `consultant.research`
- `consultant.challenge_answer`
- `consultant.compare_options`

## SOTA End-State

The final product should be the agent-native review board: budget-aware,
privacy-aware, typed, auditable, provider-agnostic, and strict about separating
evidence, assumptions, disagreements, and recommendations.

## Target Architecture

- Rust MCP server using `modelcontextprotocol/rust-sdk` / `rmcp` for tool
  schemas, request validation, and serving.
- Rust deterministic core for policy, redaction, request hashing, cache keys,
  ledger storage, and replay tooling. Provider clients remain replaceable
  integration modules behind the Rust policy boundary.
- Shared evidence envelope for consultation id, request hash, model route,
  policy decisions, cited sources, evidence gaps, and judge confidence.

## Feature Pillars

- Typed intents: decision review, research, challenge, and option comparison.
- Panel policy: model selection, fan-out size, budget, timeout, and retry rules.
- Judge synthesis: consensus, disagreement, blind spots, evidence gaps, and
  final recommendation.
- Privacy: redaction, data-minimization, local policy, and provider trace.
- Audit: request hash, prompt version, model versions, costs, latency, and
  replay metadata.
- Evaluation: fixtures for decision quality, JSON validity, citation handling,
  and failure behavior.

## Roadmap

### Phase 0: Beta Contract Hardening

- Freeze tool schemas and result shape.
- Add examples for all four tools.
- Add strict JSON validation and degraded-mode outputs.
- Add budget and privacy docs.

### Phase 1: Policy And Ledger Core

- Add deterministic request hashing, redaction trace, and consultation ledger.
- Add cache and replay semantics.
- Add provider failure taxonomy.
- Implement Rust policy, ledger, and replay primitives.

### Phase 2: Evaluation Harness

- Add fixture consultations with expected rubric outcomes.
- Add judge consistency checks.
- Add citation and evidence-gap scoring.
- Add regression tests for malformed model output.

### Phase 3: Advanced Routing

- Add task-aware model routing.
- Add confidence-based second-pass review.
- Add cost and latency optimizer.
- Add privacy tiers for provider selection.

### Phase 4: Release Scale

- Publish benchmark and eval reports.
- Add enterprise policy profiles.
- Add direct integration examples for architecture review and incident review.

## Validation Gates

- All tool outputs validate against schema.
- Failed panel calls do not hide degraded confidence.
- Provider trace includes model, latency, and policy route.
- Redaction is tested with sensitive fixtures.
- Eval fixtures prevent regression in evidence-gap reporting.

## ADRs To Land In Consultant MCP

- Provider policy and routing.
- Redaction and privacy boundary.
- Consultation ledger format.
- Judge synthesis rubric.
- Rust MCP server and deterministic policy-core decision.
