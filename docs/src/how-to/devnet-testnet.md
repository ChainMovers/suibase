---
title: Using Devnet, Testnet and Mainnet
order: 3
---
::: warning
Instructions here are for devnet, but it is the same for testnet and mainnet. Just replace "devnet" with testnet/mainnet and dsui with tsui/msui.
:::

## Starting

Generally, works similar to localnet, except you are interacting with a public network instead of your own simulated local Sui network.

```shell
$ devnet start
```
The first time will take minutes because of downloading and building the binaries.

You do not call ```sui``` directly anymore, instead call ```dsui```:

```shell
$ dsui client active-address
0x92c03721eabfc753453b097d14d87e4012a9fe562da3582a6a023da7c6120c95
```
You no longer have to "switch env". You can assume ```dsui``` always transparently execute with its proper ```sui``` client and keystore for devnet (in same way, ```tsui``` for testnet, ```msui`` for mainnet. Each have their own keystore).

Type ```devnet``` for help.
<br>

## Status
You can check the client version with ```devnet status```<br>

::: warning Work-In-Progress
Status will eventually show the health of the network and your RPC connections.
:::

## Upgrading Sui Client
Do ```devnet update``` to download/rebuild to the latest client.

By default, the latest 'devnet' branch from Mysten Labs is used, you can choose a different branch or repo by editing the suibase.yaml ([More Info]( ./configure-suibase-yaml.md#change-default-repo-and-branch )).

