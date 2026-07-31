# Sylphx Instruments — Final goals

Status: accepted (2026-07-31)  
Linked session goal: active Codex goal *without token budget cap* for this program.

## North-star goal

**Ship Sylphx Instruments** as the coherent local-first instrument family for
agents and applications: Citra, Iris, Cue, Prism, Spine, and Lookout — each
**powerful, extremely fast, extremely light**, with **SDK + CLI + MCP + full
tests**, clear tools, and citeable results.

## Locked outcomes

### G0 — Documentation & authority (immediate)

- [x] Portfolio SSOT written (`sylphx-instruments-ssot.md`)
- [x] Naming ADR accepted (`ADR-3-sylphx-instruments-naming.md`)
- [x] Per-line target roadmaps for Citra, Spine, Lookout, Iris/Cue/Prism
- [x] Portfolio README points at SSOT
- [x] Land SSOT on default branch / visible origin tip
- [x] Citra/Iris/Cue/Prism/Spine READMEs adopt brand heroes (package names still transitional)

### G1 — Family constitution in force

- Local-first default on every active product  
- Evidence = result contract (not a tool)  
- Tool clarity rules (no pathological merge)  
- Core + SDK + CLI + MCP + tests as the completeness bar  
- Archived lines stay archived (filesystem, awesome-mcp-servers, …)

### G2 — Citra (PDF flagship)

- [x] Brand **Citra** primary in public copy  
- [x] TS `Citra` SDK class export (`@sylphx/pdf-reader-mcp/sdk`) over pure-rust client  
- [x] `citra` bin alias on package (transitional npm name retained)  
- [ ] Dedicated `@sylphx/citra` package publish  
- First-class SDK surfaces continue (Rust crate ergonomics) isomorphic with MCP  
- Auto-first agent path; progressive power-user options  
- Install/registry/bench honesty  
- Remains packaging & quality template for the family  

### G3 — Spine (architecture)

- [x] Product spec v0 (`docs/portfolio/specs/spine-product-spec-v0.md`)
- [x] `architecture_path` tool with hop provenance + tests
- [x] Human CLI `bin/spine` + TS SDK export `@sylphx/architecture-reader-mcp/sdk`
- [ ] Full language coverage / perf gates / npm package rename to spine


- Greenfield-capable redesign under Instruments rules  
- Competitive clarity vs Understand-Anything + Graphify; no Serena-edit ownership  
- Deterministic structural graph without required LLM  
- Target tools: index/status/overview/search/path/trace/impact/evidence  
- SDK + CLI + MCP + golden/perf tests  

### G4 — Lookout (web)

- [x] Product spec v0 (`docs/portfolio/specs/lookout-product-spec-v0.md`)
- [x] Public scaffold repo [SylphxAI/lookout](https://github.com/SylphxAI/lookout) (SDK/CLI/core stubs)
- [x] Phase 1 engine: web_search/fetch/extract/cache + SSRF + local cache + MCP/CLI/SDK + tests
- [ ] Optional heavy browser profile; multi-engine quality bar vs wigolo


- New product specified vs wigolo  
- Light default (no mandatory multi-GB warmup)  
- Core tools: search/fetch/extract/cache  
- SDK + CLI + MCP + tests  
- SSRF/robots/rate-limit policy enforced  

### G5 — Iris, Cue, Prism

- [x] Iris/Cue SDK façades (`@sylphx/*-reader-mcp/sdk`)
- [x] Prism SDK façade (`@sylphx/smart-reader-mcp/sdk`)
- [ ] Iris/Cue meet full media evidence bar (perf/release gates)
- Prism routes only; suite push after children qualify  
- Full surfaces and tests for all three  

### G6 — Distribution & proof

- MCP registry / client install blocks where applicable  
- Public reproducible benches per product  
- Optional Instruments overview page  

## Non-goals (final)

- Star count as a hard engineering gate  
- Reviving filesystem-mcp as a primary Instruments brand without new ADR  
- LLM multi-agent as the only way to produce structure  
- Mega-tool catalogs or mega-merged god tools  

## Priority sequence

`G0 → G2 → G3 → G4 → G5 → G6` (G1 is continuous policy).

- [x] Living surface matrix: `docs/portfolio/INSTRUMENTS_SURFACE_MATRIX.md`
- [x] Automated surface gate `scripts/check-instruments-surfaces.mjs` (6/6 when siblings present)
- [x] Lightweight multi-repo test orchestrator `scripts/run-instruments-tests.mjs` / `bun run test:instruments`

- [x] Publish-ready brand alias packages under `packages/instruments-aliases/*` (`@sylphx/citra` … `@sylphx/spine`)


## Completion rule

A goal item is complete only with **durable artifacts** (docs landed, packages
published, or tests/benches green on a declared revision) — not chat claims alone.
