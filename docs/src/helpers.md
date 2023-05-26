---
title: Suibase Helpers Overview
---

This page is an introduction. For more details... follow the arrow:<br>
[<i class="iconfont icon-arrow"></i> Rust Suibase Helper API](../rust/helper.md)<br>
[<i class="iconfont icon-arrow"></i> Python Suibase Helper API](../python/helper.md)<br>

## What is a Suibase Helper?
An API providing what is needed to initialize Sui Network SDKs.

That includes params such as the active client address and valid RPC URL.

Some other params needed in a typical "edit/publish/test" dev cycle are *your modules* package and shared_object IDs that you last published.

These IDs are generated at publication time by the Sui client and written in a JSON file. Suibase automatically preserve this file in the workdir, and makes the IDs easily readable by your apps.

### Example 1: What is the active client address for localnet?

::: code-tabs

@tab:active Python

```python
    import suibase;

    # Create suibase helper.
    sbh = suibase.Helper()
    if not sbh.is_installed():
        print("suibase is not installed. Please do ~/suibase/install first.")
        exit(1)

    # Select a workdir.
    sbh.select_workdir("localnet")

    # Print the active address, same as "sui client active-address"
    active_address = sbh.client_address("active")
    print(f"Active address: { active_address }")

    # Suibase supports more than just "active"...
    #
    # localnet has *always* at least 15 named addresses for deterministic test setups.
    #
    # Print one of these address by-name (see the API for how to access all of them).
    test_address_1 = sbh.client_address("sb-1-ed25519")
    print(f"Test address 1 type ed25519: { test_address_1 }")

    ######## Console output #####
    # Active address: 0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462
    # Test address 1 type ed25519: 0x0fc530455ee4132b761ed82dab732990cb7af73e69cd6e719a2a5badeaed105b
    #############################
```

@tab Rust

```rust
  use suibase::Helper;
  fn main() {
    // Create a Suibase helper API.
    let sbh = Helper::new();

    if sbh.is_installed()? {
       // Select the localnet workdir.
       sbh.select_workdir("localnet")?;

       // Print the active address, same as "sui client active-address"
       println!("Active address: {}", sbh.client_address("active"));

       // Suibase supports more than just "active"...
       //
       // localnet has *always* at least 15 named addresses for deterministic test setups.
       //
       // Print one of these address by-name (see the API for how to access all of them).
       println!("Test address 1 type ed25519: {}", sbh.client_address("sb-1-ed25519"));
    }
  }

  //Console output:
  //Active address: 0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462
  //Test address 1 type ed25519: 0x0fc530455ee4132b761ed82dab732990cb7af73e69cd6e719a2a5badeaed105b

```

:::


#### Example 2: What is my last published package ID?
::: code-tabs

@tab:active Python

```python
TODO
```

@tab Rust

```rust
TODO
```

:::

#### Example 3: Which URL should be used right now for localnet?
Suibase monitor RPC health of multiple fullnode and return the best URL to use.

::: code-tabs

@tab:active Python

```python
TODO
```

@tab Rust

```rust
TODO
```
:::

::: warning Work-In-Progress
RPC health monitoring is not yet implemented.<br>
For now, the Helper always returns the URLs of public fullnodes services from Mysten Labs.
:::
