# Changelog

Do '~/suibase/update' to download and update suibase itself to latest.<br>

Do '<workdir_name> update' to only update to latest sui client from Mysten Labs for a specific network. Example: 'localnet update'

Only notable changes are documented here. See GitHub commits for all changes.

Suibase adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.10] TBD
### Fixed

- suibase-daemon more portable on linux (static link MUSL libc instead of dynamic link glibc)

## [0.1.9] 2025-05-03
### Fixed

- (#120) Fix for faucet failing to start (options and output of binaries were changed by Mysten Labs).

## [0.1.8] 2025-04-03
### Added

- (#118) New walrus and site-builder scripts (mwalrus/twalrus and msite/tsite).
         Binaries and configs are installed/updated on "mainnet update" or "testnet update".
         See https://suibase.io/walrus for more details.

## [0.1.7] 2024-10-17
### Added

- New built-in sui explorer (do "localnet status" to see URL). Code from https://github.com/kkomelin/sui-explorer
- New VSCode extension https://marketplace.visualstudio.com/items?itemName=suibase.suibase
- (#113) Reduce localnet storage (less checkpoints per secs)
- (#101) Eliminate rust dependencies (suibase-daemon precompiled for most platforms)

### Fixed

- (#112) fix for keytool generate command (.key were created in unexpected location)
- "lsui/dsui/tsui client faucet" commands now work.
- More robust handling of backend (suibase-daemon)
- Reduce localnet storage on regen (delete full_node_db).

### Changed

- For better stability, localnet uses Mysten Labs testnet branch (instead of devnet).

## [0.1.6] 2023-11-01

### Added
- (#68) Precompiled binaries for macOS, x86_64 Linux and Windows WSL2.

### Fixed
- (#83) Do sui binaries --locked cargo build for consistency.
- (#65) Fix support for 'sui client publish' and 'sui move' when path and/or install-dir are not specified.
- (#24) Fix help for faucet.
- Misc fix to support sui client >1.10.x for CLI new output format (tables).


## [0.1.5] 2023-08-28

### Added

- (#59,#52) Multi-Link RPC (proxy server) major feature
- Scripts: (#62) Can disable auto-generations of the 15 private keys.
- Scripts: (#62) Add more easily your own private keys to any workdir.
- Scripts: (#57) New "build" command (e.g. "testnet build -p sui-node").
- Transaction result options to cookbook
- Python transactions to cookbook code-snippets

### Fixed

- Scripts: (#71) Sui v1.7.0 keytool changes.
- Scripts: (#60) "localnet start" now works even when suibase.yaml is deleted.
- Scripts: (#43) Ignore http_proxy envs when trying to use the sui-faucet.
- Typos in Keypair cookbook code-snippets

### Changed

- Transaction python cookbook entries

## [0.1.4] 2023-05-26

### Added

- Rust/Python Suibase Helper (more info: https://suibase.io/helpers)
- Object cookbook
- MultiSig cookbook entries for Python

### Fixed

- Scripts: (#44) Fix log display issue related to Sui client v1.2.0
- Rust demo-app: Fix by increasing gas amount, plus filtering on package-id.

### Changed

- Git _organization_ name changed from sui-base to ChainMovers. May affect some URL and local repositories (e.g github.com/_chainmovers_/suibase.git instead of github.com/_sui-base_/suibase.git )
- Bumped pysui version
- Prefix each cookbook code subject with Facts section

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
