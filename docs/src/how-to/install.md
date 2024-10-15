---
title: Installation
order: 1
---

## Requirements
**Supported operating systems**
  * Linux >=20.04 (Arch and Ubuntu tested)
  * macOS Monterey or later (Intel and Apple CPU)
  * Windows 10/11 with WSL2


**Prerequisites (Mandatory)**
- [cURL](https://curl.se)
- [git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)


**Prerequisites (Optional)**
Only if you disable precompiled binaries:
  - [Mysten Labs build prerequisites](https://docs.sui.io/build/install#prerequisites).

Only if you use Typescript related features:
  - [Node.js](https://nodejs.org/en/download/package-manager) (>=20)
  - [pnpm](https://pnpm.io/installation) (>=9)

Note: Suibase nicely informs you of missing dependencies as you start to interact with a feature.


## Installation Steps
```shell
$ cd ~
$ git clone https://github.com/chainmovers/suibase.git
$ cd suibase
$ ./install
```
Suibase is not intrusive on your system:

- All its files and workdirs are in ```~/suibase``` and ```~/.local/bin```
- Requires ```~/.local/bin``` to be in the [PATH](https://unix.stackexchange.com/questions/26047/how-to-correctly-add-a-path-to-path) env variable (you might have to add it manually).




## Update
```shell
$ ~/suibase/update
```
Will pull latest from GitHub to only update suibase itself.
To update sui clients and their local repos, use instead the workdir scripts (e.g. ```mainnet update```)
<br>

## Uninstall
**Important:** Save a copy of all your keys ( See [Where are the keys stored?]( ./devnet-testnet.md#where-are-the-keys-stored))

To completely remove suibase (and all keys) do:
```shell
$ ~/suibase/uninstall
$ rm -r ~/suibase
```


## Install FAQ
::: details Why suibase need to be cloned in user home (~)?
Suibase files are an "open standard" and benefit from being easily found by many apps and sdks. The user home is the most convenient solution.
:::

::: details How will suibase get the Sui binaries?
Suibase automatically install the proper binaries and repos from Mysten Labs matching each network.<br>
To reduce versioning problems, your apps can easily be build with the same Rust code matching the binaries (e.g. Sui Rust SDK crates). [More Info]( ./scripts.md#faster-rust-and-move-build)
:::
