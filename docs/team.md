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

Publish the map of any repository to a private link that only your GitHub org
can open, refreshed on every push to your default branch. Viewers don't need a
seat. Maps hold file paths and symbol names, never source code.

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
