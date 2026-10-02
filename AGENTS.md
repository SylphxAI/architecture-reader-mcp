# repomap

repomap gives AI agents a map of a codebase (code graph, search, call paths,
change impact) as one local Rust binary that needs no API key. The goal is
answers that are correct, fast, and cost few tokens. Docs:
https://repomap.sylphx.com. Formerly Spine, Locus and CodeRAG; those
npm packages are thin aliases.

## Layout

- `crates/repomap-core`: parsing, graph, ranking and the queries
- `crates/repomap`: the binary (CLI, MCP stdio server, `serve`, `export`, `setup`)
- `ui/`: graph UI source; `bun run build:ui` writes `crates/repomap/assets/`,
  which is committed so cargo builds need no JS toolchain
- `packages/`: npm launcher, native binaries, aliases
- `docs/`: VitePress site; `bench/` and `scripts/`: benchmarks and tooling
- `brand/`: the brand home; `brand/README.md` explains the pinned shared generator ([usage](brand/README.md))

## Rules and their reasons

- The MCP surface stays six tools (`map`, `search`, `context`, `trace`,
  `impact`, `db`): agents pay context for every tool definition. Legacy names
  route in `crates/repomap/src/tools.rs::canonical`.
- Public copy has one source, `brand.json` plus `repomap tools`; edit it and run
  `bun scripts/copy.ts --write` rather than the generated README, docs and
  manifest fields, because CI diffs them.
- Set versions with `bun scripts/set-version.ts X.Y.Z && cargo update -w`; the
  manifests must agree. Merging a new version publishes through `release.yml`.
- New language: a grammar crate plus a query in
  `crates/repomap-core/src/lang.rs` (capture conventions at the top), and a case
  in the `parse.rs` tests.
- Live database connections stay read-only and never store or print the
  connection string: users point repomap at production schemas.

## How a result is judged

- `cargo test -p sylphx-repomap-core`, then `cargo test --workspace`; CI runs the same
  plus the smoke test (CLI, export, MCP over stdio, hook, npm launcher) and
  `bun run docs:build`.
- Benchmarks run in CI (`bench.yml`, `bench-localization.yml`), not locally;
  search quality is NDCG@10 and localization is Acc@k, documented in
  `docs/benchmarks.md`. A change to ranking must not lower them.
