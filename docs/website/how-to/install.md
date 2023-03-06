---
hide:
  - toc
---
# Install Sui-Base

**Supported operating systems**

  * Linux - (Ubuntu tested, others may work)
  * macOS
  * Windows 10/11 WSL (Ubuntu tested only)

**Prerequisites**

Install the [Sui prerequisites](https://docs.sui.io/build/install#prerequisites). 

You can skip the section about installing the Sui binaries (unless you have an application that depends on ~/.sui/sui_config to exist).

!!! note
    To avoid version mismatch issues, sui-base automatically download the source code and builds its own sui client binaries for each workdir. The Sui code is left in each workdir on-purpose. Your apps will now be able to refer to the same local Sui Rust SDK crates, and be "in sync" with everything else.

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
    Sui-base value come from having many tools using its workdirs. It benefits from being easily found... and any software can figure out where the user home directory is located. The choice was about favoring simplicity (over flexibility of being cloned anywhere).

**Starting Localnet**
``` console
$ localnet start
```
The first time will take minutes because of downloading and building the source code.

Type "localnet" for help.