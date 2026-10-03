<script setup>
import team from './.vitepress/team.json'
</script>

# repomap Team

repomap maps your code for your agent, free, forever. Team puts that map where
your team decides: on every pull request, across every repository, and in one
shared link.

## Blast radius on every pull request

Install repomap Review on your GitHub org. Every pull request gets one check
and one comment, updated on each push: the files and modules it reaches, the
callers it can break, its risk, the tests to run, and the code owners it
touches. No CI changes, no tokens in your workflows, and results in under a
minute. It reads the code to analyse it and keeps none of it.

Free on public repositories.

## One map across repositories

List your services in `repomap.workspace.toml` and repomap joins them into one
graph. `impact` and `trace` follow your shared packages from one repository into
the next, so your agent sees the service a change really breaks. It runs on
your machine.

Every tool takes the whole workspace:

- `map` gives one map per repository and lists which repository imports which
  package from which.
- `search` ranks hits across all repositories, and says which other repositories
  use the file a hit is in.
- `context` finds a file or symbol in every repository that has it and lists the
  files in other repositories that import it. Write `api:src/auth.ts` to ask one
  repository only.
- `trace` and `impact` follow imports across repository borders.
- `db` answers once per repository and never merges them: separate repositories
  are usually separate databases, so a merged schema would be a guess. Without a
  URL you get each repository's own schema and the code that queries it. With
  `url` or `url_env` you get the one live database shown to each repository, so
  you can see which repository queries each table.

```toml
# repomap.workspace.toml
roots = ["../api", "../web", "../shared"]
```

The workspace file is found in the current repository or a parent folder. The
same list also works as a `workspace` argument on every tool (not listed in the
tool schemas, to keep them small), and on the CLI as `--workspace ../api,../web`. Without a Team licence the call answers for the
current repository, says that the other repositories were not joined, and
carries a `pro_required` field your agent can relay.

## Private shared maps

Every repository in your Team installation gets a map at one link, refreshed on
every push to the default branch:

```text
https://review.repomap.sylphx.com/m/acme/api
```

Only people who can read the repository on GitHub can open it. Viewers don't
need a seat. Maps hold file paths, symbol names and the call graph, never source
code, comments or string literals. The server checks your GitHub access on every
read, so someone removed from the repository loses the link within minutes.

- **In a browser** the link shows the graph viewer, after you sign in with GitHub.
- **For your agent**, run `repomap login` once (GitHub's device flow, nothing to
  paste; for CI set `REPOMAP_TOKEN` to a session token). Then pass the link as
  `root`, as `repomap map --root <link>`, or as an entry in `roots` or
  `workspace`. All six tools stay; they answer from the map:
  - `map`, `trace` and `impact` with a `target` work as on a local clone.
  - `context` gives the outline, callers, callees and tests, and a GitHub
    permalink in place of the code, so your agent reads the source with its own
    GitHub access.
  - `search` matches symbol names and file paths only. It holds no source text.
  - `impact --changed` and `db` are not available on a shared map: it has no
    working tree and no schema files.
  - A link next to your local repository in a workspace gives the
    cross-repository graph, with no seat needed: your org paid for the map and
    GitHub says you may read it.

```toml
# repomap.workspace.toml
roots = ["../web", "https://review.repomap.sylphx.com/m/acme/api"]
```

`repomap export --shared map.json --out map.html` writes the same source-free
map locally, so you can check what a shared map holds.

## Support

Email support at [hi@sylphx.com](mailto:hi@sylphx.com), with a reply within two business days.

## What stays free

Everything free in repomap stays free, under MIT, including `repomap impact`
in your own CI: `map` (with `--tokens`), `search`, `context`, `trace`, `impact`
(with `--changed`), `db`, `serve`, `export`, `score` and the score Action, the
Claude Code hook, `setup`, the six MCP tools, and `repomap licence status`.
Team sells only what is new for teams, and a capability never moves from free
to Team.

## Price

**US$180 per developer per year**, paid upfront. Tax is added where it
applies.

- **A seat** is a developer who opens a pull request that repomap Review
  reviews on a private repository, counted over a rolling 30-day window.
- **Packs** of 5, 10, 25 and 50 seats.
- **Public repositories are free**, with no seat count.
- **One Team licence** covers the pull request review, the multi-repository
  graph and private shared maps, plus support.

Blast-radius analysis is in CodeRabbit's Advanced plan at US$72 a developer a
month, and Greptile charges US$30 a seat a month (coderabbit.ai/pricing,
greptile.com/pricing, read 2 October 2026). repomap Team is deterministic,
instant, and a fraction of the price.

<div v-if="team.checkoutUrl" class="team-buy">

<p><a :href="team.checkoutUrl">Buy repomap Team</a></p>

By buying you ask us to supply repomap Team to you at once, and you
acknowledge that once it is supplied you lose your right to cancel.

</div>

## Activate

Already bought? Run `repomap licence activate <token>`, and paste the same
token on the repomap Review setup page. `repomap licence status` shows what is
active.

`repomap licence buy` opens the Team page; in-terminal purchase turns on when checkout is live.
