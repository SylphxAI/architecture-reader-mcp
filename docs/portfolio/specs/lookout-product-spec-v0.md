# Lookout — Product Spec v0

Status: **accepted intent** (2026-07-31)  
Brand: **Lookout**  
Family: [Sylphx Instruments SSOT](../sylphx-instruments-ssot.md)  
Primary competitor: [KnockOutEZ/wigolo](https://github.com/KnockOutEZ/wigolo)

## 1. Problem

Agents need the web (docs, issues, APIs, blogs) but:

- Host built-in web search is opaque / metered / non-portable across clients
- Cloud search APIs (Tavily, Exa, …) require keys and per-query cost
- Existing local stacks (wigolo) prove demand but ship **heavy** runtimes (~1.5GB browser + on-device models) and AGPL constraints

## 2. Job

**Lookout** is the local-first **web instrument** for agents and apps:

> Search, fetch, extract, and cache web content with citeable excerpts — no required API key, tiny default install, same API on SDK / CLI / MCP.

## 3. Non-goals

- Repo / architecture search (Spine)
- Deep PDF/image intelligence (Citra / Iris) — may **hand off** fetched bytes
- Crawler SaaS / multi-tenant hosted product as core identity
- Day-1 autonomous multi-step `agent` god-tool (hosts orchestrate)

## 4. Constraints

| Constraint | Requirement |
| --- | --- |
| Local-first | Default needs no vendor API key; cache under user data dir |
| Light | Default path: no mandatory headless browser or multi-GB models |
| Fast | Publish p50/p95 for search + fetch fixtures |
| Powerful | Multi-engine search fusion path; clean fetch; structured extract; cache replay |
| Safety | SSRF deny private ranges; redirect limits; robots/rate-limit policy; size caps |
| License | Prefer MIT/Apache family consistent with Instruments |

## 5. Surfaces

| Surface | Acceptance |
| --- | --- |
| Core | Prefer Rust for fetch policy + parse hot path |
| SDK | TypeScript + Rust long-term; isomorphic calls |
| CLI | `lookout search|fetch|extract|cache|doctor|mcp` |
| MCP | Core tools listed below; clear descriptions |
| Tests | Unit + contract + offline fixtures; live e2e optional/gated |
| Optional later | REST (wigolo-like), heavy browser profile, Python SDK |

## 6. Tools (clear, not merged)

### Core

| Tool | Input (min) | Output essentials |
| --- | --- | --- |
| `web_search` | `query` (string or string[]) | ranked hits, scores/explanation, engine telemetry, warnings |
| `web_fetch` | `url` | markdown/text, metadata, links, **cite spans**, block/degraded codes |
| `web_extract` | `url` or fetched id + schema/mode | tables, json-ld, named schemas, custom JSON Schema |
| `web_cache` | query / stats / clear | local replay, change hints |

### Advanced (phase 2+)

`web_crawl`, `web_find_similar`, `web_diff`, `web_watch`, `web_research`

## 7. Evidence contract (web)

Every successful fetch/search hit that claims content should allow citation:

- URL (final after redirects, with redirect chain if material)
- Excerpt text + span (byte or paragraph locator)
- `route` (`http`, `browser_optional`, `cache`, …)
- Honesty: `blocked_by_challenge`, engine failure, truncation, stale cache

## 8. Phases

| Phase | Exit criteria |
| --- | --- |
| 0 | This spec accepted; security policy drafted; tool schemas frozen |
| 1 | Light search (2–4 public adapters) + fetch + extract + cache; CLI+MCP+TS SDK; offline tests green |
| 2 | Explainable rank fusion; stronger cite spans; optional headless profile |
| 3 | Crawl/similar/watch; research helper; REST; broader SDK |

## 9. Success metrics

- Cold `search` + `fetch` usable without heavy warmup
- Default install footprint << wigolo full init
- Same query via SDK/CLI/MCP yields same semantic payload
- SSRF suite 100% deny on private targets
- Public bench table with honest methodology

## 10. Open decisions (not blockers for Phase 0/1 start)

- Exact public search adapters list (legal/ToS review)
- Cache directory XDG vs `~/.lookout`
- Package name cutover `@sylphx/lookout` only vs dual
