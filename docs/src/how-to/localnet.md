---
title: Using Localnet
order: 2
---
**Starting Localnet**
```shell
$ localnet start
```
The first time will take minutes because of downloading and building the binaries.

You do not call ```sui``` directly anymore. Instead call ```lsui``` :

```shell
$ lsui client active-address
0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462
```

```lsui``` is a *small* frontend to the Mysten Lab sui client, but what it brings in convenience is *huge*.<br>

You no longer have to "switch env". You can assume ```lsui``` always transparently call the proper ```sui``` binary and client keystore for this localnet (in same way, devnet/testnet have their own dsui/tsui trick).

Type ```localnet``` for help.
<br>

**Localnet Status / Stopping**<br>
You can monitor the client version and process health with ```localnet status```:<br>
<img :src="$withBase('/assets/localnet-status.png')" alt="Localnet Status"><br>
To stop the process, do ```localnet stop```:<br>
<img :src="$withBase('/assets/localnet-stop.png')" alt="Localnet Stop"><br>

**Upgrading Sui Client / Repo**<br>
Do ```localnet update``` to download/rebuild/restart the localnet with the latest.

By default, the latest 'devnet' branch from Mysten Labs is used, you can choose a different branch or repo by editing the sui-base.yaml ([More Info]( ./configure-sui-base-yaml.md#change-default-repo-and-branch )).

**Localnet Regeneration**
```shell
$ localnet regen
```
Quickly brings back the network to its initial state (with same addresses and all funds back). Useful for wiping out the network after testing.

The network is **always** initialized with 15 pre-funded addresses. 5 for each key type (ed25519, secp256k, secp256r1). Your Rust/Python apps can further access these addresses "by-name" for automated test setup.
<br>

**Localnet Faucet**<br>
Get funds to either a single address or all addresses at once on your localnet.

The following demo should be self-explanatory:<br>
<img :src="$withBase('/assets/faucet-demo.png')" alt="Faucet Demo">
Type ```localnet faucet``` for balance and help.