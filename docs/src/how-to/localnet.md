---
title: "Using Localnet"
order: 2
---
## Starting
```shell
$ localnet start
```
The first time will take minutes because of downloading and building the binaries.

You do not call ```sui``` directly anymore. Instead call ```lsui``` :

```shell
$ lsui client active-address
0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462
```

```lsui``` is a *small* frontend to the Mysten Labs sui client, but its convenience is *huge*.<br>

You no longer have to "switch env". You can assume ```lsui``` always transparently execute with the proper ```sui``` client and keystore for this localnet (in same way, use ```dsui``` for devnet, and ```tsui``` for testnet).

Type ```localnet``` for help.
<br>

## Status / Stopping
You can monitor the client version and process health with ```localnet status```:<br>
<img :src="$withBase('/assets/localnet-status.png')" alt="Localnet Status"><br>
To stop the process, do ```localnet stop```:<br>
<img :src="$withBase('/assets/localnet-stop.png')" alt="Localnet Stop"><br>

## Upgrading Sui Client
Do ```localnet update``` to download/rebuild/restart the localnet with the latest.

This also update the local repo that can provide the matching Rust SDK and Move dependencies to your app.

By default, the latest 'devnet' branch from Mysten Labs is used, you can choose a different branch or repo by editing suibase.yaml ([More Info]( ./configure-suibase-yaml.md#change-default-repo-and-branch )).

## Regeneration
```shell
$ localnet regen
```
Quickly brings back the network to its initial state (with same addresses and all funds back). Useful for wiping out the network after testing.

The network is **always** initialized with 15 pre-funded addresses. 5 for each key type (ed25519, secp256k, secp256r1). Your Rust/Python apps can further access these addresses "by-name" for automated test setup.
<br>

## Faucet
Get funds to either a single address or all addresses at once on your localnet.

The following demo should be self-explanatory:<br>
<img :src="$withBase('/assets/faucet-demo.png')" alt="Faucet Demo">
Type ```localnet faucet``` for balance and help.