# Workdir Customization ( sui-base.yaml )

Changing the remote github repo, branch, RPC ports etc... are done using the sui-base.yaml found in each workdir ( Example: ~/sui-base/workdirs/localnet/sui-base.yaml )

We will cover here only a few common use case. See this [sui-base.yaml](https://github.com/sui-base/sui-base/blob/main/scripts/defaults/localnet/sui-base.yaml) for the complete parameters list.

### Increase localnet initial funding
Add ```initial_fund_per_address: 9999999999999999999``` to the file then type ```localnet regen```.

Set the number to as much as you need (max 64 bits unsigned supported).

### Change default repo and branch
Add the default_repo_XXXX variables (it is ok to change only one) and then type the workdir update command (e.g. "localnet update"). Example:

``` yaml
default_repo_url: "https://github.com/acme/forked_sui.git"
default_repo_branch: "main"
```

