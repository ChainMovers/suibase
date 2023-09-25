[![Discord chat](https://img.shields.io/discord/964205366541963294.svg?logo=discord&style=flat-square)](https://discord.gg/Erb6SwsVbH)

[![Active Development](https://img.shields.io/badge/Maintenance%20Level-Actively%20Developed-brightgreen.svg)](https://gist.github.com/cheerfulstoic/d107229326a01ff0f333a1d3476e068d)
[![nightly tests](https://github.com/ChainMovers/suibase/actions/workflows/nightly-tests.yaml/badge.svg)](https://github.com/ChainMovers/suibase/actions/workflows/nightly-tests.yaml)

Suibase provides a development environment for the Sui network

It complements your existing sui installation with features such as:
  - Easy start/stop/status of localnet and faucet services.
  
  - Very fast installation and upgrade of Sui clients (no compilation needed[^1]).

  - RPC failover and load-balancing among free public RPC servers.
    
  - Rust and Python Helper for test automation.

All features are design to work out-of-the-box, and can be progressively introduced in your workflow and configured to your need.

More info: [https://suibase.io](https://suibase.io/)

-------------

[^1]: Uses official published MystenLabs precompiled binaries. Not all platforms supported, in which case Suibase automatically revert to build from source.
