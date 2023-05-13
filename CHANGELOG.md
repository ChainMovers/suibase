# Changelog

Do '~/suibase/update' to download and update suibase itself to latest.

('~/sui-base/update' for v0.1.2 and less)

Do '<workdir_name> update' when you instead want to update only a local Sui repo to the latest from Mysten Labs. Example: 'localnet update'

Only notable changes are documented here. See github commits for all changes.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unpublished]

### Added

### Fixed

### Changed
- Git *organization* name changed from sui-base to ChainMovers. May affect some URL and local repositories (e.g github.com/*chainmovers*/suibase.git instead of github.com/*sui-base*/suibase.git )
- Bumped pysui version
- Prefix each cookbook code subject with Facts section

### Removed

## [0.1.3] 2023-05-03

### Added

- Scripts: mainnet support. New 'msui' and 'mainnet' scripts.
- Python example of Programmable Transaction
- Language neutral cookbook guide introducing Programmable Transactions

### Fixed

- Display from `coinage` Python demo

### Changed

- Breaking changes: Renaming of project from sui-base to suibase. Affects paths, URL links, API.
  Change will facilitate multi-platform consistency by using a namespace without dash.
- Upgraded Python demos to use newest version of `pysui` 0.17.0
- Python demo3 (prgtxn.py) updated for changes in 0.17.0

### Removed

- Python demo common utility `low_level_utils.py` as SuiConfig now has `sui_base_config()` class method.

## [0.1.2] 2023-04-10

### Fixed

- Scripts: (#25) Fix for Sui 0.31 support (change to config.yaml)

## [0.1.1] 2023-04-01

### Fixed

- Scripts: (#23) asui was not working when cargobin was the active workdir.

## [0.1.0] 2023-03-31

### Added

- Scripts: localnet and faucet process start/stop/status
- Scripts: localnet/devnet/testnet, lsui/dsui/tsui shortcuts
- Scripts: asui for user selectable active workdir (look for 'set-active' option).
- Scripts: csui for "cargobin" workdir created when ./cargo/bin/sui exists.
- Rust: demo-app
- Python: demo app(s) added: `sysinfo` and `coinage`

### Changed

- Python requirements.txt updated to use `pysui 0.15.0`
- Added sysinfo code for Sui 0.29.1 types
