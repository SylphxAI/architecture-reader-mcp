# Contributing to repomap

repomap maps code locally for navigation, search, call paths and change impact.
Start with the [README](README.md) for the product and [AGENTS.md](AGENTS.md) for
its layout and contracts.

## Find the right place

- Search [existing issues](https://github.com/SylphxAI/repomap/issues) and pull
  requests before starting. The [good first issue list](https://github.com/SylphxAI/repomap/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22good%20first%20issue%22)
  contains tasks when suitable ones are available.
- Use [Discussions](https://github.com/SylphxAI/repomap/discussions) for usage
  questions and early ideas. For a bug, feature request or documentation correction, use the
  [issue forms](https://github.com/SylphxAI/repomap/issues/new/choose).
- Share a minimal public or synthetic repository, commands/tool arguments,
  version and expected file:line results. Redact private source, personal paths,
  tokens and database connection strings. Database examples need only a
  sanitized schema, not rows or credentials.
- Report vulnerabilities through the [security policy](https://github.com/SylphxAI/repomap/security/policy),
  not a public issue.

## Make a focused change

Fork the repository and create a branch from `main`. Small fixes can go straight
to a pull request; discuss new languages, public tool changes and larger work
first. Keep the change tied to one outcome, add a regression test for changed
behavior, and update the relevant docs.

Rust stable builds the engine and CLI. Bun 1.4.0 runs the UI and docs tooling.
From your checkout, the relevant checks are:

```bash
cargo test -p sylphx-repomap-core
cargo test --workspace
bun install --frozen-lockfile
bun scripts/check-version.ts
bun run docs:build
```

For UI changes, run `bun run build:ui` and include the rebuilt
`crates/repomap/assets/` bundle. New language support belongs in
`crates/repomap-core/src/lang.rs`, with a parser test. Public copy is generated:
follow the copy instructions in [AGENTS.md](AGENTS.md) rather than editing the
generated sections. Run benchmarks in CI, not on the shared desk.

## Open the pull request

Target `main`, link the issue if there is one, and describe the user-visible
result. List the checks you ran and any you could not run. CI covers the Rust
suite, CLI/MCP/launcher smoke tests, docs and applicable platform checks; merge
follows the repository's normal queue. Contributions use the repository's
[MIT license](LICENSE) and the organization [code of conduct](https://github.com/SylphxAI/.github/blob/main/.github/CODE_OF_CONDUCT.md).
