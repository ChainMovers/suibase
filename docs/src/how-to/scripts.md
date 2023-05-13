---
title: Suibase Scripts
order: 1
---

## Introduction
All scripts are listed below and briefly described.

Best way to learn about each is probably just to try them... and "--help".


| **Script Name**                                       | **What are they for?**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| <h3>lsui<br>dsui<br>tsui<br>msui<br></h3>             | Front-ends to Mysten Lab "sui" binaries, each targeting a specific network (no need to "switch" env):<br><p style="text-align:center"><b>l</b>sui→<b>l</b>ocalnet,&nbsp;<b>d</b>sui→<b>d</b>evnet,&nbsp;<b>t</b>sui→<b>t</b>estnet,&nbsp;<b>m</b>sui→<b>m</b>ainnet</p>Each script always run within the proper workdir (client+keystore container) for the intended network.<br>The scripts are mostly transparent; all arguments are pass unchanged to a single Mysten Labs sui client call.<br><br>Example: `$ lsui client gas`   ← same as `sui client gas` but *always* for localnet |
| <h3>localnet<br>devnet<br>testnet<br>mainnet<br></h3> | These are the "workdir scripts" providing suibase specific features.<br><br>Example: `$ localnet faucet all`  ← sends Sui coins to every address on your localnet<br>                                                                                                                                                                                                                                                                                                                                                                                                                     |
| <h3>asui</h3>                                         | You can designate one workdir as "active". [More Info](scripts.md#what-does-active-mean)<br> This script will call the "active sui" client.                                                                                                                                                                                                                                                                                                                                                                                                                                               |

## Faster Rust and Move Build
Suibase download the Mysten Lab's repo locally to build a sui client for each network, so your apps might as well re-use these.

Advantages are:

   * Faster local file access on rebuild.
   * Less build/publish errors (sometimes GitHub do have trouble serving, causing dependencies loading errors)
   * Control having your app, client and localnet being built from the **same** source (avoid version mismatch issues).

Location of these repos are:

  - ~/suibase/workdirs/**localnet**/sui-repo
  - ~/suibase/workdirs/**devnet**/sui-repo
  - ~/suibase/workdirs/**testnet**/sui-repo
  - ~/suibase/workdirs/**mainnet**/sui-repo

Update to latest with the "update" command (e.g. "localnet update").
<br>

## Cargo.toml dependencies to local repos
This is optional, but highly recommended. Instead of git, use "path" to the local repos.

Example, replace:<br>
```toml
[dependencies]
sui-sdk = { git = "https://github.com/MystenLabs/sui", branch = "devnet" }
```
with
```toml
[dependencies]
sui-sdk = { path = "../../suibase/workdirs/active/sui-repo/crates/sui-sdk/" }
```
The number of ".." may need to be adjusted depending on where your Cargo.toml is located relative to ~/suibase.

If you always target the same network you can replace the "active" word with a specific workdir (e.g. localnet/devnet/testnet/mainnet).

Demo Example: [Cargo.toml](https://github.com/chainmovers/suibase/blob/main/rust/demo-app/Cargo.toml)

## What does "active" mean?
A single workdir is designated as active and allows multiple tools/scripts to run in the same context and target the same network.

You choose the active with a workdir "set-active" command. Examples:
```shell
$ devnet set-active
devnet is now active
```
The "asui" will conveniently call the correct "sui client" for the active workdir. You will typically want to use "asui" in your own script for operations that are expected to work for *all* network type.

## Move.toml dependencies to local repos
This is optional, but highly recommended. Instead of git, use "local" to the local repos.

Example, replace:<br>
```toml
[dependencies]
Sui = { git = "https://github.com/MystenLabs/sui.git", subdir="crates/sui-framework/packages/sui-framework/", rev = "devnet" }
```
with
```toml
[dependencies]
Sui = { local = "../../suibase/workdirs/active/sui-repo/crates/sui-framework/packages/sui-framework" }
```
You may need to adjust the number of ".." depending on where your Move.toml is located relative to ~/suibase.

If you prefer to always target the same network you can replace the "active" word with a specific workdir (e.g. localnet/devnet/testnet).

Demo example: [Move.toml](https://github.com/sui-base/suibase/blob/main/rust/demo-app/move/Move.toml)

## How to publish?
Suibase has a workdir command to make it easier to publish.

Example to publish on localnet:
```shell
$ cd <location of Move.toml>
$ localnet publish
```

Alternatively you can do:
```$ localnet publish --path <location of Move.toml>```

If you have coded your dependencies' path in Move.toml with "active", then you can easily switch and publish on any network:
```shell
$ testnet set-active
testnet is now active
$ testnet publish
```

This should work assuming you have enough fund in the active-address (and the network is up and running!).

Do `$ localnet publish --help` for more info.

