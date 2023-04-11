---
title: Installation
order: 1
---

## Requirements
**Supported operating systems**
  * Linux
  * macOS
  * Windows 10/11 WSL2
<br>

**Prerequisites**

Install the [Sui prerequisites](https://docs.sui.io/build/install#prerequisites).

Skip installing the Sui binaries (unless you have an application that depends on ~/.sui/sui_config).<br>

::: details How will sui-base get the Sui binaries?
Sui-base automatically download the code and builds a sui client for each workdir. One binary to properly match each network.<br><br>
For faster build your Rust app can also later change its dependencies to the same downloaded code (Sui Rust SDK crates). [More Info]( ./scripts.md#faster-rust-and-move-build)
:::

## Installation Steps
```shell
$ cd ~
$ git clone https://github.com/sui-base/sui-base.git
$ cd sui-base
$ ./install
```
Sui-base is not intrusive on your system. The installation is per user:

   - All its files and workdirs are kept in ~/sui-base
   - The installation only creates symlinks in ~/.local/bin

::: details Why sui-base need to be cloned in user home (~)?
Sui-base files are an "open standard" and benefit from being easily found by many apps and sdks. The user home is the convenient solution.
:::

## Update
```shell
$ ~/sui-base/update
```
Will pull latest from github to only update sui-base itself.
To update sui clients and local repos, use instead the workdir scripts (e.g. ```localnet update```)
<br>

## Uninstall
```shell
$ ~/sui-base/uninstall
$ rm -r ~/sui-base
```
Will remove sui-base completely.