---
title: Suibase Helpers Overview
---

This page is an introduction. When ready, check the following for language specific docs:
[<iconify-icon class="font-icon icon" icon="marketeq:curve-arrow-right"></iconify-icon> Rust Helper](./rust/helper.md)
[<iconify-icon class="font-icon icon" icon="marketeq:curve-arrow-right"></iconify-icon> Python Helper](./python/helper.md)<br>

## What is a Suibase Helper?
An API providing information to accelerate the development and testing of Sui apps.

Your app get access to:
- Package ID of most recently published modules (can query by name).
- IDs of the shared objects created on last publish of your module.
- active client address (can also query by alias).
- A healthy RPC URL for a specific network (e.g. devnet).
- Various utility functions to help automating development.

**How it works?**
The magic happens when you do a workdir "publish" command (e.g. ```testnet publish```). This is a drop-in replacement of the Sui binary approach (e.g. ```sui publish```) and the same parameters can be specified.

The Suibase command calls the proper Mysten Labs Sui client version matching the network. It adds parameters to save the output in a JSON file. The data is copied in the Suibase workdir structure, and becomes accessible to your apps through an Helper API.


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
    # Get one of these address using its alias.
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
       // Get one of these address using its alias.
       let test_address = sbh.client_address("sb-1-ed25519");
       println!("Test address 1 type ed25519: {}", test_address );
    }
  }

  //Console output:
  //Active address: 0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462
  //Test address 1 type ed25519: 0x0fc530455ee4132b761ed82dab732990cb7af73e69cd6e719a2a5badeaed105b

```

:::


#### Example 2: What is my last published package ID on devnet?
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

#### Example 3: Which URL should be used right now for testnet?
Suibase monitor RPC health of multiple servers and return the best URL to use.

::: code-tabs

@tab:active Python

```python
TODO
```

@tab Rust

```rust
TODO
```