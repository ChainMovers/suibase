::: warning
This feature is work-in-progress. It is actively being developed, but not yet released.
Related Issue: https://github.com/ChainMovers/suibase/issues/117
:::

Autocoins is an optional feature for automatic daily deposit of testnet Sui coins into any Sui address.

Once enabled, there is no further action required from you.

Some requirements to be aware of:
  - Daily deposits starts approximately 25 days after enabling the feature.
  - Requires ~500MB of disk space (recoverable when disabling the feature).

These requirements relate to the "proof-of-installation" protocol used to fight bots.

For help type ```testnet autocoins --help```

## Enabling Autocoins

Type ```tesnet autocoins enable```

You can verify your config with ```testnet autocoins status```


## Setup Deposit address

Type ```testnet autocoins set <address>```

It may take up to 48 hours before a new address starts to be used for deposits.

If you do not set a deposit address, the default will be the active address at the time the feature was enabled.

## Disabling Autocoins
Type ```tesnet autocoins disable```

This will stop further verification and deposits, but will still consume ~500MB on your local storage.

If you


## Proof-of-installation Data
The protocol requires ~500MB stored in ```~/suibase/workdirs/common/autocoins/data```

There is nothing for you to add/modify there, but you might appreciate knowing **where** the data is stored.

To delete the storage, do ```testnet autocoins purge-data```. It will take up to 25 days to re-download the data and resume daily deposits.

# Wallet
If you are new to Suibase, a testnet wallet was generated for your and located in:
 ```~/suibase/workdirs/testnet/config/sui.keystore```

You can check the balance with ```tsui client balance``` and get the active-address with ```tsui client active-address```
