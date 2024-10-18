---
title: "Using Localnet"
order: 2
---
## Starting
```shell
$ localnet start
```
The first time may take longer because of binaries installation.

You do not call ```sui``` directly anymore. Instead, call ```lsui``` :

```shell
$ lsui client active-address
0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462
```

```lsui``` is a *small* frontend to the Mysten Labs sui client, but its convenience is *huge*.<br>

You no longer have to "switch env". You can assume ```lsui``` always transparently execute with the proper ```sui``` client and keystore for this localnet (in same way, use ```dsui``` for devnet, and ```tsui``` for testnet).

Type ```localnet``` for help.

## Status / Stopping
Monitor the client version and process health with ```localnet status```:<br>
<img :src="$withBase('/assets/localnet-status.png')" alt="Localnet Status"><br>
To stop the process, do ```localnet stop```:<br>
<img :src="$withBase('/assets/localnet-stop.png')" alt="Localnet Stop"><br>

Monitor the RPC node servers with ```localnet links``` ([More Info]( ./proxy.md#monitoring-rpc-links))

## Upgrading Sui Client
Do ```localnet update``` to update to the latest binaries.

This also synchronize the local repo. That way the Rust SDK and Move dependencies also use that same latest version.

By default, the latest 'testnet' branch from Mysten Labs is used, you can choose a different branch by editing suibase.yaml ([More Info]( ./configure-suibase-yaml.md#change-default-repo-and-branch )).

## Regeneration
```shell
$ localnet regen
```
Quickly brings back the network to its initial state (with same addresses, alias and funds back). Useful for wiping out the network after testing.

The network is **always** initialized with 15 pre-funded addresses. 5 for each key type (ed25519, secp256k, secp256r1). Your Rust/Python apps can further access these addresses "by-name" for automated test setup.

## Faucet (Method #1)
Do ```lsui client faucet``` to get coins to the active-address.

Do ```lsui client gas``` or ```lsui client balance``` to see the balance changes.


## Faucet (Method #2)
A ```localnet faucet``` command provides more flexibility.

You can get funds to either a single address or all addresses at once (not just the active ones).

The following demo should be self-explanatory:<br>
<img :src="$withBase('/assets/faucet-demo.png')" alt="Faucet Demo">
Type ```localnet faucet``` for balance and help.

## Sui explorer
Open http://localhost:44380 for a Sui explorer on the localnet.
