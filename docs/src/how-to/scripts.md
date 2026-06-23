---
title: Suibase Scripts
order: 1
---

## Introduction
All scripts are listed below and briefly described.

Best way to learn about each is probably just to try them... and "--help".


| **Script Name**                                       | **What are they for?**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| <h3>lsui<br>dsui<br>tsui<br>msui<br></h3>             | Frontends to Mysten Lab "sui" binaries, each targeting a specific network (no need to "switch" env):<br><p style="text-align:center"><b>l</b>sui→<b>l</b>ocalnet,&nbsp;<b>d</b>sui→<b>d</b>evnet,&nbsp;<b>t</b>sui→<b>t</b>estnet,&nbsp;<b>m</b>sui→<b>m</b>ainnet</p>Each script always runs within the proper workdir (client+keystore container) for the intended network.<br>The scripts are mostly transparent; all arguments are passed unchanged to a single Mysten Labs sui client call.<br><br>Example: `$ lsui client gas`   ← same as `sui client gas` but *always* for localnet |
| <h3>localnet<br>devnet<br>testnet<br>mainnet<br></h3> | These are the "workdir scripts" providing suibase specific features.<br><br>Example: `$ localnet faucet all`  ← sends Sui coins to every address on your localnet                                                                                                                                                                                                                                                                                                                                                                                                                    |
| <h3>twalrus<br>mwalrus<br>lwalrus<br>tsite<br>msite<br></h3>     | Frontends to the Mysten Labs **walrus** and **site-builder** binaries, each targeting a network (the proper config, context and wallet are added automatically):<br><p style="text-align:center"><b>t</b>walrus→<b>t</b>estnet,&nbsp;<b>m</b>walrus→<b>m</b>ainnet,&nbsp;<b>l</b>walrus→<b>l</b>ocalnet</p>`lwalrus` is a localnet-only **subset** of the `walrus` CLI (localnet Walrus — store/read/blob-status/delete); run `lwalrus --help` for the "Not supported for localnet" list.<br>Use `tsite`/`msite` instead of `site-builder` for Walrus Sites (testnet/mainnet).<br><br>Example: `$ twalrus info`. For localnet support and more info, see [Walrus](../walrus.md).                                                                                                                                                                |

## How to publish?
Suibase has a workdir command to make it easier to publish.

Example to publish on localnet:
```shell
$ cd <location of Move.toml>
$ localnet publish
```

Alternatively you can do:
```$ localnet publish --path <location of Move.toml>```

Use `testnet publish` / `mainnet publish` to publish on another network.

This should work assuming you have enough funds in the active-address (and the network is up and running!).

Do `$ localnet publish --help` for more info.

