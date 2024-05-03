
# Workdir Conventions

Suibase define a few conventions to coordinate among SDKs, apps and user.

::: tip

This section present more advanced subjects. If just starting to use Suibase scripts, then you may want to skip it.

:::

## Filesystem Path Convention

There are 5 <WORKDIR\>: `active`, `localnet`, `devnet`, `testnet` and `mainnet`

Each <WORKDIR\> has the following components:

| Component      | Purpose                                                                                                                                                                         |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| sui-exec       | Script for any app to call the right sui client+config combination for this workdir. Use it like you would use the "sui" client from Mysten Labs.                  |
| config         | Directory with Mysten Labs files needed to run the sui client (client.yaml and sui.keystore).                                                                                    |
| sui-repo       | Local repo of Mysten Labs sui code for building the client binary. Provides also the matching Rust SDK crates.|
| published-data | Info about last package published from this <WORKDIR\>. Can be retrieved through JSON files or suibase SDK helpers.                   |
| workdir-exec   | Script allowing any app to call the right "workdir script".<br> Example: `$ ~/suibase/workdirs/localnet/workdir-exec update` is equivalent to the shortcut `$ localnet update` |

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
                               └─ ... various JSON information ...
```
::: details Official and Complete Path List
```
    ~/suibase/workdirs/<WORKDIR>/sui-exec
    ~/suibase/workdirs/<WORKDIR>/workdir-exec
    ~/suibase/workdirs/<WORKDIR>/config/client.yaml
    ~/suibase/workdirs/<WORKDIR>/config/sui.keystore
    ~/suibase/workdirs/<WORKDIR>/sui-repo/
    ~/suibase/workdirs/<WORKDIR>/published-data/<PACKAGE_NAME>/most-recent/package-id.json
    ~/suibase/workdirs/<WORKDIR>/published-data/<PACKAGE_NAME>/most-recent/created-objects.json
    ~/suibase/workdirs/<WORKDIR>/published-data/<PACKAGE_NAME>/most-recent/publish-output.json
```
:::

## What is the publish-output.json?
It is the output of a publish command.

Example, type `$ cd ~/suibase/rust/demo-app && localnet publish`). This will create a publish-output.json file in `~/suibase/workdirs/localnet/published-data/demo/most-recent`

The published-output.json can then be parsed by your app to get package and object IDs of the published module(s).

That info is also available in two convenient JSON files: package-id.json and created-objects.json.

See [Suibase Helper](./helpers.md) for convenient SDKs to find and read these JSON files.

