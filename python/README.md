---
hide:
  - toc
---

## sui-base Python

Contains python program examples for interacting with Sui blockchain. The demo applications
require `pysui` Python SUI Client SDK to run.

## Pre Setup

You should first setup `sui-base` by following steps found [Here](https://sui-base.io/how-to/install/)

## Setup

??? abstract "Setup Steps"

    ```
    $ cd sui-base
    $ python3 -m venv env
    $ . env/bin/activate
    $ pip install -U pip
    $ pip install --use-pep517 -r requirements.txt
    ```

## Demo's

The examples for python search the `sui-base` workdirs to figure out which configuration
to use. If looks for the `active` symlink and reads the `client.yaml` from that link.

However; if you are running a localnet you will, of course, have to `localnet start` before
running the python demos.

For convenience, shell scripts have been added to `sui-base/python/bin`. It is expected
that when you want to run a script you are in the python folder and you've activated the
virtual environment (`. env/bin/activate`).

| Demo    | What it does                                  | Invoke        | source    |
| ------- | --------------------------------------------- | ------------- | --------- |
| sysinfo | displays general sui chain information        | `bin/sysinfo` | src/demo1 |
| coinage | displays information about coins and balanced | `bin/coinage` | src/demo2 |