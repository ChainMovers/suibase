---
title: Python Suibase Helper
---

As needed, read first the [Helper Overview](../helpers.md).

## Setup

Call `~/suibase/pip-install` within any python virtual environment in which you want to use the API.

Example creating a new environment and installing the API:
```shell
$ cd ~/myproject
$ python3 -m venv env
$ . env/bin/activate
$ ~/suibase/pip-install
```

## Typical Usage

    1. import suibase;
    2. Create an instance of suibase.Helper
    3. Verify suibase is_installed()
    4. select_workdir()
    5. ... use the rest of the API ...

## API
For now, there is no python documentation generated (work-in-progress).

The API very closely matches the [Rust API](https://chainmovers.github.io/suibase-api-docs/suibase/struct.Helper.html).

There is only one class: `Helper`

 Some demo calls for each methods:
 ```python
$ python3
Python 3.10.6 (main, Mar 10 2023, 10:55:28) [GCC 11.3.0] on linux
Type "help", "copyright", "credits" or "license" for more information.

>>> import suibase;
>>> helper=suibase.Helper();

>>> helper.is_installed()
True

>>> helper.select_workdir("localnet")

>>> helper.workdir()
'localnet'

>>> helper.keystore_pathname();
'/home/user/suibase/workdirs/localnet/config/sui.keystore'

>>> helper.client_address("active")
'0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462'

>>> helper.client_address("sb-1-ed25519");
'0x0fc530455ee4132b761ed82dab732990cb7af73e69cd6e719a2a5badeaed105b'

>>> helper.rpc_url()
'http://0.0.0.0:9000'

>>> helper.ws_url()
'ws://0.0.0.0:9000'

>>> helper.package_id("demo")
'0x794fc1d80f18a02eb0b7094d2f5a9f9f40bcf653996291f7a7086404689a19b5'

>>> helper.published_new_objects("demo::Counter::Counter")
['0xef876238524a33124a924aba5a141f2b317f1e61b12032e78fed5c6aba650093']
```

For the package_id and published_new_objects call to succeed, you have to first publish the package 'demo' on localnet:
```bash
$ localnet publish --path ~/suibase/rust/demo-app
```