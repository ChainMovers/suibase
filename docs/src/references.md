# Workdir Conventions

Suibase define a few conventions to coordinate among SDKs, apps and user.

::: tip

This section present more advanced subjects. If just starting to use Suibase scripts, then you may want to skip it.

:::

## Filesystem Path Convention

There are 6 <WORKDIR\>: `active`, `localnet`, `devnet`, `testnet`, `mainnet` and `cargobin`

Each <WORKDIR\> has the following components:

| Component      | Purpose                                                                                                                                                                                 |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| sui-exec       | A script allowing any app to safely call the right sui client+config combination for this workdir. Use it like you would use the "sui" client from Mysten Lab.                          |
| config         | Directory with Mysten Lab files needed to run the sui client (client.yaml and sui.keystore).                                                                                            |
| sui-repo       | A local repo of the Mysten lab sui code for building the client binary, but also for any apps to use the Rust SDK crates for compatibility.                                             |
| published-data | Information about last package published from this <WORKDIR\> using suibase scripts. This can be retrieved through JSON files or through suibase SDK helpers.                           |
| workdir-exec   | A script allowing any app to safely call the right "workdir script".<br> Example: `$ ~/workdirs/localnet/workdir-exec client gas` is equivalent to the shortcut `$ localnet client gas` |

Applications can expect the components to be always at these **fix** locations:
```text
 ~/suibase/
     └─ workdirs/
          └─ <WORKDIR>/
                ├── sui-exec
                ├── workdir-exec
                │
                ├── config
                │      ├── client.yaml
                │      └── sui.keystore
                │
                ├── sui-repo
                │      ├── crates/
                │      ├── target/
                │      └── ... complete sui repo (debug built) ...
                │
                └── published-data
                       └─ <package name>
                               └─ publish-output.json
```
::: details Official and Complete Path List
```
    ~/suibase/workdirs/<WORKDIR>/sui-exec
    ~/suibase/workdirs/<WORKDIR>/workdir-exec
    ~/suibase/workdirs/<WORKDIR>/config/client.yaml
    ~/suibase/workdirs/<WORKDIR>/config/sui.keystore
    ~/suibase/workdirs/<WORKDIR>/sui-repo/
    ~/suibase/workdirs/<WORKDIR>/published-data/<PACKAGE_NAME>/publish-output.json
```
:::

## What is the publish-output.json?
Doing publish using suibase scripts will create a publish-output.json file in the published-data directory of the workdir.

This file can then conveniently be read by your app (through Suibase helpers) to get package_id and object_id of new shared object published.


## Concurrency Limitation

When attempting to do many things at once (multiple apps accessing multiple network at same time), a few rules to keep in mind:

  - In the context of Suibase, each Sui client is associated with a single network (localnet, devnet, testnet, mainnet). You should NEVER edit the client.yaml to target different network type (localnet vs testnet). It is ok to have multiple env to target different RPC toward the **same** network (e.g. `fullnode.testnet.sui.io`, `fullnode.testnet.vincagame.com` ...)

  - Each Sui client have a single active address. An app changing the active-address should not assume that it will remain unchanged (because the user or other app are allowed to change it also). Suibase helper provides alternative to get client addresses in a more deterministic way (e.g. by name) for advanced test setup.

  - Changing the active workdir for "asui" can be safely done from multiple apps (and API), but only the last call will be effective. Changing the active workdir is not to be done "lightly", and typically it is expected that it will be driven by one user on the CLI (e.g. "localnet set-active").

::: warning
When the active workdir of "asui" is changed, it is recommended to "cargo clean" or "rebuild" your dependent apps. This is to make sure that the new workdir context is re-applied to all tools/SDKs.


