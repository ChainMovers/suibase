---
hide:
  - toc
---
# Install Sui-Base

**Supported operating systems**

  * Linux - (Ubuntu tested, others may work)
  * macOS
  * Windows 10/11 WSL2 (Ubuntu tested only)

**Prerequisites**

Install the [Sui prerequisites](https://docs.sui.io/build/install#prerequisites). 

You can skip the section about installing the Sui binaries (unless you have an application that depends on ~/.sui/sui_config to exist).

??? question "How will sui-base get the Sui binaries?"
    sui-base download the source code and builds its own sui client for each workdir. This allows to have binaries always ready for each targeted network (something not easy to do with the normal Sui procedure of only one binary installed at the time).
    
    Your app can also refer to these local Sui Rust SDK crates and avoid potential obscure bug because of a binary version mismatch with a target network.

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