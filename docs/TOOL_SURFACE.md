# Tool surface — this product

Policy: **few, powerful, obvious** tools. Prefer the primary read tool first.

| Tool | Role |
| --- | --- |
| `architecture_index` | Build/update local index |
| `architecture_status` | Index readiness |
| `architecture_overview` | Architecture overview |
| `architecture_search` | Search graph |
| `architecture_path` / `architecture_trace` | Path/trace between nodes |
| `architecture_impact` | Change impact |
| `architecture_evidence` | Resolve evidence ids → file:line payloads |
| `architecture_context_pack` | Advanced pack |

## Rules

1. Do not add near-duplicate tools that only differ by vanity naming.
2. Advanced tools must be labeled advanced in README/skill.
3. Schema fields should be agent-obvious; fail closed on unsafe input.
4. Composition with sibling products is via public contracts, not monorepo imports.
