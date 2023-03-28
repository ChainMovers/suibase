---
hide:
  - toc
---
# Install Sui-Base

**Supported operating systems**

  * Linux
  * macOS
  * Windows 10/11 WSL2

**Prerequisites**

Install the [Sui prerequisites](https://docs.sui.io/build/install#prerequisites).

You can skip the section about installing the Sui binaries (unless you have an application that depends on ~/.sui/sui_config)

??? question "How will sui-base get the Sui binaries?"
    sui-base automatically download the code and builds a sui client for each workdir. One binary to target each network. This is better than a manual procedure installing a single binary per user and "switch network"... which does not work well if the binary happens to not be compatible with one of the network.

    For extra convenience your Rust app can also refer to the same Sui Rust SDK crates used by sui-base.

**Sui-Base Installation**
``` console
$ cd ~
$ git clone https://github.com/sui-base/sui-base.git
$ cd sui-base
$ ./install
```
Sui-base is not intrusive on your system. The installation is per user:

   - All its files and workdirs are kept in ~/sui-base
   - The installation only creates symlinks in ~/.local/bin

??? question "Why does sui-base need to be cloned in the user home (~) directory?"
    Sui-base workdir is an "open standard" and benefit from being easily found by many apps and sdks. The user home directory is the convenient solution.

**Starting Localnet**
``` console
$ localnet start
```
The first time will take minutes because of downloading and building the source code.

Type "localnet" for help.

**Repair**

"localnet regen" quickly brings back the network to its initial state (with all funds back). Very useful for just wiping out the network after testing.


**Uninstall**

To remove sui-base completely, do ~/sui-base/uninstall and delete ~/sui-base.