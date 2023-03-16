# Workdir Customization ( sui-base.yaml )

Changing the remote github repo, branch, remote RPC, ports etc... are done using the sui-base.yaml found in each workdir ( Example: ~/sui-base/workdirs/localnet/sui-base.yaml )

The website will cover only the most common use case.

See this [sui-base.yaml :octicons-link-external-16:](https://github.com/sui-base/sui-base/blob/main/scripts/defaults/localnet/sui-base.yaml) for the complete list of configurable parameters.

### How to increase the initial funding of localnet?
Add `#!yaml initial_fund_per_address: "9999999999999999999"` then type "localnet regen".

Set the number to as much as you need (max 64 bits unsigned supported).

### How to add more address with different key type?

`#!yaml default_client_add_new_address:
    -  { n: <number>, scheme: "string", path: "string" }`

n: number of key of this scheme added (max 100)<br>
path: optional parameter derivation path

Example:
``` yaml
 default_client_add_new_address:
  - { n: 3, scheme: "ed25519", path: "m/44'/784'/0'/0'/0'" }
  - { n: 1, scheme: "secp256r1"}
```

On localnet, the addresses are added to the ones that are normally created by default.

On devnet/testnet, this is the definition of what is going to be created by default.

The client addresses do not normally changes on regen/start/update commands.

Use the command "reset-keystore" to force the application of these configuration changes (e.g. localnet reset-keystore).


