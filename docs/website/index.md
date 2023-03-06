---
hide:
  - toc
---
# What is Sui-Base?

sui-base create "workdirs", each defining a development environment targeting a network.

The workdir abstraction allows your whole development environment (app, SDKs, automation) to seamlessly switch environment... and use the properly matching sui binary version, keystore, configuration and more... (e.g the ObjectID of your last packaged published).

Other features includes:

  * Simple "$ localnet star/stop/status" command.
  * Deterministic "$ localnet regen" with always the same client addresses and customizable gas refill.
  * "$ localnet publish" for quick edit/debug cycle.
  * lsui/dsui/tsui scripts to call respectively the **proper** localnet/devnet/testnet sui binary.
  * More to come...

Easy to [install](how-to/install.md) and not intrusive on your system.

Sui-base is community driven, please join [our Discord :octicons-link-external-16:](https://discord.com/invite/Erb6SwsVbH) to share with us your Sui development need!
