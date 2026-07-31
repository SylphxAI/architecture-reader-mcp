# Publish status — Spine

| Field | Value |
| --- | --- |
| Transitional npm | `@sylphx/architecture-reader-mcp` |
| Brand npm | `@sylphx/spine` |
| Version | `0.2.1` |
| Registry | **live** (dual expand–contract where brand ≠ transitional) |
| Auth | GitHub org `NPM_TOKEN` via publish workflows |

## Install

```bash
# preferred brand
npm i -g @sylphx/spine
# transitional still valid during expand
npm i -g @sylphx/architecture-reader-mcp
```

Workflows: `publish-npm-package.yml`, `publish-brand-alias.yml`.
