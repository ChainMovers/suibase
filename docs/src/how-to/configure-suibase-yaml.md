# Workdir Customization ( suibase.yaml )

Changing the remote github repo, branch, RPC ports etc... are done using the suibase.yaml found in each workdir ( Example: `~/suibase/workdirs/localnet/suibase.yaml` )

We will cover here only a few common use case. See this [suibase.yaml](https://github.com/chainmovers/suibase/blob/main/scripts/defaults/localnet/suibase.yaml) for the complete parameters list.


### Increase localnet initial funding
Add `initial_fund_per_address: 9999999999999999999` to the file then type `localnet regen`.

Set the number to as much as you need (max 64 bits unsigned supported).

### Change default repo and branch
Add the default_repo_XXXX variables (it is ok to change only one) and then type the workdir update command (e.g. `localnet update`). Example:

``` yaml
default_repo_url: "https://github.com/acme/forked_sui.git"
default_repo_branch: "main"
```

### Add your own private keys
You can have suibase includes in the sui.keystore your own private keys with an ```add_private_keys``` YAML array list. Example:

``` yaml
add_private_keys:
  - AOToawZbfMNATU6KPldYuoGQpp82BE0w5BknPCTBjgXT
  - 0x126e82a77f7768a59d355eb4ceb9c1a33b3652b8896c22d6b7e0ff94cee23109
```

- YAML is indentation sensitive. You need exactly two spaces in front of the '-'. 
- The private key can be either in sui.keystore format (Base64 33 bytes) or wallet format (Hex 32 bytes).

To apply the change you need to perform an update, regen or start workdir command (e.g. `localnet regen`).

### Disable auto generation of addresses
By default wallets are created with 15 addresses (5 of each types) for convenience of automated testing. This can be disabled with `auto_key_generation: false`

For localnet, this change is applied on 'localnet regen' only.

For remote network (testnet/devnet/mainnet) you need to modify the `<workdir name>/suibase.yaml` after the workdir 'create' command and before any other command that create a wallet (e.g. `mainnet start`). Alternatively, you can disable auto-generation for all workdirs with [global customization]( #global-customization-advanced-feature ).

### Global Customization (advanced feature)
You can apply the same default customization to **all** your workdir with a suibase.yaml located at `~/suibase/workdirs/common/suibase.yaml`.

Everytime you run a suibase command, it loads up to 3 YAML files in a specific order:
  (1) ~/suibase/scritps/defaults/\<workdir name>/suibase.yaml
  (2) ~/suibase/workdirs/common/suibase.yaml
  (3) ~/suibase/workdirs/\<workdir name>/suibase.yaml

When a given variable is defined by more than one suibase.yaml, the last one takes effect.

In short... (1) is how suibase defines **every** default variables for every possible workdir, you then optionally create (2) for your own default customization for all workdir and optionally (3) for the final level of customization specific to a workdir.
