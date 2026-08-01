# Tool surface — Spine

Policy: **few, powerful, obvious** tools. Prefer a short primary path first.

## Primary (agent default path)

| Tool | Role |
| --- | --- |
| `architecture_index` | Build/update local index (once per workspace) |
| `architecture_status` | Index readiness / coverage |
| `architecture_overview` | Top-level map |
| `architecture_search` | Find nodes with evidence locators |
| `architecture_path` | Shortest path with hop provenance |
| `architecture_impact` | Diff blast radius |

## Advanced (use only when needed)

| Tool | Role |
| --- | --- |
| `architecture_trace` | Multi-hop relation trace |
| `architecture_evidence` | Resolve evidence ids → file:line |
| `architecture_context_pack` | Neighborhood pack for large context fills |

## Rules

1. Lead with **index → status → overview/search**; do not open with context_pack.
2. Do not add near-duplicate vanity tools.
3. Schema fields agent-obvious; fail closed on unsafe roots.
4. Composition with Locus is public contracts only (chunk search ≠ architecture map).
5. Local-first: no LLM required for graph authority.
