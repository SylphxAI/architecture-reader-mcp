# Instruments brand alias packages

Publish-ready npm aliases:

| Brand package | Delegates to |
| --- | --- |
| `@sylphx/citra` | `@sylphx/pdf-reader-mcp` |
| `@sylphx/iris` | `@sylphx/image-reader-mcp` |
| `@sylphx/cue` | `@sylphx/video-reader-mcp` |
| `@sylphx/prism` | `@sylphx/smart-reader-mcp` |
| `@sylphx/spine` | `@sylphx/architecture-reader-mcp` |
| `@sylphx/lookout` | first-class package in [SylphxAI/lookout](https://github.com/SylphxAI/lookout) |

These packages are **not auto-published** in this change (npm auth required). They exist so brand install can ship without renaming transitional repos immediately.

```bash
# from this monorepo after linking deps
node scripts/check-instruments-surfaces.mjs
```
