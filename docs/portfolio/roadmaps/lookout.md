# Lookout — product targets (Web)

Brand: **Lookout** (new product; not yet scaffolded)  
SSOT: `docs/portfolio/sylphx-instruments-ssot.md`

## Final target

Become the default **local-first web instrument** for agents and apps:

- Search, fetch, extract, cache without a required vendor API key
- Citeable excerpts and honest degradation
- **Tiny default install** (no mandatory multi-GB browser/model warmup)
- **SDK + CLI + MCP** (REST optional later)

## Competitive anchor

| Anchor | Learn | Differ |
| --- | --- | --- |
| wigolo | Multi-surface (MCP/CLI/SDK/REST), clear tools, $0 narrative, score honesty, agent install wiring | Mandatory heavy runtime (~1.5GB), AGPL; default must stay light |
| Tavily / Exa / host WebSearch | Result quality bars | Metered cloud keys — contrast, don’t clone billing model |

## Target tool catalog

**Core**

- `web_search`
- `web_fetch`
- `web_extract`
- `web_cache`

**Advanced**

- `web_crawl`, `web_find_similar`, `web_diff` / `web_watch`, `web_research`  
- Avoid Day-1 mega autonomous `agent` god-tool; host agents can orchestrate.

## Phased goals

| Phase | Outcome |
| --- | --- |
| 0 | Spec: SSRF/robots/rate limits, tool schemas, non-goals |
| 1 | Light search adapters + fetch + extract + disk cache; CLI + MCP + TS SDK |
| 2 | Rank fusion explainability; cite spans; optional headless profile |
| 3 | Crawl/similar/watch; Python SDK; REST; research helper |

## Non-goals

- Repo/code search (Spine / code intelligence)
- Owning PDF/image deep parse (hand off to Citra/Iris after fetch when needed)
- Crawler SaaS identity
