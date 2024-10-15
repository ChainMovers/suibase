---
title: "Integration with other tools"
contributors: true
editLink: true
---

## Walrus & Walrus Site Builder

Suibase has its own environment configuration for every network (localnet, devnet, testnet, mainnet).

To be able to use Walrus or Walrus Site Builder with Suibase, you first need to activate a particular Suibase network environment like this:

```bash
testnet start
```

_Currently Walrus is only available on Testnet, so the examples here a for Testnet only, but feel free to adjust the commands and paths according to your target environment._

Next step is to supply Suibase wallet configuration to Walrus or Walrus Site Builder this way:

```bash
walrus --wallet ~/suibase/workdirs/[network]/config/client.yaml <cmd>
#or
site-builder --wallet ~/suibase/workdirs/[network]/config/client.yaml <cmd>
```
where `[network]` is either localnet, devnet, testnet or mainnet.


For example, to deploy a web app to Walrus Sites on testnet, you need to do the following:

```bash
site-builder --config ./walrus.yaml --wallet ~/suibase/workdirs/testnet/config/client.yaml  publish ./dist
```
where `walrus.yaml` is a Walrus Site Builder config file.

For localnet, Suibase generate coins for all addresses on ```localnet start```. For testnet/devnet use ```dsui client faucet``` or ```tsui client faucet``` to get coins.

Please refer to the [Walrus documentation](https://docs.walrus.site/) to learn more about its config files and commands.
