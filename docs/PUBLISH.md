# Publish status — Spine

| Field | Value |
| --- | --- |
| Package | `@sylphx/architecture-reader-mcp` |
| Repo version | `0.2.0` |
| Registry state | **not on npm (404)** |
| npm auth in this environment | `ENEEDAUTH` |

## Honest install authority today

**Git clone + `bun run build:rust` is the supported runtime install path.**

The npm package (when published) currently ships SDK/tool schemas + bin *launchers* that resolve a local Rust engine via:

- `ARCHITECTURE_READER_MCP_RUST_BIN` / `ARCHITECTURE_READER_CLI_BIN`, or
- repo `bin/native/*` / `target/{release,debug}/*` after build

It does **not** yet ship multi-platform optional native binaries inside the npm tarball. Do not market `npm i -g @sylphx/architecture-reader-mcp` as a complete zero-build runtime until optionalDeps natives land.

## Git install (current SSOT)

```bash
git clone https://github.com/SylphxAI/architecture-reader-mcp.git
cd architecture-reader-mcp
bun install
bun run build:rust
./bin/spine doctor
```
