# Sylphx Instruments — Portfolio SSOT

Status: **accepted baseline** (2026-07-31)  
Audience: product owners, implementation agents, maintainers  
Authority: this document is the portfolio source of truth for active
instrument products. Repo-local roadmaps must not contradict it; they may
only refine product-local detail.

## 1. One-line thesis

**Sylphx Instruments** is a family of **local-first agent instruments**:
extremely fast, extremely light, powerful tools with **first-class SDK, CLI,
and MCP** surfaces, clear tool names, and **citeable results** — not a pile of
`*-reader-mcp` utilities and not a cloud SaaS suite.

## 2. Hard constraints (every product)

| Constraint | Meaning |
| --- | --- |
| **Local-first** | Default path needs no cloud account and no required API key. Cache/config stay on machine. Remote is opt-in and labeled. |
| **Extreme performance** | Hot paths are native-fast. Publish reproducible p50/p95 for fixture workloads. |
| **Extreme lightweight** | Default install is small; cold start is short. Heavy browser/ML runtimes are **opt-in profiles**, never mandatory warmup. |
| **Powerful** | Solves a real agent job end-to-end; not a thin wrapper demo. |
| **Clear tools** | Tools are independently meaningful. Do not pathologically merge into one mega-`operation` tool. Do not multiply near-duplicates. Tool Search is common — clarity beats artificial minimal count. |
| **Evidence contract** | Results carry locators, routes, and honesty signals. “Evidence” is **not** a tool named `evidence_first`. |
| **Full surfaces** | **Core + SDK + CLI + MCP + full tests**. Same semantics on every surface. |
| **Dogfood** | Internal use goes through public SDK/CLI/MCP only. |

## 3. What “Evidence” means

**Evidence is a result contract, not a tool.**

Agents call ordinary tools (`read_*`, `search_*`, `architecture_impact`,
`web_fetch`, …). Responses should include, where applicable:

- **Locator** — page/bbox, timestamp, `path:line`, URL + span
- **Route** — how the fact was produced (`native_text`, `ocr`, `ast_import`, `http`, …)
- **Integrity** — source hash / etag when useful
- **Honesty** — warnings, gaps, `unknown`, blocked, degraded backends
- **Edge kind** (graphs) — `extracted` vs `inferred` (vs optional `llm`)

Anti-patterns:

- Marketing “Evidence First” with empty envelopes
- Envelopes so large default answers become unusable
- Pretending OCR/LLM output is deterministic structure without labels

External copy prefers plain language: *citations, not guesses*;
*file:line proof*; *local*.

## 4. Delivery shape (every product)

```text
Domain Core (prefer Rust)
        │
        ├─ SDKs (TypeScript + Rust required long-term; Python optional)
        ├─ CLI (humans, CI, scripting; includes doctor)
        └─ MCP (stdio primary; HTTP optional)
```

Rules:

1. Core owns semantics. MCP/CLI/SDK are adapters.
2. `sdk(x) === cli(x) === mcp(x)` for the same inputs (modulo transport framing).
3. Tests: unit/golden + CLI e2e + MCP contract + performance gates.
4. Packaging: small wrapper + optional platform natives where needed; no required
   postinstall network binary fetch; `doctor` explains install failures.

## 5. Tool granularity rules

**Create a separate public tool when most of these hold:**

1. Distinct job verb agents recognize
2. Materially different I/O shape
3. Different failure modes
4. Different permission class (e.g. read vs write / network)
5. Commonly invoked alone
6. Describable in two short sentences for Tool Search

**Do not split** mere detail flags (`include_*`, `detail=fast|balanced|full`).  
**Do not merge** unrelated jobs into one enum god-tool to “look simple.”  
**Do** use **core vs advanced** listing in docs/instructions when the catalog grows.

## 6. Brand and naming (locked)

**Umbrella:** Sylphx Instruments  

| Brand | Role | Transitional repo / package |
| --- | --- | --- |
| **Citra** | PDF evidence | `pdf-reader-mcp` / `@sylphx/pdf-reader-mcp` |
| **Iris** | Image evidence | `image-reader-mcp` |
| **Cue** | Video timeline evidence | `video-reader-mcp` |
| **Prism** | Local media format router | `smart-reader-mcp` |
| **Spine** | Repo architecture engine | `architecture-reader-mcp` |
| **Lookout** | Local web search/fetch | *new product (not created yet)* |

Target bins/packages (end state): `@sylphx/citra`, `citra`, etc.

**Rename policy**

1. Display brand first (README/site/hero).
2. CLI + MCP server id next.
3. New scoped packages; old names alias/deprecate.
4. Repo rename last.
5. **Citra** keeps the longest dual-name bridge (existing stars/SEO).

## 7. Active vs archived

### Active

Citra, Iris, Cue, Prism, Spine, Lookout (planned).

### Archived (do not invest as product lines)

- `filesystem-mcp` (archived 2026-07-31)
- `awesome-mcp-servers` (archived 2026-07-31)
- `consultant-mcp`, `mcp-server-sdk`, `linear-mcp`, `smart-read-mcp`,
  `rag-server-mcp`, `reader-evidence`, and other retired MCP SKUs

## 8. Product definitions

### 8.1 Citra (PDF) — flagship

- **Job:** Turn PDFs into citeable agent document twins (structure, tables, OCR,
  visual proof, trust signals) — locally.
- **Competitors:** Docling (+ MCP), MinerU, Marker, Xberg, assorted dump-style PDF MCPs.
- **Tools (keep clear):** `read_pdf`, `search_pdf`, `pdf_evidence`.
- **Status:** Production; reference implementation for family quality and messaging.
- **Priorities:** Citra branding layer; first-class SDK (not MCP-only); auto-first
  docs; install/registry/bench honesty; do not bloat tool count.

