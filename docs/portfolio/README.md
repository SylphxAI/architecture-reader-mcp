# Sylphx Instruments — Portfolio

Status: **accepted baseline** (2026-07-31)  
Audience: implementation agents, maintainers, product owners

## Start here

| Document | Purpose |
| --- | --- |
| **[sylphx-instruments-ssot.md](./sylphx-instruments-ssot.md)** | **Portfolio SSOT** — constraints, evidence meaning, surfaces, brands, products, priority |
| **[ADR-3 Naming](../adr/ADR-3-sylphx-instruments-naming.md)** | Locked product names |
| [roadmaps/citra.md](./roadmaps/citra.md) | PDF / Citra targets |
| [roadmaps/spine.md](./roadmaps/spine.md) | Architecture / Spine targets |
| [roadmaps/lookout.md](./roadmaps/lookout.md) | Web / Lookout targets |
| [specs/spine-product-spec-v0.md](./specs/spine-product-spec-v0.md) | Spine greenfield spec |
| [specs/lookout-product-spec-v0.md](./specs/lookout-product-spec-v0.md) | Lookout product spec |
| [roadmaps/iris-cue-prism.md](./roadmaps/iris-cue-prism.md) | Image / Video / Router targets |
| [notes/portfolio-positioning-and-growth.md](./notes/portfolio-positioning-and-growth.md) | Historical growth packaging note (subordinate to SSOT) |

Older per-repo files under `roadmaps/*-reader-mcp.md` are transitional. Prefer brand roadmaps + SSOT on conflict.

## Thesis

Build a coherent suite of **local-first agent instruments**, not a loose bag of
`*-reader-mcp` utilities. Each product owns one category job, installs quickly,
returns citeable results, exposes **SDK + CLI + MCP**, and proves speed with
reproducible benches.

Star growth (including ambitious public targets) is an **outcome**. Acceptance
is the readiness checklist in the SSOT.

## Brands

| Brand | Job |
| --- | --- |
| **Citra** | PDF evidence for agents |
| **Iris** | Image evidence |
| **Cue** | Video timeline evidence |
| **Prism** | Local media routing |
| **Spine** | Repo architecture engine |
| **Lookout** | Local web search/fetch |

## Active vs archived

**Active:** Citra, Iris, Cue, Prism, Spine, Lookout (planned).  

**Archived (no primary investment):** `filesystem-mcp`, `awesome-mcp-servers`,
retired consultant/sdk/linear/smart-read/rag-server/reader-evidence lines, etc.

## Runtime & packaging standards

- Prefer **Rust cores** for deterministic hot paths.
- Thin TS adapters only when they reduce protocol/package risk; they must not own semantics.
- npm shape: small wrapper + optional platform natives; **no required postinstall network binary fetch**; `doctor` for install failure diagnosis.
- WASM is for sandboxed/portable plugins — not the default local server runtime.

## Evidence standard

Outputs should be agent-usable evidence objects where claims matter: locators,
routes, warnings/gaps. See SSOT §3. There is **no** `evidence_first` tool.

## Tool standard

Clear, independently meaningful tools. Reasonable catalog size. No pathological
merge into one mega-tool; no vanity explosion. Core vs advanced in docs.

## Surface standard

Every Instruments product targets:

1. Domain core  
2. SDKs (TS + Rust long-term)  
3. CLI  
4. MCP  
5. Full automated tests (golden/contract/e2e/perf as applicable)

## Priority order

1. SSOT/docs lock (**done in this tree**)  
2. Citra template (brand + SDK + gates)  
3. Spine greenfield  
4. Lookout greenfield  
5. Iris / Cue  
6. Prism suite entry  
7. Cross-cutting registry & benches  

## Final goals (portfolio)

See [GOALS.md](./GOALS.md).
