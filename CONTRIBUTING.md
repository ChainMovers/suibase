# Contributing to Suibase

## Releasing a new `suibase-daemon` version

A daemon release is a coordinated change across three repos: this one
(`suibase`), [`chainmovers/sui-binaries`](https://github.com/chainmovers/sui-binaries)
(which publishes precompiled binaries for end users), and the user's local
suibase checkout.

The release pipeline below makes the coordination automatic. **You should
not need to time anything manually** — running `~/suibase/scripts/dev/merge`
on `dev` triggers everything else.

### The branches and what they mean

```
dev ──merge script──► pre-staging ──cron auto──► staging ──auto-PR──► main
```

| Branch | Role | Tests run here? | Failures mean |
| --- | --- | --- | --- |
| `dev` | Working branch. Daemon is built from source, tests use that build. | Yes (full suite, fresh source build) | Real bug — fix it. |
| `pre-staging` | "We've committed to a release. Trigger the binary build." | **No tests.** | Nothing — this branch is never red. |
| `staging` | "The precompiled binary is published. Validate it." | Yes (against the precompiled, like main but pre-merge) | Real bug — either the binary or our scripts can't handle it. |
| `main` | Released. End users see this. | Defense-in-depth smoke test only (always green by the time it runs). | Should never happen — investigate the pipeline. |

### The pipeline workflows

| Workflow | Fires on | What it does |
| --- | --- | --- |
| `pre-staging.yml` | push to `pre-staging` | If `chainmovers/sui-binaries` doesn't yet have a release tag for the daemon version in this branch's `Cargo.toml`, push that `Cargo.toml` into `chainmovers/sui-binaries:triggers/suibase-daemon/` — which is wired on that side to build + publish the binary. Always exits green. |
| `staging-promote.yml` | cron, every 15 min | Polls `chainmovers/sui-binaries`. When the matching release tag exists and `pre-staging` is ahead of `staging`, opens an auto-merge PR `pre-staging → staging`. Silent when there's nothing to promote (no alarm). |
| `staging.yml` | push to `staging` | Downloads the just-published precompiled, installs it, runs the release-tests sanity suite against it. On green, opens an auto-merge PR `staging → main`. Failures here are real bugs. |
| `release-tests.yml` | push to `main` | Defense-in-depth smoke test. By the time main updates, staging already validated everything, so this is expected to be green every time. |

### DX — what you actually do

For day-to-day development on `dev`, nothing changes. CI on `dev` builds
the daemon from source so every change validates against the branch's own
code (see [Branch-aware install routing](#branch-aware-install-routing)
below).

When you want to cut a release:

```bash
# from dev, with whatever commits you want to release
~/suibase/scripts/dev/merge
```

That's the whole DX. The script:

1. Verifies `dev` is clean and up-to-date with the remote.
2. Calls `~/suibase/scripts/dev/sync` to pull back anything that's
   accumulated on `main`/`pre-staging`/`staging` (normally a no-op).
3. Fast-forwards `pre-staging` to `dev`'s HEAD and pushes it.

From there the workflows take over. Within ~15-30 minutes (assuming the
sui-binaries build is healthy), the change is on `main`. You'll see PR
notifications fly past — those are the auto-promotion PRs and you don't
need to do anything with them.

### When the cron is "stuck"

If `staging-promote.yml` keeps running and never opens a PR, the
`chainmovers/sui-binaries` build is the bottleneck. Check that repo's
[Actions tab](https://github.com/chainmovers/sui-binaries/actions) — if
the "Build Suibase Daemon" workflow there is failing or stuck, fix it
there. The cron will pick up the publish on its next tick (no need to
touch this repo).

### When `staging.yml` fails

That's a real signal. Either:

- The published precompiled is broken (different from what the source
  builds — investigate `chainmovers/sui-binaries`).
- Our scripts can't handle the new precompiled (this repo's bug — fix on
  `dev` and run `merge` again, which will reset `pre-staging`).

### No hotfix path to main

By design. Branch protection on `main` requires the PR's head ref to be
`staging`, and the staging workflows must have passed. Emergencies route
through the same pipeline. The pipeline is fast enough (~15-30 min for a
binary-bumping change, <5 min for a no-binary change) that this is rarely
a real constraint.

### Version sources

Three places track "what version is `suibase-daemon`":

| Where | Authority |
| --- | --- |
| `rust/suibase/Cargo.toml` (this repo) | What the **branch under test** describes |
| `workdirs/common/bin/suibase-daemon-version.yaml` | What is **actually running** on a user's machine |
| Latest release tag on `chainmovers/sui-binaries` | What is **available to download** |

The user-facing invariant the install/upgrade logic enforces is:

```
installed_binary_version <= local_source_version
```

That is, a user's binary can lag their scripts (safe — old binary speaks an
older protocol), but it never gets ahead of their scripts. The
`sb_app_install` gate in `scripts/common/__apps.sh` refuses to download a
precompiled newer than the user's source.

### Branch-aware install routing

`sb_app_install` (in `scripts/common/__apps.sh`) routes `src_type=suibase`
installs differently depending on the local branch:

- On `main`: download the published precompiled binary from
  `chainmovers/sui-binaries`. End users land here.
- On `dev` / any feature branch: **build from source** (`update-daemon`
  semantics). The released precompiled lags the dev source by design (the
  release tag is created via the pipeline above), so the only correct
  artifact for `dev` is whatever `cargo build` produces.

This is why CI on dev branches just calls `install` and `start-daemon`
without an explicit build step — the routing handles it.

### CI conventions

- Per-branch workflow trigger:
  - `dev` pushes fire `suibase-daemon-tests.yml`, `rust-tests.yml`,
    `scripts-tests.yml`, `typescript-tests.yml`, `lint.yml`.
  - `pre-staging` pushes fire `pre-staging.yml` only (no tests).
  - `staging` pushes fire `staging.yml` only.
  - `main` pushes fire `release-tests.yml` and `main-nightly-tests.yml`.
- Every workflow that drives the daemon goes through the install +
  start-daemon flow, which automatically picks source-vs-precompiled per
  the branch-aware routing.
