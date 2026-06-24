---
title: Installation
order: 1
---

::: tip Using an AI agent? Paste this prompt:
```text:no-line-numbers
Install Suibase by following https://suibase.io/how-to/install.html
Then make sure ~/.local/bin is on my PATH.
Suibase is self-contained, so don't touch the standard Sui directory (~/.sui).
```
:::

## Requirements
**Supported operating systems**
  * Linux Ubuntu >=20.04 recommended (works also with Arch Linux)
  * macOS Monterey or later (Apple Silicon)
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
- ```~/.local/bin``` should be on your ```PATH```. If it isn't, copy the line for your shell and restart it:

<!-- no-copy-code: two alternatives — the user must select the ONE line for their shell, not copy both. -->
<div class="no-copy-code">

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc   # bash (Linux default)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc    # zsh (macOS default)
```

</div>




## Update
```shell
$ ~/suibase/update
```
This will pull the latest from GitHub to update only suibase itself.
To update sui clients, use the workdir scripts instead (e.g. ```mainnet update```)
<br>

## Uninstall
**Important:** Save a copy of all your keys ( See [Where are the keys stored?]( ./devnet-testnet.md#where-are-the-keys-stored))

To completely remove suibase (and all keys) do:
```shell
$ ~/suibase/uninstall
$ rm -r ~/suibase
```


## Install FAQ
::: details Why does suibase need to be cloned in user home (~)?
Suibase files are an "open standard" and benefit from being easily found by many apps and sdks. The user home is the most convenient solution.
:::

::: details How will suibase get the Sui binaries?
Suibase automatically downloads and installs the proper precompiled Sui binaries from Mysten Labs matching each network.

On a platform without a precompiled binary, Suibase automatically falls back to cloning the Sui repo and building the binaries locally.
:::
