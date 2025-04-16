# Workdir Customization (suibase.yaml)

Changing the remote GitHub repo, branch, RPC ports etc... are done using the suibase.yaml found in each workdir (Example: `~/suibase/workdirs/localnet/suibase.yaml`)

We will cover here only a few common use case. See this [suibase.yaml](https://github.com/chainmovers/suibase/blob/main/scripts/defaults/localnet/suibase.yaml) for the complete parameters list.

### GitHub Rate Limit ( GITHUB_TOKEN )
Suibase make use of the GitHub API, which is rate limited for non-authenticated users.

You can avoid rate limit errors by creating and adding your own GITHUB_TOKEN to suibase.yaml:


``` yaml
github_token: ghp_9UsdjErt5jJusimndApo3i2wreuYsu2dHnEm
```

Recommended adding to ~/suibase/workdirs/common/suibase.yaml so that it applies to all workdir.

More info: [ Github Rate Limits ]( https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api ), [ GitHub Tokens ]( https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens )


### Change default repo branch
Add the `default_repo_branch` to suibase.yaml and type the workdir "update" command (e.g. `localnet update`). Example:

``` yaml
default_repo_branch: "main"
```

### Force to build locally
By default, Suibase install official binaries from Mysten Labs or uses open-source continuous integration ( [Github](https://github.com/ChainMovers/sui-binaries) ).

If you prefer (or need) to build your own binaries, then add ```precompiled_bin: false``` to a suibase.yaml. Suibase will then automatically "cargo build" on a workdir "update" (e.g. ```devnet update```).


### Add your own private keys
You can have suibase includes in the sui.keystore your own private keys with ```add_private_keys``` YAML array list. Example:

``` yaml
add_private_keys:
  - AOToawZbfMNATU6KPldYuoGQpp82BE0w5BknPCTBjgXT
  - 0x126e82a77f7768a59d355eb4ceb9c1a33b3652b8896c22d6b7e0ff94cee23109
```

- YAML is indentation sensitive. You need exactly two spaces in front of the '-'.
- The private key can be either in sui.keystore format (Base64 33 bytes) or wallet format (Hex 32 bytes).

To apply the change you need to perform an update, regen or start workdir command (e.g. `localnet regen`).

### Disable auto generation of addresses
By default wallets are created with 15 addresses (5 of each type) for convenience of automated testing. This can be disabled with `auto_key_generation: false`

For localnet, this change is applied on 'localnet regen' only.

For remote network (testnet/devnet/mainnet) you need to modify the `<workdir name>/suibase.yaml` after the workdir 'create' command and before any other command that create a wallet (e.g. `mainnet start`). Alternatively, you can disable auto-generation for all workdirs with [global customization]( #global-customization-advanced-feature ).

### Global Customization (advanced feature)
You can apply the same default customization to **all** your workdir with a suibase.yaml located at `~/suibase/workdirs/common/suibase.yaml`.

Every time you run a suibase command, it loads up to 3 YAML files in a specific order:
  (1) ~/suibase/scripts/defaults/\<workdir name>/suibase.yaml
  (2) ~/suibase/workdirs/common/suibase.yaml
  (3) ~/suibase/workdirs/\<workdir name>/suibase.yaml

You should never modify the files under ~/suibase/scripts/defaults. They are overwritten when you update Suibase. Instead, always create/edit the files (2) and (3) for customization.

When the same variable is in more than one suibase.yaml, the last one loaded takes effect.

In short... (1) is how suibase first initialize defaults for **every** variable, you then optionally create (2) to apply customization on all workdir and optionally edit (3) for the final level of customization specific to a workdir.

