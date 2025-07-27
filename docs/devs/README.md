This README is *only* for developers working on the Suibase project.

Suibase users should instead check the [Online Docs](https://suibase.com/docs/).

Steps to release a new version of suibase-daemon
================================================
1. Bump version in `rust/suibase/Cargo.toml`
2. Push to 'dev' branch. Verify that the CI passes.
3. Verify that https://github.com/chainmovers/sui-binaries was triggered and that a new release was generated.
4. Run a ~/suibase/update to verify it works for users.


