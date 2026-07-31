# Spine — product targets (Architecture)

Brand: **Spine** (transitional repo: `architecture-reader-mcp`)  
Company portfolio knowledge: https://github.com/SylphxAI/instruments (docs only; not owned by this repo)

## Final target

Become the default **local architecture engine** agents and apps call before
large edits, reviews, migrations, and incident triage:

- Deterministic structural graph by default (**no required LLM**)
- Query: overview, search, path, trace, impact, evidence
- **SDK + CLI + MCP** isomorphic APIs
- Extreme performance and lightweight footprint vs skill/dashboard-heavy tools

## Competitive anchors

| Anchor | Learn | Do not copy as identity |
| --- | --- | --- |
| Understand-Anything | Onboarding narrative, impact/diff workflow, multi-host distribution | Dashboard-first; multi-agent LLM as structural truth |
| Graphify | Local AST, EXTRACTED/INFERRED edges, CLI path/explain, 0 LLM for code | Skill-only distribution; weak first-class MCP/SDK productization |
| Serena | Agent-first tool clarity | Symbol edit / IDE ownership |

## Target surfaces

| Surface | Acceptance |
| --- | --- |
| Core | Rust graph index + query |
| SDK | Rust + TypeScript public APIs |
| CLI | `spine index|status|overview|search|path|trace|impact|evidence|doctor|mcp` |
| MCP | Clear tools (not one god-tool); Tool Search friendly descriptions |
| Tests | Golden graphs, MCP contracts, incremental index, perf gates |

## Target tool catalog (clear, separate)

- `architecture_index`
- `architecture_status`
- `architecture_overview`
- `architecture_search`
- `architecture_path`
- `architecture_trace`
- `architecture_impact`
- `architecture_evidence`
- Optional advanced: `architecture_context_pack`

## Phased goals

| Phase | Outcome |
| --- | --- |
| 0 | Spec + graph schema + fixtures frozen |
| 1 | Index/search/path/evidence on polyglot fixture; CLI + Rust SDK |
| 2 | Impact + overview; MCP + TS SDK; perf gates |
| 3 | More languages; optional viewer reading same graph file; optional LLM enrich **off by default** |

## Non-goals

- Visualization-first product identity
- Replacing CodeRAG generic retrieval
- Owning filesystem writes or web search
