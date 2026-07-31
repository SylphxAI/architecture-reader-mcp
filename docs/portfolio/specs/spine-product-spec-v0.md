# Spine — Product Spec v0 (greenfield)

Status: **accepted intent** (2026-07-31)  
Brand: **Spine**  
Transitional repo: `architecture-reader-mcp`  
Independence: this repo is product SSOT; siblings are separate repos.

## 1. Job

Local architecture engine for agents and apps:

> Index a repository once. Query structure, paths, and blast radius with **file:line proof**. No cloud. No required LLM. **SDK · CLI · MCP.**

## 2. Competitive anchors

| Anchor | Spine stance |
| --- | --- |
| [Understand-Anything](https://github.com/Egonex-AI/Understand-Anything) | Same category. Different weapon: agent/SDK engine, deterministic core — not dashboard + multi-agent LLM as structural truth |
| [Graphify](https://github.com/Graphify-Labs/graphify) | Absorb local AST honesty + edge tags + CLI query UX; add first-class MCP/SDK and architecture/impact workflows |
| [Serena](https://github.com/oraios/serena) | Do **not** own symbol edit |

## 3. Constraints

Local-first · extreme performance · extreme lightweight · powerful · clear tools · evidence contract · full surfaces · full tests.

Structural graph production **must not require LLM**. Optional semantic enrich is off by default and labeled `route=llm`.

## 4. Graph and evidence

- Nodes: packages, modules, files, symbols (as extracted), routes, schemas, workflows, docs/ADR refs as available
- Edges: imports, calls (best-effort), owns, routes_to, configures, tests, … with `extracted` | `inferred`
- Every answer includes locators (path + line range when known), extractor route, coverage gaps

## 5. Tools (do not merge)

`architecture_index`, `architecture_status`, `architecture_overview`, `architecture_search`, `architecture_path`, `architecture_trace`, `architecture_impact`, `architecture_evidence`  
Advanced: `architecture_context_pack`

## 6. Surfaces

| Surface | Target |
| --- | --- |
| Core | Rust |
| CLI | `spine index .` · `spine path A B` · `spine impact --git-diff` · `spine mcp` |
| SDK | `Architecture.create({root}).index().search().path().impact()` |
| MCP | same tool names |
| Tests | golden polyglot fixture; incremental; MCP contract; perf gates |

## 7. Phases

| Phase | Exit |
| --- | --- |
| 0 | Spec + schema + fixture freeze |
| 1 | Index/search/path/evidence; CLI + Rust SDK; goldens |
| 2 | Impact/overview; MCP + TS SDK; perf |
| 3 | More languages; optional HTML viewer on same graph artifact; optional LLM enrich |

## 8. Non-goals

Dashboard-first identity; CodeRAG replacement; filesystem product; web search; required multi-agent LLM pipeline.

## 9. Relation to current tree

Existing `architecture-reader-mcp` code is a **starting point, not a sacred shape**. Greenfield refactors that better hit this spec are preferred over preserving transitional scaffolding.
