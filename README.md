TODO Add Rust/Python/examples to doc...

sui-base provides scripts for a reproducible localnet setup (same addresses, same initial fund).

By default, each address has 5 coins with 8 Sui each, but can be customized to billions easily...

The sui-base-helper Rust module provides utilities such as getting the id of the last package that you did publish on localnet/devnet/testnet.

## Documentation

Online docs can be found [Here](https://sui-base.io/)

## Installation

- Install the Sui Requirements (Git, Rust etc...)
  The Sui binaries are not needed (the script download Sui repo)

- Clone sui-base (this repo)

- run './install'. It creates only symlinks in the user account (no system change)

- run 'localnet start' ... will take many minutes to create the localnet.

The localnet will be at http://0.0.0.0:9000

## Where is the sui client?

You must type "lsui" instead of "sui".

Example, do "lsui client gas" instead of "sui client gas".

Why is that? sui-base also support "dsui" and "tsui" for respectively devnet and testnet... and each can be different binaries!

## Features (work in progress)

- Keeps localnet/devnet/testnet keystores seperated.
- Does not touch the user ~/.sui and its keystore (assumes might be used later for mainnet).
- Repeatable localnet that can be reset quickly with the same pre-funded address.
- Customizable pre-funding amount (See scripts/genesis/config.yaml ).
- Convenient Sui CLI frontends for each network ("dsui" for devnet, "tsui" for testnet...)

## Initial State

Always same 5 client addresses. The 0xc7148~89a7 is the default active client.

```
$ lsui client addresses
Showing 5 results.
0x4e8b8c06d7aed3c11195794fa7b0469855c57b30
0x5f11df8d90fef7a642d561aed0f2ee64de5c373c
0x8638a4d6438b399a77659463a25fdf2bdf0b229b
0x86f066b23d7e60ec4dbb280a4c265772c186693b
0xc7148f0c0086adf172eb4c2076c7d888337789a7

$ lsui client active-address
0xc7148f0c0086adf172eb4c2076c7d888337789a7

$ lsui client gas
                 Object ID                  |  Gas Value
----------------------------------------------------------------------
 0x0b162ef4f83118cc0ad811de35ed330ec3441d7b | 800000000000000
 0x2d43245a6af1f65847f7c18d5f6aabbd8e11299b | 800000000000000
 0x9811c29f1dadb67aadcd59c75693b4a91b347fbb | 800000000000000
 0xc8381677d3c213f9b0e9ef3d2d14051458b6af8a | 800000000000000
 0xd0b2b2227244707bce233d13bf537af7a6710c01 | 800000000000000
```

(Default 40 Sui per client, you can customize to as much as you need)

## Development Setup

```
<Your home directory>
    │
    ├── <Other Rust app can also refer to "~/sui-base/workdirs/devnet-branch">
    |
    └── sui-base/    # This git cloned repo
          ├── install            # Do './install' first
          |
          ├── rust/
          |    ├── hello-app     # An hello world! example.
          |    └── ...
          |
          ├── scripts/
          │    ├── localnet      # To manage your localnet
          │    ├── lsui          # Sui CLI frontend for localnet
          │    ├── dsui          # Sui CLI frontend for live devnet
          │    └── tsui          # Sui CLI frontend for live testnet
          │
          └── workdirs/       # Created by the scripts
               ├── devnet-branch      # Complete local repo of Sui devnet branch.
               ├── testnet-branch     # Complete local repo of Sui testnet branch (later)
               ├── localnet-workdir   # All localnet files. runs at http://0.0.0.0:9000
               ├── devnet-workdir     # Keystore for live devnet network
               └── testnet-workdir    # Keystore for live tesnet network
```

================

```$ localnet --help
Usage: localnet [COMMAND]

Simulate a sui network running fully on this machine
Accessible from http://0.0.0.0:9000

COMMAND:
   start:  start localnet (sui process will run in background)
   stop:   stop the localnet (sui process will exit)
   status: indicate if running or not
   update: Update local sui repo branch and regen localnet.
   regen:  Regenerate localnet. Useful for recovering.

All localnet DB and temporary files are in ~/sui-base/localnet-workdir
```
