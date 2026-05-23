# Contributing to Suibase

## Releasing a new `suibase-daemon` version

A daemon release is a coordinated change across three repos: this one
(`suibase`), [`chainmovers/sui-binaries`](https://github.com/chainmovers/sui-binaries)
(which publishes precompiled binaries for end users), and the user's local
suibase checkout. Get the order wrong and end users land between versions.

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

### Merge sequence for a version bump

The dev → main flow is the brittle part. Follow this order:

1. **Bump `rust/suibase/Cargo.toml` on `dev`** (e.g. `0.3.0` → `0.4.0`) and
   open a PR against `dev`. CI on `dev` builds the daemon from source.
2. **Wait for `sui-binaries` to publish the precompiled.**
   `.github/workflows/suibase-daemon-tests.yml` has a `trig` job that pushes
   `Cargo.toml` into `chainmovers/sui-binaries:triggers/suibase-daemon/`,
   which kicks off the binary build on that side. Confirm via the releases
   page of `chainmovers/sui-binaries` that a release tagged with the new
   version has appeared.
3. **Only then merge `dev` → `main`.**
   Once main is bumped, every existing user worldwide running
   `~/suibase/update` (or any command that triggers
   `start_suibase_daemon_as_needed`) pulls the new precompiled. If
   sui-binaries hasn't published yet, the local-source-version gate keeps
   them on the old binary and prints a warning — which is correct but
   noisy. Best to skip the noisy window entirely.

### What happens if I get it wrong

- **Merge to main before sui-binaries publishes**: end users on main will
  pull the bumped scripts via `~/suibase/update` but the install logic
  will refuse to download a binary newer than their source. They stay on
  the old binary with a warning. They're not broken, but they don't get
  the new daemon until sui-binaries catches up.
- **Bump version mid-dev without intending to release**: harmless. The
  `sui-binaries` side will see the trigger but the published tag is
  CI-suffixed (`_ci`), which the user-facing install filter excludes.
  End users see nothing.

### CI conventions

- Workflows in `.github/workflows/` trigger on `push: branches: [dev]`.
  Each workflow that needs the daemon's source-built version (e.g.
  `typescript-tests.yml`) explicitly runs `scripts/dev/update-daemon` on
  non-main runs so the binary under test matches the branch's source.
- On main, those workflows let `~/suibase/install` download the
  precompiled instead, which doubles as a smoke test of the download path.
