[![Active Development](https://img.shields.io/badge/Maintenance%20Level-Actively%20Developed-brightgreen.svg)](https://gist.github.com/cheerfulstoic/d107229326a01ff0f333a1d3476e068d)

Suibase provides a development environment for the Sui network

Complements your existing sui installation with scripts and features such as:
  - Easy start/stop/status of localnet and faucet services.
  
  - Very fast installation and upgrade of Sui clients (no compilation needed*).

  - RPC failover and load-balancing among free public RPC servers.
    
  - Rust and Python Helper for automating your testing and publication.

All features are design to work out-of-the-box, and can be progressively introduced in your workflow and be configured to fit your need.

Visit [https://suibase.io](https://suibase.io/) for all the details.

-------------

(*) Uses official MystenLabs precompiled binaries published with every release. Not all platforms supported, in which
    case Suibase automatically revert to build from source.
