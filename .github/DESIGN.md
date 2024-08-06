## Documentation Workflows

All documentation, including suibase.io site, are done with Markdown files (.md).

Changes can be done on either main or dev with push or PR.

Only the docs on main are published/visible.

Changes are detected by actions/diffs and triggers these workflows:
(1) deploy-docs.yml
build/publish the suibase.io site (hosted by GitHub pages).
The generated site is in the gh-pages branch.

(2) trig-rust-api-docs.yml
push a file to ChainMovers/suibase-api-docs repo to remotely trig the rustdoc build/publish
The resulting docs are hosted by GitHub pages at https://chainmovers.github.io/suibase-api-docs/suibase

## Source code changes workflows
Push/PR only on the **dev branch**.

The dev branch have various quick tests with multiple OS. Intended to catch errors early after every change. These tests (and more) are also included in the daily extensive tests.

actions/diffs detect changes and triggers these workflows:

(1) scripts-tests.yml
Done on bash script changes

(2) suibase-daemon-tests.yml
Tests on code changes related to suibase-daemon. Upon success, will send a file to ChainMovers/suibase-binaries repo to trig potentially further build/publish a new version.
This test force the building of the suibase-daemon (does not use pre-build binaries).

(3) rust-tests.yml
Done on code changes that might affect the rust API, demos and other rust projects (excluding rust suibase-daemon backend)


## Merge Check Workflows
(1) main-merge-check.yml
Does a few quick sanity checks (not involving lengthy building), for VSCode/daemon/scripts version compatibility.
In particular, verifies that suibase-daemon binaries were built and released successfully prior to merging code requiring a new
version on main.

## Other QA Workflows
(1) main-nithgly-tests.yml/dev-nightly-tests.yml
**Extensive** branch tests done once a day, even when no Suibase changes (in case a dependency breaks something).
Results are published as passed/failed "badges" on GitHub.

(2) release-check.yml
Done on any script/code changes on main.

Simulate someone updating to latest Suibase version. Verifies that binaries can properly be downloaded/installed.

The goal is not to test extensively all features, but rather just validation that the continuous integration itself is working as expected (e.g. if something depend on a backend version, make sure that version is indeed still published).


## About run-all.sh
Many GitHub Actions simply call "scripts/tests/run-all.sh".

A subset of tests can be selected with a combination of:
  --scripts-tests
  --suibase-daemon-tests
  --rust-tests
  --release-tests
  --main-merge-check
  --dev-push-check

**Extensive** tests happen when there are no skip parameters.







