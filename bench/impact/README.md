# Impact benchmark

How well does `repomap impact <file>` name the files a change reaches? 30 cases
from the real history of five public repositories, scored at file level.

| Repository | Language | Cases |
|---|---|---:|
| BurntSushi/ripgrep | Rust | 7 |
| sharkdp/fd | Rust | 5 |
| honojs/hono | TypeScript | 6 |
| pallets/flask | Python | 6 |
| spf13/cobra | Go | 6 |

Each repository is pinned in `gold.json` (`pin`); every case is a commit
reachable from its pin.

## How the gold set was derived

`mine.py` walks the first-parent history back from the pin and keeps a commit
when it has 2 to 12 changed files, at least one test file and at least two
non-test source files. Then, per commit, it takes one **seed** file and fills
the gold lists from the commit itself, not from repomap or any index:

- **callers**: other changed non-test source files whose *added lines* use, as a
  whole word, an identifier the seed defines (at least 5 characters, defined in
  only one changed file, not a generic name). These are call sites the commit
  had to change.
- **tests**: changed test files whose added lines use such an identifier or the
  seed's module name.
- **callees**: the reverse direction (files the seed's added lines use that the
  commit also changed). Recorded, not scored: `impact` reports what depends on
  a change, not what it depends on. They are kept for a trace benchmark.

Cases are taken in history order from the pin (the most recent that qualify,
at most 8 per repository, a few dropped by hand because the seed was a
benchmark script or a packaging manifest). No case was chosen or dropped by
looking at repomap output. Rerun `mine.py` to regenerate candidates:

```bash
python3 bench/impact/mine.py <repo-dir> <rust|typescript|python|go> <pin> --max 8 --scan 500
```

## Running it

```bash
python3 bench/impact/run.py target/release/repomap /tmp/impact-corpus out.json
python3 bench/impact/run.py --summary out.json     # Markdown table
```

For each case the runner checks out the commit and runs
`repomap impact <seed> --json` on the tree as the commit left it, with
embeddings off. Predicted callers are the answer's `files` and `importers`
minus the seed and minus tests; predicted tests are its `tests`. Precision and
recall are per case, averaged per language, then over languages. A case with an
empty gold list is left out of that axis; an empty answer scores precision 0.

`floor.json` (once a baseline exists) holds the lowest acceptable macro score
per metric; the workflow fails below it. Raise it when the tool improves, never
lower it to make a run pass.

## Weaknesses of this set

- **Precision is a lower bound.** The gold only holds files that one commit
  touched. A true caller the commit did not need to change counts as a false
  positive, so real precision is higher and nobody can say by how much.
- **Recall is the more trustworthy number**, but it only covers dependents a
  developer happened to edit together with the seed.
- **Textual rule, not a compiler.** A caller is matched by a shared identifier,
  so a same-named but unrelated symbol can enter the gold, and a call through
  an alias or re-export can be missed.
- **Tests.** Rust unit tests live inside source files and never show up as test
  files; Go, Python and TypeScript tests do.
- **Small.** 30 cases; one case moves a language average by 3 to 20 points.
  Rust has 12 cases, the others 6. The set is ours, not independent.
- **Seed is a file.** `impact` is also used on symbols and on `--changed` diffs;
  this set does not measure those.
