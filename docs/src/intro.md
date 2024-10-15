
# What is Suibase?

Suibase makes it easy to create "workdirs", each defining a distinct development environment targeting a network.

![Workdirs](./.vuepress/public/assets/workdirs-intro.png)

Other features like:
  * Simple "update" commands to keep your binaries and Sui SDKs in-sync with the latest network version.
  * **$ lsui/dsui/tsui/msui** scripts calls the **proper** sui binary+config combination for localnet, devnet, testnet or mainnet respectively. No "switch env" needed anymore.

  * **$ localnet star/stop/status** scripts and faucet.
  * **$ localnet regen** to reset the network with consistent tests addresses and alias (all pre-funded with an abundance of Sui).


Easy to [install](how-to/install.md).

Suibase works independently of any other Mysten Labs default installation and key store (never access ~/.sui, ~/.config/bin etc...). Therefore, it is safe to have Suibase and other [standard installation]( https://docs.sui.io/guides/developer/getting-started/sui-install ) co-exists on the same system.

Community driven. Join us on [Discord](https://discord.com/invite/Erb6SwsVbH)
