# CI / Workflows

## Release pipeline

```
dev ──merge script──► pre-staging ──cron auto──► staging ──auto-PR──► main
```

| Branch | Pushes from | Tests | Failure means |
| --- | --- | --- | --- |
| `dev` | direct or PR | full suite, source-built daemon (`actions/diffs` gates each leg) | real bug |
| `pre-staging` | `~/suibase/scripts/dev/merge` only | **none** (intentional) | not meaningful — branch is never alarm-red |
| `staging` | `staging-promote.yml` cron (auto-merge PR) | release-tests style against the published precompiled | real bug in binary or scripts |
| `main` | `staging.yml` (auto-merge PR) | defense-in-depth smoke test | should never happen — investigate the pipeline |

**main / staging / pre-staging are branch-protected**: no direct push, PR head_ref must come from the immediately-upstream branch (enforced by `pr-gate.yml`).

### Workflow files (per branch)

| File | Fires on | Does |
| --- | --- | --- |
| `.github/workflows/scripts-tests.yml` | push to dev | bash script tests via `run-all.sh --scripts-tests` |
| `.github/workflows/suibase-daemon-tests.yml` | push to dev | daemon source-build tests via `run-all.sh --suibase-daemon-tests` |
| `.github/workflows/rust-tests.yml` | push to dev | rust API/demo tests via `run-all.sh --rust-tests` |
| `.github/workflows/typescript-tests.yml` | push to dev | helper + integration tests |
| `.github/workflows/lint.yml` | any push / PR | lint |
| `.github/workflows/pre-staging.yml` | push to pre-staging | push `Cargo.toml` to `chainmovers/sui-binaries` IF the matching daemon release tag isn't published yet |
| `.github/workflows/staging-promote.yml` | cron every 15 min on **main** | poll sui-binaries; once `suibase-daemon-v<ver>` is published with assets, open auto-merge PR pre-staging → staging |
| `.github/workflows/staging.yml` | push to staging | download published precompiled, validate, open auto-merge PR staging → main |
| `.github/workflows/release-tests.yml` | push to main | defense-in-depth post-merge smoke test |
| `.github/workflows/main-nightly-tests.yml` | cron | extensive nightly regressions on main |
| `.github/workflows/dev-nightly-tests.yml` | cron | extensive nightly regressions on dev |
| `.github/workflows/pr-gate.yml` | PR to main / staging / pre-staging | required check: head_ref must be the immediately-upstream branch |

### Cross-repo coupling

`chainmovers/sui-binaries` publishes the daemon precompiled. `pre-staging.yml` writes `triggers/suibase-daemon/Cargo.toml` over there (auth: `SUI_BINARIES_TOKEN` org secret); that repo's `Build Suibase Daemon` workflow then builds + publishes `suibase-daemon-v<ver>`.

Daemon currently builds for `ubuntu-x86_64` and `macos-arm64` only. **macOS x86_64 (Intel) is not supported** — `sb_app_install` fails with a clear message if attempted.

## How to ship a change

```bash
# from dev with whatever commits you want to release
~/suibase/scripts/dev/merge
```

That's it. The script merges dev → pre-staging; the workflows above carry it the rest of the way (typical wall-clock: 5-30 min depending on whether sui-binaries needs to build a new binary).

## How a developer reads CI status

- **Red on `dev`**: real test failure. Look at the failing workflow log.
- **Red on `pre-staging`**: shouldn't happen by design (no tests). If it does (e.g. SUI_BINARIES_TOKEN expired), the trigger to sui-binaries didn't fire — fix the secret/credentials and re-trigger.
- **Red on `staging`**: precompiled is broken OR scripts don't handle it. Fix on dev → `merge` again → pipeline replaces.
- **Red on `main`**: pipeline invariant violated. Should be impossible if `staging` was green — investigate.

## Other QA workflows

- `release-tests.yml`: simulates a user updating to latest. Verifies the published precompiled is downloadable and matches main's Cargo.toml. Already green by the time it runs (staging.yml validated the same thing pre-merge).
- `*-nightly-tests.yml`: extensive periodic regression check. Catches dependency drift even when no Suibase change happens.

## `run-all.sh` selectors

Many workflows just call `scripts/tests/run-all.sh` with one of:
- `--scripts-tests`
- `--suibase-daemon-tests`
- `--rust-tests`
- `--release-tests`
- `--main-merge-check`
- `--dev-push-check`

No selector = full extensive suite.

## See also

- `CONTRIBUTING.md` — narrative for the same pipeline, plus version-source policy and branch-aware install routing.
- `CLAUDE.md` — short AI-agent guidance pointing here.
