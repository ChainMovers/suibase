# Workdir Customization ( sui-base.yaml )

Changing the remote github repo, branch, remote RPC, ports etc... are done using the sui-base.yaml found in each workdir ( Example: ~/sui-base/workdirs/localnet/sui-base.yaml )

The website will cover only the most common use case.

See this [sui-base.yaml :octicons-link-external-16:](https://github.com/sui-base/sui-base/blob/main/scripts/defaults/localnet/sui-base.yaml) for the complete list of configurable parameters.

### How to increase the initial funding of localnet?
Add `#!yaml initial_fund_per_address: 9999999999999999999` then type "localnet regen".

Set the number to as much as you need (max 64 bits unsigned supported).
