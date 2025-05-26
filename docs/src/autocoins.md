---
title: Autocoins
---

::: warning
This feature is work-in-progress. It is actively being developed, but not yet released.
Github tracking: https://github.com/ChainMovers/suibase/issues/117
:::

Autocoins is an optional feature for automatic daily deposit of testnet Sui coins into any Sui address.

Once enabled, there is no further action required from you.

Some requirements to be aware of:
  - Daily deposits starts ~25 days after enabling autocoins for the first time.
  - Requires ~500MB of disk space (recoverable when disabling the feature).

These requirements relate to the "proof-of-installation" protocol used to fight bots.

For help type ```$ testnet autocoins --help```

## Enabling Autocoins

If not already done, first [install suibase]( how-to/install.md ).

Do ```$ testnet autocoins enable```

You can verify your config with ```$ testnet autocoins status```


## Set deposit address

Do ```$ testnet autocoins set <address>```

It may take up to 48 hours before a new address starts to be used for deposits.

If you do not set a deposit address, the default will be the active address at the time the feature was enabled.

## Disabling Autocoins
Type ```$ testnet autocoins disable```

This will stop further verification and deposits, but will still consume space on your local storage.

If later re-enabling, then it may take up to 48 hours for the deposits to resume.

## Proof-of-installation storage
The protocol requires ~500MB stored in ```~/suibase/workdirs/common/autocoins/data```

There is nothing for you to add/modify there, but you might appreciate knowing **where** the data is stored.

To delete the storage, do ```$ testnet autocoins purge-data```. It will take ~25 days to re-download the data and resume daily deposits.

## Wallet
Suibase generate its own testnet keystore, from which the default active-address is selected.

Location of the keystore is:
 ```~/suibase/workdirs/testnet/config/sui.keystore```

You can check the balance with ```$ tsui client balance``` and get the active-address with ```$ tsui client active-address```

## How is this implemented?
Check this [youtube video](https://youtu.be/U1RaYA0BJUE) for the high level design.

For more implementation details, ask on the [ChainMovers Discord](https://discord.com/invite/Erb6SwsVbH)
