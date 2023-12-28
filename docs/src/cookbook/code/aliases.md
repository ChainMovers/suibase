---
title: Aliases for Sui Addresses
contributors: true
editLink: true
---

## Facts

::: tip Fact Sheet

- Sui cli `sui client` will automatically generate a alias file (~/.sui/sui_config/sui.aliases) starting in version 1.16.0
- The alias file has a 1:1 mapping of alias names to the public key of the associated keypair
- The alias name must start with a letter and can contain only letters, digits, hyphens (-), or underscores (_)
- Command line caveats:
    - To rename an alias you will need to edit the alias file via editor
    - There is no known alias name length
- PySui support of aliases:
    - pysui will check for alias file when using `default_config()`, if not found it will generate one that complies with Sui 1.16.0 alias file format
    - pysui's `SuiConfig` has methods to list, rename, use aliases for address and keypair lookups, and address or keypair lookup of aliases
    - pysui enforces min and max aliases lengths to be between 3 and 64 characters. However; if alias name in alias file is modified manually pysui will continue to operate
    - An alias can be provided in the creation of new address/keypairs as well as recovering of same
    - pysui docs on [Aliases](https://pysui.readthedocs.io/en/latest/aliases.html)

:::

## Inspecting aliases

::: code-tabs

@tab sui

```shell
sui keytool list
```

@tab pysui

```python
from pysui import SuiConfig

def alias_look():
    """Show the aliase, associated address and public key."""
    cfg = SuiConfig.default_config()
    # If running localnet w/suibase use this
    # cfg = SuiConfig.sui_base_config()
    # Loop through aliases and print
    print()
    for alias in cfg.aliases:
        print(f"Alias:      {alias}")
        print(f"Address:    {cfg.addr4al(alias)}")
        print(f"PublicKey:  {cfg.pk4al(alias)}\n")

```

:::

## Renaming aliases

::: code-tabs

@tab sui

```shell
sui keytool update-alias old_alias_name _new_alias_name_
```

@tab pysui

```python
from pysui import SuiConfig

def alias_rename():
    """Rename an alias."""
    cfg = SuiConfig.default_config()
    # If running localnet w/suibase use this
    # cfg = SuiConfig.sui_base_config()
    # Rename alias for the active_address
    new_alias = "Primary"
    exiting_alias = cfg.al4addr(cfg.active_address)
    print(f"Existing alias for active address {cfg.active_address} is {exiting_alias}")
    cfg.rename_alias(old_alias=exiting_alias, new_alias=new_alias)
    print(f"Address associated to new alias 'Primary' = {cfg.addr4al(new_alias)}\n")

```
:::

## Using aliases

::: code-tabs

@tab sui

```shell
Not applicable at this time
```

@tab pysui

```python
from pysui import SyncClient,SuiConfig

def alias_use():
    """Use alias to lookup address for transaciton."""
    cfg = SuiConfig.default_config()
    # If running localnet w/suibase use this
    # cfg = SuiConfig.sui_base_config()
    client = SyncClient(cfg)

    # Using alias for convenience
    result = client.execute(GetAllCoins(owner=cfg.addr4al("Primary")))
    if result.is_ok():
        print(result.result_data.to_json(indent=2))
    else:
        print(result.result_string)

```

:::