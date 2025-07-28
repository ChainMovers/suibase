This README is *only* for developers working on the Suibase project.

Suibase users should instead check the [Online Docs](https://suibase.com/docs/).

Steps to release a new version of suibase-daemon
================================================
1. Bump version in `rust/suibase/Cargo.toml`
2. Run scripts/dev/update-daemon (will re-build and update Cargo.lock).
3. Push to 'dev' branch. Verify CI passes on dev.
4. Verify that https://github.com/chainmovers/sui-binaries publish a new release.
5. Run scripts/dev/merge. Verify CI passes on main.
6. Run a ~/suibase/update on another setup to verify it works for users.


