---
title: "Suibase Python"
---

Contains python program examples for interacting with Sui blockchain. The demo applications
require [pysui]( https://pysui.readthedocs.io/ ) Python SUI Client SDK to run.

## Pre Setup

If not already done, you should first [install suibase](../../how-to/install.md)

## Setup

```shell
$ cd ~/suibase/python
$ python3 -m venv env
$ . env/bin/activate
$ pip install -U pip
$ pip install --use-pep517 -r requirements.txt
```

## Demo's

The examples for python target the `active` workdir (one of localnet/testnet/devnet etc...). Type `asui` to display the active.

To switch the active, use the workdir "set-active" command. Example, `localnet set-active`.

The workdir should be initialized/started before running the demos. As an example, if 'localnet' then `localnet start` should have been done.

For convenience, shell scripts have been added to `suibase/python/bin`. It is expected
that when you want to run a script you are in the python folder, and you've activated the
virtual environment (`. env/bin/activate`).

| Demo    | What it does                                  | Invoke        | source                                                                          |
| ------- | --------------------------------------------- | ------------- | ------------------------------------------------------------------------------- |
| sysinfo | displays general sui chain information        | `bin/sysinfo` | [src/demo1](https://github.com/ChainMovers/suibase/tree/main/python/src/demo1 ) |
| coinage | displays information about coins and balanced | `bin/coinage` | [src/demo2](https://github.com/ChainMovers/suibase/tree/main/python/src/demo2)  |
| pkgtxn  | demonstrate programmable transaction          | `bin/prgtxn`  | [src/demo3](https://github.com/ChainMovers/suibase/tree/main/python/src/demo3)  |

## Config (client.yaml)
When pysui runs with suibase installed, it will look for a client.yaml in:<br>
`~/suibase/workdirs/active/config`

The `active` portion of the path is a symlink resolving to either `localnet`, `devnet` etc... as an example, when localnet is active, the resolved path becomes:<br>
`~/suibase/workdirs/localnet/config/client.yaml`

## pysui utilities

With installing `pysui` you also have access to a number of installed utilities:

1. `wallet --local [command]`
2. `async-gas --local`
3. `async-sub --local`
4. `async-sub-txn --local`
