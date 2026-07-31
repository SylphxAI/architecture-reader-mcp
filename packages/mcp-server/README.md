# @sylphx/architecture-reader-mcp (Spine)

Local architecture engine for agents — **SDK · CLI · MCP**.

Product repository (SSOT): https://github.com/SylphxAI/architecture-reader-mcp

## Install

```bash
npm i -g @sylphx/architecture-reader-mcp
# or from git SSOT
git clone https://github.com/SylphxAI/architecture-reader-mcp.git
cd architecture-reader-mcp && bun install && bun run build:rust
```

## Surfaces

| Surface | Entry |
| --- | --- |
| CLI | `spine` / `architecture-reader-mcp` |
| MCP | stdio via package bin (Rust rmcp when built) |
| SDK | `@sylphx/architecture-reader-mcp/sdk` |

## Native binary

The MCP/CLI engines are **Rust**. After install from npm, set:

- `ARCHITECTURE_READER_MCP_RUST_BIN` — path to `architecture-reader-mcp-server`
- `ARCHITECTURE_READER_CLI_BIN` — path to `architecture-reader-cli`

Or install from git and run `bun run build:rust` so `bin/native/*` is populated.

Doctor: `spine doctor` (when CLI bin available) or use package doctor helper.

## Independence

This package is one product. No multi-product monorepo; no central Instruments hub.
