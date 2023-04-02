## Complete list of scripts
This is just a brief intro.

Best way to learn about these scripts is probably just... try them... and "--help".


| <h3>**Script Name**                       | <h3>**What are they for?**<h3>                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| <h3>lsui<br>dsui<br>tsui<br><h3>          | These scripts are front-ends to Mysten Lab "sui" binaries.<br> They target directly a network, no need to "switch" env.<br><br>  (lsui->localnet, dsui->devnet, tsui->testnet). <br><br>Each script always uses the proper Sui binary+keystore+client.yaml set for the intended network.<br> The scripts are mostly transparent; all their arguments are pass unchanged to a Mysten sui binary.<br><br>Example: '$ lsui client gas'   <-- same as 'sui client gas' when active-env is 'localnet' |
| <h3>localnet<br>devnet<br>testnet<br><h3> | To avoid confusion the lsui/dsui/tsui scripts are intended to remain as close as possible to Mysten lab sui client binary.<br><br>These localnet/devnet/testnet are called the "workdir" script and are for providing the additional sui-base specific features.<br><br>Example: "$ localnet faucet all"  <-- This will send Sui coins to every address on your localnet<br>                                                                                                                     |
| <h3>asui<h3>                              | You can designate one workdir as "active".<br> This script will call the "active sui" client.                                                                                                                                                                                                                                                                                                                                                                                                    |

## More reliable Rust and Move build with a Sui local repo.
Sui-base download a local repo of the Mysten Lab's code to build the sui client, so you might as well use the same for your own code.

Advantages are:

   * Faster build (local file access vs remote download)
   * Less build/publish errors (sometimes github do have trouble serving, causing dependencies loading errors)
   * Control having your app, client and localnet being built from the **same** source (avoid version mismatch issues).

Location of these repos are:

  - ~/sui-base/workdirs/**localnet**/sui-repo
  - ~/sui-base/workdirs/**devnet**/sui-repo
  - ~/sui-base/workdirs/**testnet**/sui-repo

Update to latest with the "update" command (e.g. "localnet update").
<br>

## Cargo.toml dependencies to local repos
This is an optional change, but highly recommended. Instead of git, use "path" to the local repos.

Example, replace:<br>
```sui-sdk = { git = "https://github.com/MystenLabs/sui", branch = "devnet" }```
<br>with<br>
```sui-sdk = { path = "../../sui-base/workdirs/active/sui-repo/crates/sui-sdk/" }```

The number of ".." may need to be adjusted depending where your Cargo.toml is located relative to ~/sui-base.

This is a working example: [Cargo.toml :octicons-mark-github-16:](https://github.com/sui-base/sui-base/blob/main/rust/demo-app/Cargo.toml)

If you always target the same network you can replace the "active" word with the specific workdir (e.g. localnet/devnet/testnet).

## What does "active" mean?
A single workdir is designated as active and allows multiple tools/scripts to execute within the same environment.

You choose the active with the workdir "set-active" command. Examples:
``` shell
$ localnet set-active
$ devnet set-active
```
The "asui" will conveniently call the "sui client" for the active workdir. You can then write your own automation that will work with the currently active network.

## Move.toml dependencies to local repos
This is optional, but highly recommended. Instead of git, use "local" to the local repos.

Example, replace:<br>
```
[dependencies]
Sui = { git = "https://github.com/MystenLabs/sui.git", subdir="crates/sui-framework/packages/sui-framework/", rev = "devnet" }
```
<br>with<br>
```
Sui = { local = "../../sui-base/workdirs/active/sui-repo/crates/sui-framework/packages/sui-framework" }
```

You may need to adjust the number of ".." depending where your Move.toml is located relative to ~/sui-base.

If you prefer to always target the same network you can replace the "active" word with the specific workdir (e.g. localnet/devnet/testnet).

## How to publish?
Sui-base has a workdir command to make it easier to publish.

Example to publish on localnet:
```
$ cd <location of Move.toml>
$ localnet publish
```

Alternatively you can do:
```$ localnet publish --path <location of Move.toml>```

If you have coded your dependencies path in Move.toml with "active" then you can easily switch and publish on any network:
```
$ devnet set-active
$ devnet publish
```

This should work assuming you have enough fund in the active-address (and the network is up and running!).

Check "localnet publish --help" for more info.

