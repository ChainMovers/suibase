---
title: Using Testnet, Mainnet and Devnet
order: 3
---
::: warning
Instructions here are for testnet, but it is the same for other networks. Just replace "testnet" with devnet/mainnet and tsui with dsui/msui.
:::

## Starting

Works similar to localnet, except you are now interacting with a public network:

```shell
$ testnet start
```
The first time may take longer because of binaries installation.

You do not call ```sui``` directly anymore, instead call ```tsui```:

```shell
$ tsui client active-address
0x92c03721eabfc753453b097d14d87e4012a9fe562da3582a6a023da7c6120c95
```
You no longer have to "switch env". You can assume ```tsui``` always transparently executes with the proper binary and keystore for testnet (in the same way, ```msui``` for mainnet, ```dsui``` for devnet).

Type ```testnet``` for help.

## How to get coins from the faucet?
Do ```tsui client faucet```.

After ~1 minute, you should see more coins added to the active-address.
Do ```tsui client gas``` or ```tsui client balance``` to see the balance changes.


## Where are the keys stored?
| Network  | Keystore Location                               |
| -------- | ----------------------------------------------- |
| localnet | ~/suibase/workdirs/**localnet**/config/sui.keystore |
| testnet  | ~/suibase/workdirs/**testnet**/config/sui.keystore  |
| mainnet  | ~/suibase/workdirs/**mainnet**/config/sui.keystore  |
| devnet   | ~/suibase/workdirs/**devnet**/config/sui.keystore   |


Suibase is designed to make it easy for your apps to find what they need consistently ([More Info]( ../references.md)).

## Status
You can check the client version and testnet services status with ```testnet status```.

You can monitor the RPC node servers with ```testnet links``` ([More Info]( ./proxy.md#monitoring-rpc-links)).


## Upgrading Sui Client
Periodically do ```testnet update``` to install the latest client.

By default, the recommended branch from Mysten Labs is used, you can choose a different branch or repo by editing the suibase.yaml ([More Info]( ./configure-suibase-yaml.md#change-default-repo-and-branch )).
