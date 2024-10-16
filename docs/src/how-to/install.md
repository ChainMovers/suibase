---
title: Installation
order: 1
---

## Requirements
**Supported operating systems**
  * Linux Ubuntu >=20.04 recommended (works also on Arch Linux)
  * macOS Monterey or later (Intel and Apple CPU)
  * Windows 10/11 with WSL2

**Prerequisites**
- [cURL](https://curl.se)
- [git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)

## Installation Steps
```shell
$ cd ~
$ git clone https://github.com/chainmovers/suibase.git
$ ~/suibase/install
```
- All Suibase files are created in ```~/suibase``` and ```~/.local/bin```
- ```~/.local/bin``` must be listed in the PATH env variable ( on some setup you may have to [add it manually](https://unix.stackexchange.com/questions/26047/how-to-correctly-add-a-path-to-path) ).




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
