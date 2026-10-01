# Release publishing

The existing `scripts/set-version.ts` owns the version for Cargo, npm and the
MCP manifest. The crates workflow does not set a version or create another
release. Native binaries, npm, MCP and container publication remain in the
shared release job in `.github/workflows/release.yml`.

## Registry package verification

The same workflow verifies registry packages on pull requests, merge groups
and main. `python3 scripts/release_crates.py check` reads Cargo metadata,
orders publishable workspace crates by their dependencies, and copies tracked
sources into a temporary directory. Only those temporary manifests gain exact
registry version constraints on internal path dependencies. The tracked
manifests, lockfile and version setter are unchanged.

Cargo packages both crates together for `crates-io`, verifies builds from the
packages, and writes the `.crate` archives to the `registry-packages` artifact.
This covers the CLI's committed HTML, JavaScript and CSS assets as well as its
core dependency. Rust packaging/build verification runs in CI, not on the desk.
Pure Python guard tests can run locally with
`python3 scripts/test_release_crates.py`.

## One-time owner setup before publication

Publication is off unless repository variable `CRATES_IO_PUBLISH_ENABLED` is
exactly `true`. Do not enable it before the release owner authorizes publication
and confirms registry ownership and both trusted-publisher records. Enabling
it permits the next main push or manual release dispatch to publish the current
workspace version, including a version absent from crates.io even when npm has
already published it.

Required owner chores:

- Confirm authorized control of crates.io packages `repomap-core` and `repomap`.
  Both already exist at version `0.1.0`; the public owner logins observed were
  `zeon256` and `syf20020816`, respectively. Public ownership listings do not
  establish company control. Resolve this before granting publication; do not
  assume a name is available or rename it silently.
- Register a GitHub Actions trusted publisher for each package with GitHub
  owner `SylphxAI`, repository `repomap`, workflow filename `release.yml`, and
  environment `crates-io`. These are the required configuration values, not a
  claim that records already exist.
- Configure the GitHub environment `crates-io` consistently with those records,
  restrict deployment to main, and then enable the repository variable only
  after authorization. No long-lived registry secret is required.

The publish job requests a short-lived token using the official crates.io
OIDC action. Its post hook revokes the token. The script publishes
`repomap-core` before `repomap`, waits for each exact version to appear in the
sparse index, and skips an already-published exact version on a later dispatch.
A yanked exact version or registry error fails rather than being treated as
missing. If a publish is interrupted, dispatch the existing workflow again;
it resumes with the missing packages and never overwrites a published version.

The default CLI model/cache behavior and installation copy are independent of
this workflow and are not changed by registry packaging.
