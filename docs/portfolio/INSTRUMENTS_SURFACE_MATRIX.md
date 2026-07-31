# Instruments surface matrix (living evidence)

Updated: 2026-07-31

| Brand | Repo | Core | SDK | CLI | MCP | Tests (honest) |
| --- | --- | --- | --- | --- | --- | --- |
| **Citra** | pdf-reader-mcp | Rust pure engine | `@sylphx/pdf-reader-mcp/sdk` (`Citra`) | `pdf-reader-mcp` / `citra` bin alias | yes | extensive existing + citra-sdk-export |
| **Iris** | image-reader-mcp | Rust+TS | `./sdk` (`Iris`) | `image-reader-mcp` | yes | existing suite; SDK thin façade |
| **Cue** | video-reader-mcp | Rust+TS | `./sdk` (`Cue`) | `video-reader-mcp` | yes | existing suite; SDK thin façade |
| **Prism** | smart-reader-mcp | Rust sniff + TS | `./sdk` (`Prism`) | `smart-reader-mcp` | yes | existing suite; SDK thin façade |
| **Spine** | architecture-reader-mcp | Rust graph | `./sdk` (`Spine`) | `bin/spine` + JSON cli | yes (8 tools incl. path) | cargo 52+; spine-sdk unit |
| **Lookout** | lookout | TS engine + Rust policy | `@sylphx/lookout` | `bin/lookout` | yes | bun 9 offline + optional live |

Archived (not in matrix): filesystem-mcp, awesome-mcp-servers, …

## Automated gate

```bash
node scripts/check-instruments-surfaces.mjs
# requires sibling checkouts under ../ (SylphxAI/*)
```

Brand alias packages (publish-ready): `packages/instruments-aliases/{citra,iris,cue,prism,spine}`.
