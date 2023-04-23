# Changelog

Do '~/sui-base/upgrade' to download and upgrade sui-base itself to latest.

Do '<workdir name> update' when you instead only want to download and re-build a workdir local Sui repo with the latest from Mysten Labs. Example: 'localnet update'

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unpublished]

### Added

- Python example of Programmable Transaction

### Fixed

- Display from `coinage` Python demo

### Changed

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
