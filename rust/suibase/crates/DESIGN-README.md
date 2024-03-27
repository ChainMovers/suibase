**Crates**
common
======
Code re-useable for most other crates.
Dependencies are minimized to avoid long build. In particular no dependencies on:
  - Mysten Labs Sui crates
  - Suibase helpers

common-sui
==========
Add more to "common", but with dependencies on Sui crates allowed.

dtp-core, dtp-sdk
=================
DTP Rust SDK. The dtp-core is most of the implementation. The dtp-sdk is a lightweight facade that defines the public API.

dtp-daemon
==========
The DTP Services daemon. A relatively easy-to-use bridge between web2 apps and web3 transport. Its behavior is mostly defined by each workdir/suibase.yaml. See online documentation.

suibase-daemon
==============
A behind the scene daemon that implements some Suibase features, particularly the JSON-RPC proxy server.

**Some Tech choices**
All daemon uses:
  - Tokio thread and async Mutex/RwLock.
  - Generic error handling with 'anyhow'.
  - Mostly Arc<RwLock> globals for multi-threading safe data sharing.
  - Most threads supports auto-restart on panic. This is implemented with 
       https://docs.rs/tokio-graceful-shutdown/latest/tokio_graceful_shutdown/
    
**State Coordination**
Because thread can be restarted at any time, we need to be careful about state coordination.

State change are mostly reactive (inform all consumer of the change with messages), but an additional layer of periodic "audit" provides a safety net for "eventual consistency".

All threads are design to handle 3 type of messages:

  - EVENT_AUDIT: A fast consistency check. Read-only access to shared variables for performance reason. Should emit an EVENT_UPDATE to self when detecting the need to mutate a shared variable (e.g. globals).
  
  - EVENT_UPDATE: Similar to audit, but allowed to apply shared variables state changes.
  
  - EVENT_EXEC: This is the reactive mechanism to execute what is specified by the params (command, data_string...). Shared variables (e.g. globals) write access allowed.

