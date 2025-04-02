
# What is Suibase?

Suibase makes it easy to create "workdirs", each defining a distinct development environment targeting a network.

![Workdirs](./.vuepress/public/assets/workdirs-intro.png)

Other features like:
  * Simple "update" commands to keep all your binaries in-sync with the latest network version.
  * **$ lsui/dsui/tsui/msui** scripts calls the **proper** sui binary+config combination for localnet, devnet, testnet or mainnet respectively. No "switch env" needed anymore.
  * **$ twalrus/mwalrus** and **tsite/msite** scripts for always using proper binaries and configs combination with Walrus.
  * **$ localnet star/stop/status** scripts and faucet.
  * **$ localnet regen** to reset the network with consistent tests addresses and alias (all pre-funded with an abundance of Sui).



Easy to [install](how-to/install.md).

Suibase works independently of any other Mysten Labs default installation and key store (e.g. never touch ~/.sui, ~/.config/bin etc...). Therefore, it is safe to have other Sui and Walrus [standard installation]( https://docs.sui.io/guides/developer/getting-started/sui-install ) co-exists on same system.

Community driven. Join us on [Discord](https://discord.com/invite/Erb6SwsVbH)