### 8.2 Iris (Image)

- **Job:** Deterministic image facts — metadata, OCR geometry, crops — not default VLM captions.
- **Status:** Early; treat as greenfield-capable.
- **Surfaces:** Core + SDK + CLI + MCP + tests.
- **Demos:** UI screenshots, chart digits, whiteboard/scan OCR boxes.

### 8.3 Cue (Video)

- **Job:** Timeline facts (probe, subtitles, scenes, keyframe locators) without full-video VLM.
- **Status:** Early; release/contract gates must be green before hard marketing.
- **Surfaces:** Core + SDK + CLI + MCP + tests.

### 8.4 Prism (router)

- **Job:** Sniff local media bytes → delegate to Citra/Iris/Cue → preserve provenance.
- **Rule:** Never reimplement sibling parsers. Do not lead suite marketing until
  Iris/Cue meet the family bar.
- **Tools:** One primary read/route tool is fine because the job is singular.

### 8.5 Spine (Architecture)

- **Job:** Local architecture engine for agents/apps: index, overview, search,
  path/trace, impact, evidence with file:line proof.
- **Primary competitive anchors:**
  - [Egonex-AI/Understand-Anything](https://github.com/Egonex-AI/Understand-Anything) (~77k):
    teach humans with dashboard + multi-agent LLM semantics + multi-host skills.
  - [Graphify-Labs/graphify](https://github.com/Graphify-Labs/graphify) (~99k):
    local tree-sitter graph, EXTRACTED/INFERRED edges, CLI path/explain, 0 LLM for code.
- **Side boundary:** [oraios/serena](https://github.com/oraios/serena) = symbol edit IDE — Spine does **not** own edits.
- **Win theme:** Faster/lighter deterministic core + **first-class SDK/CLI/MCP** +
  architecture/impact workflows. Not dashboard-first. Not required LLM multi-agent.
- **Suggested tools (clear, separate):**  
  `architecture_index`, `architecture_status`, `architecture_overview`,
  `architecture_search`, `architecture_path`, `architecture_trace`,
  `architecture_impact`, `architecture_evidence`  
  Advanced optional: `architecture_context_pack`.
- **Status:** Public beta/draft; **greenfield redesign allowed**.

### 8.6 Lookout (Web) — planned

- **Job:** Local-first web access for agents/apps: search, fetch, extract, cache;
  no required API key; $0 metered cloud bill for core path.
- **Primary anchor:** [KnockOutEZ/wigolo](https://github.com/KnockOutEZ/wigolo) (~4k):
  MCP+CLI+REST+SDK, clear tools, citeable excerpts; heavy ~1.5GB init (browser +
  on-device models); AGPL.
- **Win theme:** **Tiny default**, Rust hot path, permissive license posture
  aligned with family, optional heavy profile later.
- **Core tools:** `web_search`, `web_fetch`, `web_extract`, `web_cache`.  
  Advanced: crawl, similar, diff/watch, research (avoid Day-1 mega `agent` tool).
- **Status:** Not scaffolded yet; specified here as portfolio intent.

## 9. Portfolio topology

```text
                    Prism (local media route)
                     │
         ┌───────────┼───────────┐
         ▼           ▼           ▼
      Citra        Iris         Cue
     (PDF)       (image)      (video)

Spine ── local repo structure / impact
Lookout ── external web search / fetch
```

Orthogonal on purpose. Do not merge web crawl into Prism. Do not merge
architecture into media readers. Code retrieval (CodeRAG) remains a sibling
code-intelligence product, not Spine’s job.

## 10. Definition of “best” / public-ready

A product may claim family-grade readiness only if:

1. One-sentence job + before/after is true and demoable in ≤60s  
2. SDK + CLI + MCP exist and match semantics  
3. Tests: golden/contract + e2e + perf gate green  
4. Default path is local-first and lightweight  
5. Tools are clear; docs list core vs advanced  
6. Results meet the evidence contract where claims matter  
7. Security boundaries documented (path, SSRF, size, robots as applicable)  
8. README uses brand name; unsupported claims absent  

GitHub stars (including 10k aspirations) are **outcomes**, not acceptance gates.
Engineering accepts on the checklist above.

## 11. Execution priority (portfolio)

1. **Lock docs** (this SSOT + naming + per-product targets) — *this change*  
2. **Citra** — brand layer + SDK-as-product template + quality gates  
3. **Spine** — greenfield engine vs UA/Graphify  
4. **Lookout** — greenfield light web vs wigolo  
5. **Iris / Cue** — media bar  
6. **Prism** — suite entry after children qualify  
7. Cross-cutting registry, install blocks, public benches  

## 12. Explicit non-goals

- Reviving archived filesystem/awesome MCP lines as primary bets  
- Pathological tool merging or vanity tool inflation  
- Parallel “everything to 10k stars this quarter” without sequence  
- Required LLM multi-agent to produce structural truth (Spine/Lookout/Citra core)  
- Dashboard- or model-runtime-first identity  
- Using “Evidence First” as a substitute for measurable quality  

## 13. Document control

| Field | Value |
| --- | --- |
| Accepted | 2026-07-31 |
| Supersedes | Informal portfolio notes that still list filesystem as active control line or treat star count as a hard gate |
| Related | `docs/portfolio/README.md`, `docs/adr/ADR-3-sylphx-instruments-naming.md`, product roadmaps under `docs/portfolio/roadmaps/` |

