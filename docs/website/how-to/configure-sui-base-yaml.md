# Workdir Customization ( sui-base.yaml )

Changing the remote github repo, branch, RPC ports etc... are done using the sui-base.yaml found in each workdir ( Example: ~/sui-base/workdirs/localnet/sui-base.yaml )

We will cover here only a few common use case. See this [sui-base.yaml :octicons-link-external-16:](https://github.com/sui-base/sui-base/blob/main/scripts/defaults/localnet/sui-base.yaml) for the complete parameters list.

### How to increase the initial funding of localnet?
Add `#!yaml initial_fund_per_address: 9999999999999999999` then type "localnet regen".

Set the number to as much as you need (max 64 bits unsigned supported).

### How to change the default branch and/or repo used by a workdir?
Add the default_repo_XXXX variables (it is ok to change only one) and then type the workdir update command (e.g. "localnet update"). Example:

``` yaml
default_repo_url: "https://github.com/acme/forked_sui.git"
default_repo_branch: "main"
```

