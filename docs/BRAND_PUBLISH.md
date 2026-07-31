# Spine — brand npm publish (expand–contract)

**Publish authority:** this repository only.

| Field | Value |
| --- | --- |
| Brand | **Spine** |
| Canonical brand npm id | `@sylphx/spine` |
| Transitional npm id | `@sylphx/architecture-reader-mcp` |
| Marketplace title | Spine (`server.json`) |

## Policy (expand → contract)

1. **One codebase / one version** — never two products.
2. **Expand:** dual-publish `@sylphx/architecture-reader-mcp@X.Y.Z` and `@sylphx/spine@X.Y.Z` (same artifacts).
3. **Contract (later):** `npm deprecate` transitional toward brand; keep bins as long as cheap.
4. Workflow: `.github/workflows/publish-brand-alias.yml` (org `NPM_TOKEN`).

## User install

```bash
# preferred
npm i -g @sylphx/spine
# transitional still valid during expand
npm i -g @sylphx/architecture-reader-mcp
```

## Authority

No central Instruments monorepo. Brand alias ships only from this product repo.
