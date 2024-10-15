---
title: Using Devnet, Testnet and Mainnet
order: 3
---
::: warning
Instructions here are for devnet, but it is the same for other networks. Just replace "devnet" with testnet/mainnet and dsui with tsui/msui.
:::

## Starting

Works similar to localnet, except you are now interacting with a public network:

```shell
$ devnet start
```
The first time may take longer because of Sui binaries installation.

You do not call ```sui``` directly anymore, instead call ```dsui```:

```shell
$ dsui client active-address
0x92c03721eabfc753453b097d14d87e4012a9fe562da3582a6a023da7c6120c95
```
You no longer have to "switch env". You can assume ```dsui``` always transparently execute with the proper binary and keystore for devnet (in same way, ```tsui``` for testnet, ```msui``` for mainnet).

Type ```devnet``` for help.

## How to get coins from the faucet?
Do ```dsui client faucet``` (or ```tsui client faucet``` for testnet).

After ~1 minute, you should see more coins added to the active-address.
Do ```dsui client gas``` or ```dsui client balance``` to see the balance changes.


## Where are the keys stored?
| Network  | Keystore Location                               |
| -------- | ----------------------------------------------- |
| localnet | ~/suibase/workdirs/**localnet**/config/sui.keystore |
| devnet   | ~/suibase/workdirs/**devnet**/config/sui.keystore   |
| testnet  | ~/suibase/workdirs/**testnet**/config/sui.keystore  |
| mainnet  | ~/suibase/workdirs/**mainnet**/config/sui.keystore  |


Suibase is design to make it easy for your apps find what it needs consistently ([More Info]( ../references.md)).

## Status
You can check the client version and devnet services status with ```devnet status```.

You can monitor the RPC node servers with ```devnet links``` ([More Info]( ./proxy.md#monitoring-rpc-links)).


## Upgrading Sui Client
Periodically do ```devnet update``` to install the latest client.

By default, the recommended branch from Mysten Labs is used, you can choose a different branch or repo by editing the suibase.yaml ([More Info]( ./configure-suibase-yaml.md#change-default-repo-and-branch )).

