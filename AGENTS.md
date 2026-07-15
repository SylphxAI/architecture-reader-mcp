# architecture-reader-mcp — local agent notes only

Doctrine and fleet delivery law live in the **host always-on constitution**
(`~/.grok/AGENTS.md` / Doctrine template). This file must **not** restate,
weaken, or fork that law (including PR-vs-direct-trunk delivery).

Local truth: `PROJECT.md`, `.doctrine/project.json` when present.

## Boundary hazards

- Never commit secrets, tokens, `.env` files, or credentials.

## Local commands

- `PROJECT.md` is the human-readable project boundary.
- `.doctrine/project.json` is the Sylphx governance adapter.
- `project.manifest.json` is the vendor-neutral machine manifest.
- `docs/architecture.md` is the durable architecture overview.
- `docs/specs/` owns product and protocol specifications.
- `docs/adr/` owns durable architectural decisions.
- Prefer the **narrowest** affected check before full workspace runs.
- Report layers honestly: local diff · trunk FF · deploy · prod proof (do not collapse).

## Validation notes

- Prefer the **narrowest** affected check before full workspace runs.
- Report layers honestly: local diff · trunk FF · deploy · prod proof (do not collapse).
