---
title: Installation
order: 1
---

## Requirements
**Supported operating systems**
  * Linux (Arch and Ubuntu tested)
  * macOS
  * Windows with WSL2
<br>

**Prerequisites**

Install the [Sui prerequisites](https://docs.sui.io/build/install#prerequisites).

Skip installing the Sui binaries (unless you have an application that depends on ~/.sui/sui_config).<br>

::: details How will suibase get the Sui binaries?
Suibase will automatically download the sui client and repos from Mysten Labs that match each network version.<br>
For consistency your Rust app can optionally have their dependencies set to the same downloaded code (Sui Rust SDK crates). [More Info]( ./scripts.md#faster-rust-and-move-build)
:::

## Installation Steps
```shell
$ cd ~
$ git clone https://github.com/chainmovers/suibase.git
$ cd suibase
$ ./install
```
Suibase is not intrusive on your system. The installation is per user:

   - All its files and workdirs are kept in ~/suibase
   - The installation only creates symlinks in ~/.local/bin

::: details Why suibase need to be cloned in user home (~)?
Suibase files are an "open standard" and benefit from being easily found by many apps and sdks. The home directory is the convenient solution.
:::

## Update
```shell
$ ~/suibase/update
```
Will pull latest from github to only update suibase itself.
To update sui clients and their local repos, use instead the workdir scripts (e.g. ```mainnet update```)
<br>

## Uninstall
```shell
$ ~/suibase/uninstall
$ rm -r ~/suibase
```
Will remove suibase completely.