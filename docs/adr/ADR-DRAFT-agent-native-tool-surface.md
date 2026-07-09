# ADR-DRAFT: Agent-Native Tool Surface

Date: 2026-07-09

## Status

Accepted

## Context

The product is mainly for AI agents, not humans browsing a dashboard. Agents
need compact structured responses, stable schemas, provenance, and drilldown
paths. They do not need large visual graph payloads by default.

## Decision

Expose a small MCP tool set:

- `architecture_index`
- `architecture_status`
- `architecture_overview`
- `architecture_search`
- `architecture_trace`
- `architecture_impact`
- `architecture_evidence`

Every tool returns a shared response envelope with repository freshness,
answer, evidence, gaps, and metrics.

## Consequences

Positive:

- Agents can compose tools predictably.
- Overview answers stay compact while evidence remains accessible.
- Tool contracts can be validated independently of prose docs.

Negative:

- Some human-facing exploration features are deferred.
- Tool design must be strict; vague fields will make downstream agents brittle.

## Alternatives Considered

### Single `architecture_query` Tool

Rejected. One broad tool would be easier to expose but harder for agents to
plan, cache, and validate.

### Dashboard-First API

Rejected. Dashboard payloads are too large and too presentation-shaped for the
primary agent workflow.
