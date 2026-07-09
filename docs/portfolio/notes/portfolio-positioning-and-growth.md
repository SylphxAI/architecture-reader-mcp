# Portfolio Note: Portfolio Positioning And Growth Standard

Status: planning note
Date: 2026-07-09  
Decision owner: SylphxAI

## Context

The business target is a portfolio where each MCP can become a category leader
and average 10,000+ GitHub stars. Star growth is not the primary value, but it
is a useful proxy for developer pull, install trust, and brand reach.

Category-leading MCPs need stronger product packaging than internal utilities.
They must communicate their job instantly, prove value quickly, and work
reliably for users who have never seen the codebase.

## Decision

Each MCP will be positioned as a focused product with a single category promise.
Portfolio consistency matters as much as individual repo quality.

Every repo must have:

- a sharp one-line promise;
- clear tool list and when to use each tool;
- install command and MCP client config near the top;
- one tiny fixture and one realistic demo;
- stable output examples;
- performance and trust claims backed by commands;
- security boundary and local data policy;
- release and package status;
- roadmap with SOTA end-state and validation gates.

The portfolio brand should emphasize:

- agent-native evidence;
- local-first trust;
- Rust-first speed;
- predictable installation;
- composable tools instead of monolithic dashboards;
- measurable quality.

## Rationale

Developers star and adopt repos that are immediately legible, reliable, and
ambitious. Agents benefit from small high-trust tools with precise contracts.
The company benefits when the suite looks like one intentional product family.

## Consequences

Docs must avoid vague superlatives unless there is proof. Planning documents may
state goals, but README claims must reflect shipped behavior.

Each repo should maintain a product-quality roadmap. Roadmaps can be ambitious,
but every milestone must include validation gates and release evidence.

## Validation Gates

- New users can install and run a fixture demo in under one minute.
- README has no unsupported performance or adoption claim.
- Benchmarks run locally and in CI.
- Package publish status is visible.
- Tool examples are generated or checked by tests.
- Security boundaries are documented before write-capable tools are promoted.
