[![Discord chat](https://img.shields.io/discord/1038616996062953554.svg?logo=discord&style=flat-square)](https://discord.gg/Erb6SwsVbH)

[![Active Development](https://img.shields.io/badge/Maintenance%20Level-Actively%20Developed-brightgreen.svg)](https://gist.github.com/cheerfulstoic/d107229326a01ff0f333a1d3476e068d)

[![release](https://github.com/ChainMovers/suibase/actions/workflows/main-nightly-tests.yml/badge.svg)](https://github.com/ChainMovers/suibase/actions/workflows/main-nightly-tests.yml)

[![nightly](https://github.com/ChainMovers/suibase/actions/workflows/dev-nightly-tests.yml/badge.svg)](https://github.com/ChainMovers/suibase/actions/workflows/dev-nightly-tests.yml)

Streamlines development and testing of your Sui network apps.

Suibase features:

  - Easy start/stop/status of localnet and faucet services.

  - Very fast installation and upgrade of Sui clients (no compilation needed[^1]).

  - Built-in localnet sui explorer

  - RPC failover and load-balancing among free public RPC servers.

  - Rust and Python Helper for test automation.


All features work out-of-the-box, and can progressively be integrated and customized in your workflow.

Can safely co-exist with other official Sui installations.

More info: [https://suibase.io](https://suibase.io/)

[^1]: Uses official published Mysten Labs precompiled binaries. Not all platforms supported, in which case Suibase automatically revert to build from source.
