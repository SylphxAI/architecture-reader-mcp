# Publish status

| Field | Value |
| --- | --- |
| Package | `@sylphx/architecture-reader-mcp` |
| Repo version | `0.2.0` |
| Registry state | **not_on_registry** |
| npm auth in this environment | `ENEEDAUTH` (cannot live-publish here) |

## Install paths

### npm (when published)

```bash
npm i -g @sylphx/architecture-reader-mcp
```

### Git (always available; product SSOT)

```bash
git clone https://github.com/SylphxAI/architecture-reader-mcp.git
cd architecture-reader-mcp
bun install
```

### Residual

Live `npm publish` for unpublished packages requires `@sylphx` automation token / 2FA on a trusted publisher machine. That is an **external credential blocker**, not a product design gap.

See also [BRAND_PUBLISH.md](./BRAND_PUBLISH.md) when present.
